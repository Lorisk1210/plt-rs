//! Colours looked up from two values in a 2-D table (matplotlib's
//! `BivarColormap`, `SegmentedBivarColormap` and
//! `BivarColormapFromImage`).

use std::str::FromStr;

use plt_core::Rgba;

use super::MultivarError;
use crate::colors::{Color, Colormap, Normalized};
use crate::numeric::{Grid, linspace};

/// What a bivariate colormap does with value pairs outside its unit
/// square (matplotlib's `shape`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BivarShape {
    /// Each value is clipped into `[0, 1]` (`"square"`).
    #[default]
    Square,
    /// Pairs are pulled radially onto the circle around the centre; the
    /// table shows as a disc (`"circle"`).
    Circle,
    /// Pairs outside the square get the *outside* colour (`"ignore"`).
    Ignore,
    /// Pairs outside the circle get the *outside* colour; the table shows
    /// as a disc (`"circleignore"`).
    CircleIgnore,
}

impl BivarShape {
    fn is_circle(self) -> bool {
        matches!(self, Self::Circle | Self::CircleIgnore)
    }

    fn ignores(self) -> bool {
        matches!(self, Self::Ignore | Self::CircleIgnore)
    }
}

impl FromStr for BivarShape {
    type Err = MultivarError;

    fn from_str(name: &str) -> Result<Self, MultivarError> {
        match name {
            "square" => Ok(Self::Square),
            "circle" => Ok(Self::Circle),
            "ignore" => Ok(Self::Ignore),
            "circleignore" => Ok(Self::CircleIgnore),
            other => Err(MultivarError::UnknownShape(other.to_owned())),
        }
    }
}

/// One of the two axes of a bivariate colormap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BivarAxis {
    /// The first variate, along the table's rows.
    First,
    /// The second variate, along its columns.
    Second,
}

/// Colours looked up from value pairs in an `N × M` table (matplotlib's
/// `BivarColormap`).
///
/// ```
/// use plt::colors::Normalized;
/// use plt::colors::multivar::bivar_colormap;
/// let cmap = bivar_colormap("BiOrangeBlue")?;
/// let corner = cmap.lookup(Normalized::Fraction(1.0), Normalized::Fraction(1.0), None)?;
/// assert_eq!(corner.to_array(), [1.0, 1.0, 1.0, 1.0]);
/// # Ok::<(), plt::colors::multivar::MultivarError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct BivarColormap {
    name: String,
    lut: Grid<[f64; 4]>,
    shape: BivarShape,
    origin: (f64, f64),
    bad: Rgba,
    outside: Rgba,
}

impl BivarColormap {
    /// A table of `n × n` colours interpolated bilinearly from the RGB
    /// grid `patch` (matplotlib's `SegmentedBivarColormap`); `origin` is
    /// the pair, as fractions, where [`BivarColormap::component`] cuts.
    pub fn segmented(
        name: impl Into<String>,
        patch: &Grid<[f64; 3]>,
        n: usize,
        shape: BivarShape,
        origin: (f64, f64),
    ) -> Result<Self, MultivarError> {
        let (rows, cols) = patch.shape();
        if rows == 0 || cols == 0 || n == 0 {
            return Err(MultivarError::EmptyTable);
        }
        let row_at = linspace(0.0, (rows - 1) as f64, n);
        let col_at = linspace(0.0, (cols - 1) as f64, n);
        let lut = Grid::from_fn(n, n, |i, j| {
            let (row, col) = (row_at[i], col_at[j]);
            let (left, top) = (row as usize, col as usize);
            let (row_frac, col_frac) = (row - left as f64, col - top as f64);
            let (right, bottom) = ((left + 1).min(rows - 1), (top + 1).min(cols - 1));
            let (tl, tr) = (patch[(left, top)], patch[(right, top)]);
            let (bl, br) = (patch[(left, bottom)], patch[(right, bottom)]);
            let channel = |c: usize| {
                tl[c] * (1.0 - row_frac) * (1.0 - col_frac)
                    + tr[c] * row_frac * (1.0 - col_frac)
                    + bl[c] * (1.0 - row_frac) * col_frac
                    + br[c] * row_frac * col_frac
            };
            [channel(0), channel(1), channel(2), 1.0]
        });
        Ok(Self::from_table(name.into(), lut, shape, origin))
    }

    /// A colormap with the given table of RGBA colours (matplotlib's
    /// `BivarColormapFromImage`).
    pub fn from_image(
        name: impl Into<String>,
        lut: Grid<[f64; 4]>,
        shape: BivarShape,
        origin: (f64, f64),
    ) -> Result<Self, MultivarError> {
        if lut.is_empty() {
            return Err(MultivarError::EmptyTable);
        }
        Ok(Self::from_table(name.into(), lut, shape, origin))
    }

    fn from_table(
        name: String,
        lut: Grid<[f64; 4]>,
        shape: BivarShape,
        origin: (f64, f64),
    ) -> Self {
        Self {
            name,
            lut,
            shape,
            origin,
            bad: Rgba::TRANSPARENT,
            outside: Rgba::new(1.0, 0.0, 1.0, 1.0),
        }
    }

    /// The name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The table size `(N, M)`.
    pub fn size(&self) -> (usize, usize) {
        self.lut.shape()
    }

    /// What happens to pairs outside the unit square.
    pub fn shape(&self) -> BivarShape {
        self.shape
    }

    /// Where [`BivarColormap::component`] cuts, as fractions.
    pub fn origin(&self) -> (f64, f64) {
        self.origin
    }

    /// The colour of bad values.
    pub fn bad(&self) -> Rgba {
        self.bad
    }

    /// The colour of pairs outside the colormap.
    pub fn outside(&self) -> Rgba {
        self.outside
    }

    /// The table, with the corners outside the disc transparent for
    /// circular shapes (matplotlib's `lut`).
    pub fn lut(&self) -> Grid<[f64; 4]> {
        if !self.shape.is_circle() {
            return self.lut.clone();
        }
        let (n, m) = self.lut.shape();
        let (along_n, along_m) = (linspace(-1.0, 1.0, n), linspace(-1.0, 1.0, m));
        Grid::from_fn(n, m, |i, j| {
            let mut color = self.lut[(i, j)];
            if along_n[i].powi(2) + along_m[j].powi(2) > 1.0 {
                color[3] = 0.0;
            }
            color
        })
    }

    /// The colour of the pair `(x0, x1)` (matplotlib's `cmap((x0, x1),
    /// alpha)`): fractions scale to the table, indices pick from it; the
    /// shape decides what happens beyond it. `alpha` replaces the opacity.
    pub fn lookup(
        &self,
        x0: Normalized,
        x1: Normalized,
        alpha: Option<f64>,
    ) -> Result<Rgba, MultivarError> {
        let (x0, x1) = self.clipped(x0, x1)?;
        let (n, m) = self.lut.shape();
        let (a, outside_a) = scaled(x0, n);
        let (b, outside_b) = scaled(x1, m);
        let bad = x0.is_bad() || x1.is_bad();
        let outside = outside_a || outside_b;
        let mut color = match (a, b) {
            (Some(a), Some(b)) if !bad && !outside => Rgba::from(self.lut[(a, b)]),
            _ => Rgba::TRANSPARENT,
        };
        if outside {
            color = self.outside;
        }
        if bad {
            color = self.bad;
        }
        if let Some(alpha) = alpha {
            color.a = alpha.clamp(0.0, 1.0);
            if bad && self.bad == Rgba::TRANSPARENT {
                color = Rgba::TRANSPARENT;
            }
        }
        Ok(color)
    }

    /// The pair after the shape's clipping (matplotlib's `_clip`).
    fn clipped(
        &self,
        x0: Normalized,
        x1: Normalized,
    ) -> Result<(Normalized, Normalized), MultivarError> {
        let (n, m) = self.lut.shape();
        Ok(match self.shape {
            BivarShape::Square => (clip_square(x0, n), clip_square(x1, m)),
            BivarShape::Ignore => (clip_ignore(x0, n), clip_ignore(x1, m)),
            BivarShape::Circle | BivarShape::CircleIgnore => {
                let (f0, f1) = match (x0, x1) {
                    (Normalized::Fraction(f0), Normalized::Fraction(f1)) => (f0, f1),
                    (Normalized::Index(_), _) | (_, Normalized::Index(_)) => {
                        return Err(MultivarError::CircleNeedsFractions);
                    }
                    // Masked fractions keep their place: they are bad.
                    _ => return Ok((x0, x1)),
                };
                let radius_sqr = (f0 - 0.5).powi(2) + (f1 - 0.5).powi(2);
                if radius_sqr <= 0.25 {
                    (x0, x1)
                } else if self.shape == BivarShape::Circle {
                    let overextend = 2.0 * radius_sqr.sqrt();
                    let pull = |f: f64| Normalized::Fraction((f - 0.5) / overextend + 0.5);
                    (pull(f0), pull(f1))
                } else {
                    (Normalized::Fraction(-1.0), Normalized::Fraction(-1.0))
                }
            }
        })
    }

    /// A copy with the given colours, shape and origin where given
    /// (matplotlib's `with_extremes`).
    pub fn with_extremes(
        mut self,
        bad: Option<Color>,
        outside: Option<Color>,
        shape: Option<BivarShape>,
        origin: Option<(f64, f64)>,
    ) -> Result<Self, MultivarError> {
        if let Some(bad) = bad {
            self.bad = bad.resolve()?;
        }
        if let Some(outside) = outside {
            self.outside = outside.resolve()?;
        }
        self.shape = shape.unwrap_or(self.shape);
        self.origin = origin.unwrap_or(self.origin);
        Ok(self)
    }

    /// A copy whose table is the colormap sampled on `rows × cols` evenly
    /// spaced pairs (matplotlib's `resampled`): a size of `None` (or 1)
    /// keeps the axis's size, a negative one runs the axis backwards
    /// (`-1` with the same size), and `transposed` swaps the axes.
    pub fn resampled(
        &self,
        lutshape: (Option<isize>, Option<isize>),
        transposed: bool,
    ) -> Result<Self, MultivarError> {
        let (n, m) = self.lut.shape();
        let axis = |size: Option<isize>, own: usize| {
            let size = size.unwrap_or(1);
            let count = match size.unsigned_abs() {
                0 => return Err(MultivarError::EmptyTable),
                1 => own,
                count => count,
            };
            Ok((count, size < 0))
        };
        let ((rows, flip_rows), (cols, flip_cols)) = (axis(lutshape.0, n)?, axis(lutshape.1, m)?);
        let at_rows = linspace(0.0, 1.0, rows);
        let at_cols = linspace(0.0, 1.0, cols);
        let square = Self {
            shape: BivarShape::Square,
            ..self.clone()
        };
        let lut = Grid::from_fn(rows, cols, |i, j| {
            let x0 = at_rows[if flip_rows { rows - 1 - i } else { i }];
            let x1 = at_cols[if flip_cols { cols - 1 - j } else { j }];
            let (first, second) = if transposed { (x1, x0) } else { (x0, x1) };
            square
                .lookup(
                    Normalized::Fraction(first),
                    Normalized::Fraction(second),
                    None,
                )
                .expect("square colormaps look up any fraction")
                .to_array()
        });
        let origin = if transposed {
            (self.origin.1, self.origin.0)
        } else {
            self.origin
        };
        let mut resampled = Self::from_table(self.name.clone(), lut, self.shape, origin);
        resampled.bad = self.bad;
        resampled.outside = self.outside;
        Ok(resampled)
    }

    /// A copy running backwards along the chosen axes (matplotlib's
    /// `reversed`).
    pub fn reversed(&self, first: bool, second: bool) -> Result<Self, MultivarError> {
        let step = |reverse: bool| Some(if reverse { -1 } else { 1 });
        self.resampled((step(first), step(second)), false)
    }

    /// A copy with the axes swapped (matplotlib's `transposed`).
    pub fn transposed(&self) -> Result<Self, MultivarError> {
        self.resampled((None, None), true)
    }

    /// The 1-D colormap along `axis` through the origin (matplotlib's
    /// `cmap[0]` and `cmap[1]`); with an ignoring shape its under and
    /// over colours are the outside colour.
    pub fn component(&self, axis: BivarAxis) -> Result<Colormap, MultivarError> {
        let (n, m) = self.lut.shape();
        let cut = |fraction: f64, len: usize| ((fraction * len as f64) as usize).min(len - 1);
        let (suffix, colors): (&str, Vec<Rgba>) = match axis {
            BivarAxis::First => {
                let col = cut(self.origin.1, m);
                (
                    "0",
                    (0..n).map(|i| Rgba::from(self.lut[(i, col)])).collect(),
                )
            }
            BivarAxis::Second => {
                let row = cut(self.origin.0, n);
                (
                    "1",
                    (0..m).map(|j| Rgba::from(self.lut[(row, j)])).collect(),
                )
            }
        };
        let cmap =
            Colormap::listed(format!("{}_{suffix}", self.name), colors)?.with_bad(self.bad)?;
        Ok(if self.shape.ignores() {
            cmap.with_under(self.outside)?.with_over(self.outside)?
        } else {
            cmap
        })
    }
}

/// A value scaled to a table axis of `len` entries: its index, and
/// whether it falls outside the table (NaN does not).
#[expect(
    clippy::float_cmp,
    reason = "matplotlib moves exactly the table size onto the last entry"
)]
fn scaled(value: Normalized, len: usize) -> (Option<usize>, bool) {
    let len_f = len as f64;
    let x = match value {
        Normalized::Fraction(f) => {
            let x = f * len_f;
            if x == len_f { len_f - 1.0 } else { x }
        }
        Normalized::Index(i) => i as f64,
        Normalized::Masked => return (None, false),
    };
    let outside = x < 0.0 || x >= len_f;
    ((!outside && !x.is_nan()).then_some(x as usize), outside)
}

/// matplotlib's `"square"` clipping of one variate.
fn clip_square(value: Normalized, len: usize) -> Normalized {
    match value {
        Normalized::Fraction(f) if f < 0.0 => Normalized::Fraction(0.0),
        Normalized::Fraction(f) if f > 1.0 => Normalized::Fraction(1.0),
        Normalized::Index(i) if i < 0 => Normalized::Index(0),
        Normalized::Index(i) if i >= len as i64 => Normalized::Index(len as i64 - 1),
        other => other,
    }
}

/// matplotlib's `"ignore"` marking of one variate outside its range.
fn clip_ignore(value: Normalized, len: usize) -> Normalized {
    match value {
        Normalized::Fraction(f) if f < 0.0 || f > 1.0 => Normalized::Fraction(-1.0),
        Normalized::Index(i) if i < 0 || i >= len as i64 => Normalized::Index(-1),
        other => other,
    }
}
