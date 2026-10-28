//! Decomposition of contours into the profiles' crossings
//! (`ftraster.c`'s `Decompose_Curve`, `Line_To`, `Conic_To`, `Cubic_To`,
//! `Line_Up`, `Bezier_Up` and the Bezier splitters).

use plt_font::{ScanPoint, ScanPointKind};

use super::{Converter, Direction, MAX_BEZIER, Vec2};
use crate::raster::mono::RasterError;

impl Converter {
    /// `Line_Up`: the crossings of an ascending segment within
    /// `min_y..=max_y`.
    fn line_up(&mut self, from: Vec2, to: Vec2, min_y: i64, max_y: i64) -> Result<(), RasterError> {
        let p = self.precision;
        let (Vec2 { x: mut x1, y: y1 }, Vec2 { x: x2, y: y2 }) = (from, to);
        if y2 < min_y || y1 > max_y {
            return Ok(());
        }
        let e2 = if y2 > max_y { max_y } else { p.floor(y2) };
        let mut e = if y1 < min_y { min_y } else { p.ceiling(y1) };
        if y1 == e {
            e += p.one();
        }
        if e2 < e {
            return Ok(());
        }
        let size = usize::try_from(p.trunc(e2 - e) + 1).unwrap_or(usize::MAX);
        if self.full(size) {
            return Err(RasterError::Overflow);
        }
        let mut dx = x2 - x1;
        let dy = y2 - y1;
        if dx == 0 {
            self.pool.extend(std::iter::repeat_n(x1, size));
            return Ok(());
        }
        let mut ix = (e - y1) * dx / dy;
        x1 += ix;
        self.pool.push(x1);
        if size > 1 {
            let mut ax = dx * (e - y1) - dy * ix;
            ix = p.one() * dx / dy;
            let mut rx = dx * p.one() - dy * ix;
            dx = 1;
            if x2 < x1 {
                ax = -ax;
                rx = -rx;
                dx = -dx;
            }
            for _ in 1..size {
                x1 += ix;
                ax += rx;
                if ax >= dy {
                    ax -= dy;
                    x1 += dx;
                }
                self.pool.push(x1);
            }
        }
        Ok(())
    }

    /// `Line_Down`: [`line_up`](Self::line_up) mirrored vertically.
    fn line_down(&mut self, from: Vec2, to: Vec2) -> Result<(), RasterError> {
        let flip = |v: Vec2| Vec2 { x: v.x, y: -v.y };
        self.line_up(flip(from), flip(to), -self.max_y, -self.min_y)
    }

    /// `Bezier_Up`: the crossings of the ascending arc of `degree` at
    /// `arc` of the Bezier stack (end first), split until flat.
    fn bezier_up(
        &mut self,
        degree: usize,
        arcs: &mut Vec<Vec2>,
        mut arc: usize,
        min_y: i64,
        max_y: i64,
    ) -> Result<(), RasterError> {
        let p = self.precision;
        let y1 = arcs[arc + degree].y;
        let y2 = arcs[arc].y;
        if y2 < min_y || y1 > max_y {
            return Ok(());
        }
        let e2 = if y2 > max_y { max_y } else { p.floor(y2) };
        let mut e = if y1 < min_y { min_y } else { p.ceiling(y1) };
        if y1 == e {
            e += p.one();
        }
        if e2 < e {
            return Ok(());
        }
        let needed = usize::try_from(p.trunc(e2 - e) + 1).unwrap_or(usize::MAX);
        if self.full(needed) {
            return Err(RasterError::Overflow);
        }
        loop {
            let Vec2 { x: x2, y: y2 } = arcs[arc];
            if y2 > e {
                let dy = y2 - arcs[arc + degree].y;
                let dx = x2 - arcs[arc + degree].x;
                if dy > p.step() || dx > p.step() || -dx > p.step() {
                    split(degree, arcs, arc);
                    arc += degree;
                    continue;
                }
                self.pool.push(x2 - (y2 - e) * dx / dy);
                e += p.one();
            } else if y2 == e {
                self.pool.push(x2);
                e += p.one();
            }
            match arc.checked_sub(degree) {
                Some(below) if e <= e2 => arc = below,
                _ => return Ok(()),
            }
        }
    }

    /// `Bezier_Down`: [`bezier_up`](Self::bezier_up) mirrored vertically.
    fn bezier_down(
        &mut self,
        degree: usize,
        arcs: &mut Vec<Vec2>,
        arc: usize,
    ) -> Result<(), RasterError> {
        for point in &mut arcs[arc..=arc + degree] {
            point.y = -point.y;
        }
        let result = self.bezier_up(degree, arcs, arc, -self.max_y, -self.min_y);
        arcs[arc].y = -arcs[arc].y;
        result
    }

    /// `Line_To`.
    fn line_to(&mut self, to: Vec2) -> Result<(), RasterError> {
        if to.y != self.last.y {
            let ascending = self.last.y < to.y;
            self.turn(if ascending {
                Direction::Ascending
            } else {
                Direction::Descending
            })?;
            if ascending {
                self.line_up(self.last, to, self.min_y, self.max_y)?;
            } else {
                self.line_down(self.last, to)?;
            }
        }
        self.last = to;
        Ok(())
    }

    /// `Conic_To` and `Cubic_To`: an arc of `degree` from the last point
    /// through `controls` to `to`, split until each piece is y-monotonic.
    fn curve_to(&mut self, controls: &[Vec2], to: Vec2) -> Result<(), RasterError> {
        let degree = controls.len() + 1;
        let mut arcs = vec![Vec2::default(); degree * MAX_BEZIER + 1];
        arcs[0] = to;
        for (slot, &control) in arcs[1..degree].iter_mut().zip(controls.iter().rev()) {
            *slot = control;
        }
        arcs[degree] = self.last;
        let p = self.precision;
        let mut arc = 0;
        loop {
            let ends = (arcs[arc + degree].y, arcs[arc].y);
            let (end_min, end_max) = (ends.0.min(ends.1), ends.0.max(ends.1));
            let inner = &arcs[arc + 1..arc + degree];
            let inner_min = inner.iter().map(|v| v.y).min().unwrap_or(end_min);
            let inner_max = inner.iter().map(|v| v.y).max().unwrap_or(end_max);
            if inner_min < p.floor(end_min) || inner_max > p.ceiling(end_max) {
                split(degree, &mut arcs, arc);
                arc += degree;
                continue;
            }
            if ends.0 != ends.1 {
                let ascending = ends.0 < ends.1;
                self.turn(if ascending {
                    Direction::Ascending
                } else {
                    Direction::Descending
                })?;
                if ascending {
                    self.bezier_up(degree, &mut arcs, arc, self.min_y, self.max_y)?;
                } else {
                    self.bezier_down(degree, &mut arcs, arc)?;
                }
            }
            self.last = arcs[arc];
            if arc == 0 {
                return Ok(());
            }
            arc -= degree;
        }
    }

    /// `Decompose_Curve`: one contour, closed back to its start.
    pub(super) fn decompose(
        &mut self,
        contour: &[ScanPoint],
        flipped: bool,
    ) -> Result<(), RasterError> {
        let p = self.precision;
        let scaled = |point: &ScanPoint| {
            let (x, y) = (p.scaled(point.x), p.scaled(point.y));
            if flipped {
                Vec2 { x: y, y: x }
            } else {
                Vec2 { x, y }
            }
        };
        let (Some(first), Some(last)) = (contour.first(), contour.last()) else {
            return Ok(());
        };
        let mut limit = contour.len() - 1;
        let mut start = scaled(first);
        // The index of the next point to read.
        let mut next = 1;
        match first.kind {
            ScanPointKind::Cubic => return Err(RasterError::InvalidOutline),
            ScanPointKind::Conic => {
                if last.kind == ScanPointKind::On {
                    start = scaled(last);
                    limit -= 1;
                } else {
                    let end = scaled(last);
                    start = Vec2 {
                        x: i64::midpoint(start.x, end.x),
                        y: i64::midpoint(start.y, end.y),
                    };
                }
                next = 0;
            }
            ScanPointKind::On => {}
        }
        self.last = start;
        while next <= limit {
            let point = &contour[next];
            next += 1;
            match point.kind {
                ScanPointKind::On => self.line_to(scaled(point))?,
                ScanPointKind::Conic => {
                    let mut control = scaled(point);
                    loop {
                        if next > limit {
                            return self.curve_to(&[control], start);
                        }
                        let point = &contour[next];
                        next += 1;
                        let to = scaled(point);
                        match point.kind {
                            ScanPointKind::On => {
                                self.curve_to(&[control], to)?;
                                break;
                            }
                            ScanPointKind::Conic => {
                                let middle = Vec2 {
                                    x: i64::midpoint(control.x, to.x),
                                    y: i64::midpoint(control.y, to.y),
                                };
                                self.curve_to(&[control], middle)?;
                                control = to;
                            }
                            ScanPointKind::Cubic => return Err(RasterError::InvalidOutline),
                        }
                    }
                }
                ScanPointKind::Cubic => {
                    if next > limit || contour[next].kind != ScanPointKind::Cubic {
                        return Err(RasterError::InvalidOutline);
                    }
                    let controls = [scaled(point), scaled(&contour[next])];
                    next += 1;
                    if next > limit {
                        return self.curve_to(&controls, start);
                    }
                    self.curve_to(&controls, scaled(&contour[next]))?;
                    next += 1;
                }
            }
        }
        self.line_to(start)
    }
}

/// `Split_Conic` or `Split_Cubic` of the arc at `arc` (end first) into the
/// arcs at `arc` (the end half) and `arc + degree` (the start half).
fn split(degree: usize, arcs: &mut Vec<Vec2>, arc: usize) {
    if arcs.len() < arc + 2 * degree + 1 {
        arcs.resize(arc + 2 * degree + 1, Vec2::default());
    }
    let base = &mut arcs[arc..=arc + 2 * degree];
    let (mut xs, mut ys) = ([0; 7], [0; 7]);
    for ((point, x), y) in base.iter().zip(&mut xs).zip(&mut ys) {
        (*x, *y) = (point.x, point.y);
    }
    let halve = if degree == 2 {
        split_conic
    } else {
        split_cubic
    };
    halve(&mut xs);
    halve(&mut ys);
    for ((point, x), y) in base.iter_mut().zip(xs).zip(ys) {
        *point = Vec2 { x, y };
    }
}

/// `Split_Conic` on one coordinate.
fn split_conic(base: &mut [i64]) {
    base[4] = base[2];
    let a = base[0] + base[1];
    let b = base[1] + base[2];
    base[3] = b >> 1;
    base[2] = (a + b) >> 2;
    base[1] = a >> 1;
}

/// `Split_Cubic` on one coordinate (the smooth rasterizer's
/// `gray_split_cubic` computes the same halves).
pub(in crate::raster) fn split_cubic(base: &mut [i64]) {
    base[6] = base[3];
    let mut a = base[0] + base[1];
    let b = base[1] + base[2];
    let mut c = base[2] + base[3];
    base[5] = c >> 1;
    c += b;
    base[4] = c >> 2;
    base[1] = a >> 1;
    a += b;
    base[2] = a >> 2;
    base[3] = (a + c) >> 3;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_halve_arcs() {
        let mut conic = [0, 64, 128, 0, 0];
        split_conic(&mut conic);
        assert_eq!(conic, [0, 32, 64, 96, 128]);
        let mut cubic = [0, 0, 64, 64, 0, 0, 0];
        split_cubic(&mut cubic);
        assert_eq!(cubic, [0, 0, 16, 32, 48, 64, 64]);
    }
}
