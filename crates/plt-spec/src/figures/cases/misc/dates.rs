//! Date axis cases of `figures/cases_axes_misc.py`.

use plt::dates::{
    AutoDateLocator, ConciseDateFormatter, DateError, DateFormatter, DateTime, Datetime64,
    RRuleLocator,
};
use plt::gridspec::SubplotOverrides;
use plt::prelude::*;
use plt::text::HorizontalAlignment;

use super::wave;
use crate::figures::FigureCase;

const MINUTE: i64 = 60_000_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

/// `_dates(start, step, n)`: `n` date-times `step` microseconds apart.
fn dates(start: DateTime, step: i64, n: usize) -> Result<Vec<DateTime>, DateError> {
    let origin = start.timestamp_micros();
    (0..n as i64)
        .map(|i| DateTime::from_timestamp_micros(origin + i * step))
        .collect()
}

/// `np.arange(start, stop, step)` of `datetime64` values.
fn datetime64_range(start: DateTime, stop: DateTime, step: i64) -> Vec<Datetime64> {
    let (start, stop) = (start.timestamp_micros(), stop.timestamp_micros());
    (start..stop)
        .step_by(usize::try_from(step).unwrap_or(usize::MAX))
        .map(|micros| Datetime64::new(micros.div_euclid(1_000_000), 0))
        .collect()
}

/// `datetime(year, month, day, hour, minute)`.
fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Result<DateTime, DateError> {
    DateTime::new(year, month, day, hour, minute, 0, 0)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/dates_minutes", |fig| {
        fig.subplot()
            .plot(dates(at(2024, 3, 1, 12, 0)?, MINUTE, 25)?, wave(25))?;
        Ok(())
    })
    .sized(6.4, 3.0, 100.0),
    FigureCase::new("misc/dates_hours", |fig| {
        fig.subplot()
            .plot(dates(at(2024, 3, 1, 6, 0)?, HOUR, 15)?, wave(15))?;
        Ok(())
    })
    .sized(8.0, 3.0, 100.0),
    FigureCase::new("misc/dates_days", |fig| {
        fig.subplot()
            .plot(dates(at(2024, 2, 20, 0, 0)?, DAY, 6)?, wave(6))?;
        Ok(())
    }),
    FigureCase::new("misc/dates_weeks_month_reset", |fig| {
        fig.subplot()
            .plot(dates(at(2024, 2, 20, 0, 0)?, DAY, 40)?, wave(40))?;
        Ok(())
    })
    .sized(8.0, 3.0, 100.0),
    FigureCase::new("misc/dates_months", |fig| {
        fig.subplot()
            .plot(dates(at(2023, 11, 15, 0, 0)?, 7 * DAY, 25)?, wave(25))?;
        Ok(())
    })
    .sized(6.4, 3.0, 100.0),
    FigureCase::new("misc/dates_years", |fig| {
        fig.subplot()
            .plot(dates(at(2015, 1, 1, 0, 0)?, 60 * DAY, 40)?, wave(40))?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("misc/dates_decades", |fig| {
        let decades = (0..30)
            .map(|i| at(1900 + 4 * i, 7, 1, 0, 0))
            .collect::<Result<Vec<_>, _>>()?;
        fig.subplot().plot(decades, wave(30))?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("misc/dates_concise_formatter", |fig| {
        let axs = fig.subplots(3, 1)?;
        let spans = [
            (at(2024, 1, 30, 22, 0)?, 10 * MINUTE),
            (at(2024, 1, 25, 0, 0)?, 8 * HOUR),
            (at(2023, 10, 1, 0, 0)?, 9 * DAY),
        ];
        for (ax, (start, step)) in axs.as_slice().iter().zip(spans) {
            ax.xaxis()
                .set_major_locator(AutoDateLocator::new())
                .set_major_formatter(ConciseDateFormatter::new());
            ax.plot(dates(start, step, 30)?, wave(30))?;
        }
        fig.subplots_adjust(&SubplotOverrides {
            hspace: Some(0.6),
            ..SubplotOverrides::default()
        })?;
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("misc/dates_autofmt_xdate", |fig| {
        fig.subplot()
            .plot(dates(at(2024, 5, 1, 0, 0)?, 3 * DAY, 30)?, wave(30))?;
        fig.autofmt_xdate(0.2, 30.0, HorizontalAlignment::Right)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("misc/dates_datetime64_bar", |fig| {
        let days = datetime64_range(at(2024, 1, 1, 0, 0)?, at(2024, 1, 7, 0, 0)?, DAY);
        let bars = BarOptions {
            thickness: vec![0.8],
            ..BarOptions::default()
        };
        fig.subplot()
            .bar(days, [3.0, 1.0, 4.0, 1.0, 5.0, 2.0], bars)?;
        Ok(())
    })
    .sized(8.0, 3.0, 100.0),
    FigureCase::new("misc/dates_datetime64_locator_formatter", |fig| {
        let ax = fig.subplot();
        let hours = datetime64_range(at(2024, 1, 1, 0, 0)?, at(2024, 1, 3, 0, 0)?, 2 * HOUR);
        let n = hours.len();
        ax.plot_fmt(hours, wave(n), "o-")?;
        ax.xaxis()
            .set_major_locator(RRuleLocator::hours(&[0, 12], 1))
            .set_major_formatter(DateFormatter::new("%d %b %H:%M"))
            .set_minor_locator(RRuleLocator::hours(&[], 3));
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
];
