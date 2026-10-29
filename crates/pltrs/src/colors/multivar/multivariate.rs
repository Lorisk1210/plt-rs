//! Several colormaps whose colours mix into one (matplotlib's
//! `MultivarColormap`).

use std::str::FromStr;

use pltrs_core::Rgba;

use super::MultivarError;
use crate::colors::colormap::truncate_byte;
use crate::colors::{Color, Colormap, Normalized};

/// How a multivariate colormap mixes the colours of its colormaps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombinationMode {
    /// The colours add up, mixing brighter (`"sRGB_add"`).
    Add,
    /// Their complements add up, mixing darker (`"sRGB_sub"`).
    Sub,
}

impl FromStr for CombinationMode {
    type Err = MultivarError;

    /// matplotlib's names, `"sRGB_add"` and `"sRGB_sub"`.
    fn from_str(name: &str) -> Result<Self, MultivarError> {
        match name {
            "sRGB_add" => Ok(Self::Add),
            "sRGB_sub" => Ok(Self::Sub),
            other => Err(MultivarError::UnknownMode(other.to_owned())),
        }
    }
}

/// One colormap per variate, whose colours mix into one (matplotlib's
/// `MultivarColormap`). Two are equal when their colormaps, mode and bad
/// colour are, whatever their names.
///
/// ```
/// use pltrs::colors::Normalized;
/// use pltrs::colors::multivar::multivar_colormap;
/// let cmap = multivar_colormap("2VarAddA")?;
/// let black = cmap.lookup(&[Normalized::Fraction(0.0), Normalized::Fraction(0.0)], None)?;
/// assert_eq!(black.to_array(), [0.0, 0.0, 0.0, 1.0]);
/// # Ok::<(), pltrs::colors::multivar::MultivarError>(())
/// ```
#[derive(Clone, Debug)]
pub struct MultivarColormap {
    name: String,
    colormaps: Vec<Colormap>,
    mode: CombinationMode,
    bad: Rgba,
}

impl PartialEq for MultivarColormap {
    /// matplotlib's `MultivarColormap.__eq__`: the names do not count.
    fn eq(&self, other: &Self) -> bool {
        self.colormaps == other.colormaps && self.mode == other.mode && self.bad == other.bad
    }
}

impl MultivarColormap {
    /// Mixes `colormaps` (at least two) as `mode` says; points with a bad
    /// value in any variate are transparent.
    pub fn new(
        name: impl Into<String>,
        colormaps: Vec<Colormap>,
        mode: CombinationMode,
    ) -> Result<Self, MultivarError> {
        if colormaps.len() < 2 {
            return Err(MultivarError::TooFewColormaps(colormaps.len()));
        }
        Ok(Self {
            name: name.into(),
            colormaps,
            mode,
            bad: Rgba::TRANSPARENT,
        })
    }

    /// The name of the colormap family.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The colormaps, one per variate.
    pub fn colormaps(&self) -> &[Colormap] {
        &self.colormaps
    }

    /// The number of variates.
    pub fn n_variates(&self) -> usize {
        self.colormaps.len()
    }

    /// How the colours mix.
    pub fn combination_mode(&self) -> CombinationMode {
        self.mode
    }

    /// The colour of points with a bad value.
    pub fn bad(&self) -> Rgba {
        self.bad
    }

    /// The mixed colour of one point, clipped into `[0, 1]`: `values[i]`
    /// for colormap `i`; `alpha` multiplies the opacity (matplotlib's
    /// `cmap((x0, x1, ...), alpha)`).
    pub fn lookup(&self, values: &[Normalized], alpha: Option<f64>) -> Result<Rgba, MultivarError> {
        let [r, g, b, a] = self.mix(values)?.map(|c| c.clamp(0.0, 1.0));
        let a = alpha.map_or(a, |alpha| a * alpha.clamp(0.0, 1.0));
        Ok(Rgba::new(r, g, b, a))
    }

    /// Like [`MultivarColormap::lookup`] but as 8-bit channels, which
    /// matplotlib truncates (`cmap(X, alpha, bytes=True)`).
    pub fn lookup_bytes(
        &self,
        values: &[Normalized],
        alpha: Option<f64>,
    ) -> Result<[u8; 4], MultivarError> {
        Ok(self.lookup(values, alpha)?.to_array().map(truncate_byte))
    }

    /// The mixed colour of one point without clipping (matplotlib's
    /// `cmap(X, clip=False)`), as RGBA channels that may leave `[0, 1]`.
    pub fn mix(&self, values: &[Normalized]) -> Result<[f64; 4], MultivarError> {
        if values.len() != self.colormaps.len() {
            return Err(MultivarError::VariateCount {
                expected: self.colormaps.len(),
                given: values.len(),
            });
        }
        let mut mixed = [0.0, 0.0, 0.0, 1.0];
        for (i, (cmap, &value)) in self.colormaps.iter().zip(values).enumerate() {
            let [r, g, b, a] = cmap.lookup(value, None).to_array();
            if i == 0 {
                mixed = [r, g, b, a];
            } else {
                mixed[0] += r;
                mixed[1] += g;
                mixed[2] += b;
                mixed[3] *= a;
            }
        }
        if self.mode == CombinationMode::Sub {
            let shift = (self.colormaps.len() - 1) as f64;
            mixed[..3].iter_mut().for_each(|c| *c -= shift);
        }
        if values.iter().any(|value| value.is_bad()) {
            mixed = self.bad.to_array();
        }
        Ok(mixed)
    }

    /// A copy with the colours for bad values and, per colormap, for
    /// values below and above the range replaced where given (matplotlib's
    /// `with_extremes`; *under* and *over* colours mix like the others).
    pub fn with_extremes(
        mut self,
        bad: Option<Color>,
        under: Option<Vec<Color>>,
        over: Option<Vec<Color>>,
    ) -> Result<Self, MultivarError> {
        if let Some(bad) = bad {
            self.bad = bad.resolve()?;
        }
        let n = self.colormaps.len();
        for (colors, set) in [
            (under, Colormap::with_under as fn(Colormap, Color) -> _),
            (over, Colormap::with_over),
        ] {
            let Some(colors) = colors else {
                continue;
            };
            if colors.len() != n {
                return Err(MultivarError::VariateCount {
                    expected: n,
                    given: colors.len(),
                });
            }
            self.colormaps = std::mem::take(&mut self.colormaps)
                .into_iter()
                .zip(colors)
                .map(|(cmap, color)| set(cmap, color))
                .collect::<Result<_, _>>()?;
        }
        Ok(self)
    }

    /// A copy whose colormaps have the given numbers of entries, `None`
    /// keeping a colormap as it is (matplotlib's `resampled`).
    pub fn resampled(&self, lutshape: &[Option<usize>]) -> Result<Self, MultivarError> {
        if lutshape.len() != self.colormaps.len() {
            return Err(MultivarError::VariateCount {
                expected: self.colormaps.len(),
                given: lutshape.len(),
            });
        }
        let colormaps = self
            .colormaps
            .iter()
            .zip(lutshape)
            .map(|(cmap, size)| match size {
                Some(size) => cmap.resampled(*size),
                None => Ok(cmap.clone()),
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            colormaps,
            ..self.clone()
        })
    }
}
