//! Hinted glyph loading: synthetic fonts exercising the loader's paths,
//! FreeType's results for bundled fonts, which glyphs get bytecode, and
//! corrupted fonts.

use super::*;
use crate::outline::OutlineRecorder;
use crate::scale::Size;
use crate::tables::glyf::tests::{composite, instructed_glyph, simple_glyph, tables};
use crate::types::GlyphId;
use crate::{FontError, PathCommand};

const DEJAVU: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
const STIX: &[u8] = include_bytes!("../../fonts/STIXGeneral.ttf");
const CFF: &[u8] = include_bytes!("../../tests/fixtures/synthetic-cff.otf");

const ARGS_ARE_XY_VALUES: u16 = 0x0002;
const ROUND_XY_TO_GRID: u16 = 0x0004;

/// A TrueType font: glyph records, advances and the hinting tables.
pub(crate) struct TestFont {
    glyphs: Vec<Vec<u8>>,
    advance: u16,
    fpgm: Vec<u8>,
    prep: Vec<u8>,
    cvt: Vec<i16>,
}

impl TestFont {
    /// A font of `glyphs` without hinting programs.
    pub(crate) fn new(glyphs: Vec<Vec<u8>>) -> Self {
        Self {
            glyphs,
            advance: 500,
            fpgm: Vec::new(),
            prep: Vec::new(),
            cvt: Vec::new(),
        }
    }

    /// The sfnt file: 1000 units per em, integer ppem scaling.
    pub(crate) fn build(&self) -> Vec<u8> {
        let be16 = |values: &[i64]| -> Vec<u8> {
            values
                .iter()
                .flat_map(|&v| (v as u16).to_be_bytes())
                .collect()
        };
        let count = self.glyphs.len() as i64;
        let mut head = be16(&[1, 0, 0, 0, 0, 0, 0x5F0F, 0x3CF5, 0x0008, 1000]);
        head.extend([0; 16]);
        head.extend(be16(&[0, 0, 1000, 1000, 0, 8, 2, 1, 0]));
        let mut hhea = be16(&[1, 0, 800, -200, 0, i64::from(self.advance)]);
        hhea.extend([0; 22]);
        hhea.extend(be16(&[count]));
        let maxp = be16(&[1, 0, count, 0, 0, 0, 0, 2, 4, 8, 16, 4, 256, 512, 4, 4]);
        // Left side bearings equal to the glyphs' x_min put the origin at
        // x = 0.
        let hmtx = be16(
            &self
                .glyphs
                .iter()
                .flat_map(|glyph| {
                    let x_min = glyph
                        .get(2..4)
                        .map_or(0, |b| i16::from_be_bytes([b[0], b[1]]));
                    [i64::from(self.advance), i64::from(x_min)]
                })
                .collect::<Vec<_>>(),
        );
        let (loca, glyf) = tables(&self.glyphs);
        let cvt = be16(&self.cvt.iter().map(|&v| i64::from(v)).collect::<Vec<_>>());
        let entries: Vec<(&[u8; 4], &[u8])> = vec![
            (b"head", &head),
            (b"hhea", &hhea),
            (b"maxp", &maxp),
            (b"hmtx", &hmtx),
            (b"loca", &loca),
            (b"glyf", &glyf),
            (b"fpgm", &self.fpgm),
            (b"prep", &self.prep),
            (b"cvt ", &cvt),
        ];
        let mut font = be16(&[1, 0, entries.len() as i64, 0, 0, 0]);
        let mut offset = 12 + 16 * entries.len();
        let mut data = Vec::new();
        for (tag, table) in &entries {
            font.extend_from_slice(*tag);
            font.extend([0; 4]);
            font.extend((offset as u32).to_be_bytes());
            font.extend((table.len() as u32).to_be_bytes());
            data.extend_from_slice(table);
            offset += table.len();
        }
        font.extend(data);
        font
    }
}

/// A unit square of 100 units with its corner at (`x`, `y`).
pub(crate) fn square(x: i16, y: i16, instructions: &[u8]) -> Vec<u8> {
    instructed_glyph(
        &[&[
            (x, y, true),
            (x + 100, y, true),
            (x + 100, y + 100, true),
            (x, y + 100, true),
        ]],
        instructions,
    )
}

/// Loads `glyph` hinted at 10 pixels per em (`scale` 0.64 in 26.6).
fn load(font: &[u8], glyph: u16) -> Result<HintedGlyph, FontError> {
    let face = Face::parse(font, 0)?;
    let size = Size::new(&face, 10.0, 72.0);
    HintingInstance::new(&face, &size).load_glyph(&face, GlyphId(glyph))
}

/// The on-curve points of a hinted glyph, in 26.6.
pub(crate) fn points(glyph: &HintedGlyph) -> Vec<(i32, i32)> {
    let mut recorder = OutlineRecorder::default();
    glyph.outline(&mut recorder);
    recorder
        .commands
        .iter()
        .filter_map(|command| match *command {
            PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => {
                Some(((x * 64.0) as i32, (y * 64.0) as i32))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn unhinted_points_at_the_integer_ppem_with_a_rounded_advance() {
    let font = TestFont::new(vec![square(15, 15, &[])]).build();
    let glyph = load(&font, 0).expect("valid font");
    // 15 units at 10 ppem / 1000 units = 9.6 / 64 px.
    assert_eq!(
        points(&glyph)[..4],
        [(10, 10), (74, 10), (74, 74), (10, 74)]
    );
    // 500 units = 5 px exactly; a 499-unit advance (4.99 px) rounds too.
    assert_eq!(glyph.advance(), 320);
    let narrow = TestFont {
        advance: 499,
        ..TestFont::new(vec![square(15, 15, &[])])
    };
    assert_eq!(load(&narrow.build(), 0).expect("valid font").advance(), 320);
    assert_eq!(glyph.control_box().map(|r| r.x_min), Some(10));
}

#[test]
fn glyph_programs_round_y_only_in_backward_compatibility_mode() {
    // SVTCA[x]; MDAP[1] 0; SVTCA[y]; MDAP[1] 0; IUP[y]; IUP[x].
    let program = [0x01, 0xB0, 0, 0x2F, 0x00, 0xB0, 0, 0x2F, 0x30, 0x31];
    let font = TestFont::new(vec![square(50, 50, &program)]);
    let glyph = load(&font.build(), 0).expect("valid font");
    // y 32 rounds up to 64 and IUP shifts the untouched points with it.
    assert_eq!(
        points(&glyph)[..4],
        [(32, 64), (96, 64), (96, 128), (32, 128)]
    );
    // A native ClearType font (INSTCTRL 3 4 in prep) rounds x as well.
    let native = TestFont {
        prep: vec![0xB1, 4, 3, 0x8E],
        ..TestFont::new(vec![square(50, 50, &program)])
    };
    let glyph = load(&native.build(), 0).expect("valid font");
    assert_eq!(
        points(&glyph)[..4],
        [(64, 64), (128, 64), (128, 128), (64, 128)]
    );
}

#[test]
fn the_control_value_program_can_switch_hinting_off() {
    let program = [0x00, 0xB0, 0, 0x2F, 0x30];
    // INSTCTRL 1 1: no glyph programs.
    let font = TestFont {
        prep: vec![0xB1, 1, 1, 0x8E],
        ..TestFont::new(vec![square(50, 50, &program)])
    };
    let glyph = load(&font.build(), 0).expect("valid font");
    assert_eq!(points(&glyph)[0], (32, 32));
}

#[test]
fn functions_and_control_values_reach_glyph_programs() {
    // fpgm: FDEF 0 { SVTCA[y]; MIAP[1] point 0 to cvt 0 } ENDF.
    // glyph: PUSHB 0; CALL.
    let font = TestFont {
        fpgm: vec![0xB0, 0, 0x2C, 0x00, 0xB1, 0, 0, 0x3F, 0x2D],
        cvt: vec![140],
        ..TestFont::new(vec![square(50, 50, &[0xB0, 0, 0x2B])])
    };
    let glyph = load(&font.build(), 0).expect("valid font");
    // cvt 140 units = 89.6 / 64 px, more than the cut-in away from the
    // point's 32: the point keeps its own position, rounded to 64.
    assert_eq!(points(&glyph)[0], (32, 64));
}

#[test]
fn failing_programs() {
    let broken_fpgm = TestFont {
        fpgm: vec![0x2D],
        ..TestFont::new(vec![square(0, 0, &[])])
    };
    assert_eq!(
        load(&broken_fpgm.build(), 0),
        Err(FontError::Bytecode {
            program: BytecodeProgram::Font,
            error: BytecodeError::EndfOutsideFunction,
        })
    );
    let broken_prep = TestFont {
        prep: vec![0x4F],
        ..TestFont::new(vec![square(0, 0, &[])])
    };
    assert_eq!(
        load(&broken_prep.build(), 0),
        Err(FontError::Bytecode {
            program: BytecodeProgram::ControlValue,
            error: BytecodeError::DebugOpcode,
        })
    );
    // A failing glyph program keeps the outline.
    let broken_glyph = TestFont::new(vec![square(50, 50, &[0x00, 0xB0, 0, 0x2F, 0x4F])]);
    let glyph = load(&broken_glyph.build(), 0).expect("glyph errors are ignored");
    assert_eq!(points(&glyph)[0], (32, 64));
}

#[test]
fn composites_round_offsets_and_run_their_own_program() {
    // Glyph 1 places the square 50 units up and right, rounded to the
    // grid: only y in backward compatibility mode.
    let rounded = composite(&[(ARGS_ARE_XY_VALUES | ROUND_XY_TO_GRID, 0, &[50, 50])]);
    let font = TestFont::new(vec![square(0, 0, &[]), rounded]);
    let glyph = load(&font.build(), 1).expect("valid font");
    assert_eq!(points(&glyph)[0], (32, 64));
    // With instructions (SVTCA[y]; MDAP[1] 2) after the component.
    let mut instructed = composite(&[(ARGS_ARE_XY_VALUES | 0x0100, 0, &[0, 40])]);
    instructed.extend([0, 4, 0x00, 0xB0, 2, 0x2F]);
    let font = TestFont::new(vec![square(0, 0, &[]), instructed]);
    let glyph = load(&font.build(), 1).expect("valid font");
    // Point 2 sits at y = 140 units = 89.6/64 px and rounds to 64.
    assert_eq!(points(&glyph)[2], (64, 64));
    // Unknown components and anchors are errors.
    let missing = TestFont::new(vec![composite(&[(ARGS_ARE_XY_VALUES, 7, &[0, 0])])]);
    assert!(load(&missing.build(), 0).is_err());
    let anchored = TestFont::new(vec![
        simple_glyph(&[&[(0, 0, true)]]),
        composite(&[(0, 0, &[3, 0])]),
    ]);
    assert!(load(&anchored.build(), 1).is_err());
}

#[test]
fn bundled_glyphs_match_freetype() {
    // FreeType 2.14.3 (matplotlib's `FT2Font.load_glyph(36,
    // LoadFlags.DEFAULT)` and `get_path()`) for DejaVu Sans `A` at 10 pt
    // and 100 dpi.
    let face = Face::parse(DEJAVU, 0).expect("bundled font");
    let size = Size::new(&face, 10.0, 100.0);
    let glyph = HintingInstance::new(&face, &size)
        .load_glyph(&face, GlyphId(36))
        .expect("bundled font");
    assert_eq!(glyph.advance(), 640);
    assert_eq!(
        points(&glyph),
        [
            (306, 559),
            (186, 256),
            (427, 256),
            (306, 559),
            (256, 640),
            (357, 640),
            (606, 0),
            (514, 0),
            (454, 192),
            (160, 192),
            (100, 0),
            (7, 0),
            (256, 640),
        ]
    );
}

#[test]
fn which_hinter_glyphs_get() {
    let dejavu = Face::parse(DEJAVU, 0).expect("bundled font");
    let stix = Face::parse(STIX, 0).expect("bundled font");
    let cff = Face::parse(CFF, 0).expect("fixture");
    for aligned in [true, false] {
        for face in [&dejavu, &stix, &cff] {
            assert_eq!(Hinting::None.hinter(face, aligned), Hinter::Unhinted);
        }
        assert_eq!(Hinting::Default.hinter(&dejavu, aligned), Hinter::Bytecode);
        assert_eq!(Hinting::Native.hinter(&dejavu, aligned), Hinter::Bytecode);
        assert_eq!(Hinting::Native.hinter(&stix, aligned), Hinter::Bytecode);
        assert_eq!(
            Hinting::Native.hinter(&cff, aligned),
            Hinter::Unhinted,
            "no CFF hinter"
        );
        assert_eq!(
            Hinting::Default.hinter(&cff, aligned),
            Hinter::Unhinted,
            "no CFF hinter"
        );
    }
    // Without bytecode, or forced, axis-aligned glyphs go to the
    // autohinter and others to the native hinter.
    assert_eq!(Hinting::Default.hinter(&stix, true), Hinter::Autohinter);
    assert_eq!(Hinting::Default.hinter(&stix, false), Hinter::Bytecode);
    for face in [&dejavu, &stix, &cff] {
        assert_eq!(Hinting::Auto.hinter(face, true), Hinter::Autohinter);
    }
    assert_eq!(Hinting::Auto.hinter(&dejavu, false), Hinter::Bytecode);
    assert_eq!(Hinting::Auto.hinter(&cff, false), Hinter::Unhinted);
}

/// `SplitMix64`, for reproducible corruption.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// The byte range of table `tag` in `font`.
fn table_range(font: &[u8], tag: &[u8; 4]) -> std::ops::Range<usize> {
    let face = Face::parse(font, 0).expect("bundled font");
    let table = face.table(crate::types::Tag::new(tag)).expect("table");
    let start = table.as_ptr() as usize - font.as_ptr() as usize;
    start..start + table.len()
}

#[test]
fn corrupted_bytecode_never_panics_or_runs_away() {
    let mut random = Random(0xF0_57);
    let targets = [b"fpgm", b"prep", b"glyf", b"cvt ", b"maxp"].map(|tag| table_range(DEJAVU, tag));
    for round in 0..200 {
        let mut font = DEJAVU.to_vec();
        for _ in 0..=round % 8 {
            let range = &targets[(random.next() % targets.len() as u64) as usize];
            let at = range.start + (random.next() % range.len() as u64) as usize;
            font[at] = random.next() as u8;
        }
        let Ok(face) = Face::parse(&font, 0) else {
            continue;
        };
        let size = Size::new(&face, 6.0 + f64::from(round % 20), 100.0);
        let instance = HintingInstance::new(&face, &size);
        for glyph in (0..face.num_glyphs()).step_by(97).take(30) {
            if let Ok(hinted) = instance.load_glyph(&face, GlyphId(glyph)) {
                let _ = (hinted.advance(), hinted.control_box());
            }
        }
    }
}
