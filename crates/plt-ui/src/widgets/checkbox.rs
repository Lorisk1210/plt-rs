//! Check boxes.

use std::fmt;
use std::rc::Rc;

use plt_core::{Path, Point};
use plt_platform::{ButtonState, Key, MouseButton};

use super::frame::paint_focus_ring;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::text::TextLayout;
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
/// with the new state; so does Space while it has the focus. Clicks count
/// on the box and the label, not on the rest of the widget's rectangle.
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

    /// Checks an unchecked box and unchecks a checked one, sending the
    /// new state.
    fn toggle(&mut self, cx: &mut EventCx<'_, M>) {
        self.checked = !self.checked;
        cx.emit((self.on_toggle)(self.checked));
        cx.request_paint();
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
        let style = theme.text_style();
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
        let status = cx.status();
        let hot = status.hot;
        // Held down on the box or its label, until released over them.
        let pressed = status.active && hot;
        let side = theme.checkbox_size * scale;
        let square = Rect::new(rect.x, rect.center().y - side / 2.0, side, side).snapped();
        // Slightly tighter corners than full-size controls.
        let radius = (theme.corner_radius - theme.space(Spacing::Xxs)) * scale;
        if self.checked {
            let face = if pressed {
                theme.accent_active
            } else if hot {
                theme.accent_hot
            } else {
                theme.accent
            };
            cx.canvas().fill_rounded_rect(square, radius, face)?;
            tick().paint(cx.canvas(), square, theme.text_on_accent)?;
        } else {
            let face = if pressed {
                theme.button_active
            } else if hot {
                theme.button_hot
            } else {
                theme.surface
            };
            cx.canvas().fill_rounded_rect(square, radius, face)?;
            cx.canvas()
                .stroke_rounded_rect(square, radius, scale, theme.border_strong)?;
        }
        if status.focus_visible() {
            paint_focus_ring(cx, square, radius)?;
        }
        let origin = Point::new(
            square.max_x() + theme.space(GAP) * scale,
            rect.center().y - layout.size().height / 2.0,
        );
        cx.canvas().draw_text(layout, origin, theme.text)
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        match event {
            UiEvent::PointerUp {
                button: MouseButton::Left,
                position,
                ..
            } if rect.contains(*position) && position.x < rect.x + self.content_width => {
                self.toggle(cx);
            }
            UiEvent::Key(key)
                if key.state == ButtonState::Pressed
                    && key.key == Key::Character(' ')
                    && !key.modifiers.control()
                    && !key.modifiers.alt() =>
            {
                if !key.repeat {
                    self.toggle(cx);
                }
                cx.set_handled();
            }
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
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

    #[test]
    fn space_toggles_a_focused_box() {
        use plt_platform::{KeyEvent, Modifiers};

        let mut checkbox = Checkbox::new("legend", |on| on);
        let space = |key| {
            UiEvent::Key(KeyEvent {
                key,
                state: ButtonState::Pressed,
                repeat: false,
                text: Some(" ".into()),
                modifiers: Modifiers::NONE,
                scancode: 65,
            })
        };
        let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        checkbox.event(&mut cx, &space(Key::Character(' ')), rect);
        checkbox.event(&mut cx, &space(Key::Character(' ')), rect);
        assert!(cx.handled);
        let mut enter = EventCx::new(cx.messages);
        checkbox.event(&mut enter, &space(Key::Enter), rect);
        assert!(!enter.handled, "Enter goes on to the default button");
        assert_eq!(messages, [true, false]);
    }
}
