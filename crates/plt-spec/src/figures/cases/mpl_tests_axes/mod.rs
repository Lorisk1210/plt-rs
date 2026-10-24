//! `figures/cases_mpl_tests_axes.py`: matplotlib's own tests of `Axes`
//! (`test_axes.py`) that draw without an image comparison. Here the
//! `fig_test` side of `test_limits_empty_data`, one case per plotting
//! function, in the `default` style the test picks (no rc): plotting
//! empty data first must not change the autoscaled limits of the dates
//! plotted next. The cases of the second half of the file are in
//! [`draws`].

mod draws;

use plt::axes::{Axes, FillBetweenOptions, ScatterColors, ScatterOptions};
use plt::collections::CollectionProps;
use plt::colors::Color;
use plt::dates::Datetime64;
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::units::AxisValues;

use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    let mut cases = vec![
        FigureCase::new("mpl_tests_axes/limits_empty_data_scatter", |fig| {
            limits_empty_data(fig, PlotFun::Scatter)
        }),
        FigureCase::new("mpl_tests_axes/limits_empty_data_plot", |fig| {
            limits_empty_data(fig, PlotFun::Plot)
        }),
        FigureCase::new("mpl_tests_axes/limits_empty_data_fill_between", |fig| {
            limits_empty_data(fig, PlotFun::FillBetween)
        }),
    ];
    cases.extend(draws::cases());
    cases
}

/// The plotting functions `test_limits_empty_data` is parametrised over.
#[derive(Clone, Copy)]
enum PlotFun {
    Scatter,
    Plot,
    FillBetween,
}

impl PlotFun {
    /// `plot_fun(x, y, color="C0")` on `ax` (`fill_between` fills down to
    /// zero).
    fn draw(self, ax: &Axes, x: AxisValues, y: Vec<f64>) -> CaseResult {
        let color = Color::from("C0");
        match self {
            Self::Scatter => {
                let options = ScatterOptions {
                    colors: ScatterColors::Single(color),
                    ..ScatterOptions::default()
                };
                ax.scatter_with(x, y, options)?;
            }
            Self::Plot => {
                ax.plot_with(x, y, LineProps::default().color(color))?;
            }
            Self::FillBetween => {
                let options = FillBetweenOptions {
                    props: CollectionProps::default().color(color),
                    ..FillBetweenOptions::default()
                };
                ax.fill_between(x, y, [0.0], options)?;
            }
        }
        Ok(())
    }
}

/// `plot_fun([], [])`, then `plot_fun` of the days of 2010 against their
/// index.
fn limits_empty_data(fig: &Figure, plot_fun: PlotFun) -> CaseResult {
    const JANUARY_1ST_2010: i64 = 14_610;
    const SECONDS_PER_DAY: i64 = 86_400;
    let days: Vec<Datetime64> = (JANUARY_1ST_2010..JANUARY_1ST_2010 + 365)
        .map(|day| Datetime64::new(day * SECONDS_PER_DAY, 0))
        .collect();
    let index: Vec<f64> = (0..days.len()).map(|i| i as f64).collect();
    let ax = fig.subplot();
    plot_fun.draw(&ax, AxisValues::Numbers(Vec::new()), Vec::new())?;
    plot_fun.draw(&ax, days.into(), index)
}
