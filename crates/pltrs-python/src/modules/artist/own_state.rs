//! Artists whose Python objects keep state of theirs (`_own_state`): an
//! object asks to be told before native calls reach its artist, and the
//! package touches an artist to have its object told now.

use pltrs::artist::RawHandle;

use crate::binding::prelude::*;

/// Has `callable` called, without arguments, before every native call of
/// a thread's own (not one made inside another) that takes the native
/// object `native` as an argument; `None` ends it. The calls `callable`
/// makes meanwhile make no artist stale and tell nobody.
fn tell_before_calls(native: Owned, callable: Option<Owned>) -> PyResult<()> {
    Python::with_gil(|py| crate::binding::set_asker(&native.bind(py), callable))
}

/// Nothing: a call taking the artist, before which its object is told.
fn touch(_artist: RawHandle) -> PyResult<()> {
    Ok(())
}

/// The native functions before which no artist is told, each as its
/// module's name, its own and what it reads or writes
/// (`binding::PASSING_BY`): what the sweep of changes in place makes its
/// rows of.
fn passing_by() -> PyResult<Vec<Vec<&'static str>>> {
    Ok(crate::binding::PASSING_BY
        .iter()
        .map(|&(module, name, what)| vec![module, name, what])
        .collect())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("tell_before_calls", tell_before_calls)?;
    module.add("touch", touch)?;
    module.add("passing_by", passing_by)
}
