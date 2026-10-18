//! The help window of figure windows (matplotlib's `ToolHelp`): every
//! keyboard shortcut, from the `keymap.*` rcParams and the figure's own
//! tools.

use plt_platform::{ButtonState, Context, Key, LogicalSize, WindowEvent, WindowId, WindowOptions};
use plt_ui::{Button, Flex, Label, UiWindow};

use super::tools::ToolSpec;
use super::{GuiError, icons};
use crate::rc::RcParams;

/// Width of the shortcut column in logical pixels.
const KEYS_WIDTH: f64 = 170.0;

/// What the help window's widgets report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HelpMessage {
    /// Close the window.
    Close,
}

/// The shortcuts listed: what they do and their keys.
fn shortcuts(rc: &RcParams, tools: &[ToolSpec]) -> Vec<(String, Vec<String>)> {
    let builtin = [
        ("Reset original view", &rc.keymap_home),
        ("Back to previous view", &rc.keymap_back),
        ("Forward to next view", &rc.keymap_forward),
        ("Pan and zoom with the mouse", &rc.keymap_pan),
        ("Zoom to a rectangle", &rc.keymap_zoom),
        ("Save the figure", &rc.keymap_save),
        ("Toggle full screen", &rc.keymap_fullscreen),
        ("Cycle the major grids", &rc.keymap_grid),
        ("Cycle the minor grids", &rc.keymap_grid_minor),
        ("Toggle a log x axis", &rc.keymap_xscale),
        ("Toggle a log y axis", &rc.keymap_yscale),
        ("Close the window", &rc.keymap_quit),
        ("Show this help", &rc.keymap_help),
    ];
    let keyboard = |keys: &[String]| -> Vec<String> {
        keys.iter()
            .filter(|key| !key.starts_with("MouseButton."))
            .cloned()
            .collect()
    };
    builtin
        .into_iter()
        .map(|(action, keys)| (action.to_owned(), keyboard(keys)))
        .chain(
            tools
                .iter()
                .map(|tool| (tool.description.clone(), keyboard(&tool.keys))),
        )
        .filter(|(_, keys)| !keys.is_empty())
        .collect()
}

/// A window listing the keyboard shortcuts of a figure window.
#[derive(Debug)]
pub(super) struct HelpDialog {
    window: UiWindow<HelpMessage>,
}

impl HelpDialog {
    /// Opens the help for a window whose figure has `tools`.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub(super) fn open(cx: &mut Context<'_>, tools: &[ToolSpec]) -> Result<Self, GuiError> {
        let mut root = Flex::column().with_padding(12.0).with_spacing(6.0);
        let mut header = Flex::row().with_spacing(8.0);
        header.push(
            Label::new("Shortcut").bold().with_min_width(KEYS_WIDTH),
            0.0,
        );
        header.push(Label::new("Action").bold(), 1.0);
        root.push(header, 0.0);
        let rows = shortcuts(&crate::rc::params(), tools);
        for (action, keys) in &rows {
            let mut row = Flex::row().with_spacing(8.0);
            row.push(Label::new(keys.join(", ")).with_min_width(KEYS_WIDTH), 0.0);
            row.push(Label::new(action.clone()), 1.0);
            root.push(row, 0.0);
        }
        let mut buttons = Flex::row();
        buttons.push(Label::new(""), 1.0);
        buttons.push(Button::new("Close", HelpMessage::Close).primary(), 0.0);
        root.push(buttons, 0.0);
        let height = 90.0 + 26.0 * f64::from(u32::try_from(rows.len()).unwrap_or(u32::MAX));
        let options = WindowOptions::new("Help")
            .with_app_id("plt-rs")
            .with_icon(icons::window_icon()?)
            .with_inner_size(LogicalSize::new(480.0, height))
            .with_min_inner_size(LogicalSize::new(320.0, 200.0));
        Ok(Self {
            window: UiWindow::open(cx, &options, root)?,
        })
    }

    /// The window.
    pub(super) fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Handles an event of the help window; returns whether it stays open.
    ///
    /// # Errors
    ///
    /// When the window cannot be updated.
    pub(super) fn handle(
        &mut self,
        cx: &mut Context<'_>,
        event: &WindowEvent,
    ) -> Result<bool, GuiError> {
        match event {
            WindowEvent::CloseRequested => return Ok(false),
            WindowEvent::Key(key)
                if key.key == Key::Escape && key.state == ButtonState::Pressed =>
            {
                return Ok(false);
            }
            _ => {}
        }
        let open = !self.window.handle(cx, event)?.contains(&HelpMessage::Close);
        cx.request_redraw(self.window.id())?;
        Ok(open)
    }

    /// Closes the window.
    ///
    /// # Errors
    ///
    /// When the window system fails to close it.
    pub(super) fn close(self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        cx.close_window(self.id())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use plt_ui::Icon;

    use super::*;
    use crate::gui::tools::ToolKind;

    #[test]
    fn shortcuts_list_the_keymaps_and_the_tools() {
        let tool = ToolSpec {
            name: "mark".into(),
            description: "Mark the peak".into(),
            icon: Icon::new(24.0),
            keys: vec!["m".into()],
            kind: ToolKind::Trigger,
        };
        let rows = shortcuts(&RcParams::matplotlib(), &[tool]);
        let back = rows
            .iter()
            .find(|(action, _)| action == "Back to previous view");
        assert_eq!(
            back.map(|(_, keys)| keys.clone()),
            Some(vec!["left".into(), "c".into(), "backspace".into()]),
            "mouse buttons are not keys"
        );
        assert_eq!(
            rows.last(),
            Some(&("Mark the peak".into(), vec!["m".into()]))
        );
        assert!(
            !rows.iter().any(|(_, keys)| keys.is_empty()),
            "unbound actions are left out"
        );
    }
}
