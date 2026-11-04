//! Memoised layouts. A figure lays out the same tick labels, titles and
//! formulas many times per draw (constrained layout measures every axes
//! twice before drawing), and every layout is a pure function of its
//! inputs, so the engine keeps results by their inputs, as matplotlib's
//! `Text` caches its layouts.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Mutex, PoisonError};

use crate::FontRequest;

/// How many results a memo keeps before it starts afresh.
const LIMIT: usize = 16_384;

/// The results of a pure function by their inputs `K`. Inputs are filed
/// under a cheap fingerprint and told apart by full equality, so a
/// fingerprint need not cover every input.
#[derive(Debug)]
pub(crate) struct Memo<K, V> {
    entries: Mutex<Entries<K, V>>,
}

/// The stored results and their count.
#[derive(Debug)]
struct Entries<K, V> {
    by_fingerprint: HashMap<u64, Vec<(K, V)>>,
    len: usize,
}

impl<K, V: Clone> Memo<K, V> {
    /// An empty memo.
    pub(crate) fn new() -> Self {
        Self {
            entries: Mutex::new(Entries {
                by_fingerprint: HashMap::new(),
                len: 0,
            }),
        }
    }

    /// The result for the inputs that `matches` recognises, filed under
    /// `fingerprint`: the stored one, or `compute`'s, stored with the
    /// inputs `key` builds. Failures are not stored.
    pub(crate) fn get_or_compute<E>(
        &self,
        fingerprint: u64,
        matches: impl Fn(&K) -> bool,
        key: impl FnOnce() -> K,
        compute: impl FnOnce() -> Result<V, E>,
    ) -> Result<V, E> {
        let found = {
            let entries = self.lock();
            entries
                .by_fingerprint
                .get(&fingerprint)
                .and_then(|bucket| bucket.iter().find(|(stored, _)| matches(stored)))
                .map(|(_, value)| value.clone())
        };
        if let Some(value) = found {
            return Ok(value);
        }
        // Computed without the lock: others may compute the same result
        // meanwhile, which only costs time.
        let value = compute()?;
        let mut entries = self.lock();
        if entries.len >= LIMIT {
            entries.by_fingerprint.clear();
            entries.len = 0;
        }
        entries
            .by_fingerprint
            .entry(fingerprint)
            // Fingerprints rarely collide: room for one result, not the
            // four an empty vector first makes room for.
            .or_insert_with(|| Vec::with_capacity(1))
            .push((key(), value.clone()));
        entries.len += 1;
        Ok(value)
    }

    /// Forgets every result.
    pub(crate) fn clear(&self) {
        let mut entries = self.lock();
        entries.by_fingerprint.clear();
        entries.len = 0;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Entries<K, V>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A fingerprint of a font request (its families, style, size, generic
/// lists and rendering options) and of `extra`.
pub(crate) fn fingerprint(request: &FontRequest, extra: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    let query = &request.query;
    query.families.hash(&mut hasher);
    (query.style, query.variant, query.weight, query.stretch).hash(&mut hasher);
    query.size.to_bits().hash(&mut hasher);
    request.generics.hash(&mut hasher);
    (request.last_resort, request.hinting).hash(&mut hasher);
    extra.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_are_computed_once_and_failures_never_stored() {
        let memo: Memo<String, usize> = Memo::new();
        let calls = std::cell::Cell::new(0);
        let compute = |text: &str| {
            calls.set(calls.get() + 1);
            Ok::<_, ()>(text.len())
        };
        for _ in 0..3 {
            let len =
                memo.get_or_compute(7, |k| k == "abc", || "abc".to_owned(), || compute("abc"));
            assert_eq!(len, Ok(3));
        }
        assert_eq!(calls.get(), 1);
        // A shared fingerprint still tells the inputs apart.
        let other = memo.get_or_compute(7, |k| k == "xy", || "xy".to_owned(), || compute("xy"));
        assert_eq!((other, calls.get()), (Ok(2), 2));
        let failed = memo.get_or_compute(
            8,
            |k| k == "e",
            || "e".to_owned(),
            || Err::<usize, _>("bad"),
        );
        assert_eq!(failed, Err("bad"));
        let retried = memo.get_or_compute(8, |k| k == "e", || "e".to_owned(), || Ok::<_, &str>(1));
        assert_eq!(retried, Ok(1), "the failure was not kept");
        memo.clear();
        let recomputed =
            memo.get_or_compute(7, |k| k == "abc", || "abc".to_owned(), || compute("abc"));
        assert_eq!((recomputed, calls.get()), (Ok(3), 3), "cleared");
    }
}
