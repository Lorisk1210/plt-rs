//! One face at one size, ready for glyph programs: FreeType's
//! `tt_size_init_bytecode`, `tt_size_run_fpgm`, `tt_size_run_prep` and the
//! size-dependent part of `tt_loader_init`.

use super::error::BytecodeProgram;
use super::interp::{
    BACKWARD_COMPATIBILITY, CodeRanges, DefinitionTable, Definitions, Exec, Metrics, Resources,
    State,
};
use super::state::GraphicsState;
use super::zone::{Vector, Zone};
use crate::error::{FontError, Result};
use crate::face::Face;
use crate::fixed::{div_fix, mul_div, mul_fix};
use crate::scale::Size;
use crate::types::Tag;

/// The work one program run, or one glyph load, may do (instructions
/// plus points visited) before it is stopped as too long. Real fonts stay
/// orders of magnitude below; hostile bytecode stops within a fraction of
/// a second.
pub(crate) const WORK_BUDGET: u64 = 100_000_000;

/// The state the font and control value programs leave for the glyph
/// programs.
#[derive(Clone, Debug)]
pub(crate) struct Prepared<'f> {
    font_program: Option<&'f [u8]>,
    control_value_program: Option<&'f [u8]>,
    pub(crate) definitions: DefinitionTable,
    /// Control values, storage and twilight zone as the control value
    /// program left them, with the backward compatibility bits glyph loads
    /// start with and a fresh work budget.
    pub(crate) resources: Resources,
    /// The graphics state glyph programs start from.
    pub(crate) graphics_state: GraphicsState,
    /// Whether glyph programs run (`INSTCTRL` can turn them off).
    pub(crate) instructions: bool,
    pub(crate) stack_size: usize,
}

impl<'f> Prepared<'f> {
    /// The code ranges for the glyph program `glyph`.
    pub(crate) const fn ranges<'g>(&self, glyph: &'g [u8]) -> CodeRanges<'g>
    where
        'f: 'g,
    {
        [self.font_program, self.control_value_program, Some(glyph)]
    }
}

/// The TrueType hinting state of one face at one size: the scaled control
/// values and the definitions and settings left by the font and control
/// value programs, ready to hint glyphs with
/// [`load_glyph`](Self::load_glyph).
///
/// Like FreeType's `FT_Size` after `FT_Set_Char_Size`, it hints at the
/// integer pixels per em when the font asks for it (`head` flags bit 3),
/// and every glyph starts from the state the control value program left,
/// as when matplotlib sets the size before rendering each glyph.
#[derive(Clone, Debug)]
pub struct HintingInstance<'f> {
    pub(crate) x_scale: i64,
    pub(crate) y_scale: i64,
    pub(crate) metrics: Metrics,
    pub(crate) num_glyphs: u16,
    /// Vertical pixels per em, which decide the scan-conversion precision.
    pub(crate) y_ppem: u16,
    pub(crate) prepared: Result<Prepared<'f>>,
}

impl<'f> HintingInstance<'f> {
    /// Runs `face`'s font and control value programs at `size`. Their
    /// failure is reported by every later [`load_glyph`](Self::load_glyph),
    /// as FreeType then refuses hinted glyphs.
    pub fn new(face: &Face<'f>, size: &Size) -> Self {
        let ppem = i64::from(size.x_ppem());
        let (x_scale, y_scale) = if face.head().flags & 8 != 0 {
            let scale = div_fix(ppem << 6, i64::from(face.units_per_em()));
            (scale, scale)
        } else {
            (size.x_scale(), size.y_scale())
        };
        let metrics = Metrics {
            ppem,
            point_size: mul_div(ppem, 64 * 72, i64::from(size.resolution())),
            scale: x_scale,
            orus_scale: x_scale,
        };
        let num_glyphs = face.num_glyphs();
        Self {
            x_scale,
            y_scale,
            metrics,
            num_glyphs,
            y_ppem: size.y_ppem(),
            prepared: prepare(face, metrics, num_glyphs),
        }
    }
}

/// A non-empty table's bytes.
fn program<'f>(face: &Face<'f>, tag: [u8; 4]) -> Option<&'f [u8]> {
    face.table(Tag(tag)).filter(|code| !code.is_empty())
}

/// Runs the font or control value program `which` with a fresh work
/// budget and returns the graphics state it leaves.
fn run(
    state: State<'_>,
    graphics_state: GraphicsState,
    ranges: CodeRanges<'_>,
    which: BytecodeProgram,
) -> Result<GraphicsState> {
    let mut empty = Zone::default();
    let mut exec = Exec::new(state, &mut empty, graphics_state, ranges, which, false);
    match exec.run() {
        Ok(()) => Ok(*exec.graphics_state()),
        Err(error) => Err(FontError::Bytecode {
            program: which,
            error,
        }),
    }
}

fn prepare<'f>(face: &Face<'f>, metrics: Metrics, num_glyphs: u16) -> Result<Prepared<'f>> {
    let maxp = face.max_profile();
    let font_program = program(face, *b"fpgm");
    let control_value_program = program(face, *b"prep");
    let max_stack = usize::from(maxp.max_stack_elements);
    let stack_size = max_stack + (max_stack / 2).max(128);
    let mut definitions = DefinitionTable::new(maxp.max_function_defs, maxp.max_instruction_defs);
    let unscaled: Vec<i64> = face
        .table(Tag::new(b"cvt "))
        .unwrap_or(&[])
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&value| i64::from(i16::from_be_bytes(value)))
        .collect();
    let twilight = Zone::twilight(usize::from(maxp.max_twilight_points) + 4);
    let mut resources = Resources {
        cvt: vec![0; unscaled.len()],
        storage: vec![0; usize::from(maxp.max_storage)],
        twilight_len: twilight.len(),
        twilight,
        backward_compatibility: 0,
        work: WORK_BUDGET,
    };
    if font_program.is_some() {
        let state = State {
            definitions: Definitions::Writable(&mut definitions),
            resources: &mut resources,
            metrics,
            num_glyphs,
            stack_size,
        };
        run(
            state,
            GraphicsState::default(),
            [font_program, None, None],
            BytecodeProgram::Font,
        )?;
    }
    // The control value program starts from the default state, zeroed
    // twilight points and storage, and the control values scaled to the
    // size.
    let twilight = &mut resources.twilight;
    twilight.org.fill(Vector::default());
    twilight.cur.fill(Vector::default());
    resources.twilight_len = twilight.len();
    resources.storage.fill(0);
    for (scaled, &value) in resources.cvt.iter_mut().zip(&unscaled) {
        *scaled = mul_fix(value, metrics.scale);
    }
    resources.work = WORK_BUDGET;
    let mut graphics_state = GraphicsState::default();
    if control_value_program.is_some() {
        let state = State {
            definitions: Definitions::Writable(&mut definitions),
            resources: &mut resources,
            metrics,
            num_glyphs,
            stack_size,
        };
        graphics_state = run(
            state,
            graphics_state,
            [font_program, control_value_program, None],
            BytecodeProgram::ControlValue,
        )?
        .saved();
    }
    let instructions = graphics_state.instruct_control & 1 == 0;
    if graphics_state.instruct_control & 2 != 0 {
        graphics_state = GraphicsState::default();
    }
    resources.twilight_len = resources.twilight.len();
    resources.backward_compatibility =
        (graphics_state.instruct_control & 4) ^ BACKWARD_COMPATIBILITY;
    resources.work = WORK_BUDGET;
    Ok(Prepared {
        font_program,
        control_value_program,
        definitions,
        resources,
        graphics_state,
        instructions,
        stack_size,
    })
}
