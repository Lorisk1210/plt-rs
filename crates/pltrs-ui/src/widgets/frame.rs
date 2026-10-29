//! Painting shared by the controls: the faces and outlines of framed ones
//! (text fields, lists, buttons, drop-downs), the strong edge identifying
//! field-like ones, and the focus ring of every focusable one.

use pltrs_core::Rgba;

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
        paint_halo(cx, rect, radius, theme.focus_ring)?;
    }
    Ok(())
}

/// Paints the edge identifying a field-like control (a text field, a
/// drop-down, a segmented control) in `rect`, whose corners are rounded to
/// `radius` pixels: the bottom of its outline, one logical pixel high, in
/// the theme's [`border_strong`](crate::Theme::border_strong), thinning
/// out into the corners. The rest of the outline stays soft, so forms
/// stay light while each field stands out from its card (WCAG 1.4.11).
/// Controls leave it out while a focus ring or a refusal outlines them.
pub(crate) fn paint_edge(cx: &mut PaintCx<'_>, rect: Rect, radius: f64) -> Result<(), UiError> {
    let theme = cx.theme();
    let scale = cx.scale_factor();
    let band = Rect::new(rect.x, rect.max_y() - scale, rect.width, scale);
    cx.canvas()
        .clipped(band)
        .stroke_rounded_rect(rect, radius, scale, theme.border_strong)
}

/// Paints the outline of a framed control in `rect` (corners rounded to
/// `radius` pixels) whose value the application refused: a one-pixel line
/// in the danger colour, with a halo of it outside when `focused`, in
/// place of the focus ring.
pub(crate) fn paint_invalid(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    radius: f64,
    focused: bool,
) -> Result<(), UiError> {
    let theme = cx.theme();
    let scale = cx.scale_factor();
    let danger = theme.danger;
    cx.canvas()
        .stroke_rounded_rect(rect, radius, scale, danger)?;
    if focused {
        let halo = Rgba::new(danger.r, danger.g, danger.b, theme.focus_ring.a);
        paint_halo(cx, rect, radius, halo)?;
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
    paint_halo(cx, rect, radius, theme.focus_ring)
}

/// Paints a translucent halo of `color`, as wide as the focus ring, just
/// outside `rect`.
fn paint_halo(cx: &mut PaintCx<'_>, rect: Rect, radius: f64, color: Rgba) -> Result<(), UiError> {
    let width = cx.theme().focus_ring_width * cx.scale_factor();
    cx.canvas()
        .stroke_rounded_rect(rect.inset(-width), radius + width, width, color)
}
