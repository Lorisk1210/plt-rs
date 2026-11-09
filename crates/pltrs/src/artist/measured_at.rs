//! The resolution of a renderer the program hands a measuring method
//! (matplotlib's `get_window_extent(renderer)`, `get_tightbbox(renderer)`,
//! `OffsetBox.get_bbox(renderer)`) when it is not the figure's.
//!
//! matplotlib measures with the renderer it is handed: its lines of text
//! come out that renderer's size (`get_text_width_height_descent`), and so
//! do the pads its code converts with the renderer's `points_to_pixels`
//! (the pads and separations of offset boxes, the pads of axisartist's
//! labels, the pads of table cells), while the figure stays laid out,
//! spaced and placed at its own resolution.

use std::cell::Cell;

use super::LayoutContext;

thread_local! {
    /// The resolution set by [`set_measured_at`], if any.
    static MEASURED_AT: Cell<Option<f64>> = const { Cell::new(None) };
}

/// Has what matplotlib measures with the renderer it is handed measured at
/// `dpi` from now on (`None`: at the resolution the figure is laid out
/// at): the width, height and descent of every line of text, and the pads
/// [`LayoutContext::measuring_points_to_pixels`] converts; the figure is
/// still laid out, spaced and placed at its own resolution. Returns the
/// resolution set before.
///
/// ```
/// use pltrs::artist::{ArtistHandle, set_measured_at};
/// use pltrs::figure::Figure;
/// let fig = Figure::with_size(6.4, 4.8, 100.0);
/// let note = fig.text(0.5, 0.5, "note");
/// let at_figure = note.raw().extent_as_is().expect("measured");
/// let before = set_measured_at(Some(50.0));
/// let at_half = note.raw().extent_as_is().expect("measured");
/// set_measured_at(before);
/// assert!((at_half.width() - at_figure.width() / 2.0).abs() < 2.0);
/// assert_eq!(at_half.x0(), at_figure.x0(), "placed at the figure's resolution");
/// ```
pub fn set_measured_at(dpi: Option<f64>) -> Option<f64> {
    MEASURED_AT.replace(dpi)
}

/// The resolution things laid out at `dpi` are measured at, when it is
/// another (see [`set_measured_at`]).
pub(crate) fn measured_apart(dpi: f64) -> Option<f64> {
    MEASURED_AT.get().filter(|at| at.to_bits() != dpi.to_bits())
}

impl LayoutContext<'_> {
    /// Converts points to pixels as matplotlib's code converts them with
    /// the renderer it measures with (`renderer.points_to_pixels`): at the
    /// resolution [`set_measured_at`] set, else at the canvas'.
    pub fn measuring_points_to_pixels(&self, points: f64) -> f64 {
        let dpi = self.dpi();
        points * measured_apart(dpi).unwrap_or(dpi) / 72.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn pads_follow_the_measuring_resolution_only_while_it_is_set() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let state = fig.state();
        let ctx = LayoutContext::new(&state);
        assert_eq!(ctx.measuring_points_to_pixels(72.0), 100.0);
        let before = set_measured_at(Some(50.0));
        assert_eq!(ctx.measuring_points_to_pixels(72.0), 50.0);
        assert_eq!(
            ctx.points_to_pixels(72.0),
            100.0,
            "the canvas keeps its own"
        );
        set_measured_at(before);
        assert_eq!(ctx.measuring_points_to_pixels(72.0), 100.0);
    }
}
