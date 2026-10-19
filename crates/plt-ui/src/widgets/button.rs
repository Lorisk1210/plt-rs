//! Push buttons.

use plt_platform::MouseButton;

use super::frame::paint_frame;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::TextLayout;
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// A button with a text label that sends a message when clicked: pressed
/// and released with the left button while the pointer is over it.
///
/// Buttons are as tall as the theme's controls and at least
/// [`Theme::button_min_width`](crate::Theme) wide, so that the buttons of
/// a dialog match. A secondary button has a light face and an outline;
/// the [primary](Button::primary) one is filled with the accent colour.
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
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let style = theme.text_style();
        let text = TextLayout::new(&self.label, &style, scale)?;
        let padding = theme.space(Spacing::Md) * scale;
        let width = (text.size().width + 2.0 * padding).max(theme.button_min_width * scale);
        self.text = Some(text);
        Ok(constraints.constrain(Size::new(width, theme.control_height * scale)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let status = cx.status();
        let theme = cx.theme();
        let radius = theme.corner_radius * cx.scale_factor();
        let pressed = status.active && self.pressed;
        let ink = if self.primary {
            let face = if pressed {
                theme.accent_active
            } else if status.hot {
                theme.accent_hot
            } else {
                theme.accent
            };
            cx.canvas().fill_rounded_rect(rect, radius, face)?;
            theme.text_on_accent
        } else {
            let face = if pressed {
                theme.button_active
            } else if status.hot {
                theme.button_hot
            } else {
                theme.button
            };
            paint_frame(cx, rect, radius, face)?;
            theme.text
        };
        if let Some(text) = &self.text {
            let origin = rect.centered(text.size()).origin();
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
