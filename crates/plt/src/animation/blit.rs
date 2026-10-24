//! Blitted animation frames (matplotlib's `Animation._blit_draw` and
//! `_blit_clear`): the artists a frame changed are drawn over the saved
//! background of their axes, and only those axes are shown again.

use std::rc::Rc;

use plt_core::Bbox;

use crate::artist::RawHandle;
use crate::axes::{Axes, AxesState};
use crate::figure::{BufferRegion, CanvasError, DrawErrors, Figure};
use crate::ticker::AxisName;

/// The limits of an axes and whether they follow the data (matplotlib's
/// `Axes._get_view()`): a saved background shows the axes in this view.
#[derive(Clone, Copy, Debug, PartialEq)]
struct View {
    xlim: (f64, f64),
    ylim: (f64, f64),
    autoscale: (bool, bool),
}

/// What draws an animated artist, and is blitted for it: its axes, or
/// the figure for an artist outside any 2-D axes.
#[derive(Clone, Debug, PartialEq)]
enum Owner {
    Axes(Axes),
    Figure,
}

impl Owner {
    /// The owner of `artist`: its closest 2-D axes, if any.
    fn of(artist: &RawHandle) -> Self {
        let state = artist.figure_state().borrow();
        let mut next = Some(artist.id());
        while let Some(id) = next {
            let Some(item) = state.store.get(id) else {
                break;
            };
            if state.store.get_as::<AxesState>(id).is_some() {
                let handle = RawHandle::new(Rc::clone(artist.figure_state()), id);
                return Self::Axes(Axes::from_raw(handle));
            }
            next = item.base().parent;
        }
        Self::Figure
    }

    /// The box the owner's background and blits cover.
    fn bbox(&self, figure: &Figure) -> Bbox {
        match self {
            Self::Axes(ax) => ax.bbox(),
            Self::Figure => figure.bbox(),
        }
    }

    /// The view a background of the owner shows (none for the figure).
    fn view(&self) -> Option<View> {
        match self {
            Self::Axes(ax) => Some(View {
                xlim: ax.xlim(),
                ylim: ax.ylim(),
                autoscale: (
                    ax.autoscale_on(Some(AxisName::X)),
                    ax.autoscale_on(Some(AxisName::Y)),
                ),
            }),
            Self::Figure => None,
        }
    }
}

/// A saved background (an entry of matplotlib's `_blit_cache`).
#[derive(Debug)]
struct Background {
    owner: Owner,
    view: Option<View>,
    region: BufferRegion,
}

/// Where a blitted animation stands: the saved backgrounds and the
/// artists of the last frame (matplotlib's `_blit_cache` and
/// `_drawn_artists`).
#[derive(Debug, Default)]
pub(super) struct Blitter {
    backgrounds: Vec<Background>,
    /// The artists the last frame changed, in z-order.
    pub(super) drawn: Vec<RawHandle>,
}

impl Blitter {
    /// Forgets every saved background (the figure was resized).
    pub(super) fn forget_backgrounds(&mut self) {
        self.backgrounds.clear();
    }

    /// The owners of the artists of the last frame, each once.
    fn owners(&self) -> Vec<Owner> {
        let mut owners: Vec<Owner> = Vec::new();
        for owner in self.drawn.iter().map(Owner::of) {
            if !owners.contains(&owner) {
                owners.push(owner);
            }
        }
        owners
    }

    /// Erases the artists of the last frame by putting back the saved
    /// background of their owners (matplotlib's `_blit_clear`); a
    /// background whose axes changed view, or that the canvas was drawn
    /// again since, is dropped instead.
    pub(super) fn clear(&mut self, figure: &Figure) {
        let canvas = figure.canvas();
        for owner in self.owners() {
            let Some(index) = self.backgrounds.iter().position(|b| b.owner == owner) else {
                continue;
            };
            let background = &self.backgrounds[index];
            let restored = background.view == owner.view()
                && canvas.restore_region(&background.region).is_ok();
            if !restored {
                self.backgrounds.remove(index);
            }
        }
    }

    /// Draws the artists of the last frame over the canvas and shows their
    /// owners (matplotlib's `_blit_draw`): first the background of every
    /// owner without one for its current view is saved, then the artists
    /// are drawn, then each owner is blitted.
    ///
    /// # Errors
    ///
    /// When a background cannot be saved or the canvas is gone, or an
    /// artist cannot be drawn (the others are, and every owner is
    /// blitted).
    pub(super) fn draw(&mut self, figure: &Figure) -> Result<(), CanvasError> {
        let canvas = figure.canvas();
        let owners = self.owners();
        for owner in &owners {
            let view = owner.view();
            let saved = self.backgrounds.iter().position(|b| &b.owner == owner);
            if saved.is_some_and(|index| self.backgrounds[index].view == view) {
                continue;
            }
            let background = Background {
                owner: owner.clone(),
                view,
                region: canvas.copy_from_bbox(&owner.bbox(figure))?,
            };
            match saved {
                Some(index) => self.backgrounds[index] = background,
                None => self.backgrounds.push(background),
            }
        }
        let mut problems = Vec::new();
        for artist in self.drawn.iter().filter(|artist| artist.is_attached()) {
            match figure.draw_artist(artist) {
                Ok(()) => {}
                Err(CanvasError::Draw(DrawErrors(errors))) => problems.extend(errors),
                Err(err) => return Err(err),
            }
        }
        for owner in &owners {
            canvas.blit(Some(&owner.bbox(figure)))?;
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(CanvasError::Draw(DrawErrors(problems)))
        }
    }
}
