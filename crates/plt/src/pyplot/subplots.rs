//! Making, choosing and removing axes, clearing and saving figures
//! (matplotlib's `subplots`, `subplot`, `subplot2grid`, `subplot_mosaic`,
//! `axes`, `twinx`, `polar`, `cla`, `clf`, `savefig`, ...).

use std::path::Path;

use super::error::PyplotError;
use super::registry::{fignum_exists, figure, figure_num, gca, gcf};
use crate::artist::ArtistHandle;
use crate::axes::{AspectError, Axes, ProjectionKind};
use crate::figure::{
    AxesRectError, Figure, Mosaic, MosaicAxes, MosaicError, MosaicOptions, SaveError, SaveOptions,
    Share,
};
use crate::gridspec::{GridSpecError, SubplotSpec};
use crate::lines::Line2D;
use crate::numeric::Grid;
use crate::projections::{ProjectionError, ProjectionSpec};
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

/// A new current figure with a grid of axes with `projection`, sharing
/// their x and y axes as requested (matplotlib's `subplots(sharex=...,
/// sharey=..., subplot_kw={"projection": ..., **options})`).
///
/// ```
/// use ::plt::axes::ProjectionKind;
/// use ::plt::figure::Share;
/// use plt::pyplot as plt;
/// let (fig, axs) = plt::subplots_with_projection(2, 1, Share::None, Share::None, ProjectionKind::Polar)?;
/// assert_eq!((plt::gcf(), axs[(1, 0)].projection()), (fig, ProjectionKind::Polar));
/// plt::close_all();
/// # Ok::<(), ::plt::projections::ProjectionError>(())
/// ```
///
/// # Errors
///
/// As [`Figure::subplots_with_projection`].
pub fn subplots_with_projection(
    nrows: usize,
    ncols: usize,
    sharex: Share,
    sharey: Share,
    projection: impl Into<ProjectionSpec>,
) -> Result<(Figure, Grid<Axes>), ProjectionError> {
    let fig = figure();
    let axes = fig.subplots_with_projection(nrows, ncols, sharex, sharey, projection)?;
    Ok((fig, axes))
}

/// A new figure numbered `num` with a grid of axes, made current
/// (matplotlib's `subplots(num=num, clear=clear)`). A figure already
/// numbered `num` is cleared and reused when `clear`, and refused
/// otherwise: these functions make figures, they do not add to them.
///
/// ```
/// use plt::pyplot as plt;
/// let fig = plt::figure_num(1);
/// assert!(plt::subplots_num(1, false, 2, 1).is_err());
/// let (again, axes) = plt::subplots_num(1, true, 2, 1)?;
/// assert_eq!((again, axes.as_slice().len()), (fig, 2));
/// plt::close_all();
/// # Ok::<(), plt::PyplotError>(())
/// ```
///
/// # Errors
///
/// [`PyplotError::FigureExists`] when figure `num` is open and not to be
/// cleared, [`PyplotError::Grid`] when the grid has no cells.
pub fn subplots_num(
    num: u32,
    clear: bool,
    nrows: usize,
    ncols: usize,
) -> Result<(Figure, Grid<Axes>), PyplotError> {
    let fig = fresh_figure_num(num, clear)?;
    let axes = fig.subplots(nrows, ncols)?;
    Ok((fig, axes))
}

/// The figure numbered `num` for the functions making figures, made
/// current: a new one, or the existing one cleared when `clear`
/// (matplotlib's `_raise_if_figure_exists` and `figure(num, clear=...)`).
fn fresh_figure_num(num: u32, clear: bool) -> Result<Figure, PyplotError> {
    if fignum_exists(num) && !clear {
        return Err(PyplotError::FigureExists(num));
    }
    let fig = figure_num(num);
    fig.clear();
    Ok(fig)
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
    let ax = match cell_axes(&fig, &spec, None) {
        Some(ax) => ax,
        None => fig.add_subplot_spec(&spec)?,
    };
    fig.sca(&ax);
    Ok(ax)
}

/// The axes with `projection` in cell `index` (1-based, row-major) of an
/// `nrows × ncols` grid of the current figure: one already there if it was
/// made with that projection and the same options, else a new one,
/// leaving any other in place (matplotlib's `subplot(nrows, ncols, index,
/// projection=..., **options)` and `polar=True`); it becomes the current
/// axes.
///
/// ```
/// use ::plt::axes::ProjectionKind;
/// use plt::pyplot as plt;
/// let plain = plt::subplot(1, 2, 1)?;
/// let polar = plt::subplot_with_projection(1, 2, 1, ProjectionKind::Polar)?;
/// assert_ne!(polar, plain);
/// assert_eq!(plt::subplot_with_projection(1, 2, 1, ProjectionKind::Polar)?, polar);
/// assert_eq!(plt::gcf().axes().len(), 2);
/// plt::close_all();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the grid is empty or `index` is outside it, or as
/// [`Figure::add_subplot_spec_with_projection`] for a new axes.
pub fn subplot_with_projection(
    nrows: usize,
    ncols: usize,
    index: usize,
    projection: impl Into<ProjectionSpec>,
) -> Result<Axes, ProjectionError> {
    let projection = projection.into();
    let fig = gcf();
    let spec = fig.numbered_cell(nrows, ncols, index)?;
    let ax = match cell_axes(&fig, &spec, Some(&projection)) {
        Some(ax) => ax,
        None => fig.add_subplot_spec_with_projection(&spec, projection)?,
    };
    fig.sca(&ax);
    Ok(ax)
}

/// The first axes of `fig` in the cell `spec` made with `projection` (any
/// projection for `None`).
fn cell_axes(
    fig: &Figure,
    spec: &SubplotSpec,
    projection: Option<&ProjectionSpec>,
) -> Option<Axes> {
    fig.axes().into_iter().find(|ax| {
        ax.subplot_spec().as_ref() == Some(spec)
            && projection.is_none_or(|projection| ax.made_with() == *projection)
    })
}

/// A new current axes spanning `rowspan × colspan` cells from cell `loc`
/// (row, column) of a `shape` (rows, columns) grid of the current figure
/// (matplotlib's `subplot2grid`). The grid of the figure's other axes is
/// reused when it has that shape, so layout engines see a single grid. A
/// span reaching past the grid stops at its edge, as matplotlib slices
/// the grid.
///
/// ```
/// use plt::pyplot as plt;
/// let ax = plt::subplot2grid((3, 2), (2, 1), 1, 2)?;
/// assert_eq!(ax.subplot_spec().unwrap().cols(), 1..2);
/// plt::close_all();
/// # Ok::<(), ::plt::gridspec::GridSpecError>(())
/// ```
///
/// # Errors
///
/// When the grid is empty or the block starts outside it.
pub fn subplot2grid(
    shape: (usize, usize),
    loc: (usize, usize),
    rowspan: usize,
    colspan: usize,
) -> Result<Axes, GridSpecError> {
    let fig = gcf();
    let grid = fig.figure_grid(shape.0, shape.1)?;
    let (row, col) = loc;
    let spec = grid.span(
        row..(row + rowspan).min(shape.0),
        col..(col + colspan).min(shape.1),
    )?;
    fig.add_subplot_spec(&spec)
}

/// A new current figure with axes laid out by `mosaic`, given as for
/// [`Figure::subplot_mosaic`] (matplotlib's `subplot_mosaic`).
///
/// # Errors
///
/// As [`Figure::subplot_mosaic`]; text that does not parse makes no
/// figure.
pub fn subplot_mosaic(
    mosaic: impl TryInto<Mosaic, Error: Into<MosaicError>>,
    options: MosaicOptions,
) -> Result<(Figure, MosaicAxes), MosaicError> {
    let mosaic: Mosaic = mosaic.try_into().map_err(Into::into)?;
    let fig = figure();
    let axes = fig.subplot_mosaic(mosaic, options)?;
    Ok((fig, axes))
}

/// A new figure numbered `num` with axes laid out by `mosaic`, made
/// current (matplotlib's `subplot_mosaic(..., num=num, clear=clear)`); an
/// open figure `num` is cleared and reused when `clear`, and refused
/// otherwise, as by [`subplots_num`].
///
/// # Errors
///
/// [`PyplotError::FigureExists`] when figure `num` is open and not to be
/// cleared, [`PyplotError::Mosaic`] as [`Figure::subplot_mosaic`].
pub fn subplot_mosaic_num(
    num: u32,
    clear: bool,
    mosaic: impl TryInto<Mosaic, Error: Into<MosaicError>>,
    options: MosaicOptions,
) -> Result<(Figure, MosaicAxes), PyplotError> {
    let mosaic: Mosaic = mosaic.try_into().map_err(Into::<MosaicError>::into)?;
    let fig = fresh_figure_num(num, clear)?;
    let axes = fig.subplot_mosaic(mosaic, options)?;
    Ok((fig, axes))
}

/// A new current axes in the current figure: at `rect`
/// (`[left, bottom, width, height]` in figure fractions), or filling the
/// default subplot area when `None` (matplotlib's `axes`).
///
/// # Errors
///
/// As [`Figure::add_axes`] for `rect`.
pub fn axes(rect: Option<[f64; 4]>) -> Result<Axes, AxesRectError> {
    let fig = gcf();
    match rect {
        Some(rect) => fig.add_axes(rect),
        None => Ok(fig.subplot()),
    }
}

/// A new current axes with `projection` in the current figure, placed as
/// by [`axes`] (matplotlib's `axes(rect, projection=...)`).
///
/// ```
/// use ::plt::axes::ProjectionKind;
/// use plt::pyplot as plt;
/// let polar = plt::axes_with_projection(None, ProjectionKind::Polar)?;
/// let plain = plt::axes(None)?;
/// assert_eq!((polar.projection(), plain.projection()), (ProjectionKind::Polar, ProjectionKind::Rectilinear));
/// assert_eq!(plt::gca(), plain);
/// plt::close_all();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// As [`Figure::add_axes_with_projection`] for `rect`, or
/// [`Figure::subplot_with_projection`].
pub fn axes_with_projection(
    rect: Option<[f64; 4]>,
    projection: impl Into<ProjectionSpec>,
) -> Result<Axes, ProjectionError> {
    let fig = gcf();
    match rect {
        Some(rect) => fig.add_axes_with_projection(rect, projection),
        None => fig.subplot_with_projection(projection),
    }
}

/// A new current axes sharing the x axis of `ax`, or of the current axes,
/// with its y axis on the right (matplotlib's `twinx`). It takes the axes
/// to twin, as matplotlib's pyplot `twinx(ax=None)` does, where
/// [`Axes::twinx`] is a method of it.
///
/// # Errors
///
/// As [`Axes::twinx`]: when the axes fixes its data ratio (polar and
/// geographic axes).
pub fn twinx(ax: Option<&Axes>) -> Result<Axes, AspectError> {
    ax.cloned().unwrap_or_else(gca).twinx()
}

/// A new current axes sharing the y axis of `ax`, or of the current axes,
/// with its x axis on top (matplotlib's `twiny`). It takes the axes to
/// twin, as matplotlib's pyplot `twiny(ax=None)` does, where
/// [`Axes::twiny`] is a method of it.
///
/// # Errors
///
/// As [`Axes::twiny`].
pub fn twiny(ax: Option<&Axes>) -> Result<Axes, AspectError> {
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
/// (matplotlib's `delaxes`). It takes the axes, as matplotlib's pyplot
/// `delaxes(ax=None)` does; an axes removes itself with
/// [`ArtistHandle::remove`](crate::artist::ArtistHandle::remove).
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
pub fn savefig_with(path: impl AsRef<Path>, options: SaveOptions) -> Result<(), SaveError> {
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
        axes[(0, 0)].set_xlim(1.0, 2.0).expect("finite limits");
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
        let clipped = subplot2grid((3, 3), (2, 2), 2, 1).unwrap();
        assert_eq!(
            clipped.subplot_spec().unwrap().rows(),
            2..3,
            "spans stop at the grid's edge, as matplotlib slices it"
        );
        assert_eq!(
            subplot2grid((3, 3), (3, 0), 1, 1).unwrap_err(),
            GridSpecError::OutOfRange
        );
        close_all();
    }

    #[test]
    fn free_axes_twins_and_polar_plots() {
        close_all();
        let inset = axes(Some([0.6, 0.6, 0.3, 0.3])).expect("a valid rectangle");
        assert!(matches!(
            axes(Some([0.0, 0.0, -1.0, 1.0])),
            Err(AxesRectError::NegativeSize(_))
        ));
        assert_eq!((gca(), inset.position().x0()), (inset.clone(), 0.6));
        let twin = twinx(None).expect("rectilinear axes have twins");
        assert_eq!(gca(), twin);
        assert_eq!(twiny(Some(&inset)).map(|twin| twin.figure()), Ok(gcf()));
        assert!(matches!(
            polar(vec![0.0, 1.0], vec![1.0, 2.0]),
            Err(PyplotError::NotPolar)
        ));
        figure();
        let line = polar(vec![0.0, 1.0], vec![1.0, 2.0]).unwrap();
        assert_eq!(gca().projection(), ProjectionKind::Polar);
        assert!(gca().lines().contains(&line));
        axes(None).expect("the default subplot area");
        assert_eq!(gcf().axes().len(), 2);
        close_all();
    }

    #[test]
    fn mosaics_and_saving() {
        close_all();
        let mosaic = "AB;CC".parse::<Mosaic>().unwrap();
        let (fig, axes) = subplot_mosaic(&mosaic, MosaicOptions::default()).unwrap();
        assert_eq!((gcf(), axes.len()), (fig, 3));
        let path = std::env::temp_dir().join(format!("plt-pyplot-{}.svg", std::process::id()));
        savefig(&path).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains("<svg"));
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(
            savefig_with("x.unknown", SaveOptions::default()),
            Err(SaveError::UnsupportedFormat(_))
        ));
        close_all();
    }

    #[test]
    fn numbered_figures_are_made_or_cleared_never_added_to() {
        close_all();
        let mosaic = "AB".parse::<Mosaic>().unwrap();
        let options = MosaicOptions::default();
        let (made, axes) = subplot_mosaic_num(2, false, &mosaic, options.clone()).unwrap();
        assert_eq!((crate::pyplot::get_fignums(), axes.len()), (vec![2], 2));
        assert_eq!(
            subplot_mosaic_num(2, false, &mosaic, options.clone()).err(),
            Some(PyplotError::FigureExists(2))
        );
        let (again, _) = subplot_mosaic_num(2, true, &mosaic, options).unwrap();
        assert_eq!(again, made);
        assert_eq!(made.axes().len(), 2, "the old axes were cleared away");
        assert_eq!(
            subplots_num(3, false, 0, 1).err(),
            Some(PyplotError::Grid(GridSpecError::EmptyGrid))
        );
        close_all();
    }

    #[test]
    fn subplots_of_a_projection_replace_none_of_another() {
        close_all();
        let polar = subplot_with_projection(1, 1, 1, ProjectionKind::Polar).unwrap();
        assert_eq!(subplot(1, 1, 1).unwrap(), polar, "any projection is reused");
        let plain = subplot_with_projection(1, 1, 1, ProjectionKind::Rectilinear).unwrap();
        assert_ne!(plain, polar);
        let map = axes_with_projection(Some([0.1, 0.1, 0.3, 0.3]), ProjectionKind::Mollweide)
            .expect("a valid rectangle");
        assert_eq!(map.projection(), ProjectionKind::Mollweide);
        assert_eq!(gcf().axes().len(), 3);
        close_all();
    }

    #[test]
    fn removed_axes_handles_stay_comparable() {
        close_all();
        let ax = axes(None).expect("the default subplot area");
        delaxes(Some(&ax));
        assert!(!ax.raw().is_attached());
        close_all();
    }
}
