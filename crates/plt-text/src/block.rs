//! Multi-line text blocks: line stacking, alignment and rotation
//! (matplotlib's `Text._get_layout`).

use plt_core::{Bbox, Point};

/// Measurements of one laid-out line, in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineMetrics {
    /// Advance width.
    pub width: f64,
    /// Height including the descent.
    pub height: f64,
    /// Depth below the baseline.
    pub descent: f64,
}

/// The font's own line metrics, in pixels: matplotlib takes them from the
/// `OS/2` typo values (or `hhea`) of the primary font.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FontHeightMetrics {
    /// Minimum ascent of a line.
    pub ascent: f64,
    /// Minimum descent of a line.
    pub descent: f64,
    /// Extra gap between lines.
    pub line_gap: f64,
}

/// Horizontal alignment of a text block (or of lines within it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HAlign {
    /// Anchor at the left edge.
    #[default]
    Left,
    /// Anchor at the centre.
    Center,
    /// Anchor at the right edge.
    Right,
}

/// Vertical alignment of a text block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VAlign {
    /// Anchor at the top.
    Top,
    /// Anchor at the bottom.
    Bottom,
    /// Anchor at the middle.
    Center,
    /// Anchor at the baseline of the last line.
    #[default]
    Baseline,
    /// Anchor halfway up the first line, from its baseline.
    CenterBaseline,
}

/// How rotation interacts with alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RotationMode {
    /// Rotate first, then align the rotated bounding box.
    #[default]
    Default,
    /// Align the unrotated text, then rotate about the anchor.
    Anchor,
    /// Like `Default`, choosing the horizontal alignment from the angle
    /// (for x tick labels).
    XTick,
    /// Like `Default`, choosing the vertical alignment from the angle
    /// (for y tick labels).
    YTick,
}

/// Line spacing of multi-line text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LineSpacing {
    /// The font's natural spacing (ascent + descent + line gap).
    #[default]
    Normal,
    /// A multiple of the font's ascent + descent.
    Factor(f64),
}

/// Alignment and rotation of a text block.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BlockStyle {
    /// Counter-clockwise rotation in degrees.
    pub rotation: f64,
    /// Horizontal alignment of the block.
    pub halign: HAlign,
    /// Vertical alignment of the block.
    pub valign: VAlign,
    /// Alignment of lines relative to each other.
    pub multialign: HAlign,
    /// How rotation and alignment combine.
    pub rotation_mode: RotationMode,
    /// Spacing between lines.
    pub linespacing: LineSpacing,
}

/// Where one line of a block is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinePlacement {
    /// Advance width.
    pub width: f64,
    /// Ascent used for stacking.
    pub ascent: f64,
    /// Descent used for stacking.
    pub descent: f64,
    /// Position of the line's baseline origin relative to the anchor,
    /// already rotated, in pixels.
    pub origin: Point,
}

/// A laid-out text block, relative to its anchor point (pixels).
#[derive(Clone, Debug, PartialEq)]
pub struct BlockLayout {
    /// Axis-aligned bounds of the rotated text.
    pub bbox: Bbox,
    /// Placement of every line.
    pub lines: Vec<LinePlacement>,
    /// Lower-left corner of the rotated, text-aligned frame (for boxes
    /// drawn around the text) and its unrotated size.
    pub frame: (Point, (f64, f64)),
}

/// Rotates `(x, y)` by `angle` degrees, exactly as matplotlib's
/// `_rotate_point` (no rotation for zero).
fn rotate(angle: f64, x: f64, y: f64) -> (f64, f64) {
    if angle == 0.0 {
        return (x, y);
    }
    let (sin, cos) = angle.to_radians().sin_cos();
    (cos * x - sin * y, sin * x + cos * y)
}

/// matplotlib's `_ha_for_angle` for `rotation_mode="xtick"`.
fn ha_for_angle(angle: f64, valign: VAlign) -> HAlign {
    let anchor_at_bottom = valign == VAlign::Bottom;
    if angle <= 10.0
        || (85.0..=95.0).contains(&angle)
        || 350.0 <= angle
        || (170.0..=190.0).contains(&angle)
        || (265.0..=275.0).contains(&angle)
    {
        HAlign::Center
    } else if (10.0 < angle && angle < 85.0) || (190.0 < angle && angle < 265.0) {
        if anchor_at_bottom {
            HAlign::Left
        } else {
            HAlign::Right
        }
    } else if anchor_at_bottom {
        HAlign::Right
    } else {
        HAlign::Left
    }
}

/// matplotlib's `_va_for_angle` for `rotation_mode="ytick"`.
fn va_for_angle(angle: f64, halign: HAlign) -> VAlign {
    let anchor_at_left = halign == HAlign::Left;
    if angle <= 10.0
        || 350.0 <= angle
        || (170.0..=190.0).contains(&angle)
        || (80.0..=100.0).contains(&angle)
        || (260.0..=280.0).contains(&angle)
    {
        VAlign::Center
    } else if (190.0 < angle && angle < 260.0) || (10.0 < angle && angle < 80.0) {
        if anchor_at_left {
            VAlign::Baseline
        } else {
            VAlign::Top
        }
    } else if anchor_at_left {
        VAlign::Top
    } else {
        VAlign::Baseline
    }
}

/// Stacks, aligns and rotates `lines` (matplotlib's `Text._get_layout`).
///
/// `lines` must not be empty (an empty string is one empty line).
pub fn layout_block(
    lines: &[LineMetrics],
    font: FontHeightMetrics,
    style: &BlockStyle,
) -> BlockLayout {
    let line_gap = if lines.len() == 1 { 0.0 } else { font.line_gap };
    let mut stacked = Vec::with_capacity(lines.len());
    let mut this_y = 0.0;
    let mut baseline = 0.0;
    let mut descent = 0.0;
    for line in lines {
        let (mut a, mut d) = (line.height - line.descent, line.descent);
        match style.linespacing {
            LineSpacing::Normal => {
                a = a.max(font.ascent) + line_gap / 2.0;
                d = d.max(font.descent) + line_gap / 2.0;
            }
            LineSpacing::Factor(factor) => {
                let leading = factor * (font.ascent + font.descent) - (a + d);
                a += leading / 2.0;
                d += leading / 2.0;
            }
        }
        baseline = a - this_y;
        this_y -= a;
        stacked.push((line.width, a, d, this_y));
        this_y -= d;
        descent = d;
    }

    let width = stacked
        .iter()
        .map(|&(w, ..)| w)
        .fold(f64::NEG_INFINITY, f64::max);
    let last_y = stacked.last().map_or(0.0, |&(.., y)| y);
    let (xmin, xmax, ymax, ymin) = (0.0, width, 0.0, last_y - descent);
    let offsets: Vec<(f64, f64)> = stacked
        .iter()
        .map(|&(w, _, _, y)| match style.multialign {
            HAlign::Left => (0.0, y),
            HAlign::Center => (width / 2.0 - w / 2.0, y),
            HAlign::Right => (width - w, y),
        })
        .collect();

    let angle = style.rotation;
    let corners = [(xmin, ymin), (xmin, ymax), (xmax, ymax), (xmax, ymin)];
    let rotated = corners.map(|(x, y)| rotate(angle, x, y));
    let pick = |f: fn(f64, f64) -> f64, select: fn(&(f64, f64)) -> f64| {
        rotated.iter().map(select).reduce(f).unwrap_or(0.0)
    };
    let (rxmin, rxmax) = (pick(f64::min, |p| p.0), pick(f64::max, |p| p.0));
    let (rymin, rymax) = (pick(f64::min, |p| p.1), pick(f64::max, |p| p.1));
    let (width_rot, height_rot) = (rxmax - rxmin, rymax - rymin);

    let (offset_x, offset_y) = if style.rotation_mode == RotationMode::Anchor {
        let (x0, y0, x1, y1) = (xmin, ymin, xmax, ymax);
        let ox = match style.halign {
            HAlign::Left => x0,
            HAlign::Right => x1,
            HAlign::Center => f64::midpoint(x0, x1),
        };
        let oy = match style.valign {
            VAlign::Bottom => y0,
            VAlign::Top => y1,
            VAlign::Center => f64::midpoint(y0, y1),
            VAlign::Baseline => y1 - baseline,
            VAlign::CenterBaseline => y1 - baseline / 2.0,
        };
        rotate(angle, ox, oy)
    } else {
        let mut halign = style.halign;
        let mut valign = style.valign;
        match style.rotation_mode {
            RotationMode::XTick => halign = ha_for_angle(angle, style.valign),
            RotationMode::YTick => valign = va_for_angle(angle, style.halign),
            RotationMode::Default | RotationMode::Anchor => {}
        }
        let ox = match halign {
            HAlign::Left => rxmin,
            HAlign::Right => rxmax,
            HAlign::Center => f64::midpoint(rxmin, rxmax),
        };
        let oy = match valign {
            VAlign::Bottom => rymin,
            VAlign::Top => rymax,
            VAlign::Center => f64::midpoint(rymin, rymax),
            VAlign::Baseline => rymin + descent,
            VAlign::CenterBaseline => rymin + height_rot - baseline / 2.0,
        };
        (ox, oy)
    };

    let lines = stacked
        .iter()
        .zip(&offsets)
        .map(|(&(w, a, d, _), &(x, y))| {
            let (rx, ry) = rotate(angle, x, y);
            LinePlacement {
                width: w,
                ascent: a,
                descent: d,
                origin: Point::new(rx - offset_x, ry - offset_y),
            }
        })
        .collect();
    let corner = rotated[0];
    BlockLayout {
        bbox: Bbox::from_bounds(rxmin - offset_x, rymin - offset_y, width_rot, height_rot),
        lines,
        frame: (
            Point::new(corner.0 - offset_x, corner.1 - offset_y),
            (xmax - xmin, ymax - ymin),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: FontHeightMetrics = FontHeightMetrics {
        ascent: 10.0,
        descent: 3.0,
        line_gap: 1.0,
    };

    fn line(width: f64) -> LineMetrics {
        LineMetrics {
            width,
            height: 9.0,
            descent: 2.0,
        }
    }

    #[test]
    fn single_line_baseline_alignment() {
        let layout = layout_block(&[line(40.0)], FONT, &BlockStyle::default());
        // ascent = max(7, 10) = 10, descent = max(2, 3) = 3, no line gap.
        assert_eq!(layout.bbox, Bbox::from_bounds(0.0, -3.0, 40.0, 13.0));
        assert_eq!(layout.lines[0].origin, Point::new(0.0, 0.0));
    }

    #[test]
    fn lines_stack_with_the_font_gap() {
        let style = BlockStyle {
            valign: VAlign::Top,
            multialign: HAlign::Center,
            ..BlockStyle::default()
        };
        let layout = layout_block(&[line(40.0), line(20.0)], FONT, &style);
        assert_eq!(layout.lines[0].origin, Point::new(0.0, -10.5));
        assert_eq!(layout.lines[1].origin, Point::new(10.0, -10.5 - 3.5 - 10.5));
        assert_eq!(layout.bbox.y1(), 0.0);
    }

    #[test]
    fn rotation_by_ninety_degrees() {
        let style = BlockStyle {
            rotation: 90.0,
            halign: HAlign::Center,
            valign: VAlign::Center,
            ..BlockStyle::default()
        };
        let layout = layout_block(&[line(40.0)], FONT, &style);
        assert!(
            (layout.bbox.width() - 13.0).abs() < 1e-9 && (layout.bbox.height() - 40.0).abs() < 1e-9
        );
        assert!(
            layout.bbox.center().length() < 1e-9,
            "centred on the anchor"
        );
    }

    #[test]
    fn tick_rotation_modes_pick_alignment() {
        assert_eq!(ha_for_angle(45.0, VAlign::Top), HAlign::Right);
        assert_eq!(ha_for_angle(45.0, VAlign::Bottom), HAlign::Left);
        assert_eq!(ha_for_angle(90.0, VAlign::Top), HAlign::Center);
        assert_eq!(va_for_angle(30.0, HAlign::Left), VAlign::Baseline);
        assert_eq!(va_for_angle(120.0, HAlign::Right), VAlign::Baseline);
    }
}
