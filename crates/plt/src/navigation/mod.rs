//! Interactive navigation of a figure (matplotlib's `NavigationToolbar2`
//! and the axes' pan/zoom methods), independent of any window system.
//!
//! A [`Navigation`] follows pointer and key input given in display
//! coordinates (pixels from the bottom-left corner of the figure, like
//! matplotlib's `event.x`/`event.y`):
//!
//! * in [`NavMode::Pan`], the left button drags the view and the right
//!   button zooms around the pointer (polar axes rotate their radial
//!   labels or scale the radius instead);
//! * in [`NavMode::Zoom`], dragging a rectangle zooms into it (left
//!   button) or out of it (right button);
//! * on 3-D axes, outside the toolbar modes, the left button rotates the
//!   view, the middle one pans and the right one zooms (matplotlib's
//!   `Axes3D.mouse_init`); the pan mode pans them and the zoom mode zooms
//!   to a rectangle;
//! * the wheel zooms around the pointer in any mode;
//! * every completed change is pushed on a view history browsed with
//!   [`Navigation::home`], [`Navigation::back`] and [`Navigation::forward`].
//!
//! [`Navigation::readout`] formats the data coordinates under the pointer
//! for a status bar (matplotlib's `format_coord`).

mod history;
mod orbit;
mod pan;
mod readout;
mod zoom;

use plt_core::{Bbox, Point};

use self::history::ViewStack;
use self::orbit::Orbit;
use self::pan::PanStart;
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{LockError, LockOwner};
use crate::figure::Figure;
use crate::mplot3d::Axes3D;

pub use self::readout::axes_at;

/// What dragging with the pointer does (matplotlib's toolbar mode).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NavMode {
    /// Nothing: the pointer only reports coordinates.
    #[default]
    None,
    /// Pan with the left button, zoom with the right.
    Pan,
    /// Zoom to a dragged rectangle.
    Zoom,
}

/// A pointer button, as far as navigation cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerButton {
    /// The primary (left) button.
    Left,
    /// The middle button or wheel click.
    Middle,
    /// The secondary (right) button.
    Right,
}

/// A key held while dragging that constrains the motion (matplotlib's
/// `event.key` during pan and zoom).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DragKey {
    /// No constraint.
    #[default]
    None,
    /// Only x changes (`x` key).
    X,
    /// Only y changes (`y` key).
    Y,
    /// Equal motion along both axes (control).
    Control,
    /// Snap to horizontal, vertical or diagonal motion (shift).
    Shift,
}

/// The zoom rectangle to show while dragging, in display coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rubberband {
    /// The rectangle (corners in drag order, not normalised).
    pub bbox: Bbox,
}

/// An interaction in progress.
#[derive(Debug)]
enum Drag {
    /// Panning the axes, each with its starting state.
    Pan {
        button: PointerButton,
        axes: Vec<(Axes, PanStart)>,
    },
    /// Zooming to a rectangle started at `start`.
    Zoom {
        button: PointerButton,
        start: Point,
        axes: Vec<Axes>,
    },
    /// Rotating, panning or zooming a 3-D axes.
    Orbit(Orbit),
    /// Zooming a 3-D axes to a rectangle started at `start`.
    Zoom3D {
        button: PointerButton,
        start: Point,
        axes: Axes3D,
    },
}

/// The navigation state of one figure window: the mode, the view history
/// and the interaction in progress.
#[derive(Debug, Default)]
pub struct Navigation {
    mode: NavMode,
    history: ViewStack,
    drag: Option<Drag>,
    /// Holds the figure's widget lock while a mode is active.
    owner: LockOwner,
}

/// How much one wheel step zooms.
const WHEEL_ZOOM: f64 = 1.25;

impl Navigation {
    /// Navigation with no mode and an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current mode.
    pub fn mode(&self) -> NavMode {
        self.mode
    }

    /// Switches to `mode`, or back to [`NavMode::None`] if it is already
    /// active (matplotlib's `pan()` and `zoom()` toggles); an interaction
    /// in progress is abandoned. An active mode holds the figure's widget
    /// lock, so that widgets leave the pointer alone.
    ///
    /// # Errors
    ///
    /// When a widget holds the lock (matplotlib's "pan unavailable").
    pub fn toggle(&mut self, figure: &Figure, mode: NavMode) -> Result<(), LockError> {
        if !figure.widgets_available(self.owner) {
            return Err(LockError::AlreadyLocked);
        }
        self.drag = None;
        if self.mode == mode {
            self.mode = NavMode::None;
            figure.release_widgets(self.owner)
        } else {
            self.mode = mode;
            figure.lock_widgets(self.owner)
        }
    }

    /// Whether [`Navigation::back`] would change the view.
    pub fn can_go_back(&self) -> bool {
        self.history.can_go_back()
    }

    /// Whether [`Navigation::forward`] would change the view.
    pub fn can_go_forward(&self) -> bool {
        self.history.can_go_forward()
    }

    /// Restores the first recorded view and records it again.
    pub fn home(&mut self, figure: &Figure) {
        self.history.home();
        self.history.restore(figure);
    }

    /// Steps back in the view history.
    pub fn back(&mut self, figure: &Figure) {
        self.history.back();
        self.history.restore(figure);
    }

    /// Steps forward in the view history.
    pub fn forward(&mut self, figure: &Figure) {
        self.history.forward();
        self.history.restore(figure);
    }

    /// Forgets the view history (matplotlib's `update()`).
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Records the current view of every axes.
    pub fn push_current(&mut self, figure: &Figure) {
        self.history.push(figure);
    }

    /// A button went down at display point `at`; returns whether an
    /// interaction started.
    pub fn press(&mut self, figure: &Figure, at: Point, button: PointerButton) -> bool {
        if self.mode == NavMode::None {
            // 3-D axes turn, pan and zoom with the mouse outside the
            // toolbar modes, recording the home view first.
            let Some(orbit) = Orbit::start(figure, at, button) else {
                return false;
            };
            self.record_home(figure);
            self.drag = Some(Drag::Orbit(orbit));
            return true;
        }
        if button == PointerButton::Middle {
            return false;
        }
        let zoom = self.mode == NavMode::Zoom;
        let axes = self.interaction_axes(figure, at, zoom);
        if axes.is_empty() {
            return self.press_3d(figure, at, button, zoom);
        }
        self.drag = Some(if zoom {
            Drag::Zoom {
                button,
                start: at,
                axes,
            }
        } else {
            let axes = axes
                .into_iter()
                .filter_map(|ax| PanStart::new(&ax, at, button).map(|start| (ax, start)))
                .collect();
            Drag::Pan { button, axes }
        });
        true
    }

    /// A press in the pan or zoom mode over a 3-D axes (and no 2-D one):
    /// panning with either button, or zooming to a rectangle.
    fn press_3d(&mut self, figure: &Figure, at: Point, button: PointerButton, zoom: bool) -> bool {
        let drag = if zoom {
            orbit::axes3d_at(figure, at).map(|axes| Drag::Zoom3D {
                button,
                start: at,
                axes,
            })
        } else {
            Orbit::pan(figure, at).map(Drag::Orbit)
        };
        if drag.is_some() {
            self.record_home(figure);
        }
        self.drag = drag;
        self.drag.is_some()
    }

    /// Records the current view as the home view if nothing was recorded.
    fn record_home(&mut self, figure: &Figure) {
        if self.history.is_empty() {
            self.history.push(figure);
        }
    }

    /// The pointer moved to `at` with `key` held; returns the zoom
    /// rectangle to show, if zooming.
    pub fn drag(&mut self, at: Point, key: DragKey) -> Option<Rubberband> {
        match self.drag.as_mut()? {
            Drag::Pan { button, axes } => {
                for (ax, start) in axes.iter() {
                    start.drag(ax, *button, key, at);
                }
                None
            }
            Drag::Zoom { start, axes, .. } => {
                let ax = axes.first()?;
                zoom::rubberband(ax, *start, at, key)
            }
            Drag::Orbit(orbit) => {
                orbit.drag(at, key);
                None
            }
            Drag::Zoom3D { start, axes, .. } => zoom::rubberband(&axes.plane(), *start, at, key),
        }
    }

    /// The button was released at `at` with `key` held: finishes the
    /// interaction and records the new view.
    pub fn release(&mut self, figure: &Figure, at: Point, key: DragKey) {
        match self.drag.take() {
            Some(Drag::Pan { .. } | Drag::Orbit(_)) => self.history.push(figure),
            Some(Drag::Zoom3D {
                button,
                start,
                axes,
            }) if !zoom::is_click(start, at, key) => {
                orbit::zoom_to_rect(&axes, start, at, button != PointerButton::Left, key);
                self.history.push(figure);
            }
            Some(Drag::Zoom {
                button,
                start,
                axes,
            }) if zoom::zoom_to_rect(figure, &axes, start, at, button, key) => {
                self.history.push(figure);
            }
            Some(Drag::Zoom { .. } | Drag::Zoom3D { .. }) | None => {}
        }
    }

    /// Whether an interaction is in progress.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Zooms around display point `at` by `steps` wheel steps (positive
    /// zooms in) and records the new view.
    pub fn wheel(&mut self, figure: &Figure, at: Point, steps: f64) {
        if steps == 0.0 {
            return;
        }
        let scale = WHEEL_ZOOM.powf(steps);
        let axes = self.interaction_axes(figure, at, true);
        if !axes.is_empty() {
            zoom::zoom_around(figure, &axes, at, scale);
        } else if let Some(ax) = orbit::axes3d_at(figure, at) {
            self.record_home(figure);
            // Limits only fail to update for degenerate scales, which
            // leave the view as it was.
            let _ = ax.scale_limits([1.0 / scale; 3]);
        } else {
            return;
        }
        self.history.push(figure);
    }

    /// The text for a status bar with the pointer at `at`: the data
    /// coordinates under it, then on a second line the value of an image
    /// (or other artist with `mouseover` set) under it, as `[0.532]`; an
    /// empty string outside every axes (matplotlib's
    /// `_mouse_event_to_message`).
    pub fn readout(&self, figure: &Figure, at: Point) -> String {
        if let Some(Drag::Orbit(orbit)) = &self.drag
            && orbit.is_rotating()
        {
            return orbit::readout(orbit.axes(), at, true);
        }
        if let Some(ax) = axes_at(figure, at) {
            if !ax.navigate() {
                return String::new();
            }
            let coords = readout::format_coord(&ax, at);
            let coords = coords.trim_end();
            return match readout::cursor_text(&ax, at) {
                Some(data) => format!("{coords}\n{data}"),
                None => coords.to_owned(),
            };
        }
        orbit::axes3d_at(figure, at).map_or_else(String::new, |ax| orbit::readout(&ax, at, false))
    }

    /// The axes an interaction at `at` acts on (matplotlib's
    /// `_start_event_axes_interaction`): the topmost axes under the
    /// pointer that can pan (or zoom), with their twins, down to the first
    /// one that keeps events from those below (by default, one with a
    /// visible background; see `Axes::set_forward_navigation_events`).
    /// Records the home view first.
    fn interaction_axes(&mut self, figure: &Figure, at: Point, zoom: bool) -> Vec<Axes> {
        let candidates: Vec<Axes> = figure
            .axes()
            .into_iter()
            .filter(|ax| {
                ax.navigate()
                    && readout::contains(ax, at)
                    && if zoom { ax.can_zoom() } else { ax.can_pan() }
            })
            .collect();
        if candidates.is_empty() {
            return candidates;
        }
        self.record_home(figure);
        let mut zorders: Vec<f64> = candidates.iter().map(ArtistHandle::zorder).collect();
        zorders.sort_by(|a, b| b.total_cmp(a));
        zorders.dedup();
        let mut chosen: Vec<Axes> = Vec::new();
        'levels: for zorder in zorders {
            let level = candidates
                .iter()
                .rev()
                .filter(|ax| ax.zorder().total_cmp(&zorder).is_eq());
            for ax in level {
                for member in std::iter::once(ax.clone()).chain(ax.twins()) {
                    if !chosen.contains(&member) {
                        chosen.push(member);
                    }
                }
                if ax.captures_navigation() {
                    break 'levels;
                }
            }
        }
        chosen
    }
}

#[cfg(test)]
mod tests;
