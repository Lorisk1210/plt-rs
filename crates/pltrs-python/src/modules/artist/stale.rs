//! Whether an artist changed since it was last drawn (matplotlib's
//! `Artist.stale`), as pltrs (Rust) tracks it for every artist of a
//! figure, the figure itself included, how many times it was drawn, and
//! the artists a change marked stale, reported to the package for the
//! program's stale callbacks (`pltrs::figure::Figure::report_stale`).

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::figure::Figure;

use crate::binding::prelude::*;

/// Whether the artist changed since it was last drawn (`stale`).
fn get_stale(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.stale())
}

/// Marks the artist as changed, with what contains it unless it is
/// animated, or as drawn (`stale = value`): the program setting it runs
/// the callbacks itself, so the change is not reported.
fn set_stale(artist: RawHandle, stale: bool) -> PyResult<()> {
    pltrs::figure::unreported(|| artist.set_stale(stale));
    Ok(())
}

/// Has the figure report the artists its changes mark stale, or no
/// longer (`on`), for [`deliver_stale_reports`] to hand the package.
fn report_stale(figure: Figure, on: bool) -> PyResult<()> {
    figure.report_stale(on);
    Ok(())
}

/// Calls `function` with `argument`, the artists its changes mark stale
/// reported to no one: a mirror handing its part its state, which
/// matplotlib's part takes as its own, staling only itself.
fn unreported(function: Owned, argument: Owned) -> PyResult<()> {
    Python::with_gil(|py| {
        pltrs::figure::unreported(|| function.bind(py).call(&[argument.bind(py)], &[]).map(drop))
    })
}

/// The artists of the figure marked stale since they were last taken,
/// each once, in the order of their first change.
fn take_stale_reports(figure: Figure) -> PyResult<Vec<RawHandle>> {
    Ok(figure.take_stale_reports())
}

/// Has the package take what the figures reporting report at the end of
/// the thread's outermost native call, once a figure of the thread was
/// asked to report (`_artist_staleness.stale_reported`, registered as
/// `"stale_reported"`), which runs the stale callbacks matplotlib would
/// have run meanwhile.
pub(in crate::modules) fn deliver_stale_reports(py: Python<'_>) -> PyResult<()> {
    if !crate::binding::outermost_call() || !pltrs::figure::stale_reports_asked() {
        return Ok(());
    }
    crate::binding::lookup(py, "stale_reported")?.call(&[], &[])?;
    Ok(())
}

/// How many times the artist was drawn, which tells what matplotlib's
/// `draw` methods compute as they draw from what they hold before.
fn draw_count(artist: RawHandle) -> PyResult<u64> {
    Ok(artist.draw_count())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_stale", get_stale)?;
    module.add("set_stale", set_stale)?;
    module.add("report_stale", report_stale)?;
    module.add("take_stale_reports", take_stale_reports)?;
    module.add("unreported", unreported)?;
    module.add("draw_count", draw_count)
}
