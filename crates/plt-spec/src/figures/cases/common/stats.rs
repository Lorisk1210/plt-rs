//! Options of the statistics plots several case groups draw: histograms,
//! box plots and violin plots changed in one respect.

use plt::axes::{
    BoxplotOptions, BxpOptions, HistPlotOptions, ViolinDrawOptions, ViolinplotOptions,
};
use plt::stats::Bins;

/// `bins=n`.
pub(crate) fn bins(n: usize) -> HistPlotOptions<'static> {
    HistPlotOptions {
        bins: Some(Bins::Count(n)),
        ..HistPlotOptions::default()
    }
}

/// Box plot options with outliers styled by the format string `sym`.
pub(crate) fn with_sym(sym: &str) -> BoxplotOptions {
    BoxplotOptions {
        sym: Some(sym.into()),
        ..BoxplotOptions::default()
    }
}

/// Box plot options that only change how the boxes are drawn.
pub(crate) fn drawn_boxes(draw: BxpOptions) -> BoxplotOptions {
    BoxplotOptions {
        draw,
        ..BoxplotOptions::default()
    }
}

/// Violin plot options that only change how the violins are drawn.
pub(crate) fn drawn_violins(draw: ViolinDrawOptions) -> ViolinplotOptions {
    ViolinplotOptions {
        draw,
        ..ViolinplotOptions::default()
    }
}
