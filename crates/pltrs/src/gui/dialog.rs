//! What the windows opened from a figure window's toolbar share: their
//! window options, theme, margins, the footer holding their buttons, and
//! their keyboard conventions (Qt's and GTK's): the first field has the
//! focus when a dialog opens, Tab moves it, Enter presses the primary
//! button unless the focused widget uses Enter, and Escape closes the
//! dialog unless it closes an open drop-down.

use pltrs_platform::{ButtonState, Context, Key, LogicalSize, WindowEvent, WindowOptions};
use pltrs_ui::{
    Button, Flex, Label, Overflow, Separator, Spacing, Theme, Tone, Ui, UiWindow, Widget, WidgetId,
};

use super::{GuiError, app_icon};
use crate::colors::Rgba;
use crate::figure::Figure;

/// The space around a dialog's content.
pub(super) const MARGIN: Spacing = Spacing::Lg;
/// The space between the parts of a dialog's content.
pub(super) const GAP: Spacing = Spacing::Md;

/// The options of a pltrs window titled `title`: the application id
/// and icon of pltrs.
pub(super) fn window_options(title: &str) -> WindowOptions {
    WindowOptions::new(title)
        .with_app_id("pltrs")
        .with_icon(app_icon::app_icon())
}

/// The theme of the windows of `figure`: the chrome [look](crate::look)
/// for the figure's background, taken as white when it does not resolve
/// (drawing the figure reports it).
pub(super) fn chrome_theme(figure: &Figure) -> Theme {
    let background = figure.facecolor().resolve().unwrap_or(Rgba::WHITE);
    crate::look::appearance().chrome.theme(background)
}

/// Opens a dialog of the window of `figure` in the figure's
/// [theme](chrome_theme), at its content's natural size in that theme,
/// with the focus on its first field.
pub(super) fn open_fitted<M: 'static>(
    cx: &mut Context<'_>,
    figure: &Figure,
    options: &WindowOptions,
    root: impl Widget<M>,
) -> Result<UiWindow<M>, GuiError> {
    let unlimited = LogicalSize::new(f64::INFINITY, f64::INFINITY);
    open_fitted_within(cx, figure, options, root, unlimited)
}

/// Opens a dialog as [`open_fitted`] does, but no larger than `max`
/// logical pixels: content that does not fit scrolls (see
/// [`pltrs_ui::Scroll`]).
pub(super) fn open_fitted_within<M: 'static>(
    cx: &mut Context<'_>,
    figure: &Figure,
    options: &WindowOptions,
    root: impl Widget<M>,
    max: LogicalSize,
) -> Result<UiWindow<M>, GuiError> {
    let theme = chrome_theme(figure);
    let mut window = UiWindow::open_fitted_within(cx, options, root, theme, max)?;
    window.ui_mut().focus_first();
    Ok(window)
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
    /// A dialog whose footer shows problems on its left, shortened with
    /// an ellipsis where they would reach the buttons; returns the status
    /// line's id too.
    pub(super) fn with_status() -> (Self, WidgetId) {
        let mut footer = Self::footer_row();
        let status = Label::new("")
            .with_tone(Tone::Danger)
            .with_overflow(Overflow::ElideEnd);
        let status = footer.push(status, 1.0);
        (Self { footer }, status)
    }

    /// A dialog whose footer shows `note` on its left, in the quieter
    /// secondary tone (the dialog is made wide enough for it).
    pub(super) fn with_note(note: &str) -> Self {
        let mut footer = Self::footer_row();
        footer.push(Label::new(note).with_tone(Tone::Secondary), 1.0);
        Self { footer }
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
    /// `message`. Enter presses it (see [`pltrs_ui::Button::primary`]).
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
    use pltrs_platform::{KeyEvent, Modifiers, MouseButton, PhysicalPosition, PhysicalSize};
    use pltrs_ui::Dropdown;

    use super::*;

    #[test]
    fn windows_take_the_chrome_look() {
        let figure = Figure::new();
        figure.set_facecolor("black");
        let chrome = crate::look::appearance().chrome;
        assert_eq!(chrome_theme(&figure), chrome.theme(Rgba::BLACK));
        figure.set_facecolor("not a colour");
        assert_eq!(chrome_theme(&figure), chrome.theme(Rgba::WHITE));
    }

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
