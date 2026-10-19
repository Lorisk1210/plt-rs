//! `figures/cases_mpl_axes_b.py`: matplotlib's own image tests of
//! `test_axes.py` (bars, grouped bars, histograms, stairs, contours,
//! hist2d, scatter, stackplot and `bxp`), split by plot family.

mod bars;
mod bxp;
mod contour;
mod hist;
mod scatter;
mod stackplot;
mod stairs;

use plt::axes::Limits;
use plt::figure::Figure;
use plt::ticker::NullFormatter;

use crate::figures::{CaseResult, FigureCase};

/// `_classic_test_patch`: what the test suite changes on top of the
/// classic style.
const CLASSIC_TEST_PATCH: &[(&str, &str)] = &[
    ("ytick.alignment", "center_baseline"),
    ("hatch.color", "edge"),
    ("text.hinting", "default"),
];

/// A case under the test suite's `("classic", "_classic_test_patch")`
/// style, on classic's 8 × 6 in canvas saved at 100 dpi.
const fn classic(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build)
        .sized(8.0, 6.0, 100.0)
        .with_style("classic")
        .with_rc(CLASSIC_TEST_PATCH)
}

/// New view limits from `low` on, keeping the other limit
/// (`set_ylim(bottom=low)` and the like).
fn from(low: f64) -> Limits {
    Limits {
        low: Some(low),
        ..Limits::default()
    }
}

/// matplotlib's `remove_ticks_and_titles` (`remove_text=True`): no
/// suptitle, no axes titles and no tick labels.
fn remove_ticks_and_titles(fig: &Figure) {
    if !fig.suptitle_text().is_empty() {
        fig.suptitle("");
    }
    for ax in fig.axes() {
        ax.set_title("");
        for axis in [ax.xaxis(), ax.yaxis()] {
            axis.set_major_formatter(NullFormatter)
                .set_minor_formatter(NullFormatter);
        }
    }
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        bars::CASES,
        hist::CASES,
        stairs::CASES,
        contour::CASES,
        scatter::CASES,
        stackplot::CASES,
        bxp::CASES,
    ]
    .concat()
}
