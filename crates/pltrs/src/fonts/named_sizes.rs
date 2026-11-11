//! Font size keywords a program adds to matplotlib's: matplotlib looks a
//! size keyword up in `font_manager.font_scalings`, a dict programs add to
//! (ultraplot gives it `med-small`, `med` and `med-large`). A
//! [`NamedSize`] is such a keyword with the factor it had when it was
//! read; [`set_size_keywords`] tells pltrs where to look them up.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, PoisonError};

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};

/// Looks a keyword up among those the program added.
type Lookup = Rc<dyn Fn(&str) -> Option<f64>>;

pltrs_core::thread_state! {
    /// Where the keywords the program added are looked up, if anywhere.
    static LOOKUP: RefCell<Option<Lookup>> = const { RefCell::new(None) };
}

/// The keywords read so far, each kept once for the life of the process
/// (sizes are `Copy` values naming them).
static NAMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// Has keywords other than matplotlib's own looked up with `lookup`, which
/// gives the factor of the base size a keyword stands for, if it is one.
pub fn set_size_keywords(lookup: impl Fn(&str) -> Option<f64> + 'static) {
    LOOKUP.with_borrow_mut(|slot| *slot = Some(Rc::new(lookup)));
}

/// A size keyword the program added, with its factor of the base size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NamedSize {
    name: &'static str,
    scale: f64,
}

impl NamedSize {
    /// The keyword `name` standing for `scale` times the base size.
    pub fn new(name: &str, scale: f64) -> Self {
        let mut names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
        let known = names.iter().find(|known| **known == name).copied();
        let name = known.unwrap_or_else(|| {
            // Leaked once per distinct keyword: programs add a handful.
            let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
            names.push(leaked);
            leaked
        });
        Self { name, scale }
    }

    /// The keyword `name` as the program defines it now, if it does.
    pub(super) fn find(name: &str) -> Option<Self> {
        // Called with the slot free: the lookup runs the program's code.
        let lookup = LOOKUP.with_borrow(Clone::clone)?;
        let scale = lookup(name)?;
        Some(Self::new(name, scale))
    }

    /// The keyword.
    pub fn name(self) -> &'static str {
        self.name
    }

    /// The factor applied to the base font size.
    pub fn scale(self) -> f64 {
        self.scale
    }
}

/// Archived as the keyword and its factor.
impl Archive for NamedSize {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.name.to_owned().encode(encoder)?;
        self.scale.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let name = String::decode(decoder)?;
        Ok(Self::new(&name, f64::decode(decoder)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::FontSize;

    #[test]
    fn keywords_the_program_adds_are_sizes() {
        assert!("med-large".parse::<FontSize>().is_err());
        set_size_keywords(|name| (name == "med-large").then_some(1.1));
        let size: FontSize = "med-large".parse().unwrap();
        assert_eq!(size.to_points(10.0), 11.0);
        assert_eq!(size.to_string(), "med-large");
        assert_eq!(size, FontSize::Named(NamedSize::new("med-large", 1.1)));
        assert!("huge".parse::<FontSize>().is_err());
        set_size_keywords(|_| None);
    }
}
