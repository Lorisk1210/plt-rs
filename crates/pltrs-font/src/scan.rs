//! Glyph outlines as FreeType's monochrome rasterizer receives them: the
//! tagged points of a loaded glyph in 26.6 pixels, with the scan-conversion
//! rules its loader sets in the outline flags (`FT_OUTLINE_HIGH_PRECISION`,
//! `FT_OUTLINE_IGNORE_DROPOUTS`, `FT_OUTLINE_SMART_DROPOUTS` and
//! `FT_OUTLINE_INCLUDE_STUBS`).

use crate::outline::{Outline, PointKind};

/// Which drop-outs the monochrome rasterizer fills: spans so thin that
/// they cover no pixel centre (TrueType's scan-conversion modes, set with
/// `SCANCTRL` and `SCANTYPE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dropouts {
    /// Drop-outs stay empty.
    Ignore,
    /// The pixel left of (or below) a drop-out is set; `stubs` also fills
    /// drop-outs at the ends of thin features.
    Simple {
        /// Whether stubs are filled too.
        stubs: bool,
    },
    /// The pixel nearest a drop-out is set; `stubs` also fills drop-outs
    /// at the ends of thin features.
    Smart {
        /// Whether stubs are filled too.
        stubs: bool,
    },
}

impl Dropouts {
    /// The rasterizer's drop-out control bits (`ras.dropOutControl`): bit 0
    /// excludes stubs, bit 1 ignores drop-outs, bit 2 rounds smartly.
    pub const fn control_bits(self) -> u8 {
        match self {
            Self::Ignore => 3,
            Self::Simple { stubs: true } => 0,
            Self::Simple { stubs: false } => 1,
            Self::Smart { stubs: true } => 4,
            Self::Smart { stubs: false } => 5,
        }
    }

    /// The mode a TrueType glyph program leaves (`TT_Load_Glyph`): none
    /// without scan control, else the one `scan_type` names.
    pub(crate) const fn from_scan_type(scan_control: bool, scan_type: i32) -> Self {
        match (scan_control, scan_type) {
            (true, 0) => Self::Simple { stubs: true },
            (true, 1) => Self::Simple { stubs: false },
            (true, 4) => Self::Smart { stubs: true },
            (true, 5) => Self::Smart { stubs: false },
            _ => Self::Ignore,
        }
    }
}

/// How a loaded glyph is scan-converted without anti-aliasing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScanRules {
    /// Whether the rasterizer may use 1/4096-pixel precision instead of
    /// 1/64 (`FT_OUTLINE_HIGH_PRECISION`, set for scaled glyphs below 24
    /// pixels per em).
    pub high_precision: bool,
    /// The drop-out control mode.
    pub dropouts: Dropouts,
}

impl ScanRules {
    /// The rules of a glyph scaled to `y_ppem` pixels per em and not
    /// hinted by its own instructions.
    pub(crate) const fn scaled(y_ppem: u16) -> Self {
        Self {
            high_precision: y_ppem < 24,
            dropouts: Dropouts::Simple { stubs: false },
        }
    }

    /// The rules of an autohinted glyph: the autohinter loads its outline
    /// unscaled, so no loader sets any flag.
    pub(crate) const UNSCALED: Self = Self {
        high_precision: false,
        dropouts: Dropouts::Simple { stubs: false },
    };
}

/// The role of an outline point (`FT_CURVE_TAG`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScanPointKind {
    /// On the curve.
    On,
    /// A quadratic (conic) control point.
    Conic,
    /// A cubic control point.
    Cubic,
}

/// One outline point in 26.6 pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScanPoint {
    /// Horizontal position, 26.6.
    pub x: i64,
    /// Vertical position (up), 26.6.
    pub y: i64,
    /// Its role.
    pub kind: ScanPointKind,
}

/// A loaded glyph outline as FreeType's monochrome rasterizer receives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanOutline {
    /// Every point, contour after contour.
    pub points: Vec<ScanPoint>,
    /// The exclusive end of each contour in `points`.
    pub contour_ends: Vec<usize>,
    /// How the outline is scan-converted.
    pub rules: ScanRules,
}

impl Outline {
    /// This outline (in 26.6 units) with the scan-conversion `rules` its
    /// loader set.
    pub(crate) fn scan_outline(&self, rules: ScanRules) -> ScanOutline {
        let points = self
            .points
            .iter()
            .map(|point| ScanPoint {
                x: point.x.round() as i64,
                y: point.y.round() as i64,
                kind: match point.kind {
                    PointKind::OnCurve => ScanPointKind::On,
                    PointKind::Quadratic => ScanPointKind::Conic,
                    PointKind::Cubic => ScanPointKind::Cubic,
                },
            })
            .collect();
        ScanOutline {
            points,
            contour_ends: self.contour_ends.clone(),
            rules,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_bits_follow_the_outline_flags() {
        assert_eq!(Dropouts::Ignore.control_bits(), 3);
        assert_eq!(Dropouts::Simple { stubs: true }.control_bits(), 0);
        assert_eq!(Dropouts::Simple { stubs: false }.control_bits(), 1);
        assert_eq!(Dropouts::Smart { stubs: true }.control_bits(), 4);
        assert_eq!(Dropouts::Smart { stubs: false }.control_bits(), 5);
    }

    #[test]
    fn scan_types_map_like_freetype() {
        assert_eq!(Dropouts::from_scan_type(false, 4), Dropouts::Ignore);
        assert_eq!(
            Dropouts::from_scan_type(true, 0),
            Dropouts::Simple { stubs: true }
        );
        assert_eq!(
            Dropouts::from_scan_type(true, 5),
            Dropouts::Smart { stubs: false }
        );
        assert_eq!(Dropouts::from_scan_type(true, 2), Dropouts::Ignore);
        assert!(ScanRules::scaled(23).high_precision);
        assert!(!ScanRules::scaled(24).high_precision);
    }

    #[test]
    fn outlines_keep_their_tags_and_contours() {
        let mut outline = Outline::default();
        outline.push(0.0, 0.0, PointKind::OnCurve);
        outline.push(64.0, 128.0, PointKind::Quadratic);
        outline.push(128.0, 0.0, PointKind::OnCurve);
        outline.end_contour();
        let scan = outline.scan_outline(ScanRules::UNSCALED);
        assert_eq!(scan.contour_ends, vec![3]);
        assert_eq!(
            scan.points[1],
            ScanPoint {
                x: 64,
                y: 128,
                kind: ScanPointKind::Conic
            }
        );
    }
}
