//! Alignment zones (`psblues.c`): the font's blue values scaled to the
//! device, and the capture of stem edges by them.

use super::hintmap::{GHOST_BOTTOM, GHOST_TOP, Hint, LOCKED, SYNTHETIC};
use super::stack::int_to_fixed;
use crate::fixed::{div_fix, mul_div, mul_fix};

/// `CF2_MIN_COUNTER`: half a pixel.
pub(super) const MIN_COUNTER: i32 = 0x8000;

/// The top and bottom of the ideographic em box (`CF2_ICF_Top`,
/// `CF2_ICF_Bottom`).
const ICF_TOP: i32 = 880 << 16;
const ICF_BOTTOM: i32 = -120 << 16;

/// The hinting entries of a font's Private dictionary, in font units, as
/// FreeType reads them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PrivateDict {
    pub(crate) blue_values: Vec<i32>,
    pub(crate) other_blues: Vec<i32>,
    pub(crate) family_blues: Vec<i32>,
    pub(crate) family_other_blues: Vec<i32>,
    /// `BlueScale` times 1000, in 16.16.
    pub(crate) blue_scale: i32,
    pub(crate) blue_shift: i32,
    pub(crate) blue_fuzz: i32,
    pub(crate) language_group: i32,
}

impl Default for PrivateDict {
    /// FreeType's defaults (`T1_Open_Face`).
    fn default() -> Self {
        Self {
            blue_values: Vec::new(),
            other_blues: Vec::new(),
            family_blues: Vec::new(),
            family_other_blues: Vec::new(),
            blue_scale: 2_596_864,
            blue_shift: 7,
            blue_fuzz: 1,
            language_group: 0,
        }
    }
}

/// `cf2_fixedRound`.
pub(super) const fn fixed_round(value: i32) -> i32 {
    ((value as u32).wrapping_add(0x8000) & 0xFFFF_0000) as i32
}

/// One alignment zone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Zone {
    cs_bottom: i32,
    cs_top: i32,
    cs_flat: i32,
    ds_flat: i32,
    bottom: bool,
}

/// The alignment zones of a font at a scale.
#[derive(Clone, Debug, Default)]
pub(crate) struct Blues {
    blue_shift: i32,
    blue_fuzz: i32,
    zones: Vec<Zone>,
    suppress_overshoot: bool,
    /// The synthetic em box edges of ideographic fonts without zones.
    pub(super) em_box: Option<(Hint, Hint)>,
}

/// `values[index]` as FreeType reads it from its fixed-size arrays, which
/// are zero past the count.
fn value(values: &[i32], index: usize) -> i32 {
    values.get(index).copied().unwrap_or(0)
}

/// The flat edge of a zone moved to the closest of the font family's
/// within a pixel, and that distance (`i32::MAX` when none is).
fn family_edge(flat: i32, candidates: impl Iterator<Item = i32>, pixel: i32) -> (i32, i32) {
    let mut best = flat;
    let mut min_diff = i32::MAX;
    for candidate in candidates {
        let diff = flat.wrapping_sub(candidate).wrapping_abs();
        if diff < min_diff && diff < pixel {
            best = candidate;
            min_diff = diff;
            if diff == 0 {
                break;
            }
        }
    }
    (best, min_diff)
}

impl Blues {
    /// `cf2_blues_init`: the zones of `private` at vertical scale `scale`
    /// (stem darkening is off, so zones are not moved for it).
    pub(super) fn new(private: &PrivateDict, scale: i32) -> Self {
        let fixed = |values: &[i32]| values.iter().map(|&v| int_to_fixed(v)).collect::<Vec<_>>();
        let blue_values = fixed(&private.blue_values);
        let other_blues = fixed(&private.other_blues);
        let family_blues = fixed(&private.family_blues);
        let family_other = fixed(&private.family_other_blues);
        let mut blues = Self {
            blue_shift: int_to_fixed(private.blue_shift),
            blue_fuzz: int_to_fixed(private.blue_fuzz),
            ..Self::default()
        };
        let mut blue_scale = div_fix(i64::from(private.blue_scale), 1000 << 16) as i32;
        let no_zones = blue_values.is_empty()
            || (blue_values.len() == 4
                && blue_values[0] < ICF_BOTTOM
                && blue_values[1] < ICF_BOTTOM
                && blue_values[2] > ICF_TOP
                && blue_values[3] > ICF_TOP);
        if private.language_group == 1 && no_zones {
            blues.em_box = Some(Self::em_box(scale));
            return blues;
        }
        let mut max_height = 0;
        let mut add_zones = |values: &[i32], first_is_bottom: bool, all_bottom: bool| {
            for i in (0..values.len()).step_by(2) {
                let (cs_bottom, cs_top) = (value(values, i), value(values, i + 1));
                let height = cs_top.wrapping_sub(cs_bottom);
                if height < 0 {
                    continue;
                }
                max_height = max_height.max(height);
                let bottom = all_bottom || (first_is_bottom && i == 0);
                blues.zones.push(Zone {
                    cs_bottom,
                    cs_top,
                    cs_flat: if bottom { cs_top } else { cs_bottom },
                    ds_flat: 0,
                    bottom,
                });
            }
        };
        add_zones(&blue_values, true, false);
        add_zones(&other_blues, false, true);
        let pixel = div_fix(1 << 16, i64::from(scale)) as i32;
        for zone in &mut blues.zones {
            zone.cs_flat = if zone.bottom {
                let others = (0..family_other.len())
                    .step_by(2)
                    .map(|j| value(&family_other, j + 1));
                let (edge, min_diff) = family_edge(zone.cs_flat, others, pixel);
                // The first FamilyBlues zone is a bottom zone too.
                match family_blues.get(1) {
                    Some(&first) => {
                        let diff = zone.cs_flat.wrapping_sub(first).wrapping_abs();
                        if diff < min_diff && diff < pixel {
                            first
                        } else {
                            edge
                        }
                    }
                    None => edge,
                }
            } else {
                let tops = (2..family_blues.len()).step_by(2).map(|j| family_blues[j]);
                family_edge(zone.cs_flat, tops, pixel).0
            };
        }
        if max_height > 0 {
            let limit = div_fix(1 << 16, i64::from(max_height)) as i32;
            blue_scale = blue_scale.min(limit);
        }
        let mut boost = 0;
        if scale < blue_scale {
            blues.suppress_overshoot = true;
            let six_tenths = 39_322;
            boost = six_tenths - mul_div(six_tenths.into(), scale.into(), blue_scale.into()) as i32;
            boost = boost.min(0x7FFF);
        }
        for zone in &mut blues.zones {
            let scaled = mul_fix(zone.cs_flat.into(), scale.into()) as i32;
            zone.ds_flat = fixed_round(if zone.bottom {
                scaled.wrapping_sub(boost)
            } else {
                scaled.wrapping_add(boost)
            });
        }
        blues
    }

    /// The synthetic ghost hints at the bottom and top of the em box.
    fn em_box(scale: i32) -> (Hint, Hint) {
        let edge = |cs: i32, flag: u8, offset: i32| Hint {
            flags: flag | LOCKED | SYNTHETIC,
            index: 0,
            cs,
            ds: fixed_round(mul_fix(cs.into(), scale.into()) as i32).wrapping_add(offset),
            scale,
        };
        (
            edge(ICF_BOTTOM - 1, GHOST_BOTTOM, -MIN_COUNTER),
            edge(ICF_TOP + 1, GHOST_TOP, MIN_COUNTER),
        )
    }

    /// `cf2_blues_capture`: whether a zone captures the stem's bottom or
    /// top edge; if so both edges move to align the captured one and are
    /// locked.
    pub(super) fn capture(&self, bottom: &mut Hint, top: &mut Hint) -> bool {
        let fuzz = self.blue_fuzz;
        let mut moved = None;
        for zone in &self.zones {
            let inside = |cs: i32| {
                zone.cs_bottom.wrapping_sub(fuzz) <= cs && cs <= zone.cs_top.wrapping_add(fuzz)
            };
            if zone.bottom && bottom.is_bottom() && inside(bottom.cs) {
                let new = if self.suppress_overshoot {
                    zone.ds_flat
                } else if zone.cs_top.wrapping_sub(bottom.cs) >= self.blue_shift {
                    fixed_round(bottom.ds).min(zone.ds_flat.wrapping_sub(1 << 16))
                } else {
                    fixed_round(bottom.ds)
                };
                moved = Some(new.wrapping_sub(bottom.ds));
                break;
            }
            if !zone.bottom && top.is_top() && inside(top.cs) {
                let new = if self.suppress_overshoot {
                    zone.ds_flat
                } else if top.cs.wrapping_sub(zone.cs_bottom) >= self.blue_shift {
                    fixed_round(top.ds).max(zone.ds_flat.wrapping_add(1 << 16))
                } else {
                    fixed_round(top.ds)
                };
                moved = Some(new.wrapping_sub(top.ds));
                break;
            }
        }
        let Some(delta) = moved else {
            return false;
        };
        for edge in [bottom, top] {
            if edge.is_valid() {
                edge.ds = edge.ds.wrapping_add(delta);
                edge.flags |= LOCKED;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmr10() -> PrivateDict {
        PrivateDict {
            blue_values: vec![-22, 0, 683, 705, 431, 448, 666, 677],
            other_blues: vec![-206, -194],
            ..PrivateDict::default()
        }
    }

    #[test]
    fn zones_at_100_ppem() {
        // (419430 + 32) / 64: FreeType's hinted scale at 100 pixels per em.
        let blues = Blues::new(&cmr10(), 6554);
        assert_eq!(blues.zones.len(), 5);
        assert!(!blues.suppress_overshoot);
        assert_eq!(blues.zones[0].ds_flat, 0);
        assert_eq!(blues.zones[1].ds_flat, 68 << 16);
        assert!(blues.zones[4].bottom);
        let mut bottom = Hint {
            flags: super::super::hintmap::PAIR_BOTTOM,
            cs: -10 << 16,
            ds: mul_fix(-10 << 16, 6554) as i32,
            ..Hint::default()
        };
        let mut top = Hint::default();
        assert!(blues.capture(&mut bottom, &mut top));
        assert!(bottom.flags & LOCKED != 0 && top.flags == 0);
        assert_eq!(bottom.ds, -1 << 16);
    }

    #[test]
    fn small_sizes_suppress_overshoot() {
        let blues = Blues::new(&cmr10(), 600);
        assert!(blues.suppress_overshoot);
        let ideographic = PrivateDict {
            language_group: 1,
            ..PrivateDict::default()
        };
        let blues = Blues::new(&ideographic, 6554);
        assert!(blues.em_box.is_some() && blues.zones.is_empty());
        assert_eq!(fixed_round(0x18000), 0x20000);
    }
}
