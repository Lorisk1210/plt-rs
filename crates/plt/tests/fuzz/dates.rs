//! Dates: ranges of date-times and `drange` over absurd spans and steps
//! (zero, negative, a microsecond across millennia, the extremes of a
//! duration), durations of whole units at the edges of their types, and
//! zoned date-times (mixed zones, fixed offsets at their limits, named
//! zones across clock changes, instants at the ends of years 1 and 9999)
//! plotted, used as limits, spans, error bar origins and text positions
//! on 2-D and 3-D axes, which are drawn.

use plt::axes::{Axes, ErrorbarOptions};
use plt::dates::{
    DateRange, DateTime, MAX_RANGE_DATES, TimeDelta, TimeZone, UtcOffset, ZonedDateTime, drange,
};
use plt::figure::{Figure, SaveFormat};
use plt::lines::LineProps;
use plt::mplot3d::{Axes3D, Axes3DOptions};
use plt::units::AxisValues;

use crate::harness::{Gen, fuzz};
use crate::support::{format, render};

/// Microseconds from 1970 to the first instant of year 1.
const FIRST_MICROS: i64 = -62_135_596_800_000_000;
/// Microseconds from 1970 to the last instant of year 9999.
const LAST_MICROS: i64 = 253_402_300_799_999_999;
/// Microseconds from 1970 to 2020-03-29T01:00:00, Berlin's spring change.
const SPRING_MICROS: i64 = 1_585_443_600_000_000;

/// Named zones with clock changes, odd offsets or none.
const ZONES: [&str; 6] = [
    "Europe/Berlin",
    "America/New_York",
    "Australia/Lord_Howe",
    "Asia/Kathmandu",
    "Pacific/Kiritimati",
    "UTC",
];

/// Spans of a range in microseconds: none, a microsecond, an hour, a day,
/// a year.
const SPANS: [i64; 5] = [0, 1, 3_600_000_000, 86_400_000_000, 31_536_000_000_000];

/// A date-time: near either end of years 1 to 9999, near a clock change,
/// or anywhere between.
fn date_time(g: &mut Gen) -> DateTime {
    let jitter = |g: &mut Gen| (g.between(0.0, 1.0) * g.pick(&SPANS) as f64) as i64;
    let micros = match g.below(4) {
        0 => FIRST_MICROS + jitter(g),
        1 => LAST_MICROS - jitter(g),
        2 => SPRING_MICROS + jitter(g) - jitter(g),
        _ => g.between(FIRST_MICROS as f64, LAST_MICROS as f64) as i64,
    };
    DateTime::from_timestamp_micros(micros.clamp(FIRST_MICROS, LAST_MICROS))
        .expect("an instant within years 1 to 9999")
}

/// A step: ordinary, or zero, negative, a microsecond, or at the
/// extremes of a duration and of the whole-unit constructors.
fn step(g: &mut Gen) -> TimeDelta {
    let edges = [i32::MIN, -1, 0, 1, 7, 24, i32::MAX];
    match g.below(7) {
        0 => TimeDelta::from_micros(g.pick(&[i64::MIN, -1, 0, 1, 1_000, i64::MAX])),
        1 => TimeDelta::from_secs(g.pick(&edges)),
        2 => TimeDelta::from_mins(g.pick(&edges)),
        3 => TimeDelta::from_hours(g.pick(&edges)),
        4 => TimeDelta::from_days(g.pick(&[i64::MIN, -1, 0, 1, 365, i64::MAX]))
            .unwrap_or(TimeDelta::ZERO),
        5 => TimeDelta::from_weeks(g.below(1 << 20) as i64).unwrap_or(TimeDelta::ZERO),
        _ => TimeDelta::from_micros(g.between(1.0, 1e12) as i64),
    }
}

/// A range's input: a start, an end a few steps away or anywhere, and a
/// step.
#[derive(Debug)]
struct RangeInput {
    start: DateTime,
    end: DateTime,
    step: TimeDelta,
}

impl RangeInput {
    fn generate(g: &mut Gen) -> Self {
        let start = date_time(g);
        let step = step(g);
        let end = if g.below(3) == 0 {
            date_time(g)
        } else {
            let steps = g.below(300) as i64 - 20;
            let reach = step.checked_mul(steps).unwrap_or(TimeDelta::ZERO);
            start.checked_add(reach).unwrap_or(start)
        };
        Self { start, end, step }
    }
}

#[test]
fn date_ranges() {
    fuzz("date ranges", 10_000, RangeInput::generate, |input| {
        let RangeInput { start, end, step } = *input;
        let numbers = drange(&ZonedDateTime::utc(start), &ZonedDateTime::utc(end), step);
        if let Ok(numbers) = &numbers {
            assert!(numbers.len() <= MAX_RANGE_DATES);
        }
        let Ok(range) = DateRange::new(start, end, step) else {
            return;
        };
        let len = range.len();
        assert!(len <= MAX_RANGE_DATES && step > TimeDelta::ZERO);
        let front: Vec<DateTime> = range.clone().take(64).collect();
        let back: Vec<DateTime> = range.clone().rev().take(64).collect();
        assert!(front.iter().chain(&back).all(|t| (start..end).contains(t)));
        assert!(front.windows(2).all(|pair| pair[1] - pair[0] == step));
        assert_eq!(front.first(), (len > 0).then_some(&start));
        if len <= 4096 {
            let values = AxisValues::from(range);
            assert_eq!(values.len(), len);
        }
    });
}

/// A zone: UTC, a fixed offset up to its limits, or a named zone.
fn zone(g: &mut Gen) -> TimeZone {
    match g.below(3) {
        0 => TimeZone::Utc,
        1 => {
            let seconds = g.pick(&[-86_399, -18_000, -1, 0, 1, 20_700, 86_399]);
            TimeZone::Fixed(UtcOffset::from_seconds(seconds).expect("an offset under a day"))
        }
        _ => TimeZone::named(g.pick(&ZONES)).unwrap_or(TimeZone::Utc),
    }
}

/// A zoned date-time: a wall-clock time in a zone, which may stand for
/// no instant of years 1 to 9999.
fn zoned(g: &mut Gen) -> ZonedDateTime {
    ZonedDateTime::from_local(date_time(g), zone(g))
}

/// Something done with zoned date-times.
#[derive(Debug)]
enum Call {
    Plot(Vec<ZonedDateTime>),
    Scatter(Vec<ZonedDateTime>),
    PlotRange(RangeInput),
    Limits(ZonedDateTime, ZonedDateTime),
    Span(ZonedDateTime, ZonedDateTime),
    Line(ZonedDateTime),
    Text(ZonedDateTime),
    Errorbar(Vec<ZonedDateTime>, TimeDelta),
    AxisDate(TimeZone),
    Plot3D(Vec<ZonedDateTime>),
    Limits3D(ZonedDateTime, ZonedDateTime),
}

impl Call {
    fn generate(g: &mut Gen) -> Self {
        let dates = |g: &mut Gen| (0..g.len(12)).map(|_| zoned(g)).collect::<Vec<_>>();
        match g.below(11) {
            0 => Self::Plot(dates(g)),
            1 => Self::Scatter(dates(g)),
            2 => Self::PlotRange(RangeInput::generate(g)),
            3 => Self::Limits(zoned(g), zoned(g)),
            4 => Self::Span(zoned(g), zoned(g)),
            5 => Self::Line(zoned(g)),
            6 => Self::Text(zoned(g)),
            7 => Self::Errorbar(dates(g), step(g)),
            8 => Self::AxisDate(zone(g)),
            9 => Self::Plot3D(dates(g)),
            _ => Self::Limits3D(zoned(g), zoned(g)),
        }
    }
}

/// Ascending numbers as many as `dates`, for the other coordinate.
fn counts<T>(dates: &[T]) -> Vec<f64> {
    (0..dates.len()).map(|i| i as f64).collect()
}

/// Does `call` on `ax` or, for the 3-D calls, on `ax3`; errors are fine.
fn make(ax: &Axes, ax3: &Axes3D, call: &Call) {
    match call {
        Call::Plot(dates) => {
            let _ = ax.plot(dates, counts(dates));
        }
        Call::Scatter(dates) => {
            let _ = ax.scatter(dates, counts(dates));
        }
        Call::PlotRange(input) => {
            if let Ok(range) = DateRange::new(input.start, input.end, input.step) {
                let dates: Vec<DateTime> = range.take(512).collect();
                let _ = ax.plot(&dates, counts(&dates));
            }
        }
        Call::Limits(low, high) => {
            let _ = ax.set_xlim(low, high);
        }
        Call::Span(low, high) => {
            ax.axvspan(low, high);
        }
        Call::Line(at) => {
            ax.axvline(at, LineProps::default());
        }
        Call::Text(at) => {
            ax.text(at, 0.5, "at");
        }
        Call::Errorbar(dates, error) => {
            let options = ErrorbarOptions {
                xerr: Some((*error).into()),
                ..ErrorbarOptions::default()
            };
            let _ = ax.errorbar(dates, counts(dates), options);
        }
        Call::AxisDate(tz) => {
            ax.xaxis_date(tz);
        }
        Call::Plot3D(dates) => {
            let _ = ax3.plot(counts(dates), counts(dates), dates);
        }
        Call::Limits3D(low, high) => {
            let _ = ax3.set_zlim(low, high);
        }
    }
}

#[test]
fn zoned_dates_on_axes() {
    fuzz(
        "zoned dates",
        200,
        |g| {
            let calls: Vec<Call> = (0..=g.len(6)).map(|_| Call::generate(g)).collect();
            (calls, format(g))
        },
        |(calls, format)| {
            let fig = Figure::with_size(3.0, 1.5, 40.0);
            let ax = fig.add_subplot(1, 2, 1).expect("a cell");
            let ax3 = fig.add_subplot_3d(1, 2, 2, Axes3DOptions::default());
            let ax3 = ax3.expect("a 3-D axes");
            for call in calls {
                make(&ax, &ax3, call);
            }
            render(&fig, *format);
            if *format != SaveFormat::Png {
                render(&fig, SaveFormat::Png);
            }
        },
    );
}
