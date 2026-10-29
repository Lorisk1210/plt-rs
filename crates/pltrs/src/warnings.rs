//! Warnings: problems pltrs reports and carries on from while it works,
//! where matplotlib calls `_api.warn_external` (data cast to a narrower
//! type to be drawn, say).
//!
//! Warnings go to the handler of the thread, which a program installs
//! with [`set_handler`] (pltrs (Python) turns them into Python warnings);
//! without one they are written to standard error, as Python prints
//! warnings nobody filters.
//!
//! ```
//! use std::cell::RefCell;
//! use std::rc::Rc;
//! use pltrs::warnings;
//! let seen = Rc::new(RefCell::new(Vec::new()));
//! let sink = Rc::clone(&seen);
//! warnings::set_handler(move |message| sink.borrow_mut().push(message.to_owned()));
//! warnings::warn("Casting input data from float128 to float64 for imshow.");
//! warnings::clear_handler();
//! assert_eq!(seen.borrow().len(), 1);
//! ```

use std::cell::RefCell;
use std::rc::Rc;

/// A function receiving the warnings of its thread.
type Handler = Rc<dyn Fn(&str)>;

thread_local! {
    /// The thread's handler; `None` writes warnings to standard error.
    static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
}

/// Sends the warnings of this thread to `handler` from now on.
pub fn set_handler(handler: impl Fn(&str) + 'static) {
    HANDLER.with(|slot| *slot.borrow_mut() = Some(Rc::new(handler)));
}

/// Writes the warnings of this thread to standard error again.
pub fn clear_handler() {
    HANDLER.with(|slot| *slot.borrow_mut() = None);
}

/// Issues the warning `message` to the thread's handler.
#[expect(
    clippy::print_stderr,
    reason = "without a handler, warnings go where Python prints unfiltered ones"
)]
pub fn warn(message: &str) {
    // The handler runs on a copy, so that one installing another handler
    // or warning again does not borrow the slot twice.
    match HANDLER.with(|slot| slot.borrow().clone()) {
        Some(handler) => handler(message),
        None => eprintln!("pltrs warning: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handlers_receive_warnings_until_cleared() {
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let sink = Rc::clone(&seen);
        set_handler(move |message| sink.borrow_mut().push(message.to_owned()));
        warn("first");
        warn("second");
        clear_handler();
        assert_eq!(*seen.borrow(), ["first", "second"]);
    }

    #[test]
    fn a_handler_may_warn_again() {
        let count = Rc::new(RefCell::new(0));
        let counter = Rc::clone(&count);
        set_handler(move |message| {
            *counter.borrow_mut() += 1;
            if message == "outer" {
                warn("inner");
            }
        });
        warn("outer");
        clear_handler();
        assert_eq!(*count.borrow(), 2);
    }
}
