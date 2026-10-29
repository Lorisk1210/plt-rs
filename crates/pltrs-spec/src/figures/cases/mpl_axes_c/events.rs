//! Event plot tests: per-row colours, offsets and lengths, the defaults,
//! per-row widths and styles, and rows of dates.

use pltrs::axes::EventplotOptions;
use pltrs::colors::Color;
use pltrs::dates::DateTime;
use pltrs::figure::Figure;
use pltrs::lines::LineStyle;

use super::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(crate) const CASES: &[FigureCase] = &[
    classic("mpl_axes_c/eventplot", |fig| {
        let (data1, data2) = event_data();
        let mut colors = vec![Color::from((0.0, 1.0, 0.7)); data1.len()];
        colors.extend(
            [
                (1.0, 0.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 0.0, 1.0),
                (1.0, 0.75, 0.0),
                (1.0, 0.0, 1.0),
                (0.0, 1.0, 1.0),
            ]
            .map(Color::from),
        );
        let mut lineoffsets: Vec<f64> = (0_u32..32).map(|i| 12.0 + f64::from(i) * 0.33).collect();
        lineoffsets.extend([-15.0, -3.0, 1.0, 1.5, 6.0, 10.0]);
        let mut linelengths = vec![0.33; data1.len()];
        linelengths.extend([5.0, 2.0, 1.0, 1.0, 3.0, 1.5]);
        let options = EventplotOptions {
            colors,
            lineoffsets,
            linelengths,
            ..EventplotOptions::default()
        };
        fig.subplot().eventplot(&[data1, data2].concat(), options)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_c/eventplot_defaults", |fig| {
        let (data1, data2) = event_data();
        fig.subplot()
            .eventplot(&[data1, data2].concat(), EventplotOptions::default())?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_c/eventplot_problem_kwargs", |fig| {
        let mut rng = SpecRng::new(0);
        let data = [rng.uniform(20, 0.0, 1.0), rng.uniform(10, 0.0, 1.0)];
        let options = EventplotOptions {
            colors: vec![Color::from("r"), Color::from("b")],
            linewidths: Some(vec![2.0, 1.0]),
            linestyles: vec![LineStyle::Solid, LineStyle::Dashed],
            ..EventplotOptions::default()
        };
        fig.subplot().eventplot(&data, options)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_c/eventplot_units_list", units_list),
];

/// 32 rows and 6 rows of 20 uniform events, in that order from one
/// stream (`np.random.random([32, 20])` and `([6, 20])` with seed 0).
fn event_data() -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let mut rng = SpecRng::new(0);
    let mut rows =
        |n: usize| -> Vec<Vec<f64>> { (0..n).map(|_| rng.uniform(20, 0.0, 1.0)).collect() };
    let data1 = rows(32);
    let data2 = rows(6);
    (data1, data2)
}

/// `test_eventplot_units_list`: two rows of dates.
fn units_list(fig: &Figure) -> CaseResult {
    let day = |day| DateTime::new(2021, 1, day, 0, 0, 0, 0);
    let ax = fig.subplot();
    let rows = [vec![day(1)?, day(2)?, day(3)?], vec![day(15)?, day(16)?]]
        .map(|dates| ax.xaxis().convert_units(dates));
    ax.eventplot(&rows, EventplotOptions::default())?;
    Ok(())
}
