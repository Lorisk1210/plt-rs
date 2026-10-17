//! The toolbar below the canvas (matplotlib's `NavigationToolbar2`):
//! navigation and save buttons, and the status line.

use plt_platform::Context;
use plt_ui::{Flex, Label, Separator, ToolButton, UiWindow, WidgetId};

use super::Message;
use crate::gui::{GuiError, icons};
use crate::navigation::{NavMode, Navigation};

/// Height of the toolbar in logical pixels, separator included.
pub(super) const HEIGHT: f64 = 37.0;

/// A toolbar button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui) enum Tool {
    /// Restore the original view.
    Home,
    /// Previous view.
    Back,
    /// Next view.
    Forward,
    /// Toggle the pan tool.
    Pan,
    /// Toggle the zoom tool.
    Zoom,
    /// Open the subplot configuration tool.
    Subplots,
    /// Save the figure as PNG.
    Save,
}

impl Tool {
    /// What the tool does and its shortcut, for the status line.
    pub(super) fn hint(self) -> &'static str {
        match self {
            Self::Home => "Reset original view (h)",
            Self::Back => "Back to previous view (c)",
            Self::Forward => "Forward to next view (v)",
            Self::Pan => {
                "Left button pans, right button zooms; x/y fixes axis, Ctrl fixes aspect (p)"
            }
            Self::Zoom => "Zoom to rectangle; x/y fixes axis (o)",
            Self::Subplots => "Configure subplots",
            Self::Save => "Save the figure as PNG (s)",
        }
    }
}

/// The widgets of the toolbar that change after it is built.
#[derive(Debug, Clone, Copy)]
pub(super) struct Toolbar {
    back: WidgetId,
    forward: WidgetId,
    pan: WidgetId,
    zoom: WidgetId,
    status: WidgetId,
}

impl Toolbar {
    /// Appends a separator and the toolbar to `root`, in matplotlib's
    /// button order.
    pub(super) fn build(root: &mut Flex<Message>) -> Self {
        let mut row = Flex::row().with_padding(2.0).with_spacing(2.0);
        let button = |row: &mut Flex<Message>, icon, tool| {
            let button = ToolButton::new(icon, Message::Tool(tool))
                .with_hover_messages(Message::Hint(Some(tool)), Message::Hint(None));
            row.push(button, 0.0)
        };
        button(&mut row, icons::home(), Tool::Home);
        let back = button(&mut row, icons::back(), Tool::Back);
        let forward = button(&mut row, icons::forward(), Tool::Forward);
        row.push(Separator::vertical(), 0.0);
        let pan = button(&mut row, icons::pan(), Tool::Pan);
        let zoom = button(&mut row, icons::zoom(), Tool::Zoom);
        button(&mut row, icons::subplots(), Tool::Subplots);
        row.push(Separator::vertical(), 0.0);
        button(&mut row, icons::save(), Tool::Save);
        let status = row.push(Label::new(""), 1.0);
        root.push(Separator::horizontal(), 0.0);
        root.push(row, 0.0);
        Self {
            back,
            forward,
            pan,
            zoom,
            status,
        }
    }

    /// Enables the history buttons that lead somewhere and checks the
    /// active tool.
    pub(super) fn refresh(
        &self,
        window: &mut UiWindow<Message>,
        cx: &mut Context<'_>,
        navigation: &Navigation,
    ) -> Result<(), GuiError> {
        let mode = navigation.mode();
        let states = [
            (self.back, navigation.can_go_back(), false),
            (self.forward, navigation.can_go_forward(), false),
            (self.pan, true, mode == NavMode::Pan),
            (self.zoom, true, mode == NavMode::Zoom),
        ];
        for (id, enabled, checked) in states {
            window.update(cx, id, |button: &mut ToolButton<Message>| {
                button.set_enabled(enabled);
                button.set_checked(checked);
            })?;
        }
        Ok(())
    }

    /// Shows `text` in the status line.
    pub(super) fn set_status(
        &self,
        window: &mut UiWindow<Message>,
        cx: &mut Context<'_>,
        text: String,
    ) -> Result<(), GuiError> {
        window.update(cx, self.status, |label: &mut Label| label.set_text(text))?;
        Ok(())
    }
}
