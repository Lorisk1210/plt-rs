//! The graphics context: stroke, fill-modifier and clipping state for a draw call.

use pltrs_core::path::convert::SketchParams;
use pltrs_core::{Affine2D, Bbox, Path, Rgba};

/// How the ends of open strokes are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CapStyle {
    /// Flush with the end point.
    #[default]
    Butt,
    /// Squared off, extending half the line width past the end point.
    Projecting,
    /// A semicircle around the end point.
    Round,
}

pltrs_core::archive_enum!(CapStyle {
    Butt,
    Projecting,
    Round,
});

/// How stroke segments are joined at corners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum JoinStyle {
    /// Sharp corner (subject to the miter limit).
    Miter,
    /// Rounded corner.
    #[default]
    Round,
    /// Cut-off corner.
    Bevel,
}

pltrs_core::archive_enum!(JoinStyle {
    Miter,
    Round,
    Bevel,
});

impl CapStyle {
    /// Parses matplotlib's names (`"butt"`, `"projecting"`, `"round"`).
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "butt" => Self::Butt,
            "projecting" => Self::Projecting,
            "round" => Self::Round,
            _ => return None,
        })
    }

    /// matplotlib's name for the style.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Butt => "butt",
            Self::Projecting => "projecting",
            Self::Round => "round",
        }
    }
}

impl JoinStyle {
    /// Parses matplotlib's names (`"miter"`, `"round"`, `"bevel"`).
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "miter" => Self::Miter,
            "round" => Self::Round,
            "bevel" => Self::Bevel,
            _ => return None,
        })
    }

    /// matplotlib's name for the style.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Miter => "miter",
            Self::Round => "round",
            Self::Bevel => "bevel",
        }
    }
}

/// A dash pattern in points: alternating on/off lengths starting at
/// `offset` (matplotlib's `(offset, dash_list)` of a graphics context).
///
/// A line without a dash list and one with an empty list both draw
/// solid, but the SVG, PDF and PGF backends write an empty list where
/// they write none for no list, as matplotlib does.
///
/// ```
/// use pltrs_render::Dashes;
/// assert!(Dashes::solid().is_solid() && Dashes::new(0.0, Vec::new()).is_solid());
/// assert_ne!(Dashes::solid(), Dashes::new(0.0, Vec::new()));
/// assert!(!Dashes::new(1.0, vec![3.0, 1.0]).is_solid());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dashes {
    /// Distance into the pattern at which the stroke starts.
    pub offset: f64,
    /// On/off lengths, or `None` for no dash list (matplotlib's `None`).
    pub pattern: Option<Vec<f64>>,
}

pltrs_core::archive_struct!(Dashes { offset, pattern });

impl Dashes {
    /// A solid line: no dash list.
    pub const fn solid() -> Self {
        Self {
            offset: 0.0,
            pattern: None,
        }
    }

    /// The on/off lengths `pattern` from `offset` on; an empty pattern
    /// draws solid.
    pub const fn new(offset: f64, pattern: Vec<f64>) -> Self {
        Self {
            offset,
            pattern: Some(pattern),
        }
    }

    /// The on/off lengths, empty without a dash list.
    pub fn lengths(&self) -> &[f64] {
        self.pattern.as_deref().unwrap_or_default()
    }

    /// Returns `true` when the pattern draws a continuous line.
    pub fn is_solid(&self) -> bool {
        self.lengths().iter().all(|&len| len <= 0.0)
    }
}

/// A hatch fill drawn over a path's face.
#[derive(Clone, Debug, PartialEq)]
pub struct Hatch {
    /// matplotlib hatch pattern, e.g. `"//"`, `"x"`, `"o."`.
    pub pattern: String,
    /// Colour of the hatch lines.
    pub color: Rgba,
    /// Hatch line width in points.
    pub linewidth: f64,
}

pltrs_core::archive_struct!(Hatch {
    pattern,
    color,
    linewidth
});

/// Parameters of the hand-drawn "sketch" path filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sketch {
    /// Amplitude of the wiggle perpendicular to the line, in pixels.
    pub scale: f64,
    /// Length of the wiggle along the line, in pixels.
    pub length: f64,
    /// Factor by which the length is randomly shrunk or expanded.
    pub randomness: f64,
}

pltrs_core::archive_struct!(Sketch {
    scale,
    length,
    randomness
});

impl Sketch {
    /// The path converters' sketch filter settings.
    pub(crate) fn params(self) -> SketchParams {
        SketchParams::new(self.scale, Some(self.length), Some(self.randomness))
    }
}

/// An arbitrary clip region: a path and the transform to display space.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipPath {
    /// The region's outline.
    pub path: Path,
    /// Maps `path` to display coordinates.
    pub transform: Affine2D,
    /// Whether the outline is made anew each time a renderer is handed
    /// it, as matplotlib's fancy boxes and arrows build a new `Path` at
    /// every `get_path`: a renderer that keeps the last clip path it
    /// rasterized (matplotlib's Agg, which knows a path by its vertex
    /// array) never finds such an outline among those it kept.
    pub rebuilt: bool,
}

impl ClipPath {
    /// A clip region that is the same path each time it is handed over.
    #[must_use]
    pub fn new(path: Path, transform: Affine2D) -> Self {
        Self {
            path,
            transform,
            rebuilt: false,
        }
    }

    /// The region, made anew each time it is handed over when `rebuilt`
    /// (see the field [`rebuilt`](Self::rebuilt)).
    #[must_use]
    pub fn with_rebuilt(self, rebuilt: bool) -> Self {
        Self { rebuilt, ..self }
    }
}

/// Snapping behaviour for rectilinear paths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SnapMode {
    /// Snap paths consisting only of horizontal/vertical segments.
    #[default]
    Auto,
    /// Always snap vertices to pixel centres.
    Always,
    /// Never snap.
    Never,
}

pltrs_core::archive_enum!(SnapMode {
    Auto,
    Always,
    Never,
});

/// Everything about *how* a primitive is drawn, mirroring matplotlib's
/// `GraphicsContextBase`. Lengths are in points unless stated otherwise;
/// coordinates are display pixels with the origin at the bottom left.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphicsContext {
    /// Stroke colour.
    pub foreground: Rgba,
    /// When set, replaces the alpha of the stroke and fill colours.
    pub alpha: Option<f64>,
    /// Anti-aliasing on/off.
    pub antialiased: bool,
    /// Line end style.
    pub cap: CapStyle,
    /// Line join style.
    pub join: JoinStyle,
    /// Line width in points (0 disables stroking).
    pub linewidth: f64,
    /// Dash pattern in points.
    pub dashes: Dashes,
    /// Rectangular clip region in display coordinates.
    pub clip_rect: Option<Bbox>,
    /// Arbitrary clip region.
    pub clip_path: Option<ClipPath>,
    /// Hatch drawn over filled faces.
    pub hatch: Option<Hatch>,
    /// Pixel snapping.
    pub snap: SnapMode,
    /// Hand-drawn wobble.
    pub sketch: Option<Sketch>,
    /// Hyperlink target (vector backends).
    pub url: Option<String>,
    /// Group identifier (vector backends).
    pub gid: Option<String>,
}

impl Default for GraphicsContext {
    fn default() -> Self {
        Self {
            foreground: Rgba::BLACK,
            alpha: None,
            antialiased: true,
            cap: CapStyle::Butt,
            join: JoinStyle::Round,
            linewidth: 1.0,
            dashes: Dashes::solid(),
            clip_rect: None,
            clip_path: None,
            hatch: None,
            snap: SnapMode::Auto,
            sketch: None,
            url: None,
            gid: None,
        }
    }
}

impl GraphicsContext {
    /// The effective stroke colour (alpha override applied).
    pub fn stroke_color(&self) -> Rgba {
        self.apply_alpha(self.foreground)
    }

    /// The effective colour for a face fill (alpha override applied).
    pub fn fill_color(&self, face: Rgba) -> Rgba {
        self.apply_alpha(face)
    }

    /// Whether anything visible would be stroked.
    pub fn strokes(&self) -> bool {
        self.linewidth > 0.0 && !self.stroke_color().is_transparent()
    }

    fn apply_alpha(&self, color: Rgba) -> Rgba {
        self.alpha.map_or(color, |a| color.with_alpha(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_override_applies_to_stroke_and_fill() {
        let gc = GraphicsContext {
            foreground: Rgba::new(1.0, 0.0, 0.0, 0.2),
            alpha: Some(0.5),
            ..GraphicsContext::default()
        };
        assert_eq!(gc.stroke_color().a, 0.5);
        assert_eq!(gc.fill_color(Rgba::WHITE).a, 0.5);
        assert!(gc.strokes());
    }

    #[test]
    fn style_names_round_trip() {
        for cap in [CapStyle::Butt, CapStyle::Projecting, CapStyle::Round] {
            assert_eq!(CapStyle::from_name(cap.name()), Some(cap));
        }
        for join in [JoinStyle::Miter, JoinStyle::Round, JoinStyle::Bevel] {
            assert_eq!(JoinStyle::from_name(join.name()), Some(join));
        }
        assert_eq!(CapStyle::from_name("square"), None);
    }

    #[test]
    fn zero_length_dash_patterns_are_solid() {
        assert!(Dashes::solid().is_solid());
        assert!(Dashes::new(0.0, vec![0.0, 0.0]).is_solid());
        assert!(Dashes::new(2.0, Vec::new()).is_solid());
        assert!(!Dashes::new(0.0, vec![3.0, 1.0]).is_solid());
        assert_eq!(Dashes::solid().lengths(), []);
    }
}
