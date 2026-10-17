//! Toolbar buttons: an icon that can be checked (a mode is on) or
//! disabled (nothing to do).

use plt_platform::MouseButton;

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// The side of the icon in logical pixels.
const ICON_SIZE: f64 = 20.0;

/// An icon button that sends a message when clicked (pressed and released
/// with the left button over it). A checked button shows the accent
/// colour; a disabled one is greyed out and ignores the pointer.
#[derive(Debug)]
pub struct ToolButton<M> {
    icon: Icon,
    message: M,
    /// Messages sent when the pointer enters and leaves the button.
    hover: Option<(M, M)>,
    checked: bool,
    enabled: bool,
    /// Whether the left button went down on this button and is held.
    pressed: bool,
}

impl<M> ToolButton<M> {
    /// An enabled, unchecked button showing `icon` that sends `message`.
    pub fn new(icon: Icon, message: M) -> Self {
        Self {
            icon,
            message,
            hover: None,
            checked: false,
            enabled: true,
            pressed: false,
        }
    }

    /// Also sends `enter` when the pointer moves onto the button and
    /// `leave` when it moves off (e.g. to show a hint).
    #[must_use]
    pub fn with_hover_messages(mut self, enter: M, leave: M) -> Self {
        self.hover = Some((enter, leave));
        self
    }

    /// Checks or unchecks the button (e.g. while its mode is on).
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    /// Whether the button is checked.
    pub fn checked(&self) -> bool {
        self.checked
    }

    /// Enables or disables the button.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.pressed = false;
        }
    }

    /// Whether the button reacts to clicks.
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

impl<M: Clone + 'static> Widget<M> for ToolButton<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let side = (ICON_SIZE + 2.0 * cx.theme().padding) * cx.scale_factor();
        Ok(constraints.constrain(Size::new(side, side)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let status = cx.status();
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let face = if !self.enabled {
            None
        } else if status.active && self.pressed {
            Some(theme.button_active)
        } else if self.checked {
            Some(theme.button_checked)
        } else if status.hot {
            Some(theme.button_hot)
        } else {
            None
        };
        let side = rect.width.min(rect.height);
        let square = Rect::new(
            rect.center().x - side / 2.0,
            rect.center().y - side / 2.0,
            side,
            side,
        );
        if let Some(face) = face {
            cx.canvas().fill_rounded_rect(
                square.inset(scale),
                theme.corner_radius * scale,
                face,
            )?;
        }
        let color = if !self.enabled {
            theme.text_disabled
        } else if self.checked {
            theme.accent
        } else {
            theme.text
        };
        let icon = square.inset(theme.padding * scale);
        self.icon.paint(cx.canvas(), icon, color)
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        let clicking = matches!(
            event,
            UiEvent::PointerDown { .. } | UiEvent::PointerUp { .. }
        );
        if clicking && !self.enabled {
            return;
        }
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
            UiEvent::PointerEnter(_) => {
                if let Some((enter, _)) = &self.hover {
                    cx.emit(enter.clone());
                }
            }
            UiEvent::PointerLeave => {
                if let Some((_, leave)) = &self.hover {
                    cx.emit(leave.clone());
                }
            }
            _ => {}
        }
    }
}
