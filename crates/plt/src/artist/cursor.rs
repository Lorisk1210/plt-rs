//! What a status bar shows of the data under the pointer (matplotlib's
//! `Artist.format_cursor_data` and the colour-mapped override of
//! `ColorizingArtist`).

use crate::colors::{NormKind, Normalized, ScalarMappable};
use crate::ticker::{g_sig_digits, pyfmt};

/// The digits matplotlib falls back to when a value's resolution is
/// unknown.
const DEFAULT_DIGITS: usize = 3;

/// Plain values as `[v1, v2]`, three significant digits each (matplotlib's
/// `Artist.format_cursor_data`).
pub(crate) fn format_values(values: impl IntoIterator<Item = f64>) -> String {
    let items: Vec<String> = values
        .into_iter()
        .map(|value| pyfmt::general(value, DEFAULT_DIGITS))
        .collect();
    format!("[{}]", items.join(", "))
}

/// A colour-mapped value with as many digits as one colour step of the
/// colormap resolves; `[]` when it is masked (matplotlib's
/// `_format_cursor_data_override`).
pub(crate) fn format_mapped(mappable: &ScalarMappable, value: Option<f64>) -> String {
    match value {
        Some(value) => format!(
            "[{}]",
            pyfmt::general_alternate(value, sig_digits(mappable, value))
        ),
        None => "[]".to_owned(),
    }
}

/// The significant digits that tell `value` from its neighbours in the
/// colormap (matplotlib's `_sig_digits_from_norm`). The norm is asked on
/// a copy, so that the status bar never rescales it.
#[expect(
    clippy::float_cmp,
    reason = "matplotlib singles out exactly equal limits (vmin == vmax)"
)]
fn sig_digits(mappable: &ScalarMappable, value: f64) -> usize {
    let mut norm = mappable.norm().clone_box();
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
                    let n = mappable.cmap().n() as f64;
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
    use crate::colors::{BoundaryNorm, Extend, Normalize, colormaps};

    fn viridis(norm: impl crate::colors::Norm + 'static) -> ScalarMappable {
        ScalarMappable::with_norm(colormaps::get("viridis").unwrap(), Box::new(norm))
    }

    #[test]
    fn values_get_three_digits() {
        assert_eq!(
            format_values([0.123_456, 255.0, 1234.0]),
            "[0.123, 255, 1.23e+03]"
        );
        assert_eq!(format_values([]), "[]");
    }

    #[test]
    fn mapped_values_get_the_digits_of_one_colour_step() {
        let mappable = viridis(Normalize::new(0.0, 1.0));
        // 256 colours: steps of 1/256 need three decimals.
        assert_eq!(format_mapped(&mappable, Some(0.123_456)), "[0.123]");
        assert_eq!(format_mapped(&mappable, Some(0.5)), "[0.500]");
        assert_eq!(format_mapped(&mappable, None), "[]");
        let singular = viridis(Normalize::new(2.0, 2.0));
        assert_eq!(format_mapped(&singular, Some(2.345)), "[2.3]");
        let binned =
            viridis(BoundaryNorm::new(vec![0.0, 1.0, 5.0, 10.0], 256, Extend::Neither).unwrap());
        assert_eq!(
            format_mapped(&binned, Some(4.567)),
            "[5.]",
            "bins up to 5 wide"
        );
    }
}
