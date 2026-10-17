//! Check boxes.

use std::fmt;
use std::rc::Rc;

use plt_core::{Path, Point};
use plt_platform::MouseButton;

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::{TextLayout, TextStyle};
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// A box with a label that a click checks or unchecks, sending a message
/// with the new state.
pub struct Checkbox<M> {
    label: String,
    checked: bool,
    on_toggle: Rc<dyn Fn(bool) -> M>,
    /// The label laid out by the last measure.
    layout: Option<TextLayout>,
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
        let style = TextStyle::new(cx.theme().font_size);
        let layout = TextLayout::new(&self.label, &style, cx.scale_factor())?;
        let padding = cx.theme().padding * cx.scale_factor();
        let size = layout.size();
        self.layout = Some(layout);
        let width = size.height + padding + size.width + 2.0 * padding;
        Ok(constraints.constrain(Size::new(width, size.height + padding)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let Some(layout) = &self.layout else {
            return Ok(());
        };
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let padding = theme.padding * scale;
        let side = layout.size().height * 0.85;
        let top = rect.center().y - side / 2.0;
        let square = Rect::new(rect.x + padding, top, side, side);
        let radius = theme.corner_radius * scale * 0.5;
        if self.checked {
            cx.canvas()
                .fill_rounded_rect(square, radius, theme.accent)?;
            let tick = Path::polyline(vec![
                Point::new(square.x + side * 0.22, top + side * 0.52),
                Point::new(square.x + side * 0.42, top + side * 0.72),
                Point::new(square.x + side * 0.78, top + side * 0.3),
            ]);
            cx.canvas()
                .stroke_path(&tick, 1.8 * scale, plt_core::Rgba::WHITE)?;
        } else {
            let border = if cx.status().hot {
                theme.accent
            } else {
                theme.separator
            };
            cx.canvas().fill_rounded_rect(square, radius, border)?;
            cx.canvas()
                .fill_rounded_rect(square.inset(scale), radius - scale, theme.surface)?;
        }
        let origin = Point::new(
            square.max_x() + padding,
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
