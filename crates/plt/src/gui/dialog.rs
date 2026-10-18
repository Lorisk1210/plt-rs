//! What the windows opened from a figure window's toolbar share: their
//! window options, margins, and the footer holding their buttons.

use plt_platform::{ButtonState, Key, WindowEvent, WindowOptions};
use plt_ui::{Button, Flex, Label, Separator, Spacing, Tone, Ui, Widget, WidgetId};

use super::{GuiError, icons};

/// The space around a dialog's content.
pub(super) const MARGIN: Spacing = Spacing::Lg;
/// The space between the parts of a dialog's content.
pub(super) const GAP: Spacing = Spacing::Md;

/// The options of a plt-rs window titled `title`: the application id
/// and icon of plt-rs.
pub(super) fn window_options(title: &str) -> Result<WindowOptions, GuiError> {
    Ok(WindowOptions::new(title)
        .with_app_id("plt-rs")
        .with_icon(icons::window_icon()?))
}

/// Whether `event` asks the dialog showing `ui` to close without acting:
/// the window's close button, or Escape unless it closes a popup.
pub(super) fn dismissed<M: 'static>(event: &WindowEvent, ui: &Ui<M>) -> bool {
    match event {
        WindowEvent::CloseRequested => true,
        WindowEvent::Key(key) => {
            key.key == Key::Escape && key.state == ButtonState::Pressed && !ui.has_popup()
        }
        _ => false,
    }
}

/// A dialog being built: its footer, below the content, holds secondary
/// buttons and then the primary one on the right.
pub(super) struct Dialog<M> {
    footer: Flex<M>,
}

impl<M: Clone + 'static> Dialog<M> {
    /// A dialog whose footer shows problems on its left; returns the
    /// status line's id too.
    pub(super) fn with_status() -> (Self, WidgetId) {
        let mut footer = Self::footer_row();
        let status = footer.push(Label::new("").with_tone(Tone::Danger), 1.0);
        (Self { footer }, status)
    }

    /// A dialog whose footer only holds buttons.
    pub(super) fn without_status() -> Self {
        let mut footer = Self::footer_row();
        footer.push_spacer(1.0);
        Self { footer }
    }

    fn footer_row() -> Flex<M> {
        Flex::row()
            .with_padding_xy(MARGIN, Spacing::Md)
            .with_spacing(Spacing::Sm)
    }

    /// Adds a secondary button labelled `label` sending `message`.
    pub(super) fn button(&mut self, label: &str, message: M) {
        self.footer.push(Button::new(label, message), 0.0);
    }

    /// Adds the primary button, last: labelled `label`, sending
    /// `message`.
    pub(super) fn primary(&mut self, label: &str, message: M) {
        self.footer.push(Button::new(label, message).primary(), 0.0);
    }

    /// The dialog's root: `content` within the dialog's margins, a
    /// hairline and the footer.
    pub(super) fn finish(self, content: impl Widget<M>) -> Flex<M> {
        let body = Flex::column()
            .with_padding(MARGIN)
            .with_flex_child(content, 1.0);
        Flex::column()
            .with_flex_child(body, 1.0)
            .with_child(Separator::horizontal())
            .with_child(self.footer)
    }
}

#[cfg(test)]
mod tests {
    use plt_platform::{KeyEvent, Modifiers, MouseButton, PhysicalPosition, PhysicalSize};
    use plt_ui::Dropdown;

    use super::*;

    #[test]
    fn escape_closes_dialogs_unless_a_popup_is_open() {
        let escape = WindowEvent::Key(KeyEvent {
            key: Key::Escape,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers: Modifiers::NONE,
            scancode: 9,
        });
        let choices = Dropdown::new(vec!["Axes 1".into(), "Axes 2".into()], |_| ());
        let (mut chrome, _) = Dialog::with_status();
        chrome.primary("OK", ());
        let mut ui = Ui::new(chrome.finish(choices), PhysicalSize::new(300, 200), 1.0).unwrap();
        assert!(dismissed(&escape, &ui));
        assert!(dismissed(&WindowEvent::CloseRequested, &ui));
        assert!(!dismissed(&WindowEvent::RedrawRequested, &ui));
        let press = WindowEvent::MouseInput {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            position: PhysicalPosition::new(30.0, 30.0),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        ui.handle(&press).unwrap();
        assert!(ui.has_popup(), "the press opened the drop-down");
        assert!(!dismissed(&escape, &ui), "Escape closes the list first");
        ui.handle(&escape).unwrap();
        assert!(!ui.has_popup());
    }
}
