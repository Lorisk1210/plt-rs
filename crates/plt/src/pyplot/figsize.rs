//! Figure sizes for pyplot's figures: sizes with units and missing sides
//! (matplotlib's `figure(figsize=...)`) and sizes fitting an aspect ratio
//! (matplotlib's `figaspect`).

use super::registry::new_figure;
use crate::figure::{FigSize, Figure, SizeError};

/// A new figure of `size` at `dpi`, made current (matplotlib's
/// `figure(figsize=size, dpi=dpi)`): sizes in inches, centimetres or
/// pixels, a missing side taken from `figure.figsize`.
///
/// ```
/// use ::plt::figure::{FigSize, SizeUnit};
/// use plt::pyplot as plt;
/// let size = FigSize::new(Some(600.0), Some(400.0), SizeUnit::Pixels);
/// let fig = plt::figure_with_figsize(&size, 100.0)?;
/// assert_eq!(fig.size_inches(), (6.0, 4.0));
/// plt::close_all();
/// # Ok::<(), ::plt::figure::SizeError>(())
/// ```
///
/// # Errors
///
/// As [`FigSize::inches`]; no figure is made.
pub fn figure_with_figsize(size: &FigSize, dpi: f64) -> Result<Figure, SizeError> {
    Figure::with_figsize(size, dpi).map(|fig| new_figure(fig, String::new()))
}

/// The figure size in inches for an axes of height-to-width ratio
/// `ratio` (matplotlib's `figaspect`): as tall as `figure.figsize`, as
/// wide as the ratio asks, then scaled to fit within 4 × 2 and 16 × 16
/// inches.
///
/// ```
/// assert_eq!(plt::pyplot::figaspect(2.0), (4.0, 8.0));
/// ```
pub fn figaspect(ratio: f64) -> (f64, f64) {
    let height = crate::rc::params().figure_figsize[1];
    let (min, max) = ([4.0, 2.0], [16.0, 16.0]);
    let size = [height / ratio, height];
    let shrink = (size[0] / min[0]).min(size[1] / min[1]).min(1.0);
    let size = size.map(|side| side / shrink);
    let grow = (size[0] / max[0]).max(size[1] / max[1]).max(1.0);
    let size = size.map(|side| side / grow);
    (size[0].clamp(min[0], max[0]), size[1].clamp(min[1], max[1]))
}

/// The figure size in inches for an array of `rows × cols`
/// (matplotlib's `figaspect` of an array): [`figaspect`] of its aspect,
/// `figure.figsize` for an empty array.
///
/// ```
/// assert_eq!(plt::pyplot::figaspect_of_shape(4, 1), (4.0, 16.0));
/// ```
pub fn figaspect_of_shape(rows: usize, cols: usize) -> (f64, f64) {
    if rows == 0 || cols == 0 {
        let [width, height] = crate::rc::params().figure_figsize;
        return (width, height);
    }
    figaspect(rows as f64 / cols as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::SizeUnit;
    use crate::pyplot::{close_all, get_fignums};

    #[test]
    fn figaspect_follows_matplotlib() {
        // matplotlib's test_figaspect, at the default 6.4 x 4.8 in.
        assert_eq!(figaspect(2.0), (4.0, 8.0));
        assert_eq!(figaspect_of_shape(1, 2), (9.6, 4.8));
        assert_eq!(figaspect_of_shape(2, 2), (4.8, 4.8));
        assert_eq!(figaspect_of_shape(0, 3), (6.4, 4.8));
    }

    #[test]
    fn invalid_sizes_make_no_figure() {
        close_all();
        let unspecified = FigSize::new(None, None, SizeUnit::Inches);
        assert_eq!(
            figure_with_figsize(&unspecified, 100.0),
            Err(SizeError::Unspecified)
        );
        assert_eq!(get_fignums(), [] as [u32; 0]);
        let fig = figure_with_figsize(&FigSize::new(None, Some(2.0), SizeUnit::Inches), 100.0)
            .expect("valid size");
        assert_eq!(fig.size_inches(), (6.4, 2.0));
        assert_eq!(get_fignums(), [1]);
        close_all();
    }
}
