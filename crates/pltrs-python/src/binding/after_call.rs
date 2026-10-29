//! Work left to the end of each native call: functions the matplotlib
//! modules install to deliver what pltrs (Rust) queued while the call
//! ran, such as the warnings matplotlib issues while it lays text out.

use std::sync::OnceLock;

use super::error::PyResult;
use super::gil::Python;

/// A function run when a native call returns successfully.
pub(crate) type AfterCall = for<'py> fn(Python<'py>) -> PyResult<()>;

/// The functions installed, once, when the extension module is made.
static HOOKS: OnceLock<Vec<AfterCall>> = OnceLock::new();

/// Installs the functions run at the end of every native call; only the
/// first installation counts (the extension module is made once).
pub(crate) fn install(hooks: Vec<AfterCall>) {
    // A second installation would come from a second module definition,
    // which runs the same registration: the first one is as good.
    let _ = HOOKS.set(hooks);
}

/// Runs the installed functions in order; the first error stops them and
/// is raised in place of the call's result.
pub(crate) fn run(py: Python<'_>) -> PyResult<()> {
    HOOKS
        .get()
        .map_or(Ok(()), |hooks| hooks.iter().try_for_each(|hook| hook(py)))
}
