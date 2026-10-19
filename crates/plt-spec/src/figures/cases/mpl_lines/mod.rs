//! `figures/cases_mpl_lines.py`: matplotlib's own image tests of
//! `test_lines.py` (one submodule) and `test_path.py` (another).

mod lines;
mod paths;

use plt::axes::{Axes, PlotError};
use plt::lines::{Line2D, LineProps};

use crate::figures::FigureCase;

/// `ax.plot(y, ...)`: `y` against its indices.
fn plot_y(ax: &Axes, y: impl AsRef<[f64]>, props: LineProps) -> Result<Line2D, PlotError> {
    let y = y.as_ref();
    let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
    ax.plot_with(x, y, props)
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [lines::CASES, paths::CASES].concat()
}
