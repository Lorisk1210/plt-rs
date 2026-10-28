//! Fonts: the font family from the rcParams, a title in a font chosen by
//! its file, a table of the default font's first 256 characters and every
//! font property set through font properties or text keywords
//! (`font_family_rc`, `font_file`, `font_table`, `fonts_demo` and
//! `fonts_demo_kw`).

use plt::fonts::{
    FontExt, FontManager, FontProperties, FontSize, FontWeight, RelativeSize, parse_style,
    parse_variant,
};
use plt::layout::TightLayout;
use plt::prelude::*;
use plt::table::{TableLoc, TableOptions};
use plt::text::{HAlign, Text, VAlign};
use plt_font::{FontDatabase, FontEntry, FontSource, GlyphId};

use crate::figures::{CaseResult, FigureCase};

/// `print_text`: one big centred line on a pale green figure without
/// axes decorations.
fn print_text(fig: &Figure, text: &str) -> CaseResult {
    fig.set_facecolor(Color::from("#eefade").resolve()?);
    let ax = fig.subplot();
    ax.text(0.5, 0.5, text)
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_fontsize(40.0);
    ax.set_axis_off();
    Ok(())
}

fn font_family_rc_1(fig: &Figure) -> CaseResult {
    print_text(fig, "Hello World! 01")
}

fn font_family_rc_3(fig: &Figure) -> CaseResult {
    print_text(fig, "Hello World! 03")
}

/// The file of the default font (matplotlib's
/// `findfont(FontProperties())`).
fn default_font_file() -> Result<String, Box<dyn std::error::Error>> {
    let rc = plt::rc::params();
    Ok(FontManager::global().find_font(&FontProperties::from_rc(&rc), &rc, FontExt::Ttf, true)?)
}

/// The bundled fonts and the face of the font file `file` among them.
fn bundled_face(file: &str) -> Result<(FontDatabase, FontEntry), Box<dyn std::error::Error>> {
    let db = FontDatabase::with_bundled_fonts();
    let entry = db
        .entries()
        .iter()
        .find(|entry| db.source(entry.font).and_then(FontSource::file_name) == Some(file))
        .cloned()
        .ok_or_else(|| format!("{file} is not bundled"))?;
    Ok((db, entry))
}

fn font_file(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let file = "cmr10.ttf";
    ax.set_title(format!("This is a special font: {file}"))
        .set_font(FontProperties::from_file(file, &plt::rc::params())?);
    ax.set_xlabel("This is the default font");
    Ok(())
}

/// The characters below 256 the face of the bundled `file` maps, by
/// code (matplotlib's `FT2Font.get_charmap`).
fn charmap_below_256(file: &str) -> Result<Vec<Option<char>>, Box<dyn std::error::Error>> {
    let (db, entry) = bundled_face(file)?;
    let mut chars = vec![None; 256];
    for (code, glyph) in db.face(entry.font)?.mappings() {
        if glyph != GlyphId(0)
            && let Some(slot) = usize::try_from(code).ok().and_then(|i| chars.get_mut(i))
        {
            *slot = char::from_u32(code);
        }
    }
    Ok(chars)
}

fn font_table(fig: &Figure) -> CaseResult {
    let file = default_font_file()?;
    let chars = charmap_below_256(&file)?;
    let ax = fig.subplot();
    ax.set_title(file.as_str());
    ax.set_axis_off();
    let palegreen = || vec![Color::from("palegreen"); 16];
    let table = ax.table(TableOptions {
        cell_colours: Some(vec![vec![Color::from(".95"); 16]; 16]),
        cell_loc: Some(HAlign::Center),
        row_labels: Some((0..16).map(|row| format!("{:02X}", row * 16)).collect()),
        row_colours: Some(palegreen()),
        col_labels: Some((0..16).map(|col| format!("{col:X}")).collect()),
        col_colours: Some(palegreen()),
        loc: TableLoc::UpperLeft,
        ..TableOptions::with_text(
            chars
                .chunks(16)
                .map(|row| {
                    row.iter()
                        .map(|c| c.map(String::from).unwrap_or_default())
                        .collect()
                })
                .collect(),
        )
    })?;
    let font = FontProperties::from_file(&file, &plt::rc::params())?;
    for (row, col) in table.cell_keys() {
        if row > 0
            && col > -1
            && let Some(cell) = table.cell(row, col)
        {
            cell.set_font(font.clone());
        }
    }
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

/// The anchor heights of the rows of `fonts_demo`.
const YP: [f64; 7] = [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2];
const FAMILIES: [&str; 5] = ["serif", "sans-serif", "cursive", "fantasy", "monospace"];
const STYLES: [&str; 3] = ["normal", "italic", "oblique"];
const VARIANTS: [&str; 2] = ["normal", "small-caps"];
const WEIGHTS: [&str; 7] = [
    "light", "normal", "medium", "semibold", "bold", "heavy", "black",
];
const SIZES: [RelativeSize; 7] = [
    RelativeSize::XxSmall,
    RelativeSize::XSmall,
    RelativeSize::Small,
    RelativeSize::Medium,
    RelativeSize::Large,
    RelativeSize::XLarge,
    RelativeSize::XxLarge,
];
/// The heights and sizes of the bold italic samples.
const BOLD_ITALIC: [(f64, RelativeSize); 3] = [
    (0.1, RelativeSize::XSmall),
    (0.2, RelativeSize::Medium),
    (0.3, RelativeSize::XLarge),
];

/// A text of `fonts_demo`, centred on its baseline at `(x, y)`.
fn sample(fig: &Figure, x: f64, y: f64, text: &str) -> Text {
    let text = fig.text(x, y, text);
    text.set_halign(HAlign::Center).set_valign(VAlign::Baseline);
    text
}

/// A weight keyword of `fonts_demo`.
fn weight(name: &str) -> Result<FontWeight, String> {
    FontWeight::from_name(name).ok_or_else(|| format!("unknown weight {name}"))
}

/// `fonts_demo`: every font property set through `FontProperties`.
fn fonts_demo(fig: &Figure) -> CaseResult {
    let rc = plt::rc::params();
    let base = || FontProperties::from_rc(&rc);
    let sized = |size: RelativeSize| base().with_size(FontSize::Relative(size), rc.font_size);
    let heading = sized(RelativeSize::Large);
    let column = |x: f64, title: &str, fonts: Vec<(&str, FontProperties)>| {
        sample(fig, x, 0.9, title).set_font(heading.clone());
        for ((label, font), y) in fonts.into_iter().zip(YP) {
            sample(fig, x, y, label).set_font(font);
        }
    };
    let family = |name: &str| FontProperties {
        families: vec![name.to_owned()],
        ..base()
    };
    column(
        0.1,
        "family",
        FAMILIES.iter().map(|&name| (name, family(name))).collect(),
    );
    let mut styles = Vec::new();
    for name in STYLES {
        styles.push((
            name,
            FontProperties {
                style: parse_style(name)?,
                ..family("sans-serif")
            },
        ));
    }
    column(0.3, "style", styles);
    let mut variants = Vec::new();
    for name in VARIANTS {
        variants.push((
            name,
            FontProperties {
                variant: parse_variant(name)?,
                ..family("serif")
            },
        ));
    }
    column(0.5, "variant", variants);
    let mut weights = Vec::new();
    for name in WEIGHTS {
        weights.push((name, base().with_weight(weight(name)?)));
    }
    column(0.7, "weight", weights);
    column(
        0.9,
        "size",
        SIZES
            .iter()
            .map(|&size| (size.name(), sized(size)))
            .collect(),
    );
    for (y, size) in BOLD_ITALIC {
        let font = FontProperties {
            style: parse_style("italic")?,
            ..sized(size).with_weight(FontWeight::BOLD)
        };
        sample(fig, 0.3, y, "bold italic").set_font(font);
    }
    Ok(())
}

/// `fonts_demo_kw`: every font property set through text keywords.
fn fonts_demo_kw(fig: &Figure) -> CaseResult {
    sample(fig, 0.1, 0.9, "family").set_fontsize(RelativeSize::Large);
    for (name, y) in FAMILIES.into_iter().zip(YP) {
        sample(fig, 0.1, y, name).set_family([name]);
    }
    sample(fig, 0.3, 0.9, "style");
    for (name, y) in STYLES.into_iter().zip(YP) {
        sample(fig, 0.3, y, name)
            .set_family(["sans-serif"])
            .set_style(parse_style(name)?);
    }
    sample(fig, 0.5, 0.9, "variant");
    for (name, y) in VARIANTS.into_iter().zip(YP) {
        sample(fig, 0.5, y, name)
            .set_family(["serif"])
            .set_variant(parse_variant(name)?);
    }
    sample(fig, 0.7, 0.9, "weight");
    for (name, y) in WEIGHTS.into_iter().zip(YP) {
        sample(fig, 0.7, y, name).set_weight(weight(name)?);
    }
    sample(fig, 0.9, 0.9, "size");
    for (size, y) in SIZES.into_iter().zip(YP) {
        sample(fig, 0.9, y, size.name()).set_fontsize(size);
    }
    for (y, size) in BOLD_ITALIC {
        sample(fig, 0.3, y, "bold italic")
            .set_style(parse_style("italic")?)
            .set_weight(FontWeight::BOLD)
            .set_fontsize(size);
    }
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_text/font_family_rc_1", font_family_rc_1)
        .sized(6.0, 1.0, 100.0)
        .with_rc(&[("font.family", "sans-serif")]),
    FigureCase::new("gallery_text/font_family_rc_3", font_family_rc_3)
        .sized(6.0, 1.0, 100.0)
        .with_rc(&[("font.family", "monospace")]),
    FigureCase::new("gallery_text/font_file", font_file),
    FigureCase::new("gallery_text/font_table", font_table).sized(8.0, 4.0, 100.0),
    FigureCase::new("gallery_text/fonts_demo", fonts_demo),
    FigureCase::new("gallery_text/fonts_demo_kw", fonts_demo_kw),
];
