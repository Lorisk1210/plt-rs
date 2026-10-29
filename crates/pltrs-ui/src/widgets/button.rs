//! Push buttons.

use pltrs_platform::{ButtonState, Key, KeyEvent, MouseButton};

use super::frame::{paint_focus_ring, paint_frame};
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::TextLayout;
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// A button with a text label that sends a message when clicked (pressed
/// and released with the left button while the pointer is over it), or
/// when Space or Enter is pressed while it has the focus.
///
/// Buttons are as tall as the theme's controls and at least
/// [`Theme::button_min_width`](crate::Theme) wide, so that the buttons of
/// a dialog match. A secondary button has a light face and an outline;
/// the [primary](Button::primary) one is filled with the accent colour,
/// and Enter presses it from anywhere in the window.
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

    /// The button styled as the main action of its group, filled with the
    /// theme's accent colour, and made the window's default button: Enter
    /// presses it when the focused widget leaves Enter unused (a text
    /// field, a check box), as in Qt and GTK dialogs.
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

/// Whether `key` presses a focused button: Space or Enter.
fn presses(key: &KeyEvent) -> bool {
    matches!(key.key, Key::Character(' ') | Key::Enter)
        && !key.modifiers.control()
        && !key.modifiers.alt()
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
            if status.focus_visible() {
                paint_focus_ring(cx, rect, radius)?;
            }
            theme.text_on_accent
        } else {
            let face = if pressed {
                theme.button_active
            } else if status.hot {
                theme.button_hot
            } else {
                theme.button
            };
            paint_frame(cx, rect, radius, face, status.focus_visible())?;
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
            UiEvent::Key(key) if key.state == ButtonState::Pressed && presses(key) => {
                if !key.repeat {
                    cx.emit(self.message.clone());
                }
                cx.set_handled();
            }
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn handles_shortcut(&self, key: &KeyEvent) -> bool {
        self.primary && key.key == Key::Enter && presses(key)
    }
}

#[cfg(test)]
mod tests {
    use pltrs_platform::Modifiers;

    use super::*;

    fn key(key: Key, repeat: bool, modifiers: Modifiers) -> UiEvent {
        UiEvent::Key(KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat,
            text: None,
            modifiers,
            scancode: 0,
        })
    }

    #[test]
    fn space_and_enter_press_a_focused_button() {
        let mut button = Button::new("OK", "ok");
        let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        button.event(
            &mut cx,
            &key(Key::Character(' '), false, Modifiers::NONE),
            rect,
        );
        button.event(&mut cx, &key(Key::Enter, false, Modifiers::NONE), rect);
        button.event(&mut cx, &key(Key::Enter, true, Modifiers::NONE), rect);
        assert!(cx.handled);
        let mut ignored = EventCx::new(cx.messages);
        button.event(
            &mut ignored,
            &key(Key::Character('x'), false, Modifiers::NONE),
            rect,
        );
        button.event(
            &mut ignored,
            &key(Key::Enter, false, Modifiers::CONTROL),
            rect,
        );
        assert!(!ignored.handled, "other keys go on");
        assert_eq!(messages, ["ok", "ok"], "auto-repeat presses nothing");
    }

    #[test]
    fn the_primary_button_takes_enter_as_a_shortcut() {
        let enter = KeyEvent {
            key: Key::Enter,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers: Modifiers::NONE,
            scancode: 0,
        };
        let space = KeyEvent {
            key: Key::Character(' '),
            ..enter.clone()
        };
        let primary = Button::new("Save", ()).primary();
        assert!(primary.handles_shortcut(&enter));
        assert!(!primary.handles_shortcut(&space));
        assert!(!Button::new("Cancel", ()).handles_shortcut(&enter));
    }
}
