//! Interpreter tests: opcode semantics, version 40 restrictions, limits,
//! and random bytecode that must neither panic nor run away.

use super::*;
use crate::fixed::mul_fix;
use crate::hinting::state::UnitVector;
use crate::hinting::zone::TAG_ON;

const METRICS: Metrics = Metrics {
    ppem: 12,
    point_size: 12 * 64,
    scale: 0x6000,
    orus_scale: 0x6000,
};

/// A small machine: a few control values, storage slots and twilight
/// points.
struct Machine {
    definitions: DefinitionTable,
    resources: Resources,
}

impl Machine {
    fn new() -> Self {
        Self {
            definitions: DefinitionTable::new(64, 4),
            resources: Resources {
                cvt: vec![0, 100, 200, -50],
                storage: vec![0; 8],
                twilight: Zone::twilight(6),
                twilight_len: 6,
                backward_compatibility: 0,
                work: 10_000_000,
            },
        }
    }

    /// Runs `code` as `program` over `zone`; `functions` is the font
    /// program glyph programs may call. Returns the result and the stack.
    fn run_with(
        &mut self,
        program: BytecodeProgram,
        code: &[u8],
        functions: &[u8],
        zone: &mut Zone,
    ) -> (Result<(), BytecodeError>, Vec<i64>) {
        let font = (!functions.is_empty()).then_some(functions);
        let (definitions, ranges): (Definitions<'_>, CodeRanges<'_>) = match program {
            BytecodeProgram::Font => (
                Definitions::Writable(&mut self.definitions),
                [Some(code), None, None],
            ),
            BytecodeProgram::ControlValue => (
                Definitions::Writable(&mut self.definitions),
                [font, Some(code), None],
            ),
            BytecodeProgram::Glyph => (
                Definitions::ReadOnly(&self.definitions),
                [font, None, Some(code)],
            ),
        };
        let state = State {
            definitions,
            resources: &mut self.resources,
            metrics: METRICS,
            num_glyphs: 10,
            stack_size: 64,
        };
        let mut exec = Exec::new(
            state,
            zone,
            GraphicsState::default(),
            ranges,
            program,
            false,
        );
        let result = exec.run();
        (result, exec.stack[..exec.top].to_vec())
    }

    fn run(
        &mut self,
        program: BytecodeProgram,
        code: &[u8],
    ) -> (Result<(), BytecodeError>, Vec<i64>) {
        self.run_with(program, code, &[], &mut Zone::default())
    }
}

/// A glyph zone: a triangle contour (points 0–2) plus four phantom points,
/// with font-unit and current positions equal to `points` (26.6).
fn glyph_zone(points: &[(i64, i64)]) -> Zone {
    let cur: Vec<Vector> = points
        .iter()
        .map(|&(x, y)| Vector::new(x, y))
        .chain([Vector::default(); 4])
        .collect();
    Zone {
        org: cur.clone(),
        orus: cur.clone(),
        tags: vec![TAG_ON; cur.len()],
        contours: vec![points.len() - 1],
        cur,
    }
}

#[test]
fn pushes_and_arithmetic() {
    let mut machine = Machine::new();
    // PUSHB 10 20, ADD; NPUSHW -2 64; MUL; PUSHB 3; NEG; ABS; MAX.
    let code = [
        0xB1, 10, 20, 0x60, 0x41, 2, 0xFF, 0xFE, 0x00, 0x40, 0x63, 0xB0, 3, 0x65, 0x64, 0x8B,
    ];
    let (result, stack) = machine.run(BytecodeProgram::Font, &code);
    assert_eq!(result, Ok(()));
    assert_eq!(stack, vec![30, 3]);
    // DIV, FLOOR, CEILING, comparisons and logic.
    let code = [
        0xB1, 128, 64, 0x62, 0xB0, 100, 0x66, 0xB0, 100, 0x67, 0xB1, 1, 2, 0x50, 0xB1, 0, 7, 0x5B,
        0xB0, 0, 0x5C,
    ];
    let (result, stack) = machine.run(BytecodeProgram::Font, &code);
    assert_eq!(result, Ok(()));
    assert_eq!(stack, vec![128, 64, 128, 1, 1, 1]);
    let (result, _) = machine.run(BytecodeProgram::Font, &[0xB1, 1, 0, 0x62]);
    assert_eq!(result, Err(BytecodeError::DivideByZero));
}

#[test]
fn stack_manipulation() {
    let mut machine = Machine::new();
    // PUSHB 1 2 3; DUP; DEPTH; CINDEX 2 (the DUP); MINDEX 5; ROLL; SWAP.
    let code = [
        0xB2, 1, 2, 3, 0x20, 0x24, 0xB0, 2, 0x25, 0xB0, 5, 0x26, 0x8A, 0x23,
    ];
    let (result, stack) = machine.run(BytecodeProgram::Font, &code);
    assert_eq!(result, Ok(()));
    assert_eq!(stack, vec![1, 3, 3, 3, 4, 2]);
    let (_, stack) = machine.run(BytecodeProgram::Font, &[0xB2, 1, 2, 3, 0x22, 0xB0, 9]);
    assert_eq!(stack, vec![9], "CLEAR empties the stack");
    // Missing arguments read as zero.
    let (result, stack) = machine.run(BytecodeProgram::Font, &[0x60]);
    assert_eq!((result, stack), (Ok(()), vec![0]));
}

#[test]
fn conditionals_and_jumps() {
    let mut machine = Machine::new();
    // PUSHB 0; IF; PUSHB 1; ELSE; PUSHB 2; EIF.
    let code = [0xB0, 0, 0x58, 0xB0, 1, 0x1B, 0xB0, 2, 0x59];
    assert_eq!(machine.run(BytecodeProgram::Font, &code).1, vec![2]);
    // PUSHB 1; IF; PUSHB 1; IF; PUSHB 7; EIF; ELSE; PUSHB 2; EIF.
    let code = [
        0xB0, 1, 0x58, 0xB0, 1, 0x58, 0xB0, 7, 0x59, 0x1B, 0xB0, 2, 0x59,
    ];
    assert_eq!(machine.run(BytecodeProgram::Font, &code).1, vec![7]);
    // PUSHB 3 1; JROT over PUSHB 9; PUSHB 5.
    let code = [0xB1, 3, 1, 0x78, 0xB0, 9, 0xB0, 5];
    assert_eq!(machine.run(BytecodeProgram::Font, &code).1, vec![5]);
    // A backward jump forever: the loop budget stops it.
    let code = [0xB8, 0xFF, 0xFD, 0x1C];
    let (result, _) = machine.run(BytecodeProgram::Font, &code);
    assert_eq!(result, Err(BytecodeError::ExecutionTooLong));
    // An IF without EIF runs off the code.
    let (result, _) = machine.run(BytecodeProgram::Font, &[0xB0, 0, 0x58, 0xB0, 1]);
    assert_eq!(result, Err(BytecodeError::CodeOverflow));
}

#[test]
fn functions_and_calls() {
    let mut machine = Machine::new();
    // FDEF 0 { PUSHB 7 } ENDF; PUSHB 3 0; LOOPCALL; PUSHB 0; CALL.
    let fpgm = [
        0xB0, 0, 0x2C, 0xB0, 7, 0x2D, 0xB1, 3, 0, 0x2A, 0xB0, 0, 0x2B,
    ];
    let (result, stack) = machine.run(BytecodeProgram::Font, &fpgm);
    assert_eq!(result, Ok(()));
    assert_eq!(stack, vec![7, 7, 7, 7]);
    // Glyph programs call the font program's functions but define none.
    let mut zone = glyph_zone(&[(0, 0), (64, 0), (0, 64)]);
    let (result, stack) =
        machine.run_with(BytecodeProgram::Glyph, &[0xB0, 0, 0x2B], &fpgm, &mut zone);
    assert_eq!((result, stack), (Ok(()), vec![7]));
    let (result, _) = machine.run_with(
        BytecodeProgram::Glyph,
        &[0xB0, 1, 0x2C, 0x2D],
        &fpgm,
        &mut zone,
    );
    assert_eq!(result, Err(BytecodeError::DefinitionInGlyphProgram));
    let (result, _) = machine.run(BytecodeProgram::Font, &[0xB0, 9, 0x2B]);
    assert_eq!(result, Err(BytecodeError::InvalidReference));
    let (result, _) = machine.run(BytecodeProgram::Font, &[0x2D]);
    assert_eq!(result, Err(BytecodeError::EndfOutsideFunction));
    // FDEF 1 { PUSHB 1; CALL } ENDF; PUSHB 1; CALL: endless recursion.
    let (result, _) = machine.run(
        BytecodeProgram::Font,
        &[0xB0, 1, 0x2C, 0xB0, 1, 0x2B, 0x2D, 0xB0, 1, 0x2B],
    );
    assert_eq!(result, Err(BytecodeError::StackOverflow));
    let (result, _) = machine.run(BytecodeProgram::Font, &[0xB0, 1, 0x2C, 0xB0, 1, 0x2C]);
    assert_eq!(result, Err(BytecodeError::NestedDefinitions));
}

#[test]
fn instruction_definitions_extend_the_opcodes() {
    let mut machine = Machine::new();
    // IDEF 0x91 { PUSHB 42 } ENDF; then opcode 0x91.
    let code = [0xB0, 0x91, 0x89, 0xB0, 42, 0x2D, 0x91];
    assert_eq!(machine.run(BytecodeProgram::Font, &code).1, vec![42]);
    let (result, _) = machine.run(BytecodeProgram::Font, &[0x93]);
    assert_eq!(result, Err(BytecodeError::InvalidOpcode(0x93)));
}

#[test]
fn limits_are_enforced() {
    let mut machine = Machine::new();
    let mut overflow = vec![0x40, 200];
    overflow.extend([1; 200]);
    let (result, _) = machine.run(BytecodeProgram::Font, &overflow);
    assert_eq!(result, Err(BytecodeError::StackOverflow));
    let (result, _) = machine.run(BytecodeProgram::Font, &[0xB2, 1, 2]);
    assert_eq!(result, Err(BytecodeError::CodeOverflow));
    let (result, _) = machine.run(BytecodeProgram::Font, &[0x4F]);
    assert_eq!(result, Err(BytecodeError::DebugOpcode));
    let (result, _) = machine.run(BytecodeProgram::Font, &[0xB0, 7, 0x5F]);
    assert_eq!(result, Err(BytecodeError::BadArgument), "SDS above 6");
    machine.resources.work = 20;
    let (result, _) = machine.run(BytecodeProgram::Font, &[0xB0; 80]);
    assert_eq!(result, Err(BytecodeError::ExecutionTooLong), "work budget");
}

#[test]
fn storage_and_control_values() {
    let mut machine = Machine::new();
    // WS 3 <- 77; RS 3; RS 99 (out of bounds); RCVT 1; WCVTP 2 <- 5;
    // RCVT 2; WCVTF 3 <- 64 units; RCVT 3.
    let code = [
        0xB1, 3, 77, 0x42, 0xB0, 3, 0x43, 0xB0, 99, 0x43, 0xB0, 1, 0x45, 0xB1, 2, 5, 0x44, 0xB0, 2,
        0x45, 0xB1, 3, 64, 0x70, 0xB0, 3, 0x45,
    ];
    let (result, stack) = machine.run(BytecodeProgram::ControlValue, &code);
    assert_eq!(result, Ok(()));
    assert_eq!(stack, vec![77, 0, 100, 5, 24]);
}

#[test]
fn graphics_state_queries() {
    let mut machine = Machine::new();
    // GETINFO 1 | 64 | 1024 | 2048 | 4096 (pushed as a word); MPPEM; MPS;
    // SVTCA[y]; GPV.
    let code = [0xB8, 0x1C, 0x41, 0x88, 0x4B, 0x4C, 0x00, 0x0C];
    let (result, stack) = machine.run(BytecodeProgram::Font, &code);
    assert_eq!(result, Ok(()));
    assert_eq!(
        stack,
        vec![
            0x28 | 1 << 13 | 1 << 17 | 1 << 18 | 1 << 19,
            12,
            12 * 64,
            0,
            0x4000
        ]
    );
    // SPVFS (3, 4) normalises to (0.6, 0.8) in 2.14.
    let (_, stack) = machine.run(BytecodeProgram::Font, &[0xB1, 3, 4, 0x0A, 0x0C]);
    assert!(
        (stack[0] - 9830).abs() <= 1 && (stack[1] - 13107).abs() <= 1,
        "{stack:?}"
    );
}

#[test]
fn backward_compatibility_blocks_x_moves() {
    let mut machine = Machine::new();
    // SVTCA[x]; MDAP[1] 0; SVTCA[y]; MDAP[1] 0.
    let code = [0x01, 0xB0, 0, 0x2F, 0x00, 0xB0, 0, 0x2F];
    let mut zone = glyph_zone(&[(70, 90), (200, 0), (0, 200)]);
    machine.resources.backward_compatibility = BACKWARD_COMPATIBILITY;
    assert_eq!(
        machine
            .run_with(BytecodeProgram::Glyph, &code, &[], &mut zone)
            .0,
        Ok(())
    );
    assert_eq!(zone.cur[0], Vector::new(70, 64), "only y is rounded");
    assert_eq!(zone.tags[0] & 0x18, 0x18, "both axes are touched");
    let mut zone = glyph_zone(&[(70, 90), (200, 0), (0, 200)]);
    machine.resources.backward_compatibility = 0;
    assert_eq!(
        machine
            .run_with(BytecodeProgram::Glyph, &code, &[], &mut zone)
            .0,
        Ok(())
    );
    assert_eq!(
        zone.cur[0],
        Vector::new(64, 64),
        "native ClearType rounds both"
    );
}

#[test]
fn interpolation_and_the_post_iup_freeze() {
    let mut machine = Machine::new();
    machine.resources.backward_compatibility = BACKWARD_COMPATIBILITY;
    // SVTCA[y]; MDAP[1] 1 (y 100 -> 128); MDAP[0] 2; IUP[y]; IUP[x];
    // MDAP[1] 0: frozen after both IUPs.
    let code = [
        0x00, 0xB0, 1, 0x2F, 0xB0, 2, 0x2E, 0x30, 0x31, 0xB0, 0, 0x2F,
    ];
    let mut zone = glyph_zone(&[(0, 50), (0, 100), (0, 0)]);
    assert_eq!(
        machine
            .run_with(BytecodeProgram::Glyph, &code, &[], &mut zone)
            .0,
        Ok(())
    );
    assert_eq!(zone.cur[1].y, 128);
    assert_eq!(zone.cur[0].y, 64, "interpolated between 0 and 128");
    assert_eq!(machine.resources.backward_compatibility, POST_IUP);
}

#[test]
fn deltas_apply_at_their_size_only_to_touched_points() {
    let mut machine = Machine::new();
    machine.resources.backward_compatibility = BACKWARD_COMPATIBILITY;
    // SVTCA[y]; DELTAP1 for point 1 at ppem 9 + 3 = 12, +8 steps of 1/8:
    // ignored as the point is untouched; then MDAP[0] 1 and again.
    let delta = [0xB2, 0x3F, 1, 1, 0x5D];
    let mut code = vec![0x00];
    code.extend(delta);
    code.extend([0xB0, 1, 0x2E]);
    code.extend(delta);
    let mut zone = glyph_zone(&[(0, 0), (0, 100), (0, 200)]);
    assert_eq!(
        machine
            .run_with(BytecodeProgram::Glyph, &code, &[], &mut zone)
            .0,
        Ok(())
    );
    assert_eq!(zone.cur[1].y, 100 + 64);
}

#[test]
fn instruction_control() {
    let mut machine = Machine::new();
    let mut zone = Zone::default();
    let state = State {
        definitions: Definitions::Writable(&mut machine.definitions),
        resources: &mut machine.resources,
        metrics: METRICS,
        num_glyphs: 10,
        stack_size: 64,
    };
    // PUSHB 4 3; INSTCTRL: native ClearType from the control value program.
    let code = [0xB1, 4, 3, 0x8E];
    let ranges = [None, Some(&code[..]), None];
    let gs = GraphicsState::default();
    let mut exec = Exec::new(
        state,
        &mut zone,
        gs,
        ranges,
        BytecodeProgram::ControlValue,
        false,
    );
    assert_eq!(exec.run(), Ok(()));
    assert_eq!(exec.graphics_state().instruct_control, 4);
    // In a glyph program the same toggles backward compatibility.
    machine.resources.backward_compatibility = BACKWARD_COMPATIBILITY;
    let mut zone = glyph_zone(&[(0, 0), (1, 1), (2, 2)]);
    assert_eq!(
        machine
            .run_with(BytecodeProgram::Glyph, &code, &[], &mut zone)
            .0,
        Ok(())
    );
    assert_eq!(machine.resources.backward_compatibility, 0);
}

#[test]
fn vectors_and_measurements() {
    let mut machine = Machine::new();
    let mut zone = glyph_zone(&[(0, 0), (128, 128), (0, 256)]);
    // SPVTL[0] from point 0 to point 1 (diagonal); GPV; the current
    // distance between them (opcode 0x49) and the original one (0x4A, in
    // font units times the scale); SFVTPV; GFV.
    let code = [
        0xB1, 1, 0, 0x06, 0x0C, 0xB1, 1, 0, 0x49, 0xB1, 1, 0, 0x4A, 0x0E, 0x0D,
    ];
    let (result, stack) = machine.run_with(BytecodeProgram::Glyph, &code, &[], &mut zone);
    assert_eq!(result, Ok(()));
    let diagonal = i64::from(
        UnitVector {
            x: 0x2D41,
            y: 0x2D41,
        }
        .x,
    );
    assert!(
        (stack[0] - diagonal).abs() <= 1 && stack[0] == stack[1],
        "{stack:?}"
    );
    assert!((stack[2] - 181).abs() <= 1, "128 * sqrt 2 = {}", stack[2]);
    assert_eq!(stack[3], mul_fix(stack[2], METRICS.orus_scale));
    assert_eq!((stack[4], stack[5]), (stack[0], stack[1]));
}

/// `SplitMix64`, for reproducible random bytecode.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Random bytecode biased towards small pushes so that arguments are
    /// often valid point and function numbers.
    fn program(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| match self.next() % 4 {
                0 => 0xB0 + (self.next() % 2) as u8,
                1 => (self.next() % 8) as u8,
                _ => self.next() as u8,
            })
            .collect()
    }
}

#[test]
fn random_bytecode_never_panics_or_runs_away() {
    let mut random = Random(0x5EED);
    for round in 0..3000 {
        let mut machine = Machine::new();
        machine.resources.work = 2_000_000;
        let lengths = [random.next() % 64, random.next() % 64, random.next() % 96];
        let [fpgm, prep, glyph] = lengths.map(|len| random.program(len as usize + 1));
        let _ = machine.run(BytecodeProgram::Font, &fpgm);
        let _ = machine.run_with(
            BytecodeProgram::ControlValue,
            &prep,
            &fpgm,
            &mut Zone::default(),
        );
        machine.resources.backward_compatibility = if round % 2 == 0 {
            BACKWARD_COMPATIBILITY
        } else {
            0
        };
        let mut zone = glyph_zone(&[(0, 0), (300, 40), (150, 400)]);
        let _ = machine.run_with(BytecodeProgram::Glyph, &glyph, &fpgm, &mut zone);
        assert_eq!(zone.len(), 7, "zones keep their size");
        assert!(machine.resources.twilight_len <= machine.resources.twilight.len());
    }
}
