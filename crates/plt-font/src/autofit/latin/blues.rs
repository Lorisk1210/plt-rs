//! Measuring a latin style's blue zones on its blue characters
//! (`af_latin_metrics_init_blues`): flat extrema give a zone's reference
//! position, round ones its overshoot.

use super::metrics::{LatinBlue, blue_flag};
use super::segments::flat_threshold;
use crate::autofit::hints::FontOutline;
use crate::autofit::metrics::MetricsContext;
use crate::autofit::shaper::{Clusters, shape_cluster};
use crate::autofit::tables::{BlueString, blue};
use crate::autofit::widths::Width;
use crate::types::GlyphId;

/// The extreme point of one glyph for a zone.
struct Extremum {
    y: i64,
    round: bool,
}

/// The blue zones of the style, or none (the style is then unhinted).
pub(crate) fn blue_zones(context: &MetricsContext<'_, '_>) -> Vec<LatinBlue> {
    let units_per_em = context.face.units_per_em();
    let mut blues = Vec::new();
    for &string in context.class.blues {
        let mut zone = ZoneMeasure::default();
        for cluster in Clusters::new(string.chars) {
            zone.add_cluster(context, string, cluster, units_per_em);
        }
        if let Some(blue) = zone.finish(string) {
            blues.push(blue);
        }
    }
    order_blues(&mut blues);
    blues
}

/// Whether a zone is measured upwards (top and sub-top zones).
fn upwards(string: BlueString) -> bool {
    string.has(blue::LATIN_TOP) || string.has(blue::LATIN_SUB_TOP)
}

/// The extrema measured for one blue string.
#[derive(Default)]
struct ZoneMeasure {
    flats: Vec<i64>,
    rounds: Vec<i64>,
    ascender: i64,
    descender: i64,
}

impl ZoneMeasure {
    /// Measures the glyphs of `cluster` and keeps the most extreme one
    /// (in FreeType's comparison, which looks up only for top zones).
    fn add_cluster(
        &mut self,
        context: &MetricsContext<'_, '_>,
        string: BlueString,
        cluster: &str,
        units_per_em: u16,
    ) {
        let glyphs = shape_cluster(context.shaping, cluster, context.coverage());
        if glyphs.is_empty() {
            return;
        }
        let top = string.has(blue::LATIN_TOP);
        let mut best = if top { i64::MIN } else { i64::MAX };
        let mut best_round = false;
        for glyph in glyphs {
            if glyph.glyph == GlyphId::NOTDEF {
                continue;
            }
            let Ok((outline, _)) = FontOutline::load(context.face, glyph.glyph) else {
                continue;
            };
            if outline.points.len() <= 2 {
                continue;
            }
            let y_offset = i64::from(glyph.y_offset);
            let Some(extremum) = self.measure(&outline, string, y_offset, units_per_em) else {
                continue;
            };
            if (top && extremum.y > best) || (!top && extremum.y < best) {
                best = extremum.y;
                best_round = extremum.round;
            }
        }
        if best != i64::MIN && best != i64::MAX {
            if best_round {
                self.rounds.push(best);
            } else {
                self.flats.push(best);
            }
        }
    }

    /// The extremum of one glyph, `None` when FreeType skips the glyph (a
    /// long zone on a degenerate contour, or a round extremum for a neutral
    /// zone).
    fn measure(
        &mut self,
        outline: &FontOutline,
        string: BlueString,
        y_offset: i64,
        units_per_em: u16,
    ) -> Option<Extremum> {
        let up = upwards(string);
        let mut best: Option<(usize, i64)> = None;
        let mut contour: Option<(usize, usize)> = None;
        let mut start = 0;
        for &last in &outline.contour_ends {
            let first = start;
            start = last + 1;
            // Single-point contours are never rasterized (they may be far
            // away mark attachment points).
            if last <= first {
                continue;
            }
            for index in first..=last {
                let y = outline.points[index].1;
                let better =
                    best.is_none_or(|(_, best_y)| if up { y > best_y } else { y < best_y });
                if better {
                    best = Some((index, y));
                }
                if better == up {
                    self.ascender = self.ascender.max(y + y_offset);
                } else {
                    self.descender = self.descender.min(y + y_offset);
                }
            }
            let best_index = best.map_or(-1, |(index, _)| index as i64);
            if best_index > contour.map_or(-1, |(_, last)| last as i64) {
                contour = Some((first, last));
            }
        }
        let (Some((best_point, best_y)), Some((first, last))) = (best, contour) else {
            return Some(Extremum { y: 0, round: false });
        };
        let walk = ContourWalk {
            outline,
            first,
            last,
        };
        let mut segment = walk.flat_segment(best_point, best_y);
        if string.has(blue::LATIN_LONG) {
            walk.lengthen(&mut segment, best_point, units_per_em)?;
        }
        let points = &outline.points;
        let round = match (segment.first_on, segment.last_on) {
            (Some(a), Some(b))
                if (points[b].0 - points[a].0).abs() > flat_threshold(units_per_em) =>
            {
                false
            }
            _ => !outline.is_on(segment.first) || !outline.is_on(segment.last),
        };
        if round && string.has(blue::LATIN_NEUTRAL) {
            return None;
        }
        Some(Extremum {
            y: segment.y + y_offset,
            round,
        })
    }

    /// The zone from the measured extrema: the medians of the flat and
    /// round positions, corrected when the overshoot is on the wrong side.
    fn finish(self, string: BlueString) -> Option<LatinBlue> {
        let Self {
            mut flats,
            mut rounds,
            ascender,
            descender,
        } = self;
        if flats.is_empty() && rounds.is_empty() {
            return None;
        }
        flats.sort_unstable();
        rounds.sort_unstable();
        let median = |values: &[i64]| values[values.len() / 2];
        let (mut reference, mut overshoot) = if flats.is_empty() {
            (median(&rounds), median(&rounds))
        } else if rounds.is_empty() {
            (median(&flats), median(&flats))
        } else {
            (median(&flats), median(&rounds))
        };
        if overshoot != reference && upwards(string) ^ (overshoot > reference) {
            reference = i64::midpoint(overshoot, reference);
            overshoot = reference;
        }
        let property_flags = [
            (blue::LATIN_TOP, blue_flag::TOP),
            (blue::LATIN_SUB_TOP, blue_flag::SUB_TOP),
            (blue::LATIN_NEUTRAL, blue_flag::NEUTRAL),
            (blue::LATIN_CAPITAL_BOTTOM, blue_flag::BOTTOM),
            (blue::LATIN_SMALL_BOTTOM, blue_flag::BOTTOM_SMALL),
            (blue::LATIN_X_HEIGHT, blue_flag::ADJUSTMENT),
        ];
        let flags = property_flags
            .iter()
            .filter(|&&(property, _)| string.has(property))
            .fold(0, |flags, &(_, flag)| flags | flag);
        let width = |org| Width {
            org,
            cur: 0,
            fit: 0,
        };
        Some(LatinBlue {
            reference: width(reference),
            overshoot: width(overshoot),
            ascender,
            descender,
            flags,
        })
    }
}

/// Whether a zone's top is its overshoot (top zones) or its reference.
const fn top_is_overshoot(blue: &LatinBlue) -> bool {
    blue.flags & (blue_flag::TOP | blue_flag::SUB_TOP) != 0
}

/// Makes zones not overlap: sorted by their bottom (the reference of top
/// zones, the overshoot of the others), each zone's top is lowered to the
/// next one's when above it.
fn order_blues(blues: &mut [LatinBlue]) {
    let bottom = |blue: &LatinBlue| {
        if top_is_overshoot(blue) {
            blue.reference.org
        } else {
            blue.overshoot.org
        }
    };
    let top = |blue: &LatinBlue| {
        if top_is_overshoot(blue) {
            blue.overshoot.org
        } else {
            blue.reference.org
        }
    };
    let mut sorted: Vec<usize> = (0..blues.len()).collect();
    sorted.sort_by_key(|&index| bottom(&blues[index]));
    for pair in sorted.windows(2) {
        let next_top = top(&blues[pair[1]]);
        let blue = &mut blues[pair[0]];
        if top(blue) > next_top {
            if top_is_overshoot(blue) {
                blue.overshoot.org = next_top;
            } else {
                blue.reference.org = next_top;
            }
        }
    }
}

/// The contour holding a zone's extremum.
struct ContourWalk<'a> {
    outline: &'a FontOutline,
    first: usize,
    last: usize,
}

/// The run of points around an extremum.
struct FlatSegment {
    first: usize,
    last: usize,
    first_on: Option<usize>,
    last_on: Option<usize>,
    /// The position of the zone.
    y: i64,
    /// Where the walk after the extremum stopped (which FreeType's long
    /// zone search reads by mistake).
    walked_to: usize,
}

impl ContourWalk<'_> {
    fn prev(&self, index: usize) -> usize {
        if index > self.first {
            index - 1
        } else {
            self.last
        }
    }

    fn next(&self, index: usize) -> usize {
        if index < self.last {
            index + 1
        } else {
            self.first
        }
    }

    fn x(&self, index: usize) -> i64 {
        self.outline.points[index].0
    }

    fn y(&self, index: usize) -> i64 {
        self.outline.points[index].1
    }

    fn on(&self, index: usize) -> Option<usize> {
        self.outline.is_on(index).then_some(index)
    }

    /// Whether the vector from `from` to `to` leaves the horizontal: more
    /// than 5 units tall and steeper than about 2.9 degrees.
    fn leaves(&self, from: usize, to: usize) -> bool {
        let dist = (self.y(to) - self.y(from)).abs();
        dist > 5 && (self.x(to) - self.x(from)).abs() <= 20 * dist
    }

    /// The points before and after `best` that stay near its height.
    fn flat_segment(&self, best: usize, best_y: i64) -> FlatSegment {
        let mut segment = FlatSegment {
            first: best,
            last: best,
            first_on: self.on(best),
            last_on: self.on(best),
            y: best_y,
            walked_to: best,
        };
        let mut prev = best;
        loop {
            prev = self.prev(prev);
            if self.leaves(best, prev) {
                break;
            }
            segment.first = prev;
            if let Some(on) = self.on(prev) {
                segment.first_on = Some(on);
                segment.last_on = segment.last_on.or(Some(on));
            }
            if prev == best {
                break;
            }
        }
        let mut next = best;
        loop {
            next = self.next(next);
            if self.leaves(best, next) {
                break;
            }
            segment.last = next;
            if let Some(on) = self.on(next) {
                segment.last_on = Some(on);
                segment.first_on = segment.first_on.or(Some(on));
            }
            if next == best {
                break;
            }
        }
        segment.walked_to = next;
        segment
    }

    /// For long zones: when the flat segment is short, replaces it with
    /// the next long enough segment running the same way, not too far
    /// below the extremum. `None` for a contour whose points share the
    /// extremum's x (the glyph is skipped).
    fn lengthen(&self, segment: &mut FlatSegment, best: usize, units_per_em: u16) -> Option<()> {
        let length_threshold = i64::from(units_per_em) / 25;
        let mut dist = (self.x(segment.last) - self.x(segment.first)).abs();
        let span = segment.last as i64 - segment.first as i64 + 2;
        if dist >= length_threshold || span > self.last as i64 - self.first as i64 {
            return Some(());
        }
        let height_threshold = i64::from(units_per_em) / 4;
        let best_x = self.x(best);
        let mut prev = best;
        loop {
            prev = self.prev(prev);
            if self.x(prev) != best_x || prev == best {
                break;
            }
        }
        if prev == best {
            return None;
        }
        let left_to_right = self.x(prev) < best_x;
        let (mut first, mut last) = (segment.last, segment.last);
        let (mut first_on, mut last_on) = (None, None);
        let mut hit = false;
        loop {
            if !hit {
                first = last;
                (first_on, last_on) = (self.on(first), self.on(first));
                hit = true;
            }
            last = self.next(last);
            if (segment.y - self.y(first)).abs() > height_threshold {
                hit = false;
            } else {
                dist = (self.y(last) - self.y(first)).abs();
                if dist > 5 && (self.x(last) - self.x(first)).abs() <= 20 * dist {
                    hit = false;
                } else {
                    if let Some(on) = self.on(last) {
                        last_on = Some(on);
                        first_on = first_on.or(Some(on));
                    }
                    let l2r = self.x(first) < self.x(last);
                    if l2r == left_to_right
                        && (self.x(last) - self.x(first)).abs() >= length_threshold
                    {
                        last = self.extend(segment, first, last, dist, &mut first_on, &mut last_on);
                        *segment = FlatSegment {
                            first,
                            last,
                            first_on,
                            last_on,
                            y: self.y(first),
                            walked_to: segment.walked_to,
                        };
                        return Some(());
                    }
                }
            }
            if last == segment.first {
                return Some(());
            }
        }
    }

    /// Extends a long segment from `first` past `last` while it stays
    /// flat; returns its end. FreeType tests the angle with the point the
    /// flat walk stopped at and the last vertical distance found.
    fn extend(
        &self,
        segment: &FlatSegment,
        first: usize,
        mut last: usize,
        dist: i64,
        first_on: &mut Option<usize>,
        last_on: &mut Option<usize>,
    ) -> usize {
        loop {
            last = self.next(last);
            let d = (self.y(last) - self.y(first)).abs();
            if d > 5 && (self.x(segment.walked_to) - self.x(first)).abs() <= 20 * dist {
                return self.prev(last);
            }
            // FreeType records the end as an on point whatever its kind.
            *last_on = Some(last);
            *first_on = first_on.or_else(|| self.on(last));
            if last == segment.first {
                return last;
            }
        }
    }
}
