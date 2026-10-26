//! The scale, tick locators and tick formatter of a colorbar's long axis
//! (matplotlib's `_reset_locator_formatter_scale`,
//! `_get_ticker_locator_formatter` and `update_ticks`).

use std::cell::RefCell;
use std::rc::Rc;

use super::blocks::{Blocks, NormClass, forward_boundaries, inverse_boundaries};
use super::{Colorbar, ColorbarError, ColorbarFormat, ColorbarTicks, Spacing};
use crate::axes::Axis;
use crate::colors::{Norm, NormKind, Normalized};
use crate::scale::{
    FuncScale, FuncTransform, LinearScale, Scale, ScaleName, ScaleOptions, scale_factory,
};
use crate::ticker::{FixedLocator, Formatter, IndexLocator, Locator, NullLocator};

/// The scale of the long axis: from the norm, or spreading discrete blocks
/// evenly.
pub(crate) fn long_axis_scale(
    blocks: &Blocks,
    norm: &dyn Norm,
    spacing: Spacing,
    contours: bool,
) -> Box<dyn Scale> {
    let kind = norm.kind();
    if contours && matches!(&kind, NormKind::Scaled(scale) if scale.name() == ScaleName::Log) {
        return scale_factory("log", ScaleOptions::default())
            .expect("the log scale is always registered");
    }
    if blocks.explicit || blocks.class == NormClass::Boundary {
        return match spacing {
            Spacing::Uniform => {
                let boundaries: Rc<[f64]> = Rc::from(blocks.boundaries.as_slice());
                let inverse = Rc::clone(&boundaries);
                Box::new(FuncScale::from_transform(FuncTransform::from_fns(
                    move |x| forward_boundaries(&boundaries, &[x])[0],
                    move |y| inverse_boundaries(&inverse, y),
                )))
            }
            Spacing::Proportional => Box::new(LinearScale),
        };
    }
    match kind {
        NormKind::Scaled(scale) => scale,
        NormKind::Linear => Box::new(LinearScale),
        _ => norm_scale(norm),
    }
}

/// Whether [`long_axis_scale`] is a function scale through the norm, which
/// in matplotlib calls the norm itself and so follows its limits.
pub(crate) fn scale_follows_limits(blocks: &Blocks, norm: &dyn Norm) -> bool {
    !blocks.explicit && matches!(norm.kind(), NormKind::Identity | NormKind::Other)
}

/// A function scale through the norm and its inverse.
fn norm_scale(norm: &dyn Norm) -> Box<dyn Scale> {
    let forward = RefCell::new(norm.clone_box());
    let inverse = norm.clone_box();
    Box::new(FuncScale::from_transform(FuncTransform::from_fns(
        move |x| {
            let mapped = forward.borrow_mut().normalize(&[x]);
            match mapped.as_deref() {
                Ok([Normalized::Fraction(v)]) => *v,
                Ok([Normalized::Index(i)]) => *i as f64,
                _ => f64::NAN,
            }
        },
        move |y| {
            inverse
                .inverse(&[y])
                .ok()
                .and_then(|v| v.first().copied().flatten())
                .unwrap_or(f64::NAN)
        },
    )))
}

/// The tick helpers a colorbar installs on its long axis; `None` keeps the
/// axis default.
#[derive(Debug, Default)]
pub(crate) struct Tickers {
    /// The major tick locator.
    pub(crate) major: Option<Rc<dyn Locator>>,
    /// The minor tick locator.
    pub(crate) minor: Option<Rc<dyn Locator>>,
    /// The major tick formatter.
    pub(crate) formatter: Option<Rc<dyn Formatter>>,
}

impl Tickers {
    /// matplotlib's `_get_ticker_locator_formatter`, from the user's
    /// `ticks` and `format`.
    pub(crate) fn choose(
        blocks: &Blocks,
        norm: &dyn Norm,
        ticks: Option<&ColorbarTicks>,
        format: Option<&ColorbarFormat>,
    ) -> Result<Self, ColorbarError> {
        let mut major = ticks.map(|ticks| match ticks {
            ColorbarTicks::Values(values) => {
                fixed(FixedLocator::new(values.clone()).with_nbins(values.len()))
            }
            ColorbarTicks::Locator(locator) => Rc::clone(locator),
        });
        let mut minor = None;
        match norm.kind() {
            NormKind::Boundary(boundary) => {
                let b = boundary.boundaries().to_vec();
                major.get_or_insert_with(|| fixed(FixedLocator::new(b.clone()).with_nbins(10)));
                minor = Some(fixed(FixedLocator::new(b)));
            }
            NormKind::Identity => {
                let base = 1 + blocks.values.len() / 10;
                major.get_or_insert_with(|| Rc::new(IndexLocator::new(base as f64, 0.5)));
                minor = Some(Rc::new(NullLocator));
            }
            _ if blocks.explicit => {
                let inside = blocks.inside().to_vec();
                major.get_or_insert_with(|| fixed(FixedLocator::new(inside).with_nbins(10)));
                minor = Some(Rc::new(NullLocator));
            }
            _ => {}
        }
        Ok(Self {
            major,
            minor,
            formatter: format.map(formatter).transpose()?,
        })
    }

    /// matplotlib's `update_ticks`.
    pub(crate) fn install(&self, axis: &Axis) {
        if let Some(locator) = &self.major {
            axis.set_shared_major_locator(locator);
        }
        if let Some(locator) = &self.minor {
            axis.set_shared_minor_locator(locator);
        }
        if let Some(formatter) = &self.formatter {
            axis.set_shared_major_formatter(formatter);
        }
    }
}

fn fixed(locator: FixedLocator) -> Rc<dyn Locator> {
    Rc::new(locator)
}

/// The tick helpers of the long axis, as properties of the colorbar
/// (matplotlib's `locator`, `minorlocator`, `formatter` and
/// `minorformatter`). Those set stay when the mappable's limits or
/// colormap change; a new norm brings its own.
impl Colorbar {
    /// The locator of the major ticks along the colorbar.
    pub fn locator(&self) -> Rc<dyn Locator> {
        self.long_axis().major_locator()
    }

    /// Places the major ticks along the colorbar with `locator`.
    pub fn set_locator(&self, locator: impl Locator) -> &Self {
        self.long_axis().set_major_locator(locator);
        self
    }

    /// The locator of the minor ticks along the colorbar.
    pub fn minor_locator(&self) -> Rc<dyn Locator> {
        self.long_axis().minor_locator()
    }

    /// Places the minor ticks along the colorbar with `locator`.
    pub fn set_minor_locator(&self, locator: impl Locator) -> &Self {
        self.long_axis().set_minor_locator(locator);
        self
    }

    /// The formatter of the major tick labels along the colorbar.
    pub fn formatter(&self) -> Rc<dyn Formatter> {
        self.long_axis().major_formatter()
    }

    /// Labels the major ticks along the colorbar with `formatter`.
    pub fn set_formatter(&self, formatter: impl Formatter) -> &Self {
        self.long_axis().set_major_formatter(formatter);
        self
    }

    /// The formatter of the minor tick labels along the colorbar.
    pub fn minor_formatter(&self) -> Rc<dyn Formatter> {
        self.long_axis().minor_formatter()
    }

    /// Labels the minor ticks along the colorbar with `formatter`.
    pub fn set_minor_formatter(&self, formatter: impl Formatter) -> &Self {
        self.long_axis().set_minor_formatter(formatter);
        self
    }
}

/// The formatter for `format`: a `%` template when it formats a number,
/// else a `str.format` template.
fn formatter(format: &ColorbarFormat) -> Result<Rc<dyn Formatter>, ColorbarError> {
    match format {
        ColorbarFormat::Formatter(formatter) => Ok(Rc::clone(formatter)),
        ColorbarFormat::Label(label) => Ok(label.formatter()?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colorbar::ExtendFrac;
    use crate::colorbar::blocks::BlockSpec;
    use crate::colors::{BoundaryNorm, Extend, LogNorm, Normalize, TwoSlopeNorm};

    fn blocks(norm: &mut dyn Norm, boundaries: Option<&[f64]>) -> Blocks {
        let spec = BlockSpec {
            ncolors: 8,
            boundaries,
            values: None,
            value_kind: crate::image::ScalarKind::Float,
            extend: Extend::Neither,
            inverted: false,
            extendfrac: ExtendFrac::Default,
            spacing: Spacing::Uniform,
        };
        Blocks::new(&spec, norm).unwrap()
    }

    #[test]
    fn scales_follow_the_norm() {
        let mut linear = Normalize::new(0.0, 1.0);
        let b = blocks(&mut linear, None);
        assert_eq!(
            long_axis_scale(&b, &linear, Spacing::Uniform, false).name(),
            ScaleName::Linear
        );
        let mut log = LogNorm::new(1.0, 100.0);
        let b = blocks(&mut log, None);
        assert_eq!(
            long_axis_scale(&b, &log, Spacing::Uniform, false).name(),
            ScaleName::Log
        );
        let mut two = TwoSlopeNorm::new(0.0, Some(-1.0), Some(4.0)).unwrap();
        let b = blocks(&mut two, None);
        let scale = long_axis_scale(&b, &two, Spacing::Uniform, false);
        assert_eq!(scale.name(), ScaleName::Function);
        assert_eq!(scale.forward(&[0.0, 4.0]), [0.5, 1.0]);
        assert_eq!(scale.inverse(&[0.5]), [0.0]);
    }

    #[test]
    fn discrete_colorbars_spread_or_keep_their_blocks() {
        let mut norm = BoundaryNorm::new([0.0, 1.0, 4.0], 2, Extend::Neither).unwrap();
        let b = blocks(&mut norm, None);
        let uniform = long_axis_scale(&b, &norm, Spacing::Uniform, false);
        assert_eq!(uniform.forward(&[1.0, 2.5]), [0.5, 0.75]);
        let proportional = long_axis_scale(&b, &norm, Spacing::Proportional, false);
        assert_eq!(proportional.name(), ScaleName::Linear);
    }

    #[test]
    fn boundary_norms_tick_their_edges() {
        let mut norm = BoundaryNorm::new([0.0, 1.0, 4.0], 2, Extend::Neither).unwrap();
        let b = blocks(&mut norm, None);
        let tickers = Tickers::choose(&b, &norm, None, None).unwrap();
        assert!(tickers.major.is_some() && tickers.minor.is_some());
        assert!(tickers.formatter.is_none());
        let mut linear = Normalize::new(0.0, 1.0);
        let b = blocks(&mut linear, None);
        let format = ColorbarFormat::Label("{x:.1f}".into());
        let tickers = Tickers::choose(&b, &linear, None, Some(&format)).unwrap();
        assert!(tickers.major.is_none() && tickers.minor.is_none());
        assert!(tickers.formatter.is_some(), "a str.format template");
    }

    #[test]
    fn ticker_properties_are_the_long_axis_ones() {
        use crate::colorbar::ColorbarOptions;
        use crate::colors::{ScalarMappable, colormaps};
        use crate::figure::Figure;
        use crate::ticker::{LogFormatter, NullFormatter};

        let fig = Figure::new();
        let ax = fig.subplot();
        let mappable = ScalarMappable::with_norm(
            colormaps::get("viridis").unwrap(),
            Box::new(Normalize::new(0.0, 1.0)),
        );
        let colorbar = fig
            .colorbar(&mappable, ColorbarOptions::beside(&ax))
            .unwrap();
        let axis = colorbar.ax().yaxis();
        colorbar
            .set_locator(FixedLocator::new(vec![0.0, 0.5]))
            .set_minor_locator(NullLocator)
            .set_formatter(LogFormatter::default())
            .set_minor_formatter(NullFormatter);
        assert!(Rc::ptr_eq(&colorbar.locator(), &axis.major_locator()));
        assert!(Rc::ptr_eq(&colorbar.minor_locator(), &axis.minor_locator()));
        assert!(Rc::ptr_eq(&colorbar.formatter(), &axis.major_formatter()));
        assert!(Rc::ptr_eq(
            &colorbar.minor_formatter(),
            &axis.minor_formatter()
        ));
        let locator = colorbar.locator();
        assert!((locator.as_ref() as &dyn std::any::Any).is::<FixedLocator>());
    }
}
