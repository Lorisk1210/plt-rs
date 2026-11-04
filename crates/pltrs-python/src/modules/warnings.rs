//! The warnings pltrs (Rust) issues while it works (`pltrs::warnings`),
//! issued as matplotlib issues its own: through `_api.warn_external`,
//! which the package registers (`_native.register("warn_external", ...)`),
//! and the warnings it logs, through the package's `log_warning`, which
//! logs them with the logger of the module of matplotlib's that logs them.
//!
//! The runtime warnings of the arithmetic pltrs (Rust) does where
//! matplotlib's numpy code warns (`warn_runtime`) are issued as numpy
//! issues them, as `RuntimeWarning`s.
//!
//! A warning that the warning filters turn into an exception raises from
//! the native call that issued it, as an exception of a callback does
//! (see `binding::callback`).

use crate::binding::prelude::*;
use crate::binding::{lookup, report, runtime_warning};

/// Sends pltrs (Rust)'s warnings to `_api.warn_external` and its log
/// messages to the package's `log_warning` from now on.
pub(super) fn register(_module: &Module<'_>) -> PyResult<()> {
    pltrs::warnings::set_handler(warn);
    pltrs::warnings::set_log_handler(log);
    pltrs::warnings::set_runtime_handler(warn_runtime);
    Ok(())
}

/// Issues `message` as a `RuntimeWarning`, as numpy does; one the warning
/// filters turn into an exception raises from the native call running.
fn warn_runtime(message: &str) {
    Python::with_gil(|py| {
        if let Err(err) = runtime_warning(py, message) {
            report(err);
        }
    });
}

/// Logs `message` as a warning of the package's module `module` through
/// the registered `log_warning`; before the package registered it, to
/// standard error.
#[expect(
    clippy::print_stderr,
    reason = "before the package registers log_warning, messages go to standard error"
)]
fn log(module: &str, message: &str) {
    let log_warning =
        Python::with_gil(|py| lookup(py, "log_warning").and_then(|log| log.extract::<Callback>()));
    match log_warning {
        Ok(log_warning) => {
            // An exception goes to the native call running, which raises it.
            let _: Option<Owned> = log_warning.call((module.to_owned(), message.to_owned()));
        }
        Err(_) => eprintln!("{message}"),
    }
}

/// Issues `message` through the registered `warn_external`; before the
/// package registered it, to standard error.
#[expect(
    clippy::print_stderr,
    reason = "before the package registers warn_external, warnings go to standard error"
)]
fn warn(message: &str) {
    let warn_external = Python::with_gil(|py| {
        lookup(py, "warn_external").and_then(|warn| warn.extract::<Callback>())
    });
    match warn_external {
        Ok(warn_external) => {
            // An exception goes to the native call running, which raises it.
            let _: Option<Owned> = warn_external.call((message.to_owned(),));
        }
        Err(_) => eprintln!("pltrs warning: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, str, with_python};

    #[test]
    fn warnings_go_to_the_registered_function() {
        with_python(|py| {
            let seen = eval(py, "seen = []", "seen");
            let record = seen.getattr("append").unwrap();
            extension(py)
                .getattr("register")
                .and_then(|register| register.call(&[str(py, "warn_external"), record], &[]))
                .unwrap();
            pltrs::warnings::set_handler(warn);
            pltrs::warnings::warn("Casting input data from float128 to float64 for imshow.");
            pltrs::warnings::clear_handler();
            let count: usize = seen.call_method("__len__", &[]).unwrap().extract().unwrap();
            assert_eq!(count, 1);
        });
    }
}
