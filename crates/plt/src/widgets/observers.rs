//! The callbacks of a widget signal and the ids that disconnect them
//! (matplotlib's widget `_observers`, a `CallbackRegistry`).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Identifies a callback connected to a widget with one of its `on_*`
/// methods, for the widget's `disconnect` (matplotlib's `cid`).
///
/// Ids are unique across widgets, so an id only ever disconnects the
/// callback it was returned for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId(u64);

impl CallbackId {
    /// A fresh id.
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// A callback, shared so that it can run while the list is changed.
type Callback<T> = Rc<RefCell<dyn FnMut(&T)>>;

/// The connected callbacks with their ids, in connection order.
type CallbackList<T> = Vec<(CallbackId, Callback<T>)>;

/// Callbacks taking a value of type `T`, shared between a widget and its
/// event handlers.
pub(super) struct Observers<T> {
    callbacks: Rc<RefCell<CallbackList<T>>>,
}

impl<T> Clone for Observers<T> {
    fn clone(&self) -> Self {
        Self {
            callbacks: Rc::clone(&self.callbacks),
        }
    }
}

impl<T> Default for Observers<T> {
    fn default() -> Self {
        Self {
            callbacks: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl<T> fmt::Debug for Observers<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Observers")
            .field("connected", &self.callbacks.borrow().len())
            .finish()
    }
}

impl<T> Observers<T> {
    /// Adds `callback`; returns its id.
    pub(super) fn connect(&self, callback: impl FnMut(&T) + 'static) -> CallbackId {
        let id = CallbackId::next();
        let callback: Callback<T> = Rc::new(RefCell::new(callback));
        self.callbacks.borrow_mut().push((id, callback));
        id
    }

    /// Removes callback `id`; returns whether it was connected here.
    pub(super) fn disconnect(&self, id: CallbackId) -> bool {
        let mut callbacks = self.callbacks.borrow_mut();
        let before = callbacks.len();
        callbacks.retain(|(connected, _)| *connected != id);
        callbacks.len() != before
    }

    /// Calls the connected callbacks with `value`, in connection order.
    /// Callbacks connected while this runs wait for the next call, and a
    /// callback that is already running (a value processed from inside
    /// itself) is skipped.
    pub(super) fn process(&self, value: &T) {
        let running: Vec<Callback<T>> = self
            .callbacks
            .borrow()
            .iter()
            .map(|(_, callback)| Rc::clone(callback))
            .collect();
        for callback in running {
            if let Ok(mut callback) = callback.try_borrow_mut() {
                callback(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn disconnected_callbacks_stop_running() {
        let observers = Observers::<i32>::default();
        let sum = Rc::new(Cell::new(0));
        let (a, b) = (Rc::clone(&sum), Rc::clone(&sum));
        let first = observers.connect(move |v| a.set(a.get() + v));
        let second = observers.connect(move |v| b.set(b.get() + 10 * v));
        assert_ne!(first, second);
        observers.process(&1);
        assert_eq!(sum.get(), 11);
        assert!(observers.disconnect(first));
        assert!(!observers.disconnect(first), "already disconnected");
        observers.process(&1);
        assert_eq!(sum.get(), 21);
    }

    #[test]
    fn callbacks_may_disconnect_while_running() {
        let observers = Observers::<()>::default();
        let calls = Rc::new(Cell::new(0));
        let id = Rc::new(Cell::new(None));
        let (inner, counter, own) = (observers.clone(), Rc::clone(&calls), Rc::clone(&id));
        id.set(Some(observers.connect(move |()| {
            counter.set(counter.get() + 1);
            if let Some(own) = own.get() {
                inner.disconnect(own);
            }
        })));
        observers.process(&());
        observers.process(&());
        assert_eq!(calls.get(), 1, "the callback disconnected itself");
    }
}
