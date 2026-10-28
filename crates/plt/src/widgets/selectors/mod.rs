//! Selector widgets (matplotlib's `SpanSelector`, `RectangleSelector`,
//! `EllipseSelector`, `LassoSelector`, `PolygonSelector` and `Lasso`):
//! tools that select a range, a region or a shape of an axes with the
//! mouse and report it to callbacks.
//!
//! The selectors share matplotlib's `_SelectorWidget` protocol: a press
//! inside the axes starts a selection, motion updates it and the release
//! completes it. Held modifier keys switch on states such as moving or
//! squaring the shape ([`ModifierKeys`]), and the clear key (`escape`)
//! removes the selection. Interactive selectors keep their shape after
//! the release and draw handles to resize or move it. A selector ignores
//! the pointer while another tool holds the figure's widget lock (the pan
//! and zoom tools do while active).

mod driver;
mod handles;
mod lasso;
mod polygon;
mod rectangle;
mod region;
mod shape;
mod span;

use std::fmt;
use std::str::FromStr;

pub use lasso::{Lasso, LassoSelector};
pub use polygon::{PolygonSelector, PolygonSelectorOptions};
pub use rectangle::{EllipseSelector, RectangleSelector, RectangleSelectorOptions};
pub use region::SelectorUnits;
pub use span::{SpanSelector, SpanSelectorOptions};

use crate::navigation::PointerButton;

/// A behaviour a held modifier key switches on (matplotlib's selector
/// states, see [`ModifierKeys`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectorState {
    /// Move the existing shape (`"move"`).
    Move,
    /// Keep the shape square (`"square"`).
    Square,
    /// Resize the shape about its centre (`"center"`).
    Center,
    /// Rotate the shape (`"rotate"`); each press of its key toggles it.
    Rotate,
    /// Move one vertex of an unfinished polygon (`"move_vertex"`).
    MoveVertex,
    /// Move every vertex of a polygon (`"move_all"`).
    MoveAll,
}

impl SelectorState {
    /// Every state, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::Move,
        Self::Square,
        Self::Center,
        Self::Rotate,
        Self::MoveVertex,
        Self::MoveAll,
    ];

    /// matplotlib's name of the state (`"move"`, `"square"`, ...).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Square => "square",
            Self::Center => "center",
            Self::Rotate => "rotate",
            Self::MoveVertex => "move_vertex",
            Self::MoveAll => "move_all",
        }
    }
}

/// A name that is not one of matplotlib's selector states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownState(pub String);

impl fmt::Display for UnknownState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a selector state", self.0)
    }
}

impl std::error::Error for UnknownState {}

impl FromStr for SelectorState {
    type Err = UnknownState;

    /// Parses matplotlib's state names, as `add_state` takes them.
    ///
    /// ```
    /// use plt::widgets::SelectorState;
    /// assert_eq!("square".parse(), Ok(SelectorState::Square));
    /// assert!("clear".parse::<SelectorState>().is_err());
    /// ```
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|state| state.name() == name)
            .ok_or_else(|| UnknownState(name.to_owned()))
    }
}

/// The keys a selector reacts to (matplotlib's `state_modifier_keys`):
/// the key clearing the selection and the key switching on each state it
/// supports. Keys use matplotlib's names (`"shift"`, `"control"`, `" "`).
#[derive(Clone, Debug, PartialEq)]
pub struct ModifierKeys {
    /// The key clearing the selection, if any.
    pub clear: Option<String>,
    /// The supported states and their keys.
    pub states: Vec<(SelectorState, String)>,
}

impl Default for ModifierKeys {
    /// matplotlib's defaults: space moves, shift squares, control resizes
    /// about the centre, `r` rotates and escape clears.
    fn default() -> Self {
        Self {
            clear: Some("escape".to_owned()),
            states: vec![
                (SelectorState::Move, " ".to_owned()),
                (SelectorState::Square, "shift".to_owned()),
                (SelectorState::Center, "control".to_owned()),
                (SelectorState::Rotate, "r".to_owned()),
            ],
        }
    }
}

impl ModifierKeys {
    /// The defaults of span selectors: space moves and escape clears.
    pub fn span() -> Self {
        Self {
            clear: Some("escape".to_owned()),
            states: vec![(SelectorState::Move, " ".to_owned())],
        }
    }

    /// The defaults of polygon selectors: control moves one vertex, shift
    /// moves all of them and escape starts a new polygon.
    pub fn polygon() -> Self {
        Self {
            clear: Some("escape".to_owned()),
            states: vec![
                (SelectorState::MoveVertex, "control".to_owned()),
                (SelectorState::MoveAll, "shift".to_owned()),
            ],
        }
    }

    /// `self` with `key` switching `state` on.
    #[must_use]
    pub fn with(mut self, state: SelectorState, key: impl Into<String>) -> Self {
        self.states.retain(|(s, _)| *s != state);
        self.states.push((state, key.into()));
        self
    }

    /// `self` without `state`.
    #[must_use]
    pub fn without(mut self, state: SelectorState) -> Self {
        self.states.retain(|(s, _)| *s != state);
        self
    }

    /// The key of `state`, if supported.
    pub fn key(&self, state: SelectorState) -> Option<&str> {
        self.states
            .iter()
            .find(|(s, _)| *s == state)
            .map(|(_, key)| key.as_str())
    }
}

/// A mouse event as a selector sees it (matplotlib's events after
/// `_clean_event`): the pointer in display pixels and in the selector's
/// data coordinates, clipped to its view.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectorEvent {
    /// Pixels from the left of the figure.
    pub x: f64,
    /// Pixels from the bottom of the figure.
    pub y: f64,
    /// Data x coordinate in the selector's axes.
    pub xdata: f64,
    /// Data y coordinate in the selector's axes.
    pub ydata: f64,
    /// The button pressed, released or held.
    pub button: Option<PointerButton>,
    /// The key held, in matplotlib's naming.
    pub key: Option<String>,
}

/// Why a selector refused a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorError {
    /// The selector has no key for this state.
    UnsupportedState(SelectorState),
    /// The state was not on.
    StateNotSet(SelectorState),
    /// The selector draws no handles.
    NoHandles,
    /// A polygon needs at least one vertex.
    NoVertices,
}

impl fmt::Display for SelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedState(state) => {
                write!(f, "the selector does not support the {state:?} state")
            }
            Self::StateNotSet(state) => write!(f, "the {state:?} state is not on"),
            Self::NoHandles => f.write_str("this selector doesn't have handles"),
            Self::NoVertices => f.write_str("a polygon needs at least one vertex"),
        }
    }
}

impl std::error::Error for SelectorError {}

/// The methods every selector has, delegating to its driver.
macro_rules! selector_methods {
    ($selector:ty) => {
        impl $selector {
            /// Whether the selector reacts to events (matplotlib's
            /// `active`).
            pub fn active(&self) -> bool {
                self.driver.shared.borrow().core.active
            }

            /// Turns the selector on or off (matplotlib's `set_active`).
            pub fn set_active(&self, active: bool) -> &Self {
                self.driver.shared.borrow_mut().core.active = active;
                self
            }

            /// Whether the selection is shown (matplotlib's
            /// `get_visible`).
            pub fn visible(&self) -> bool {
                self.driver.shared.borrow().core.visible
            }

            /// Shows or hides the selection and its handles (matplotlib's
            /// `set_visible`).
            pub fn set_visible(&self, visible: bool) -> &Self {
                self.driver.shared.borrow_mut().set_visible(visible);
                self
            }

            /// Whether a selection was completed and is kept (matplotlib's
            /// `_selection_completed`).
            pub fn selection_completed(&self) -> bool {
                self.driver.shared.borrow().core.selection_completed
            }

            /// The states switched on, by keys held or `add_state`, in the
            /// order of
            /// [`SelectorState::ALL`](crate::widgets::SelectorState::ALL)
            /// (matplotlib's `_state`).
            pub fn states(&self) -> Vec<$crate::widgets::SelectorState> {
                let shared = self.driver.shared.borrow();
                $crate::widgets::SelectorState::ALL
                    .into_iter()
                    .filter(|state| shared.core.state.contains(state))
                    .collect()
            }

            /// Removes the selection, ready for a new one (matplotlib's
            /// `clear`).
            pub fn clear(&self) {
                self.driver.shared.borrow_mut().clear();
            }

            /// Switches `state` on as if its key were held (matplotlib's
            /// `add_state`).
            ///
            /// # Errors
            ///
            /// When the selector does not support `state`.
            pub fn add_state(
                &self,
                state: $crate::widgets::SelectorState,
            ) -> Result<(), $crate::widgets::SelectorError> {
                self.driver.shared.borrow_mut().core.add_state(state)
            }

            /// Switches `state` off (matplotlib's `remove_state`).
            ///
            /// # Errors
            ///
            /// When the selector does not support `state` or it is off.
            pub fn remove_state(
                &self,
                state: $crate::widgets::SelectorState,
            ) -> Result<(), $crate::widgets::SelectorError> {
                self.driver.shared.borrow_mut().core.remove_state(state)
            }

            /// The axes the selector works in.
            pub fn ax(&self) -> $crate::axes::Axes {
                self.driver.shared.borrow().core.ax.clone()
            }
        }
    };
}

use selector_methods;
