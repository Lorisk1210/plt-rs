//! Check boxes.

use std::fmt;
use std::rc::Rc;

use plt_core::{Path, Point};
use plt_platform::MouseButton;

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::text::{TextLayout, TextStyle};
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// The gap between the box and its label.
const GAP: Spacing = Spacing::Sm;

/// The tick in a checked box.
fn tick() -> Icon {
    Icon::new(24.0)
        .with_stroke_width(3.0)
        .stroke(Path::polyline(vec![
            Point::new(6.0, 12.5),
            Point::new(10.0, 16.5),
            Point::new(18.0, 8.0),
        ]))
}

/// A box with a label that a click checks or unchecks, sending a message
/// with the new state. Clicks count on the box and the label, not on the
/// rest of the widget's rectangle.
pub struct Checkbox<M> {
    label: String,
    checked: bool,
    on_toggle: Rc<dyn Fn(bool) -> M>,
    /// The label laid out by the last measure.
    layout: Option<TextLayout>,
    /// The width of the box, gap and label from the last measure.
    content_width: f64,
}

impl<M> fmt::Debug for Checkbox<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Checkbox")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .finish_non_exhaustive()
    }
}

impl<M> Checkbox<M> {
    /// An unchecked box labelled `label`, sending `on_toggle(checked)`.
    pub fn new(label: impl Into<String>, on_toggle: impl Fn(bool) -> M + 'static) -> Self {
        Self {
            label: label.into(),
            checked: false,
            on_toggle: Rc::new(on_toggle),
            layout: None,
            content_width: 0.0,
        }
    }

    /// Whether the box is checked.
    pub fn checked(&self) -> bool {
        self.checked
    }

    /// Checks or unchecks the box; no message is sent.
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }
}

impl<M: 'static> Widget<M> for Checkbox<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let style = TextStyle::new(theme.font_size);
        let layout = TextLayout::new(&self.label, &style, scale)?;
        self.content_width = (theme.checkbox_size + theme.space(GAP)) * scale + layout.size().width;
        self.layout = Some(layout);
        let natural = Size::new(self.content_width, theme.control_height * scale);
        Ok(constraints.constrain(natural))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let Some(layout) = &self.layout else {
            return Ok(());
        };
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let hot = cx.status().hot;
        let side = theme.checkbox_size * scale;
        let square = Rect::new(rect.x, rect.center().y - side / 2.0, side, side).snapped();
        // Slightly tighter corners than full-size controls.
        let radius = (theme.corner_radius - theme.space(Spacing::Xxs)) * scale;
        if self.checked {
            let face = if hot { theme.accent_hot } else { theme.accent };
            cx.canvas().fill_rounded_rect(square, radius, face)?;
            tick().paint(cx.canvas(), square, theme.text_on_accent)?;
        } else {
            let outline = if hot { theme.border_hot } else { theme.border };
            cx.canvas()
                .fill_rounded_rect(square, radius, theme.surface)?;
            cx.canvas()
                .stroke_rounded_rect(square, radius, scale, outline)?;
        }
        let origin = Point::new(
            square.max_x() + theme.space(GAP) * scale,
            rect.center().y - layout.size().height / 2.0,
        );
        cx.canvas().draw_text(layout, origin, theme.text)
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        if let UiEvent::PointerUp {
            button: MouseButton::Left,
            position,
            ..
        } = event
            && rect.contains(*position)
            && position.x < rect.x + self.content_width
        {
            self.checked = !self.checked;
            cx.emit((self.on_toggle)(self.checked));
            cx.request_paint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling() {
        let mut checkbox = Checkbox::new("legend", |on| on);
        assert!(!checkbox.checked());
        checkbox.set_checked(true);
        assert!(checkbox.checked());
    }
}
