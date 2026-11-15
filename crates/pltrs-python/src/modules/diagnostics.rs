//! `_native.diagnostics`: what pltrs keeps alive in the process, for the
//! leak harness (`python/tests/leaks`): native objects Python holds,
//! figure states and the removed artists figures keep (see
//! `pltrs::diagnostics`).

use std::collections::BTreeMap;

use crate::binding::prelude::*;

/// The counts by name: `native_objects`, `figure_states` and
/// `removed_artists`.
fn live_counts() -> PyResult<BTreeMap<String, usize>> {
    Ok(BTreeMap::from([
        ("native_objects".to_owned(), crate::binding::live_objects()),
        (
            "figure_states".to_owned(),
            pltrs::diagnostics::live_figure_states(),
        ),
        (
            "removed_artists".to_owned(),
            pltrs::diagnostics::kept_removed_artists(),
        ),
    ]))
}

pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("live_counts", live_counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::with_python;
    use crate::binding::wrap_native;

    #[test]
    fn native_objects_count_while_python_holds_them() {
        with_python(|py| {
            let object = wrap_native(py, 7_u8).unwrap();
            let held = live_counts().unwrap()["native_objects"];
            assert!(held >= 1);
            drop(object);
            assert_eq!(
                live_counts().unwrap().into_keys().collect::<Vec<_>>(),
                ["figure_states", "native_objects", "removed_artists"]
            );
        });
    }
}
