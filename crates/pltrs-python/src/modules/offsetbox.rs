//! `_native.offsetbox`: the packing rules of matplotlib's `offsetbox`
//! (`_get_packed_offsets`, `_get_aligned_offsets`), by pltrs (Rust)'s
//! `offsetbox` module.

use pltrs::offsetbox::{PackAlign, PackMode, aligned_offsets, packed_offsets};

use crate::binding::prelude::*;

mod add;
mod arrow;
mod edit;
mod program;
mod tree;

/// The packing mode called `name` (`"fixed"`, `"expand"`, `"equal"`).
pub(super) fn pack_mode(name: &str) -> PyResult<PackMode> {
    match name {
        "fixed" => Ok(PackMode::Fixed),
        "expand" => Ok(PackMode::Expand),
        "equal" => Ok(PackMode::Equal),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for mode; supported values are 'fixed', 'expand', \
             'equal'"
        ))),
    }
}

/// The alignment called `name` (`"baseline"`, `"left"`, `"top"`,
/// `"right"`, `"bottom"`, `"center"`).
pub(super) fn pack_align(name: &str) -> PyResult<PackAlign> {
    match name {
        "baseline" => Ok(PackAlign::Baseline),
        "left" => Ok(PackAlign::Left),
        "top" => Ok(PackAlign::Top),
        "right" => Ok(PackAlign::Right),
        "bottom" => Ok(PackAlign::Bottom),
        "center" => Ok(PackAlign::Center),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for align; supported values are 'baseline', 'left', \
             'top', 'right', 'bottom', 'center'"
        ))),
    }
}

/// The total length and the offsets of boxes of `widths` packed with
/// `mode` (`_get_packed_offsets`); `sep` is only read where the mode uses
/// it.
fn packed(
    widths: Vec<f64>,
    total: Option<f64>,
    sep: f64,
    mode: String,
) -> PyResult<(f64, NdArray)> {
    let (total, offsets) = packed_offsets(&widths, total, sep, pack_mode(&mode)?);
    Ok((total, NdArray::from_vec(offsets)))
}

/// The span covered by boxes spanning `yspans` aligned with `align`, and
/// their offsets (`_get_aligned_offsets`).
fn aligned(
    yspans: Vec<(f64, f64)>,
    height: Option<f64>,
    align: String,
) -> PyResult<((f64, f64), NdArray)> {
    let (span, offsets) = aligned_offsets(&yspans, height, pack_align(&align)?);
    Ok((span, NdArray::from_vec(offsets)))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("packed", packed)?;
    module.add("aligned", aligned)?;
    arrow::register(module)?;
    program::register(module)?;
    edit::register(module)?;
    add::register(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_select_matplotlibs_modes_and_alignments() {
        assert_eq!(pack_mode("expand").unwrap(), PackMode::Expand);
        assert_eq!(pack_align("center").unwrap(), PackAlign::Center);
        assert!(pack_mode("tight").is_err() && pack_align("middle").is_err());
    }
}
