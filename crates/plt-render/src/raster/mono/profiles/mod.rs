//! Profiles (`ftraster.c`'s `Convert_Glyph`): every y-monotonic run of a
//! contour with its crossings of the scanlines, which pass through pixel
//! centres. Like `FreeType`, the crossings live in a render pool of fixed
//! size; a band that does not fit is reported as [`RasterError::Overflow`]
//! so that it can be split.

mod decompose;

pub(in crate::raster) use decompose::split_cubic;

use plt_font::ScanPoint;

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
}

#[cfg(test)]
mod tests {
    use plt_font::ScanPointKind;

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
