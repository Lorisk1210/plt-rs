//! The state pltrs keeps where matplotlib keeps process-wide variables
//! (the rc parameters, the registries of colormaps and projections, the
//! handlers of warnings, ...): each thread has its own, unless threads
//! work with one [`State`] together.
//!
//! A crate declares such a variable with
//! [`thread_state!`](crate::thread_state) as it would declare a
//! `thread_local!`, and reaches it through its [`StateKey`]. A thread
//! works with its own values (thread-local, made the first time the
//! thread reaches them) until [`set_current`] makes it work with a
//! `State`, which holds a value of every variable reached through it,
//! made the first time it is reached there:
//!
//! ```
//! use std::cell::Cell;
//! use pltrs_core::state::{self, State};
//! pltrs_core::thread_state! {
//!     /// A setting, per thread unless shared.
//!     static LEVEL: Cell<u32> = const { Cell::new(1) };
//! }
//! LEVEL.set(2);
//! let shared = State::new();
//! let own = state::set_current(Some(shared));
//! assert_eq!(LEVEL.get(), 1, "the state's value starts afresh");
//! LEVEL.set(3);
//! state::set_current(own);
//! assert_eq!(LEVEL.get(), 2, "the thread's own value");
//! ```
//!
//! A `State` is neither `Send` nor `Sync`: its values are those of
//! single-threaded code (`Rc`s, `RefCell`s). Threads can only work with
//! one together under a guarantee the compiler cannot see, that they
//! take turns: pltrs (Python)'s threads share one while they hold
//! Python's GIL, as matplotlib's globals are shared.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::LocalKey;

thread_local! {
    /// The state the thread works with; `None` for its own values.
    static CURRENT: Cell<Option<&'static State>> = const { Cell::new(None) };
}

/// The number the next variable reached in a state gets: its place there.
static NEXT_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Makes the calling thread work with `state` (its own values with
/// `None`), returning what it worked with before.
pub fn set_current(state: Option<&'static State>) -> Option<&'static State> {
    CURRENT.replace(state)
}

/// The values of the variables of [`thread_state!`](crate::thread_state)
/// that threads working with it see, in place of their own.
pub struct State {
    /// Each variable's value, by its index, once reached.
    values: RefCell<Vec<Option<&'static dyn Any>>>,
}

impl State {
    /// A new state, whose variables start from their initial values. It
    /// lives as long as the program, as the variables it stands for do.
    pub fn new() -> &'static Self {
        Box::leak(Box::new(Self {
            values: RefCell::new(Vec::new()),
        }))
    }

    /// The value of the variable of `key`, made when first reached.
    fn value<T: 'static>(&self, key: &StateKey<T>) -> &'static T {
        let index = key.index();
        let made = self.values.borrow().get(index).copied().flatten();
        let value = made.unwrap_or_else(|| {
            // The value is made without the values borrowed, since making
            // it may reach other variables.
            let fresh: &'static dyn Any = Box::leak(Box::new((key.init)()));
            let mut values = self.values.borrow_mut();
            if values.len() <= index {
                values.resize(index + 1, None);
            }
            *values[index].get_or_insert(fresh)
        });
        value
            .downcast_ref()
            .expect("a state keeps a variable's value at its key's index")
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reached = self.values.borrow().iter().flatten().count();
        f.debug_struct("State").field("reached", &reached).finish()
    }
}

/// A thread's last lookup of a variable in a state: the state and the
/// value there (the variables of [`thread_state!`](crate::thread_state)
/// keep one each).
#[doc(hidden)]
pub type LastLookup<T> = Cell<Option<(&'static State, &'static T)>>;

/// A variable of [`thread_state!`](crate::thread_state): the thread's own
/// value, or the one of the [`State`] it works with.
pub struct StateKey<T: 'static> {
    /// The thread's own value.
    own: &'static LocalKey<T>,
    /// The thread's last lookup in a state: values never move, so looking
    /// again in the same state finds the same.
    last: &'static LocalKey<LastLookup<T>>,
    /// The initial value, for states.
    init: fn() -> T,
    /// The variable's place in states, given when first reached in one.
    index: OnceLock<usize>,
}

impl<T: 'static> StateKey<T> {
    /// The variable whose own values `own` holds, whose last lookups in
    /// states `last` keeps, and whose values in states start as `init`
    /// makes them: what [`thread_state!`](crate::thread_state) expands to.
    #[doc(hidden)]
    pub const fn new(
        own: &'static LocalKey<T>,
        last: &'static LocalKey<LastLookup<T>>,
        init: fn() -> T,
    ) -> Self {
        Self {
            own,
            last,
            init,
            index: OnceLock::new(),
        }
    }

    /// The variable's place in states.
    fn index(&self) -> usize {
        *self
            .index
            .get_or_init(|| NEXT_INDEX.fetch_add(1, Ordering::Relaxed))
    }

    /// Runs `f` with the value the thread works with (as
    /// [`LocalKey::with`]).
    pub fn with<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
        let Some(state) = CURRENT.get() else {
            return self.own.with(f);
        };
        let value = match self.last.get() {
            Some((last, value)) if std::ptr::eq(last, state) => value,
            _ => {
                let value = state.value(self);
                self.last.set(Some((state, value)));
                value
            }
        };
        f(value)
    }
}

impl<T: 'static> StateKey<RefCell<T>> {
    /// Runs `f` with the value borrowed (as [`LocalKey::with_borrow`]).
    pub fn with_borrow<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
        self.with(|cell| f(&cell.borrow()))
    }

    /// Runs `f` with the value borrowed mutably (as
    /// [`LocalKey::with_borrow_mut`]).
    pub fn with_borrow_mut<R>(&'static self, f: impl FnOnce(&mut T) -> R) -> R {
        self.with(|cell| f(&mut cell.borrow_mut()))
    }

    /// Replaces the value, returning the previous one (as
    /// [`LocalKey::replace`]).
    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }

    /// Sets the value (as [`LocalKey::set`]), dropping the previous one
    /// once the cell is no longer borrowed.
    pub fn set(&'static self, value: T) {
        drop(self.replace(value));
    }

    /// Takes the value, leaving the default (as [`LocalKey::take`]).
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.replace(T::default())
    }
}

impl<T: Copy + 'static> StateKey<Cell<T>> {
    /// The value (as [`LocalKey::get`]).
    pub fn get(&'static self) -> T {
        self.with(Cell::get)
    }

    /// Sets the value (as [`LocalKey::set`]).
    pub fn set(&'static self, value: T) {
        self.with(|cell| cell.set(value));
    }

    /// Replaces the value, returning the previous one (as
    /// [`LocalKey::replace`]).
    pub fn replace(&'static self, value: T) -> T {
        self.with(|cell| cell.replace(value))
    }
}

impl<T: 'static> fmt::Debug for StateKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateKey").finish_non_exhaustive()
    }
}

/// Declares variables of pltrs's [state](crate::state): per thread, as
/// with `thread_local!` (whose syntax it takes, `const { ... }`
/// initialisers included), unless the thread works with a shared
/// [`State`](crate::state::State).
///
/// ```
/// use std::cell::RefCell;
/// pltrs_core::thread_state! {
///     /// The names registered so far.
///     static NAMES: RefCell<Vec<String>> = RefCell::new(Vec::new());
/// }
/// NAMES.with_borrow_mut(|names| names.push("viridis".to_owned()));
/// assert_eq!(NAMES.with_borrow(Vec::len), 1);
/// ```
#[macro_export]
macro_rules! thread_state {
    () => {};
    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = const { $init:expr }; $($rest:tt)*) => {
        $(#[$attr])*
        $vis static $name: $crate::state::StateKey<$ty> = {
            ::std::thread_local! {
                static OWN: $ty = const { $init };
                static LAST: $crate::state::LastLookup<$ty> = const { ::std::cell::Cell::new(None) };
            }
            fn init() -> $ty {
                $init
            }
            $crate::state::StateKey::new(&OWN, &LAST, init)
        };
        $crate::thread_state!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident: $ty:ty = $init:expr; $($rest:tt)*) => {
        $(#[$attr])*
        $vis static $name: $crate::state::StateKey<$ty> = {
            ::std::thread_local! {
                static OWN: $ty = $init;
                static LAST: $crate::state::LastLookup<$ty> = const { ::std::cell::Cell::new(None) };
            }
            fn init() -> $ty {
                $init
            }
            $crate::state::StateKey::new(&OWN, &LAST, init)
        };
        $crate::thread_state!($($rest)*);
    };
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    crate::thread_state! {
        /// A counter.
        static COUNT: Cell<u32> = const { Cell::new(0) };
        /// Names, made lazily.
        static NAMES: RefCell<Vec<String>> = RefCell::new(vec!["first".to_owned()]);
        /// A value made from another variable.
        static DOUBLED: u32 = COUNT.get() * 2;
    }

    /// Runs `f` working with `state`, then with the thread's own values
    /// again.
    fn working_with<R>(state: &'static State, f: impl FnOnce() -> R) -> R {
        let own = set_current(Some(state));
        let result = f();
        set_current(own);
        result
    }

    #[test]
    fn threads_work_with_their_own_values() {
        COUNT.set(5);
        let elsewhere = std::thread::spawn(|| COUNT.get()).join().expect("joins");
        assert_eq!((COUNT.get(), elsewhere), (5, 0));
    }

    #[test]
    fn a_state_keeps_values_apart_from_the_thread() {
        NAMES.with_borrow_mut(|names| names.push("own".to_owned()));
        let state = State::new();
        working_with(state, || {
            assert_eq!(NAMES.with_borrow(Vec::len), 1, "made afresh");
            NAMES.with_borrow_mut(|names| names.push("shared".to_owned()));
        });
        assert_eq!(NAMES.take(), ["first", "own"]);
        let shared = working_with(state, || NAMES.replace(Vec::new()));
        assert_eq!(shared, ["first", "shared"], "kept by the state");
    }

    #[test]
    fn each_state_keeps_its_own_values() {
        let (one, two) = (State::new(), State::new());
        working_with(one, || COUNT.set(1));
        working_with(two, || COUNT.set(2));
        let seen = [one, two, one].map(|state| working_with(state, || COUNT.get()));
        assert_eq!(seen, [1, 2, 1]);
    }

    #[test]
    fn values_made_in_a_state_may_reach_other_variables() {
        let state = State::new();
        let doubled = working_with(state, || {
            COUNT.set(21);
            DOUBLED.with(|value| *value)
        });
        assert_eq!(doubled, 42);
        assert!(format!("{state:?}").contains("reached: 2"), "{state:?}");
    }

    #[test]
    fn states_hold_values_that_are_not_send() {
        crate::thread_state! {
            static SHARED: RefCell<Option<Rc<str>>> = const { RefCell::new(None) };
        }
        let state = State::new();
        working_with(state, || SHARED.set(Some(Rc::from("kept"))));
        let kept = working_with(state, || SHARED.with_borrow(Clone::clone));
        assert_eq!(kept.as_deref(), Some("kept"));
        assert!(SHARED.with_borrow(Option::is_none), "not the thread's own");
    }
}
