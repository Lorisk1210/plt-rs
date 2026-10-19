//! `figures/cases_mpl_small.py`: matplotlib's own image tests from its
//! smaller test modules: spines and scales in one submodule, markers and
//! artists in another, subplot layouts and transforms in a third.

mod layouts;
mod markers;
mod spines;

use plt::figure::Figure;
use plt::ticker::NullFormatter;

use crate::figures::FigureCase;

/// `matplotlib.style.library["_classic_test_patch"]`, applied on top of
/// the classic sheet (`style="_classic_test"`).
const CLASSIC_TEST_PATCH: &[(&str, &str)] = &[
    ("ytick.alignment", "center_baseline"),
    ("hatch.color", "edge"),
    ("text.hinting", "default"),
];

/// `remove_ticks_and_titles`: no suptitle, titles or tick labels.
fn remove_text(fig: &Figure) {
    fig.suptitle("");
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
    [spines::CASES, markers::CASES, layouts::CASES].concat()
}
