//! The scale, tick locators and tick formatter of a colorbar's long axis
//! (matplotlib's `_reset_locator_formatter_scale`,
//! `_get_ticker_locator_formatter` and `update_ticks`).

use std::cell::RefCell;
use std::rc::Rc;

use super::blocks::{Blocks, NormClass, forward_boundaries, inverse_boundaries};
use super::{Colorbar, ColorbarError, ColorbarFormat, ColorbarTicks, Spacing};
use crate::axes::Axis;
use crate::colors::{ArrayForm, Norm, NormKind, Normalized};
use crate::scale::{
    ArrayLayout, FuncScale, FuncTransform, LinearScale, Scale, ScaleFunction, ScaleName,
    ScaleOptions, scale_factory,
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
                Box::new(FuncScale::from_transform(FuncTransform::from_functions(
                    Rc::new(BoundarySpacing {
                        boundaries: Rc::clone(&boundaries),
                        inverse: false,
                    }),
                    Rc::new(BoundarySpacing {
                        boundaries,
                        inverse: true,
                    }),
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
    Box::new(FuncScale::from_transform(FuncTransform::from_functions(
        Rc::new(NormSpacing {
            norm: RefCell::new(norm.clone_box()),
            inverse: false,
        }),
        Rc::new(NormSpacing {
            norm: RefCell::new(norm.clone_box()),
            inverse: true,
        }),
    )))
}

/// The positions of values spread evenly between a colorbar's block
/// boundaries (matplotlib's `_forward_boundaries`), or with `inverse`,
/// the values at positions (`_inverse_boundaries`).
#[derive(Debug)]
pub(crate) struct BoundarySpacing {
    boundaries: Rc<[f64]>,
    inverse: bool,
}

pltrs_core::archive_struct!(BoundarySpacing {
    boundaries,
    inverse
});

impl ScaleFunction for BoundarySpacing {
    fn call(&self, value: f64) -> f64 {
        if self.boundaries.is_empty() {
            f64::NAN
        } else if self.inverse {
            inverse_boundaries(&self.boundaries, value)
        } else {
            forward_boundaries(&self.boundaries, &[value])[0]
        }
    }

    fn calls_program(&self) -> bool {
        false
    }
}

/// A value mapped through a norm, or with `inverse`, the value a norm
/// maps to a fraction: the long axis of a colorbar whose norm has no
/// scale of its own.
#[derive(Debug)]
pub(crate) struct NormSpacing {
    norm: RefCell<Box<dyn Norm>>,
    inverse: bool,
}

pltrs_core::archive_struct!(NormSpacing { norm, inverse });

impl NormSpacing {
    /// Runs `work` on the norm: a norm of the program's on a copy of its
    /// own (which calls the same Python norm), with no borrow held while
    /// the program's code runs, as it may draw the figure, and this scale
    /// with it, again.
    fn with_norm<R>(&self, work: impl FnOnce(&mut dyn Norm) -> R) -> R {
        let copy = (self.norm.try_borrow().ok())
            .filter(|norm| norm.calls_program())
            .map(|norm| norm.clone_box());
        match copy {
            Some(mut norm) => work(norm.as_mut()),
            None => work(self.norm.borrow_mut().as_mut()),
        }
    }
}

impl ScaleFunction for NormSpacing {
    fn call(&self, value: f64) -> f64 {
        if self.inverse {
            return self
                .with_norm(|norm| norm.inverse(&[value]))
                .ok()
                .and_then(|v| v.first().copied().flatten())
                .unwrap_or(f64::NAN);
        }
        let mapped = self.with_norm(|norm| norm.normalize(&[value]));
        match mapped.as_deref() {
            Ok([Normalized::Fraction(v)]) => *v,
            Ok([Normalized::Index(i)]) => *i as f64,
            _ => f64::NAN,
        }
    }

    fn calls_program(&self) -> bool {
        // (A norm borrowed is one of the program's, being called.)
        self.norm
            .try_borrow()
            .map_or(true, |norm| norm.calls_program())
    }

    /// The norm (or its inverse) called once with all the values, as
    /// matplotlib's colorbar scale calls it with arrays, laid out as
    /// `layout` says (a column where matplotlib's code calls the scale's
    /// `transform`), which a norm of the program's sees.
    fn call_all(&self, values: &mut [f64], layout: ArrayLayout) {
        let mapped: Option<Vec<f64>> = if self.inverse {
            self.with_norm(|norm| norm.inverse(values))
                .ok()
                .map(|mapped| mapped.into_iter().map(|v| v.unwrap_or(f64::NAN)).collect())
        } else {
            let shape = match layout {
                ArrayLayout::Row | ArrayLayout::MaskedRow => vec![values.len()],
                ArrayLayout::Column => vec![values.len(), 1],
            };
            let data: Vec<Option<f64>> = values.iter().copied().map(Some).collect();
            let form = ArrayForm {
                shape: &shape,
                masked: layout == ArrayLayout::MaskedRow,
            };
            let normalized = self.with_norm(|norm| norm.normalize_shaped(&data, None, form));
            normalized.ok().map(|mapped| {
                mapped
                    .into_iter()
                    .map(|v| match v {
                        Normalized::Fraction(v) => v,
                        Normalized::Index(i) => i as f64,
                        Normalized::Masked => f64::NAN,
                    })
                    .collect()
            })
        };
        match mapped {
            Some(mapped) if mapped.len() == values.len() => values.copy_from_slice(&mapped),
            _ => values.fill(f64::NAN),
        }
    }
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
            data: None,
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
