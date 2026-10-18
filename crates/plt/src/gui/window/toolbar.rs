//! The toolbar below the canvas (matplotlib's `NavigationToolbar2`):
//! navigation and save buttons, the figure's own tools, help, and the
//! status line.

use plt_platform::Context;
use plt_ui::{
    Align, Flex, Label, Overflow, Separator, Spacing, Theme, Tone, ToolButton, UiWindow, WidgetId,
};

use super::Message;
use crate::gui::tools::{ToolKind, ToolSpec};
use crate::gui::{GuiError, icons};
use crate::navigation::{NavMode, Navigation};

/// The space left and right of the toolbar's contents.
const PADDING_X: Spacing = Spacing::Sm;
/// The space above and below the buttons.
const PADDING_Y: Spacing = Spacing::Xs;
/// The space between neighbouring buttons.
const BUTTON_GAP: Spacing = Spacing::Xxs;
/// The space between the last button and the status line.
const STATUS_GAP: Spacing = Spacing::Md;

/// Height of the toolbar in logical pixels, the hairline above it
/// included.
pub(super) fn height() -> f64 {
    let theme = Theme::default();
    theme.tool_button_size + 2.0 * theme.space(PADDING_Y) + 1.0
}

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
    /// A built-in tool's name and shortcut, for its tooltip.
    fn builtin_tooltip(self) -> &'static str {
        match self {
            Self::Home => "Home (h)",
            Self::Back => "Back (c)",
            Self::Forward => "Forward (v)",
            Self::Pan => "Pan (p)",
            Self::Zoom => "Zoom (o)",
            Self::Subplots => "Subplots",
            Self::Options => "Figure options",
            Self::Save => "Save (s)",
            Self::Help => "Help (F1)",
            Self::User(_) => "",
        }
    }

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
        let mut row = Flex::row()
            .with_padding_xy(PADDING_X, PADDING_Y)
            .with_spacing(BUTTON_GAP);
        let button = |row: &mut Flex<Message>, icon, tool: Tool, tooltip: String| {
            let button = ToolButton::new(icon, Message::Tool(tool))
                .with_hover_messages(Message::Hint(Some(tool)), Message::Hint(None))
                .with_tooltip(tooltip);
            row.push(button, 0.0)
        };
        let builtin = |row: &mut Flex<Message>, icon, tool: Tool| {
            button(row, icon, tool, tool.builtin_tooltip().to_owned())
        };
        builtin(&mut row, icons::home(), Tool::Home);
        let back = builtin(&mut row, icons::back(), Tool::Back);
        let forward = builtin(&mut row, icons::forward(), Tool::Forward);
        row.push(Separator::vertical(), 0.0);
        let pan = builtin(&mut row, icons::pan(), Tool::Pan);
        let zoom = builtin(&mut row, icons::zoom(), Tool::Zoom);
        builtin(&mut row, icons::subplots(), Tool::Subplots);
        builtin(&mut row, icons::options(), Tool::Options);
        row.push(Separator::vertical(), 0.0);
        builtin(&mut row, icons::save(), Tool::Save);
        if !tools.is_empty() {
            row.push(Separator::vertical(), 0.0);
        }
        let user: Vec<(WidgetId, ToolSpec)> = tools
            .into_iter()
            .enumerate()
            .map(|(index, tool)| {
                let tooltip = user_hint(&tool);
                let id = button(&mut row, tool.icon.clone(), Tool::User(index), tooltip);
                (id, tool)
            })
            .collect();
        row.push(Separator::vertical(), 0.0);
        builtin(&mut row, icons::help(), Tool::Help);
        let mut status_area = Flex::row().with_padding_xy(STATUS_GAP, Spacing::None);
        let status = status_area.push(
            Label::new("")
                .with_tone(Tone::Secondary)
                .with_align(Align::End)
                .with_overflow(Overflow::ElideEnd),
            1.0,
        );
        row.push(status_area, 1.0);
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
        self.user
            .get(index)
            .map_or_else(String::new, |(_, tool)| user_hint(tool))
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

/// What a figure's tool does and its shortcuts.
fn user_hint(tool: &ToolSpec) -> String {
    if tool.keys.is_empty() {
        tool.description.clone()
    } else {
        format!("{} ({})", tool.description, tool.keys.join(", "))
    }
}
