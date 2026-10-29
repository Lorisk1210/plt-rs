//! `figures/cases_mpl_tests_figure.py`: matplotlib's non-image tests from
//! `test_figure.py` that draw, in the test suite's default style: a bar
//! label in a subfigure of a constrained figure, a removed shared axes
//! and subfigures on blocks of a grid.

use pltrs::axes::{BarLabelOptions, BarOptions};
use pltrs::figure::{Share, SubfigureOptions};
use pltrs::gridspec::GridSpec;
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;

use super::mpl_tests::classic;
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic("mpl_tests_figure/subfigure_pdf", |fig| {
            fig.set_layout_engine(LayoutEngine::constrained())?;
            let sub = fig.subfigures(1, 1, SubfigureOptions::default())?;
            let ax = sub[(0, 0)].subplot();
            let bars = ax.bar(vec![1.0], [1.0], BarOptions::default())?;
            ax.bar_label(&bars, BarLabelOptions::default())?;
            Ok(())
        }),
        classic("mpl_tests_figure/removed_axis", |fig| {
            let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
            axs.as_slice()[0].remove();
            Ok(())
        }),
        classic("mpl_tests_figure/subfigure_spanning", |fig| {
            fig.set_layout_engine(LayoutEngine::constrained())?;
            let grid = GridSpec::new(3, 3)?;
            for (rows, cols) in [(0..1, 0..1), (0..2, 1..2), (2..3, 1..3), (0..3, 1..3)] {
                fig.add_subfigure(&grid.span(rows, cols)?).subplot();
            }
            Ok(())
        }),
    ]
}
