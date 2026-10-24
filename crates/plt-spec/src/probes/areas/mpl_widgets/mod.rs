//! Probe operations for the `mpl_widgets` area: matplotlib's own tests of
//! its widgets (`tests/test_widgets.py`; see
//! `spec/reference/probes/test_mpl_widgets.py`), in the test suite's
//! classic style. The selectors replay [`steps`] of synthetic events and
//! queries: [`region`] (rectangles and ellipses), [`span`], [`polygon`]
//! and [`lasso`]; [`buttons`] covers check and radio buttons and text
//! boxes, [`sliders`] the sliders and [`cursors`] the cursors.

mod buttons;
mod cursors;
mod lasso;
mod polygon;
mod props;
mod region;
mod sliders;
mod span;
mod steps;

use plt::axes::Axes;
use plt::figure::Figure;
use plt::lines::Line2D;
use plt::navigation::PointerButton;
use plt::rc::{Aspect, ColorOrAuto};

use crate::json::Json;
use crate::probes::encode::{count, numbers, resolved_rgba};
use crate::probes::{Op, ProbeError, malformed};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("RectangleSelector", region::rectangle),
    Op::new("EllipseSelector", region::ellipse),
    Op::new("SpanSelector", span::span),
    Op::new("SpanSelector._snap", span::snap),
    Op::new("LassoSelector", lasso::lasso_selector),
    Op::new("Lasso", lasso::lasso),
    Op::new("PolygonSelector", polygon::polygon),
    Op::new("CheckButtons", buttons::check_buttons),
    Op::new("CheckButtons.set_active", buttons::check_set_active),
    Op::new("RadioButtons", buttons::radio_buttons),
    Op::new("RadioButtons.activecolor", buttons::radio_activecolor),
    Op::new("_Buttons.on_clicked", buttons::on_clicked),
    Op::new("TextBox", buttons::textbox),
    Op::new("Slider", sliders::slider),
    Op::new("Slider.slidermin", sliders::slidermin),
    Op::new("Slider.slidermax", sliders::slidermax),
    Op::new("RangeSlider", sliders::range_slider),
    Op::new("MultiCursor", cursors::multicursor),
    Op::new("Cursor.useblit", cursors::cursor_useblit),
];

/// matplotlib's `testing.widgets.get_ax`: the axes of a new figure showing
/// the line from `(0, 0)` to `(200, 200)` at equal aspect, drawn.
fn get_ax() -> Result<Axes, ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.plot(vec![0.0, 200.0], vec![0.0, 200.0])
        .map_err(ProbeError::rejected)?;
    ax.set_aspect(Aspect::Ratio(1.0))
        .map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(ax)
}

/// matplotlib's `Line2D.get_markerfacecolor`: the line colour when
/// automatic.
fn markerfacecolor(line: &Line2D) -> Result<Json, ProbeError> {
    match line.markerfacecolor() {
        ColorOrAuto::Color(color) => resolved_rgba(&color),
        ColorOrAuto::Auto => resolved_rgba(&line.color()),
    }
}

/// A mouse button given by matplotlib's number.
fn button(value: &Json) -> Result<PointerButton, ProbeError> {
    match value.as_i64() {
        Some(1) => Ok(PointerButton::Left),
        Some(2) => Ok(PointerButton::Middle),
        Some(3) => Ok(PointerButton::Right),
        _ => Err(malformed("button")),
    }
}
