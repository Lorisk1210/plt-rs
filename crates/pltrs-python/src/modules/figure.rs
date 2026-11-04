//! `_native.figure`: figures, subfigures, their subplots and layout
//! engines (matplotlib's `figure`); `_native.backends` prints them.

use pltrs::artist::{DrawErrorKind, RawHandle};
use pltrs::axes::{Axes, ProjectionKind};
use pltrs::colors::Color;
use pltrs::figure::{AxesRectError, DrawErrors, Figure, SaveError, SizeError, SubfigureOptions};
use pltrs::gridspec::{GridSpecError, SubplotOverrides, SubplotSpec};
use pltrs::layout::{
    ConstrainedLayout, LayoutEngine, LayoutEngineError, PlaceHolderLayout, SolvedGrid, TightLayout,
};
use pltrs::text::Text;
use pltrs_render::RasterBackendError;

use super::gridspec::Overrides;
use crate::binding::prelude::*;

artist_handle!(Figure);

impl From<GridSpecError> for PyErr {
    fn from(err: GridSpecError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<AxesRectError> for PyErr {
    fn from(err: AxesRectError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<LayoutEngineError> for PyErr {
    /// matplotlib's `RuntimeError` when a colorbar pins the engine.
    fn from(err: LayoutEngineError) -> Self {
        match err {
            LayoutEngineError::IncompatibleColorbars => PyErr::runtime_error(
                "Colorbar layout of new layout engine not compatible with old engine, and a \
                 colorbar has been created.  Engine not changed.",
            ),
        }
    }
}

impl From<SizeError> for PyErr {
    fn from(err: SizeError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<DrawErrors> for PyErr {
    /// The first problem as matplotlib raises it, drawing stopping there:
    /// a `ValueError` with its message for a value of the program's that
    /// cannot be drawn (malformed mathtext, ...), a `TypeError` for one of
    /// the wrong type, an `OverflowError` for an infinity rounded, a
    /// `NotImplementedError` for what matplotlib does not support yet, an
    /// `IndexError` for an index out of range, else a `RuntimeError`
    /// naming every problem.
    fn from(errors: DrawErrors) -> Self {
        match errors.0.first() {
            Some(first) if first.kind == DrawErrorKind::InvalidValue => {
                PyErr::value_error(first.message.clone())
            }
            Some(first) if first.kind == DrawErrorKind::InvalidType => {
                PyErr::new(ExceptionKind::TypeError, first.message.clone())
            }
            Some(first) if first.kind == DrawErrorKind::Overflow => {
                PyErr::new(ExceptionKind::OverflowError, first.message.clone())
            }
            Some(first) if first.kind == DrawErrorKind::Unsupported => {
                PyErr::new(ExceptionKind::NotImplementedError, first.message.clone())
            }
            Some(first) if first.kind == DrawErrorKind::OutOfRange => {
                PyErr::new(ExceptionKind::IndexError, first.message.clone())
            }
            _ => PyErr::runtime_error(errors.to_string()),
        }
    }
}

impl From<SaveError> for PyErr {
    /// matplotlib's `ValueError` for unknown formats and canvases too large
    /// to make (Agg's limit), `OSError` for files that cannot be written,
    /// and the problems drawing met as [`DrawErrors`] raise them.
    fn from(err: SaveError) -> Self {
        let kind = match err {
            SaveError::Draw(errors) => return errors.into(),
            SaveError::Io(_) => ExceptionKind::OSError,
            SaveError::UnsupportedFormat(_)
            | SaveError::MetadataUnsupported(_)
            | SaveError::Backend(RasterBackendError::Canvas(_)) => ExceptionKind::ValueError,
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

/// The current axes, `None` when the figure has none.
fn current_axes(figure: Figure) -> PyResult<Option<Axes>> {
    Ok(figure.current_axes())
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

/// Sets the figure's centred x label.
fn supxlabel(figure: Figure, text: String) -> PyResult<Text> {
    Ok(figure.supxlabel(text))
}

/// Sets the figure's centred y label.
fn supylabel(figure: Figure, text: String) -> PyResult<Text> {
    Ok(figure.supylabel(text))
}

/// The strings of the figure's centred title, x label and y label, empty
/// where unset (`get_suptitle`, `get_supxlabel`, `get_supylabel`).
fn super_texts(figure: Figure) -> PyResult<(String, String, String)> {
    Ok((
        figure.suptitle_text(),
        figure.supxlabel_text(),
        figure.supylabel_text(),
    ))
}

/// The texts added to the figure itself (`fig.texts`).
fn texts(figure: Figure) -> PyResult<Vec<Text>> {
    Ok(figure.texts())
}

/// The artists added to the figure itself that no other list holds
/// (`fig.artists`).
fn artists(figure: Figure) -> PyResult<Vec<RawHandle>> {
    Ok(figure.artists())
}

/// The built-in projection named `name` (`"polar"`, `"hammer"`...).
pub(super) fn projection_kind(name: &str) -> PyResult<ProjectionKind> {
    ProjectionKind::BUILTIN
        .into_iter()
        .find(|kind| kind.name() == name)
        .ok_or_else(|| PyErr::value_error(format!("Unknown projection {name:?}")))
}

/// The topmost visible axes containing the display point `(x, y)`
/// (matplotlib's `canvas.inaxes`).
fn axes_at(figure: Figure, x: f64, y: f64) -> PyResult<Option<Axes>> {
    Ok(pltrs::navigation::axes_at(
        &figure,
        pltrs::transforms::Point::new(x, y),
    ))
}

/// The Python description of a constrained layout engine: `("constrained",
/// (h_pad, w_pad, hspace, wspace), rect, compress)`.
type ConstrainedSpec = (String, (f64, f64, f64, f64), (f64, f64, f64, f64), bool);

/// The Python description of a tight layout engine: `("tight", pad,
/// h_pad, w_pad, rect)`.
type TightSpec = (String, f64, Option<f64>, Option<f64>, (f64, f64, f64, f64));

/// A layout engine as Python describes it: `("constrained", (h_pad,
/// w_pad, hspace, wspace), rect, compress)`, `("tight", pad, h_pad,
/// w_pad, rect)` with `rect` `(left, bottom, right, top)`, or
/// `("placeholder", adjust_compatible, colorbar_gridspec)`.
struct EngineSpec(LayoutEngine);

impl FromPy<'_> for EngineSpec {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let kind: String = object
            .try_iter()?
            .next()
            .ok_or_else(|| PyErr::value_error("a layout engine needs a kind"))??
            .extract()?;
        Ok(Self(match kind.as_str() {
            "constrained" => {
                let (_, (h_pad, w_pad, hspace, wspace), rect, compress): ConstrainedSpec =
                    object.extract()?;
                LayoutEngine::Constrained(ConstrainedLayout {
                    w_pad,
                    h_pad,
                    wspace,
                    hspace,
                    rect: [rect.0, rect.1, rect.2, rect.3],
                    compress,
                })
            }
            "tight" => {
                let (_, pad, h_pad, w_pad, rect): TightSpec = object.extract()?;
                LayoutEngine::Tight(TightLayout {
                    pad,
                    h_pad,
                    w_pad,
                    rect: Some([rect.0, rect.1, rect.2, rect.3].map(Some)),
                })
            }
            "placeholder" => {
                let (_, adjust_compatible, colorbar_gridspec): (String, bool, bool) =
                    object.extract()?;
                LayoutEngine::PlaceHolder(PlaceHolderLayout {
                    adjust_compatible,
                    colorbar_gridspec,
                })
            }
            _ => return Err(PyErr::value_error(format!("no layout engine {kind:?}"))),
        }))
    }
}

/// Gives the figure the layout engine `engine` (`None`: none, or a
/// placeholder for the engine it had; `set_layout_engine`).
fn set_layout_engine(figure: Figure, engine: Option<EngineSpec>) -> PyResult<()> {
    figure.set_layout_engine(engine.map(|spec| spec.0))?;
    Ok(())
}

/// Lays the figure out once with `engine` (`LayoutEngine.execute`),
/// returning the grids constrained layout solved, if any (see
/// `layoutgrid`).
fn execute_layout(figure: Figure, engine: EngineSpec) -> PyResult<Option<Vec<SolvedGrid>>> {
    Ok(figure.execute_layout(engine.0))
}

/// Axes at `[left, bottom, width, height]` in figure fractions
/// (`add_axes(rect)`).
fn add_axes(figure: Figure, (left, bottom, width, height): (f64, f64, f64, f64)) -> PyResult<Axes> {
    Ok(figure.add_axes([left, bottom, width, height])?)
}

/// An axes of `figure` at `[left, bottom, width, height]` in its
/// fractions, made outside it until `add_axes(ax)` adds it (matplotlib's
/// `Axes(fig, rect)`).
fn removed_axes(
    figure: Figure,
    (left, bottom, width, height): (f64, f64, f64, f64),
) -> PyResult<Axes> {
    Ok(figure.removed_axes([left, bottom, width, height])?)
}

/// An axes of `figure` on the block `spec`, made outside it until
/// `add_subplot(ax)` adds it (matplotlib's `Axes(fig, subplotspec)`).
fn removed_subplot(figure: Figure, spec: SubplotSpec) -> PyResult<Axes> {
    Ok(figure.removed_subplot(&spec)?)
}

/// Axes with the projection `projection` at `[left, bottom, width,
/// height]` in figure fractions, made outside the figure until
/// `add_axes(ax)` adds them (`PolarAxes(fig, rect)`).
fn removed_axes_projection(
    figure: Figure,
    (left, bottom, width, height): (f64, f64, f64, f64),
    projection: String,
) -> PyResult<Axes> {
    let kind = projection_kind(&projection)?;
    figure
        .removed_axes_with_projection([left, bottom, width, height], kind)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// Axes with the projection `projection` on the block `spec`, made outside
/// the figure until `add_subplot(ax)` adds them (`PolarAxes(fig,
/// subplotspec)`).
fn removed_subplot_projection(
    figure: Figure,
    spec: SubplotSpec,
    projection: String,
) -> PyResult<Axes> {
    let kind = projection_kind(&projection)?;
    figure
        .removed_subplot_with_projection(&spec, kind)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// Axes with the projection `projection` at `[left, bottom, width,
/// height]` in figure fractions (`add_axes(rect, projection=...)`).
fn add_axes_projection(
    figure: Figure,
    (left, bottom, width, height): (f64, f64, f64, f64),
    projection: String,
) -> PyResult<Axes> {
    let kind = projection_kind(&projection)?;
    figure
        .add_axes_with_projection([left, bottom, width, height], kind)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// Axes with the projection `projection` on the block `spec`
/// (`add_subplot(gs[...], projection=...)`).
fn add_subplot_spec_projection(
    figure: Figure,
    spec: SubplotSpec,
    projection: String,
) -> PyResult<Axes> {
    let kind = projection_kind(&projection)?;
    figure
        .add_subplot_spec_with_projection(&spec, kind)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// Changes some of the figure's subplot parameters (`subplots_adjust`).
fn subplots_adjust(
    figure: Figure,
    (left, bottom, right, top, wspace, hspace): Overrides,
) -> PyResult<()> {
    figure.subplots_adjust(SubplotOverrides {
        left,
        bottom,
        right,
        top,
        wspace,
        hspace,
    })?;
    Ok(())
}

/// A grid of subfigures filling `figure`, row by row (`subfigures`):
/// `spacing` is `(wspace, hspace)`.
fn subfigures(
    figure: Figure,
    (nrows, ncols): (usize, usize),
    (wspace, hspace): (Option<f64>, Option<f64>),
    width_ratios: Option<Vec<f64>>,
    height_ratios: Option<Vec<f64>>,
) -> PyResult<Vec<Figure>> {
    let options = SubfigureOptions {
        wspace,
        hspace,
        width_ratios,
        height_ratios,
    };
    Ok(figure.subfigures(nrows, ncols, options)?.into_vec())
}

/// A subfigure of `figure` covering the block `spec` (`add_subfigure`).
fn add_subfigure(figure: Figure, spec: SubplotSpec) -> PyResult<Figure> {
    Ok(figure.add_subfigure(&spec))
}

/// Removes everything from the figure, its subfigures included (`clear`).
fn clear(figure: Figure) -> PyResult<()> {
    figure.clear();
    Ok(())
}

/// Draws the figure without output (`draw_without_rendering`).
fn draw_without_rendering(figure: Figure) -> PyResult<()> {
    Ok(figure.draw_without_rendering()?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("set_layout_engine", set_layout_engine)?;
    module.add("execute_layout", execute_layout)?;
    module.add("add_axes", add_axes)?;
    module.add("removed_axes", removed_axes)?;
    module.add("removed_subplot", removed_subplot)?;
    module.add("removed_axes_projection", removed_axes_projection)?;
    module.add("removed_subplot_projection", removed_subplot_projection)?;
    module.add("add_subplot_spec_projection", add_subplot_spec_projection)?;
    module.add("add_axes_projection", add_axes_projection)?;
    module.add("subplots_adjust", subplots_adjust)?;
    module.add("draw_without_rendering", draw_without_rendering)?;
    module.add("clear", clear)?;
    module.add("subfigures", subfigures)?;
    module.add("add_subfigure", add_subfigure)?;
    module.add("axes_at", axes_at)?;
    module.add("get_size_inches", get_size_inches)?;
    module.add("set_size_inches", set_size_inches)?;
    module.add("get_dpi", get_dpi)?;
    module.add("set_dpi", set_dpi)?;
    module.add("get_facecolor", get_facecolor)?;
    module.add("set_facecolor", set_facecolor)?;
    module.add("set_edgecolor", set_edgecolor)?;
    module.add("set_linewidth", set_linewidth)?;
    module.add("set_frameon", set_frameon)?;
    module.add("current_axes", current_axes)?;
    module.add("sca", sca)?;
    module.add("axes", axes)?;
    module.add("suptitle", suptitle)?;
    module.add("supxlabel", supxlabel)?;
    module.add("supylabel", supylabel)?;
    module.add("super_texts", super_texts)?;
    module.add("texts", texts)?;
    module.add("artists", artists)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_background_reads_back_as_set() {
        let fig = Figure::new();
        set_edgecolor(fig.clone(), Color::from("red")).unwrap();
        set_linewidth(fig.clone(), 2.5).unwrap();
        set_frameon(fig.clone(), false).unwrap();
        assert_eq!(fig.edgecolor(), Color::from("red"));
        assert_eq!(fig.linewidth(), 2.5);
        assert!(!fig.frameon());
    }
}
