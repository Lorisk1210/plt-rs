//! The first text a measurement could not lay out.
//!
//! matplotlib measures a text by laying it out, and a text that cannot be
//! laid out (malformed mathtext, a font file that is not there) raises
//! its error out of whatever measures it: `get_tightbbox`, `tight_layout`.
//! pltrs measures texts where an extent may be missing
//! ([`Artist::window_extent`](crate::artist::Artist::window_extent)), so a
//! text that fails notes its error here, and the measurements matplotlib
//! fails run through [`measuring`] to learn of it.

use std::cell::RefCell;

use super::TextDrawError;

thread_local! {
    /// The first text error noted since the innermost [`measuring`] began.
    static FAILED: RefCell<Option<TextDrawError>> = const { RefCell::new(None) };
}

/// Notes that a text could not be laid out as it was measured, unless an
/// earlier one did since the measurement began.
pub(crate) fn note(err: &TextDrawError) {
    FAILED.with_borrow_mut(|failed| {
        if failed.is_none() {
            *failed = Some(err.clone());
        }
    });
}

/// Runs `measure`, failing with the error of the first text it could not
/// lay out, as matplotlib's measurement raises it.
///
/// ```
/// use pltrs::prelude::*;
/// let ax = Figure::new().subplot();
/// ax.set_title(r"$\stackrel{2}{3}$");
/// let measured = pltrs::text::measuring(|| ax.tight_bbox(false));
/// assert!(measured.unwrap_err().to_string().contains(r"Unknown symbol: \stackrel"));
/// ```
///
/// # Errors
///
/// The error of the first text `measure` could not lay out.
pub fn measuring<R>(measure: impl FnOnce() -> R) -> Result<R, TextDrawError> {
    let outer = FAILED.take();
    let measured = measure();
    FAILED.replace(outer).map_or(Ok(measured), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs_text::TextError;

    fn math(why: &str) -> TextDrawError {
        TextDrawError::Layout(TextError::Math(why.to_owned()))
    }

    #[test]
    fn the_first_failure_of_a_measurement_is_its_error() {
        assert_eq!(measuring(|| 1), Ok(1));
        let failed = measuring(|| {
            note(&math("first"));
            note(&math("second"));
        });
        assert_eq!(failed, Err(math("first")));
        assert_eq!(measuring(|| ()), Ok(()), "nothing is left over");
    }

    #[test]
    fn measurements_nest() {
        let outer = measuring(|| {
            note(&math("outer"));
            let inner = measuring(|| note(&math("inner")));
            assert_eq!(inner, Err(math("inner")));
        });
        assert_eq!(outer, Err(math("outer")));
    }
}
