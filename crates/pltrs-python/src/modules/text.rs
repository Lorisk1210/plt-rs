//! `_native.text`: texts (matplotlib's `text`: titles, labels,
//! annotations).

use pltrs::text::Text;

use crate::binding::prelude::*;

artist_handle!(Text);

/// The string shown.
fn get_text(text: Text) -> PyResult<String> {
    Ok(text.text())
}

/// Sets the string shown.
fn set_text(text: Text, value: String) -> PyResult<()> {
    text.set_text(value);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_text", get_text)?;
    module.add("set_text", set_text)
}
