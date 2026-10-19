//! Making, choosing and removing axes, clearing and saving figures
//! (matplotlib's `subplots`, `subplot`, `subplot2grid`, `subplot_mosaic`,
//! `axes`, `twinx`, `polar`, `cla`, `clf`, `savefig`, ...).

use std::path::Path;

use super::error::PyplotError;
use super::registry::{figure, gca, gcf};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, ProjectionKind};
use crate::figure::{
    Figure, Mosaic, MosaicAxes, MosaicError, MosaicOptions, SaveError, SaveOptions, Share,
};
use crate::gridspec::{GridSpec, GridSpecError};
use crate::lines::Line2D;
use crate::numeric::Grid;
use crate::units::AxisValues;

/// A new current figure with a grid of axes (matplotlib's `subplots`).
///
/// # Errors
///
/// When the grid has no cells.
pub fn subplots(nrows: usize, ncols: usize) -> Result<(Figure, Grid<Axes>), GridSpecError> {
    subplots_shared(nrows, ncols, Share::None, Share::None)
}

/// A new current figure with a grid of axes sharing their x and y axes as
/// requested (matplotlib's `subplots(sharex=..., sharey=...)`).
///
/// # Errors
///
/// When the grid has no cells.
pub fn subplots_shared(
    nrows: usize,
    ncols: usize,
    sharex: Share,
    sharey: Share,
) -> Result<(Figure, Grid<Axes>), GridSpecError> {
    let fig = figure();
    let axes = fig.subplots_shared(nrows, ncols, sharex, sharey)?;
    Ok((fig, axes))
}

/// The axes in cell `index` (1-based, row-major) of an `nrows × ncols`
/// grid of the current figure, reusing an axes already there (matplotlib's
/// `subplot`); it becomes the current axes.
///
/// # Errors
///
/// When the grid is empty or `index` is outside it.
pub fn subplot(nrows: usize, ncols: usize, index: usize) -> Result<Axes, GridSpecError> {
    let fig = gcf();
    let spec = fig.numbered_cell(nrows, ncols, index)?;
    let existing = fig
        .axes()
        .into_iter()
        .find(|ax| ax.subplot_spec().as_ref() == Some(&spec));
    let ax = match existing {
        Some(ax) => ax,
        None => fig.add_subplot_spec(&spec)?,
    };
    fig.sca(&ax);
    Ok(ax)
}

/// A new current axes spanning `rowspan × colspan` cells from cell `loc`
/// (row, column) of a `shape` (rows, columns) grid of the current figure
/// (matplotlib's `subplot2grid`). The grid of the figure's other axes is
/// reused when it has that shape, so layout engines see a single grid.
///
/// # Errors
///
/// When the grid is empty or the block leaves it.
pub fn subplot2grid(
    shape: (usize, usize),
    loc: (usize, usize),
    rowspan: usize,
    colspan: usize,
) -> Result<Axes, GridSpecError> {
    let fig = gcf();
    let grid = match fig.existing_gridspec(shape.0, shape.1) {
        Some(grid) => grid,
        None => GridSpec::new(shape.0, shape.1)?,
    };
    let (row, col) = loc;
    let spec = grid.span(row..row + rowspan, col..col + colspan)?;
    fig.add_subplot_spec(&spec)
}

/// A new current figure with axes laid out by `mosaic` (matplotlib's
/// `subplot_mosaic`).
///
/// # Errors
///
/// As [`Figure::subplot_mosaic`].
pub fn subplot_mosaic(
    mosaic: &Mosaic,
    options: &MosaicOptions,
) -> Result<(Figure, MosaicAxes), MosaicError> {
    let fig = figure();
    let axes = fig.subplot_mosaic(mosaic, options)?;
    Ok((fig, axes))
}

/// A new current axes in the current figure: at `rect`
/// (`[left, bottom, width, height]` in figure fractions), or filling the
/// default subplot area when `None` (matplotlib's `axes`).
pub fn axes(rect: Option<[f64; 4]>) -> Axes {
    let fig = gcf();
    match rect {
        Some(rect) => fig.add_axes(rect),
        None => fig.subplot(),
    }
}

/// A new current axes sharing the x axis of `ax`, or of the current axes,
/// with its y axis on the right (matplotlib's `twinx`).
pub fn twinx(ax: Option<&Axes>) -> Axes {
    ax.cloned().unwrap_or_else(gca).twinx()
}

/// A new current axes sharing the y axis of `ax`, or of the current axes,
/// with its x axis on top (matplotlib's `twiny`).
pub fn twiny(ax: Option<&Axes>) -> Axes {
    ax.cloned().unwrap_or_else(gca).twiny()
}

/// Plots `r` against the angles `theta` (radians) on polar axes
/// (matplotlib's `polar`): a new polar subplot when the current figure
/// has no axes, else the current axes.
///
/// # Errors
///
/// [`PyplotError::NotPolar`] when the current axes is not polar (which
/// matplotlib deprecates), or the data as [`Axes::plot`].
pub fn polar(
    theta: impl Into<AxisValues>,
    r: impl Into<AxisValues>,
) -> Result<Line2D, PyplotError> {
    let fig = gcf();
    let ax = if fig.axes().is_empty() {
        fig.polar_subplot().axes().clone()
    } else {
        fig.gca()
    };
    if ax.projection() != ProjectionKind::Polar {
        return Err(PyplotError::NotPolar);
    }
    Ok(ax.plot(theta, r)?)
}

/// Clears the current axes (matplotlib's `cla`).
pub fn cla() {
    gca().clear();
}

/// Clears the current figure (matplotlib's `clf`).
pub fn clf() {
    gcf().clear();
}

/// Removes `ax` from its figure, or the current axes when `None`
/// (matplotlib's `delaxes`).
pub fn delaxes(ax: Option<&Axes>) {
    match ax {
        Some(ax) => ax.remove(),
        None => gca().remove(),
    }
}

/// Saves the current figure in the format its extension names (PNG, JPEG,
/// TIFF, GIF, WebP, SVG, PDF, PS or EPS; matplotlib's `savefig`).
///
/// # Errors
///
/// When the figure cannot be rendered or written.
pub fn savefig(path: impl AsRef<Path>) -> Result<(), SaveError> {
    gcf().savefig(path)
}

/// Saves the current figure with explicit options (matplotlib's
/// `savefig(fname, dpi=..., bbox_inches=..., ...)`).
///
/// # Errors
///
/// When the figure cannot be rendered or written.
pub fn savefig_with(path: impl AsRef<Path>, options: &SaveOptions) -> Result<(), SaveError> {
    gcf().savefig_with(path, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyplot::{close_all, plot, sca, title};

    #[test]
    fn plotting_goes_to_the_current_axes() {
        close_all();
        let line = plot(vec![0.0, 1.0], vec![0.0, 2.0]).unwrap();
        let ax = gca();
        assert!(gcf().contains_artist(&line));
        assert_eq!(gcf().axes(), std::slice::from_ref(&ax));
        let right = subplot(1, 2, 2).unwrap();
        assert_eq!(gca(), right);
        assert_eq!(subplot(1, 2, 2).unwrap(), right, "subplot reuses the cell");
        sca(&ax);
        assert_eq!(gca(), ax);
        title("t");
        assert_eq!(ax.title(crate::axes::TitleLocation::Center), "t");
        close_all();
    }

    #[test]
    fn clearing_and_removing() {
        close_all();
        plot(vec![0.0, 1.0], vec![0.0, 2.0]).unwrap();
        cla();
        assert_eq!(gca().lines(), Vec::<Line2D>::new());
        delaxes(None);
        assert_eq!(gcf().axes(), Vec::<Axes>::new());
        subplot(2, 1, 1).unwrap();
        clf();
        assert_eq!(gcf().axes(), Vec::<Axes>::new());
        close_all();
    }

    #[test]
    fn subplots_make_a_new_figure() {
        close_all();
        let (fig, axes) = subplots(2, 2).unwrap();
        assert_eq!(gcf(), fig);
        assert_eq!(axes.as_slice().len(), 4);
        assert_eq!(gca(), axes[(1, 1)], "the last axes added is current");
        let (shared, axes) = subplots_shared(2, 1, Share::All, Share::None).unwrap();
        assert_ne!(shared, fig);
        axes[(0, 0)].set_xlim(1.0, 2.0);
        assert_eq!(axes[(1, 0)].xlim(), (1.0, 2.0));
        close_all();
    }

    #[test]
    fn subplot2grid_spans_cells_of_one_grid() {
        close_all();
        let wide = subplot2grid((3, 3), (0, 0), 1, 3).unwrap();
        let tall = subplot2grid((3, 3), (1, 2), 2, 1).unwrap();
        assert_eq!(gca(), tall);
        let (wide_spec, tall_spec) = (wide.subplot_spec().unwrap(), tall.subplot_spec().unwrap());
        assert!(wide_spec.gridspec().is_same(tall_spec.gridspec()));
        assert_eq!((wide_spec.cols(), tall_spec.rows()), (0..3, 1..3));
        assert_eq!(
            subplot2grid((3, 3), (2, 2), 2, 1).unwrap_err(),
            GridSpecError::OutOfRange
        );
        close_all();
    }

    #[test]
    fn free_axes_twins_and_polar_plots() {
        close_all();
        let inset = axes(Some([0.6, 0.6, 0.3, 0.3]));
        assert_eq!((gca(), inset.position().x0()), (inset.clone(), 0.6));
        let twin = twinx(None);
        assert_eq!(gca(), twin);
        assert_eq!(twiny(Some(&inset)).figure(), gcf());
        assert!(matches!(
            polar(vec![0.0, 1.0], vec![1.0, 2.0]),
            Err(PyplotError::NotPolar)
        ));
        figure();
        let line = polar(vec![0.0, 1.0], vec![1.0, 2.0]).unwrap();
        assert_eq!(gca().projection(), ProjectionKind::Polar);
        assert!(gca().lines().contains(&line));
        axes(None);
        assert_eq!(gcf().axes().len(), 2);
        close_all();
    }

    #[test]
    fn mosaics_and_saving() {
        close_all();
        let mosaic = Mosaic::parse("AB;CC").unwrap();
        let (fig, axes) = subplot_mosaic(&mosaic, &MosaicOptions::default()).unwrap();
        assert_eq!((gcf(), axes.len()), (fig, 3));
        let path = std::env::temp_dir().join(format!("plt-pyplot-{}.svg", std::process::id()));
        savefig(&path).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains("<svg"));
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(
            savefig_with("x.unknown", &SaveOptions::default()),
            Err(SaveError::UnsupportedFormat(_))
        ));
        close_all();
    }

    #[test]
    fn removed_axes_handles_stay_comparable() {
        close_all();
        let ax = axes(None);
        delaxes(Some(&ax));
        assert!(!ax.raw().is_alive());
        close_all();
    }
}
