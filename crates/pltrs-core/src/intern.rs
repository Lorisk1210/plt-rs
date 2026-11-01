//! Strings kept for the life of the process: [`interned`].

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

/// `text` as a `&'static str`, allocated once per distinct string for the
/// life of the process: names that only come at run time (read from an
/// archive, given by a program) where a `&'static str` is needed.
///
/// ```
/// let name = pltrs_core::interned(&String::from("skewx"));
/// assert_eq!(name, "skewx");
/// assert!(std::ptr::eq(name, pltrs_core::interned("skewx")), "allocated once");
/// ```
pub fn interned(text: &str) -> &'static str {
    static INTERNED: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
    let mut interned = INTERNED.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(&known) = interned.get(text) {
        return known;
    }
    let leaked: &'static str = Box::leak(text.to_owned().into_boxed_str());
    interned.insert(leaked);
    leaked
}
