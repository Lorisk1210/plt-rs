//! Evaluating a colormap (matplotlib's `Colormap.__call__`).

use plt_core::Rgba;
use plt_core::numeric::np_clip;

use super::{Colormap, ColormapError};
use crate::colors::Alpha;

/// A value ready for colormap lookup: the output of a norm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Normalized {
    /// A position along the colormap: `[0, 1]` spans the colours, values
    /// below 0 are *under*, above 1 *over*, and NaN is *bad*.
    Fraction(f64),
    /// A direct index into the colours (`BoundaryNorm`): negative values
    /// are *under* and indices `>= N` *over*.
    Index(i64),
    /// A missing (masked) value, drawn in the *bad* colour.
    Masked,
}

impl From<f64> for Normalized {
    fn from(x: f64) -> Self {
        Self::Fraction(x)
    }
}

impl From<Option<f64>> for Normalized {
    fn from(x: Option<f64>) -> Self {
        x.map_or(Self::Masked, Self::Fraction)
    }
}

impl Normalized {
    /// The fraction, if this is one.
    pub fn fraction(self) -> Option<f64> {
        match self {
            Self::Fraction(x) => Some(x),
            Self::Index(_) | Self::Masked => None,
        }
    }

    /// Whether the value is missing or NaN (drawn in the bad colour).
    pub fn is_bad(self) -> bool {
        match self {
            Self::Fraction(x) => x.is_nan(),
            Self::Index(_) => false,
            Self::Masked => true,
        }
    }
}

impl Colormap {
    /// The colour at `x` (a fraction of the range; see
    /// [`Normalized::Fraction`]).
    ///
    /// Like matplotlib, `x` selects entry `floor(x * N)`, with `x = 1`
    /// mapping to the last colour.
    pub fn sample(&self, x: f64) -> Rgba {
        self.lut[self.lut_index(Normalized::Fraction(x))]
    }

    /// The colour of table entry `index` (under below 0, over from `N`).
    pub fn sample_index(&self, index: i64) -> Rgba {
        self.lut[self.lut_index(Normalized::Index(index))]
    }

    /// The colour for a normalised value, with an optional alpha override
    /// (clipped to `[0, 1]`). With an override, bad values stay fully
    /// transparent if the bad colour is.
    pub fn lookup(&self, value: Normalized, alpha: Option<f64>) -> Rgba {
        let color = self.lut[self.lut_index(value)];
        match alpha {
            None => color,
            Some(_) if value.is_bad() && self.bad().to_array() == [0.0; 4] => Rgba::TRANSPARENT,
            Some(a) => color.with_alpha(np_clip(a, 0.0, 1.0)),
        }
    }

    /// Like [`lookup`](Self::lookup) but as 8-bit channels, which matplotlib
    /// truncates (`(value * 255).astype(uint8)`).
    pub fn lookup_bytes(&self, value: Normalized, alpha: Option<f64>) -> [u8; 4] {
        let [r, g, b, a] = self.lut[self.lut_index(value)]
            .to_array()
            .map(truncate_byte);
        match alpha {
            None => [r, g, b, a],
            Some(_) if value.is_bad() && self.bad().to_array().map(truncate_byte) == [0; 4] => {
                [0; 4]
            }
            Some(alpha) => [r, g, b, truncate_byte(np_clip(alpha, 0.0, 1.0))],
        }
    }

    /// Colours for many values (matplotlib's `cmap(X, alpha)`).
    pub fn map(&self, values: &[Normalized], alpha: &Alpha) -> Result<Vec<Rgba>, ColormapError> {
        self.map_with(values, alpha, Self::lookup)
    }

    /// 8-bit colours for many values (matplotlib's `cmap(X, alpha,
    /// bytes=True)`).
    pub fn map_bytes(
        &self,
        values: &[Normalized],
        alpha: &Alpha,
    ) -> Result<Vec<[u8; 4]>, ColormapError> {
        self.map_with(values, alpha, Self::lookup_bytes)
    }

    fn map_with<T>(
        &self,
        values: &[Normalized],
        alpha: &Alpha,
        lookup: impl Fn(&Self, Normalized, Option<f64>) -> T,
    ) -> Result<Vec<T>, ColormapError> {
        match alpha {
            Alpha::Keep => Ok(values.iter().map(|&v| lookup(self, v, None)).collect()),
            Alpha::Uniform(a) => Ok(values.iter().map(|&v| lookup(self, v, Some(*a))).collect()),
            Alpha::Each(alphas) if alphas.len() == values.len() => Ok(values
                .iter()
                .zip(alphas)
                .map(|(&v, &a)| lookup(self, v, Some(a)))
                .collect()),
            Alpha::Each(alphas) => Err(ColormapError::AlphaCount {
                values: values.len(),
                alphas: alphas.len(),
            }),
        }
    }

    /// The LUT row for a value, following `_get_rgba_and_mask`: fractions
    /// scale by `N` (with `N` itself pulled back to `N - 1`) and truncate.
    fn lut_index(&self, value: Normalized) -> usize {
        let n = self.n();
        let (under, over, bad) = (n, n + 1, n + 2);
        match value {
            Normalized::Masked => bad,
            Normalized::Index(i) if i < 0 => under,
            Normalized::Index(i) => usize::try_from(i).ok().filter(|&i| i < n).unwrap_or(over),
            Normalized::Fraction(x) => {
                let scaled = x * n as f64;
                let scaled = if scaled == n as f64 {
                    scaled - 1.0
                } else {
                    scaled
                };
                if scaled.is_nan() {
                    bad
                } else if scaled < 0.0 {
                    under
                } else if scaled >= n as f64 {
                    over
                } else {
                    scaled as usize
                }
            }
        }
    }
}

/// numpy's `float -> uint8` cast of `value * 255` (truncating; NaN is 0).
pub(crate) fn truncate_byte(value: f64) -> u8 {
    (value * 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::rgb_colormap;

    #[test]
    fn float_and_index_rules() {
        let cmap = rgb_colormap();
        let lut = cmap.lut().to_vec();
        assert_eq!(cmap.sample(0.0), lut[0]);
        assert_eq!(cmap.sample(0.34), lut[1]);
        assert_eq!(cmap.sample(1.0), lut[2], "x = 1 maps to the last colour");
        assert_eq!(cmap.sample(-1e-12), cmap.under());
        assert_eq!(cmap.sample(1.0 + 1e-12), cmap.over());
        assert_eq!(cmap.sample(f64::NAN), cmap.bad());
        assert_eq!(cmap.sample(f64::INFINITY), cmap.over());
        assert_eq!(cmap.sample_index(2), lut[2]);
        assert_eq!(cmap.sample_index(3), cmap.over());
        assert_eq!(cmap.sample_index(-1), cmap.under());
        assert_eq!(cmap.lookup(Normalized::Masked, None), cmap.bad());
    }

    #[test]
    fn alpha_overrides_and_bytes() {
        let cmap = rgb_colormap();
        assert_eq!(cmap.lookup(0.0.into(), Some(1.5)).a, 1.0);
        assert_eq!(cmap.lookup(f64::NAN.into(), Some(0.5)), Rgba::TRANSPARENT);
        assert_eq!(cmap.lookup_bytes(0.5.into(), Some(0.5)), [0, 128, 0, 127]);
        assert_eq!(
            cmap.map(&[0.0.into(), 1.0.into()], &Alpha::Each(vec![0.1])),
            Err(ColormapError::AlphaCount {
                values: 2,
                alphas: 1
            })
        );
        let grey_bad = cmap.with_bad("0.5").unwrap();
        assert_eq!(grey_bad.lookup(Normalized::Masked, Some(0.25)).a, 0.25);
    }
}
