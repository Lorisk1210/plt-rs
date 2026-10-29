//! `figures/cases_offsetbox.py`: anchored and annotation boxes, the
//! `axes_grid1` anchored artists, `Axes.arrow`, `Axes.ecdf`,
//! `pyplot.figtext` and `pyplot.xkcd`.

mod anchored;
mod annotation;
mod gaps;
mod layouts;
mod pointers;

use crate::figures::cases::common::{sized, xkcd};
use pltrs::numeric::Grid;
use pltrs::offsetbox::{BoxItem, OffsetBox, TextArea};
use pltrs::prelude::*;
use pltrs::text::TextProps;

use crate::figures::FigureCase;
use pltrs::axes::PlotError;

/// Every case of the group.
pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("offsetbox/anchored_text", anchored::anchored_text),
    FigureCase::new("offsetbox/anchored_packers", anchored::anchored_packers),
    FigureCase::new(
        "offsetbox/padded_and_aux_boxes",
        anchored::padded_and_aux_boxes,
    ),
    FigureCase::new("offsetbox/anchored_size_bar", anchored::anchored_size_bar),
    FigureCase::new(
        "offsetbox/anchored_direction_arrows",
        anchored::anchored_direction_arrows,
    ),
    FigureCase::new(
        "offsetbox/anchored_drawing_area_and_aux",
        anchored::anchored_drawing_area_and_aux,
    ),
    FigureCase::new("offsetbox/annotation_bbox_text", annotation::text),
    FigureCase::new("offsetbox/annotation_bbox_image", annotation::image),
    FigureCase::new(
        "offsetbox/annotation_bbox_drawing_area",
        annotation::drawing_area,
    ),
    FigureCase::new("offsetbox/axes_arrow", gaps::axes_arrow),
    FigureCase::new("offsetbox/ecdf", gaps::ecdf).sized(9.0, 3.5, 100.0),
    FigureCase::new("offsetbox/figtext", gaps::figtext),
    FigureCase::new("offsetbox/xkcd_bacon", gaps::xkcd_bacon).with_rc_setup(xkcd),
    FigureCase::new("offsetbox/xkcd_ticks", gaps::xkcd_ticks).with_rc_setup(gaps::xkcd_strong),
    FigureCase::new(
        "offsetbox/anchored_text_location_codes",
        layouts::location_codes,
    ),
    FigureCase::new(
        "offsetbox/packer_alignments_and_modes",
        layouts::packer_alignments,
    ),
    FigureCase::new(
        "offsetbox/drawing_area_descents_in_packer",
        layouts::drawing_area_descents,
    ),
    FigureCase::new(
        "offsetbox/textarea_multiline_baseline",
        layouts::multiline_baseline,
    ),
    FigureCase::new("offsetbox/anchored_text_boxstyles", layouts::text_boxstyles),
    FigureCase::new("offsetbox/figure_anchored", layouts::figure_anchored).sized(4.0, 3.0, 100.0),
    FigureCase::new("offsetbox/subfigure_anchored", layouts::subfigure_anchored)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new(
        "offsetbox/annotation_bbox_packed_children",
        pointers::packed_children,
    ),
    FigureCase::new(
        "offsetbox/annotation_bbox_aux_and_padded",
        pointers::aux_and_padded,
    ),
    FigureCase::new(
        "offsetbox/annotation_bbox_mixed_coords",
        pointers::mixed_coords,
    ),
];

/// `T = np.linspace(0, 10, 40)`.
fn t() -> Vec<f64> {
    linspace(0.0, 10.0, 40)
}

/// `T` mapped through `f`.
fn of_t(f: impl Fn(f64) -> f64) -> Vec<f64> {
    t().into_iter().map(f).collect()
}

/// `TextArea(text, textprops=props)`.
fn text_area(text: &str, props: TextProps) -> OffsetBox<BoxItem> {
    TextArea::from_text(text, props).into()
}

/// `_gradient(rows, cols)`: `sin(x / 3) + cos(y / 4)` on the index grid.
fn gradient(rows: usize, cols: usize) -> Grid<f64> {
    Grid::from_fn(rows, cols, |r, c| {
        (c as f64 / 3.0).sin() + (r as f64 / 4.0).cos()
    })
}

/// `_unit_axes(fig)`: an axes over `(0, 1)²`.
fn unit_axes(fig: &Figure) -> Result<Axes, PlotError> {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    Ok(ax)
}

/// A patch of `shape` with face colour `face`.
pub(super) fn filled<S: pltrs::patches::PatchShape>(shape: S, face: &str) -> BoxItem {
    BoxItem::patch(shape, PatchProps::default().facecolor(face))
}
