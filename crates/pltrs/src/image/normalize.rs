//! Normalising the scalars of an image the way numpy's dtypes make
//! matplotlib's norms compute: integers under an identity norm index the
//! colormap, and `float32` data passes through `Normalize` and the
//! colormap lookup in single precision.

use pltrs_core::numeric::np_clip;

use super::{Precision, ScalarKind};
use crate::colors::{NormError, NormKind, Normalized, ScalarMappable};

/// The norm applied to scalar image data. Integer data under an identity
/// norm (matplotlib's `NoNorm`) indexes the colormap, after numpy's cast
/// back to the data's dtype: booleans become 0 or 1, integers truncate.
pub(crate) fn normalize(
    mappable: &mut ScalarMappable,
    values: &[Option<f64>],
    kind: ScalarKind,
) -> Result<Vec<Normalized>, NormError> {
    let floats = matches!(kind, ScalarKind::Float | ScalarKind::Float32);
    if floats || !mappable.norm().is_identity() {
        return mappable.normalize(values);
    }
    Ok(values
        .iter()
        .map(|v| {
            v.map_or(Normalized::Masked, |v| {
                Normalized::Index(match kind {
                    ScalarKind::Boolean => i64::from(v != 0.0),
                    ScalarKind::Integer | ScalarKind::Float | ScalarKind::Float32 => v as i64,
                })
            })
        })
        .collect())
}

/// [`normalize`] for values numpy holds in `precision`. A `float32` array
/// under a linear `Normalize` that does not clip is normalised as numpy
/// does it — each step computed against the `float64` limits and rounded
/// back to `float32` — and its colormap rows are found by multiplying in
/// `float32`; the results are colormap indices. Other norms, and clipping
/// (whose `np.clip` widens to `float64`), compute in double precision.
pub(crate) fn normalize_in(
    mappable: &mut ScalarMappable,
    values: &[Option<f64>],
    kind: ScalarKind,
    precision: Precision,
) -> Result<Vec<Normalized>, NormError> {
    let norm = mappable.norm_mut();
    let single =
        precision == Precision::Single && matches!(norm.kind(), NormKind::Linear) && !norm.clip();
    if single && !norm.scaled() {
        norm.autoscale_none(values);
    }
    let limits = norm.vmin().zip(norm.vmax()).filter(|(lo, hi)| lo < hi);
    let (Some((lo, hi)), true) = (limits, single) else {
        return normalize(mappable, values, kind);
    };
    let n = mappable.cmap().n();
    Ok(values
        .iter()
        .map(|v| {
            v.map_or(Normalized::Masked, |v| {
                let shifted = (v - lo) as f32;
                single_row((f64::from(shifted) / (hi - lo)) as f32, n)
            })
        })
        .collect())
}

/// The colormap rows of `values` (those `mask` flags masked) under a
/// linear `Normalize` in double precision: [`normalize_in`] and the
/// colormap's lookup in one pass, without their intermediate arrays.
/// `None` for other norms, for single precision and for limits that are
/// not increasing, which take the general path.
pub(crate) fn linear_rows(
    mappable: &mut ScalarMappable,
    values: &[f64],
    mask: &[bool],
    precision: Precision,
) -> Option<Vec<u32>> {
    let norm = mappable.norm_mut();
    let clip = norm.clip();
    if !matches!(norm.kind(), NormKind::Linear) || (precision == Precision::Single && !clip) {
        return None;
    }
    if !norm.scaled() {
        norm.autoscale_none_unmasked(values, mask);
    }
    let (lo, hi) = norm.vmin().zip(norm.vmax()).filter(|(lo, hi)| lo < hi)?;
    let cmap = mappable.cmap();
    let bad = cmap.lut_index(Normalized::Masked) as u32;
    Some(
        values
            .iter()
            .zip(mask)
            .map(|(&v, &masked)| {
                if masked {
                    return bad;
                }
                let v = if clip { np_clip(v, lo, hi) } else { v };
                cmap.lut_index(Normalized::Fraction((v - lo) / (hi - lo))) as u32
            })
            .collect(),
    )
}

/// The colormap row of a `float32` fraction among `n` colours, as
/// matplotlib's `_get_rgba_and_mask` finds it: scaled by `n` in single
/// precision, `n` itself pulled back to the last row, NaN bad and values
/// beyond the ends under or over.
#[expect(
    clippy::float_cmp,
    reason = "matplotlib pulls exactly `n` back onto the last row"
)]
fn single_row(fraction: f32, n: usize) -> Normalized {
    let size = n as f32;
    let scaled = fraction * size;
    if scaled.is_nan() {
        Normalized::Masked
    } else if scaled < 0.0 {
        Normalized::Index(-1)
    } else if scaled == size {
        Normalized::Index(n as i64 - 1)
    } else if scaled > size {
        Normalized::Index(n as i64)
    } else {
        Normalized::Index(scaled as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{Colormap, NoNorm, Normalize, colormaps};

    fn gray(norm: Normalize) -> ScalarMappable {
        ScalarMappable::with_norm(colormaps::get("gray").unwrap(), Box::new(norm))
    }

    #[test]
    fn identity_norms_index_with_integers() {
        let cmap = Colormap::listed("rgb", vec!["r", "g", "b"]).unwrap();
        let mut mappable = ScalarMappable::with_norm(cmap, Box::new(NoNorm::default()));
        let values = [Some(0.0), Some(2.9), None];
        assert_eq!(
            normalize(&mut mappable, &values, ScalarKind::Integer).unwrap(),
            [
                Normalized::Index(0),
                Normalized::Index(2),
                Normalized::Masked
            ]
        );
        assert_eq!(
            normalize(&mut mappable, &[Some(0.5)], ScalarKind::Boolean).unwrap(),
            [Normalized::Index(1)]
        );
        for kind in [ScalarKind::Float, ScalarKind::Float32] {
            assert_eq!(
                normalize(&mut mappable, &[Some(1.0)], kind).unwrap(),
                [Normalized::Fraction(1.0)]
            );
        }
    }

    #[test]
    fn single_precision_rounds_every_step() {
        // (0.125 - 0.1) / 0.8 = 1/32 in exact arithmetic: float64 lands
        // just below and picks row 7, float32 rounds to 1/32 → row 8.
        let value = 0.125;
        let mut double = gray(Normalize::new(0.1, 0.9));
        let fraction = double.normalize(&[Some(value)]).unwrap()[0];
        let mut single = gray(Normalize::new(0.1, 0.9));
        let row = normalize_in(
            &mut single,
            &[Some(value)],
            ScalarKind::Float32,
            Precision::Single,
        )
        .unwrap()[0];
        assert_eq!(row, Normalized::Index(8));
        assert!(matches!(fraction, Normalized::Fraction(f) if (f * 256.0) < 8.0));
        let mut clipped = gray(Normalize::new(0.1, 0.9).with_clip(true));
        let widened = normalize_in(
            &mut clipped,
            &[Some(value)],
            ScalarKind::Float32,
            Precision::Single,
        );
        assert_eq!(widened.unwrap()[0], fraction, "clipping widens to float64");
    }

    #[test]
    fn single_rows_follow_the_lookup_rules() {
        assert_eq!(single_row(1.0, 256), Normalized::Index(255));
        assert_eq!(single_row(-0.001, 256), Normalized::Index(-1));
        assert_eq!(single_row(1.001, 256), Normalized::Index(256));
        assert_eq!(single_row(f32::NAN, 256), Normalized::Masked);
        let mut autoscaled = gray(Normalize::default());
        let rows = normalize_in(
            &mut autoscaled,
            &[Some(2.0), None, Some(4.0)],
            ScalarKind::Float32,
            Precision::Single,
        )
        .unwrap();
        assert_eq!(
            rows,
            [
                Normalized::Index(0),
                Normalized::Masked,
                Normalized::Index(255)
            ]
        );
    }
}
