//! The box a legend lays out its title and entries in (matplotlib's
//! `_legend_box`), described for Python, which makes `offsetbox` objects
//! of it around the legend's artists.
//!
//! A box crosses as a tuple naming its kind, then whether it is visible:
//! `("hpacker" | "vpacker", visible, (pad, sep, width, height), (align,
//! mode), children)`, `("text", visible, artist, multilinebaseline)` or
//! `("drawing", visible, (width, height, xdescent, ydescent), clip,
//! artists)`, each artist as its handle and its kind (see
//! [`artist_kind`]). A legend's box holds no other kind of box.

use pltrs::artist::RawHandle;
use pltrs::legend::Legend;
use pltrs::offsetbox::{OffsetBox, PackAlign, PackMode};

use super::artist_kind;
use crate::binding::prelude::*;

/// The name matplotlib gives `align`.
fn align_name(align: PackAlign) -> &'static str {
    match align {
        PackAlign::Baseline => "baseline",
        PackAlign::Left => "left",
        PackAlign::Top => "top",
        PackAlign::Right => "right",
        PackAlign::Bottom => "bottom",
        PackAlign::Center => "center",
    }
}

/// The name matplotlib gives `mode`.
fn mode_name(mode: PackMode) -> &'static str {
    match mode {
        PackMode::Fixed => "fixed",
        PackMode::Expand => "expand",
        PackMode::Equal => "equal",
    }
}

/// An artist of the box with its kind.
fn artist(raw: &RawHandle) -> (RawHandle, &'static str) {
    (raw.clone(), artist_kind(raw))
}

/// The description of `tree` (see the module documentation).
fn describe<'py>(py: Python<'py>, tree: &OffsetBox<RawHandle>) -> PyResult<Bound<'py>> {
    let visible = tree.is_visible();
    match tree {
        OffsetBox::HPacker(packer) | OffsetBox::VPacker(packer) => {
            let kind = if matches!(tree, OffsetBox::HPacker(_)) {
                "hpacker"
            } else {
                "vpacker"
            };
            let children = packer
                .children
                .iter()
                .map(|child| describe(py, child))
                .collect::<PyResult<Vec<_>>>()?;
            (
                kind,
                visible,
                (packer.pad, packer.sep, packer.width, packer.height),
                (align_name(packer.align), mode_name(packer.mode)),
                children,
            )
                .into_py(py)
        }
        OffsetBox::TextArea(area) => {
            ("text", visible, artist(&area.text), area.multilinebaseline).into_py(py)
        }
        OffsetBox::DrawingArea(area) => (
            "drawing",
            visible,
            (area.width, area.height, area.xdescent, area.ydescent),
            area.clip,
            area.artists.iter().map(artist).collect::<Vec<_>>(),
        )
            .into_py(py),
        OffsetBox::AuxTransformBox(_) | OffsetBox::Image(_) | OffsetBox::PaddedBox(_) => Err(
            PyErr::value_error("a legend's box holds packers, text areas and drawing areas"),
        ),
    }
}

/// The box `legend` lays out its title and entries in (see the module
/// documentation).
pub(super) fn legend_box(legend: Legend) -> PyResult<Owned> {
    Python::with_gil(|py| Ok(describe(py, &legend.legend_box())?.unbind()))
}

#[cfg(test)]
mod tests {
    use super::super::super::offsetbox::{pack_align, pack_mode};
    use super::*;

    #[test]
    fn names_are_those_the_offsetbox_parsers_read() {
        for align in [
            PackAlign::Baseline,
            PackAlign::Left,
            PackAlign::Top,
            PackAlign::Right,
            PackAlign::Bottom,
            PackAlign::Center,
        ] {
            assert_eq!(pack_align(align_name(align)).unwrap(), align);
        }
        for mode in [PackMode::Fixed, PackMode::Expand, PackMode::Equal] {
            assert_eq!(pack_mode(mode_name(mode)).unwrap(), mode);
        }
    }
}
