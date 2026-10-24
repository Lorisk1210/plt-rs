//! Font files: TrueType, OpenType (CFF) and collection files with bits
//! flipped, spans repeated or deleted, tables lying about their contents
//! and tails cut, through `plt_font` directly (parsing, outlines,
//! hinting, shaping, subsetting) and through figures drawing text in
//! them to every format; and corrupted AFM files.

use std::path::{Path, PathBuf};

use plt::figure::Figure;
use plt::fonts::{FontManager, FontProperties};
use plt::rc::RcParams;
use plt_font::{
    AfmMetrics, AutohintTarget, Autohinter, Face, FontChain, GlyphId, Hinting, HintingInstance,
    OutlineRecorder, ShapeOptions, Size, face_count, shape, subset,
};
use plt_text::{FontDatabase, FontQuery, FontRequest, TextEngine, bundled_font_data};

use crate::harness::{Corruption, Gen, fuzz};
use crate::support::{FORMATS, render};

/// The text shaped and drawn: kerning pairs, ligatures, accents,
/// combining marks, Greek, a minus sign and Arabic.
const SAMPLE: &str =
    "AVATAR office fi \u{e9}x\u{301} \u{3b1}\u{3b2}\u{3b3} \u{2212}1 \u{644}\u{627}";

/// A font of the corpus: its name and bytes.
type Font = (&'static str, Vec<u8>);

/// The fonts corrupted: small bundled ones, `DejaVu Sans` (with `GSUB`,
/// `GPOS` and `kern`), and the synthetic CFF, collection and cursive
/// fonts of `plt-font`'s tests.
fn corpus() -> Vec<Font> {
    let bundled = [
        "cmr10.ttf",
        "DejaVuSerifDisplay.ttf",
        "STIXSizOneSymBol.ttf",
        "DejaVuSans.ttf",
    ]
    .map(|name| {
        (
            name,
            bundled_font_data(name).expect("a bundled font").to_vec(),
        )
    });
    let fixtures = ["synthetic-cff.otf", "synthetic.ttc", "arabic-cursive.ttf"].map(|name| {
        (
            name,
            std::fs::read(font_fixture(name)).expect("a plt-font fixture"),
        )
    });
    bundled.into_iter().chain(fixtures).collect()
}

/// The path of a font fixture of `plt-font`'s tests.
fn font_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../plt-font/tests/fixtures")
        .join(name)
}

/// The `(offset, length)` of every table in the directory of `data` (the
/// first face of a collection is skipped over: its directory is found
/// through the header).
fn tables(data: &[u8]) -> Vec<(usize, usize)> {
    let read = |at: usize, len: usize| -> Option<usize> {
        let bytes = data.get(at..at + len)?;
        Some(bytes.iter().fold(0, |acc, &b| acc << 8 | usize::from(b)))
    };
    let start = if data.starts_with(b"ttcf") {
        read(12, 4).unwrap_or(0)
    } else {
        0
    };
    let count = read(start + 4, 2).unwrap_or(0);
    (0..count)
        .filter_map(|i| {
            let record = start + 12 + 16 * i;
            Some((read(record + 8, 4)?, read(record + 12, 4)?))
        })
        .collect()
}

/// A corruption of `data`: anywhere, or inside one of its tables.
fn corruption(g: &mut Gen, data: &[u8]) -> Corruption {
    let tables = tables(data);
    match tables.get(g.below(tables.len() * 2)) {
        Some(&(offset, length)) if offset < data.len() => {
            let end = (offset + length).min(data.len());
            let mut corruption = g.corrupt(&data[offset..end]);
            corruption.shift(offset);
            corruption
        }
        _ => g.corrupt(data),
    }
}

/// Every reading of `data` the font layer offers.
fn exercise_face(data: &[u8]) {
    let Ok(count) = face_count(data) else { return };
    for index in 0..count.min(2) {
        let Ok(face) = Face::parse(data, index) else {
            continue;
        };
        let _ = (
            face.family_name(),
            face.style_name(),
            face.postscript_name(),
        );
        let _ = (
            face.ascender(),
            face.descender(),
            face.height(),
            face.mappings().len(),
        );
        let size = Size::new(&face, 12.0, 72.0);
        let hinting = HintingInstance::new(&face, &size);
        let autohinter = Autohinter::new(&face);
        let glyphs = u32::from(face.num_glyphs());
        let picked: Vec<GlyphId> = (0..glyphs.min(24))
            .chain((0..glyphs).step_by(53))
            .map(|g| GlyphId(g as u16))
            .collect();
        for &glyph in &picked {
            let mut sink = OutlineRecorder::default();
            let _ = face.outline(glyph, &mut sink);
            let _ = size.glyph_cbox(&face, glyph);
            if let Ok(hinted) = hinting.load_glyph(&face, glyph) {
                hinted.outline(&mut sink);
            }
            if let Ok(hinted) = autohinter.load_glyph(&face, &size, glyph, AutohintTarget::Light) {
                hinted.outline(&mut sink);
            }
            let _ = (face.glyph_name(glyph), face.kerning(glyph, GlyphId(36)));
        }
        let _ = subset(&face, picked.iter().copied());
        let _ = shape(&face, SAMPLE);
        let chain = FontChain::new(vec![face]);
        for hinting in [Hinting::None, Hinting::Default, Hinting::Auto] {
            let _ = chain.layout(SAMPLE, 10.0, 100.0, &ShapeOptions::default(), hinting);
        }
    }
}

/// A unique file for a corrupted font: its faces are cached by path.
fn font_file(seed_text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plt-fuzz-fonts-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let hash = seed_text.bytes().fold(0xCBF2_9CE4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
    });
    dir.join(format!("{hash:016x}.ttf"))
}

/// Draws text in the font at `path`, through a private engine and through
/// a figure saved in `format`.
fn draw_with(path: &Path, format: plt::figure::SaveFormat) {
    let engine = TextEngine::new(FontDatabase::new());
    if let Ok(ids) = engine.add_font_file(path)
        && !ids.is_empty()
    {
        let request = FontRequest::new(FontQuery::default());
        if let Ok(run) = engine.layout_line(SAMPLE, &request, 72.0) {
            for glyph in &run.glyphs {
                let _ = engine.glyph_outline(glyph.font, glyph.glyph, 12.0, 72.0);
            }
        }
    }
    let rc = RcParams::matplotlib();
    if let Ok(font) = FontProperties::from_file(path, &rc) {
        let fig = Figure::with_size(2.0, 1.0, 40.0);
        fig.text(0.1, 0.5, SAMPLE).set_font(font.clone());
        fig.text(0.1, 0.2, "$x^2$ AV")
            .set_font(font)
            .set_rotation(30.0);
        render(&fig, format);
    }
}

#[test]
fn corrupted_fonts() {
    let corpus = corpus();
    fuzz(
        "fonts",
        500,
        |g| {
            let font = g.below(corpus.len());
            (corpus[font].0, font, corruption(g, &corpus[font].1))
        },
        |(_, font, corruption)| exercise_face(&corruption.apply(&corpus[*font].1)),
    );
}

#[test]
fn drawing_with_corrupted_fonts() {
    let corpus = corpus();
    fuzz(
        "font drawing",
        200,
        |g| {
            let font = g.below(corpus.len());
            let format = g.pick(&FORMATS);
            (corpus[font].0, font, corruption(g, &corpus[font].1), format)
        },
        |input| {
            let (_, font, corruption, format) = input;
            let path = font_file(&format!("{input:?}"));
            if std::fs::write(&path, corruption.apply(&corpus[*font].1)).is_ok() {
                draw_with(&path, *format);
                let _ = std::fs::remove_file(&path);
            }
        },
    );
}

/// The environment variable that marks the child process adding fonts to
/// the shared catalogue.
const CATALOGUE_CHILD: &str = "PLT_FUZZ_FONT_CATALOGUE";

/// Adds the corrupted font to the shared catalogue (matplotlib's
/// `fontManager.addfont`) and draws text in its family.
fn add_and_draw(path: &Path, bytes: &[u8], format: plt::figure::SaveFormat) {
    if FontManager::global().add_font(path).is_err() {
        return;
    }
    let family = Face::parse(bytes, 0)
        .ok()
        .and_then(|face| face.family_name());
    let fig = Figure::with_size(2.0, 1.0, 40.0);
    let text = fig.text(0.1, 0.5, SAMPLE);
    text.set_family(family.into_iter().chain(["DejaVu Sans".to_owned()]));
    render(&fig, format);
}

/// Corrupted fonts added to the catalogue, in a child process: the
/// catalogue is shared by every test of a process, and fonts claiming
/// the default family would leak into the other targets.
#[test]
fn adding_corrupted_fonts_to_the_catalogue() {
    if std::env::var_os(CATALOGUE_CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
            .args([
                "--exact",
                "fonts::adding_corrupted_fonts_to_the_catalogue",
                "--test-threads=1",
            ])
            .env(CATALOGUE_CHILD, "1")
            .output()
            .expect("a child process");
        assert!(
            output.status.success(),
            "the child adding fonts failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let corpus = corpus();
    fuzz(
        "font catalogue",
        100,
        |g| {
            let font = g.below(corpus.len());
            let format = g.pick(&FORMATS);
            (corpus[font].0, font, corruption(g, &corpus[font].1), format)
        },
        |input| {
            let (_, font, corruption, format) = input;
            let bytes = corruption.apply(&corpus[*font].1);
            let path = font_file(&format!("catalogue {input:?}"));
            if std::fs::write(&path, &bytes).is_ok() {
                add_and_draw(&path, &bytes, *format);
            }
        },
    );
}

/// The AFM files of the PDF core fonts.
fn afm_corpus() -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../plt-font/fonts/pdfcorefonts");
    ["Helvetica.afm", "Courier-Bold.afm", "Symbol.afm"]
        .iter()
        .map(|name| {
            let bytes = std::fs::read(dir.join(name)).expect("a bundled AFM file");
            ((*name).to_owned(), bytes)
        })
        .collect()
}

#[test]
fn corrupted_afm_files() {
    let corpus = afm_corpus();
    fuzz(
        "afm",
        1_500,
        |g| {
            let file = g.below(corpus.len());
            (corpus[file].0.clone(), file, g.corrupt(&corpus[file].1))
        },
        |(_, file, corruption)| {
            if let Ok(metrics) = AfmMetrics::parse(&corruption.apply(&corpus[*file].1)) {
                let _ = (metrics.font_name(), metrics.ascender(), metrics.descender());
                let _ = metrics.str_bbox_and_descent(SAMPLE);
                let _ = (
                    metrics.width_from_char_name("A"),
                    metrics.kern_dist_from_name("A", "V"),
                );
                let _ = (metrics.char_by_code(65), metrics.composite("Aacute"));
            }
        },
    );
}
