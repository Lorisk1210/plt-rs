//! VP8's loop filters (libwebp's `dsp/dec.c` and `DoFilter`; RFC 6386
//! section 15), smoothing the edges of macroblocks and of their 4×4
//! blocks after the whole frame is reconstructed.

use super::header::{FilterKind, FilterStrength};

/// The planes of a frame, `stride` bytes per row.
#[derive(Debug)]
pub(super) struct Plane {
    pub(super) data: Vec<u8>,
    pub(super) stride: usize,
}

/// `v` clamped to a signed byte.
fn sclip1(v: i32) -> i32 {
    v.clamp(-128, 127)
}

/// `v` clamped to −16–15.
fn sclip2(v: i32) -> i32 {
    v.clamp(-16, 15)
}

/// `v` clamped to a pixel.
fn clip1(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// The pixels across an edge: `p[at]` is the first pixel after it, and
/// `step` the distance between pixels across it.
struct Edge<'a> {
    pixels: &'a mut [u8],
    at: usize,
    step: usize,
}

impl Edge<'_> {
    /// The pixel `offset` steps from the edge (negative: before it).
    fn get(&self, offset: isize) -> i32 {
        i32::from(self.pixels[self.index(offset)])
    }

    fn set(&mut self, offset: isize, value: i32) {
        let index = self.index(offset);
        self.pixels[index] = clip1(value);
    }

    fn index(&self, offset: isize) -> usize {
        (self.at as isize + offset * self.step as isize) as usize
    }

    /// Whether the step across the edge is within `threshold`.
    fn needs_filter(&self, threshold: i32) -> bool {
        4 * (self.get(-1) - self.get(0)).abs() + (self.get(-2) - self.get(1)).abs() <= threshold
    }

    /// [`needs_filter`](Self::needs_filter), and every step on either
    /// side within `interior`.
    fn needs_filter2(&self, threshold: i32, interior: i32) -> bool {
        self.needs_filter(threshold)
            && [(-4, -3), (-3, -2), (-2, -1), (3, 2), (2, 1), (1, 0)]
                .iter()
                .all(|&(a, b)| (self.get(a) - self.get(b)).abs() <= interior)
    }

    /// Whether either side varies more than `threshold` next to the edge.
    fn high_variance(&self, threshold: i32) -> bool {
        (self.get(-2) - self.get(-1)).abs() > threshold
            || (self.get(1) - self.get(0)).abs() > threshold
    }

    /// Adjusts the two pixels next to the edge.
    fn filter2(&mut self) {
        let a = 3 * (self.get(0) - self.get(-1)) + sclip1(self.get(-2) - self.get(1));
        let a1 = sclip2((a + 4) >> 3);
        let a2 = sclip2((a + 3) >> 3);
        let (p0, q0) = (self.get(-1), self.get(0));
        self.set(-1, p0 + a2);
        self.set(0, q0 - a1);
    }

    /// Adjusts two pixels on each side (inner edges).
    fn filter4(&mut self) {
        let a = 3 * (self.get(0) - self.get(-1));
        let a1 = sclip2((a + 4) >> 3);
        let a2 = sclip2((a + 3) >> 3);
        let a3 = (a1 + 1) >> 1;
        let [p1, p0, q0, q1] = [-2, -1, 0, 1].map(|offset| self.get(offset));
        self.set(-2, p1 + a3);
        self.set(-1, p0 + a2);
        self.set(0, q0 - a1);
        self.set(1, q1 - a3);
    }

    /// Adjusts three pixels on each side (macroblock edges).
    fn filter6(&mut self) {
        let [p2, p1, p0, q0, q1, q2] = [-3, -2, -1, 0, 1, 2].map(|offset| self.get(offset));
        let a = sclip1(3 * (q0 - p0) + sclip1(p1 - q1));
        let a1 = (27 * a + 63) >> 7;
        let a2 = (18 * a + 63) >> 7;
        let a3 = (9 * a + 63) >> 7;
        self.set(-3, p2 + a3);
        self.set(-2, p1 + a2);
        self.set(-1, p0 + a1);
        self.set(0, q0 - a1);
        self.set(1, q1 - a2);
        self.set(2, q2 - a3);
    }
}

/// Which way an edge runs.
#[derive(Clone, Copy, Debug)]
enum Direction {
    /// A vertical edge, filtered along rows.
    Vertical,
    /// A horizontal edge, filtered along columns.
    Horizontal,
}

/// Filters `length` positions along the edge starting at `at`: with the
/// macroblock filter (`mb_edge`) or the inner one.
#[expect(
    clippy::too_many_arguments,
    reason = "libwebp's FilterLoop26/24 one to one"
)]
fn filter_edge(
    plane: &mut Plane,
    at: usize,
    direction: Direction,
    length: usize,
    strength: &FilterStrength,
    limit: i32,
    mb_edge: bool,
    simple: bool,
) {
    let stride = plane.stride;
    let (step, along) = match direction {
        Direction::Vertical => (1, stride),
        Direction::Horizontal => (stride, 1),
    };
    let threshold = 2 * limit + 1;
    for i in 0..length {
        let mut edge = Edge {
            pixels: &mut plane.data,
            at: at + i * along,
            step,
        };
        if simple {
            if edge.needs_filter(threshold) {
                edge.filter2();
            }
        } else if edge.needs_filter2(threshold, strength.interior) {
            if edge.high_variance(strength.hev_threshold) {
                edge.filter2();
            } else if mb_edge {
                edge.filter6();
            } else {
                edge.filter4();
            }
        }
    }
}

/// Filters the edges of one plane's block of `size` pixels at `origin`:
/// left and top macroblock edges (unless on the frame border) and, when
/// `strength.inner`, the inner edges every four pixels.
fn filter_block(
    plane: &mut Plane,
    origin: usize,
    size: usize,
    strength: &FilterStrength,
    (left, top): (bool, bool),
    simple: bool,
) {
    let stride = plane.stride;
    let limit = strength.limit;
    if left {
        filter_edge(
            plane,
            origin,
            Direction::Vertical,
            size,
            strength,
            limit + 4,
            true,
            simple,
        );
    }
    if strength.inner {
        for x in (4..size).step_by(4) {
            filter_edge(
                plane,
                origin + x,
                Direction::Vertical,
                size,
                strength,
                limit,
                false,
                simple,
            );
        }
    }
    if top {
        filter_edge(
            plane,
            origin,
            Direction::Horizontal,
            size,
            strength,
            limit + 4,
            true,
            simple,
        );
    }
    if strength.inner {
        for y in (4..size).step_by(4) {
            let at = origin + y * stride;
            filter_edge(
                plane,
                at,
                Direction::Horizontal,
                size,
                strength,
                limit,
                false,
                simple,
            );
        }
    }
}

/// Filters macroblock `(mb_x, mb_y)` of the frame planes.
pub(super) fn filter_macroblock(
    planes: &mut [Plane; 3],
    kind: FilterKind,
    strength: &FilterStrength,
    (mb_x, mb_y): (usize, usize),
) {
    if strength.limit == 0 || kind == FilterKind::None {
        return;
    }
    let simple = kind == FilterKind::Simple;
    let borders = (mb_x > 0, mb_y > 0);
    let [y, u, v] = planes;
    filter_block(
        y,
        mb_y * 16 * y.stride + mb_x * 16,
        16,
        strength,
        borders,
        simple,
    );
    if !simple {
        for chroma in [u, v] {
            let origin = mb_y * 8 * chroma.stride + mb_x * 8;
            filter_block(chroma, origin, 8, strength, borders, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_is_smoothed_and_a_cliff_left_alone() {
        let strength = FilterStrength {
            limit: 10,
            interior: 10,
            hev_threshold: 0,
            inner: false,
        };
        let row = |left: u8, right: u8| -> Vec<u8> {
            (0..32).map(|x| if x < 16 { left } else { right }).collect()
        };
        let mut planes = [
            Plane {
                data: row(100, 104).repeat(16),
                stride: 32,
            },
            Plane {
                data: vec![0; 16 * 8],
                stride: 16,
            },
            Plane {
                data: vec![0; 16 * 8],
                stride: 16,
            },
        ];
        filter_macroblock(&mut planes, FilterKind::Normal, &strength, (1, 0));
        let filtered = &planes[0].data[..32];
        // a = 3 · 4 − 4 = 8 moves both sides by (27 · 8 + 63) >> 7 = 2.
        assert_eq!((filtered[15], filtered[16]), (102, 102));
        assert_eq!(
            (filtered[12], filtered[13]),
            (100, 101),
            "three pixels deep"
        );
        assert_eq!((filtered[0], filtered[31]), (100, 104));
        planes[0].data = row(0, 200).repeat(16);
        filter_macroblock(&mut planes, FilterKind::Simple, &strength, (1, 0));
        assert_eq!(
            planes[0].data[..32],
            row(0, 200)[..],
            "too steep to be an artefact"
        );
    }
}
