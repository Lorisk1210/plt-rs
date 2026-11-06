//! The event protocol every selector shares (matplotlib's
//! `_SelectorWidget`): which events a selector takes, how they are
//! cleaned, and the modifier-key states.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::{Rc, Weak};

use pltrs_core::Point;

use super::{ModifierKeys, SelectorError, SelectorEvent, SelectorState};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{Event, EventKind, KeyEvent, Location, LockOwner, MouseEvent};
use crate::figure::Figure;
use crate::navigation::PointerButton;
use crate::widgets::{CallbackId, Connections, Observers, data_in, ordered};

/// An event's pointer in the selector's axes, before cleaning (matplotlib's
/// `_call_with_reparented_event`).
#[derive(Clone, Debug)]
pub(super) struct Pointer {
    pub(super) x: f64,
    pub(super) y: f64,
    /// The data coordinates in the selector's axes, when defined.
    pub(super) data: Option<Point>,
    pub(super) button: Option<PointerButton>,
    pub(super) key: Option<String>,
}

/// The state every selector has (matplotlib's `_SelectorWidget`
/// attributes).
#[derive(Debug)]
pub(super) struct Core {
    pub(super) ax: Axes,
    pub(super) active: bool,
    /// Whether the selection is shown (matplotlib's `_visible`).
    pub(super) visible: bool,
    /// The buttons that select; all when `None`.
    buttons: Option<Vec<PointerButton>>,
    keys: ModifierKeys,
    /// Set once a selection is complete.
    pub(super) selection_completed: bool,
    /// The cleaned press starting the current interaction.
    pub(super) eventpress: Option<SelectorEvent>,
    /// The cleaned release ending it.
    pub(super) eventrelease: Option<SelectorEvent>,
    prev_event: Option<SelectorEvent>,
    pub(super) state: HashSet<SelectorState>,
    /// Identifies the selector to the figure's widget lock.
    owner: LockOwner,
}

impl Core {
    /// The state of a new selector on `ax`.
    pub(super) fn new(ax: &Axes, buttons: Option<Vec<PointerButton>>, keys: ModifierKeys) -> Self {
        Self {
            ax: ax.clone(),
            active: true,
            visible: true,
            buttons,
            keys,
            selection_completed: false,
            eventpress: None,
            eventrelease: None,
            prev_event: None,
            state: HashSet::new(),
            owner: LockOwner::new(),
        }
    }

    /// `location` in the selector's axes.
    fn reparent(
        &self,
        location: &Location,
        button: Option<PointerButton>,
        key: Option<String>,
    ) -> Pointer {
        Pointer {
            x: location.x,
            y: location.y,
            data: data_in(&self.ax, location),
            button,
            key,
        }
    }

    /// Whether no other tool holds the figure's widget lock.
    pub(super) fn lock_available(&self) -> bool {
        self.ax.figure().widgets_available(self.owner)
    }

    /// Whether the pointer is over the axes background.
    pub(super) fn contains(&self, pointer: &Pointer) -> bool {
        self.ax.contains_point(Point::new(pointer.x, pointer.y))
    }

    /// Whether a mouse event is none of the selector's business
    /// (matplotlib's `ignore`).
    pub(super) fn ignore(&self, pointer: &Pointer) -> bool {
        if !self.active || !self.ax.visible() {
            return true;
        }
        if !self.lock_available() {
            return true;
        }
        if let Some(buttons) = &self.buttons
            && !pointer.button.is_some_and(|b| buttons.contains(&b))
        {
            return true;
        }
        match &self.eventpress {
            None => !self.contains(pointer),
            Some(press) if pointer.button == press.button => false,
            Some(_) => true,
        }
    }

    /// The event with data coordinates clipped to the view, or the
    /// previous event when it has none (matplotlib's `_clean_event`).
    pub(super) fn clean(&mut self, pointer: &Pointer) -> Option<SelectorEvent> {
        let mut event = match pointer.data {
            Some(data) => SelectorEvent {
                x: pointer.x,
                y: pointer.y,
                xdata: data.x,
                ydata: data.y,
                button: pointer.button,
                key: pointer.key.clone(),
            },
            None => self.prev_event.clone()?,
        };
        let (x0, x1) = ordered(self.ax.xlim());
        let (y0, y1) = ordered(self.ax.ylim());
        event.xdata = event.xdata.clamp(x0, x1);
        event.ydata = event.ydata.clamp(y0, y1);
        self.prev_event = Some(event.clone());
        Some(event)
    }

    /// The key of `state`, if supported.
    pub(super) fn key(&self, state: SelectorState) -> Option<&str> {
        self.keys.key(state)
    }

    /// The key clearing the selection, if any.
    pub(super) fn clear_key(&self) -> Option<&str> {
        self.keys.clear.as_deref()
    }

    fn supported(&self, state: SelectorState) -> Result<(), SelectorError> {
        self.keys
            .key(state)
            .map(|_| ())
            .ok_or(SelectorError::UnsupportedState(state))
    }

    /// Switches `state` on (matplotlib's `add_state`).
    pub(super) fn add_state(&mut self, state: SelectorState) -> Result<(), SelectorError> {
        self.supported(state)?;
        self.state.insert(state);
        Ok(())
    }

    /// Switches `state` off (matplotlib's `remove_state`).
    pub(super) fn remove_state(&mut self, state: SelectorState) -> Result<(), SelectorError> {
        self.supported(state)?;
        if self.state.remove(&state) {
            Ok(())
        } else {
            Err(SelectorError::StateNotSet(state))
        }
    }
}

/// What makes one selector differ from another: its artists and how it
/// reacts to validated events (matplotlib's `_press`, `_onmove`,
/// `_release`, ...).
pub(super) trait Tool: 'static {
    /// What a selection reports to the callbacks.
    type Selection: 'static;

    /// Shows or hides every artist of the selector.
    fn show(&self, visible: bool);

    /// A validated press.
    fn press(&mut self, core: &mut Core, event: &SelectorEvent);

    /// A validated motion with a button held; returns what the move
    /// callbacks receive.
    fn drag(&mut self, core: &mut Core, event: &SelectorEvent) -> Option<Self::Selection>;

    /// A validated release; returns what the select callbacks receive.
    fn release(&mut self, core: &mut Core, event: &SelectorEvent) -> Option<Self::Selection>;

    /// Any pointer motion (matplotlib's `onmove` validator): by default a
    /// drag once a press was taken.
    fn motion(&mut self, core: &mut Core, pointer: &Pointer) -> Option<Self::Selection> {
        if core.ignore(pointer) || core.eventpress.is_none() {
            return None;
        }
        let event = core.clean(pointer)?;
        self.drag(core, &event)
    }

    /// A key press after the states were updated.
    fn key_press(&mut self, _core: &mut Core, _pointer: &Pointer) {}

    /// A key release after the states were updated.
    fn key_release(&mut self, _core: &mut Core, _pointer: &Pointer) {}

    /// Removes the selection (matplotlib's `_clear_without_update`).
    fn clear(&mut self, core: &mut Core) {
        core.selection_completed = false;
        core.visible = false;
        self.show(false);
    }
}

/// A selector's state, shared with its event handlers.
#[derive(Debug)]
pub(super) struct Shared<T> {
    pub(super) core: Core,
    pub(super) tool: T,
}

impl<T: Tool> Shared<T> {
    /// Shows or hides the selection (matplotlib's `set_visible`).
    pub(super) fn set_visible(&mut self, visible: bool) {
        self.core.visible = visible;
        self.tool.show(visible);
    }

    /// Removes the selection (matplotlib's `clear`).
    pub(super) fn clear(&mut self) {
        self.tool.clear(&mut self.core);
    }

    fn pointer(&self, event: &MouseEvent) -> Pointer {
        self.core
            .reparent(&event.location, event.button, event.key.clone())
    }

    /// matplotlib's `press` validator.
    fn press(&mut self, event: &MouseEvent) {
        let pointer = self.pointer(event);
        if self.core.ignore(&pointer) {
            return;
        }
        let Some(event) = self.core.clean(&pointer) else {
            return;
        };
        self.core.eventpress = Some(event.clone());
        let key = event
            .key
            .as_deref()
            .unwrap_or("")
            .replace("ctrl", "control");
        // The move state is locked in on a press.
        if self.core.key(SelectorState::Move) == Some(key.as_str()) {
            self.core.state.insert(SelectorState::Move);
        }
        self.tool.press(&mut self.core, &event);
    }

    /// matplotlib's `release` validator.
    fn release(&mut self, event: &MouseEvent) -> Option<T::Selection> {
        let pointer = self.pointer(event);
        if self.core.ignore(&pointer) || self.core.eventpress.is_none() {
            return None;
        }
        let event = self.core.clean(&pointer)?;
        self.core.eventrelease = Some(event.clone());
        let selection = self.tool.release(&mut self.core, &event);
        self.core.eventpress = None;
        self.core.eventrelease = None;
        self.core.state.remove(&SelectorState::Move);
        selection
    }

    fn motion(&mut self, event: &MouseEvent) -> Option<T::Selection> {
        let pointer = self.pointer(event);
        self.tool.motion(&mut self.core, &pointer)
    }

    /// matplotlib's `on_key_press`: the clear key clears, the state keys
    /// switch their states on (the rotate key toggles).
    fn key_press(&mut self, event: &KeyEvent) {
        if !self.core.active {
            return;
        }
        let key = event.key.replace("ctrl", "control");
        if self.core.keys.clear.as_deref() == Some(key.as_str()) {
            self.clear();
            return;
        }
        let parts: Vec<&str> = key.split('+').collect();
        let held: Vec<SelectorState> = self
            .core
            .keys
            .states
            .iter()
            .filter(|(_, modifier)| parts.contains(&modifier.as_str()))
            .map(|(state, _)| *state)
            .collect();
        for state in held {
            if state == SelectorState::Rotate && self.core.state.contains(&state) {
                self.core.state.remove(&state);
            } else {
                self.core.state.insert(state);
            }
        }
        let pointer = self
            .core
            .reparent(&event.location, None, Some(event.key.clone()));
        self.tool.key_press(&mut self.core, &pointer);
    }

    /// matplotlib's `on_key_release`: the state keys switch their states
    /// off, except rotation.
    fn key_release(&mut self, event: &KeyEvent) {
        if !self.core.active {
            return;
        }
        let parts: Vec<&str> = event.key.split('+').collect();
        let released: Vec<SelectorState> = self
            .core
            .keys
            .states
            .iter()
            .filter(|(state, modifier)| {
                *state != SelectorState::Rotate && parts.contains(&modifier.as_str())
            })
            .map(|(state, _)| *state)
            .collect();
        for state in released {
            self.core.state.remove(&state);
        }
        let pointer = self
            .core
            .reparent(&event.location, None, Some(event.key.clone()));
        self.tool.key_release(&mut self.core, &pointer);
    }
}

/// A selector connected to its figure: the shared state, the callbacks
/// and the event connections.
pub(super) struct Driver<T: Tool> {
    pub(super) shared: Rc<RefCell<Shared<T>>>,
    /// Called with each completed selection (matplotlib's `onselect`).
    pub(super) selected: Observers<T::Selection>,
    /// Called while a selection changes (matplotlib's `onmove_callback`).
    pub(super) moved: Observers<T::Selection>,
    /// The connections to the events of the selector's figure.
    connections: RefCell<Connections>,
}

impl<T: Tool + std::fmt::Debug> std::fmt::Debug for Driver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Driver")
            .field("shared", &self.shared)
            .finish_non_exhaustive()
    }
}

impl<T: Tool> Driver<T> {
    /// Disconnects callback `id` from either signal; returns whether it
    /// was connected.
    pub(super) fn disconnect(&self, id: CallbackId) -> bool {
        self.selected.disconnect(id) || self.moved.disconnect(id)
    }

    /// Connects a selector made of `core` and `tool` to the figure's
    /// mouse and key events.
    pub(super) fn new(core: Core, tool: T) -> Self {
        let figure = core.ax.figure();
        let shared = Rc::new(RefCell::new(Shared { core, tool }));
        let selected = Observers::default();
        let moved = Observers::default();
        let connections = RefCell::new(connect_events(figure, &shared, &selected, &moved));
        Self {
            shared,
            selected,
            moved,
            connections,
        }
    }

    /// Makes the selector work in `ax` from now on, taking the events of
    /// its figure (the event routing of matplotlib's `new_axes`, which
    /// connects to the canvas of the new axes when it is another).
    pub(super) fn move_to(&self, ax: &Axes) {
        self.shared.borrow_mut().core.ax = ax.clone();
        let figure = ax.figure();
        if *self.connections.borrow().figure() != figure {
            let connections = connect_events(figure, &self.shared, &self.selected, &self.moved);
            // The old connections end as they are dropped.
            drop(self.connections.replace(connections));
        }
    }
}

/// Connects the selector behind `shared` to the mouse and key events of
/// `figure`, its selections reported to `selected` and `moved`.
fn connect_events<T: Tool>(
    figure: Figure,
    shared: &Rc<RefCell<Shared<T>>>,
    selected: &Observers<T::Selection>,
    moved: &Observers<T::Selection>,
) -> Connections {
    let mut connections = Connections::new(figure);
    let weak = || Rc::downgrade(shared);
    connect(
        &mut connections,
        EventKind::ButtonPress,
        weak(),
        |shared, event| {
            if let Event::ButtonPress(mouse) = event {
                shared.borrow_mut().press(mouse);
            }
        },
    );
    let observers = selected.clone();
    connect(
        &mut connections,
        EventKind::ButtonRelease,
        weak(),
        move |shared, event| {
            if let Event::ButtonRelease(mouse) = event {
                let selection = shared.borrow_mut().release(mouse);
                if let Some(selection) = selection {
                    observers.process(&selection);
                }
            }
        },
    );
    let observers = moved.clone();
    connect(
        &mut connections,
        EventKind::Motion,
        weak(),
        move |shared, event| {
            if let Event::Motion(mouse) = event {
                let selection = shared.borrow_mut().motion(mouse);
                if let Some(selection) = selection {
                    observers.process(&selection);
                }
            }
        },
    );
    connect(
        &mut connections,
        EventKind::KeyPress,
        weak(),
        |shared, event| {
            if let Event::KeyPress(key) = event {
                shared.borrow_mut().key_press(key);
            }
        },
    );
    connect(
        &mut connections,
        EventKind::KeyRelease,
        weak(),
        |shared, event| {
            if let Event::KeyRelease(key) = event {
                shared.borrow_mut().key_release(key);
            }
        },
    );
    connections
}

/// Connects `handler` for events of `kind`, for as long as the selector
/// behind `shared` lives.
fn connect<T: Tool>(
    connections: &mut Connections,
    kind: EventKind,
    shared: Weak<RefCell<Shared<T>>>,
    handler: impl Fn(&RefCell<Shared<T>>, &Event) + 'static,
) {
    connections.connect(kind, move |event| {
        if let Some(shared) = shared.upgrade() {
            handler(&shared, event);
        }
    });
}
