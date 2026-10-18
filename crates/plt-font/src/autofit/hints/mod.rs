//! Glyph hints: the outline's points with their topology, and the
//! segments and edges found on each axis (FreeType's `afhints.c`).
//!
//! Points, segments and edges refer to each other by index, where FreeType
//! uses pointers.

mod align;
mod reload;

use crate::error::Result;
use crate::face::Face;
use crate::fixed::{div_fix, mul_fix};
use crate::outline::PointKind;
use crate::types::GlyphId;

/// Point flags (`AF_FLAG_*`).
pub(crate) mod point_flag {
    /// A quadratic control point.
    pub(crate) const CONIC: u16 = 1 << 0;
    /// A cubic control point.
    pub(crate) const CUBIC: u16 = 1 << 1;
    /// Any control point.
    pub(crate) const CONTROL: u16 = CONIC | CUBIC;
    /// Grid-fitted horizontally.
    pub(crate) const TOUCH_X: u16 = 1 << 2;
    /// Grid-fitted vertically.
    pub(crate) const TOUCH_Y: u16 = 1 << 3;
    /// Interpolated from its contour neighbours.
    pub(crate) const WEAK_INTERPOLATION: u16 = 1 << 4;
    /// Very near to the next point.
    pub(crate) const NEAR: u16 = 1 << 5;
    /// Never part of a segment.
    pub(crate) const IGNORE: u16 = 1 << 6;
}

/// Edge and segment flags (`AF_EDGE_*`).
pub(crate) mod edge_flag {
    /// Made of round segments.
    pub(crate) const ROUND: u8 = 1 << 0;
    /// A serif.
    pub(crate) const SERIF: u8 = 1 << 1;
    /// Already grid-fitted.
    pub(crate) const DONE: u8 = 1 << 2;
    /// Aligned to a neutral blue zone.
    pub(crate) const NEUTRAL: u8 = 1 << 3;
    /// Never aligned to a blue zone.
    pub(crate) const NO_BLUE: u8 = 1 << 4;
}

/// The axis hints work on: x coordinates (vertical segments and edges)
/// or y coordinates (horizontal ones).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Dimension {
    /// x coordinates (`AF_DIMENSION_HORZ`).
    Horizontal,
    /// y coordinates (`AF_DIMENSION_VERT`).
    Vertical,
}

impl Dimension {
    /// Both dimensions, horizontal first, as FreeType loops over them.
    pub(crate) const BOTH: [Self; 2] = [Self::Horizontal, Self::Vertical];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Horizontal => 0,
            Self::Vertical => 1,
        }
    }
}

/// A hint direction (`AF_Direction`); two directions are opposite when
/// their values sum to zero.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Direction(i8);

impl Direction {
    pub(crate) const NONE: Self = Self(4);
    pub(crate) const RIGHT: Self = Self(1);
    pub(crate) const LEFT: Self = Self(-1);
    pub(crate) const UP: Self = Self(2);
    pub(crate) const DOWN: Self = Self(-2);

    /// The direction with a positive value (`FT_ABS`).
    pub(crate) const fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Whether `self` and `other` point in opposite directions.
    pub(crate) const fn is_opposite(self, other: Self) -> bool {
        self.0 + other.0 == 0
    }

    /// The direction of the vector `(dx, dy)`, or none when it is not
    /// within about four degrees of an axis (`af_direction_compute`).
    pub(crate) fn of(dx: i64, dy: i64) -> Self {
        let (dir, long, short) = if dy >= dx {
            if dy >= -dx {
                (Self::UP, dy, dx)
            } else {
                (Self::LEFT, -dx, dy)
            }
        } else if dy >= -dx {
            (Self::RIGHT, dx, dy)
        } else {
            (Self::DOWN, -dy, dx)
        };
        if long <= short.wrapping_abs().wrapping_mul(14) {
            Self::NONE
        } else {
            dir
        }
    }
}

/// One outline point (`AF_PointRec`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Point {
    pub(crate) flags: u16,
    pub(crate) in_dir: Direction,
    pub(crate) out_dir: Direction,
    /// Original scaled position.
    pub(crate) ox: i64,
    pub(crate) oy: i64,
    /// Original position in font units.
    pub(crate) fx: i16,
    pub(crate) fy: i16,
    /// Current position.
    pub(crate) x: i64,
    pub(crate) y: i64,
    /// Scratch values: the current and original coordinate along the axis
    /// being processed, or index deltas while the topology is analysed.
    pub(crate) u: i64,
    pub(crate) v: i64,
    /// Next and previous point of the contour.
    pub(crate) next: usize,
    pub(crate) prev: usize,
}

/// A run of points aligned along an axis (`AF_SegmentRec`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Segment {
    pub(crate) flags: u8,
    pub(crate) dir: Direction,
    /// Position across the axis, in font units.
    pub(crate) pos: i16,
    /// Deviation from `pos`.
    pub(crate) delta: i16,
    /// Extent along the axis.
    pub(crate) min_coord: i16,
    pub(crate) max_coord: i16,
    /// The segment's length, slightly enlarged for serif detection.
    pub(crate) height: i16,
    /// The edge the segment belongs to.
    pub(crate) edge: Option<usize>,
    /// The next segment of that edge (a circular list).
    pub(crate) edge_next: usize,
    /// The segment this one forms a stem with.
    pub(crate) link: Option<usize>,
    /// The primary segment of a serif.
    pub(crate) serif: Option<usize>,
    pub(crate) score: i64,
    pub(crate) len: i64,
    pub(crate) first: usize,
    pub(crate) last: usize,
}

impl Segment {
    /// A segment starting and ending at `point`, all else cleared.
    pub(crate) const fn new(dir: Direction, point: usize) -> Self {
        Self {
            flags: 0,
            dir,
            pos: 0,
            delta: 0,
            min_coord: 0,
            max_coord: 0,
            height: 0,
            edge: None,
            edge_next: 0,
            link: None,
            serif: None,
            score: 32000,
            len: 0,
            first: point,
            last: point,
        }
    }
}

/// The distance between the positions of two segments
/// (`AF_SEGMENT_DIST`).
pub(crate) fn segment_distance(a: &Segment, b: &Segment) -> i64 {
    (i64::from(a.pos) - i64::from(b.pos)).abs()
}

/// Segments at one position, grid-fitted together (`AF_EdgeRec`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Edge {
    /// Original position in font units.
    pub(crate) fpos: i16,
    /// Original scaled position.
    pub(crate) opos: i64,
    /// Current position.
    pub(crate) pos: i64,
    pub(crate) flags: u8,
    pub(crate) dir: Direction,
    /// Cached interpolation scale to the next edge.
    pub(crate) scale: i64,
    /// The fitted position of the blue zone the edge aligns to.
    pub(crate) blue: Option<i64>,
    /// The edge this one forms a stem with.
    pub(crate) link: Option<usize>,
    /// The primary edge of a serif.
    pub(crate) serif: Option<usize>,
    /// First and last segment of the edge.
    pub(crate) first: usize,
    pub(crate) last: usize,
}

impl Edge {
    /// An edge made of `segment` alone, at font-unit position `fpos`.
    pub(crate) fn new(segment: usize, dir: Direction, fpos: i16, scale: i64) -> Self {
        let opos = mul_fix(i64::from(fpos), scale);
        Self {
            fpos,
            opos,
            pos: opos,
            flags: 0,
            dir,
            scale: 0,
            blue: None,
            link: None,
            serif: None,
            first: segment,
            last: segment,
        }
    }
}

/// The segments and edges of one dimension (`AF_AxisHintsRec`).
#[derive(Clone, Debug, Default)]
pub(crate) struct AxisHints {
    pub(crate) segments: Vec<Segment>,
    /// Edges sorted by position.
    pub(crate) edges: Vec<Edge>,
    /// The direction of segments on the left side of a stem.
    pub(crate) major_dir: Direction,
}

impl Default for Direction {
    fn default() -> Self {
        Self::NONE
    }
}

impl AxisHints {
    /// Inserts a new edge at `fpos`, keeping the edges sorted (ascending,
    /// or descending for top-to-bottom hinting), and returns its index
    /// (`af_axis_hints_new_edge`). Among edges at the same position, one
    /// in the major direction goes after those already there.
    pub(crate) fn insert_edge(&mut self, edge: Edge, top_to_bottom: bool) -> usize {
        let mut index = self.edges.len();
        while index > 0 {
            let before = self.edges[index - 1].fpos;
            if (top_to_bottom && before > edge.fpos) || (!top_to_bottom && before < edge.fpos) {
                break;
            }
            if before == edge.fpos && edge.dir == self.major_dir {
                break;
            }
            index -= 1;
        }
        self.edges.insert(index, edge);
        index
    }

    /// Points every segment of an edge back at the edge.
    pub(crate) fn link_edge_segments(&mut self) {
        for edge in 0..self.edges.len() {
            let members: Vec<usize> = self.edge_segments(edge).collect();
            for segment in members {
                self.segments[segment].edge = Some(edge);
            }
        }
    }

    /// Moves edge `serif` by the distance its `base` edge has moved
    /// (`af_latin_align_serif_edge`, `af_cjk_align_serif_edge`).
    pub(crate) fn align_serif(&mut self, base: usize, serif: usize) {
        let base = self.edges[base];
        let serif = &mut self.edges[serif];
        serif.pos = base.pos + (serif.opos - base.opos);
    }

    /// The segments of edge `edge`, starting with its first one.
    pub(crate) fn edge_segments(&self, edge: usize) -> EdgeSegments<'_> {
        let first = self.edges[edge].first;
        EdgeSegments {
            axis: self,
            first,
            next: Some(first),
        }
    }
}

/// Iterator over the circular segment list of an edge.
pub(crate) struct EdgeSegments<'a> {
    axis: &'a AxisHints,
    first: usize,
    next: Option<usize>,
}

impl Iterator for EdgeSegments<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let current = self.next?;
        let following = self.axis.segments[current].edge_next;
        self.next = (following != self.first).then_some(following);
        Some(current)
    }
}

/// The latin and CJK hinters' flags (`AF_LATIN_HINTS_*`), derived from
/// the target.
pub(crate) mod other_flag {
    /// Snap vertical stem widths to whole pixels.
    pub(crate) const HORZ_SNAP: u32 = 1 << 0;
    /// Snap horizontal stem heights to whole pixels.
    pub(crate) const VERT_SNAP: u32 = 1 << 1;
    /// Adjust stem widths to the grid.
    pub(crate) const STEM_ADJUST: u32 = 1 << 2;
    /// Monochrome rendering.
    pub(crate) const MONO: u32 = 1 << 3;
}

/// Scaler flags (`AF_SCALER_FLAG_*`).
pub(crate) mod scaler_flag {
    /// No horizontal hinting.
    pub(crate) const NO_HORIZONTAL: u32 = 1;
    /// No advance width hinting.
    pub(crate) const NO_ADVANCE: u32 = 4;
}

/// The hints of one glyph (`AF_GlyphHintsRec`).
#[derive(Clone, Debug, Default)]
pub(crate) struct GlyphHints {
    pub(crate) x_scale: i64,
    pub(crate) y_scale: i64,
    pub(crate) points: Vec<Point>,
    /// First point of each contour.
    pub(crate) contours: Vec<usize>,
    /// Vertical extrema of each contour, when computed.
    pub(crate) contour_y_minima: Vec<i64>,
    pub(crate) contour_y_maxima: Vec<i64>,
    pub(crate) axis: [AxisHints; 2],
    /// [`scaler_flag`]s.
    pub(crate) scaler_flags: u32,
    /// Writing-system specific flags.
    pub(crate) other_flags: u32,
}

impl GlyphHints {
    /// Hints scaled by `x_scale`/`y_scale` (16.16) with `scaler_flags`.
    pub(crate) fn new(x_scale: i64, y_scale: i64, scaler_flags: u32) -> Self {
        Self {
            x_scale,
            y_scale,
            scaler_flags,
            ..Self::default()
        }
    }

    pub(crate) fn axis(&self, dim: Dimension) -> &AxisHints {
        &self.axis[dim.index()]
    }

    pub(crate) fn axis_mut(&mut self, dim: Dimension) -> &mut AxisHints {
        &mut self.axis[dim.index()]
    }

    /// The scale of `dim`.
    pub(crate) const fn scale(&self, dim: Dimension) -> i64 {
        match dim {
            Dimension::Horizontal => self.x_scale,
            Dimension::Vertical => self.y_scale,
        }
    }

    /// Whether horizontal hinting is on (`AF_HINTS_DO_HORIZONTAL`).
    pub(crate) const fn do_horizontal(&self) -> bool {
        self.scaler_flags & scaler_flag::NO_HORIZONTAL == 0
    }

    /// Whether the advance width is hinted (`AF_HINTS_DO_ADVANCE`).
    pub(crate) const fn do_advance(&self) -> bool {
        self.scaler_flags & scaler_flag::NO_ADVANCE == 0
    }

    /// Whether writing-system flag `flag` is set.
    pub(crate) const fn has(&self, flag: u32) -> bool {
        self.other_flags & flag != 0
    }

    /// Whether `dim` is hinted.
    pub(crate) const fn hints(&self, dim: Dimension) -> bool {
        match dim {
            Dimension::Horizontal => self.do_horizontal(),
            Dimension::Vertical => true,
        }
    }

    /// The points of the contour starting at `first`, beginning with the
    /// one after `first` and ending with `first` (FreeType's usual
    /// `do { p = p->next; ... } while ( p != first )`).
    pub(crate) fn contour_from(&self, first: usize) -> ContourPoints<'_> {
        ContourPoints {
            points: &self.points,
            first,
            current: Some(first),
        }
    }

    /// The interpolation scale between `before` and `after` on `dim`,
    /// cached in `before` (used by strong point alignment).
    pub(crate) fn edge_scale(&mut self, dim: Dimension, before: usize, after: usize) -> i64 {
        let edges = &mut self.axis[dim.index()].edges;
        if edges[before].scale == 0 {
            let (b, a) = (edges[before], edges[after]);
            edges[before].scale = div_fix(a.pos - b.pos, i64::from(a.fpos) - i64::from(b.fpos));
        }
        edges[before].scale
    }
}

/// Iterator over a contour's points, see [`GlyphHints::contour_from`].
pub(crate) struct ContourPoints<'a> {
    points: &'a [Point],
    first: usize,
    current: Option<usize>,
}

impl Iterator for ContourPoints<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let next = self.points[self.current?].next;
        self.current = (next != self.first).then_some(next);
        Some(next)
    }
}

/// An outline in font units as the autohinter reads it: points, their
/// kinds and the index of the last point of each contour.
#[derive(Clone, Debug, Default)]
pub(crate) struct FontOutline {
    pub(crate) points: Vec<(i64, i64)>,
    pub(crate) kinds: Vec<PointKind>,
    pub(crate) contour_ends: Vec<usize>,
}

impl FontOutline {
    /// Loads `glyph` as `FT_Load_Glyph` with `FT_LOAD_NO_SCALE` does;
    /// returns the outline and the advance width in font units.
    pub(crate) fn load(face: &Face<'_>, glyph: GlyphId) -> Result<(Self, i64)> {
        let (outline, advance) = face.load_unscaled(glyph)?;
        let points = outline
            .points
            .iter()
            .map(|point| (point.x as i64, point.y as i64))
            .collect();
        let kinds = outline.points.iter().map(|point| point.kind).collect();
        let contour_ends = outline.contour_ends.iter().map(|&end| end - 1).collect();
        Ok((
            Self {
                points,
                kinds,
                contour_ends,
            },
            advance,
        ))
    }

    /// Whether point `index` is on the curve.
    pub(crate) fn is_on(&self, index: usize) -> bool {
        self.kinds[index] == PointKind::OnCurve
    }
}
