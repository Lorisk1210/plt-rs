//! `figures/cases_mpl_tests_table.py`: matplotlib's non-image tests of
//! `test_table.py` that only check that a figure draws, in the test
//! suite's default style.

use plt::colors::Color;
use plt::table::TableOptions;

use super::mpl_tests::classic;
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        // `test_non_square`: a table of two cell colours, one per row.
        classic("mpl_tests_table/non_square", |fig| {
            let colours = ["b", "r"].map(|c| vec![Color::from(c)]).to_vec();
            fig.subplot().table(TableOptions {
                cell_colours: Some(colours),
                ..TableOptions::default()
            })?;
            Ok(())
        }),
    ]
}
