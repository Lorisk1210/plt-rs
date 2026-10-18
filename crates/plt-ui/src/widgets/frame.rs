//! Painting shared by the framed controls (text fields, lists, buttons,
//! check boxes): faces, outlines and focus rings.

use plt_core::Rgba;

use crate::error::UiError;
use crate::geometry::Rect;
use crate::widget::PaintCx;

/// Paints a control's face and outline in `rect`, with corners rounded
/// to `radius` pixels: `face` inside a one-pixel outline in the border
/// colour, darker under the pointer and in the accent colour with the
/// focus ring around it while the control is focused.
pub(crate) fn paint_frame(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    radius: f64,
    face: Rgba,
) -> Result<(), UiError> {
    let theme = cx.theme();
    let status = cx.status();
    let scale = cx.scale_factor();
    let outline = if status.focused {
        theme.accent
    } else if status.hot || status.active {
        theme.border_hot
    } else {
        theme.border
    };
    cx.canvas().fill_rounded_rect(rect, radius, face)?;
    cx.canvas()
        .stroke_rounded_rect(rect, radius, scale, outline)?;
    if status.focused {
        paint_focus_ring(cx, rect, radius)?;
    }
    Ok(())
}

/// Paints the focus ring just outside `rect`, whose corners are rounded
/// to `radius` pixels. Widgets may paint this far outside their
/// rectangle (see [`Theme::focus_ring_width`](crate::Theme)).
fn paint_focus_ring(cx: &mut PaintCx<'_>, rect: Rect, radius: f64) -> Result<(), UiError> {
    let theme = cx.theme();
    let width = theme.focus_ring_width * cx.scale_factor();
    cx.canvas()
        .stroke_rounded_rect(rect.inset(-width), radius + width, width, theme.focus_ring)
}
