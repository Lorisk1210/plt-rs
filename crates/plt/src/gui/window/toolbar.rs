//! The toolbar below the canvas (matplotlib's `NavigationToolbar2`):
//! navigation and save buttons, the figure's own tools, help, and the
//! status line.

use plt_platform::Context;
use plt_ui::{Flex, Label, Separator, ToolButton, UiWindow, WidgetId};

use super::Message;
use crate::gui::tools::{ToolKind, ToolSpec};
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
    /// Open the figure options dialog.
    Options,
    /// Open the save dialog.
    Save,
    /// Show the keyboard shortcuts.
    Help,
    /// The figure's tool at this position.
    User(usize),
}

impl Tool {
    /// What a built-in tool does and its shortcut, for the status line.
    fn builtin_hint(self) -> &'static str {
        match self {
            Self::Home => "Reset original view (h)",
            Self::Back => "Back to previous view (c)",
            Self::Forward => "Forward to next view (v)",
            Self::Pan => {
                "Left button pans, right button zooms; x/y fixes axis, Ctrl fixes aspect (p)"
            }
            Self::Zoom => "Zoom to rectangle; x/y fixes axis (o)",
            Self::Subplots => "Configure subplots",
            Self::Options => "Edit axis, curve and image parameters",
            Self::Save => "Save the figure (s)",
            Self::Help => "Keyboard shortcuts (F1)",
            Self::User(_) => "",
        }
    }
}

/// The widgets of the toolbar that change after it is built.
#[derive(Debug, Clone)]
pub(super) struct Toolbar {
    back: WidgetId,
    forward: WidgetId,
    pan: WidgetId,
    zoom: WidgetId,
    status: WidgetId,
    /// The buttons of the figure's tools, with the tools as built.
    user: Vec<(WidgetId, ToolSpec)>,
}

impl Toolbar {
    /// Appends a separator and the toolbar to `root`, in matplotlib's
    /// button order, the figure's `tools` after the built-in buttons.
    pub(super) fn build(root: &mut Flex<Message>, tools: Vec<ToolSpec>) -> Self {
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
        button(&mut row, icons::options(), Tool::Options);
        row.push(Separator::vertical(), 0.0);
        button(&mut row, icons::save(), Tool::Save);
        if !tools.is_empty() {
            row.push(Separator::vertical(), 0.0);
        }
        let user = tools
            .into_iter()
            .enumerate()
            .map(|(index, tool)| (button(&mut row, tool.icon.clone(), Tool::User(index)), tool))
            .collect();
        row.push(Separator::vertical(), 0.0);
        button(&mut row, icons::help(), Tool::Help);
        let status = row.push(Label::new(""), 1.0);
        root.push(Separator::horizontal(), 0.0);
        root.push(row, 0.0);
        Self {
            back,
            forward,
            pan,
            zoom,
            status,
            user,
        }
    }

    /// The name of the figure's tool behind [`Tool::User`] `index`.
    pub(super) fn user_tool(&self, index: usize) -> Option<&str> {
        self.user.get(index).map(|(_, tool)| tool.name.as_str())
    }

    /// What `tool` does and its shortcuts, for the status line.
    pub(super) fn hint(&self, tool: Tool) -> String {
        let Tool::User(index) = tool else {
            return tool.builtin_hint().to_owned();
        };
        self.user.get(index).map_or_else(String::new, |(_, tool)| {
            if tool.keys.is_empty() {
                tool.description.clone()
            } else {
                format!("{} ({})", tool.description, tool.keys.join(", "))
            }
        })
    }

    /// Enables the history buttons that lead somewhere and checks the
    /// active navigation tool and the toggles of the figure's tools that
    /// are on (`toggled` lists their names).
    pub(super) fn refresh(
        &self,
        window: &mut UiWindow<Message>,
        cx: &mut Context<'_>,
        navigation: &Navigation,
        toggled: &[String],
    ) -> Result<(), GuiError> {
        let mode = navigation.mode();
        let builtin = [
            (self.back, navigation.can_go_back(), false),
            (self.forward, navigation.can_go_forward(), false),
            (self.pan, true, mode == NavMode::Pan),
            (self.zoom, true, mode == NavMode::Zoom),
        ];
        let user = self.user.iter().map(|(id, tool)| {
            let on = tool.kind == ToolKind::Toggle && toggled.contains(&tool.name);
            (*id, true, on)
        });
        for (id, enabled, checked) in builtin.into_iter().chain(user) {
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
