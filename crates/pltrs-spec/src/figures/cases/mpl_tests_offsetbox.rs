//! `figures/cases_mpl_tests_offsetbox.py`: matplotlib's non-image tests of
//! `test_offsetbox.py` that only check that a figure draws, in the test
//! suite's default style: framed anchored boxes at every location name, and
//! an expanded two-column legend under tight layout.

use pltrs::layout::TightLayout;
use pltrs::legend::{LegendMode, LegendOptions};
use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, BoxItem, DrawingArea};
use pltrs::prelude::*;

use super::mpl_tests::classic;
use crate::figures::FigureCase;

/// The location names of `test_offsetbox_loc_codes`.
const LOCATIONS: [&str; 10] = [
    "upper right",
    "upper left",
    "lower left",
    "lower right",
    "right",
    "center left",
    "center right",
    "lower center",
    "upper center",
    "center",
];

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        // `test_offsetbox_loc_codes`: an empty drawing area anchored at
        // every location name.
        classic("mpl_tests_offsetbox/offsetbox_loc_codes", |fig| {
            let ax = fig.subplot();
            for name in LOCATIONS {
                let loc: AnchorLoc = name.parse()?;
                let area = DrawingArea::<BoxItem>::new(100.0, 100.0, 0.0, 0.0);
                ax.add_anchored_box(AnchoredBox::new(area, AnchoredOptions::new(loc)));
            }
            Ok(())
        }),
        // `test_expand_with_tight_layout`: an expanded legend in two
        // columns, then tight layout.
        classic("mpl_tests_offsetbox/expand_with_tight_layout", |fig| {
            let ax = fig.subplot();
            ax.plot_y([1.0, 2.0]).set_label("series 1");
            ax.plot_y([2.0, 1.0]).set_label("series 2");
            ax.legend(LegendOptions::default().ncols(2).mode(LegendMode::Expand))?;
            fig.tight_layout(TightLayout::default())?;
            Ok(())
        }),
    ]
}
