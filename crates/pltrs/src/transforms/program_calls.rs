//! The program's maps called where matplotlib calls them, and pltrs'
//! own needs served from what they answered.
//!
//! A map of the program's (a function scale, a scale or transform class
//! of a host language) sees every call matplotlib makes to it: drawing
//! and measuring an axes call it once for every tick label measured or
//! drawn, twice for every tick line and gridline drawn (its path, then
//! its points), once for every spine measured or drawn. pltrs makes those
//! calls where matplotlib makes them (`axes::transform_sites`), with the
//! values matplotlib hands over, and lays out and draws the same parts its
//! own way meanwhile, needing the same values in other batches and at
//! other moments. That work runs [`quietly`]: a map of the program's
//! answers it from what the program answered for the same values since
//! the last layout began ([`forget`]), and is called only for a value it
//! never answered.
//!
//! The answers are kept per map (by its address, the map kept alive with
//! them) and per value (its bits), for the thread laying out and drawing.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::AxisMap;

/// How many values of one map are kept at most: a map answering more
/// (a program animating a large dataset) starts afresh.
const KEPT_PER_MAP: usize = 1 << 16;

/// What one map of the program's answered, by the bits of the value it
/// was given.
struct MapAnswers {
    /// The map, kept alive so that its address names it.
    map: Rc<dyn AxisMap>,
    values: HashMap<u64, f64>,
}

thread_local! {
    /// How many [`quietly`] scopes the thread is in.
    static QUIET: Cell<usize> = const { Cell::new(0) };
    /// The answers of the program's maps, by map address.
    static ANSWERS: RefCell<HashMap<usize, MapAnswers>> = RefCell::new(HashMap::new());
}

/// The address naming `map` among the answers.
fn address(map: &Rc<dyn AxisMap>) -> usize {
    Rc::as_ptr(map).cast::<()>().addr()
}

/// Restores the quiet depth it was made with when dropped.
struct QuietDepth(usize);

impl Drop for QuietDepth {
    fn drop(&mut self) {
        QUIET.set(self.0);
    }
}

/// Runs `work`, pltrs' own computation of values matplotlib computes
/// elsewhere, with the program's maps answering from what they answered
/// before (see the module documentation).
pub(crate) fn quietly<R>(work: impl FnOnce() -> R) -> R {
    let depth = QUIET.get();
    let _restore = QuietDepth(depth);
    QUIET.set(depth + 1);
    work()
}

/// Runs `work`, calls matplotlib makes, with the program's maps called
/// even inside a [`quietly`] scope.
pub(crate) fn observed<R>(work: impl FnOnce() -> R) -> R {
    let _restore = QuietDepth(QUIET.get());
    QUIET.set(0);
    work()
}

/// Forgets every answer, as a layout begins: what the program's maps
/// answer from then on is what that layout and the draw after it use.
pub(crate) fn forget() {
    // Dropped after the borrow ends: a map's drop may run the program's
    // code.
    let answers = ANSWERS.with_borrow_mut(std::mem::take);
    drop(answers);
}

/// The answers of `map` for every value of `values`, when it answered
/// each before.
fn known(map: &Rc<dyn AxisMap>, values: &[f64]) -> Option<Vec<f64>> {
    ANSWERS.with_borrow(|answers| {
        let kept = answers.get(&address(map))?;
        (values.iter())
            .map(|value| kept.values.get(&value.to_bits()).copied())
            .collect()
    })
}

/// Keeps what `map` answered: `mapped[i]` for `given[i]`.
fn keep(map: &Rc<dyn AxisMap>, given: &[f64], mapped: &[f64]) {
    ANSWERS.with_borrow_mut(|answers| {
        let kept = answers.entry(address(map)).or_insert_with(|| MapAnswers {
            map: Rc::clone(map),
            values: HashMap::new(),
        });
        if kept.values.len() + given.len() > KEPT_PER_MAP {
            kept.values.clear();
        }
        for (value, answer) in given.iter().zip(mapped) {
            kept.values.insert(value.to_bits(), *answer);
        }
        debug_assert!(
            std::ptr::addr_eq(Rc::as_ptr(&kept.map), Rc::as_ptr(map)),
            "the address names the map"
        );
    });
}

/// Maps every value of `values` through `map` in place, as
/// [`AxisMap::forward_all`] does: a map of the program's called once for
/// all of them, or answering from what it answered before while pltrs'
/// own work runs [`quietly`].
pub(crate) fn forward_all(map: &Rc<dyn AxisMap>, values: &mut [f64]) {
    if !map.calls_program() {
        map.forward_all(values);
        return;
    }
    if QUIET.get() > 0
        && let Some(answers) = known(map, values)
    {
        values.copy_from_slice(&answers);
        return;
    }
    let given = values.to_vec();
    map.forward_all(values);
    keep(map, &given, values);
}

/// One value through `map`, as [`AxisMap::forward`] maps it (see
/// [`forward_all`]).
pub(crate) fn forward(map: &Rc<dyn AxisMap>, value: f64) -> f64 {
    if !map.calls_program() {
        return map.forward(value);
    }
    if QUIET.get() > 0
        && let Some(answers) = known(map, &[value])
    {
        return answers[0];
    }
    let answer = map.forward(value);
    keep(map, &[value], &[answer]);
    answer
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    /// Doubles values, recording every call.
    #[derive(Debug, Default)]
    struct Counted {
        calls: RefCell<Vec<Vec<f64>>>,
    }

    impl AxisMap for Counted {
        fn forward(&self, value: f64) -> f64 {
            self.calls.borrow_mut().push(vec![value]);
            2.0 * value
        }

        fn inverse(&self, value: f64) -> f64 {
            value / 2.0
        }

        fn forward_all(&self, values: &mut [f64]) {
            self.calls.borrow_mut().push(values.to_vec());
            for v in values.iter_mut() {
                *v *= 2.0;
            }
        }

        fn calls_program(&self) -> bool {
            true
        }
    }

    fn counted() -> (Rc<Counted>, Rc<dyn AxisMap>) {
        let map = Rc::new(Counted::default());
        let shared: Rc<dyn AxisMap> = Rc::clone(&map) as Rc<dyn AxisMap>;
        (map, shared)
    }

    #[test]
    fn quiet_work_is_answered_from_earlier_calls() {
        forget();
        let (map, shared) = counted();
        let mut values = [1.0, 2.0];
        forward_all(&shared, &mut values);
        assert_eq!(values, [2.0, 4.0]);
        let quiet = quietly(|| {
            let mut again = [2.0, 1.0];
            forward_all(&shared, &mut again);
            (again, forward(&shared, 1.0))
        });
        assert_eq!(quiet, ([4.0, 2.0], 2.0));
        assert_eq!(
            *map.calls.borrow(),
            [vec![1.0, 2.0]],
            "answered, not called"
        );
        forget();
    }

    #[test]
    fn values_never_answered_and_observed_calls_call_the_program() {
        forget();
        let (map, shared) = counted();
        quietly(|| {
            assert_eq!(forward(&shared, 3.0), 6.0, "never answered");
            assert_eq!(forward(&shared, 3.0), 6.0, "answered since");
            observed(|| forward(&shared, 3.0));
        });
        assert_eq!(*map.calls.borrow(), [vec![3.0], vec![3.0]]);
        forget();
        quietly(|| forward(&shared, 3.0));
        assert_eq!(map.calls.borrow().len(), 3, "forgotten");
        forget();
    }
}
