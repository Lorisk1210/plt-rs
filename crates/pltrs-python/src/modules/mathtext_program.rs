//! A program's own mathtext font classes, glyph by glyph: while a class of
//! the program's serves a fontset in `MathTextParser._font_type_mapping`
//! (`set_program_fonts`), every typesetting run of that fontset makes an
//! instance of it through the package's `program_fontset` hook and asks it
//! for each glyph and sized delimiter (`program_glyph`,
//! `program_alternatives`), as matplotlib asks the class it makes per parse
//! (see `pltrs_text::mathtext::program_fonts`). Typesetting with
//! matplotlib's own classes never reaches Python.

use std::path::PathBuf;
use std::rc::Rc;

use pltrs_font::FontStyle;
use pltrs_text::mathtext::MathStyle;
use pltrs_text::mathtext::program_fonts::{
    ProgramFonts, ProgramFontsets, ProgramGlyph, set_program_fontsets,
};

use crate::binding::prelude::*;

/// The program's classes, made into font sets through the package's
/// `program_fontset` hook.
#[derive(Debug)]
struct Fontsets;

/// The font properties of `style`'s text font, as the hook takes them:
/// `(families, style, weight, stretch, size)`.
type FontFields = (Vec<String>, &'static str, u16, u16, f64);

/// The fields of the text font `style` typesets with.
fn font_fields(style: &MathStyle) -> FontFields {
    let query = &style.font.query;
    let slant = match query.style {
        FontStyle::Normal => "normal",
        FontStyle::Italic => "italic",
        FontStyle::Oblique => "oblique",
    };
    (
        query.families.clone(),
        slant,
        query.weight.0,
        query.stretch.0,
        query.size,
    )
}

/// What the typesetter reports when the program's Python code raised: the
/// exception itself is kept in the running native call's frame, which
/// raises it once pltrs (Rust) returns, as matplotlib's parse lets it
/// through.
fn raised(err: PyErr) -> String {
    crate::binding::report(err);
    RAISED.to_owned()
}

/// The message of a typesetting run the program's code stopped.
const RAISED: &str = "the program's mathtext font class raised";

/// Stops a run whose program code raised already: later calls of the same
/// native call are skipped, so one mistake raises once.
fn not_after_failure() -> Result<(), String> {
    if crate::binding::callback_failed() {
        Err(RAISED.to_owned())
    } else {
        Ok(())
    }
}

impl ProgramFontsets for Fontsets {
    fn fonts_for(&self, style: &MathStyle) -> Result<Option<Box<dyn ProgramFonts>>, String> {
        not_after_failure()?;
        Python::with_gil(|py| {
            let made = (|| {
                let hook = crate::binding::lookup(py, "program_fontset")?;
                hook.call(
                    &[
                        style.fontset.name().into_py(py)?,
                        font_fields(style).into_py(py)?,
                        style.dpi.into_py(py)?,
                    ],
                    &[],
                )
            })()
            .map_err(raised)?;
            Ok((!made.is_none()).then(|| Box::new(Fonts(made.unbind())) as Box<dyn ProgramFonts>))
        })
    }
}

/// An instance of the program's class, for one typesetting run.
#[derive(Debug)]
struct Fonts(Owned);

impl ProgramFonts for Fonts {
    fn glyph(&self, font: &str, class: &str, symbol: &str) -> Result<ProgramGlyph, String> {
        not_after_failure()?;
        Python::with_gil(|py| {
            let glyph = (|| {
                crate::binding::lookup(py, "program_glyph")?
                    .call(
                        &[
                            self.0.bind(py),
                            font.into_py(py)?,
                            class.into_py(py)?,
                            symbol.into_py(py)?,
                        ],
                        &[],
                    )?
                    .extract::<(String, u32, u32, bool)>()
            })()
            .map_err(raised)?;
            let (file, face_index, code, slanted) = glyph;
            Ok(ProgramGlyph {
                file: PathBuf::from(file),
                face_index,
                code,
                slanted,
            })
        })
    }

    fn sized_alternatives(
        &self,
        font: &str,
        symbol: &str,
    ) -> Result<Vec<(String, String)>, String> {
        not_after_failure()?;
        Python::with_gil(|py| {
            crate::binding::lookup(py, "program_alternatives")?
                .call(
                    &[self.0.bind(py), font.into_py(py)?, symbol.into_py(py)?],
                    &[],
                )?
                .extract()
        })
        .map_err(raised)
    }
}

/// Has every typesetting run of the fontsets the program's classes serve
/// take its glyphs from them while `program` is true (the package says so
/// when `MathTextParser._font_type_mapping` changes).
fn set_program_fonts(program: bool) -> PyResult<()> {
    set_program_fontsets(program.then(|| Rc::new(Fontsets) as Rc<dyn ProgramFontsets>));
    Ok(())
}

/// matplotlib's `_mathtext_data.latex_to_bakoma` entries: `(symbol, font,
/// code)`.
fn latex_to_bakoma() -> PyResult<Vec<(&'static str, &'static str, u32)>> {
    Ok(pltrs_text::mathtext::tables::latex_to_bakoma())
}

/// matplotlib's `_mathtext_data.tex2uni` entries.
fn tex2uni() -> PyResult<Vec<(&'static str, u32)>> {
    Ok(pltrs_text::mathtext::tables::tex2uni())
}

/// matplotlib's `_mathtext_data.stix_glyph_fixes` entries.
fn stix_glyph_fixes() -> PyResult<Vec<(u32, u32)>> {
    Ok(pltrs_text::mathtext::tables::stix_glyph_fixes())
}

/// A virtual font's ranges as the package takes them: `(None, ranges)`
/// for one list, else `(classes, [])` with a list per font class.
type VirtualFontEntry = (
    String,
    Option<Vec<(String, Vec<pltrs_text::mathtext::tables::VirtualRange>)>>,
    Vec<pltrs_text::mathtext::tables::VirtualRange>,
);

/// matplotlib's `_mathtext_data.stix_virtual_fonts`, as `(name, by class,
/// single)` entries.
fn stix_virtual_fonts() -> PyResult<Vec<VirtualFontEntry>> {
    use pltrs_text::mathtext::tables::VirtualFontRanges;
    Ok(pltrs_text::mathtext::tables::stix_virtual_fonts()
        .into_iter()
        .map(|(name, ranges)| match ranges {
            VirtualFontRanges::Single(single) => (name, None, single),
            VirtualFontRanges::ByClass(classes) => (name, Some(classes), Vec::new()),
        })
        .collect())
}

/// Registers the module's functions (in `_native.mathtext`).
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_program_fonts", set_program_fonts)?;
    module.add("latex_to_bakoma", latex_to_bakoma)?;
    module.add("tex2uni", tex2uni)?;
    module.add("stix_glyph_fixes", stix_glyph_fixes)?;
    module.add("stix_virtual_fonts", stix_virtual_fonts)
}
