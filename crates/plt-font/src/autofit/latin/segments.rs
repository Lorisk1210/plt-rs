//! Segments and their links (`af_latin_hints_compute_segments`,
//! `af_latin_hints_link_segments`), which the CJK writing system shares.

use crate::autofit::hints::{Dimension, GlyphHints, Point, Segment, edge_flag, point_flag};
use crate::autofit::widths::Width;

/// The threshold below which the on points of a segment make it round
/// (`FLAT_THRESHOLD`).
pub(crate) fn flat_threshold(units_per_em: u16) -> i64 {
    i64::from(units_per_em) / 14
}

/// A constant tuned for 2048 units per em, for a face of `units_per_em`
/// (`AF_LATIN_CONSTANT`).
pub(crate) fn latin_constant(units_per_em: u16, value: i64) -> i64 {
    value * i64::from(units_per_em) / 2048
}

/// The running extent of the segment being built.
#[derive(Clone, Copy)]
struct Extent {
    min_pos: i64,
    max_pos: i64,
    min_coord: i64,
    max_coord: i64,
    min_flags: u16,
    max_flags: u16,
    min_on_coord: i64,
    max_on_coord: i64,
}

impl Extent {
    const EMPTY: Self = Self {
        min_pos: 32000,
        max_pos: -32000,
        min_coord: 32000,
        max_coord: -32000,
        min_flags: 0,
        max_flags: 0,
        min_on_coord: 32000,
        max_on_coord: -32000,
    };

    /// The extent of a segment starting at `point`.
    fn start(point: &Point) -> Self {
        let on = point.flags & point_flag::CONTROL == 0;
        Self {
            min_pos: point.u,
            max_pos: point.u,
            min_coord: point.v,
            max_coord: point.v,
            min_flags: point.flags,
            max_flags: point.flags,
            min_on_coord: if on { point.v } else { 32000 },
            max_on_coord: if on { point.v } else { -32000 },
        }
    }

    fn add(&mut self, point: &Point) {
        self.min_pos = self.min_pos.min(point.u);
        self.max_pos = self.max_pos.max(point.u);
        if point.v < self.min_coord {
            self.min_coord = point.v;
            self.min_flags = point.flags;
        }
        if point.v > self.max_coord {
            self.max_coord = point.v;
            self.max_flags = point.flags;
        }
        if point.flags & point_flag::CONTROL == 0 {
            self.min_on_coord = self.min_on_coord.min(point.v);
            self.max_on_coord = self.max_on_coord.max(point.v);
        }
    }

    /// Widens this extent with `other`'s (positions, coordinates with
    /// their flags, on-point coordinates).
    fn merge(&mut self, other: &Self) {
        self.min_pos = self.min_pos.min(other.min_pos);
        self.max_pos = self.max_pos.max(other.max_pos);
        if other.min_coord < self.min_coord {
            self.min_coord = other.min_coord;
            self.min_flags = other.min_flags;
        }
        if other.max_coord > self.max_coord {
            self.max_coord = other.max_coord;
            self.max_flags = other.max_flags;
        }
        self.min_on_coord = self.min_on_coord.min(other.min_on_coord);
        self.max_on_coord = self.max_on_coord.max(other.max_on_coord);
    }

    fn is_round(&self, flat_threshold: i64) -> bool {
        (self.min_flags | self.max_flags) & point_flag::CONTROL != 0
            && self.max_on_coord - self.min_on_coord < flat_threshold
    }

    /// Sets the position and deviation of `segment` from the positions.
    fn place(&self, segment: &mut Segment) {
        segment.pos = ((self.min_pos + self.max_pos) >> 1) as i16;
        segment.delta = ((self.max_pos - self.min_pos) >> 1) as i16;
    }

    /// Sets the coordinates and height of `segment`.
    fn span(&self, segment: &mut Segment) {
        segment.min_coord = self.min_coord as i16;
        segment.max_coord = self.max_coord as i16;
        segment.height = (i32::from(segment.max_coord) - i32::from(segment.min_coord)) as i16;
    }
}

/// Finds the segments of `dim`: runs of points whose out direction is
/// the axis' major direction (or its opposite). More than 1000 segments
/// suppress hinting of the axis.
pub(crate) fn compute_segments(hints: &mut GlyphHints, dim: Dimension, units_per_em: u16) {
    let flat = flat_threshold(units_per_em);
    let major_dir = hints.axis(dim).major_dir.abs();
    hints.axis_mut(dim).segments.clear();
    for point in &mut hints.points {
        (point.u, point.v) = match dim {
            Dimension::Horizontal => (i64::from(point.fx), i64::from(point.fy)),
            Dimension::Vertical => (i64::from(point.fy), i64::from(point.fx)),
        };
    }
    for contour in 0..hints.contours.len() {
        if !contour_segments(hints, dim, hints.contours[contour], major_dir, flat) {
            hints.axis_mut(dim).segments.clear();
            return;
        }
    }
    let GlyphHints { points, axis, .. } = hints;
    for segment in &mut axis[dim.index()].segments {
        let (first, last) = (points[segment.first], points[segment.last]);
        let (before, after) = (points[first.prev].v, points[last.next].v);
        let mut height = i64::from(segment.height);
        if first.v < last.v {
            if before < first.v {
                height += (first.v - before) >> 1;
            }
            if after > last.v {
                height += (after - last.v) >> 1;
            }
        } else {
            if before > first.v {
                height += (before - first.v) >> 1;
            }
            if after < last.v {
                height += (last.v - after) >> 1;
            }
        }
        segment.height = height as i16;
    }
}

/// The segments of the contour starting at `start`; false when there are
/// too many.
fn contour_segments(
    hints: &mut GlyphHints,
    dim: Dimension,
    start: usize,
    major_dir: crate::autofit::hints::Direction,
    flat: i64,
) -> bool {
    let GlyphHints { points, axis, .. } = hints;
    let segments = &mut axis[dim.index()].segments;
    let mut point = start;
    let mut last = points[point].prev;
    if points[point].flags & point_flag::IGNORE == 0
        && points[last].out_dir.abs() == major_dir
        && points[point].out_dir.abs() == major_dir
    {
        // Already on an edge: find its start.
        last = point;
        loop {
            point = points[point].prev;
            if points[point].out_dir.abs() != major_dir {
                point = points[point].next;
                break;
            }
            if point == last {
                break;
            }
        }
    }
    let last = point;
    let mut passed = false;
    let mut on_edge = false;
    let mut segment_dir = major_dir;
    let mut extent = Extent::EMPTY;
    let mut prev_extent = Extent::EMPTY;
    let mut prev_segment: Option<usize> = None;
    loop {
        let p = points[point];
        if on_edge {
            extent.add(&p);
            if p.flags & point_flag::IGNORE != 0 || p.out_dir != segment_dir || point == last {
                let current = segments.len() - 1;
                let joins_previous = prev_segment.filter(|&prev| {
                    p.flags & point_flag::IGNORE == 0
                        && segments[current].first == segments[prev].last
                });
                match joins_previous {
                    None => {
                        let segment = &mut segments[current];
                        segment.last = point;
                        extent.place(segment);
                        if extent.is_round(flat) {
                            segment.flags |= edge_flag::ROUND;
                        }
                        extent.span(segment);
                        prev_segment = Some(current);
                        prev_extent = extent;
                    }
                    Some(prev) => {
                        merge_segments(
                            points,
                            segments,
                            prev,
                            point,
                            &mut extent,
                            &mut prev_extent,
                            flat,
                        );
                        segments.pop();
                    }
                }
                on_edge = false;
            }
        }
        if point == last {
            if passed {
                break;
            }
            passed = true;
        }
        if p.flags & point_flag::IGNORE == 0
            && !on_edge
            && (p.out_dir.abs() == major_dir || point == p.prev)
        {
            if segments.len() > 1000 {
                return false;
            }
            segment_dir = p.out_dir;
            segments.push(Segment::new(segment_dir, point));
            if prev_segment.is_some() {
                prev_segment = Some(segments.len() - 2);
            }
            extent = Extent::start(&p);
            on_edge = true;
            if point == p.prev {
                // A one-point contour.
                let segment = segments.last_mut().expect("just pushed");
                segment.pos = extent.min_pos as i16;
                if p.flags & point_flag::CONTROL != 0 {
                    segment.flags |= edge_flag::ROUND;
                }
                segment.min_coord = p.v as i16;
                segment.max_coord = p.v as i16;
                segment.height = 0;
                on_edge = false;
            }
        }
        point = p.next;
    }
    true
}

/// Merges the segment just finished (the last one, ending at `point`)
/// into `prev`, which ends where it starts: one segment unifies both when
/// they run the same way, else the longer one survives in `prev`'s place.
fn merge_segments(
    points: &[Point],
    segments: &mut [Segment],
    prev: usize,
    point: usize,
    extent: &mut Extent,
    prev_extent: &mut Extent,
    flat: i64,
) {
    let current = segments.len() - 1;
    if points[segments[prev].last].in_dir == points[point].in_dir {
        extent.merge(prev_extent);
        let segment = &mut segments[prev];
        segment.last = point;
        extent.place(segment);
        if extent.is_round(flat) {
            segment.flags |= edge_flag::ROUND;
        } else {
            segment.flags &= !edge_flag::ROUND;
        }
        extent.span(segment);
    } else if (prev_extent.max_coord - prev_extent.min_coord).abs()
        > (extent.max_coord - extent.min_coord).abs()
    {
        // Keep the previous segment.
        prev_extent.min_pos = prev_extent.min_pos.min(extent.min_pos);
        prev_extent.max_pos = prev_extent.max_pos.max(extent.max_pos);
        let segment = &mut segments[prev];
        segment.last = point;
        prev_extent.place(segment);
    } else {
        // Keep the current segment, in the previous one's place.
        extent.min_pos = extent.min_pos.min(prev_extent.min_pos);
        extent.max_pos = extent.max_pos.max(prev_extent.max_pos);
        let segment = &mut segments[current];
        segment.last = point;
        extent.place(segment);
        if extent.is_round(flat) {
            segment.flags |= edge_flag::ROUND;
        }
        extent.span(segment);
        segments[prev] = segments[current];
        *prev_extent = *extent;
    }
}

/// Links segments facing each other into stems, scoring overlap and
/// distance (with the standard `widths`, if any, as the reference), then
/// turns one-sided links into serifs.
pub(crate) fn link_segments(
    hints: &mut GlyphHints,
    dim: Dimension,
    widths: &[Width],
    units_per_em: u16,
) {
    let axis = hints.axis_mut(dim);
    let major_dir = axis.major_dir;
    let segments = &mut axis.segments;
    let max_width = widths.last().map_or(0, |width| width.org);
    let len_threshold = latin_constant(units_per_em, 8).max(1);
    let len_score = latin_constant(units_per_em, 6000);
    let dist_score = 3000;
    for seg1 in 0..segments.len() {
        if segments[seg1].dir != major_dir {
            continue;
        }
        for seg2 in 0..segments.len() {
            let (a, b) = (segments[seg1], segments[seg2]);
            let (pos1, pos2) = (i64::from(a.pos), i64::from(b.pos));
            if !a.dir.is_opposite(b.dir) || pos2 <= pos1 {
                continue;
            }
            let min = a.min_coord.max(b.min_coord);
            let max = a.max_coord.min(b.max_coord);
            let len = i64::from(max) - i64::from(min);
            if len < len_threshold {
                continue;
            }
            let dist = pos2 - pos1;
            let dist_demerit = if max_width != 0 {
                let delta = (dist << 10) / max_width - (1 << 10);
                if delta > 10000 {
                    32000
                } else if delta > 0 {
                    delta * delta / dist_score
                } else {
                    0
                }
            } else {
                dist
            };
            let score = dist_demerit + len_score / len;
            if score < segments[seg1].score {
                segments[seg1].score = score;
                segments[seg1].link = Some(seg2);
            }
            if score < segments[seg2].score {
                segments[seg2].score = score;
                segments[seg2].link = Some(seg1);
            }
        }
    }
    for seg1 in 0..segments.len() {
        if let Some(seg2) = segments[seg1].link
            && segments[seg2].link != Some(seg1)
        {
            segments[seg1].link = None;
            segments[seg1].serif = segments[seg2].link;
        }
    }
}
