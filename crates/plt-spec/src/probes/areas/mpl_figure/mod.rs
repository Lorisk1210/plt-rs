//! Probe operations for the `mpl_figure` area: figures, ported from
//! matplotlib's `tests/test_figure.py` (see
//! `spec/reference/probes/test_mpl_figure.py`, which documents every op).

mod axes;
mod layout;
mod pyplot;
mod saving;
mod subfigures;
mod texts;

use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("pyplot.figure", pyplot::figure),
    Op::new("pyplot.figaspect", pyplot::figaspect),
    Op::new("Figure.align_labels", layout::align_labels),
    Op::new("Figure.autofmt_xdate", layout::autofmt_xdate),
    Op::new("Figure.get_layout_engine", layout::layout_engine),
    Op::new(
        "Figure.get_constrained_layout_pads",
        layout::constrained_pads,
    ),
    Op::new("Figure.get_tightbbox", layout::tightbbox),
    Op::new("SubFigure.get_tightbbox", layout::subfigure_tightbbox),
    Op::new("Figure.gca", axes::gca),
    Op::new("Figure.add_subplot", axes::add_subplot),
    Op::new("Figure.add_axes", axes::add_axes),
    Op::new("Figure.subplots", axes::shared_log_labels),
    Op::new("Axes.remove", axes::remove_shared),
    Op::new("Figure.subplot_mosaic", axes::subplot_mosaic),
    Op::new("SubplotSpec.get_gridspec", axes::reused_gridspec),
    Op::new("Figure.pick", axes::pick),
    Op::new("Figure.suptitle", texts::suptitle),
    Op::new("SubFigure.get_facecolor", texts::subfigure_facecolors),
    Op::new("Figure.get_suptitle", texts::get_suptitles),
    Op::new("Text.remove", texts::remove_suptitles),
    Op::new("Figure.get_label", texts::labels),
    Op::new("Figure.clear", subfigures::figure_clear),
    Op::new("SubFigure.bbox", subfigures::subfigure_bbox),
    Op::new("SubFigure.set_dpi", subfigures::subfigure_dpi),
    Op::new("SubFigure.remove", subfigures::subfigure_remove),
    Op::new("Figure.stale", subfigures::stale),
    Op::new("Axes.get_xticks", subfigures::subfigure_ticks),
    Op::new("Figure.set_size_inches", saving::set_size_inches),
    Op::new("Figure.set_dpi", saving::set_dpi),
    Op::new("Figure.savefig", saving::savefig),
    Op::new("Figure.set_visible", saving::set_visible),
    Op::new("Figure.colorbar", saving::locate_colorbar),
];
