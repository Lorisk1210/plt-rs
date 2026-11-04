//! Warnings: problems pltrs reports and carries on from while it works,
//! where matplotlib calls `_api.warn_external` (data cast to a narrower
//! type to be drawn, say).
//!
//! Warnings go to the handler of the thread (of the threads sharing a
//! [state](pltrs_core::state)), which a program installs with
//! [`set_handler`] (pltrs (Python) turns them into Python warnings);
//! without one they are written to standard error, as Python prints
//! warnings nobody filters.
//!
//! Where matplotlib logs a warning instead (`_log.warning`), pltrs
//! [`log`]s it, naming the module of matplotlib's that logs it: the
//! thread's log handler ([`set_log_handler`]) receives it, and without one
//! it is written to standard error, as Python's unconfigured logging
//! writes it.
//!
//! Where matplotlib's arithmetic meets invalid values, numpy warns
//! (`RuntimeWarning: divide by zero encountered in log10`): pltrs, doing
//! that arithmetic itself, issues the same warning in the same cases with
//! [`warn_runtime`], to the thread's runtime handler
//! ([`set_runtime_handler`]), or to standard error without one.
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

/// A function receiving the log messages of its thread: the module of
/// matplotlib's logging it (`backends.backend_ps`) and the message.
type LogHandler = Rc<dyn Fn(&str, &str)>;

pltrs_core::thread_state! {
    /// The thread's handler; `None` writes warnings to standard error.
    static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
    /// The thread's log handler; `None` writes messages to standard error.
    static LOG_HANDLER: RefCell<Option<LogHandler>> = const { RefCell::new(None) };
    /// The thread's handler of runtime warnings; `None` writes them to
    /// standard error.
    static RUNTIME_HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
}

/// Sends the warnings of this thread to `handler` from now on.
pub fn set_handler(handler: impl Fn(&str) + 'static) {
    HANDLER.set(Some(Rc::new(handler)));
}

/// Writes the warnings of this thread to standard error again.
pub fn clear_handler() {
    HANDLER.set(None);
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

/// Sends the runtime warnings of this thread to `handler` from now on.
pub fn set_runtime_handler(handler: impl Fn(&str) + 'static) {
    RUNTIME_HANDLER.set(Some(Rc::new(handler)));
}

/// Writes the runtime warnings of this thread to standard error again.
pub fn clear_runtime_handler() {
    RUNTIME_HANDLER.set(None);
}

/// Issues the runtime warning `message` (numpy's words for the invalid
/// arithmetic matplotlib's code would meet: `"invalid value encountered
/// in log10"`) to the thread's runtime handler, or keeps it for the work
/// pltrs runs that issues its warnings where numpy issues them (with the
/// error the arithmetic leads to, say).
#[expect(
    clippy::print_stderr,
    reason = "without a handler, warnings go where Python prints unfiltered ones"
)]
pub fn warn_runtime(message: &str) {
    let kept = CAPTURED.with_borrow_mut(|captured| {
        captured
            .as_mut()
            .map(|captured| captured.push(message.to_owned()))
            .is_some()
    });
    if kept {
        return;
    }
    match RUNTIME_HANDLER.with(|slot| slot.borrow().clone()) {
        Some(handler) => handler(message),
        None => eprintln!("RuntimeWarning: {message}"),
    }
}

thread_local! {
    /// The runtime warnings of the work [`capturing_runtime`] runs, kept
    /// instead of issued.
    static CAPTURED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Runs `work`, keeping the runtime warnings it issues instead of issuing
/// them: what `work` returns, and the warnings in order, for the caller
/// to issue where numpy issues them (with the error the arithmetic that
/// warned leads to, say).
pub(crate) fn capturing_runtime<R>(work: impl FnOnce() -> R) -> (R, Vec<String>) {
    /// Puts back the warnings kept around this work, also when it unwinds.
    struct Restore(Option<Vec<String>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CAPTURED.set(self.0.take());
        }
    }
    let restore = Restore(CAPTURED.replace(Some(Vec::new())));
    let result = work();
    let captured = CAPTURED.take().unwrap_or_default();
    drop(restore);
    (result, captured)
}

/// Sends the log messages of this thread to `handler` from now on.
pub fn set_log_handler(handler: impl Fn(&str, &str) + 'static) {
    LOG_HANDLER.set(Some(Rc::new(handler)));
}

/// Writes the log messages of this thread to standard error again.
pub fn clear_log_handler() {
    LOG_HANDLER.set(None);
}

/// Logs `message` as a warning of matplotlib's module `module` (relative
/// to the package: `backends.backend_ps`), where matplotlib logs it with
/// `_log.warning`.
#[expect(
    clippy::print_stderr,
    reason = "without a handler, messages go where Python's unconfigured logging writes them"
)]
pub fn log(module: &str, message: &str) {
    match LOG_HANDLER.with(|slot| slot.borrow().clone()) {
        Some(handler) => handler(module, message),
        None => eprintln!("{message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_runtime_warnings_are_kept_and_nest() {
        let (inner, outer) = capturing_runtime(|| {
            warn_runtime("outer");
            let ((), inner) = capturing_runtime(|| warn_runtime("inner"));
            warn_runtime("outer again");
            inner
        });
        assert_eq!(inner, ["inner"]);
        assert_eq!(outer, ["outer", "outer again"]);
    }

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
    fn runtime_handlers_receive_runtime_warnings_only() {
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let sink = Rc::clone(&seen);
        set_runtime_handler(move |message| sink.borrow_mut().push(message.to_owned()));
        warn_runtime("divide by zero encountered in log10");
        clear_runtime_handler();
        assert_eq!(*seen.borrow(), ["divide by zero encountered in log10"]);
    }

    #[test]
    fn log_handlers_receive_the_module_and_message() {
        let seen = Rc::new(RefCell::new(Vec::<(String, String)>::new()));
        let sink = Rc::clone(&seen);
        set_log_handler(move |module, message| {
            sink.borrow_mut()
                .push((module.to_owned(), message.to_owned()));
        });
        log("backends.backend_ps", "as if usetex=False");
        clear_log_handler();
        assert_eq!(
            *seen.borrow(),
            [(
                "backends.backend_ps".to_owned(),
                "as if usetex=False".to_owned()
            )]
        );
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
