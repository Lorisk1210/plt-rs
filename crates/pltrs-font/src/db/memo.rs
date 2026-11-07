//! The memo of `findfont` lookups, as matplotlib memoizes them
//! (`_findfont_cached`, an `lru_cache` over the font properties and the
//! `font.*` rcParams): the best catalogue entry of each query and generic
//! family lists, forgotten whenever the catalogue changes. Scoring every
//! catalogued font takes long on systems with hundreds of them.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use super::entry::FontEntry;
use super::query::{FontQuery, FontStyle, FontVariant, GenericFamilies, Stretch, Weight};

/// The most lookups kept (matplotlib's cache keeps 1024 too); the memo
/// starts over when it is full.
const CAPACITY: usize = 1024;

/// What a lookup's result depends on: the query but for its size, which
/// scalable fonts ignore, and the generic family lists.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct Key {
    families: Vec<String>,
    style: FontStyle,
    variant: FontVariant,
    weight: Weight,
    stretch: Stretch,
    generics: GenericFamilies,
}

/// A lookup's result: the best entry and its score.
type Found = Option<(FontEntry, f64)>;

/// The memo, with a generation that changes with the catalogue, so that a
/// lookup made while the catalogue changed is not kept.
#[derive(Debug, Default)]
pub(super) struct LookupMemo {
    state: Mutex<(u64, HashMap<Key, Found>)>,
}

impl LookupMemo {
    /// The memoized result of the lookup of `query` with `generics`, made
    /// by `find` when it is not known.
    pub(super) fn find(
        &self,
        query: &FontQuery,
        generics: &GenericFamilies,
        find: impl FnOnce() -> Found,
    ) -> Found {
        let key = Key {
            families: query.families.clone(),
            style: query.style,
            variant: query.variant,
            weight: query.weight,
            stretch: query.stretch,
            generics: generics.clone(),
        };
        let generation = {
            let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(found) = state.1.get(&key) {
                return found.clone();
            }
            state.0
        };
        let found = find();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.0 == generation {
            if state.1.len() >= CAPACITY {
                state.1.clear();
            }
            state.1.insert(key, found.clone());
        }
        found
    }

    /// Forgets every lookup: the catalogue changed.
    pub(super) fn clear(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.0 += 1;
        state.1.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups_are_kept_until_cleared() {
        let memo = LookupMemo::default();
        let query = FontQuery::default();
        let generics = GenericFamilies::default();
        let mut calls = 0;
        let mut lookup = |memo: &LookupMemo| {
            memo.find(&query, &generics, || {
                calls += 1;
                None
            })
        };
        assert_eq!(lookup(&memo), None);
        assert_eq!(lookup(&memo), None);
        memo.clear();
        assert_eq!(lookup(&memo), None);
        assert_eq!(calls, 2);
        // Another size is the same lookup; other generic lists are not.
        let larger = FontQuery {
            size: 20.0,
            ..FontQuery::default()
        };
        assert_eq!(memo.find(&larger, &generics, || unreachable!()), None);
        let mut others = GenericFamilies::default();
        others.serif.clear();
        let mut called = false;
        memo.find(&query, &others, || {
            called = true;
            None
        });
        assert!(called);
    }
}
