//! Diacritic helpers of the latin hinter (the `afadjust` parts of
//! FreeType's `aflatin.c`): contour extrema, the contour of a diacritic
//! above or below its base, tildes stretched so that they stay legible,
//! and contours moved or protected from hinting.

use crate::autofit::hints::{GlyphHints, point_flag};
use crate::fixed::pix_round;

/// Where a diacritic sits relative to its base character.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Top,
    Bottom,
}

impl GlyphHints {
    /// Moves every point of the contour starting at `first` by `delta`
    /// vertically.
    pub(crate) fn move_contour(&mut self, first: usize, delta: i64) {
        let points: Vec<usize> = self.contour_from(first).collect();
        for point in points {
            self.points[point].y += delta;
        }
    }

    /// Moves by `delta` (up for the top side, down for the bottom side)
    /// every non-flat contour beyond `limit` on that side
    /// (`af_move_contours_up`, `af_move_contours_down`).
    pub(crate) fn move_contours(&mut self, side: Side, limit: i64, delta: i64) {
        for contour in 0..self.contours.len() {
            let (min_y, max_y) = (
                self.contour_y_minima[contour],
                self.contour_y_maxima[contour],
            );
            let beyond = match side {
                Side::Top => min_y > limit,
                Side::Bottom => max_y < limit,
            };
            if min_y < max_y && beyond {
                let delta = if side == Side::Top { delta } else { -delta };
                self.move_contour(self.contours[contour], delta);
            }
        }
    }

    /// The vertical extent of every contour; contours of one or two
    /// points get an empty one (`af_compute_vertical_extrema`).
    pub(crate) fn compute_vertical_extrema(&mut self) {
        for contour in 0..self.contours.len() {
            let first = self.contours[contour];
            let (mut min_y, mut max_y) = (i64::MAX, i64::MIN);
            if self.points[self.points[first].next].next != first {
                for point in self.contour_from(first) {
                    min_y = min_y.min(self.points[point].y);
                    max_y = max_y.max(self.points[point].y);
                }
            }
            self.contour_y_minima[contour] = min_y;
            self.contour_y_maxima[contour] = max_y;
        }
    }

    /// The contour reaching furthest to `side` (the shorter one among
    /// ties), or the first one (`af_find_highest_contour`,
    /// `af_find_lowest_contour`).
    pub(crate) fn extreme_contour(&self, side: Side) -> usize {
        let mut best = 0;
        let (mut best_min, mut best_max) = (i64::MAX, i64::MIN);
        for contour in 0..self.contours.len() {
            let (min_y, max_y) = (
                self.contour_y_minima[contour],
                self.contour_y_maxima[contour],
            );
            let better = match side {
                Side::Top => max_y > best_max || (max_y == best_max && min_y > best_min),
                Side::Bottom => min_y < best_min || (min_y == best_min && max_y < best_max),
            };
            if better {
                (best, best_min, best_max) = (contour, min_y, max_y);
            }
        }
        best
    }

    /// The contour next to the extreme one towards `side`: the one
    /// reaching furthest to `side` among those extending beyond the
    /// extreme contour on the other side; the first contour for glyphs of
    /// fewer than three contours (`af_find_second_highest_contour`,
    /// `af_find_second_lowest_contour`).
    pub(crate) fn second_extreme_contour(&self, side: Side) -> usize {
        if self.contours.len() < 3 {
            return 0;
        }
        let extreme = self.extreme_contour(side);
        let mut best = 0;
        match side {
            Side::Top => {
                let limit = self.contour_y_minima[extreme];
                let mut best_max = i64::MIN;
                for contour in (0..self.contours.len()).filter(|&c| c != extreme) {
                    let max_y = self.contour_y_maxima[contour];
                    if max_y > best_max && self.contour_y_minima[contour] < limit {
                        (best, best_max) = (contour, max_y);
                    }
                }
            }
            Side::Bottom => {
                let limit = self.contour_y_maxima[extreme];
                let mut best_min = i64::MAX;
                for contour in (0..self.contours.len()).filter(|&c| c != extreme) {
                    let min_y = self.contour_y_minima[contour];
                    if min_y < best_min && self.contour_y_maxima[contour] > limit {
                        (best, best_min) = (contour, min_y);
                    }
                }
            }
        }
        best
    }

    /// Protects every point of `contour` from hinting: ignored by the
    /// segment analysis, and on points touched vertically
    /// (`af_touch_contour`).
    fn touch_contour(&mut self, contour: usize) {
        let points: Vec<usize> = self.contour_from(self.contours[contour]).collect();
        for point in points {
            let point = &mut self.points[point];
            point.flags |= point_flag::IGNORE;
            if point.flags & point_flag::CONTROL == 0 {
                point.flags |= point_flag::TOUCH_Y;
            }
        }
    }

    /// Protects the contours from `limit_contour`'s bottom on towards
    /// `side` (FreeType compares with the limit contour's bottom for both
    /// sides; `af_touch_top_contours`, `af_touch_bottom_contours`).
    fn touch_contours(&mut self, side: Side, limit_contour: usize) {
        let limit = self.contour_y_minima[limit_contour];
        for contour in 0..self.contours.len() {
            let (min_y, max_y) = (
                self.contour_y_minima[contour],
                self.contour_y_maxima[contour],
            );
            let beyond = match side {
                Side::Top => min_y >= limit,
                Side::Bottom => max_y <= limit,
            };
            if min_y < max_y && beyond {
                self.touch_contour(contour);
            }
        }
    }

    /// Stretches the tilde `contour` vertically so that its wiggles stay
    /// at least a pixel apart, protecting it (and the contours beyond it)
    /// from hinting; returns how much taller it got
    /// (`af_latin_stretch_top_tilde`, `af_latin_stretch_bottom_tilde`).
    pub(crate) fn stretch_tilde(&mut self, side: Side, contour: usize) -> i64 {
        let first = self.contours[contour];
        let (min_y, max_y) = (
            self.contour_y_minima[contour],
            self.contour_y_maxima[contour],
        );
        if min_y == max_y {
            return 0;
        }
        let height = max_y.wrapping_sub(min_y);
        let measurement = self.tilde_wiggle(first, min_y, max_y, height / 8);
        self.touch_contours(side, contour);
        let target = measurement.unwrap_or(0) + 64;
        if height >= target {
            return 0;
        }
        // The height stays small: no risk of overflow.
        let anchor = match side {
            Side::Top => min_y,
            Side::Bottom => max_y,
        };
        let points: Vec<usize> = self.contour_from(first).collect();
        for point in points {
            let y = &mut self.points[point].y;
            *y = (*y - anchor) * target / height + anchor;
        }
        target - height
    }

    /// The smallest distance between a round wiggle of the tilde starting
    /// at `first` (an on point between two control points, at neither
    /// extremum) and the extremum on the other side of it; wiggles nearer
    /// than `threshold` to an extremum are ignored.
    fn tilde_wiggle(&self, first: usize, min_y: i64, max_y: i64, threshold: i64) -> Option<i64> {
        let points = &self.points;
        let control = |index: usize| points[index].flags & point_flag::CONTROL != 0;
        let mut smallest: Option<i64> = None;
        for p in self.contour_from(first) {
            let point = points[p];
            let (prev, next) = (point.prev, point.next);
            let candidate = !control(p)
                && points[prev].y == point.y
                && points[next].y == point.y
                && point.y != min_y
                && point.y != max_y
                && control(prev)
                && control(next);
            if !candidate {
                continue;
            }
            let (mut prev_on, mut next_on) = (prev, next);
            while control(prev_on) {
                prev_on = points[prev_on].prev;
            }
            while control(next_on) {
                next_on = points[next_on].next;
            }
            let (prev_y, next_y) = (points[prev_on].y, points[next_on].y);
            let measurement = if next_y > point.y && prev_y > point.y {
                point.y - min_y
            } else if next_y < point.y && prev_y < point.y {
                max_y - point.y
            } else {
                continue;
            };
            if measurement >= threshold && smallest.is_none_or(|s| measurement < s) {
                smallest = Some(measurement);
            }
        }
        smallest
    }

    /// Moves the (stretched) tilde `contour` onto the grid: its bottom
    /// (top for a bottom tilde), or its centre when it is less than three
    /// pixels tall; returns the distance moved (`af_latin_align_top_tilde`,
    /// `af_latin_align_bottom_tilde`).
    pub(crate) fn align_tilde(&mut self, side: Side, contour: usize) -> i64 {
        let first = self.contours[contour];
        let (mut min_y, mut max_y) = (self.points[first].y, self.points[first].y);
        for point in self.contour_from(first) {
            min_y = min_y.min(self.points[point].y);
            max_y = max_y.max(self.points[point].y);
        }
        let height = max_y.wrapping_sub(min_y);
        let centering = if height < 64 * 3 {
            (pix_round(height) - height) / 2
        } else {
            0
        };
        let delta = match side {
            Side::Top => pix_round(min_y).wrapping_sub(min_y) + centering,
            Side::Bottom => pix_round(max_y).wrapping_sub(max_y) - centering,
        };
        self.move_contour(first, delta);
        delta
    }

    /// Whether `contour` overlaps horizontally the bounding box of the
    /// other contours of more than two points
    /// (`af_check_contour_horizontal_overlap`).
    pub(crate) fn overlaps_horizontally(&self, contour: usize) -> bool {
        let (mut own_min, mut own_max) = (i64::MAX, i64::MIN);
        let (mut others_min, mut others_max) = (i64::MAX, i64::MIN);
        for (index, &first) in self.contours.iter().enumerate() {
            if self.points[self.points[first].next].next == first {
                continue;
            }
            for point in self.contour_from(first) {
                let x = self.points[point].x;
                if index == contour {
                    (own_min, own_max) = (own_min.min(x), own_max.max(x));
                } else {
                    (others_min, others_max) = (others_min.min(x), others_max.max(x));
                }
            }
        }
        (others_min <= own_max && own_max <= others_max)
            || (others_min <= own_min && own_min <= others_max)
            || (own_max >= others_max && own_min <= others_min)
    }
}
