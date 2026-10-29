//! `_native.figure`: figures, their subplots and saving them
//! (matplotlib's `figure` and the `print_figure` of its backends).

use plt::axes::{Axes, ProjectionKind};
use plt::colors::Color;
use plt::figure::{Figure, SaveError, SaveFormat, SaveOptions, SizeError};
use plt::gridspec::GridSpecError;
use plt::text::Text;

use crate::binding::prelude::*;

artist_handle!(Figure);

impl From<GridSpecError> for PyErr {
    fn from(err: GridSpecError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<SizeError> for PyErr {
    fn from(err: SizeError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<SaveError> for PyErr {
    /// matplotlib's `ValueError` for unknown formats, `OSError` for files
    /// that cannot be written.
    fn from(err: SaveError) -> Self {
        let kind = match err {
            SaveError::Io(_) => ExceptionKind::OSError,
            SaveError::UnsupportedFormat(_) | SaveError::MetadataUnsupported(_) => {
                ExceptionKind::ValueError
            }
            _ => ExceptionKind::RuntimeError,
        };
        PyErr::new(kind, err.to_string())
    }
}

/// A figure of `width × height` inches at `dpi`.
fn new(width: f64, height: f64, dpi: f64) -> PyResult<Figure> {
    Ok(Figure::with_size(width, height, dpi))
}

/// The figure's size in inches.
fn get_size_inches(figure: Figure) -> PyResult<(f64, f64)> {
    Ok(figure.size_inches())
}

/// Resizes the figure.
fn set_size_inches(figure: Figure, width: f64, height: f64) -> PyResult<()> {
    figure.set_size_inches(width, height)?;
    Ok(())
}

/// The figure's resolution.
fn get_dpi(figure: Figure) -> PyResult<f64> {
    Ok(figure.dpi())
}

/// Sets the figure's resolution.
fn set_dpi(figure: Figure, dpi: f64) -> PyResult<()> {
    figure.set_dpi(dpi);
    Ok(())
}

/// Sets the background colour.
fn set_facecolor(figure: Figure, color: Color) -> PyResult<()> {
    figure.set_facecolor(color);
    Ok(())
}

/// The background colour.
fn get_facecolor(figure: Figure) -> PyResult<Color> {
    Ok(figure.facecolor())
}

/// Sets the frame's colour.
fn set_edgecolor(figure: Figure, color: Color) -> PyResult<()> {
    figure.set_edgecolor(color);
    Ok(())
}

/// Sets the frame's width.
fn set_linewidth(figure: Figure, width: f64) -> PyResult<()> {
    figure.set_linewidth(width);
    Ok(())
}

/// Shows or hides the figure's background patch.
fn set_frameon(figure: Figure, frameon: bool) -> PyResult<()> {
    figure.set_frameon(frameon);
    Ok(())
}

/// `add_subplot(nrows, ncols, index)`.
fn add_subplot(figure: Figure, nrows: usize, ncols: usize, index: usize) -> PyResult<Axes> {
    Ok(figure.add_subplot(nrows, ncols, index)?)
}

/// The axes of an `nrows × ncols` grid, row by row.
fn subplots(figure: Figure, nrows: usize, ncols: usize) -> PyResult<Vec<Axes>> {
    Ok(figure.subplots(nrows, ncols)?.into_vec())
}

/// The current axes, made if there are none.
fn gca(figure: Figure) -> PyResult<Axes> {
    Ok(figure.gca())
}

/// Makes `axes` the current axes.
fn sca(figure: Figure, axes: Axes) -> PyResult<()> {
    figure.sca(&axes);
    Ok(())
}

/// The figure's axes, in the order they were added.
fn axes(figure: Figure) -> PyResult<Vec<Axes>> {
    Ok(figure.axes())
}

/// Sets the figure's centred title.
fn suptitle(figure: Figure, text: String) -> PyResult<Text> {
    Ok(figure.suptitle(text))
}

/// The figure as a file of `format` (`print_figure`): the bytes the
/// Python side writes to a path or a file object.
fn print_figure(
    figure: Figure,
    format: String,
    dpi: Option<f64>,
    facecolor: Option<Color>,
    edgecolor: Option<Color>,
    transparent: Option<bool>,
) -> PyResult<Bytes> {
    let format: SaveFormat = format.parse()?;
    let options = SaveOptions {
        dpi,
        facecolor,
        edgecolor,
        transparent,
        format: Some(format),
        ..SaveOptions::default()
    };
    Ok(Bytes(figure.to_format(format, options)?))
}

/// The built-in projection named `name` (`"polar"`, `"hammer"`...).
fn projection_kind(name: &str) -> PyResult<ProjectionKind> {
    ProjectionKind::BUILTIN
        .into_iter()
        .find(|kind| kind.name() == name)
        .ok_or_else(|| PyErr::value_error(format!("Unknown projection {name:?}")))
}

/// Axes with the projection `projection` in cell `index` (1-based) of an
/// `nrows × ncols` grid (`add_subplot(..., projection=...)`).
fn add_subplot_projection(
    figure: Figure,
    (nrows, ncols, index): (usize, usize, usize),
    projection: String,
) -> PyResult<Axes> {
    let kind = projection_kind(&projection)?;
    figure
        .add_subplot_with_projection(nrows, ncols, index, kind)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// The topmost visible axes containing the display point `(x, y)`
/// (matplotlib's `canvas.inaxes`).
fn axes_at(figure: Figure, x: f64, y: f64) -> PyResult<Option<Axes>> {
    Ok(plt::navigation::axes_at(
        &figure,
        plt::transforms::Point::new(x, y),
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("axes_at", axes_at)?;
    module.add("add_subplot_projection", add_subplot_projection)?;
    module.add("get_size_inches", get_size_inches)?;
    module.add("set_size_inches", set_size_inches)?;
    module.add("get_dpi", get_dpi)?;
    module.add("set_dpi", set_dpi)?;
    module.add("get_facecolor", get_facecolor)?;
    module.add("set_facecolor", set_facecolor)?;
    module.add("set_edgecolor", set_edgecolor)?;
    module.add("set_linewidth", set_linewidth)?;
    module.add("set_frameon", set_frameon)?;
    module.add("add_subplot", add_subplot)?;
    module.add("subplots", subplots)?;
    module.add("gca", gca)?;
    module.add("sca", sca)?;
    module.add("axes", axes)?;
    module.add("suptitle", suptitle)?;
    module.add("print_figure", print_figure)
}
