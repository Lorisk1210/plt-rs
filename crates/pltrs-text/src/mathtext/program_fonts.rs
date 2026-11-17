//! A program's own font set for mathtext: matplotlib typesets with the
//! `Fonts` class `MathTextParser._font_type_mapping` holds for the
//! fontset, and asks it for every glyph (`_get_glyph`) and every sized
//! delimiter (`get_sized_alternatives_for_symbol`); a program putting a
//! subclass of its own there (ultraplot routes some symbols to Computer
//! Modern) changes which face draws a glyph. pltrs (Python) hands the
//! thread such font sets ([`set_program_fontsets`]); while a class of the
//! program's serves a style's fontset, each typesetting run makes one
//! ([`ProgramFontsets::fonts_for`], as matplotlib makes one per parse) and
//! takes the face, code and slant of each glyph from it, its metrics,
//! kerning and boxes still computed natively. Nothing else changes for
//! the fontsets matplotlib's own classes serve.

use std::cell::RefCell;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

use super::style::MathStyle;

/// A glyph a program's font set names (matplotlib's `_get_glyph` gives
/// `(font, num, slanted)`): the face, as its font file and face index, the
/// character code in it, and whether it leans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramGlyph {
    /// The font file.
    pub file: PathBuf,
    /// The face in the file.
    pub face_index: u32,
    /// The character code in the face.
    pub code: u32,
    /// Whether scripts treat the glyph as leaning.
    pub slanted: bool,
}

/// The font set of the program's one typesetting run asks.
pub trait ProgramFonts: fmt::Debug {
    /// The glyph drawing `symbol` in the TeX font `font` of class `class`
    /// (matplotlib's `_get_glyph(fontname, font_class, sym)`).
    ///
    /// # Errors
    ///
    /// What the program's code raised, as its message.
    fn glyph(&self, font: &str, class: &str, symbol: &str) -> Result<ProgramGlyph, String>;

    /// The size variants of `symbol`, smallest first, as (TeX font, symbol)
    /// pairs (matplotlib's `get_sized_alternatives_for_symbol`).
    ///
    /// # Errors
    ///
    /// What the program's code raised, as its message.
    fn sized_alternatives(&self, font: &str, symbol: &str)
    -> Result<Vec<(String, String)>, String>;
}

/// The font set classes of a program's, by fontset.
pub trait ProgramFontsets: fmt::Debug {
    /// A font set of the program's for one typesetting run of `style`, if
    /// a class of the program's serves its fontset; `None` leaves the run
    /// to pltrs' own.
    ///
    /// # Errors
    ///
    /// What the program's class raised making one, as its message.
    fn fonts_for(&self, style: &MathStyle) -> Result<Option<Box<dyn ProgramFonts>>, String>;
}

pltrs_core::thread_state! {
    /// The program's font set classes on this thread, if it put any in
    /// place of matplotlib's.
    static FONTSETS: RefCell<Option<Rc<dyn ProgramFontsets>>> = const { RefCell::new(None) };
}

/// Sets (or, with `None`, removes) the program's font set classes on this
/// thread.
pub fn set_program_fontsets(fontsets: Option<Rc<dyn ProgramFontsets>>) {
    FONTSETS.set(fontsets);
}

/// Whether the program put font set classes of its own in place on this
/// thread: what is typeset then may follow state of the program's, which
/// no cache of pltrs' can key on.
pub(crate) fn in_place() -> bool {
    FONTSETS.with_borrow(Option::is_some)
}

/// The font set of the program's that a typesetting run of `style` takes
/// its glyphs from, if a class of the program's serves its fontset.
pub(crate) fn program_fonts(style: &MathStyle) -> Result<Option<Box<dyn ProgramFonts>>, String> {
    match FONTSETS.with_borrow(Clone::clone) {
        Some(fontsets) => fontsets.fonts_for(style),
        None => Ok(None),
    }
}
