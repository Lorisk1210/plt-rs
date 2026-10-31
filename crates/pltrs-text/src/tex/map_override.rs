//! The program's replacement of the font map: matplotlib looks every TeX
//! font up through `PsfontsMap.__getitem__`, which its tests replace
//! (`monkeypatch.setattr`) to give fonts without a Type 1 file. pltrs
//! (Python) hands such a replacement to pltrs for the thread, and while
//! it is set every font usetex draws is looked up through it.

use std::cell::RefCell;
use std::rc::Rc;

use super::error::TexError;
use super::psfonts::PsFont;

/// Looks TeX fonts up in place of `pdftex.map`.
pub trait FontMapLookup {
    /// The PostScript font of TeX font `texname`.
    ///
    /// # Errors
    ///
    /// When there is none.
    fn lookup(&self, texname: &[u8]) -> Result<PsFont, TexError>;
}

thread_local! {
    /// The replacement of the thread's font map, if the program set one.
    static LOOKUP: RefCell<Option<Rc<dyn FontMapLookup>>> = const { RefCell::new(None) };
}

/// Sets (or, with `None`, removes) what looks TeX fonts up on this thread
/// in place of the font map.
pub fn set_font_map_lookup(lookup: Option<Rc<dyn FontMapLookup>>) {
    LOOKUP.set(lookup);
}

/// The thread's replacement of the font map.
pub(crate) fn font_map_lookup() -> Option<Rc<dyn FontMapLookup>> {
    LOOKUP.with_borrow(Clone::clone)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pltrs_font::Type1Effects;

    use super::*;
    use crate::tex::dvi::DviFont;
    use crate::tex::test_support::files;
    use crate::tex::tfm::Tfm;
    use crate::tex::tfm::tests::tfm_bytes;

    /// Gives every TeX font a PostScript font without a Type 1 file, as
    /// `test_usetex.py::test_missing_psfont` does.
    struct WithoutType1;

    impl FontMapLookup for WithoutType1 {
        fn lookup(&self, texname: &[u8]) -> Result<PsFont, TexError> {
            Ok(PsFont {
                texname: texname.to_vec(),
                psname: b"Some Font".to_vec(),
                effects: Type1Effects::default(),
                encoding: None,
                filename: None,
            })
        }
    }

    #[test]
    fn a_replacement_looks_every_font_up_while_it_is_set() {
        let files = files(&[]);
        let tfm = Arc::new(Tfm::parse(&tfm_bytes(65, 65, &[1], 0)).expect("valid"));
        let font = DviFont::new(1 << 20, tfm, b"cmr10".to_vec(), None);
        // The files of the test hold no font map.
        assert!(matches!(
            files.psfont(b"cmr10"),
            Err(TexError::NotFound { .. })
        ));
        set_font_map_lookup(Some(Rc::new(WithoutType1)));
        let psfont = files.psfont(b"cmr10");
        let path = files.font_path(&font);
        let file = files.font_file(&font);
        set_font_map_lookup(None);
        assert_eq!(psfont.expect("looked up").psname, b"Some Font");
        assert!(matches!(path, Err(TexError::NoType1Font { .. })));
        assert!(matches!(file, Err(TexError::NoType1Font { .. })));
        assert!(files.psfont(b"cmr10").is_err(), "the font map again");
    }
}
