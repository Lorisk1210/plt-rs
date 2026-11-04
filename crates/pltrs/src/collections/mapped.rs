//! A collection's values mapped to colours: the norm autoscaled when the
//! values are set, the colours made when drawing.

use super::ColorMapping;
use crate::colors::{Alpha, ColormapError, Norm, NormKind, Normalized};

/// A [`ColorMapping`] with its norm autoscaled (matplotlib autoscales the
/// norm and normalises when the array is set) and its values normalised,
/// but for the norms that map each value alone: those normalise when
/// drawing, a block at a time, so that no normalised copy of all the
/// values is kept.
#[derive(Clone, Debug)]
pub(crate) struct Mapped {
    pub(crate) source: ColorMapping,
    /// The normalised values (empty for norms normalising when drawing).
    pub(crate) normalized: Result<Vec<Normalized>, String>,
}

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

pltrs_core::archive_struct!(Mapped { source, normalized });

impl Mapped {
    /// The mapping with its norm autoscaled from its values.
    pub(super) fn new(source: ColorMapping) -> Self {
        let mut mapped = Self {
            source,
            normalized: Ok(Vec::new()),
        };
        mapped.refresh();
        mapped
    }

    /// Fills unset limits from the values and normalises them again, after
    /// the norm or its limits changed (matplotlib's `set_array` followed by
    /// `update_scalarmappable`).
    pub(super) fn refresh(&mut self) {
        let ColorMapping { values, mappable } = &mut self.source;
        mappable.norm_mut().autoscale_none(values);
        self.normalized = if normalizes_by_block(mappable.norm()) {
            Ok(Vec::new())
        } else {
            mappable.normalize(values).map_err(|e| e.to_string())
        };
    }

    /// The mapped colours with `alpha` applied.
    pub(crate) fn colors(&self, alpha: &Alpha) -> Result<Vec<pltrs_core::Rgba>, String> {
        let mappable = &self.source.mappable;
        if normalizes_by_block(mappable.norm()) {
            return self.colors_by_block(alpha);
        }
        let normalized = self.normalized.as_ref().map_err(Clone::clone)?;
        mappable
            .cmap()
            .map(normalized, alpha)
            .map_err(|err| err.to_string())
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
            assert!(normalizes_by_block(mapped.source.mappable.norm()));
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
}
