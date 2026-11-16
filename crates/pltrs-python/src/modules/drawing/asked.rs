//! Artists given a transform that computes its own matrix in the
//! program's code (an affine class with its own `get_matrix`, as mpltern's
//! shift of a tick label away from its corner computes it from where its
//! Axes is when asked).
//!
//! pltrs (Rust) holds such a transform by the matrix it had when it was
//! handed over; matplotlib asks it for its matrix wherever it draws or
//! measures the artist. The artist gets an [`AskingPainter`], which hands
//! pltrs (Rust) the transform again, through the package's
//! `ask_transform(artist, rendered)`, before every draw reaching the
//! artist, and draws it as pltrs draws it.

use pltrs::artist::{ArtistId, ArtistPainter, DrawContext, DrawTarget, Painter, RawHandle};

use crate::binding::lookup;
use crate::binding::prelude::*;

/// The painter of an artist whose given transform computes its matrix
/// in the program's code (see the module documentation).
#[derive(Debug)]
pub(in crate::modules) struct AskingPainter {
    /// A weak reference to the artist's Python object, which holds the
    /// artist's handle: a strong one would make a cycle through Rust.
    artist: Owned,
}

impl AskingPainter {
    /// The painter of the artist the weak reference `artist` refers to.
    pub(in crate::modules) fn new(artist: Owned) -> Self {
        Self { artist }
    }

    /// The artist's Python object, if it is alive.
    pub(in crate::modules) fn artist<'py>(&self, py: Python<'py>) -> Option<Bound<'py>> {
        let artist = self.artist.bind(py).call(&[], &[]).ok()?;
        (!artist.is_none()).then_some(artist)
    }

    /// Asks the artist's transforms as matplotlib's draw asks them, in a
    /// draw that renders the artist (`rendered`) or only lays the figure out
    /// (matplotlib's draw with drawing disabled, before a tight print), and
    /// hands pltrs (Rust) what they answered, through the package's
    /// `ask_transform(artist, rendered)`; nothing once the artist is gone.
    /// What the program's code raises is raised when the native call
    /// running returns.
    fn ask(&self, rendered: bool) {
        Python::with_gil(|py| {
            let artist = self.artist(py)?;
            let ask: Callback = lookup(py, "ask_transform").ok()?.extract().ok()?;
            ask.call::<Owned>((artist, rendered))
        });
    }
}

impl AskingPainter {
    /// Asks the artist's transforms as matplotlib's tight box of its Axes
    /// asks them (for a layout engine with `for_layout`), through the
    /// package's `ask_measured(artist, for_layout)`: only where that box
    /// takes the artist in.
    fn measured(&self, for_layout: bool) {
        Python::with_gil(|py| {
            let artist = self.artist(py)?;
            let ask: Callback = lookup(py, "ask_measured").ok()?.extract().ok()?;
            ask.call::<Owned>((artist, for_layout))
        });
    }
}

impl ArtistPainter for AskingPainter {
    fn prepare(&self, _artist: &RawHandle, target: DrawTarget<'_>) {
        self.ask(target.rendered);
    }

    fn draw(&self, ctx: &mut DrawContext<'_>, artist: ArtistId) {
        ctx.draw_unpainted(artist);
    }

    fn measure(&self, _artist: &RawHandle) {
        self.measured(false);
    }

    fn measure_for_layout(&self, _artist: &RawHandle) {
        self.measured(true);
    }
}

/// Has the artist `artist`, whose Python object the weak reference
/// `artist_ref` refers to, asked for its transform before every draw and
/// measure (`artist_ref` given), or no more (`None`). An artist the
/// program's class draws keeps that class' painter, whose draw reads the
/// transform as it is (`Artist.draw`); so does one asked already.
fn ask_for_transform(artist: RawHandle, artist_ref: Option<Owned>) -> PyResult<()> {
    let current = artist.painter();
    let asked = current
        .as_ref()
        .is_some_and(|painter| (painter.get() as &dyn std::any::Any).is::<AskingPainter>());
    match artist_ref {
        Some(artist_ref) if current.is_none() => {
            artist.set_painter(Some(Painter::new(AskingPainter::new(artist_ref))));
        }
        None if asked => artist.set_painter(None),
        _ => {}
    }
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("ask_for_transform", ask_for_transform)
}
