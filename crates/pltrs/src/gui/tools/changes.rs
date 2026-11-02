//! What happens to a figure's tools, told to the callbacks a program
//! connects (matplotlib's `toolmanager_connect` and the events of its tool
//! manager: `tool_added_event`, `tool_removed_event`, `tool_trigger_<name>`
//! and `tool_message_event`).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::artist::ArtistHandle;
use crate::figure::Figure;

/// A change to the tools of a figure, as the callbacks of
/// [`Figure::on_tool_change`] receive it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolChange<'a> {
    /// The tool was added (matplotlib's `tool_added_event`), after a
    /// toggle starting on was switched on.
    Added(&'a str),
    /// The tool is being removed (`tool_removed_event`): it is still
    /// there, switched off.
    Removed(&'a str),
    /// The tool was used, after its callback ran (`tool_trigger_<name>`).
    Triggered {
        /// The tool's name.
        name: &'a str,
        /// For toggles, whether the tool is now on.
        toggled: bool,
    },
    /// A tool's message for the status line (`tool_message_event`, sent by
    /// [`Figure::tool_message`]).
    Message(&'a str),
}

/// Identifies a callback of [`Figure::on_tool_change`] (matplotlib's
/// `cid` of `toolmanager_connect`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToolListenerId(u64);

/// A callback told of tool changes.
type Listener = Rc<RefCell<dyn FnMut(&Figure, &ToolChange<'_>)>>;

/// The callbacks of a figure's tools, and the message its windows show
/// next.
#[derive(Default)]
pub(crate) struct Listeners {
    callbacks: Vec<(ToolListenerId, Listener)>,
    next: u64,
    /// The last message sent and not shown yet.
    message: Option<String>,
}

impl fmt::Debug for Listeners {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Listeners")
            .field("callbacks", &self.callbacks.len())
            .field("next", &self.next)
            .field("message", &self.message)
            .finish()
    }
}

impl Figure {
    /// Calls `callback` with every change to the figure's tools: tools
    /// added, removed and used, and the messages they send (matplotlib's
    /// `toolmanager_connect`, for all its events at once). The callback
    /// may use the figure; one that is running is not called again from
    /// within itself.
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    ///
    /// use pltrs::figure::Figure;
    /// use pltrs::gui::{ToolChange, ToolKind, ToolSpec};
    ///
    /// let fig = Figure::new();
    /// let log = Rc::new(RefCell::new(Vec::new()));
    /// let seen = Rc::clone(&log);
    /// let id = fig.on_tool_change(move |_, change| seen.borrow_mut().push(format!("{change:?}")));
    /// let spec = ToolSpec { name: "grid".into(), kind: ToolKind::Toggle, ..ToolSpec::default() };
    /// fig.add_tool(spec, |event| event.figure.tool_message("grid switched"))?;
    /// fig.trigger_tool("grid");
    /// fig.remove_tool("grid");
    /// assert_eq!(
    ///     *log.borrow(),
    ///     [
    ///         r#"Added("grid")"#,
    ///         r#"Message("grid switched")"#,
    ///         r#"Triggered { name: "grid", toggled: true }"#,
    ///         r#"Message("grid switched")"#,
    ///         r#"Triggered { name: "grid", toggled: false }"#,
    ///         r#"Removed("grid")"#,
    ///     ]
    /// );
    /// assert!(fig.disconnect_tool_change(id));
    /// # Ok::<(), pltrs::gui::ToolError>(())
    /// ```
    pub fn on_tool_change(
        &self,
        callback: impl FnMut(&Figure, &ToolChange<'_>) + 'static,
    ) -> ToolListenerId {
        let mut state = self.raw().figure_state().borrow_mut();
        let listeners = &mut state.tools.listeners;
        let id = ToolListenerId(listeners.next);
        listeners.next += 1;
        listeners
            .callbacks
            .push((id, Rc::new(RefCell::new(callback))));
        id
    }

    /// Disconnects the callback `id` of [`Figure::on_tool_change`]
    /// (matplotlib's `toolmanager_disconnect`); returns whether it was
    /// connected.
    pub fn disconnect_tool_change(&self, id: ToolListenerId) -> bool {
        let mut state = self.raw().figure_state().borrow_mut();
        let callbacks = &mut state.tools.listeners.callbacks;
        let before = callbacks.len();
        callbacks.retain(|(connected, _)| *connected != id);
        callbacks.len() != before
    }

    /// Sends `message` to the callbacks of [`Figure::on_tool_change`]
    /// (matplotlib's `toolmanager.message_event`); the figure's windows
    /// show it in their status line.
    pub fn tool_message(&self, message: &str) {
        self.raw()
            .figure_state()
            .borrow_mut()
            .tools
            .listeners
            .message = Some(message.to_owned());
        self.notify_tools(&ToolChange::Message(message));
    }

    /// The message sent last and not shown yet, taken for a window's
    /// status line.
    pub(crate) fn take_tool_message(&self) -> Option<String> {
        self.raw()
            .figure_state()
            .borrow_mut()
            .tools
            .listeners
            .message
            .take()
    }

    /// Tells the connected callbacks of `change`, without the figure
    /// borrowed, so that they may change it.
    pub(super) fn notify_tools(&self, change: &ToolChange<'_>) {
        let callbacks: Vec<Listener> = {
            let state = self.raw().figure_state().borrow();
            state
                .tools
                .listeners
                .callbacks
                .iter()
                .map(|(_, callback)| Rc::clone(callback))
                .collect()
        };
        for callback in callbacks {
            if let Ok(mut callback) = callback.try_borrow_mut() {
                callback(self, change);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::{ToolKind, ToolSpec};

    fn toggle(name: &str, group: &str) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            kind: ToolKind::Toggle,
            radio_group: Some(group.into()),
            ..ToolSpec::default()
        }
    }

    /// The changes `fig` reports, as text.
    fn recorded(fig: &Figure) -> Rc<RefCell<Vec<String>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&log);
        fig.on_tool_change(move |_, change| seen.borrow_mut().push(format!("{change:?}")));
        log
    }

    #[test]
    fn radio_groups_report_both_toggles() {
        let fig = Figure::new();
        let log = recorded(&fig);
        fig.add_tool(toggle("a", "g"), |_| {}).unwrap();
        fig.add_tool(toggle("b", "g"), |_| {}).unwrap();
        fig.trigger_tool("a");
        fig.trigger_tool("b");
        assert_eq!(
            log.borrow()[2..],
            [
                r#"Triggered { name: "a", toggled: true }"#,
                r#"Triggered { name: "a", toggled: false }"#,
                r#"Triggered { name: "b", toggled: true }"#,
            ]
        );
        assert_eq!(fig.active_toggle("g").as_deref(), Some("b"));
    }

    #[test]
    fn a_toggle_starting_on_is_added_on() {
        let fig = Figure::new();
        let log = recorded(&fig);
        let spec = ToolSpec {
            default_toggled: true,
            ..toggle("a", "g")
        };
        fig.add_tool(spec, |_| {}).unwrap();
        assert_eq!(
            *log.borrow(),
            [r#"Triggered { name: "a", toggled: true }"#, r#"Added("a")"#]
        );
    }

    #[test]
    fn removing_a_tool_switches_it_off_and_tells_while_it_is_there() {
        let fig = Figure::new();
        let present = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&present);
        fig.on_tool_change(move |fig, change| {
            if let ToolChange::Removed(name) = change {
                seen.borrow_mut().push(fig.tool(name).is_some());
            }
        });
        fig.add_tool(toggle("a", "g"), |_| {}).unwrap();
        fig.trigger_tool("a");
        assert!(fig.remove_tool("a"));
        assert_eq!(*present.borrow(), [true]);
        assert_eq!(fig.active_toggle("g"), None);
        assert!(fig.tools().is_empty());
        assert!(!fig.remove_tool("a"));
    }

    #[test]
    fn listeners_may_use_the_figure_and_are_not_reentered() {
        let fig = Figure::new();
        let calls = Rc::new(RefCell::new(0));
        let count = Rc::clone(&calls);
        fig.on_tool_change(move |fig, change| {
            *count.borrow_mut() += 1;
            if let ToolChange::Added(name) = change {
                // Sending a message from the callback does not call it again.
                fig.tool_message(&format!("added {name}"));
            }
        });
        let spec = ToolSpec {
            name: "t".into(),
            ..ToolSpec::default()
        };
        fig.add_tool(spec, |_| {}).unwrap();
        assert_eq!(*calls.borrow(), 1);
        assert_eq!(fig.take_tool_message().as_deref(), Some("added t"));
        assert_eq!(fig.take_tool_message(), None, "taken once");
    }

    #[test]
    fn disconnected_listeners_are_not_called() {
        let fig = Figure::new();
        let log = Rc::new(RefCell::new(0));
        let count = Rc::clone(&log);
        let id = fig.on_tool_change(move |_, _| *count.borrow_mut() += 1);
        assert!(fig.disconnect_tool_change(id));
        assert!(!fig.disconnect_tool_change(id));
        fig.tool_message("unheard");
        assert_eq!(*log.borrow(), 0);
    }
}
