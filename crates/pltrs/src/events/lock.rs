//! The figure's widget lock (matplotlib's `canvas.widgetlock`, a
//! `LockDraw`) and the cursor its tools ask for (`canvas.set_cursor`).
//!
//! A tool that wants the pointer for itself — a selector while dragging,
//! the pan or zoom tool while active — takes the lock; the other widgets
//! leave the pointer alone until it is released.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::figure::Figure;

/// A holder of a figure's widget lock: a widget or a navigation tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LockOwner(u64);

impl LockOwner {
    /// A new owner, distinct from every other.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for LockOwner {
    fn default() -> Self {
        Self::new()
    }
}

/// Why the widget lock could not be taken or released.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockError {
    /// Another owner holds the lock.
    AlreadyLocked,
    /// The lock is not held by the owner releasing it.
    NotOwner,
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlreadyLocked => "already locked",
            Self::NotOwner => "you do not own this lock",
        })
    }
}

impl std::error::Error for LockError {}

/// The pointer shape an interactive tool asks the window for (matplotlib's
/// `backend_tools.Cursors`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CursorShape {
    /// The usual arrow.
    #[default]
    Pointer,
    /// A pointing hand.
    Hand,
    /// A crosshair for selecting a region.
    SelectRegion,
    /// Arrows in four directions.
    Move,
    /// Busy.
    Wait,
    /// Arrows left and right.
    ResizeHorizontal,
    /// Arrows up and down.
    ResizeVertical,
}

impl CursorShape {
    /// Every shape, in the order of matplotlib's `Cursors`.
    pub const ALL: [Self; 7] = [
        Self::Pointer,
        Self::Hand,
        Self::SelectRegion,
        Self::Move,
        Self::Wait,
        Self::ResizeHorizontal,
        Self::ResizeVertical,
    ];

    /// The name of the shape among matplotlib's `Cursors`
    /// (`Cursors.SELECT_REGION.name` is `"SELECT_REGION"`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Pointer => "POINTER",
            Self::Hand => "HAND",
            Self::SelectRegion => "SELECT_REGION",
            Self::Move => "MOVE",
            Self::Wait => "WAIT",
            Self::ResizeHorizontal => "RESIZE_HORIZONTAL",
            Self::ResizeVertical => "RESIZE_VERTICAL",
        }
    }
}

impl Figure {
    /// Reserves the widget lock for `owner` (matplotlib's
    /// `canvas.widgetlock(owner)`); taking it again is allowed.
    ///
    /// # Errors
    ///
    /// When another owner holds the lock.
    pub fn lock_widgets(&self, owner: LockOwner) -> Result<(), LockError> {
        let mut state = self.state_mut();
        match state.widget_lock {
            Some(holder) if holder != owner => Err(LockError::AlreadyLocked),
            _ => {
                state.widget_lock = Some(owner);
                Ok(())
            }
        }
    }

    /// Releases the widget lock held by `owner` (matplotlib's
    /// `widgetlock.release`); releasing a free lock is allowed.
    ///
    /// # Errors
    ///
    /// When another owner holds the lock.
    pub fn release_widgets(&self, owner: LockOwner) -> Result<(), LockError> {
        let mut state = self.state_mut();
        match state.widget_lock {
            Some(holder) if holder != owner => Err(LockError::NotOwner),
            _ => {
                state.widget_lock = None;
                Ok(())
            }
        }
    }

    /// Whether `owner` may use the pointer: the lock is free or its own
    /// (matplotlib's `widgetlock.available`).
    pub fn widgets_available(&self, owner: LockOwner) -> bool {
        self.state()
            .widget_lock
            .is_none_or(|holder| holder == owner)
    }

    /// Whether anyone holds the widget lock.
    pub fn widgets_locked(&self) -> bool {
        self.state().widget_lock.is_some()
    }

    /// Asks the window showing the figure for a cursor shape (matplotlib's
    /// `canvas.set_cursor`).
    pub fn set_cursor(&self, cursor: CursorShape) -> &Self {
        self.state_mut().cursor = cursor;
        self
    }

    /// The cursor shape last asked for.
    pub fn cursor(&self) -> CursorShape {
        self.state().cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_owner_at_a_time() {
        let fig = Figure::new();
        let (a, b) = (LockOwner::new(), LockOwner::new());
        assert!(!fig.widgets_locked());
        fig.lock_widgets(a).unwrap();
        fig.lock_widgets(a).unwrap();
        assert!(fig.widgets_available(a) && !fig.widgets_available(b));
        assert_eq!(fig.lock_widgets(b), Err(LockError::AlreadyLocked));
        assert_eq!(fig.release_widgets(b), Err(LockError::NotOwner));
        fig.release_widgets(a).unwrap();
        assert!(fig.widgets_available(b));
        fig.release_widgets(b).unwrap();
    }

    #[test]
    fn cursor_shapes_carry_matplotlib_names() {
        let names: Vec<&str> = CursorShape::ALL.iter().map(|shape| shape.name()).collect();
        assert_eq!(
            names,
            [
                "POINTER",
                "HAND",
                "SELECT_REGION",
                "MOVE",
                "WAIT",
                "RESIZE_HORIZONTAL",
                "RESIZE_VERTICAL"
            ]
        );
        assert_eq!(CursorShape::ALL[0], CursorShape::default());
    }

    #[test]
    fn tools_ask_for_cursors() {
        let fig = Figure::new();
        assert_eq!(fig.cursor(), CursorShape::Pointer);
        fig.set_cursor(CursorShape::Move);
        assert_eq!(fig.cursor(), CursorShape::Move);
    }
}
