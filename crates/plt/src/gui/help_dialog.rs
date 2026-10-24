//! The help window of figure windows (matplotlib's `ToolHelpBase`): every
//! keyboard shortcut, from the `keymap.*` rcParams and the figure's own
//! tools.

use plt_platform::{Context, WindowEvent, WindowId};
use plt_ui::{Form, Keycaps, UiWindow};

use super::GuiError;
use super::dialog::{self, Dialog};
use super::tools::ToolSpec;
use crate::figure::Figure;
use crate::rc::RcParams;

/// What the help window's widgets report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HelpMessage {
    /// Close the window.
    Close,
}

/// A shortcut: what it does and its keys.
type Shortcut = (String, Vec<String>);

/// The shortcuts listed, in groups: the group's name and its shortcuts.
/// Unbound actions and empty groups are left out.
fn shortcuts(rc: &RcParams, tools: &[ToolSpec]) -> Vec<(&'static str, Vec<Shortcut>)> {
    let keyboard = |keys: &[String]| -> Vec<String> {
        keys.iter()
            .filter(|key| !key.starts_with("MouseButton."))
            .cloned()
            .collect()
    };
    let group = |actions: &[(&str, &Vec<String>)]| -> Vec<Shortcut> {
        actions
            .iter()
            .map(|&(action, keys)| (action.to_owned(), keyboard(keys)))
            .collect()
    };
    let groups = [
        (
            "Navigation",
            group(&[
                ("Reset original view", &rc.keymap_home),
                ("Back to previous view", &rc.keymap_back),
                ("Forward to next view", &rc.keymap_forward),
                ("Pan and zoom with the mouse", &rc.keymap_pan),
                ("Zoom to a rectangle", &rc.keymap_zoom),
            ]),
        ),
        (
            "Figure",
            group(&[
                ("Save the figure", &rc.keymap_save),
                ("Copy the figure to the clipboard", &rc.keymap_copy),
                ("Toggle full screen", &rc.keymap_fullscreen),
            ]),
        ),
        (
            "Axes under the pointer",
            group(&[
                ("Cycle the major grids", &rc.keymap_grid),
                ("Cycle the minor grids", &rc.keymap_grid_minor),
                ("Toggle a log x axis", &rc.keymap_xscale),
                ("Toggle a log y axis", &rc.keymap_yscale),
            ]),
        ),
        (
            "Window",
            group(&[
                ("Close the window", &rc.keymap_quit),
                ("Close every figure window", &rc.keymap_quit_all),
                ("Show this help", &rc.keymap_help),
            ]),
        ),
        (
            "Tools",
            tools
                .iter()
                .map(|tool| (tool.description.clone(), keyboard(&tool.keys)))
                .collect(),
        ),
    ];
    groups
        .into_iter()
        .map(|(name, rows)| {
            let bound = rows.into_iter().filter(|(_, keys)| !keys.is_empty());
            (name, bound.collect::<Vec<_>>())
        })
        .filter(|(_, rows)| !rows.is_empty())
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
    pub(super) fn open(cx: &mut Context<'_>, figure: &Figure) -> Result<Self, GuiError> {
        let mut form = Form::new();
        for (group, rows) in shortcuts(&crate::rc::params(), &figure.tool_specs()) {
            form.section(group);
            for (action, keys) in rows {
                form.row(action, Keycaps::new(keys));
            }
        }
        let mut chrome = Dialog::without_status();
        chrome.primary("Close", HelpMessage::Close);
        let options = dialog::window_options("Help")?;
        Ok(Self {
            window: dialog::open_fitted(cx, figure, &options, chrome.finish(form))?,
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
        if dialog::dismissed(event, self.window.ui()) {
            return Ok(false);
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
        let groups = shortcuts(&RcParams::matplotlib(), &[tool]);
        let rows: Vec<&Shortcut> = groups.iter().flat_map(|(_, rows)| rows).collect();
        let back = rows
            .iter()
            .find(|(action, _)| action == "Back to previous view");
        assert_eq!(
            back.map(|(_, keys)| keys.clone()),
            Some(vec!["left".into(), "c".into(), "backspace".into()]),
            "mouse buttons are not keys"
        );
        assert_eq!(
            groups.last(),
            Some(&("Tools", vec![("Mark the peak".into(), vec!["m".into()])]))
        );
        assert!(
            !rows.iter().any(|(_, keys)| keys.is_empty()),
            "unbound actions are left out"
        );
        let without_tools = shortcuts(&RcParams::matplotlib(), &[]);
        assert!(
            without_tools
                .iter()
                .all(|(name, rows)| *name != "Tools" && !rows.is_empty()),
            "empty groups are left out"
        );
    }
}
