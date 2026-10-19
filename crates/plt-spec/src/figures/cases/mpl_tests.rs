//! What matplotlib's image tests share: the test suite's default style
//! and `remove_text=True`, for the groups porting them (`mpl_*`).

use plt::figure::Figure;
use plt::ticker::NullFormatter;

use crate::figures::{CaseResult, FigureCase};

/// `_classic_test_patch`: what the test suite changes on top of the
/// classic style.
pub(super) const CLASSIC_TEST_PATCH: &[(&str, &str)] = &[
    ("ytick.alignment", "center_baseline"),
    ("hatch.color", "edge"),
    ("text.hinting", "default"),
];

/// A case in the test suite's default style, which matplotlib's test
/// fixture applies to every test without a style of its own (`classic`
/// plus `_classic_test_patch`), on classic's 8 × 6 in figure saved at
/// 100 dpi.
pub(super) const fn classic(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build)
        .sized(8.0, 6.0, 100.0)
        .with_style("classic")
        .with_rc(CLASSIC_TEST_PATCH)
}

/// matplotlib's `remove_ticks_and_titles` (what `remove_text=True` does):
/// empty suptitle and titles, no tick labels on any axis.
pub(super) fn remove_ticks_and_titles(fig: &Figure) {
    fig.suptitle("");
    for ax in fig.axes() {
        ax.set_title("");
        for axis in [ax.xaxis(), ax.yaxis()] {
            axis.set_major_formatter(NullFormatter)
                .set_minor_formatter(NullFormatter);
        }
    }
}
