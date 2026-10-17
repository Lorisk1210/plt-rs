//! Push buttons.

use plt_core::Point;
use plt_platform::MouseButton;

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::{TextLayout, TextStyle};
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// A button with a text label that sends a message when clicked: pressed
/// and released with the left button while the pointer is over it.
#[derive(Debug)]
pub struct Button<M> {
    label: String,
    message: M,
    /// The label laid out by the last measure.
    text: Option<TextLayout>,
    /// Whether the left button went down on this button and is held.
    pressed: bool,
    /// Whether this is the primary action, filled with the accent colour.
    primary: bool,
}

impl<M> Button<M> {
    /// A button showing `label` that sends `message` when clicked.
    pub fn new(label: impl Into<String>, message: M) -> Self {
        Self {
            label: label.into(),
            message,
            text: None,
            pressed: false,
            primary: false,
        }
    }

    /// The button styled as the main action of its group: filled with the
    /// theme's accent colour.
    #[must_use]
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    /// The label.
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let style = TextStyle::new(cx.theme().font_size);
        let text = TextLayout::new(&self.label, &style, cx.scale_factor())?;
        let padding = cx.theme().padding * cx.scale_factor();
        let size = text.size();
        self.text = Some(text);
        Ok(constraints.constrain(Size::new(size.width + 2.0 * padding, size.height + padding)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let status = cx.status();
        let theme = cx.theme();
        let pressed = status.active && self.pressed;
        let (face, ink) = if self.primary {
            // Darken the accent when hovered or pressed.
            let shade = if pressed {
                0.8
            } else if status.hot {
                0.9
            } else {
                1.0
            };
            let accent = theme.accent;
            let face = plt_core::Rgba::new(
                accent.r * shade,
                accent.g * shade,
                accent.b * shade,
                accent.a,
            );
            (face, plt_core::Rgba::WHITE)
        } else if pressed {
            (theme.button_active, theme.text)
        } else if status.hot {
            (theme.button_hot, theme.text)
        } else {
            (theme.button, theme.text)
        };
        let scale = cx.scale_factor();
        let radius = theme.corner_radius * scale;
        cx.canvas()
            .fill_rounded_rect(rect.inset(scale), radius, face)?;
        if let Some(text) = &self.text {
            let center = rect.center();
            let size = text.size();
            let origin = Point::new(center.x - size.width / 2.0, center.y - size.height / 2.0);
            cx.canvas().draw_text(text, origin, ink)?;
        }
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        match event {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                ..
            } => {
                self.pressed = true;
                cx.request_paint();
            }
            UiEvent::PointerUp {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if self.pressed && rect.contains(*position) {
                    cx.emit(self.message.clone());
                }
                self.pressed = false;
                cx.request_paint();
            }
            _ => {}
        }
    }
}
