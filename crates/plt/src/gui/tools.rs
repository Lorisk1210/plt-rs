//! Tools a program adds to its figure windows (matplotlib's
//! `ToolManager.add_tool` and `ToolContainerBase.add_tool`): toolbar
//! buttons with keyboard shortcuts that run a callback, once per use or as
//! an on/off toggle. Toggles may form radio groups, the built-in pan and
//! zoom tools' among them, show a cursor while on and start on.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use plt_core::Point;
use plt_ui::Icon;

use crate::artist::ArtistHandle;
use crate::events::CursorShape;
use crate::figure::Figure;
use crate::navigation::{NavMode, Navigation, axes_at};

/// The radio group of the windows' pan and zoom tools (matplotlib's
/// `"default"`): a toggle of this group switches them off when it goes
/// on, and goes off when one of them is switched on.
pub const NAVIGATION_GROUP: &str = "default";

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
    /// The radio group of a toggle (matplotlib's `radio_group`): switching
    /// it on switches off the tool of the group that is on, running its
    /// callback first. In [`NAVIGATION_GROUP`] it takes turns with the
    /// windows' pan and zoom tools. `None` for a toggle on its own.
    pub radio_group: Option<String>,
    /// The cursor shown over navigable axes while the toggle is on, and
    /// neither pan nor zoom is (matplotlib's `cursor`); the one of the
    /// toggle switched on last wins.
    pub cursor: Option<CursorShape>,
    /// Whether the toggle starts on (matplotlib's `default_toggled`): it is
    /// switched on as it is added, its callback running then.
    pub default_toggled: bool,
}

impl Default for ToolSpec {
    /// A trigger without name, description, keys or drawing on its icon
    /// (a 24-unit grid, like the built-in icons'), in no radio group,
    /// without cursor, starting off.
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            icon: Icon::new(24.0),
            keys: Vec::new(),
            kind: ToolKind::Trigger,
            radio_group: None,
            cursor: None,
            default_toggled: false,
        }
    }
}

/// After the pan or zoom tool of `navigation` was switched on: switches
/// off the toggle of their radio group that was on (matplotlib's
/// `ToolManager` radio groups).
pub(crate) fn navigation_switched(figure: &Figure, navigation: &Navigation) {
    if navigation.mode() != NavMode::None {
        figure.untoggle_group(NAVIGATION_GROUP);
    }
}

/// Switches the pan or zoom tool of `navigation` off when a toggle of
/// their radio group is on, as that toggle went on since; returns whether
/// it did.
pub(crate) fn yield_navigation(figure: &Figure, navigation: &mut Navigation) -> bool {
    let mode = navigation.mode();
    mode != NavMode::None
        && figure.toggled_in_group(NAVIGATION_GROUP).is_some()
        && navigation.toggle(figure, mode).is_ok()
}

/// The cursor a window shows outside pan and zoom, with the pointer at
/// display point `pointer`: the one a widget asked for, else the cursor of
/// the toggle switched on last while the pointer is over navigable axes
/// (matplotlib's `ToolSetCursor`), else the arrow.
pub(crate) fn idle_cursor(figure: &Figure, pointer: Option<Point>) -> CursorShape {
    let asked = figure.cursor();
    if asked != CursorShape::Pointer {
        return asked;
    }
    let over_axes = pointer
        .and_then(|at| axes_at(figure, at))
        .is_some_and(|ax| ax.navigate());
    figure.tool_cursor().filter(|_| over_axes).unwrap_or(asked)
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

impl UserTool {
    /// Whether the tool is a toggle of radio group `group`.
    fn in_group(&self, group: &str) -> bool {
        self.spec.kind == ToolKind::Toggle && self.spec.radio_group.as_deref() == Some(group)
    }
}

/// The tools of a figure, in the order they were added.
#[derive(Default)]
pub(crate) struct ToolRegistry {
    tools: Vec<UserTool>,
    /// The toggle with a cursor switched on last, while it is on.
    cursor_tool: Option<String>,
}

impl ToolRegistry {
    fn find(&self, name: &str) -> Option<&UserTool> {
        self.tools.iter().find(|tool| tool.spec.name == name)
    }

    /// The toggle of radio group `group` that is on, if any.
    fn toggled_in_group(&self, group: &str) -> Option<&UserTool> {
        self.tools
            .iter()
            .find(|tool| tool.toggled && tool.in_group(group))
    }
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
    /// (matplotlib's `toolmanager.add_tool` and `toolbar.add_tool`), and
    /// once as it is added for a toggle starting on
    /// ([`ToolSpec::default_toggled`]). Windows show the tools their figure
    /// has when they open.
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
    ///     ..ToolSpec::default()
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
        let starts_on = spec.kind == ToolKind::Toggle && spec.default_toggled;
        let name = spec.name.clone();
        {
            let mut state = self.raw().figure_state().borrow_mut();
            if state.tools.find(&spec.name).is_some() {
                return Err(ToolError::Duplicate(spec.name));
            }
            state.tools.tools.push(UserTool {
                spec,
                toggled: false,
                action: Rc::new(RefCell::new(action)),
            });
        }
        // As matplotlib does when a toggle starting on joins a figure.
        if starts_on {
            self.trigger_tool(&name);
        }
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
    /// `toolmanager.trigger_tool`). Switching on a toggle of a radio group
    /// first switches off the tool of the group that was on. A callback
    /// that is running, the tool used from its own callback, is not run
    /// again.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::gui::{ToolKind, ToolSpec};
    /// let fig = Figure::new();
    /// for name in ["lasso", "brush"] {
    ///     let spec = ToolSpec {
    ///         name: name.into(),
    ///         kind: ToolKind::Toggle,
    ///         radio_group: Some("select".into()),
    ///         default_toggled: name == "lasso",
    ///         ..ToolSpec::default()
    ///     };
    ///     fig.add_tool(spec, |_| {})?;
    /// }
    /// assert_eq!(fig.tool_toggled("lasso"), Some(true), "starts on");
    /// fig.trigger_tool("brush");
    /// assert_eq!(fig.tool_toggled("lasso"), Some(false), "one at a time");
    /// assert_eq!(fig.tool_toggled("brush"), Some(true));
    /// # Ok::<(), plt::gui::ToolError>(())
    /// ```
    pub fn trigger_tool(&self, name: &str) -> bool {
        let displaced = {
            let state = self.raw().figure_state().borrow();
            let Some(tool) = state.tools.find(name) else {
                return false;
            };
            // Going on, a toggle of a radio group switches the other off.
            tool.spec
                .radio_group
                .as_deref()
                .filter(|&group| !tool.toggled && tool.in_group(group))
                .and_then(|group| state.tools.toggled_in_group(group))
                .map(|other| other.spec.name.clone())
        };
        if let Some(other) = displaced {
            self.trigger_tool(&other);
        }
        let found = {
            let mut state = self.raw().figure_state().borrow_mut();
            let registry = &mut state.tools;
            registry
                .tools
                .iter_mut()
                .find(|tool| tool.spec.name == name)
                .map(|tool| {
                    if tool.spec.kind == ToolKind::Toggle {
                        tool.toggled = !tool.toggled;
                        if tool.spec.cursor.is_some() {
                            if tool.toggled {
                                registry.cursor_tool = Some(name.to_owned());
                            } else if registry.cursor_tool.as_deref() == Some(name) {
                                registry.cursor_tool = None;
                            }
                        }
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
        if let Ok(mut action) = action.try_borrow_mut() {
            action(&event);
        }
        true
    }

    /// The name of the toggle of radio group `group` that is on, if any.
    pub(crate) fn toggled_in_group(&self, group: &str) -> Option<String> {
        let state = self.raw().figure_state().borrow();
        state
            .tools
            .toggled_in_group(group)
            .map(|tool| tool.spec.name.clone())
    }

    /// Switches off the toggle of radio group `group` that is on, if any,
    /// running its callback (a window's pan or zoom tool going on).
    pub(crate) fn untoggle_group(&self, group: &str) {
        if let Some(name) = self.toggled_in_group(group) {
            self.trigger_tool(&name);
        }
    }

    /// The cursor of the toggle with a cursor switched on last, while it
    /// is on (matplotlib's `ToolSetCursor`).
    pub(crate) fn tool_cursor(&self) -> Option<CursorShape> {
        let state = self.raw().figure_state().borrow();
        let tools = &state.tools;
        let name = tools.cursor_tool.as_deref()?;
        tools
            .find(name)
            .filter(|tool| tool.toggled)
            .and_then(|tool| tool.spec.cursor)
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
mod tests;
