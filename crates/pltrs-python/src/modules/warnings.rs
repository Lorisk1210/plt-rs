//! The warnings pltrs (Rust) issues while it works (`pltrs::warnings`),
//! issued as matplotlib issues its own: through `_api.warn_external`,
//! which the package registers (`_native.register("warn_external", ...)`).
//!
//! A warning that the warning filters turn into an exception raises from
//! the native call that issued it, as an exception of a callback does
//! (see `binding::callback`).

use crate::binding::lookup;
use crate::binding::prelude::*;

/// Sends pltrs (Rust)'s warnings to `_api.warn_external` from now on.
pub(super) fn register(_module: &Module<'_>) -> PyResult<()> {
    pltrs::warnings::set_handler(warn);
    Ok(())
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
