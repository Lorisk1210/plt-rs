//! How a selector shows its changes (matplotlib's `_SelectorWidget.update`
//! and `update_background`): blitting, its animated artists drawn alone
//! over a saved background of its axes, or a full draw of the figure.
//!
//! A blitting selector (`useblit`) saves the background of its axes,
//! without its artists or the figure's other animated artists, whenever
//! the canvas is drawn. An update puts that background back, draws those
//! artists over it in z-order and blits the axes; without a background it
//! saves one first. A selector that does not blit asks for a full draw
//! instead (`draw_idle`).
//!
//! Selectors ask for updates where matplotlib's call `update()`, while
//! their state is borrowed; the updates run once it is let go, at the end
//! of the event or method that asked (see [`run_requested`]), so that the
//! draws they make, and the callbacks those run, find every selector free.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

use crate::artist::{ArtistHandle, RawHandle};
use crate::axes::Axes;
use crate::figure::BufferRegion;

thread_local! {
    /// The updates selectors asked for and that have not run yet, in the
    /// order they were asked for.
    static REQUESTED: RefCell<Vec<Rc<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
    /// Whether [`run_requested`] is running the updates.
    static RUNNING: Cell<bool> = const { Cell::new(false) };
}

/// Runs a selector's update: what [`Updater::request`] queues.
#[derive(Clone)]
pub(super) struct Updater(Rc<dyn Fn()>);

impl fmt::Debug for Updater {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Updater")
    }
}

impl Updater {
    /// The updater running `update`.
    pub(super) fn new(update: impl Fn() + 'static) -> Self {
        Self(Rc::new(update))
    }

    /// An updater that does nothing, until the selector it belongs to is
    /// connected.
    pub(super) fn none() -> Self {
        Self::new(|| {})
    }

    /// Asks for an update, which [`run_requested`] runs (matplotlib's
    /// `update()`).
    pub(super) fn request(&self) {
        REQUESTED.with_borrow_mut(|requested| requested.push(Rc::clone(&self.0)));
    }
}

/// Runs the updates asked for so far, and those they ask for, in order,
/// each after the one before ended: called again by an event an update
/// sends (the draw event of its draw), it leaves them to the call running
/// them. Every selector method and event handler that may ask for an
/// update calls it once the selector's state is let go.
pub(super) fn run_requested() {
    if RUNNING.replace(true) {
        return;
    }
    while let Some(update) =
        REQUESTED.with_borrow_mut(|requested| (!requested.is_empty()).then(|| requested.remove(0)))
    {
        update();
    }
    RUNNING.set(false);
}

/// The background a blitting selector draws its artists over.
#[derive(Debug, Default)]
pub(super) struct Background(RefCell<Option<BufferRegion>>);

/// The artists of every axes of `ax`'s figure that are animated, except
/// `own` (matplotlib's `_get_animated_artists`).
fn other_animated_artists(ax: &Axes, own: &[RawHandle]) -> Vec<RawHandle> {
    ax.figure()
        .axes()
        .iter()
        .flat_map(Axes::added_children)
        .filter(|artist| artist.animated() && !own.contains(artist))
        .collect()
}

/// `own` and the other animated artists of the figure, in z-order (stable
/// for equal z-orders, as Python's `sorted`).
fn drawn_over_background(ax: &Axes, own: &[RawHandle]) -> Vec<RawHandle> {
    let mut artists: Vec<RawHandle> = own.to_vec();
    artists.extend(other_animated_artists(ax, own));
    artists.sort_by(|a, b| a.zorder().total_cmp(&b.zorder()));
    artists
}

impl Background {
    /// Saves the background of `ax` for the selector of the artists `own`
    /// (matplotlib's `update_background`): when one of them, or another
    /// animated artist, shows, the figure is drawn without them first,
    /// then they are drawn back over it. Nothing while the figure is saved.
    pub(super) fn save(&self, ax: &Axes, own: &[RawHandle]) {
        let figure = ax.figure();
        if figure.state().saving {
            return;
        }
        let canvas = figure.canvas();
        let artists = drawn_over_background(ax, own);
        let shown: Vec<&RawHandle> = artists.iter().filter(|artist| artist.visible()).collect();
        for artist in &shown {
            artist.set_visible(false);
        }
        if !shown.is_empty() {
            canvas.report(canvas.draw());
        }
        match canvas.copy_from_bbox(&ax.bbox()) {
            Ok(region) => *self.0.borrow_mut() = Some(region),
            Err(err) => canvas.report(Err(err)),
        }
        for artist in &shown {
            artist.set_visible(true);
        }
        if !shown.is_empty() {
            for artist in &artists {
                canvas.report(ax.draw_artist(artist));
            }
        }
    }

    /// Shows the selector of the artists `own` as it now is (matplotlib's
    /// `update` when blitting): the saved background put back (saved first
    /// when there is none, or the canvas changed since), the artists drawn
    /// over it and the axes blitted.
    pub(super) fn update(&self, ax: &Axes, own: &[RawHandle]) {
        let canvas = ax.figure().canvas();
        let restored = self
            .0
            .borrow()
            .as_ref()
            .is_some_and(|region| canvas.restore_region(region).is_ok());
        if !restored {
            self.save(ax, own);
        }
        for artist in drawn_over_background(ax, own) {
            canvas.report(ax.draw_artist(&artist));
        }
        canvas.report(canvas.blit(Some(&ax.bbox())));
    }
}

#[cfg(test)]
mod tests;
