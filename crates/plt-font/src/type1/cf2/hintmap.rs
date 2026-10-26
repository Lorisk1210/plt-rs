//! Hint maps (`pshints.c`): the horizontal stem edges of the active
//! hints, aligned to alignment zones or to the pixel grid, through which
//! vertical coordinates are mapped piecewise linearly to device space.
//!
//! Stem darkening is off, so edges are never moved for it.

use super::blues::{Blues, MIN_COUNTER};
use super::hints::{HintMask, StemHint};
use super::stack::int_to_fixed;
use crate::fixed::{div_fix, mul_fix};

/// Edge flags (`CF2_EdgeFlags`).
pub(super) const GHOST_BOTTOM: u8 = 0x1;
pub(super) const GHOST_TOP: u8 = 0x2;
pub(super) const PAIR_BOTTOM: u8 = 0x4;
pub(super) const PAIR_TOP: u8 = 0x8;
pub(super) const LOCKED: u8 = 0x10;
pub(super) const SYNTHETIC: u8 = 0x20;

/// `CF2_MAX_HINT_EDGES`.
const MAX_EDGES: usize = 192;

/// One edge of a hint map (`CF2_HintRec`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Hint {
    pub(super) flags: u8,
    /// The stem the edge comes from.
    pub(super) index: usize,
    pub(super) cs: i32,
    pub(super) ds: i32,
    /// The scale from this edge to the next.
    pub(super) scale: i32,
}

impl Hint {
    /// `cf2_hint_init`: the bottom or top edge of stem `index`, moved by
    /// `origin` (a `seac` accent's offset); a stem placed before keeps its
    /// place.
    fn from_stem(stems: &[StemHint], index: usize, origin: i32, scale: i32, bottom: bool) -> Self {
        let stem = stems[index];
        let width = stem.max.wrapping_sub(stem.min);
        let (flags, cs) = if width == int_to_fixed(-21) {
            if bottom {
                (GHOST_BOTTOM, stem.max)
            } else {
                (0, 0)
            }
        } else if width == int_to_fixed(-20) {
            if bottom {
                (0, 0)
            } else {
                (GHOST_TOP, stem.min)
            }
        } else if width < 0 {
            if bottom {
                (PAIR_BOTTOM, stem.max)
            } else {
                (PAIR_TOP, stem.min)
            }
        } else if bottom {
            (PAIR_BOTTOM, stem.min)
        } else {
            (PAIR_TOP, stem.max)
        };
        let mut hint = Self {
            flags,
            index,
            cs: cs.wrapping_add(origin),
            ds: 0,
            scale,
        };
        if flags != 0 && stem.used {
            hint.ds = if hint.is_top() {
                stem.max_ds
            } else {
                stem.min_ds
            };
            hint.flags |= LOCKED;
        } else {
            hint.ds = mul_fix(hint.cs.into(), scale.into()) as i32;
        }
        hint
    }

    pub(super) fn is_valid(&self) -> bool {
        self.flags != 0
    }

    fn is_pair(&self) -> bool {
        self.flags & (PAIR_BOTTOM | PAIR_TOP) != 0
    }

    fn is_pair_top(&self) -> bool {
        self.flags & PAIR_TOP != 0
    }

    pub(super) fn is_top(&self) -> bool {
        self.flags & (PAIR_TOP | GHOST_TOP) != 0
    }

    pub(super) fn is_bottom(&self) -> bool {
        self.flags & (PAIR_BOTTOM | GHOST_BOTTOM) != 0
    }

    fn is_locked(&self) -> bool {
        self.flags & LOCKED != 0
    }

    fn is_synthetic(&self) -> bool {
        self.flags & SYNTHETIC != 0
    }
}

/// `cf2_fixedFraction`.
const fn fraction(value: i32) -> i32 {
    value & 0xFFFF
}

/// A hint map.
#[derive(Clone, Debug)]
pub(super) struct HintMap {
    /// Whether the map moves coordinates (false when unhinted, or when a
    /// Type 1 glyph has too many stems).
    hinted: bool,
    pub(super) valid: bool,
    scale: i32,
    edges: Vec<Hint>,
}

impl HintMap {
    pub(super) fn new(hinted: bool, scale: i32) -> Self {
        Self {
            hinted,
            valid: false,
            scale,
            edges: Vec::new(),
        }
    }

    /// `cf2_hintmap_map`: a character-space vertical coordinate in device
    /// space.
    pub(super) fn map(&self, cs: i32) -> i32 {
        let scaled = |delta: i32, scale: i32| mul_fix(delta.into(), scale.into()) as i32;
        if self.edges.is_empty() || !self.hinted {
            return scaled(cs, self.scale);
        }
        // The highest edge at or below `cs`.
        let at = self.edges.iter().rposition(|edge| cs >= edge.cs);
        match at {
            None => {
                let first = self.edges[0];
                scaled(cs.wrapping_sub(first.cs), self.scale).wrapping_add(first.ds)
            }
            Some(at) => {
                let edge = self.edges[at];
                scaled(cs.wrapping_sub(edge.cs), edge.scale).wrapping_add(edge.ds)
            }
        }
    }

    /// `cf2_hintmap_insertHint`: inserts a stem's edges sorted by
    /// character-space position, unless they overlap edges already in.
    fn insert(&mut self, initial: Option<&HintMap>, bottom: Hint, top: Hint) {
        let (mut first, mut second, pair) = if bottom.is_valid() {
            (bottom, top, top.is_valid())
        } else {
            (top, top, false)
        };
        if pair && top.cs < bottom.cs {
            return;
        }
        let at = self
            .edges
            .iter()
            .position(|edge| edge.cs >= first.cs)
            .unwrap_or(self.edges.len());
        if let Some(next) = self.edges.get(at)
            && (next.cs == first.cs || (pair && next.cs <= second.cs) || next.is_pair_top())
        {
            return;
        }
        if let Some(initial) = initial.filter(|map| map.valid)
            && !first.is_locked()
        {
            if pair {
                // Center the stem where the initial map puts it, keeping
                // its width at the nominal scale.
                let half = second.cs.wrapping_sub(first.cs) / 2;
                let middle = initial.map(first.cs.wrapping_add(half));
                let half_width = mul_fix(half.into(), self.scale.into()) as i32;
                first.ds = middle.wrapping_sub(half_width);
                second.ds = middle.wrapping_add(half_width);
            } else {
                first.ds = initial.map(first.cs);
            }
        }
        if at > 0 && first.ds < self.edges[at - 1].ds {
            return;
        }
        if let Some(next) = self.edges.get(at) {
            let upper = if pair { second.ds } else { first.ds };
            if upper > next.ds {
                return;
            }
        }
        let last = if pair {
            self.edges.len() + 1
        } else {
            self.edges.len()
        };
        if last >= MAX_EDGES {
            return;
        }
        self.edges.insert(at, first);
        if pair {
            self.edges.insert(at + 1, second);
        }
    }

    /// `cf2_hintmap_adjustHints`: moves each unlocked stem so that one of
    /// its edges falls on a pixel boundary without overlapping its
    /// neighbours, then sets the scales between edges.
    fn adjust(&mut self) {
        let edges = &mut self.edges;
        let count = edges.len();
        let mut moves: Vec<(usize, i32)> = Vec::new();
        let mut i = 0;
        while i < count {
            let pair = edges[i].is_pair();
            let j = if pair { (i + 1).min(count - 1) } else { i };
            let (ds_i, ds_j) = (edges[i].ds, edges[j].ds);
            if !edges[i].is_locked() {
                let (frac_down, frac_up) = (fraction(ds_i), fraction(ds_j));
                let up = |frac: i32| if frac == 0 { 0 } else { (1 << 16) - frac };
                let move_up = up(frac_down).min(up(frac_up));
                let move_down = (-frac_down).max(-frac_up);
                let room_up =
                    j + 1 >= count || edges[j + 1].ds >= ds_j.wrapping_add(move_up + MIN_COUNTER);
                let room_down =
                    i == 0 || edges[i - 1].ds <= ds_i.wrapping_add(move_down - MIN_COUNTER);
                let (shift, save) = match (room_up, room_down) {
                    (true, true) => (
                        if -move_down < move_up {
                            move_down
                        } else {
                            move_up
                        },
                        false,
                    ),
                    (true, false) => (move_up, false),
                    (false, true) => (move_down, move_up < -move_down),
                    (false, false) => (0, true),
                };
                if save && j + 1 < count && !edges[j + 1].is_locked() {
                    moves.push((j, move_up - shift));
                }
                edges[i].ds = ds_i.wrapping_add(shift);
                if pair {
                    edges[j].ds = ds_j.wrapping_add(shift);
                }
            }
            let rescale = |edges: &mut Vec<Hint>, upper: usize| {
                let (low, high) = (edges[upper - 1], edges[upper]);
                if high.cs != low.cs {
                    edges[upper - 1].scale = div_fix(
                        high.ds.wrapping_sub(low.ds).into(),
                        high.cs.wrapping_sub(low.cs).into(),
                    ) as i32;
                }
            };
            if i > 0 {
                rescale(edges, i);
            }
            if pair && j > i {
                rescale(edges, j);
                i += 1;
            }
            i += 1;
        }
        // Second pass, top down: move the edges that could not move up.
        for &(j, move_up) in moves.iter().rev() {
            if edges[j + 1].ds >= edges[j].ds.wrapping_add(move_up + MIN_COUNTER) {
                edges[j].ds = edges[j].ds.wrapping_add(move_up);
                if edges[j].is_pair() && j > 0 {
                    edges[j - 1].ds = edges[j - 1].ds.wrapping_add(move_up);
                }
            }
        }
    }

    /// `cf2_hintmap_build`: the map of the horizontal stems `mask`
    /// activates (all of them without a mask), the ones captured by
    /// alignment zones first. The initial map (`initial` is `None`) only
    /// holds the captured stems, plus a locked edge at 0 if needed; other
    /// maps record where they placed their stems.
    pub(super) fn build(
        &mut self,
        initial: Option<&HintMap>,
        horizontal: &mut [StemHint],
        stems: usize,
        mask: &mut HintMask,
        origin: i32,
        blues: &Blues,
    ) {
        if !mask.valid && !mask.set_all(stems) {
            // Too many stems: a Type 1 glyph is drawn unhinted.
            self.hinted = false;
            return;
        }
        self.edges.clear();
        let mut active = *mask;
        let count = horizontal.len();
        if count > mask.bits {
            return;
        }
        if let Some((bottom, top)) = blues.em_box {
            self.insert(initial, bottom, Hint::default());
            self.insert(initial, Hint::default(), top);
        }
        let scale = self.scale;
        let edges = |index: usize, stems: &[StemHint]| {
            (
                Hint::from_stem(stems, index, origin, scale, true),
                Hint::from_stem(stems, index, origin, scale, false),
            )
        };
        for index in 0..count {
            if !active.is_set(index) {
                continue;
            }
            let (mut bottom, mut top) = edges(index, horizontal);
            if bottom.is_locked() || top.is_locked() || blues.capture(&mut bottom, &mut top) {
                self.insert(initial, bottom, top);
                active.clear(index);
            }
        }
        if initial.is_none() {
            let below = self.edges.first().is_none_or(|edge| edge.cs > 0);
            let above = self.edges.last().is_some_and(|edge| edge.cs < 0);
            if below || above {
                // Lock the baseline for glyphs without baseline hints.
                let zero = Hint {
                    flags: GHOST_BOTTOM | LOCKED | SYNTHETIC,
                    scale: self.scale,
                    ..Hint::default()
                };
                self.insert(None, zero, Hint::default());
            }
        } else {
            for index in (0..count).filter(|&index| active.is_set(index)) {
                let (bottom, top) = edges(index, horizontal);
                self.insert(initial, bottom, top);
            }
        }
        self.adjust();
        if initial.is_some() {
            for edge in self.edges.iter().filter(|edge| !edge.is_synthetic()) {
                let stem = &mut horizontal[edge.index];
                if edge.is_top() {
                    stem.max_ds = edge.ds;
                } else {
                    stem.min_ds = edge.ds;
                }
                stem.used = true;
            }
        }
        self.valid = true;
        mask.new = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type1::cf2::blues::PrivateDict;

    fn stems(pairs: &[(i32, i32)]) -> Vec<StemHint> {
        pairs
            .iter()
            .map(|&(min, max)| StemHint::new(min << 16, max << 16))
            .collect()
    }

    #[test]
    fn stems_align_to_zones_and_pixels() {
        let private = PrivateDict {
            blue_values: vec![-22, 0, 683, 705],
            ..PrivateDict::default()
        };
        let scale = 6554;
        let blues = Blues::new(&private, scale);
        let mut horizontal = stems(&[(0, 31), (318, 349), (649, 680)]);
        let mut initial = HintMap::new(true, scale);
        initial.build(
            None,
            &mut horizontal,
            3,
            &mut HintMask::default(),
            0,
            &blues,
        );
        assert!(initial.valid);
        let mut map = HintMap::new(true, scale);
        let mut mask = HintMask::default();
        map.build(Some(&initial), &mut horizontal, 3, &mut mask, 0, &blues);
        assert!(map.valid && !mask.new);
        assert_eq!(map.edges.len(), 6);
        // The baseline stem sits on the zone's flat edge.
        assert_eq!(map.map(0), 0);
        // Every stem gets an edge on the pixel grid.
        for pair in map.edges.chunks(2) {
            assert!(pair.iter().any(|edge| fraction(edge.ds) == 0), "{pair:?}");
        }
        assert!(horizontal.iter().all(|stem| stem.used));
        // Positions are kept when the stems are used again.
        let edge = Hint::from_stem(&horizontal, 1, 0, scale, true);
        assert!(edge.is_locked());
    }

    #[test]
    fn ghosts_inverted_stems_and_unhinted_maps() {
        let ghosts = stems(&[(10, -11), (30, 10)]);
        let bottom = Hint::from_stem(&ghosts, 0, 0, 0x10000, true);
        assert_eq!((bottom.flags, bottom.cs), (GHOST_BOTTOM, -11 << 16));
        assert!(!Hint::from_stem(&ghosts, 0, 0, 0x10000, false).is_valid());
        let inverted = stems(&[(30, 5)]);
        let top = Hint::from_stem(&inverted, 0, 1 << 16, 0x10000, false);
        assert_eq!((top.flags, top.cs), (PAIR_TOP, 31 << 16));
        let unhinted = HintMap::new(false, 0x400);
        assert_eq!(unhinted.map(64 << 16), 1 << 16);
        let mut too_many = HintMap::new(true, 0x10000);
        too_many.build(
            None,
            &mut stems(&[(0, 1); 97]),
            97,
            &mut HintMask::default(),
            0,
            &Blues::default(),
        );
        assert!(!too_many.valid && !too_many.hinted);
    }
}
