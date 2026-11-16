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

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("tell_before_calls", tell_before_calls)?;
    module.add("touch", touch)
}
