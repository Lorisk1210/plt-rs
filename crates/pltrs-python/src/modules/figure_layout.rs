//! `_native.figure_layout`: the layout methods of figures and subfigures
//! (matplotlib's `Figure.tight_layout`, `GridSpec.tight_layout`, the
//! `align_*` methods, `get_tightbbox` and the placement of the figure's
//! title and labels), by `pltrs::layout` and `pltrs::figure`.

use pltrs::axes::{Axes, AxisName, TickLevel};
use pltrs::figure::Figure;
use pltrs::gridspec::GridSpec;
use pltrs::layout::{LayoutEngine, TightLayout, TightLayoutError};
use pltrs::text::{HAlign, Text};
use pltrs_core::Bbox;

use crate::binding::prelude::*;

/// Tight layout's rectangle as Python gives it, `(left, bottom, right,
/// top)` with None for the sides tight layout chooses (as 0 does).
type Rect = Option<(Option<f64>, Option<f64>, Option<f64>, Option<f64>)>;

/// The parameters of a tight layout pass.
fn params(pad: f64, h_pad: Option<f64>, w_pad: Option<f64>, rect: Rect) -> TightLayout {
    TightLayout {
        pad,
        h_pad,
        w_pad,
        rect: rect.map(|(l, b, r, t)| [l, b, r, t]),
    }
}

/// The outcome of a tight layout pass: None once applied, the warning
/// matplotlib issues when it is not, matplotlib's `RuntimeError` when the
/// figure's colorbars refuse the engine.
fn outcome<T>(result: Result<T, TightLayoutError>) -> PyResult<Option<String>> {
    match result {
        Ok(_) => Ok(None),
        Err(TightLayoutError::Engine(err)) => Err(err.into()),
        Err(err) => Ok(Some(err.to_string())),
    }
}

/// Fits the figure's subplot parameters to the decorations of its
/// subplots once, leaving it without a layout engine
/// (`Figure.tight_layout`): whether the figure's engine made way (pltrs'
/// modern looks keep constrained layout), and the [`outcome`].
fn tight_layout(
    figure: Figure,
    pad: f64,
    h_pad: Option<f64>,
    w_pad: Option<f64>,
    rect: Rect,
) -> PyResult<(bool, Option<String>)> {
    let not_applied = outcome(figure.tight_layout(params(pad, h_pad, w_pad, rect)))?;
    let replaced = matches!(figure.layout_engine(), Some(LayoutEngine::PlaceHolder(_)));
    Ok((replaced, not_applied))
}

/// Fits the margins and spacing of `grid` to the decorations of its
/// subplots in `figure` once (`GridSpec.tight_layout`); see [`outcome`].
fn grid_tight_layout(
    grid: GridSpec,
    figure: Figure,
    pad: f64,
    h_pad: Option<f64>,
    w_pad: Option<f64>,
    rect: Rect,
) -> PyResult<Option<String>> {
    outcome(grid.tight_layout(&figure, params(pad, h_pad, w_pad, rect)))
}

/// Aligns the x labels, y labels or titles (`which`: "x", "y" or
/// "titles") of `axes`, or of every axes of the figure, at every later
/// draw (`align_xlabels`, `align_ylabels`, `align_titles`).
fn align(figure: Figure, axes: Option<Vec<Axes>>, which: String) -> PyResult<()> {
    let axes = axes.as_deref();
    match which.as_str() {
        "x" => figure.align_xlabels(axes),
        "y" => figure.align_ylabels(axes),
        "titles" => figure.align_titles(axes),
        _ => {
            return Err(PyErr::value_error(format!(
                "cannot align {which:?}: expected 'x', 'y' or 'titles'"
            )));
        }
    };
    Ok(())
}

/// The figure's title, x label and y label, those it has (`_suptitle`,
/// `_supxlabel`, `_supylabel`).
fn super_texts(figure: Figure) -> PyResult<(Option<Text>, Option<Text>, Option<Text>)> {
    let [title, xlabel, ylabel] = figure.super_text_artists();
    Ok((title, xlabel, ylabel))
}

/// Places the figure-wide text `kind` ("title", "xlabel" or "ylabel") by
/// hand at `at` across its edge (`suptitle(y=...)`, `supxlabel(y=...)`,
/// `supylabel(x=...)`), out of the layout engine's hands.
fn place_super_text(figure: Figure, kind: String, at: f64) -> PyResult<()> {
    match kind.as_str() {
        "title" => figure.set_suptitle_y(at),
        "xlabel" => figure.set_supxlabel_y(at),
        "ylabel" => figure.set_supylabel_x(at),
        _ => return Err(PyErr::value_error(format!("no figure-wide text {kind:?}"))),
    };
    Ok(())
}

/// Rotates the x tick labels of level `which` ("major", "minor" or
/// "both") by `rotation` degrees, aligned `ha`, keeping them on the
/// bottom row of subplots only, and makes `bottom` room below
/// (`autofmt_xdate`; the names are already checked).
fn autofmt_xdate(
    figure: Figure,
    bottom: f64,
    rotation: f64,
    ha: String,
    which: String,
) -> PyResult<()> {
    let halign: HAlign = ha.parse()?;
    let level: TickLevel = which
        .parse()
        .map_err(|err: pltrs::rc::UnknownName| PyErr::value_error(err.to_string()))?;
    figure.autofmt_xdate(bottom, rotation, halign, level)?;
    Ok(())
}

/// A box as `(x0, y0, x1, y1)`.
fn extents(bbox: Bbox) -> (f64, f64, f64, f64) {
    (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())
}

/// The extent of the axes with its decorations in display pixels, and
/// with its children (spines, insets, unclipped artists) when `children`
/// is set; None for an invisible axes (`Axes.get_tightbbox`).
fn axes_tightbbox(
    axes: Axes,
    for_layout_only: bool,
    children: bool,
) -> PyResult<Option<(f64, f64, f64, f64)>> {
    // matplotlib's `get_tightbbox` applies the aspect, then places the
    // ticks of each axis it measures; both may raise.
    axes.apply_aspect()?;
    if axes.axis_on() {
        for name in [AxisName::X, AxisName::Y] {
            let axis = axes.axis_of(name);
            if axis.visible() && (axis.in_layout() || !for_layout_only) {
                axes.ticks(name)?;
            }
        }
    }
    let bbox = if children {
        axes.tight_bbox(for_layout_only)
    } else {
        axes.decorations_tight_bbox(for_layout_only)
    };
    Ok(bbox.map(extents))
}

/// The area everything visible in the (sub)figure is drawn in, in display
/// pixels, None when nothing is (`FigureBase.get_tightbbox` before the
/// root figure's box is turned into inches).
fn figure_tightbbox(figure: Figure) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(figure.tight_extent().map(extents))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("tight_layout", tight_layout)?;
    module.add("grid_tight_layout", grid_tight_layout)?;
    module.add("align", align)?;
    module.add("super_texts", super_texts)?;
    module.add("place_super_text", place_super_text)?;
    module.add("autofmt_xdate", autofmt_xdate)?;
    module.add("axes_tightbbox", axes_tightbbox)?;
    module.add("figure_tightbbox", figure_tightbbox)?;
    Ok(())
}
