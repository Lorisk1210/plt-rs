//! Tools a program adds to its figure windows (matplotlib's
//! `ToolManager.add_tool` and `ToolContainerBase.add_tool`): toolbar
//! buttons with keyboard shortcuts that run a callback, once per use or as
//! an on/off toggle.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use plt_ui::Icon;

use crate::artist::ArtistHandle;
use crate::figure::Figure;

/// How a tool behaves when used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToolKind {
    /// Every use runs the callback (matplotlib's `ToolBase`).
    #[default]
    Trigger,
    /// Every use switches the tool on or off and runs the callback; the
    /// button shows the state (matplotlib's `ToolToggleBase`).
    Toggle,
}

/// What a tool looks like and how it is reached.
#[derive(Clone, Debug)]
pub struct ToolSpec {
    /// The tool's name, unique within its figure.
    pub name: String,
    /// What the tool does, shown in the status line and the help.
    pub description: String,
    /// The toolbar icon (see [`Icon`], drawn on a grid like the built-in
    /// icons).
    pub icon: Icon,
    /// Keyboard shortcuts in matplotlib's key names (`"ctrl+l"`, `"G"`).
    pub keys: Vec<String>,
    /// Whether the tool is a trigger or a toggle.
    pub kind: ToolKind,
}

/// A use of a tool, as its callback receives it.
#[derive(Debug)]
pub struct ToolEvent<'a> {
    /// The figure whose window the tool was used in.
    pub figure: &'a Figure,
    /// The tool's name.
    pub name: &'a str,
    /// For toggles, whether the tool is now on.
    pub toggled: bool,
}

/// Why a tool could not be added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolError {
    /// A tool of the figure already has this name.
    Duplicate(String),
    /// The name is empty.
    EmptyName,
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(name) => write!(f, "the figure already has a tool named {name:?}"),
            Self::EmptyName => f.write_str("a tool needs a name"),
        }
    }
}

impl std::error::Error for ToolError {}

/// A tool's callback.
type Action = Rc<RefCell<dyn FnMut(&ToolEvent<'_>)>>;

/// A tool added to a figure.
struct UserTool {
    spec: ToolSpec,
    toggled: bool,
    action: Action,
}

/// The tools of a figure, in the order they were added.
#[derive(Default)]
pub(crate) struct ToolRegistry {
    tools: Vec<UserTool>,
}

impl fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.tools.iter().map(|tool| &tool.spec.name))
            .finish()
    }
}

impl Figure {
    /// Adds a tool to the figure's windows: a toolbar button after the
    /// built-in ones, and its shortcuts; `action` runs on every use
    /// (matplotlib's `toolmanager.add_tool` and `toolbar.add_tool`).
    /// Windows show the tools their figure has when they open.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::gui::{Icon, ToolKind, ToolSpec};
    /// let fig = Figure::new();
    /// let spec = ToolSpec {
    ///     name: "grid".into(),
    ///     description: "Show the grid".into(),
    ///     icon: Icon::new(24.0),
    ///     keys: vec!["ctrl+g".into()],
    ///     kind: ToolKind::Toggle,
    /// };
    /// fig.add_tool(spec, |event| {
    ///     for ax in event.figure.axes() {
    ///         ax.grid(event.toggled);
    ///     }
    /// })?;
    /// fig.subplot();
    /// fig.trigger_tool("grid");
    /// assert_eq!(fig.tool_toggled("grid"), Some(true));
    /// # Ok::<(), plt::gui::ToolError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the name is empty or already taken.
    pub fn add_tool(
        &self,
        spec: ToolSpec,
        action: impl FnMut(&ToolEvent<'_>) + 'static,
    ) -> Result<&Self, ToolError> {
        if spec.name.is_empty() {
            return Err(ToolError::EmptyName);
        }
        let mut state = self.raw().figure_state().borrow_mut();
        if state
            .tools
            .tools
            .iter()
            .any(|tool| tool.spec.name == spec.name)
        {
            return Err(ToolError::Duplicate(spec.name));
        }
        state.tools.tools.push(UserTool {
            spec,
            toggled: false,
            action: Rc::new(RefCell::new(action)),
        });
        Ok(self)
    }

    /// Removes the tool `name`; returns whether there was one.
    pub fn remove_tool(&self, name: &str) -> bool {
        let mut state = self.raw().figure_state().borrow_mut();
        let before = state.tools.tools.len();
        state.tools.tools.retain(|tool| tool.spec.name != name);
        state.tools.tools.len() != before
    }

    /// The names of the figure's tools, in toolbar order.
    pub fn tool_names(&self) -> Vec<String> {
        let state = self.raw().figure_state().borrow();
        state
            .tools
            .tools
            .iter()
            .map(|tool| tool.spec.name.clone())
            .collect()
    }

    /// Whether the toggle `name` is on; `None` without such a tool.
    pub fn tool_toggled(&self, name: &str) -> Option<bool> {
        let state = self.raw().figure_state().borrow();
        state
            .tools
            .tools
            .iter()
            .find(|tool| tool.spec.name == name)
            .map(|tool| tool.toggled)
    }

    /// Uses the tool `name` as its button does, switching toggles; returns
    /// whether there was such a tool (matplotlib's
    /// `toolmanager.trigger_tool`).
    pub fn trigger_tool(&self, name: &str) -> bool {
        let found = {
            let mut state = self.raw().figure_state().borrow_mut();
            state
                .tools
                .tools
                .iter_mut()
                .find(|tool| tool.spec.name == name)
                .map(|tool| {
                    if tool.spec.kind == ToolKind::Toggle {
                        tool.toggled = !tool.toggled;
                    }
                    (Rc::clone(&tool.action), tool.toggled)
                })
        };
        // The callback runs without the figure borrowed, so that it may
        // change the figure.
        let Some((action, toggled)) = found else {
            return false;
        };
        let event = ToolEvent {
            figure: self,
            name,
            toggled,
        };
        (action.borrow_mut())(&event);
        true
    }

    /// The name of the tool bound to key `key`, if any.
    pub(crate) fn tool_for_key(&self, key: &str) -> Option<String> {
        let state = self.raw().figure_state().borrow();
        state
            .tools
            .tools
            .iter()
            .find(|tool| tool.spec.keys.iter().any(|k| k == key))
            .map(|tool| tool.spec.name.clone())
    }

    /// The tools as windows show them, in toolbar order.
    pub(crate) fn tool_specs(&self) -> Vec<ToolSpec> {
        let state = self.raw().figure_state().borrow();
        state
            .tools
            .tools
            .iter()
            .map(|tool| tool.spec.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn spec(name: &str, kind: ToolKind) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            description: format!("the {name} tool"),
            icon: Icon::new(24.0),
            keys: vec![format!("ctrl+{name}")],
            kind,
        }
    }

    #[test]
    fn triggers_run_and_toggles_switch() {
        let fig = Figure::new();
        let uses = Rc::new(Cell::new(0));
        let count = Rc::clone(&uses);
        fig.add_tool(spec("a", ToolKind::Trigger), move |_| {
            count.set(count.get() + 1);
        })
        .unwrap();
        let states = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&states);
        fig.add_tool(spec("b", ToolKind::Toggle), move |event| {
            log.borrow_mut().push(event.toggled);
            // Callbacks may use the figure.
            assert_eq!(event.figure.tool_names(), ["a", "b"]);
        })
        .unwrap();
        assert!(fig.trigger_tool("a") && fig.trigger_tool("a"));
        assert!(fig.trigger_tool("b") && fig.trigger_tool("b"));
        assert_eq!(uses.get(), 2);
        assert_eq!(*states.borrow(), [true, false]);
        assert_eq!(fig.tool_toggled("a"), Some(false));
        assert!(!fig.trigger_tool("missing"));
        assert_eq!(fig.tool_for_key("ctrl+b").as_deref(), Some("b"));
    }

    #[test]
    fn names_are_unique_and_tools_removable() {
        let fig = Figure::new();
        fig.add_tool(spec("a", ToolKind::Trigger), |_| {}).unwrap();
        assert_eq!(
            fig.add_tool(spec("a", ToolKind::Toggle), |_| {})
                .unwrap_err(),
            ToolError::Duplicate("a".into())
        );
        assert_eq!(
            fig.add_tool(spec("", ToolKind::Toggle), |_| {})
                .unwrap_err(),
            ToolError::EmptyName
        );
        assert!(fig.remove_tool("a") && !fig.remove_tool("a"));
        assert_eq!(fig.tool_names(), Vec::<String>::new());
    }
}
