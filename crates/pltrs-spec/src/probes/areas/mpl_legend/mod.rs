//! Probe operations for the `mpl_legend` area: legends, ported from
//! matplotlib's `tests/test_legend.py` (see
//! `spec/reference/probes/test_mpl_legend.py`, which documents every op).

mod colors;
mod entries;
mod frame;
mod placement;
mod plots;
mod proxies;

use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Op, ProbeError};
use pltrs::artist::ArtistHandle;
use pltrs::axes::Axes;
use pltrs::legend::{Legend, LegendOptions};
use pltrs_core::{Bbox, Rgba};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Axes.legend", entries::ordered_handles),
    Op::new(
        "legend._get_legend_handles_labels",
        entries::repeated_labels,
    ),
    Op::new("Axes.get_legend_handles_labels", entries::unlisted_artists),
    Op::new("pyplot.legend", entries::parasite_legend),
    Op::new("SubFigure.legend", entries::subfigure_legend),
    Op::new("Legend.get_draggable", entries::draggable),
    Op::new("Legend.get_window_extent", placement::window_extent),
    Op::new("Figure.legend", placement::outside),
    Op::new("Legend.set_loc", placement::set_loc),
    Op::new("Legend.get_alignment", placement::alignment),
    Op::new("Legend.get_frame", frame::frame),
    Op::new("Legend.get_title", frame::title),
    Op::new("Legend.legend_handles", proxies::legend_handles),
    Op::new("Legend.get_lines", proxies::lines),
    Op::new("Legend.get_patches", proxies::patches),
    Op::new("Legend.get_texts", colors::label_colors),
    Op::new("Legend.texts", colors::last_entry_colors),
    Op::new("Axes.plot", plots::plot),
    Op::new("Axes.boxplot", plots::boxplot),
];

/// The legend of `ax` made with `options`.
fn legend(ax: &Axes, options: LegendOptions) -> Result<Legend, ProbeError> {
    ax.legend(options).map_err(ProbeError::rejected)
}

/// `[x0, y0, x1, y1]`.
fn extents(bbox: Bbox) -> Json {
    numbers([bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()])
}

/// The extents of an artist in the figure's current layout.
fn window_extents(artist: &impl ArtistHandle) -> Result<Json, ProbeError> {
    artist
        .window_extent()
        .map(extents)
        .ok_or_else(|| ProbeError::rejected("the artist has no extent"))
}

/// The labels of a legend.
fn texts(legend: &Legend) -> Json {
    Json::from(legend.labels())
}

/// `[r, g, b, a]`, or `null` for a colour that is fully transparent black
/// (matplotlib's `"none"`).
fn maybe_rgba(color: Option<Rgba>) -> Json {
    match color {
        Some(c) if c != Rgba::TRANSPARENT => numbers(c.to_array()),
        _ => Json::Null,
    }
}
