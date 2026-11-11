//! Colormaps: lookup tables from normalised values to colours
//! (matplotlib's `Colormap`, `ListedColormap` and
//! `LinearSegmentedColormap`).
//!
//! A [`Colormap`] holds `N` colours plus three extra colours for values
//! below the range (*under*), above it (*over*) and missing (*bad*).
//! Listed maps take their `N` colours verbatim; linearly segmented maps
//! sample per-channel [`SegmentData`] exactly like matplotlib's
//! `_create_lookup_table`, including its gamma correction.
//!
//! ```
//! use pltrs::colors::Colormap;
//! let cmap = Colormap::from_list("fire", vec!["black", "red", "yellow"], 256, 1.0)?
//!     .with_over("white")?;
//! assert_eq!(cmap.sample(0.0), cmap.lut()[0]);
//! assert_eq!(cmap.sample(1.5), pltrs_core::Rgba::WHITE);
//! # Ok::<(), pltrs::colors::ColormapError>(())
//! ```

#![expect(
    clippy::float_cmp,
    reason = "matplotlib's colormap lookups select entries by exact float comparisons"
)]

mod error;
mod lookup;
mod program;
mod segment;

use std::fmt;
use std::rc::Rc;

use pltrs_core::Rgba;

use super::{Alpha, Color, Colors, to_rgba_array};
use crate::numeric::linspace;
use crate::rc::keyword_names;

pub use error::{ColormapError, SegmentProblem};
pub use lookup::Normalized;
pub(crate) use lookup::truncate_byte;
pub use program::{ColormapCall, ProgramColormap};
pub use segment::{Segment, SegmentData, SegmentFn, create_lookup_table};
pub(crate) use segment::{SegmentRecipe, cubehelix_channel};

/// A colour channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Red.
    Red,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Opacity.
    Alpha,
}

impl Channel {
    /// matplotlib's segment-data key (`"red"`, ...).
    pub fn name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Alpha => "alpha",
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Which out-of-range regions get their own colour (colorbar triangles,
/// `BoundaryNorm` bins): matplotlib's `extend` keyword.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Extend {
    /// No extensions.
    #[default]
    Neither,
    /// Both ends.
    Both,
    /// Below the range only.
    Min,
    /// Above the range only.
    Max,
}

pltrs_core::archive_enum!(Extend {
    Neither,
    Both,
    Min,
    Max,
});

impl Extend {
    const ALL: [Self; 4] = [Self::Neither, Self::Both, Self::Min, Self::Max];

    /// matplotlib's name (`neither`, `both`, `min` or `max`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Neither => "neither",
            Self::Both => "both",
            Self::Min => "min",
            Self::Max => "max",
        }
    }

    /// Whether values below the range are extended.
    pub fn extends_min(self) -> bool {
        matches!(self, Self::Min | Self::Both)
    }

    /// Whether values above the range are extended.
    pub fn extends_max(self) -> bool {
        matches!(self, Self::Max | Self::Both)
    }
}

keyword_names!(Extend, "value for extend", Self::ALL);

/// matplotlib's colormap classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColormapKind {
    /// `ListedColormap`: an explicit list of colours.
    Listed,
    /// `LinearSegmentedColormap`: per-channel segment data.
    LinearSegmented,
}

#[derive(Clone, Debug)]
enum Source {
    Listed(Vec<Rgba>),
    Segmented { data: SegmentData, gamma: f64 },
}

pltrs_core::archive_enum!(Source {
    Listed(v0),
    Segmented { data, gamma },
});

/// The user-set colours for out-of-range and missing values.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Extremes {
    bad: Rgba,
    under: Option<Rgba>,
    over: Option<Rgba>,
}

pltrs_core::archive_struct!(Extremes { bad, under, over });

impl Default for Extremes {
    fn default() -> Self {
        Self {
            bad: Rgba::TRANSPARENT,
            under: None,
            over: None,
        }
    }
}

impl Extremes {
    /// The extremes of the reversed map: under and over swap.
    fn swapped(self) -> Self {
        Self {
            under: self.over,
            over: self.under,
            ..self
        }
    }
}

/// A map from normalised values to colours.
///
/// Values are looked up as fractions of the range (`[0, 1]`), as integer
/// indices into the table, or as missing — see [`Normalized`]. Under and
/// over default to the first and last colour; bad defaults to transparent.
#[derive(Clone, Debug)]
pub struct Colormap {
    name: String,
    source: Source,
    extremes: Extremes,
    colorbar_extend: Option<Extend>,
    /// The `N` colours followed by the under, over and bad colours.
    lut: Vec<Rgba>,
    /// The program's colormap, which maps arrays of values in place of
    /// the table (see [`ProgramColormap`]).
    program: Option<Rc<dyn ProgramColormap>>,
}

pltrs_core::archive_struct!(Colormap {
    name,
    source,
    extremes,
    colorbar_extend,
    lut
} skip { program });

impl Colormap {
    /// A colormap of explicit colours (matplotlib's `ListedColormap`).
    ///
    /// ```
    /// use pltrs::colors::Colormap;
    /// let cmap = Colormap::listed("rgb", vec!["r", "g", "b"])?;
    /// assert_eq!((cmap.n(), cmap.name()), (3, "rgb"));
    /// # Ok::<(), pltrs::colors::ColormapError>(())
    /// ```
    pub fn listed(
        name: impl Into<String>,
        colors: impl Into<Colors>,
    ) -> Result<Self, ColormapError> {
        let colors = to_rgba_array(colors, Alpha::Keep)?;
        if colors.is_empty() {
            return Err(ColormapError::NoColors);
        }
        Ok(Self::from_listed(name.into(), colors, Extremes::default()))
    }

    /// A colormap of `n` entries sampled from per-channel segment data
    /// (matplotlib's `LinearSegmentedColormap`). `gamma` warps the sample
    /// positions to `x ** gamma` (alpha is never warped).
    pub fn segmented(
        name: impl Into<String>,
        data: SegmentData,
        n: usize,
        gamma: f64,
    ) -> Result<Self, ColormapError> {
        Self::from_segments(name.into(), data, n, gamma, Extremes::default())
    }

    /// A smooth colormap through evenly spaced colours
    /// (matplotlib's `LinearSegmentedColormap.from_list`).
    pub fn from_list(
        name: impl Into<String>,
        colors: impl Into<Colors>,
        n: usize,
        gamma: f64,
    ) -> Result<Self, ColormapError> {
        let colors = to_rgba_array(colors, Alpha::Keep)?;
        let positions = linspace(0.0, 1.0, colors.len());
        Self::segmented(name, stops_data(&positions, &colors), n, gamma)
    }

    /// A smooth colormap through colours placed at `(position, colour)`
    /// stops; positions must increase from 0 to 1 (matplotlib's `from_list`
    /// with `(value, color)` pairs).
    pub fn from_stops<C: Into<Color>>(
        name: impl Into<String>,
        stops: impl IntoIterator<Item = (f64, C)>,
        n: usize,
        gamma: f64,
    ) -> Result<Self, ColormapError> {
        let (positions, colors): (Vec<f64>, Vec<Color>) =
            stops.into_iter().map(|(x, c)| (x, c.into())).unzip();
        if positions.is_empty() {
            return Err(ColormapError::NoColors);
        }
        let out_of_range = positions.iter().any(|&x| x < 0.0 || x > 1.0);
        if out_of_range || positions.windows(2).any(|w| w[1] - w[0] < 0.0) {
            return Err(ColormapError::Stops);
        }
        let colors = to_rgba_array(colors, Alpha::Keep)?;
        Self::segmented(name, stops_data(&positions, &colors), n, gamma)
    }

    fn from_listed(name: String, colors: Vec<Rgba>, extremes: Extremes) -> Self {
        Self::with_table(name, Source::Listed(colors.clone()), extremes, colors)
    }

    fn from_segments(
        name: String,
        data: SegmentData,
        n: usize,
        gamma: f64,
        extremes: Extremes,
    ) -> Result<Self, ColormapError> {
        let colors = sample_segments(&data, n, gamma)?;
        Ok(Self::with_table(
            name,
            Source::Segmented { data, gamma },
            extremes,
            colors,
        ))
    }

    fn with_table(name: String, source: Source, extremes: Extremes, colors: Vec<Rgba>) -> Self {
        let mut cmap = Self {
            name,
            source,
            extremes,
            colorbar_extend: None,
            lut: colors,
            program: None,
        };
        cmap.lut.extend([Rgba::TRANSPARENT; 3]);
        cmap.update_extremes();
        cmap
    }

    /// Rewrites the three extra LUT rows from the extremes
    /// (matplotlib's `_update_lut_extremes`).
    fn update_extremes(&mut self) {
        let n = self.n();
        self.lut[n] = self.extremes.under.unwrap_or(self.lut[0]);
        self.lut[n + 1] = self.extremes.over.unwrap_or(self.lut[n - 1]);
        self.lut[n + 2] = self.extremes.bad;
    }

    /// The colormap's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The number of colours `N` (excluding under, over and bad).
    pub fn n(&self) -> usize {
        self.lut.len() - 3
    }

    /// Whether this is a listed or a linearly segmented map.
    pub fn kind(&self) -> ColormapKind {
        match self.source {
            Source::Listed(_) => ColormapKind::Listed,
            Source::Segmented { .. } => ColormapKind::LinearSegmented,
        }
    }

    /// The segment data of a linearly segmented map.
    pub fn segment_data(&self) -> Option<&SegmentData> {
        match &self.source {
            Source::Segmented { data, .. } => Some(data),
            Source::Listed(_) => None,
        }
    }

    /// The `N` colours of the lookup table.
    pub fn lut(&self) -> &[Rgba] {
        &self.lut[..self.n()]
    }

    /// The whole lookup table: the `N` colours, then the under, over and
    /// bad colours, as [`lut_index`](Self::lut_index) numbers its rows.
    pub(crate) fn table(&self) -> &[Rgba] {
        &self.lut
    }

    /// The colour of values below the range.
    pub fn under(&self) -> Rgba {
        self.lut[self.n()]
    }

    /// The colour of values above the range.
    pub fn over(&self) -> Rgba {
        self.lut[self.n() + 1]
    }

    /// The colour of missing (NaN or masked) values.
    pub fn bad(&self) -> Rgba {
        self.lut[self.n() + 2]
    }

    /// The `extend` a colorbar of this map uses by default, if any.
    pub fn colorbar_extend(&self) -> Option<Extend> {
        self.colorbar_extend
    }

    /// Whether every colour, including the extremes, is a grey.
    pub fn is_gray(&self) -> bool {
        self.lut.iter().all(|c| c.r == c.g && c.r == c.b)
    }

    /// Whether all `N` colours are identical (always `false` for segmented
    /// maps, as in matplotlib).
    pub fn is_monochrome(&self) -> bool {
        match self.source {
            Source::Listed(_) => self.lut().iter().all(|c| *c == self.lut[0]),
            Source::Segmented { .. } => false,
        }
    }

    /// The same colormap under another name.
    #[must_use]
    pub fn with_name(self, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..self
        }
    }

    /// A copy with a different colour for missing values.
    pub fn with_bad(mut self, color: impl Into<Color>) -> Result<Self, ColormapError> {
        self.extremes.bad = color.into().resolve()?;
        self.update_extremes();
        Ok(self)
    }

    /// A copy with a different colour for values below the range.
    pub fn with_under(mut self, color: impl Into<Color>) -> Result<Self, ColormapError> {
        self.extremes.under = Some(color.into().resolve()?);
        self.update_extremes();
        Ok(self)
    }

    /// A copy with a different colour for values above the range.
    pub fn with_over(mut self, color: impl Into<Color>) -> Result<Self, ColormapError> {
        self.extremes.over = Some(color.into().resolve()?);
        self.update_extremes();
        Ok(self)
    }

    /// A copy whose every colour, extremes included, has opacity `alpha`.
    pub fn with_alpha(mut self, alpha: f64) -> Result<Self, ColormapError> {
        if !(0.0..=1.0).contains(&alpha) {
            return Err(ColormapError::AlphaOutOfRange(alpha));
        }
        for color in &mut self.lut {
            color.a = alpha;
        }
        Ok(self)
    }

    /// A copy with a default colorbar `extend`.
    #[must_use]
    pub fn with_colorbar_extend(self, extend: Option<Extend>) -> Self {
        Self {
            colorbar_extend: extend,
            ..self
        }
    }

    /// The map run backwards, named `<name>_r`; under and over swap.
    ///
    /// Like matplotlib, the result is rebuilt from the definition, so a
    /// [`with_alpha`](Self::with_alpha) override does not carry over.
    #[must_use]
    pub fn reversed(&self) -> Self {
        let name = format!("{}_r", self.name);
        let extremes = self.extremes.swapped();
        match &self.source {
            Source::Listed(colors) => {
                Self::from_listed(name, colors.iter().rev().copied().collect(), extremes)
            }
            Source::Segmented { data, gamma } => {
                Self::from_segments(name, data.reversed(), self.n(), *gamma, extremes)
                    .expect("reversing valid segment data keeps it valid")
            }
        }
    }

    /// The map with `lutsize` entries, keeping its extremes. Listed maps
    /// resample their current colours; segmented maps re-sample their
    /// segment data (dropping any gamma, as matplotlib does).
    pub fn resampled(&self, lutsize: usize) -> Result<Self, ColormapError> {
        if lutsize == 0 {
            return Err(ColormapError::ZeroEntries);
        }
        let name = self.name.clone();
        match &self.source {
            Source::Listed(_) => {
                let colors = linspace(0.0, 1.0, lutsize)
                    .into_iter()
                    .map(|x| self.sample(x))
                    .collect();
                Ok(Self::from_listed(name, colors, self.extremes))
            }
            Source::Segmented { data, .. } => {
                Self::from_segments(name, data.clone(), lutsize, 1.0, self.extremes)
            }
        }
    }
}

impl PartialEq for Colormap {
    /// matplotlib's `==`: identical lookup tables (extremes included) and
    /// colorbar extension.
    fn eq(&self, other: &Self) -> bool {
        self.colorbar_extend == other.colorbar_extend && self.lut == other.lut
    }
}

/// Segment data through `colors` at `positions`, each channel continuous.
fn stops_data(positions: &[f64], colors: &[Rgba]) -> SegmentData {
    let channel = |value: fn(&Rgba) -> f64| {
        Segment::Anchors(
            positions
                .iter()
                .zip(colors)
                .map(|(&x, c)| [x, value(c), value(c)])
                .collect(),
        )
    };
    SegmentData::new(channel(|c| c.r), channel(|c| c.g), channel(|c| c.b))
        .with_alpha(channel(|c| c.a))
}

/// The `n` colours of segment data (opaque unless it has an alpha channel).
fn sample_segments(data: &SegmentData, n: usize, gamma: f64) -> Result<Vec<Rgba>, ColormapError> {
    if n == 0 {
        return Err(ColormapError::ZeroEntries);
    }
    let red = segment::lookup_table(n, &data.red, gamma, Channel::Red)?;
    let green = segment::lookup_table(n, &data.green, gamma, Channel::Green)?;
    let blue = segment::lookup_table(n, &data.blue, gamma, Channel::Blue)?;
    let alpha = match &data.alpha {
        Some(alpha) => segment::lookup_table(n, alpha, 1.0, Channel::Alpha)?,
        None => vec![1.0; n],
    };
    Ok((0..n)
        .map(|i| Rgba::new(red[i], green[i], blue[i], alpha[i]))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::rgb_colormap;

    #[test]
    fn listed_extremes_and_reversal() {
        let cmap = rgb_colormap();
        assert_eq!(cmap.under(), Rgba::opaque(1.0, 0.0, 0.0));
        assert_eq!(cmap.over(), Rgba::opaque(0.0, 0.0, 1.0));
        assert_eq!(cmap.bad(), Rgba::TRANSPARENT);
        let cmap = cmap.with_under("k").unwrap().with_bad("0.5").unwrap();
        let rev = cmap.reversed();
        assert_eq!(rev.name(), "rgb_r");
        assert_eq!(rev.lut()[0], Rgba::opaque(0.0, 0.0, 1.0));
        assert_eq!(rev.over(), Rgba::BLACK, "under and over swap");
        assert_eq!(rev.bad(), Rgba::opaque(0.5, 0.5, 0.5));
        assert!(Colormap::listed("x", Vec::<Color>::new()).is_err());
        assert!(Colormap::listed("x", vec!["red", "notacolor"]).is_err());
    }

    #[test]
    fn from_list_matches_matplotlib() {
        let cmap = Colormap::from_list("p", vec!["red", "green", "blue"], 3, 1.0).unwrap();
        assert_eq!(
            cmap.lut(),
            [
                Rgba::opaque(1.0, 0.0, 0.0),
                Rgba::opaque(0.0, 128.0 / 255.0, 0.0),
                Rgba::opaque(0.0, 0.0, 1.0)
            ]
        );
        let one = Colormap::from_list("p", vec!["red", "green", "blue"], 1, 1.0).unwrap();
        assert_eq!(one.lut(), [Rgba::opaque(0.0, 0.0, 1.0)]);
        assert!(Colormap::from_list("p", vec!["red"], 4, 1.0).is_err());
        assert_eq!(
            Colormap::from_stops("p", [(0.2, "red"), (1.0, "blue")], 5, 1.0).unwrap_err(),
            ColormapError::Segment {
                channel: Channel::Red,
                problem: SegmentProblem::Endpoints
            }
        );
        assert_eq!(
            Colormap::from_stops("p", [(-0.1, "red"), (1.0, "blue")], 5, 1.0).unwrap_err(),
            ColormapError::Stops
        );
    }

    #[test]
    fn alpha_resampling_and_equality() {
        let cmap = rgb_colormap().with_alpha(0.5).unwrap();
        assert!(cmap.lut.iter().all(|c| c.a == 0.5));
        assert!(rgb_colormap().with_alpha(1.2).is_err());
        assert_eq!(cmap.reversed().lut()[0].a, 1.0, "rebuilt from the colours");
        let two = rgb_colormap().resampled(2).unwrap();
        assert_eq!(
            two.lut(),
            [rgb_colormap().lut()[0], rgb_colormap().lut()[2]]
        );
        assert_eq!(rgb_colormap(), rgb_colormap().with_name("other"));
        assert_ne!(rgb_colormap(), rgb_colormap().reversed());
        assert!(rgb_colormap().resampled(0).is_err());
    }

    #[test]
    fn grey_and_monochrome() {
        let grey = Colormap::listed("g", vec!["0.2", "0.4"]).unwrap();
        assert!(grey.is_gray() && !grey.is_monochrome());
        let mono = Colormap::listed("m", vec!["red", "red"]).unwrap();
        assert!(mono.is_monochrome() && !mono.is_gray());
        let ramp = Colormap::from_list("w", vec!["white", "black"], 11, 1.0).unwrap();
        assert!(ramp.is_gray() && !ramp.is_monochrome());
        assert_eq!(ramp.kind(), ColormapKind::LinearSegmented);
    }

    #[test]
    fn extend_parsing() {
        assert_eq!("both".parse(), Ok(Extend::Both));
        assert!(Extend::Min.extends_min() && !Extend::Min.extends_max());
        assert_eq!(Extend::Max.to_string(), "max");
        assert_eq!(
            "sideways".parse::<Extend>().map_err(|err| err.to_string()),
            Err("\"sideways\" is not a valid value for extend".to_owned())
        );
    }
}
