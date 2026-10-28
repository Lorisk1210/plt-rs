//! Probe operations for the `mpl_colorbar` area: colorbars, ported from
//! matplotlib's `tests/test_colorbar.py` (see
//! `spec/reference/probes/test_mpl_colorbar.py`, which documents every op).

mod drawn;
mod mapping;
mod placement;
mod ticks;

use crate::json::Json;
use crate::probes::encode::{numbers, rgba};
use crate::probes::{Args, Op, ProbeError};
use plt::axes::{Axes, ImshowOptions};
use plt::colors::Extend;
use plt::image::AxesImage;
use plt::numeric::{Grid, linspace};
use plt::orientation::Orientation;
use plt::spines::SpineSide;
use plt_core::Bbox;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Colorbar._extend_patches", drawn::extend_patches),
    Op::new("Colorbar.outline", drawn::outline),
    Op::new("Colorbar.dividers", drawn::dividers),
    Op::new("Colorbar", drawn::without_mappable),
    Op::new("Axes.get_anchor", placement::parent_anchor),
    Op::new("Axes.get_position", placement::positions),
    Op::new("Colorbar.remove", placement::remove),
    Op::new("Axes.remove", placement::remove_axes),
    Op::new("YAxis.offsetText", placement::offset_text),
    Op::new("Text.get_window_extent", placement::title),
    Op::new("Colorbar.vmin", mapping::renorm),
    Op::new("ScalarMappable.norm", mapping::assigned_norm),
    Op::new("Colorbar.alpha", mapping::alpha),
    Op::new("Figure.colorbar", mapping::make),
    Op::new("pyplot.colorbar", mapping::parentless),
    Op::new("Axis.get_minorticklocs", ticks::minor_ticks),
    Op::new("Colorbar.set_label", ticks::labels),
    Op::new("Axes.get_yticks", ticks::boundary_norms),
    Op::new("Colorbar.locator", ticks::tickers),
];

/// `[x0, y0, x1, y1]`.
fn extents(bbox: Bbox) -> Json {
    let (x0, y0, x1, y1) = bbox.extents();
    numbers([x0, y0, x1, y1])
}

/// An image of `rows` in `ax`, in the default colours.
fn image(ax: &Axes, rows: Vec<Vec<f64>>) -> Result<AxesImage, ProbeError> {
    let data = Grid::from_rows(rows).map_err(ProbeError::rejected)?;
    ax.imshow(data, ImshowOptions::default())
        .map_err(ProbeError::rejected)
}

/// The `orientation` argument.
fn orientation(args: Args<'_>) -> Result<Orientation, ProbeError> {
    parse(args, "orientation")
}

/// The `extend` argument.
fn extend(args: Args<'_>) -> Result<Extend, ProbeError> {
    parse(args, "extend")
}

/// The string argument `key`, parsed.
fn parse<T: std::str::FromStr>(args: Args<'_>, key: &str) -> Result<T, ProbeError> {
    let text = args.str(key)?;
    text.parse()
        .map_err(|_| ProbeError::BadArgs(format!("bad {key} {text:?}")))
}

/// A side of the axes by matplotlib's name.
fn side(name: &str) -> Result<SpineSide, ProbeError> {
    SpineSide::from_name(name).ok_or_else(|| ProbeError::BadArgs(format!("bad side {name:?}")))
}

/// `120000 * exp(-x² - y²)` on `np.ogrid[-4:4:31j, -4:4:31j]`.
fn bump() -> Grid<f64> {
    let at = linspace(-4.0, 4.0, 31);
    Grid::from_fn(31, 31, |r, c| {
        120_000.0 * (-at[r].powi(2) - at[c].powi(2)).exp()
    })
}
