//! Painting shared by the controls: the faces and outlines of framed ones
//! (text fields, lists, buttons, drop-downs) and the focus ring of every
//! focusable one.

use plt_core::Rgba;

use crate::error::UiError;
use crate::geometry::Rect;
use crate::widget::PaintCx;

/// Paints a control's face and outline in `rect`, with corners rounded
/// to `radius` pixels: `face` inside a one-pixel outline in the border
/// colour, darker under the pointer, and in the accent colour with the
/// focus ring around it when `show_focus`.
pub(crate) fn paint_frame(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    radius: f64,
    face: Rgba,
    show_focus: bool,
) -> Result<(), UiError> {
    let theme = cx.theme();
    let status = cx.status();
    let scale = cx.scale_factor();
    let outline = if show_focus {
        theme.accent
    } else if status.hot || status.active {
        theme.border_hot
    } else {
        theme.border
    };
    cx.canvas().fill_rounded_rect(rect, radius, face)?;
    cx.canvas()
        .stroke_rounded_rect(rect, radius, scale, outline)?;
    if show_focus {
        paint_halo(cx, rect, radius)?;
    }
    Ok(())
}

/// Paints the focus ring of a control in `rect`, whose corners are
/// rounded to `radius` pixels (a circle when `rect` is a square and
/// `radius` half its side): a one-pixel accent line along its edge, as
/// framed controls show, and the translucent halo just outside it.
/// Widgets may paint this far outside their rectangle (see
/// [`Theme::focus_ring_width`](crate::Theme)).
pub(crate) fn paint_focus_ring(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    radius: f64,
) -> Result<(), UiError> {
    let theme = cx.theme();
    let scale = cx.scale_factor();
    cx.canvas()
        .stroke_rounded_rect(rect, radius, scale, theme.accent)?;
    paint_halo(cx, rect, radius)
}

/// Paints the translucent halo of the focus ring just outside `rect`.
fn paint_halo(cx: &mut PaintCx<'_>, rect: Rect, radius: f64) -> Result<(), UiError> {
    let theme = cx.theme();
    let width = theme.focus_ring_width * cx.scale_factor();
    cx.canvas()
        .stroke_rounded_rect(rect.inset(-width), radius + width, width, theme.focus_ring)
}
