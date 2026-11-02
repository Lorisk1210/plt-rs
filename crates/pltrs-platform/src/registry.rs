//! The windows of the native backends (macOS, Windows), kept per thread.
//!
//! Those operating systems report what happens to a window by calling
//! into the process: a window procedure, Objective-C methods. They do so
//! from within the backend's own calls (dispatching a message, resizing
//! or closing a window), where the callbacks cannot reach the backend. So
//! each window's state lives in a registry of its thread instead, where
//! callbacks find it by window id and queue events for the loop owning
//! the window; the backend takes them in [`Backend::poll`].
//!
//! Windows of these systems belong to the thread that made them, as the
//! event loop does, and a thread may run several loops: events are queued
//! per loop.
//!
//! [`Backend::poll`]: crate::backend::Backend::poll

use std::collections::BTreeMap;

use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::window::WindowId;

/// Identifies one event loop among those of a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoopId(u64);

/// A window's state and the loop it belongs to.
#[derive(Debug)]
struct Entry<S> {
    owner: LoopId,
    state: S,
}

/// The windows of one thread, with the state `S` a backend keeps for
/// each, and the events queued for its loops.
#[derive(Debug)]
pub(crate) struct Registry<S> {
    next_loop: u64,
    windows: BTreeMap<WindowId, Entry<S>>,
    events: Vec<(LoopId, Event)>,
}

impl<S> Registry<S> {
    /// An empty registry.
    pub(crate) const fn new() -> Self {
        Self {
            next_loop: 0,
            windows: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    /// Identifies a new loop.
    pub(crate) fn new_loop(&mut self) -> LoopId {
        self.next_loop += 1;
        LoopId(self.next_loop)
    }

    /// Adds `window`, belonging to the loop `owner`.
    pub(crate) fn insert(&mut self, owner: LoopId, window: WindowId, state: S) {
        self.windows.insert(window, Entry { owner, state });
    }

    /// Removes `window` and the events queued for it; returns its state.
    pub(crate) fn remove(&mut self, window: WindowId) -> Option<S> {
        let entry = self.windows.remove(&window)?;
        self.events
            .retain(|(_, event)| target(event) != Some(window));
        Some(entry.state)
    }

    /// The state of `window`, a window of the loop `owner`.
    pub(crate) fn get(&self, owner: LoopId, window: WindowId) -> Result<&S, Error> {
        match self.windows.get(&window) {
            Some(entry) if entry.owner == owner => Ok(&entry.state),
            _ => Err(Error::UnknownWindow(window)),
        }
    }

    /// The state of `window`, a window of the loop `owner`, to change.
    pub(crate) fn get_mut(&mut self, owner: LoopId, window: WindowId) -> Result<&mut S, Error> {
        match self.windows.get_mut(&window) {
            Some(entry) if entry.owner == owner => Ok(&mut entry.state),
            _ => Err(Error::UnknownWindow(window)),
        }
    }

    /// The state of `window`, whichever loop it belongs to (for the
    /// operating system's callbacks).
    pub(crate) fn state_mut(&mut self, window: WindowId) -> Option<&mut S> {
        self.windows.get_mut(&window).map(|entry| &mut entry.state)
    }

    /// The windows of every loop (all of which a request to quit the
    /// application closes, on macOS).
    #[cfg(any(target_os = "macos", test))]
    pub(crate) fn all_windows(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    /// The windows of the loop `owner`.
    pub(crate) fn windows(&self, owner: LoopId) -> Vec<WindowId> {
        self.windows
            .iter()
            .filter(|(_, entry)| entry.owner == owner)
            .map(|(&window, _)| window)
            .collect()
    }

    /// Queues `event` for `window`'s loop; returns `false`, queuing
    /// nothing, if `window` is unknown.
    pub(crate) fn push(&mut self, window: WindowId, event: WindowEvent) -> bool {
        let Some(entry) = self.windows.get(&window) else {
            return false;
        };
        self.events
            .push((entry.owner, Event::Window { window, event }));
        true
    }

    /// Queues the pointer motion `event` for `window`'s loop, replacing a
    /// motion of that window queued last.
    pub(crate) fn push_motion(&mut self, window: WindowId, event: WindowEvent) {
        if let Some((
            _,
            Event::Window {
                window: last,
                event: WindowEvent::CursorMoved { .. },
            },
        )) = self.events.last()
            && *last == window
        {
            self.events.pop();
        }
        self.push(window, event);
    }

    /// Takes the events queued for the loop `owner`, oldest first.
    pub(crate) fn take_events(&mut self, owner: LoopId) -> Vec<Event> {
        let (mine, others) = std::mem::take(&mut self.events)
            .into_iter()
            .partition(|(queued_for, _)| *queued_for == owner);
        self.events = others;
        mine.into_iter().map(|(_, event)| event).collect()
    }
}

/// The window an event is for.
fn target(event: &Event) -> Option<WindowId> {
    match event {
        Event::Window { window, .. } => Some(*window),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{PhysicalPosition, PhysicalSize};
    use crate::keyboard::Modifiers;

    fn moved(x: f64) -> WindowEvent {
        WindowEvent::CursorMoved {
            position: PhysicalPosition::new(x, 0.0),
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn events_are_queued_per_loop() {
        let mut registry = Registry::new();
        let (first, second) = (registry.new_loop(), registry.new_loop());
        let (a, b) = (WindowId::from_raw(1), WindowId::from_raw(2));
        registry.insert(first, a, "a");
        registry.insert(second, b, "b");
        assert!(registry.push(a, WindowEvent::Focused(true)));
        assert!(registry.push(b, WindowEvent::CloseRequested));
        assert!(!registry.push(WindowId::from_raw(3), WindowEvent::CloseRequested));
        assert_eq!(
            registry.take_events(first),
            [Event::Window {
                window: a,
                event: WindowEvent::Focused(true)
            }]
        );
        assert_eq!(registry.take_events(first), []);
        assert_eq!(registry.windows(second), [b]);
        assert_eq!(registry.all_windows(), [a, b]);
        assert_eq!(registry.take_events(second).len(), 1);
    }

    #[test]
    fn windows_belong_to_their_loop() {
        let mut registry = Registry::new();
        let (first, second) = (registry.new_loop(), registry.new_loop());
        let window = WindowId::from_raw(7);
        registry.insert(first, window, 1);
        assert_eq!(registry.get(first, window), Ok(&1));
        assert_eq!(
            registry.get(second, window),
            Err(Error::UnknownWindow(window))
        );
        *registry.get_mut(first, window).unwrap() = 2;
        assert!(registry.get_mut(second, window).is_err());
        assert_eq!(registry.state_mut(window), Some(&mut 2));
        registry.push(window, WindowEvent::Resized(PhysicalSize::new(1, 1)));
        assert_eq!(registry.remove(window), Some(2));
        assert_eq!(
            registry.take_events(first),
            [],
            "the removed window's events"
        );
        assert_eq!(registry.remove(window), None);
        assert_eq!(registry.state_mut(window), None);
    }

    #[test]
    fn consecutive_motions_coalesce() {
        let mut registry = Registry::new();
        let owner = registry.new_loop();
        let (a, b) = (WindowId::from_raw(1), WindowId::from_raw(2));
        registry.insert(owner, a, ());
        registry.insert(owner, b, ());
        registry.push_motion(a, moved(1.0));
        registry.push_motion(a, moved(2.0));
        registry.push_motion(b, moved(3.0));
        registry.push_motion(a, moved(4.0));
        let events = registry.take_events(owner);
        let expected = [(a, 2.0), (b, 3.0), (a, 4.0)].map(|(window, x)| Event::Window {
            window,
            event: moved(x),
        });
        assert_eq!(events, expected);
    }
}
