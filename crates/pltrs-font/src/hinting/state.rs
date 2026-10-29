//! The graphics state and the functions it selects: rounding, projection
//! onto the projection (and dual) vector, and movement along the freedom
//! vector (FreeType's `TT_GraphicsState`, `Round_*`, `Project*` and
//! `Compute_Funcs`).

use crate::fixed::{dot_fix14, pix_ceil, pix_floor, pix_round};

/// A unit vector in 2.14 fixed point (`FT_UnitVector`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct UnitVector {
    pub(crate) x: i16,
    pub(crate) y: i16,
}

impl UnitVector {
    /// The x axis.
    pub(crate) const X: Self = Self { x: 0x4000, y: 0 };
    /// The y axis.
    pub(crate) const Y: Self = Self { x: 0, y: 0x4000 };
}

/// The TrueType graphics state, without the fields FreeType never reads
/// (the round state lives with the rounding function, and the engine
/// compensations are always zero).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct GraphicsState {
    pub(crate) rp0: u16,
    pub(crate) rp1: u16,
    pub(crate) rp2: u16,
    /// Zone numbers of the zone pointers (0 twilight, 1 glyph).
    pub(crate) gep0: u16,
    pub(crate) gep1: u16,
    pub(crate) gep2: u16,
    pub(crate) dual_vector: UnitVector,
    pub(crate) proj_vector: UnitVector,
    pub(crate) free_vector: UnitVector,
    pub(crate) loop_count: i64,
    pub(crate) minimum_distance: i64,
    pub(crate) control_value_cutin: i64,
    pub(crate) single_width_cutin: i64,
    pub(crate) single_width_value: i64,
    pub(crate) delta_base: u16,
    pub(crate) delta_shift: u16,
    pub(crate) auto_flip: bool,
    pub(crate) instruct_control: u8,
    pub(crate) scan_control: bool,
    pub(crate) scan_type: i32,
}

impl Default for GraphicsState {
    /// `tt_default_graphics_state`.
    fn default() -> Self {
        Self {
            rp0: 0,
            rp1: 0,
            rp2: 0,
            gep0: 1,
            gep1: 1,
            gep2: 1,
            dual_vector: UnitVector::X,
            proj_vector: UnitVector::X,
            free_vector: UnitVector::X,
            loop_count: 1,
            minimum_distance: 64,
            control_value_cutin: 68,
            single_width_cutin: 0,
            single_width_value: 0,
            delta_base: 9,
            delta_shift: 3,
            auto_flip: true,
            instruct_control: 0,
            scan_control: false,
            scan_type: 0,
        }
    }
}

impl GraphicsState {
    /// The default state with the values a font or control value program
    /// may set for the glyph programs copied from `self`
    /// (`TT_Save_Context`).
    pub(crate) fn saved(&self) -> Self {
        Self {
            minimum_distance: self.minimum_distance,
            control_value_cutin: self.control_value_cutin,
            single_width_cutin: self.single_width_cutin,
            single_width_value: self.single_width_value,
            delta_base: self.delta_base,
            delta_shift: self.delta_shift,
            auto_flip: self.auto_flip,
            instruct_control: self.instruct_control,
            scan_control: self.scan_control,
            scan_type: self.scan_type,
            ..Self::default()
        }
    }
}

/// The rounding function in effect.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Rounding {
    HalfGrid,
    Grid,
    DoubleGrid,
    DownToGrid,
    UpToGrid,
    Off,
    /// `SROUND`: period, phase and threshold in 26.6.
    Super(SuperRound),
    /// `S45ROUND`: like `Super` on a √2/2 grid, computed by division.
    Super45(SuperRound),
}

/// Super-rounding parameters in 26.6 (`SetSuperRound`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SuperRound {
    period: i64,
    phase: i64,
    threshold: i64,
}

impl SuperRound {
    /// Decodes an `SROUND`/`S45ROUND` selector for a grid period in 2.14.
    pub(crate) const fn new(grid_period: i64, selector: i64) -> Self {
        let period = match selector & 0xC0 {
            0 => grid_period / 2,
            0x80 => grid_period * 2,
            _ => grid_period,
        };
        let phase = match selector & 0x30 {
            0 => 0,
            0x10 => period / 4,
            0x20 => period / 2,
            _ => period * 3 / 4,
        };
        let threshold_eighths = selector & 0x0F;
        let threshold = if threshold_eighths == 0 {
            period - 1
        } else {
            (threshold_eighths - 4) * period / 8
        };
        Self {
            period: period >> 8,
            phase: phase >> 8,
            threshold: threshold >> 8,
        }
    }
}

/// Applies a rounding to a non-negative distance and mirrors it for
/// negative ones, keeping the sign as FreeType's `Round_*` functions do.
fn symmetric(distance: i64, positive: impl Fn(i64) -> i64, floor: i64) -> i64 {
    if distance >= 0 {
        let value = positive(distance);
        if value < 0 { floor } else { value }
    } else {
        let value = positive(distance.wrapping_neg()).wrapping_neg();
        if value > 0 { -floor } else { value }
    }
}

impl Rounding {
    /// Rounds `distance` (26.6), with zero engine compensation.
    pub(crate) fn round(self, distance: i64) -> i64 {
        match self {
            Self::Off => distance,
            Self::Grid => symmetric(distance, pix_round, 0),
            Self::HalfGrid => symmetric(distance, |d| pix_floor(d).wrapping_add(32), 32),
            Self::DownToGrid => symmetric(distance, pix_floor, 0),
            Self::UpToGrid => symmetric(distance, pix_ceil, 0),
            Self::DoubleGrid => symmetric(distance, |d| d.wrapping_add(16) & !31, 0),
            Self::Super(s) => symmetric(
                distance,
                |d| (d.wrapping_add(s.threshold - s.phase) & -s.period).wrapping_add(s.phase),
                s.phase,
            ),
            Self::Super45(s) => symmetric(
                distance,
                |d| {
                    let steps = d
                        .wrapping_add(s.threshold - s.phase)
                        .checked_div(s.period)
                        .unwrap_or(0);
                    steps.wrapping_mul(s.period).wrapping_add(s.phase)
                },
                s.phase,
            ),
        }
    }
}

/// How distances are measured: along an axis or a general unit vector.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Projection {
    X,
    Y,
    Along(UnitVector),
}

impl Projection {
    /// The projection for a unit vector (`Project_x`, `Project_y`,
    /// `Project`).
    pub(crate) const fn of(vector: UnitVector) -> Self {
        if vector.x == 0x4000 {
            Self::X
        } else if vector.y == 0x4000 {
            Self::Y
        } else {
            Self::Along(vector)
        }
    }

    /// Projects the vector `(dx, dy)`.
    pub(crate) const fn project(self, dx: i64, dy: i64) -> i64 {
        match self {
            Self::X => dx,
            Self::Y => dy,
            Self::Along(v) => dot_fix14(dx, dy, v.x as i64, v.y as i64),
        }
    }
}

/// How points move along the freedom vector.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Movement {
    /// Freedom and projection vectors both along x (`Direct_Move_X`).
    X,
    /// Freedom and projection vectors both along y (`Direct_Move_Y`).
    Y,
    /// By the projected freedom vector `(x, y)` in 16.16 (`Direct_Move`).
    Along(i64, i64),
}

/// The functions the graphics state's vectors select (`Compute_Funcs`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Funcs {
    pub(crate) project: Projection,
    pub(crate) dual_project: Projection,
    pub(crate) movement: Movement,
    /// The freedom vector scaled by `1 / (f · p)` in 16.16 (FreeType's
    /// `moveVector`), also used where FreeType reads it directly.
    pub(crate) move_vector: (i64, i64),
}

impl Funcs {
    /// `Compute_Funcs` for the vectors of `gs`.
    pub(crate) fn new(gs: &GraphicsState) -> Self {
        let (p, f) = (gs.proj_vector, gs.free_vector);
        let f_dot_p =
            (i64::from(p.x) * i64::from(f.x) + i64::from(p.y) * i64::from(f.y) + 0x2000) >> 14;
        let move_vector = if f_dot_p >= 0x3FFE {
            (i64::from(f.x) * 4, i64::from(f.y) * 4)
        } else if -0x400 < f_dot_p && f_dot_p < 0x400 {
            (0, 0)
        } else {
            (
                i64::from(f.x) * 0x10000 / f_dot_p,
                i64::from(f.y) * 0x10000 / f_dot_p,
            )
        };
        let movement = if f_dot_p >= 0x3FFE && f.x == 0x4000 {
            Movement::X
        } else if f_dot_p >= 0x3FFE && f.y == 0x4000 {
            Movement::Y
        } else {
            Movement::Along(move_vector.0, move_vector.1)
        };
        Self {
            project: Projection::of(p),
            dual_project: Projection::of(gs.dual_vector),
            movement,
            move_vector,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_roundings_keep_the_sign() {
        assert_eq!(Rounding::Grid.round(95), 64);
        assert_eq!(Rounding::Grid.round(96), 128);
        assert_eq!(Rounding::Grid.round(-96), -128);
        assert_eq!(Rounding::Grid.round(-20), 0);
        assert_eq!(Rounding::HalfGrid.round(10), 32);
        assert_eq!(Rounding::HalfGrid.round(-10), -32);
        assert_eq!(Rounding::DoubleGrid.round(47), 32);
        assert_eq!(Rounding::DownToGrid.round(-127), -64);
        assert_eq!(Rounding::UpToGrid.round(1), 64);
        assert_eq!(Rounding::Off.round(-7), -7);
        // Overflow at the extremes falls back to the floor value.
        assert_eq!(Rounding::Grid.round(i64::MAX), 0);
    }

    #[test]
    fn super_rounding() {
        // SROUND 0x48: period 1 px, phase 0, threshold half a pixel.
        let half = SuperRound::new(0x4000, 0x48);
        assert_eq!(Rounding::Super(half).round(95), 64);
        assert_eq!(Rounding::Super(half).round(-96), -128);
        // Phase of half a pixel.
        let phased = SuperRound::new(0x4000, 0x68);
        assert_eq!(Rounding::Super(phased).round(10), 32);
        let diagonal = SuperRound::new(0x2D41, 0x48);
        assert_eq!(Rounding::Super45(diagonal).round(100), 90);
        assert_eq!(Rounding::Super45(diagonal).round(-100), -90);
    }

    #[test]
    fn funcs_follow_the_vectors() {
        let mut gs = GraphicsState::default();
        assert_eq!(Funcs::new(&gs).movement, Movement::X);
        gs.free_vector = UnitVector::Y;
        gs.proj_vector = UnitVector::Y;
        assert_eq!(Funcs::new(&gs).movement, Movement::Y);
        gs.proj_vector = UnitVector::X;
        let orthogonal = Funcs::new(&gs);
        assert_eq!(orthogonal.movement, Movement::Along(0, 0));
        assert_eq!(orthogonal.project, Projection::X);
        gs.proj_vector = UnitVector {
            x: 0x2D41,
            y: 0x2D41,
        };
        let diagonal = Funcs::new(&gs);
        assert_eq!(diagonal.move_vector, (0, 0x4000 * 0x10000 / 0x2D41));
        assert_eq!(diagonal.project.project(64, 64), 91);
    }
}
