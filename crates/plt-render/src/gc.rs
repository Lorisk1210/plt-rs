//! The graphics context: stroke, fill-modifier and clipping state for a draw call.

use plt_core::path::convert::SketchParams;
use plt_core::{Affine2D, Bbox, Path, Rgba};

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

/// A dash pattern in points: alternating on/off lengths starting at `offset`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dashes {
    /// Distance into the pattern at which the stroke starts.
    pub offset: f64,
    /// On/off lengths; empty means a solid line.
    pub pattern: Vec<f64>,
}

impl Dashes {
    /// A solid line.
    pub const fn solid() -> Self {
        Self {
            offset: 0.0,
            pattern: Vec::new(),
        }
    }

    /// Returns `true` when the pattern draws a continuous line.
    pub fn is_solid(&self) -> bool {
        self.pattern.is_empty() || self.pattern.iter().all(|&len| len <= 0.0)
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

    /// The hatch, unless its pattern is empty (matplotlib's artists only
    /// set non-empty hatches).
    pub(crate) fn active_hatch(&self) -> Option<&Hatch> {
        self.hatch.as_ref().filter(|h| !h.pattern.is_empty())
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
        assert!(
            Dashes {
                offset: 0.0,
                pattern: vec![0.0, 0.0]
            }
            .is_solid()
        );
        assert!(
            !Dashes {
                offset: 0.0,
                pattern: vec![3.0, 1.0]
            }
            .is_solid()
        );
    }
}
