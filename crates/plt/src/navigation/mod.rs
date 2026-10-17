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
//! * the wheel zooms around the pointer in any mode;
//! * every completed change is pushed on a view history browsed with
//!   [`Navigation::home`], [`Navigation::back`] and [`Navigation::forward`].
//!
//! [`Navigation::readout`] formats the data coordinates under the pointer
//! for a status bar (matplotlib's `format_coord`).

mod history;
mod pan;
mod readout;
mod zoom;

use plt_core::{Bbox, Point};

use self::history::ViewStack;
use self::pan::PanStart;
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::figure::Figure;

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
}

/// The navigation state of one figure window: the mode, the view history
/// and the interaction in progress.
#[derive(Debug, Default)]
pub struct Navigation {
    mode: NavMode,
    history: ViewStack,
    drag: Option<Drag>,
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
    /// in progress is abandoned.
    pub fn toggle(&mut self, mode: NavMode) {
        self.drag = None;
        self.mode = if self.mode == mode {
            NavMode::None
        } else {
            mode
        };
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
        if button == PointerButton::Middle || self.mode == NavMode::None {
            return false;
        }
        let zoom = self.mode == NavMode::Zoom;
        let axes = self.interaction_axes(figure, at, zoom);
        if axes.is_empty() {
            return false;
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

    /// The pointer moved to `at` with `key` held; returns the zoom
    /// rectangle to show, if zooming.
    pub fn drag(&mut self, at: Point, key: DragKey) -> Option<Rubberband> {
        match self.drag.as_ref()? {
            Drag::Pan { button, axes } => {
                for (ax, start) in axes {
                    start.drag(ax, *button, key, at);
                }
                None
            }
            Drag::Zoom { start, axes, .. } => {
                let ax = axes.first()?;
                zoom::rubberband(ax, *start, at, key)
            }
        }
    }

    /// The button was released at `at` with `key` held: finishes the
    /// interaction and records the new view.
    pub fn release(&mut self, figure: &Figure, at: Point, key: DragKey) {
        match self.drag.take() {
            Some(Drag::Pan { .. }) => self.history.push(figure),
            Some(Drag::Zoom {
                button,
                start,
                axes,
            }) if zoom::zoom_to_rect(figure, &axes, start, at, button, key) => {
                self.history.push(figure);
            }
            Some(Drag::Zoom { .. }) | None => {}
        }
    }

    /// Whether an interaction is in progress.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Zooms around display point `at` by `steps` wheel steps (positive
    /// zooms in) and records the new view.
    pub fn wheel(&mut self, figure: &Figure, at: Point, steps: f64) {
        let axes = self.interaction_axes(figure, at, true);
        if axes.is_empty() || steps == 0.0 {
            return;
        }
        let scale = WHEEL_ZOOM.powf(steps);
        zoom::zoom_around(figure, &axes, at, scale);
        self.history.push(figure);
    }

    /// The text for a status bar with the pointer at `at`: the data
    /// coordinates under it, or an empty string outside every axes.
    pub fn readout(&self, figure: &Figure, at: Point) -> String {
        axes_at(figure, at)
            .filter(Axes::navigate)
            .map_or_else(String::new, |ax| readout::format_coord(&ax, at))
    }

    /// The axes an interaction at `at` acts on (matplotlib's
    /// `_start_event_axes_interaction`): the topmost axes under the
    /// pointer that can pan (or zoom), with their twins, down to the first
    /// one with a visible background. Records the home view first.
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
        if self.history.is_empty() {
            self.history.push(figure);
        }
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
                if ax.patch_visible() {
                    break 'levels;
                }
            }
        }
        chosen
    }
}
