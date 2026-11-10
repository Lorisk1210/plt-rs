//! A collection's values mapped to colours: the norm autoscaled when the
//! values are set, the colours made when drawing.
//!
//! A norm of the program's sees every call matplotlib makes to it: it is
//! autoscaled once, when the collection is made (`_scale_norm`), and
//! normalises the collection's array, as matplotlib's collection holds
//! it, at every draw (`update_scalarmappable`).

use std::cell::RefCell;

use super::ColorMapping;
use crate::artist::ProgramShaping;
use crate::colors::{Alpha, ArrayForm, ColormapError, Norm, NormError, NormKind, Normalized};

/// A [`ColorMapping`] with its norm autoscaled (matplotlib autoscales the
/// norm and normalises when the array is set) and its values normalised,
/// but for the norms that map each value alone: those normalise when
/// drawing, a block at a time, so that no normalised copy of all the
/// values is kept; and for the norms of the program's, which normalise
/// whenever the collection is drawn.
#[derive(Clone, Debug)]
pub(crate) struct Mapped {
    pub(crate) source: ColorMapping,
    /// The normalised values (empty while normalising when drawing).
    pub(crate) normalized: Result<Vec<Normalized>, String>,
    /// Whether the values are normalised when drawing, a block at a time.
    by_block: bool,
    /// Whether the norm is the program's, which normalises the values at
    /// every draw (see [`DrawnValues`]).
    at_draw: bool,
    /// The shape of the array matplotlib's collection holds (its `_A`),
    /// which it hands its norm: `None` for one dimension.
    shape: Option<Vec<usize>>,
    /// What the program's norm gave at the last draw.
    drawn: DrawnValues,
}

/// The values a norm of the program's normalised at the last draw, and
/// whether the coming paint of the figure is to draw them: normalised
/// where matplotlib's draw normalises them (see `axes::tick_sites`),
/// before pltrs paints the figure.
#[derive(Clone, Debug, Default)]
struct DrawnValues(RefCell<Option<(Normalization, bool)>>);

/// The values normalised, or why they could not be.
type Normalization = Result<Vec<Normalized>, String>;

/// The values normalised and coloured at once when drawing.
const COLOR_BLOCK: usize = 1 << 16;

/// Whether `norm` maps each value by itself once its limits are set, so
/// that the values can be normalised a block at a time: the built-in
/// linear, boundary and identity norms (a program's norm may look at all
/// the values, and a norm of functions calls them per normalisation).
fn normalizes_by_block(norm: &dyn Norm) -> bool {
    matches!(
        norm.kind(),
        NormKind::Linear | NormKind::Boundary(_) | NormKind::Identity
    )
}

pltrs_core::archive_struct!(Mapped {
    source,
    normalized,
    by_block,
    at_draw,
    shape,
} skip { drawn });

impl Mapped {
    /// The mapping with its norm autoscaled from its values.
    pub(super) fn new(source: ColorMapping) -> Self {
        let mut mapped = Self {
            source,
            normalized: Ok(Vec::new()),
            by_block: false,
            at_draw: false,
            shape: None,
            drawn: DrawnValues::default(),
        };
        if mapped.source.mappable.norm().calls_program() {
            // matplotlib's plotting functions autoscale it once
            // (`_scale_norm`), and its draws normalise.
            let ColorMapping { values, mappable } = &mut mapped.source;
            mappable.norm_mut().autoscale_none(values);
        }
        mapped.refresh();
        mapped
    }

    /// A mapping of `source` whose values its owner normalised by rules of
    /// its own (a contour set's levels and colour indices), kept as they
    /// are until the norm or the colormap of the collection changes.
    pub(crate) fn with_normalized(source: ColorMapping, normalized: Vec<Normalized>) -> Self {
        Self {
            source,
            normalized: Ok(normalized),
            by_block: false,
            at_draw: false,
            shape: None,
            drawn: DrawnValues::default(),
        }
    }

    /// A mapping of `source` through a norm of the program's that its owner
    /// scaled already (a contour set's, to its levels): the values are
    /// normalised at every draw, as matplotlib's collection draws do.
    pub(crate) fn normalized_at_draw(source: ColorMapping) -> Self {
        Self {
            source,
            normalized: Ok(Vec::new()),
            by_block: false,
            at_draw: true,
            shape: None,
            drawn: DrawnValues::default(),
        }
    }

    /// Fills unset limits from the values and normalises them again, after
    /// the norm or its limits changed (matplotlib's `set_array` followed by
    /// `update_scalarmappable`).
    pub(super) fn refresh(&mut self) {
        self.drawn = DrawnValues::default();
        self.at_draw = self.source.mappable.norm().calls_program();
        if self.at_draw {
            // The program's norm normalises when drawn, autoscaling itself
            // as it needs.
            self.normalized = Ok(Vec::new());
            return;
        }
        let ColorMapping { values, mappable } = &mut self.source;
        mappable.norm_mut().autoscale_none(values);
        self.by_block = normalizes_by_block(mappable.norm());
        self.normalized = if self.by_block {
            // Inverted limits fail every block alike: met now, they are the
            // `ValueError` of matplotlib's norm, as in the other mode.
            match mappable.norm().clone_box().normalize_masked(&[], None) {
                Err(err @ NormError::InvertedLimits { .. }) => Err(err.to_string()),
                _ => Ok(Vec::new()),
            }
        } else {
            mappable.normalize(values).map_err(|e| e.to_string())
        };
    }

    /// The mapped colours with `alpha` applied.
    pub(crate) fn colors(&self, alpha: &Alpha) -> Result<Vec<pltrs_core::Rgba>, String> {
        let mappable = &self.source.mappable;
        if self.by_block {
            return self.colors_by_block(alpha);
        }
        if self.at_draw {
            return self.drawn_colors(alpha);
        }
        let normalized = self.normalized.as_ref().map_err(Clone::clone)?;
        mappable
            .cmap()
            .map(normalized, alpha)
            .map_err(|err| err.to_string())
    }

    /// Whether the values are normalised whenever drawn, through a norm of
    /// the program's.
    pub(crate) fn at_draw(&self) -> bool {
        self.at_draw
    }

    /// Hands the program's norm the values as an array of `shape`, as
    /// matplotlib's collection holds them.
    pub(crate) fn set_shape(&mut self, shape: Vec<usize>) {
        self.shape = Some(shape);
    }

    /// Why the values cannot be normalised, as the last normalisation
    /// found (for a norm of the program's, that of the last draw).
    pub(crate) fn problem(&self) -> Option<String> {
        if self.at_draw {
            let drawn = self.drawn.0.borrow();
            return drawn.as_ref().and_then(|(values, _)| values.clone().err());
        }
        self.normalized.as_ref().err().cloned()
    }

    /// The values normalised through the program's norm, as
    /// `update_scalarmappable` normalises them, for the figure's coming
    /// paint, which draws them without normalising them again.
    pub(crate) fn normalize_for_paint(&self) {
        let values = self.normalize_now();
        self.drawn.0.replace(Some((values, true)));
    }

    /// The program's norm normalising the values as a draw of the
    /// collection does: unless they were normalised for this paint.
    #[inline(never)]
    pub(crate) fn normalize_drawn(&self) {
        if let Some((_, for_paint)) = self.drawn.0.borrow_mut().as_mut()
            && std::mem::take(for_paint)
        {
            return;
        }
        let values = self.normalize_now();
        self.drawn.0.replace(Some((values, false)));
    }

    /// [`colors`](Self::colors) of the values the program's norm gave at
    /// the last draw (kept out of the draws of every other collection).
    #[inline(never)]
    fn drawn_colors(&self, alpha: &Alpha) -> Result<Vec<pltrs_core::Rgba>, String> {
        let drawn = self.drawn_values()?;
        (self.source.mappable.cmap().map(&drawn, alpha)).map_err(|err| err.to_string())
    }

    /// The values the program's norm gave at the last draw, normalised
    /// now if the collection was never drawn.
    fn drawn_values(&self) -> Result<Vec<Normalized>, String> {
        if let Some((values, _)) = self.drawn.0.borrow().as_ref() {
            return values.clone();
        }
        let values = self.normalize_now();
        self.drawn.0.replace(Some((values.clone(), false)));
        values
    }

    /// The values through a copy of the norm (the program's own object),
    /// as a masked array of the collection's shape.
    fn normalize_now(&self) -> Result<Vec<Normalized>, String> {
        let mut norm = self.source.mappable.norm().clone_box();
        normalize_shaped(&mut *norm, &self.source.values, self.shape.as_deref())
    }

    /// The program's norm normalising the values as the draw a print makes
    /// before it renders calls it (see
    /// [`Artist::program_shaping`](crate::artist::Artist::program_shaping)).
    pub(crate) fn predraw_normalization(&self) -> Option<ProgramShaping> {
        if !self.at_draw {
            return None;
        }
        let mut norm = self.source.mappable.norm().clone_box();
        let values = self.source.values.clone();
        let shape = self.shape.clone();
        Some(Box::new(move || {
            drop(normalize_shaped(&mut *norm, &values, shape.as_deref()));
        }))
    }

    /// [`Self::colors`] normalising the values a block at a time with a
    /// copy of the (autoscaled) norm.
    fn colors_by_block(&self, alpha: &Alpha) -> Result<Vec<pltrs_core::Rgba>, String> {
        let mappable = &self.source.mappable;
        let values = &self.source.values;
        let mut norm = mappable.norm().clone_box();
        let mut colors = Vec::with_capacity(values.len());
        // Empty values still meet the norm, which may refuse them.
        let blocks = values
            .chunks(COLOR_BLOCK)
            .chain(values.is_empty().then_some(&values[..]));
        for (index, block) in blocks.enumerate() {
            let normalized = norm
                .normalize_masked(block, None)
                .map_err(|e| e.to_string())?;
            let alpha = match alpha {
                Alpha::Each(alphas) if alphas.len() == values.len() => {
                    let start = index * COLOR_BLOCK;
                    Alpha::Each(alphas[start..start + block.len()].to_vec())
                }
                Alpha::Each(alphas) => {
                    let error = ColormapError::AlphaCount {
                        values: values.len(),
                        alphas: alphas.len(),
                    };
                    return Err(error.to_string());
                }
                uniform => uniform.clone(),
            };
            let mapped = mappable.cmap().map(&normalized, &alpha);
            colors.extend(mapped.map_err(|err| err.to_string())?);
        }
        Ok(colors)
    }
}

/// `values` through `norm` as a masked array of `shape` (one dimension
/// when `None`).
fn normalize_shaped(
    norm: &mut dyn Norm,
    values: &[Option<f64>],
    shape: Option<&[usize]>,
) -> Result<Vec<Normalized>, String> {
    let flat = [values.len()];
    let form = ArrayForm {
        shape: shape.unwrap_or(&flat),
        masked: true,
    };
    let normalized = norm.normalize_shaped(values, None, form);
    normalized.map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{BoundaryNorm, Extend, Normalize, ScalarMappable, colormaps};

    /// The colours of all the values normalised at once.
    fn at_once(mapped: &Mapped, alpha: &Alpha) -> Vec<pltrs_core::Rgba> {
        let mut mappable = mapped.source.mappable.clone();
        let normalized = mappable.normalize(&mapped.source.values).unwrap();
        mappable.cmap().map(&normalized, alpha).unwrap()
    }

    #[test]
    fn blocks_colour_as_the_whole_does() {
        let n = 2 * COLOR_BLOCK + 123;
        let values: Vec<Option<f64>> = (0..n)
            .map(|i| (i % 97 != 3).then(|| (i % 1000) as f64 / 7.0))
            .collect();
        let alphas: Vec<f64> = (0..n).map(|i| (i % 256) as f64 / 255.0).collect();
        let cmap = || colormaps::get("viridis").unwrap();
        let norms: [Box<dyn Norm>; 2] = [
            Box::new(Normalize::default()),
            Box::new(
                BoundaryNorm::new(vec![0.0, 30.0, 90.0, 150.0], 256, Extend::Neither).unwrap(),
            ),
        ];
        for norm in norms {
            let mapping = ColorMapping {
                values: values.clone(),
                mappable: ScalarMappable::with_norm(cmap(), norm),
            };
            let mapped = Mapped::new(mapping);
            assert!(mapped.by_block);
            assert_eq!(mapped.normalized, Ok(Vec::new()), "no copy is kept");
            for alpha in [
                Alpha::Keep,
                Alpha::Uniform(0.3),
                Alpha::Each(alphas.clone()),
            ] {
                assert_eq!(mapped.colors(&alpha).unwrap(), at_once(&mapped, &alpha));
            }
            let short = mapped.colors(&Alpha::Each(vec![0.5; 3])).unwrap_err();
            assert_eq!(
                short,
                format!("3 alpha values do not match {n} data values")
            );
        }
    }

    #[test]
    fn empty_values_still_meet_the_norm() {
        let mut norm = Normalize::default();
        norm.set_vmin(Some(1.0));
        let mapping = ColorMapping {
            values: Vec::new(),
            mappable: ScalarMappable::with_norm(colormaps::get("viridis").unwrap(), Box::new(norm)),
        };
        let mapped = Mapped::new(mapping);
        let mut whole = mapped.source.mappable.clone();
        let expected = whole.normalize(&[]).map_err(|e| e.to_string());
        assert_eq!(
            mapped.colors(&Alpha::Keep).map(|c| c.len()),
            expected.map(|v| v.len())
        );
    }

    #[test]
    fn inverted_limits_are_met_when_the_values_are_set() {
        let mut norm = Normalize::default();
        norm.set_vmin(Some(5.0));
        norm.set_vmax(Some(2.0));
        let mapping = ColorMapping {
            values: vec![Some(1.0), Some(3.0)],
            mappable: ScalarMappable::with_norm(colormaps::get("viridis").unwrap(), Box::new(norm)),
        };
        let mapped = Mapped::new(mapping);
        assert!(mapped.by_block);
        let message = "minvalue must be less than or equal to maxvalue";
        assert_eq!(mapped.normalized, Err(message.to_owned()));
        assert_eq!(mapped.colors(&Alpha::Keep), Err(message.to_owned()));
    }

    /// The calls a norm of the program's got: `"autoscale"`, or the
    /// shape and maskedness of an array normalised.
    type Calls = std::rc::Rc<std::cell::RefCell<Vec<String>>>;

    /// A linear norm over `[0, 1]` standing for one of the program's,
    /// recording its calls.
    #[derive(Clone, Debug, Default)]
    struct Program {
        calls: Calls,
    }

    impl Norm for Program {
        fn vmin(&self) -> Option<f64> {
            Some(0.0)
        }

        fn vmax(&self) -> Option<f64> {
            Some(1.0)
        }

        fn set_vmin(&mut self, _: Option<f64>) {}

        fn set_vmax(&mut self, _: Option<f64>) {}

        fn clip(&self) -> bool {
            false
        }

        fn set_clip(&mut self, _: bool) {}

        fn autoscale_none(&mut self, _: &[Option<f64>]) {
            self.calls.borrow_mut().push("autoscale".to_owned());
        }

        fn normalize_masked(
            &mut self,
            data: &[Option<f64>],
            _: Option<bool>,
        ) -> Result<Vec<Normalized>, NormError> {
            Ok(data
                .iter()
                .map(|v| v.map_or(Normalized::Masked, Normalized::Fraction))
                .collect())
        }

        fn normalize_shaped(
            &mut self,
            data: &[Option<f64>],
            clip: Option<bool>,
            form: ArrayForm<'_>,
        ) -> Result<Vec<Normalized>, NormError> {
            self.calls
                .borrow_mut()
                .push(format!("{:?} {}", form.shape, form.masked));
            self.normalize_masked(data, clip)
        }

        fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
            Ok(values.iter().copied().map(Some).collect())
        }

        fn calls_program(&self) -> bool {
            true
        }
    }

    #[test]
    fn the_program_s_norm_is_autoscaled_once_and_normalises_at_every_draw() {
        let norm = Program::default();
        let calls = std::rc::Rc::clone(&norm.calls);
        let mapping = ColorMapping {
            values: vec![Some(0.25), None, Some(0.75), Some(1.0)],
            mappable: ScalarMappable::with_norm(colormaps::get("viridis").unwrap(), Box::new(norm)),
        };
        let mut mapped = Mapped::new(mapping);
        mapped.set_shape(vec![2, 2]);
        assert_eq!(*calls.borrow(), ["autoscale"], "nothing normalised yet");
        // A site of the draw normalises; the paint draws what it gave.
        mapped.normalize_for_paint();
        mapped.normalize_drawn();
        let colors = mapped.colors(&Alpha::Keep).unwrap();
        assert_eq!(colors.len(), 4);
        assert_eq!(*calls.borrow(), ["autoscale", "[2, 2] true"]);
        // A draw no site prepared normalises itself, every time.
        mapped.normalize_drawn();
        mapped.normalize_drawn();
        assert_eq!(calls.borrow().len(), 4);
        assert_eq!(mapped.problem(), None);
    }
}
