//! `figures/cases_mpl_axes_c.py`: matplotlib's own image tests of
//! `test_axes.py` from `test_bxp_customwidths` to `test_eb_line_zorder`,
//! drawn under the conditions of matplotlib's test decorators: the
//! test's style (by default "classic" plus `_classic_test_patch`, on an
//! 8 × 6 in canvas saved at 100 dpi), its savefig dpi, and
//! `remove_ticks_and_titles` for `remove_text=True`.

mod boxplots;
mod errorbars;
mod events;
mod hists;
mod markers;
mod violins;

use plt::figure::Figure;
use plt::ticker::NullFormatter;

use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        boxplots::CASES,
        errorbars::CASES,
        events::CASES,
        hists::CASES,
        markers::CASES,
        violins::CASES,
    ]
    .concat()
}

/// `_classic_test_patch`, applied on top of the "classic" sheet.
const CLASSIC_PATCH: &[(&str, &str)] = &[
    ("ytick.alignment", "center_baseline"),
    ("hatch.color", "edge"),
    ("text.hinting", "default"),
];

/// A case in `image_comparison`'s default style (and that of every
/// `check_figures_equal` test): "classic" plus `_classic_test_patch`, on
/// the classic 8 × 6 in canvas saved at the classic 100 dpi.
const fn classic(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build)
        .sized(8.0, 6.0, 100.0)
        .with_style("classic")
        .with_rc(CLASSIC_PATCH)
}

/// A case in the default style saved at `savefig_kwarg={'dpi': 40}`.
const fn at_40_dpi(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).sized(6.4, 4.8, 40.0)
}

/// matplotlib's `remove_ticks_and_titles` (`remove_text=True`): empty
/// titles and suptitle, no tick labels.
fn remove_ticks_and_titles(fig: &Figure) {
    fig.suptitle("");
    for ax in fig.axes() {
        ax.set_title("");
        for axis in [ax.xaxis(), ax.yaxis()] {
            axis.set_major_formatter(NullFormatter);
            axis.set_minor_formatter(NullFormatter);
        }
    }
}

/// `values` mapped through `f`.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}
