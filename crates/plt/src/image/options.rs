//! How an image is placed and resampled: interpolation, interpolation
//! stage, origin, extent and per-pixel opacity.

use std::fmt;
use std::str::FromStr;

use plt_raster::resample::Interpolation as Kernel;

use crate::numeric::Grid;

/// How image pixels are interpolated when resampled to the screen
/// (matplotlib's `interpolation`).
///
/// [`Interpolation::Auto`] and [`Interpolation::Antialiased`] pick
/// nearest-neighbour sampling when an image is magnified more than three
/// times or by exactly one or two, and a Hanning filter otherwise;
/// [`Interpolation::None`] samples nearest-neighbour too, but without the
/// tie-breaking towards the left/upper pixel that `Nearest` applies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Interpolation {
    /// Nearest or Hanning, depending on the magnification.
    #[default]
    Auto,
    /// The same as [`Interpolation::Auto`] (its former name).
    Antialiased,
    /// No interpolation; nearest-neighbour on raster output.
    None,
    /// Nearest-neighbour sampling.
    Nearest,
    /// Tent filter.
    Bilinear,
    /// Cubic B-spline-like filter.
    Bicubic,
    /// 16-tap spline.
    Spline16,
    /// 36-tap spline.
    Spline36,
    /// Hanning window.
    Hanning,
    /// Hamming window.
    Hamming,
    /// Hermite cubic.
    Hermite,
    /// Kaiser window.
    Kaiser,
    /// Quadratic B-spline.
    Quadric,
    /// Catmull–Rom spline.
    Catrom,
    /// Gaussian.
    Gaussian,
    /// Jinc (Bessel) filter.
    Bessel,
    /// Mitchell–Netravali filter.
    Mitchell,
    /// Unwindowed sinc (radius from `filterrad`).
    Sinc,
    /// Lanczos-windowed sinc (radius from `filterrad`).
    Lanczos,
    /// Blackman-windowed sinc (radius from `filterrad`).
    Blackman,
}

impl Interpolation {
    /// Every interpolation, in matplotlib's documentation order.
    pub const ALL: [Self; 20] = [
        Self::None,
        Self::Nearest,
        Self::Bilinear,
        Self::Bicubic,
        Self::Spline16,
        Self::Spline36,
        Self::Hanning,
        Self::Hamming,
        Self::Hermite,
        Self::Kaiser,
        Self::Quadric,
        Self::Catrom,
        Self::Gaussian,
        Self::Bessel,
        Self::Mitchell,
        Self::Sinc,
        Self::Lanczos,
        Self::Blackman,
        Self::Antialiased,
        Self::Auto,
    ];

    /// matplotlib's name (`"spline16"`, `"auto"`, ...).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Antialiased => "antialiased",
            Self::None => "none",
            Self::Nearest => "nearest",
            Self::Bilinear => "bilinear",
            Self::Bicubic => "bicubic",
            Self::Spline16 => "spline16",
            Self::Spline36 => "spline36",
            Self::Hanning => "hanning",
            Self::Hamming => "hamming",
            Self::Hermite => "hermite",
            Self::Kaiser => "kaiser",
            Self::Quadric => "quadric",
            Self::Catrom => "catrom",
            Self::Gaussian => "gaussian",
            Self::Bessel => "bessel",
            Self::Mitchell => "mitchell",
            Self::Sinc => "sinc",
            Self::Lanczos => "lanczos",
            Self::Blackman => "blackman",
        }
    }

    /// Whether the kernel is chosen from the magnification at draw time.
    pub const fn is_adaptive(self) -> bool {
        matches!(self, Self::Auto | Self::Antialiased)
    }

    /// The resampling kernel (matplotlib's `_interpd_`); the adaptive
    /// choices and `none` map to nearest-neighbour.
    pub(crate) const fn kernel(self) -> Kernel {
        match self {
            Self::Auto | Self::Antialiased | Self::None | Self::Nearest => Kernel::Nearest,
            Self::Bilinear => Kernel::Bilinear,
            Self::Bicubic => Kernel::Bicubic,
            Self::Spline16 => Kernel::Spline16,
            Self::Spline36 => Kernel::Spline36,
            Self::Hanning => Kernel::Hanning,
            Self::Hamming => Kernel::Hamming,
            Self::Hermite => Kernel::Hermite,
            Self::Kaiser => Kernel::Kaiser,
            Self::Quadric => Kernel::Quadric,
            Self::Catrom => Kernel::Catrom,
            Self::Gaussian => Kernel::Gaussian,
            Self::Bessel => Kernel::Bessel,
            Self::Mitchell => Kernel::Mitchell,
            Self::Sinc => Kernel::Sinc,
            Self::Lanczos => Kernel::Lanczos,
            Self::Blackman => Kernel::Blackman,
        }
    }
}

impl fmt::Display for Interpolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A name that is not an interpolation, stage or origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownName {
    /// What was being parsed (`"interpolation"`, ...).
    pub kind: &'static str,
    /// The unknown name.
    pub name: String,
}

impl fmt::Display for UnknownName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a valid {}", self.name, self.kind)
    }
}

impl std::error::Error for UnknownName {}

impl FromStr for Interpolation {
    type Err = UnknownName;

    /// Parses matplotlib's names, ignoring case as matplotlib does.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|i| i.name() == lower)
            .ok_or_else(|| UnknownName {
                kind: "interpolation",
                name: s.to_owned(),
            })
    }
}

/// Whether interpolation happens on the data or on the colours
/// (matplotlib's `interpolation_stage`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InterpolationStage {
    /// Colours when the image is magnified less than three times, data
    /// otherwise.
    #[default]
    Auto,
    /// Resample the data, then map it to colours.
    Data,
    /// Map the data to colours, then resample the colours.
    Rgba,
}

impl FromStr for InterpolationStage {
    type Err = UnknownName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" | "antialiased" => Ok(Self::Auto),
            "data" => Ok(Self::Data),
            "rgba" => Ok(Self::Rgba),
            _ => Err(UnknownName {
                kind: "interpolation stage",
                name: s.to_owned(),
            }),
        }
    }
}

impl fmt::Display for InterpolationStage {
    /// matplotlib's name (`get_interpolation_stage`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::Data => "data",
            Self::Rgba => "rgba",
        })
    }
}

/// Which corner the first row and column of an image array sit in
/// (matplotlib's `origin`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Row 0 at the top (matrix and photo convention).
    #[default]
    Upper,
    /// Row 0 at the bottom.
    Lower,
}

impl FromStr for Origin {
    type Err = UnknownName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "upper" => Ok(Self::Upper),
            "lower" => Ok(Self::Lower),
            _ => Err(UnknownName {
                kind: "origin",
                name: s.to_owned(),
            }),
        }
    }
}

/// The data rectangle an image covers: `(left, right, bottom, top)` edges
/// of its outer pixels (matplotlib's `extent`). `left > right` or
/// `bottom > top` flip the image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    /// Data x of the left edge of the first column.
    pub left: f64,
    /// Data x of the right edge of the last column.
    pub right: f64,
    /// Data y of the bottom edge.
    pub bottom: f64,
    /// Data y of the top edge.
    pub top: f64,
}

impl Extent {
    /// The extent `(left, right, bottom, top)`.
    pub const fn new(left: f64, right: f64, bottom: f64, top: f64) -> Self {
        Self {
            left,
            right,
            bottom,
            top,
        }
    }

    /// matplotlib's default extent: pixel centres at integer indices,
    /// row 0 at the top for [`Origin::Upper`].
    pub fn default_for(rows: usize, cols: usize, origin: Origin) -> Self {
        let (rows, cols) = (rows as f64, cols as f64);
        match origin {
            Origin::Upper => Self::new(-0.5, cols - 0.5, rows - 0.5, -0.5),
            Origin::Lower => Self::new(-0.5, cols - 0.5, -0.5, rows - 0.5),
        }
    }
}

impl From<(f64, f64, f64, f64)> for Extent {
    fn from((left, right, bottom, top): (f64, f64, f64, f64)) -> Self {
        Self::new(left, right, bottom, top)
    }
}

/// The opacity of an image (matplotlib's `alpha`: a scalar or an array).
#[derive(Clone, Debug, PartialEq)]
pub enum ImageAlpha {
    /// One opacity in `[0, 1]` for the whole image.
    Uniform(f64),
    /// One opacity per pixel, with the image's shape.
    Each(Grid<f64>),
}

impl ImageAlpha {
    /// The opacity applied to the image as a whole (matplotlib's
    /// `_get_scalar_alpha`): the uniform value, or 1 for per-pixel alpha.
    pub fn scalar(&self) -> f64 {
        match self {
            Self::Uniform(alpha) => *alpha,
            Self::Each(_) => 1.0,
        }
    }
}

impl From<f64> for ImageAlpha {
    fn from(alpha: f64) -> Self {
        Self::Uniform(alpha)
    }
}

impl From<Grid<f64>> for ImageAlpha {
    fn from(alpha: Grid<f64>) -> Self {
        Self::Each(alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for interpolation in Interpolation::ALL {
            assert_eq!(interpolation.name().parse(), Ok(interpolation));
        }
        assert_eq!("Bilinear".parse(), Ok(Interpolation::Bilinear));
        assert!("cubic".parse::<Interpolation>().is_err());
        assert_eq!(Interpolation::Sinc.to_string(), "sinc");
        assert!(Interpolation::Antialiased.is_adaptive());
        assert_eq!(Interpolation::None.kernel(), Kernel::Nearest);
    }

    #[test]
    fn stages_and_origins_parse() {
        assert_eq!("rgba".parse(), Ok(InterpolationStage::Rgba));
        assert_eq!("lower".parse(), Ok(Origin::Lower));
        let err = "middle".parse::<Origin>().unwrap_err();
        assert_eq!(err.to_string(), "\"middle\" is not a valid origin");
    }

    #[test]
    fn default_extents_centre_pixels_on_indices() {
        assert_eq!(
            Extent::default_for(2, 3, Origin::Upper),
            Extent::new(-0.5, 2.5, 1.5, -0.5)
        );
        assert_eq!(
            Extent::default_for(2, 3, Origin::Lower),
            Extent::new(-0.5, 2.5, -0.5, 1.5)
        );
    }

    #[test]
    fn scalar_alpha() {
        assert_eq!(ImageAlpha::from(0.25).scalar(), 0.25);
        assert_eq!(ImageAlpha::from(Grid::filled(1, 1, 0.5)).scalar(), 1.0);
    }
}
