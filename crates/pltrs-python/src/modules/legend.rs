//! `_native.legend`: legends (matplotlib's `legend`).

use pltrs::legend::Legend;
use pltrs::text::Text;

use crate::binding::prelude::*;

artist_handle!(Legend);

/// The legend's entry texts.
fn get_texts(legend: Legend) -> PyResult<Vec<Text>> {
    Ok(legend.texts())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_texts", get_texts)
}
