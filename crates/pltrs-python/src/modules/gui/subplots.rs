//! The subplot configuration tool of the windows (`configure_subplots`
//! of the `pltrs` backend): opened showing the figure's layout, it
//! reports what the user asks as `"subplots"` events, which the backend
//! carries out with matplotlib's calls before showing the layout again.
//!
//! A layout travels as `(kind, values)`: `("params", (left, bottom,
//! right, top, wspace, hspace))`, the subplot parameters, or
//! `("constrained", (h_pad, w_pad, hspace, wspace))`, constrained
//! layout's spacing.

use pltrs::gridspec::SubplotParams;
use pltrs::gui::{ConstrainedSpacing, SubplotChange, SubplotLayout, WindowHost};

use super::id;
use crate::binding::prelude::*;

/// The layout `(kind, values)` stands for.
fn layout((kind, values): (String, Vec<f64>)) -> PyResult<SubplotLayout> {
    match (kind.as_str(), &values[..]) {
        ("params", &[left, bottom, right, top, wspace, hspace]) => {
            Ok(SubplotLayout::Params(SubplotParams {
                left,
                bottom,
                right,
                top,
                wspace,
                hspace,
            }))
        }
        ("constrained", &[h_pad, w_pad, hspace, wspace]) => {
            Ok(SubplotLayout::Constrained(ConstrainedSpacing {
                h_pad,
                w_pad,
                hspace,
                wspace,
            }))
        }
        _ => Err(PyErr::value_error(format!(
            "a subplot layout is (\"params\", 6 values) or (\"constrained\", 4 values), \
             not ({kind:?}, {} values)",
            values.len()
        ))),
    }
}

/// `(kind, values)` of a change the tool asks for: `("adjust", (left,
/// bottom, right, top, wspace, hspace))`, `("constrain", (h_pad, w_pad,
/// hspace, wspace))`, `("tight_layout", ())` or `("reset", ())`.
pub(super) fn change_parts(change: SubplotChange) -> (&'static str, Vec<f64>) {
    match change {
        SubplotChange::Adjust(p) => (
            "adjust",
            vec![p.left, p.bottom, p.right, p.top, p.wspace, p.hspace],
        ),
        SubplotChange::Constrain(s) => ("constrain", vec![s.h_pad, s.w_pad, s.hspace, s.wspace]),
        SubplotChange::TightLayout => ("tight_layout", Vec::new()),
        SubplotChange::Reset => ("reset", Vec::new()),
    }
}

/// Opens the subplot tool of window `window` showing `layout`, with the
/// switch to constrained layout's values when `offer_constrained`, or
/// shows `layout` in it and raises it.
fn open_subplot_tool(
    host: Native<WindowHost>,
    window: u64,
    layout_parts: (String, Vec<f64>),
    offer_constrained: bool,
) -> PyResult<()> {
    let layout = layout(layout_parts)?;
    Ok(host
        .borrow_mut()
        .open_subplot_tool(id(window), layout, offer_constrained)?)
}

/// Shows `layout` in the subplot tool of window `window`, with `problem`
/// (nothing when empty).
fn show_subplot_layout(
    host: Native<WindowHost>,
    window: u64,
    layout_parts: (String, Vec<f64>),
    problem: String,
) -> PyResult<()> {
    let layout = layout(layout_parts)?;
    Ok(host
        .borrow_mut()
        .show_subplot_layout(id(window), layout, &problem)?)
}

/// Adds the functions to `module`.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("open_subplot_tool", open_subplot_tool)?;
    module.add("show_subplot_layout", show_subplot_layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_and_changes_travel_as_kinds_and_values() {
        let params = layout(("params".into(), vec![0.1, 0.2, 0.9, 0.8, 0.3, 0.4]));
        assert!(matches!(
            params,
            Ok(SubplotLayout::Params(SubplotParams { right: 0.9, .. }))
        ));
        let spacing = layout(("constrained".into(), vec![0.04, 0.05, 0.02, 0.03]));
        assert!(matches!(
            spacing,
            Ok(SubplotLayout::Constrained(ConstrainedSpacing {
                w_pad: 0.05,
                ..
            }))
        ));
        assert!(layout(("params".into(), vec![0.1])).is_err());
        assert!(layout(("tight".into(), Vec::new())).is_err());
        assert_eq!(change_parts(SubplotChange::Reset), ("reset", Vec::new()));
        let spacing = ConstrainedSpacing {
            h_pad: 1.0,
            w_pad: 2.0,
            hspace: 3.0,
            wspace: 4.0,
        };
        assert_eq!(
            change_parts(SubplotChange::Constrain(spacing)),
            ("constrain", vec![1.0, 2.0, 3.0, 4.0])
        );
    }
}
