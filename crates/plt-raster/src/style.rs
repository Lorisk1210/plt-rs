//! Fill and stroke parameters.

use crate::error::RasterError;

/// How overlapping sub-paths and self-intersections decide what is inside.
///
/// matplotlib's Agg backend always fills with [`FillRule::NonZero`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FillRule {
    /// Inside where the winding number is non-zero.
    #[default]
    NonZero,
    /// Inside where the winding number is odd.
    EvenOdd,
}

/// Whether coverage is blended proportionally or as all-or-nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Antialias {
    /// Partial coverage blends proportionally (`renderer_scanline_aa_solid`).
    #[default]
    On,
    /// Every pixel with *any* non-zero coverage is painted at full strength
    /// (`renderer_scanline_bin_solid`), which is what matplotlib's
    /// `antialiased=False` does.
    Off,
}

impl From<bool> for Antialias {
    fn from(antialiased: bool) -> Self {
        if antialiased { Self::On } else { Self::Off }
    }
}

/// Shape of the ends of open sub-paths and dashes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineCap {
    /// The stroke ends exactly at the end point.
    #[default]
    Butt,
    /// The stroke extends half a width beyond the end point (matplotlib's
    /// `"projecting"`).
    Square,
    /// A semicircle centred on the end point.
    Round,
}

/// Shape of the outer corner where two segments meet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineJoin {
    /// Agg's `miter_join`: a miter that is cut off at the miter limit.
    #[default]
    Miter,
    /// Agg's `miter_join_revert`: a miter that falls back to a bevel
    /// beyond the limit, as in SVG/PDF. matplotlib's `"miter"` uses this.
    MiterRevert,
    /// A circular arc.
    Round,
    /// A straight cut across the corner.
    Bevel,
    /// A miter that falls back to a round join beyond the limit.
    MiterRound,
}

/// Shape of the inner corner where two segments meet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InnerJoin {
    /// A straight cut.
    Bevel,
    /// A miter limited by the inner miter limit (Agg's default, used by
    /// matplotlib).
    #[default]
    Miter,
    /// A notch back to the vertex for very short segments.
    Jag,
    /// A rounded notch for very short segments.
    Round,
}

/// At most this many `(dash, gap)` pairs are used, like Agg's
/// `vcgen_dash` (32 lengths).
pub const MAX_DASH_PAIRS: usize = 16;

/// An on/off dash pattern in pixels with a start offset.
#[derive(Clone, Debug, PartialEq)]
pub struct DashPattern {
    pairs: Vec<(f64, f64)>,
    offset: f64,
}

impl DashPattern {
    /// Creates a pattern from `(dash, gap)` pairs. Lengths must be finite
    /// and non-negative with at least one positive; pairs beyond
    /// [`MAX_DASH_PAIRS`] are ignored, as in Agg. A negative `offset`
    /// continues the pattern across sub-paths instead of restarting it.
    pub fn new(pairs: Vec<(f64, f64)>, offset: f64) -> Result<Self, RasterError> {
        let lengths = || pairs.iter().flat_map(|&(d, g)| [d, g]);
        let valid = lengths().all(|v| v.is_finite() && v >= 0.0)
            && lengths().any(|v| v > 0.0)
            && offset.is_finite();
        if !valid {
            return Err(RasterError::InvalidDashPattern);
        }
        let mut pairs = pairs;
        pairs.truncate(MAX_DASH_PAIRS);
        Ok(Self { pairs, offset })
    }

    /// Creates a pattern from a flat on/off sequence; an odd-length
    /// sequence is repeated twice, as matplotlib (and PostScript) do.
    pub fn from_lengths(lengths: &[f64], offset: f64) -> Result<Self, RasterError> {
        let n = lengths.len();
        let total = if n % 2 == 1 { 2 * n } else { n };
        let pairs = (0..total)
            .step_by(2)
            .map(|i| (lengths[i % n], lengths[(i + 1) % n]))
            .collect();
        Self::new(pairs, offset)
    }

    /// The `(dash, gap)` pairs.
    pub fn pairs(&self) -> &[(f64, f64)] {
        &self.pairs
    }

    /// The start offset into the pattern.
    pub const fn offset(&self) -> f64 {
        self.offset
    }
}

/// Everything that shapes a stroke outline, in pixels.
///
/// [`StrokeStyle::new`] gives Agg's `conv_stroke` defaults. To reproduce
/// matplotlib's `RendererAgg`, map `"miter"` to [`LineJoin::MiterRevert`],
/// `"projecting"` to [`LineCap::Square`], and set the miter limit to the
/// line width in pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    /// Full stroke width.
    pub width: f64,
    /// End caps.
    pub cap: LineCap,
    /// Outer joins.
    pub join: LineJoin,
    /// Miter length limit, in multiples of half the width.
    pub miter_limit: f64,
    /// Inner joins.
    pub inner_join: InnerJoin,
    /// Inner miter limit.
    pub inner_miter_limit: f64,
    /// Scale for the number of segments in round caps and joins.
    pub approximation_scale: f64,
    /// Optional dash pattern.
    pub dashes: Option<DashPattern>,
}

impl StrokeStyle {
    /// A solid stroke of `width` pixels with Agg's default parameters.
    pub fn new(width: f64) -> Self {
        Self {
            width,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            inner_join: InnerJoin::Miter,
            inner_miter_limit: 1.01,
            approximation_scale: 1.0,
            dashes: None,
        }
    }

    /// Sets the cap style.
    #[must_use]
    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    /// Sets the join style.
    #[must_use]
    pub fn with_join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    /// Sets the miter limit.
    #[must_use]
    pub fn with_miter_limit(mut self, limit: f64) -> Self {
        self.miter_limit = limit;
        self
    }

    /// Sets (or clears) the dash pattern.
    #[must_use]
    pub fn with_dashes(mut self, dashes: Option<DashPattern>) -> Self {
        self.dashes = dashes;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_validation() {
        assert!(DashPattern::new(vec![(1.0, 2.0)], 0.0).is_ok());
        assert!(DashPattern::new(vec![(0.0, 0.0)], 0.0).is_err());
        assert!(DashPattern::new(vec![(-1.0, 2.0)], 0.0).is_err());
        assert!(DashPattern::new(vec![(f64::NAN, 2.0)], 0.0).is_err());
        assert!(DashPattern::new(vec![(1.0, 2.0)], f64::INFINITY).is_err());
        assert!(DashPattern::new(vec![], 0.0).is_err());
        let many = DashPattern::new(vec![(1.0, 1.0); 40], 0.0).unwrap();
        assert_eq!(many.pairs().len(), MAX_DASH_PAIRS);
    }

    #[test]
    fn odd_sequences_repeat() {
        let d = DashPattern::from_lengths(&[1.0, 2.0, 3.0], 0.5).unwrap();
        assert_eq!(d.pairs(), &[(1.0, 2.0), (3.0, 1.0), (2.0, 3.0)]);
        assert_eq!(d.offset(), 0.5);
        assert!(DashPattern::from_lengths(&[], 0.0).is_err());
    }

    #[test]
    fn builders() {
        let s = StrokeStyle::new(2.0)
            .with_cap(LineCap::Round)
            .with_join(LineJoin::Bevel)
            .with_miter_limit(2.0);
        assert_eq!(
            (s.cap, s.join, s.miter_limit),
            (LineCap::Round, LineJoin::Bevel, 2.0)
        );
        assert_eq!(Antialias::from(false), Antialias::Off);
    }
}
