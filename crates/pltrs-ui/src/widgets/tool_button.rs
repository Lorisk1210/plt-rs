//! Toolbar buttons: an icon that can be checked (a mode is on) or
//! disabled (nothing to do).

use pltrs_platform::{ButtonState, Key, MouseButton};

use super::frame::paint_focus_ring;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::picture::Picture;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// How opaque the picture of a disabled button is: greyed out, as Tk
/// stipples the images of disabled buttons.
const DISABLED_OPACITY: f64 = 0.4;

/// What a [`ToolButton`] shows.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolIcon {
    /// A vector icon painted in the theme's colours: the text colour, the
    /// accent text colour when checked, the disabled text colour when
    /// disabled. It is the theme's
    /// [`icon_size`](crate::Theme::icon_size) wide.
    Vector(Icon),
    /// A full-colour picture drawn as it is, greyed out when disabled, at
    /// its own size.
    Picture(Picture),
}

impl From<Icon> for ToolIcon {
    fn from(icon: Icon) -> Self {
        Self::Vector(icon)
    }
}

impl From<Picture> for ToolIcon {
    fn from(picture: Picture) -> Self {
        Self::Picture(picture)
    }
}

/// An icon button that sends a message when clicked (pressed and released
/// with the left button over it), or when Space is pressed while it has
/// the focus. It shows a vector icon or a full-colour picture (see
/// [`ToolIcon`]). It is flat until the pointer is over it; a checked
/// button shows an accent wash (and an accent vector icon), and a disabled
/// one is greyed out and ignores clicks. It may have a
/// [tooltip](ToolButton::with_tooltip).
///
/// As in Qt, Tab gives tool buttons the focus but clicks do not, so that
/// clicking a toolbar leaves the focus (and the keys) where it was; the
/// other keys a focused tool button receives go on to shortcuts.
#[derive(Debug)]
pub struct ToolButton<M> {
    icon: ToolIcon,
    message: M,
    /// Messages sent when the pointer enters and leaves the button.
    hover: Option<(M, M)>,
    tooltip: Option<String>,
    checked: bool,
    enabled: bool,
    /// Whether the left button went down on this button and is held.
    pressed: bool,
}

impl<M> ToolButton<M> {
    /// An enabled, unchecked button showing `icon` that sends `message`.
    pub fn new(icon: impl Into<ToolIcon>, message: M) -> Self {
        Self {
            icon: icon.into(),
            message,
            hover: None,
            tooltip: None,
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

    /// Shows `text` in a tooltip when the pointer rests on the button.
    #[must_use]
    pub fn with_tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
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
        let side = cx.theme().tool_button_size * cx.scale_factor();
        Ok(constraints.constrain(Size::new(side, side)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let status = cx.status();
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let side = rect.width.min(rect.height);
        let square = rect.centered(Size::new(side, side));
        let radius = theme.corner_radius * scale;
        if self.enabled {
            if self.checked {
                cx.canvas()
                    .fill_rounded_rect(square, radius, theme.selection)?;
            }
            let wash = if status.active && self.pressed {
                Some(theme.pressed)
            } else if status.hot {
                Some(theme.hover)
            } else {
                None
            };
            if let Some(wash) = wash {
                cx.canvas().fill_rounded_rect(square, radius, wash)?;
            }
        }
        match &self.icon {
            ToolIcon::Vector(icon) => {
                let color = if !self.enabled {
                    theme.text_disabled
                } else if self.checked {
                    theme.accent_text
                } else {
                    theme.text
                };
                let side = theme.icon_size * scale;
                icon.paint(cx.canvas(), square.centered(Size::new(side, side)), color)?;
            }
            ToolIcon::Picture(picture) => {
                let opacity = if self.enabled { 1.0 } else { DISABLED_OPACITY };
                picture.paint(cx.canvas(), square, opacity)?;
            }
        }
        if status.focus_visible() {
            paint_focus_ring(cx, square, radius)?;
        }
        Ok(())
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
            UiEvent::Key(key)
                if self.enabled
                    && key.state == ButtonState::Pressed
                    && key.key == Key::Character(' ')
                    && !key.modifiers.control()
                    && !key.modifiers.alt() =>
            {
                if !key.repeat {
                    cx.emit(self.message.clone());
                }
                cx.set_handled();
            }
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        self.enabled
    }

    fn focus_on_click(&self) -> bool {
        false
    }

    fn tooltip(&self) -> Option<&str> {
        self.tooltip.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use pltrs_platform::PhysicalSize;
    use pltrs_raster::{Pixmap, Rgba8};

    use super::*;
    use crate::Ui;

    #[test]
    fn pictures_are_drawn_as_they_are_and_greyed_out_when_disabled() {
        let black = Pixmap::filled(24, 24, Rgba8::new(0, 0, 0, 255)).unwrap();
        let button = ToolButton::new(Picture::new(24.0, vec![black]), ());
        let mut ui = Ui::new(button, PhysicalSize::new(32, 32), 1.0).unwrap();
        let centre = |ui: &mut Ui<()>| {
            let frame = ui.paint().unwrap();
            frame.pixels()[(16 * 32 + 16) * 4]
        };
        assert_eq!(centre(&mut ui), 0, "black, as drawn");
        let root = ui.root_id();
        if let Some(button) = ui.widget_mut::<ToolButton<()>>(root) {
            button.set_enabled(false);
        }
        let background = ui.theme().background.r * 255.0;
        let greyed = f64::from(centre(&mut ui));
        assert!(
            (greyed - background * (1.0 - DISABLED_OPACITY)).abs() < 2.0,
            "faded towards the background: {greyed}"
        );
    }
}
