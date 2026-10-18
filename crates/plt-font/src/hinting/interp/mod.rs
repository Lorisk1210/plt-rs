//! The TrueType bytecode interpreter: FreeType 2.14's `TT_RunIns` with its
//! version 40 ("minimal subpixel hinting") behaviour.
//!
//! An [`Exec`] runs one program over borrowed state (definitions, control
//! values, storage, the twilight zone and the glyph zone). The opcodes are
//! implemented in topic modules as further `impl Exec` blocks. FreeType's
//! limits apply: a million instructions per run, the loop-call and
//! backward-jump budgets of `TT_Run_Context`, 32 nested calls and a
//! restricted twilight zone; a work budget on top of them bounds the
//! points an instruction may visit, so hostile bytecode always stops
//! quickly.

mod data;
mod flow;
mod gstate;
mod iup;
mod points;
mod shift;
#[cfg(test)]
mod tests;

use super::error::{BytecodeError, BytecodeProgram};
use super::state::{Funcs, GraphicsState, Movement, Rounding};
use super::zone::{TAG_TOUCH_X, TAG_TOUCH_Y, Vector, Zone};

pub(crate) use flow::{DefinitionTable, Definitions};

/// FreeType's `TT_CONFIG_OPTION_MAX_RUNNABLE_OPCODES`.
const MAX_RUNNABLE_OPCODES: u32 = 1_000_000;

/// Nested calls allowed (`callSize`).
const MAX_CALL_DEPTH: usize = 32;

/// Backward compatibility mode is on (bit 2 of FreeType's
/// `backward_compatibility`); bits 0 and 1 record `IUP[y]` and `IUP[x]`.
pub(crate) const BACKWARD_COMPATIBILITY: u8 = 0x4;

/// Both axes have been interpolated in backward compatibility mode: points
/// no longer move.
const POST_IUP: u8 = 0x7;

/// Size-dependent values the programs read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Metrics {
    /// Pixels per em (`MPPEM`).
    pub(crate) ppem: i64,
    /// Point size in 26.6 (`MPS`).
    pub(crate) point_size: i64,
    /// Font units to 26.6, 16.16 (`WCVTF`, `SSW`).
    pub(crate) scale: i64,
    /// Scale applied to `orus` distances: the size's scale for simple
    /// glyphs, 1.0 for composites, whose `orus` are already hinted.
    pub(crate) orus_scale: i64,
}

/// The values programs may change, owned by the hinting instance (for the
/// font and control value programs) or by one glyph load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Resources {
    pub(crate) cvt: Vec<i64>,
    pub(crate) storage: Vec<i64>,
    pub(crate) twilight: Zone,
    /// Usable twilight points; only ever reduced (`TT_Run_Context`).
    pub(crate) twilight_len: usize,
    /// FreeType's `backward_compatibility` bits, kept across the runs of
    /// one glyph load.
    pub(crate) backward_compatibility: u8,
    /// What remains of the work budget.
    pub(crate) work: u64,
}

/// The state a program runs on.
#[derive(Debug)]
pub(crate) struct State<'r> {
    pub(crate) definitions: Definitions<'r>,
    pub(crate) resources: &'r mut Resources,
    pub(crate) metrics: Metrics,
    /// Number of glyphs in the face (bounds the loop budgets).
    pub(crate) num_glyphs: u16,
    /// Interpreter stack size.
    pub(crate) stack_size: usize,
}

/// One of the two zones a zone pointer may select.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ZoneId {
    Twilight,
    Glyph,
}

/// A pending function call (`TT_CallRec`).
#[derive(Clone, Copy, Debug)]
struct Call {
    caller_range: BytecodeProgram,
    caller_ip: i64,
    count: i64,
    definition: flow::Definition,
}

/// The execution context of one program run.
#[derive(Debug)]
pub(crate) struct Exec<'r> {
    state: State<'r>,
    pts: &'r mut Zone,
    gs: GraphicsState,
    is_composite: bool,
    ranges: [Option<&'r [u8]>; 3],
    range: BytecodeProgram,
    initial: BytecodeProgram,
    code: &'r [u8],
    ip: i64,
    length: i64,
    opcode: u8,
    stack: Vec<i64>,
    top: usize,
    args: usize,
    new_top: usize,
    calls: Vec<Call>,
    funcs: Funcs,
    rounding: Rounding,
    zp: [ZoneId; 3],
    loopcall_counter: u64,
    neg_jump_counter: u64,
    loop_limit: u64,
    error: Option<BytecodeError>,
}

/// The code of the three programs (`codeRangeTable`); `None` for a
/// program that is empty or not loaded.
pub(crate) type CodeRanges<'r> = [Option<&'r [u8]>; 3];

const fn range_index(program: BytecodeProgram) -> usize {
    match program {
        BytecodeProgram::Font => 0,
        BytecodeProgram::ControlValue => 1,
        BytecodeProgram::Glyph => 2,
    }
}

impl<'r> Exec<'r> {
    /// Prepares a run of `program` (`TT_Run_Context`): the graphics state
    /// starts as `gs`, every zone pointer selects the glyph zone `pts`.
    pub(crate) fn new(
        state: State<'r>,
        pts: &'r mut Zone,
        gs: GraphicsState,
        ranges: CodeRanges<'r>,
        program: BytecodeProgram,
        is_composite: bool,
    ) -> Self {
        let points = pts.len() as u64;
        let resources = &mut *state.resources;
        let cvt_len = resources.cvt.len() as u64;
        let twilight_limit = (2 * (points + cvt_len)).clamp(30, 0xFFFF) as usize;
        resources.twilight_len = resources.twilight_len.min(twilight_limit);
        let loop_limit = if points > 0 {
            (10 * points).max(50) + (cvt_len / 10).max(50)
        } else {
            300 + 22 * cvt_len
        }
        .min(100 * u64::from(state.num_glyphs));
        resources.backward_compatibility &= !0x3;
        let stack = vec![0; state.stack_size];
        Self {
            state,
            pts,
            funcs: Funcs::new(&gs),
            gs,
            is_composite,
            ranges,
            range: program,
            initial: program,
            code: ranges[range_index(program)].unwrap_or(&[]),
            ip: 0,
            length: 0,
            opcode: 0,
            stack,
            top: 0,
            args: 0,
            new_top: 0,
            calls: Vec::new(),
            rounding: Rounding::Grid,
            zp: [ZoneId::Glyph; 3],
            loopcall_counter: 0,
            neg_jump_counter: 0,
            loop_limit,
            error: None,
        }
    }

    /// The graphics state as the program left it.
    pub(crate) const fn graphics_state(&self) -> &GraphicsState {
        &self.gs
    }

    /// Runs the program to its end (`TT_RunIns`).
    pub(crate) fn run(&mut self) -> Result<(), BytecodeError> {
        if self.code.is_empty() {
            return Ok(());
        }
        let mut count = 0;
        loop {
            count += 1;
            if count > MAX_RUNNABLE_OPCODES {
                return Err(BytecodeError::ExecutionTooLong);
            }
            self.spend(1)?;
            let opcode = self.code[self.ip as usize];
            self.opcode = opcode;
            self.length = 1;
            let (pops, pushes) = stack_effect(opcode);
            if self.top < pops {
                // Like FreeType without pedantic hinting: missing
                // arguments read as zero.
                self.stack[..pops].fill(0);
                self.args = 0;
            } else {
                self.args = self.top - pops;
            }
            self.new_top = self.args + pushes;
            if self.new_top > self.stack.len() {
                return Err(BytecodeError::StackOverflow);
            }
            self.dispatch(opcode);
            if let Some(error) = self.error.take() {
                return Err(error);
            }
            self.top = self.new_top;
            self.ip = self.ip.wrapping_add(self.length);
            if self.ip < 0 || self.ip as usize >= self.code.len() {
                // The end of the code: fine at the top level, an overflow
                // inside a function.
                return if self.calls.is_empty() {
                    Ok(())
                } else {
                    Err(BytecodeError::CodeOverflow)
                };
            }
        }
    }

    /// Charges `units` of work to the budget.
    fn spend(&mut self, units: u64) -> Result<(), BytecodeError> {
        let work = &mut self.state.resources.work;
        *work = work
            .checked_sub(units)
            .ok_or(BytecodeError::ExecutionTooLong)?;
        Ok(())
    }

    /// Charges `units` of work from inside an instruction.
    fn charge(&mut self, units: u64) -> bool {
        match self.spend(units) {
            Ok(()) => true,
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    /// The `i`-th argument of the current instruction.
    fn arg(&self, i: usize) -> i64 {
        self.stack[self.args + i]
    }

    /// Sets the `i`-th argument (or result) of the current instruction.
    fn set_arg(&mut self, i: usize, value: i64) {
        self.stack[self.args + i] = value;
    }

    /// The element `depth` places below the current instruction's
    /// arguments (`args[-depth]`).
    fn below(&self, depth: usize) -> i64 {
        self.stack[self.args - depth]
    }

    fn fail(&mut self, error: BytecodeError) {
        self.error = Some(error);
    }

    fn zone(&self, id: ZoneId) -> &Zone {
        match id {
            ZoneId::Twilight => &self.state.resources.twilight,
            ZoneId::Glyph => self.pts,
        }
    }

    fn zone_mut(&mut self, id: ZoneId) -> &mut Zone {
        match id {
            ZoneId::Twilight => &mut self.state.resources.twilight,
            ZoneId::Glyph => self.pts,
        }
    }

    /// Number of addressable points of a zone.
    fn zone_len(&self, id: ZoneId) -> usize {
        match id {
            ZoneId::Twilight => self.state.resources.twilight_len,
            ZoneId::Glyph => self.pts.len(),
        }
    }

    /// `index` if zone pointer `zp` has such a point.
    fn checked(&self, zp: usize, index: u64) -> Option<usize> {
        (index < self.zone_len(self.zp[zp]) as u64).then_some(index as usize)
    }

    /// The point a 16-bit argument names in zone pointer `zp` (FreeType's
    /// `(FT_UShort)` casts), if it exists.
    fn point16(&self, zp: usize, value: i64) -> Option<usize> {
        self.checked(zp, u64::from(value as u16))
    }

    fn cur(&self, zp: usize, point: usize) -> Vector {
        self.zone(self.zp[zp]).cur[point]
    }

    fn org(&self, zp: usize, point: usize) -> Vector {
        self.zone(self.zp[zp]).org[point]
    }

    fn orus(&self, zp: usize, point: usize) -> Vector {
        self.zone(self.zp[zp]).orus[point]
    }

    /// Projects `a - b` onto the projection vector.
    fn project(&self, a: Vector, b: Vector) -> i64 {
        let d = a.minus(b);
        self.funcs.project.project(d.x, d.y)
    }

    /// Projects `a - b` onto the dual projection vector.
    fn dual_project(&self, a: Vector, b: Vector) -> i64 {
        let d = a.minus(b);
        self.funcs.dual_project.project(d.x, d.y)
    }

    /// Recomputes the projection and movement functions after a vector
    /// changed.
    fn compute_funcs(&mut self) {
        self.funcs = Funcs::new(&self.gs);
    }

    fn backward_compatibility(&self) -> u8 {
        self.state.resources.backward_compatibility
    }

    /// Moves a point of zone pointer `zp` by `distance` along the freedom
    /// vector and touches it (`Direct_Move*`): no x movement in backward
    /// compatibility mode, no movement at all after both `IUP`s.
    fn move_point(&mut self, zp: usize, point: usize, distance: i64) {
        let bc = self.backward_compatibility();
        let movement = self.funcs.movement;
        let zone = self.zone_mut(self.zp[zp]);
        let (dx, dy) = match movement {
            Movement::X => (Some(distance), None),
            Movement::Y => (None, Some(distance)),
            Movement::Along(vx, vy) => (
                (vx != 0).then(|| crate::fixed::mul_fix(distance, vx)),
                (vy != 0).then(|| crate::fixed::mul_fix(distance, vy)),
            ),
        };
        if let Some(dx) = dx {
            if bc == 0 {
                zone.cur[point].x = zone.cur[point].x.wrapping_add(dx);
            }
            zone.tags[point] |= TAG_TOUCH_X;
        }
        if let Some(dy) = dy {
            if bc != POST_IUP {
                zone.cur[point].y = zone.cur[point].y.wrapping_add(dy);
            }
            zone.tags[point] |= TAG_TOUCH_Y;
        }
    }

    /// Moves the original position of a point along the freedom vector
    /// (`Direct_Move_Orig*`).
    fn move_original(&mut self, zp: usize, point: usize, distance: i64) {
        let movement = self.funcs.movement;
        let zone = self.zone_mut(self.zp[zp]);
        let org = &mut zone.org[point];
        match movement {
            Movement::X => org.x = org.x.wrapping_add(distance),
            Movement::Y => org.y = org.y.wrapping_add(distance),
            Movement::Along(vx, vy) => {
                if vx != 0 {
                    org.x = org.x.wrapping_add(crate::fixed::mul_fix(distance, vx));
                }
                if vy != 0 {
                    org.y = org.y.wrapping_add(crate::fixed::mul_fix(distance, vy));
                }
            }
        }
    }

    fn dispatch(&mut self, opcode: u8) {
        match opcode {
            0x00..=0x05 => self.ins_sxytca(),
            0x06 | 0x07 => self.ins_spvtl(),
            0x08 | 0x09 => self.ins_sfvtl(),
            0x0A => self.ins_spvfs(),
            0x0B => self.ins_sfvfs(),
            0x0C => self.ins_gpv(),
            0x0D => self.ins_gfv(),
            0x0E => self.ins_sfvtpv(),
            0x0F => self.ins_isect(),
            0x10..=0x12 => self.ins_srp(),
            0x13..=0x16 => self.ins_szp(),
            0x17 => self.ins_sloop(),
            0x18 => self.rounding = Rounding::Grid,
            0x19 => self.rounding = Rounding::HalfGrid,
            0x1A => self.gs.minimum_distance = self.arg(0),
            0x1B => self.ins_else(),
            0x1C => self.ins_jmpr(),
            0x1D => self.gs.control_value_cutin = self.arg(0),
            0x1E => self.gs.single_width_cutin = self.arg(0),
            0x1F => {
                self.gs.single_width_value =
                    crate::fixed::mul_fix(self.arg(0), self.state.metrics.scale);
            }
            0x20..=0x26 | 0x8A => self.ins_stack(opcode),
            0x27 => self.ins_alignpts(),
            0x29 => self.ins_utp(),
            0x2A => self.ins_loopcall(),
            0x2B => self.ins_call(),
            0x2C => self.ins_fdef(),
            0x2D => self.ins_endf(),
            0x2E | 0x2F => self.ins_mdap(),
            0x30 | 0x31 => self.ins_iup(),
            0x32 | 0x33 => self.ins_shp(),
            0x34 | 0x35 => self.ins_shc(),
            0x36 | 0x37 => self.ins_shz(),
            0x38 => self.ins_shpix(),
            0x39 => self.ins_ip(),
            0x3A | 0x3B => self.ins_msirp(),
            0x3C => self.ins_alignrp(),
            0x3D => self.rounding = Rounding::DoubleGrid,
            0x3E | 0x3F => self.ins_miap(),
            0x40 | 0x41 | 0xB0..=0xBF => self.ins_push(opcode),
            0x42 => self.ins_ws(),
            0x43 => self.ins_rs(),
            0x44 => self.ins_wcvtp(),
            0x45 => self.ins_rcvt(),
            0x46 | 0x47 => self.ins_gc(),
            0x48 => self.ins_scfs(),
            0x49 | 0x4A => self.ins_md(),
            0x4B => self.set_arg(0, self.state.metrics.ppem),
            0x4C => self.set_arg(0, self.state.metrics.point_size),
            0x4D => self.gs.auto_flip = true,
            0x4E => self.gs.auto_flip = false,
            0x4F => self.fail(BytecodeError::DebugOpcode),
            0x50..=0x57 | 0x5A..=0x5C | 0x60..=0x6F | 0x8B | 0x8C => self.ins_arithmetic(opcode),
            0x58 => self.ins_if(),
            // EIF, SANGW and AA do nothing.
            0x59 | 0x7E | 0x7F => {}
            0x5D | 0x71 | 0x72 => self.ins_deltap(),
            0x5E => self.gs.delta_base = self.arg(0) as u16,
            0x5F => self.ins_sds(),
            0x70 => self.ins_wcvtf(),
            0x73..=0x75 => self.ins_deltac(),
            0x76 | 0x77 => self.ins_sround(),
            0x78 => self.ins_jrot(),
            0x79 => self.ins_jrof(),
            0x7A => self.rounding = Rounding::Off,
            0x7C => self.rounding = Rounding::UpToGrid,
            0x7D => self.rounding = Rounding::DownToGrid,
            0x80 => self.ins_flippt(),
            0x81 | 0x82 => self.ins_fliprg(),
            0x85 => self.ins_scanctrl(),
            0x86 | 0x87 => self.ins_sdpvtl(),
            0x88 => self.ins_getinfo(),
            0x89 => self.ins_idef(),
            0x8D => self.ins_scantype(),
            0x8E => self.ins_instctrl(),
            0xC0..=0xDF => self.ins_mdrp(),
            0xE0..=0xFF => self.ins_mirp(),
            // Undefined opcodes, including GETVARIATION and GETDATA,
            // which FreeType only knows for variation fonts.
            _ => self.ins_unknown(),
        }
    }
}

/// Arguments popped and results pushed by each opcode (FreeType's
/// `Pop_Push_Count`); the pushes and loops are handled by the opcodes.
const fn stack_effect(opcode: u8) -> (usize, usize) {
    match opcode {
        0x06..=0x0B
        | 0x27
        | 0x2A
        | 0x3A
        | 0x3B
        | 0x3E
        | 0x3F
        | 0x42
        | 0x44
        | 0x48
        | 0x70
        | 0x78
        | 0x79
        | 0x81
        | 0x82
        | 0x86
        | 0x87
        | 0x8E
        | 0xE0..=0xFF => (2, 0),
        0x23 => (2, 2),
        0x0C | 0x0D => (0, 2),
        0x0F => (5, 0),
        0x10..=0x17
        | 0x1A
        | 0x1C..=0x1F
        | 0x21
        | 0x26
        | 0x29
        | 0x2B
        | 0x2C
        | 0x2E
        | 0x2F
        | 0x34..=0x38
        | 0x4F
        | 0x58
        | 0x5D..=0x5F
        | 0x71..=0x77
        | 0x7E
        | 0x7F
        | 0x85
        | 0x89
        | 0x8D
        | 0xC0..=0xDF => (1, 0),
        0x20 => (1, 2),
        0x24 | 0x4B | 0x4C | 0x92 => (0, 1),
        0x25 | 0x43 | 0x45..=0x47 | 0x56 | 0x57 | 0x5C | 0x64..=0x6F | 0x88 => (1, 1),
        0x49 | 0x4A | 0x50..=0x55 | 0x5A | 0x5B | 0x60..=0x63 | 0x8B | 0x8C => (2, 1),
        0x8A => (3, 3),
        0xB0..=0xB7 => (0, (opcode - 0xB0) as usize + 1),
        0xB8..=0xBF => (0, (opcode - 0xB8) as usize + 1),
        _ => (0, 0),
    }
}

/// Bytes an opcode occupies, inline data included (`opcode_length`, with
/// the counts of `NPUSHB`/`NPUSHW` read from `code[ip + 1]`); `None` when
/// that count lies beyond the code.
fn opcode_length(code: &[u8], ip: usize) -> Option<i64> {
    let opcode = *code.get(ip)?;
    let count = || code.get(ip + 1).map(|&n| i64::from(n));
    match opcode {
        0x40 => count().map(|n| 2 + n),
        0x41 => count().map(|n| 2 + 2 * n),
        0xB0..=0xB7 => Some(i64::from(opcode - 0xB0) + 2),
        0xB8..=0xBF => Some(2 * i64::from(opcode - 0xB8) + 3),
        _ => Some(1),
    }
}
