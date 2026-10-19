//! Profiles (`ftraster.c`'s `Convert_Glyph`): every y-monotonic run of a
//! contour with its crossings of the scanlines, which pass through pixel
//! centres. Like `FreeType`, the crossings live in a render pool of fixed
//! size; a band that does not fit is reported as [`RasterError::Overflow`]
//! so that it can be split.

use plt_font::{ScanPoint, ScanPointKind};

use super::RasterError;

/// Longs in `FreeType`'s render pool (`FT_MAX_BLACK_POOL`).
const POOL_LONGS: usize = 2048;
/// Longs a profile header takes before its crossings
/// (`offsetof(TProfile, x)`).
const HEADER_LONGS: usize = 5;
/// The Bezier stack of `Conic_To` and `Cubic_To` holds this many arcs.
const MAX_BEZIER: usize = 32;

/// A profile ascends (`Flow_Up`).
pub(super) const FLOW_UP: u8 = 0x08;
/// A profile's top end overshoots its last scanline by half a pixel.
pub(super) const OVERSHOOT_TOP: u8 = 0x10;
/// A profile's bottom end overshoots its first scanline by half a pixel.
pub(super) const OVERSHOOT_BOTTOM: u8 = 0x20;
/// A drop-out was found at the current scanline.
pub(super) const DROPOUT: u8 = 0x40;

/// The fixed-point precision of the rasterizer (`Set_High_Precision`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Precision {
    bits: u32,
    /// Bezier arcs taller or wider than this are split before being
    /// approximated by their chord.
    step: i64,
}

impl Precision {
    /// 1/4096 pixel when `high`, else 1/64.
    pub(super) const fn new(high: bool) -> Self {
        if high {
            Self {
                bits: 12,
                step: 256,
            }
        } else {
            Self { bits: 6, step: 32 }
        }
    }

    /// One pixel.
    pub(super) const fn one(self) -> i64 {
        1 << self.bits
    }

    /// Half a pixel.
    pub(super) const fn half(self) -> i64 {
        self.one() >> 1
    }

    /// `FLOOR`: the scanline at or below `x`.
    pub(super) const fn floor(self, x: i64) -> i64 {
        x & -self.one()
    }

    /// `CEILING`: the scanline at or above `x`.
    pub(super) const fn ceiling(self, x: i64) -> i64 {
        (x + self.one() - 1) & -self.one()
    }

    /// `TRUNC`: the pixel index of a scanline.
    pub(super) const fn trunc(self, x: i64) -> i64 {
        x >> self.bits
    }

    /// `FRAC`: the distance above the scanline below.
    const fn frac(self, x: i64) -> i64 {
        x & (self.one() - 1)
    }

    /// `SCALED`: a 26.6 coordinate in this precision, shifted so that
    /// pixel centres fall on whole units.
    const fn scaled(self, x: i64) -> i64 {
        x * (self.one() >> 6) - self.half()
    }

    /// `IS_BOTTOM_OVERSHOOT`.
    const fn is_bottom_overshoot(self, x: i64) -> bool {
        self.ceiling(x) - x >= self.half()
    }

    /// `IS_TOP_OVERSHOOT`.
    const fn is_top_overshoot(self, x: i64) -> bool {
        x - self.floor(x) >= self.half()
    }

    /// `SMART`: the scanline nearest the middle of `p` and `q`, symmetric
    /// cases rounding the same at either precision.
    pub(super) const fn smart(self, p: i64, q: i64) -> i64 {
        self.floor((p + q + self.one() * 63 / 64) >> 1)
    }

    /// The step under which Bezier arcs are approximated by their chord.
    const fn step(self) -> i64 {
        self.step
    }
}

/// A y-monotonic run of a contour (`TProfile`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Profile {
    /// Drop-out mode (bits 0-2) and the flags above.
    pub(super) flags: u8,
    /// The first scanline, while built the scanline it starts at.
    pub(super) start: i64,
    /// The scanlines left to sweep, zero until recorded.
    pub(super) height: i64,
    /// The index of the current crossing.
    pub(super) offset: i64,
    /// Where its crossings start in the pool.
    first_crossing: usize,
    /// The current crossing (`X`).
    pub(super) x: i64,
    /// The next profile of the same contour (cyclic), by index.
    pub(super) next: usize,
    /// The first recorded profile of its contour, by slot.
    contour_first: usize,
}

/// The profiles of an outline in one band, ready for the sweep.
#[derive(Debug)]
pub(super) struct Profiles {
    /// The recorded profiles in the order they were made.
    pub(super) list: Vec<Profile>,
    /// The render pool holding the crossings.
    pool: Vec<i64>,
    /// Every scanline where profiles start, then the top of the highest.
    pub(super) turns: Vec<i64>,
}

impl Profiles {
    /// The crossing of `profile` at `offset`.
    pub(super) fn crossing(&self, profile: &Profile, offset: i64) -> i64 {
        usize::try_from(offset)
            .ok()
            .and_then(|offset| self.pool.get(profile.first_crossing + offset))
            .copied()
            .unwrap_or(profile.x)
    }
}

/// A point in the rasterizer's coordinates (`TPoint`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Vec2 {
    x: i64,
    y: i64,
}

/// The direction of the profile being built (`TStates`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Unknown,
    Ascending,
    Descending,
}

/// Builds the profiles of an outline (the conversion half of
/// `black_TWorker`).
pub(super) struct Converter {
    precision: Precision,
    dropout_control: u8,
    /// The band, in this precision.
    min_y: i64,
    max_y: i64,
    /// The used bottom of the pool: profile headers and crossings.
    pool: Vec<i64>,
    /// Where the scanline turns start at the top of the pool (`maxBuff`).
    pool_max: usize,
    /// Every profile allocated, recorded or not.
    slots: Vec<Profile>,
    /// The recorded profiles, by slot.
    recorded: Vec<usize>,
    /// The profile being built (`cProfile`).
    current: Option<usize>,
    turns: Vec<i64>,
    state: Direction,
    last: Vec2,
    /// The first recorded profile of the contour (`gProfile`).
    contour_first: Option<usize>,
}

impl Converter {
    /// A converter with `precision` and the drop-out control bits.
    pub(super) fn new(precision: Precision, dropout_control: u8) -> Self {
        Self {
            precision,
            dropout_control,
            min_y: 0,
            max_y: 0,
            pool: Vec::new(),
            pool_max: POOL_LONGS - 1,
            slots: Vec::new(),
            recorded: Vec::new(),
            current: None,
            turns: Vec::new(),
            state: Direction::Unknown,
            last: Vec2::default(),
            contour_first: None,
        }
    }

    /// The precision the profiles are computed in.
    pub(super) const fn precision(&self) -> Precision {
        self.precision
    }

    /// `Convert_Glyph`: the profiles of the contours of `points` (ending
    /// at `contour_ends`) between scanlines `band.0` and `band.1`, with x
    /// and y swapped when `flipped` (for the horizontal sweep).
    pub(super) fn convert(
        &mut self,
        points: &[ScanPoint],
        contour_ends: &[usize],
        band: (i64, i64),
        flipped: bool,
    ) -> Result<Profiles, RasterError> {
        self.min_y = band.0 * self.precision.one();
        self.max_y = band.1 * self.precision.one();
        self.pool.clear();
        self.pool_max = POOL_LONGS - 1;
        self.slots.clear();
        self.recorded.clear();
        self.current = None;
        self.turns.clear();
        let mut start = 0;
        for &end in contour_ends {
            let contour = points.get(start..end).ok_or(RasterError::InvalidOutline)?;
            start = end;
            self.state = Direction::Unknown;
            self.contour_first = None;
            self.decompose(contour, flipped)?;
            let Some(first) = self.contour_first else {
                continue;
            };
            let current = self.current.ok_or(RasterError::InvalidOutline)?;
            // The contour's first and last profiles meet on a scanline
            // both crossed: drop the second crossing.
            if self.precision.frac(self.last.y) == 0
                && (self.min_y..=self.max_y).contains(&self.last.y)
                && self.slots[first].flags & FLOW_UP == self.slots[current].flags & FLOW_UP
            {
                self.pool.pop();
            }
            self.end_profile()?;
        }
        Ok(self.finish())
    }

    /// `Finalize_Profile_Table`: links each recorded profile to the next
    /// of its contour, the last back to the first.
    fn finish(&mut self) -> Profiles {
        let slot_index = |slot: usize| self.recorded.iter().position(|&s| s == slot);
        let mut list: Vec<Profile> = self.recorded.iter().map(|&s| self.slots[s]).collect();
        for (index, profile) in list.iter_mut().enumerate() {
            let next_slot = self
                .recorded
                .get(index + 1)
                .copied()
                .filter(|&next| self.slots[next].contour_first == profile.contour_first)
                .unwrap_or(profile.contour_first);
            profile.next = slot_index(next_slot).unwrap_or(index);
        }
        Profiles {
            list,
            pool: std::mem::take(&mut self.pool),
            turns: std::mem::take(&mut self.turns),
        }
    }

    /// The pool is full (`ras.top >= ras.maxBuff` and the like).
    fn full(&self, extra: usize) -> bool {
        self.pool.len() + extra >= self.pool_max
    }

    /// `Insert_Y_Turns`: records that a profile spans `bottom..top`.
    fn insert_turns(&mut self, bottom: i64, top: i64) -> Result<(), RasterError> {
        match self.turns.last_mut() {
            None => self.turns.push(top),
            Some(highest) if top > *highest => *highest = top,
            Some(_) => {}
        }
        let below = &self.turns[..self.turns.len() - 1];
        let at = below.partition_point(|&turn| turn <= bottom);
        if at > 0 && below[at - 1] == bottom {
            return Ok(());
        }
        self.pool_max -= 1;
        if self.pool_max <= self.pool.len() {
            return Err(RasterError::Overflow);
        }
        self.turns.insert(at, bottom);
        Ok(())
    }

    /// `New_Profile`: starts a profile going `direction` from the last
    /// point, reusing the current one if it was never recorded.
    fn new_profile(&mut self, direction: Direction) -> Result<(), RasterError> {
        let current = match self.current {
            Some(slot) if self.slots[slot].height == 0 => slot,
            _ => {
                let first_crossing = self.pool.len() + HEADER_LONGS;
                self.pool.resize(first_crossing, 0);
                if self.full(0) {
                    return Err(RasterError::Overflow);
                }
                self.slots.push(Profile {
                    flags: 0,
                    start: 0,
                    height: 0,
                    offset: 0,
                    first_crossing,
                    x: 0,
                    next: 0,
                    contour_first: 0,
                });
                self.slots.len() - 1
            }
        };
        self.current = Some(current);
        let p = self.precision;
        let last = self.last;
        let mut flags = self.dropout_control;
        let start = match direction {
            Direction::Ascending => {
                flags |= FLOW_UP;
                if p.is_bottom_overshoot(last.y) {
                    flags |= OVERSHOOT_BOTTOM;
                }
                p.ceiling(last.y)
            }
            Direction::Descending => {
                if p.is_top_overshoot(last.y) {
                    flags |= OVERSHOOT_TOP;
                }
                p.floor(last.y)
            }
            Direction::Unknown => return Err(RasterError::InvalidOutline),
        };
        let start = start.min(self.max_y).max(self.min_y);
        let profile = &mut self.slots[current];
        profile.flags = flags;
        profile.start = p.trunc(start);
        if last.y == start {
            self.pool.push(last.x);
        }
        self.state = direction;
        Ok(())
    }

    /// `End_Profile`: records the current profile if it crossed any
    /// scanline.
    fn end_profile(&mut self) -> Result<(), RasterError> {
        let slot = self.current.ok_or(RasterError::InvalidOutline)?;
        let p = self.precision;
        let last_y = self.last.y;
        let first_crossing = self.slots[slot].first_crossing;
        let Some(height) = self.pool.len().checked_sub(first_crossing) else {
            return Err(RasterError::InvalidOutline);
        };
        if height == 0 {
            return Ok(());
        }
        let profile = &mut self.slots[slot];
        let h = height as i64;
        profile.height = h;
        let (bottom, top) = if profile.flags & FLOW_UP != 0 {
            if p.is_top_overshoot(last_y) {
                profile.flags |= OVERSHOOT_TOP;
            }
            profile.offset = 0;
            profile.x = self.pool[first_crossing];
            (profile.start, profile.start + h)
        } else {
            if p.is_bottom_overshoot(last_y) {
                profile.flags |= OVERSHOOT_BOTTOM;
            }
            let top = profile.start + 1;
            profile.start = top - h;
            profile.offset = h - 1;
            profile.x = self.pool[first_crossing + height - 1];
            (top - h, top)
        };
        self.insert_turns(bottom, top)?;
        let first = *self.contour_first.get_or_insert(slot);
        self.slots[slot].contour_first = first;
        self.recorded.push(slot);
        Ok(())
    }

    /// Starts a new profile when the outline turns to `direction`.
    fn turn(&mut self, direction: Direction) -> Result<(), RasterError> {
        if self.state != direction {
            if self.state != Direction::Unknown {
                self.end_profile()?;
            }
            self.new_profile(direction)?;
        }
        Ok(())
    }

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
    fn decompose(&mut self, contour: &[ScanPoint], flipped: bool) -> Result<(), RasterError> {
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

/// `Split_Cubic` on one coordinate.
fn split_cubic(base: &mut [i64]) {
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

    fn on(x: i64, y: i64) -> ScanPoint {
        ScanPoint {
            x,
            y,
            kind: ScanPointKind::On,
        }
    }

    #[test]
    fn precisions_put_pixel_centres_on_whole_units() {
        let low = Precision::new(false);
        assert_eq!(low.scaled(32), 0);
        assert_eq!(
            (low.floor(-1), low.ceiling(1), low.trunc(130)),
            (-64, 64, 2)
        );
        let high = Precision::new(true);
        assert_eq!(high.scaled(96), 4096);
        assert_eq!(high.smart(0, 4096), 0);
        assert_eq!(low.smart(-64, 0), -64);
    }

    #[test]
    fn a_square_makes_one_profile_up_and_one_down() {
        // A 2×2 pixel square from (1, 1) to (3, 3), counter-clockwise.
        let square = [on(64, 64), on(192, 64), on(192, 192), on(64, 192)];
        let mut converter = Converter::new(Precision::new(false), 1);
        let profiles = converter.convert(&square, &[4], (0, 3), false).unwrap();
        assert_eq!(profiles.list.len(), 2);
        // Scanlines through the centres of rows 1 and 2.
        assert_eq!(profiles.turns, vec![1, 3]);
        let up = &profiles.list[0];
        assert_eq!(up.flags & FLOW_UP, FLOW_UP);
        assert_eq!((up.start, up.height), (1, 2));
        assert_eq!(profiles.crossing(up, 0), 160);
        let down = &profiles.list[1];
        assert_eq!((down.start, down.height), (1, 2));
        assert_eq!(profiles.crossing(down, 0), 32);
        assert_eq!((up.next, down.next), (1, 0));
    }

    #[test]
    fn splits_halve_arcs() {
        let mut conic = [0, 64, 128, 0, 0];
        split_conic(&mut conic);
        assert_eq!(conic, [0, 32, 64, 96, 128]);
        let mut cubic = [0, 0, 64, 64, 0, 0, 0];
        split_cubic(&mut cubic);
        assert_eq!(cubic, [0, 0, 16, 32, 48, 64, 64]);
    }

    #[test]
    fn a_full_pool_overflows() {
        // A tall thin sliver crossing thousands of scanlines.
        let sliver = [on(0, 0), on(64, 0), on(64, 64 * 3000)];
        let mut converter = Converter::new(Precision::new(false), 1);
        assert_eq!(
            converter
                .convert(&sliver, &[3], (0, 3000), false)
                .unwrap_err(),
            RasterError::Overflow
        );
        assert!(converter.convert(&sliver, &[3], (0, 900), false).is_ok());
    }
}
