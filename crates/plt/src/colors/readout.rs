//! What a status bar shows of colour-mapped data: each value with as many
//! significant digits as one colour step of its colormap resolves
//! (matplotlib's `_ColorizerInterface._format_cursor_data_override`).

use super::multivar::MultiMappable;
use super::{Norm, NormKind, Normalized, ScalarMappable};
use crate::ticker::{g_sig_digits, pyfmt};

/// The digits matplotlib falls back to when a value's resolution is
/// unknown.
const DEFAULT_DIGITS: usize = 3;

impl ScalarMappable {
    /// `value` as a status bar shows it, such as `[0.532]`, or `[]` when it
    /// is masked (matplotlib's `format_cursor_data` of colour-mapped
    /// artists).
    ///
    /// ```
    /// use plt::colors::{Normalize, ScalarMappable, colormaps};
    /// let mappable =
    ///     ScalarMappable::with_norm(colormaps::get("viridis")?, Box::new(Normalize::new(0.0, 1.0)));
    /// assert_eq!(mappable.format_cursor_data(Some(0.123_456)), "[0.123]");
    /// assert_eq!(mappable.format_cursor_data(None), "[]");
    /// # Ok::<(), plt::colors::ColormapError>(())
    /// ```
    pub fn format_cursor_data(&self, value: Option<f64>) -> String {
        format_readout(value.map(|value| vec![(self.norm(), self.cmap().n(), value)]))
    }
}

impl MultiMappable {
    /// One value per variate as a status bar shows them, such as
    /// `[0.557, 3.83]`, each resolved by its norm and its colormap's
    /// colours; `[]` when any is masked.
    ///
    /// ```
    /// use plt::colors::multivar::{MultiColormap, MultiMappable};
    /// let mut mappable = MultiMappable::new(MultiColormap::get("BiPeak")?);
    /// mappable.set_clim(Some(&[Some(0.0), Some(0.0)]), Some(&[Some(1.0), Some(100.0)]))?;
    /// assert_eq!(mappable.format_cursor_data(&[Some(0.5), Some(42.0)]), "[0.500, 42.0]");
    /// assert_eq!(mappable.format_cursor_data(&[Some(0.5), None]), "[]");
    /// # Ok::<(), plt::colors::multivar::MultivarError>(())
    /// ```
    pub fn format_cursor_data(&self, values: &[Option<f64>]) -> String {
        let norms = self.norm().norms().iter().map(AsRef::as_ref);
        let parts = norms
            .zip(self.cmap().sizes())
            .zip(values)
            .map(|((norm, n), value)| value.map(|value| (norm, n, value)))
            .collect();
        format_readout(parts)
    }
}

/// `[v1, v2, ...]` with each value's significant digits, or `[]` when any
/// value is masked (`None`).
fn format_readout(parts: Option<Vec<(&dyn Norm, usize, f64)>>) -> String {
    let Some(parts) = parts else {
        return "[]".to_owned();
    };
    let items: Vec<String> = parts
        .into_iter()
        .map(|(norm, n, value)| pyfmt::general_alternate(value, sig_digits(norm, n, value)))
        .collect();
    format!("[{}]", items.join(", "))
}

/// The significant digits that tell `value` from its neighbours in a
/// colormap of `n` colours under `norm` (matplotlib's
/// `_sig_digits_from_norm`). The norm is asked on a copy, so that the
/// status bar never rescales it.
#[expect(
    clippy::float_cmp,
    reason = "matplotlib singles out exactly equal limits (vmin == vmax)"
)]
fn sig_digits(norm: &dyn Norm, n: usize, value: f64) -> usize {
    let mut norm = norm.clone_box();
    let normed = norm
        .normalize(&[value])
        .ok()
        .and_then(|normed| normed.first().copied());
    let delta = match (normed, norm.kind()) {
        (Some(Normalized::Index(_)), NormKind::Boundary(boundary)) => {
            boundary_step(boundary.boundaries(), value)
        }
        (Some(Normalized::Fraction(normed)), _) if normed.is_finite() => {
            match (norm.vmin(), norm.vmax()) {
                (Some(vmin), Some(vmax)) if vmin == vmax => Some((vmin * 0.1).abs()),
                _ => {
                    let n = n as f64;
                    let step = (normed * n).trunc();
                    norm.inverse(&[step / n, (step + 1.0) / n])
                        .ok()
                        .and_then(|neighbours| {
                            neighbours
                                .into_iter()
                                .flatten()
                                .map(|neighbour| (neighbour - value).abs())
                                .reduce(f64::max)
                        })
                }
            }
        }
        _ => None,
    };
    delta
        .and_then(|delta| g_sig_digits(value, delta).ok())
        .unwrap_or(DEFAULT_DIGITS)
}

/// The widest of the bins next to the boundary nearest `value`
/// (`BoundaryNorm` has no inverse).
fn boundary_step(boundaries: &[f64], value: f64) -> Option<f64> {
    let nearest = boundaries
        .iter()
        .enumerate()
        .fold(None, |best: Option<(usize, f64)>, (i, b)| {
            let distance = (b - value).abs();
            match best {
                Some((_, closest)) if closest <= distance => best,
                _ => Some((i, distance)),
            }
        })?
        .0;
    let neighbours = &boundaries[nearest.saturating_sub(1)..(nearest + 2).min(boundaries.len())];
    neighbours
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .reduce(f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::multivar::MultiColormap;
    use crate::colors::{BoundaryNorm, Extend, LogNorm, MultiNorm, NormSpec, Normalize, colormaps};

    fn viridis(norm: impl Norm + 'static) -> ScalarMappable {
        ScalarMappable::with_norm(colormaps::get("viridis").unwrap(), Box::new(norm))
    }

    #[test]
    fn mapped_values_get_the_digits_of_one_colour_step() {
        let mappable = viridis(Normalize::new(0.0, 1.0));
        // 256 colours: steps of 1/256 need three decimals.
        assert_eq!(mappable.format_cursor_data(Some(0.123_456)), "[0.123]");
        assert_eq!(mappable.format_cursor_data(Some(0.5)), "[0.500]");
        let singular = viridis(Normalize::new(2.0, 2.0));
        assert_eq!(singular.format_cursor_data(Some(2.345)), "[2.3]");
        let binned =
            viridis(BoundaryNorm::new(vec![0.0, 1.0, 5.0, 10.0], 256, Extend::Neither).unwrap());
        assert_eq!(
            binned.format_cursor_data(Some(4.567)),
            "[5.]",
            "bins up to 5 wide"
        );
    }

    #[test]
    fn each_variate_has_its_digits() {
        let norm = MultiNorm::new([
            NormSpec::norm(Normalize::new(-0.73, 1.19)),
            NormSpec::norm(LogNorm::new(0.1, 825.4)),
        ])
        .unwrap();
        let cmap = MultiColormap::get("BiPeak").unwrap();
        let mappable = MultiMappable::with_norm(cmap, norm).unwrap();
        assert_eq!(
            mappable.format_cursor_data(&[Some(0.557), Some(3.83)]),
            "[0.557, 3.83]"
        );
        assert_eq!(mappable.format_cursor_data(&[None, Some(1.0)]), "[]");
    }
}
