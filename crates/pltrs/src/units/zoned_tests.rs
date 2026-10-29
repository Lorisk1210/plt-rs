//! Zoned date-times on axes: the zone of the first value becomes the
//! unit of an axis without one, as matplotlib's `update_units` makes the
//! `tzinfo` of aware `datetime`s the unit, and the values plot as their
//! instants. Plotting updates the units whenever the axis has no unit;
//! other conversions only on an axis with neither converter nor unit
//! (matplotlib's `_plot_args` against `_process_unit_info`).

use crate::axes::{Axes, Axis, BandAlign};
use crate::dates::{DateTime, TimeDelta, TimeZone, UtcOffset, ZonedDateTime};
use crate::figure::{Figure, Share};
use crate::lines::LineProps;
use crate::units::{AxisValues, Unit};

/// A zone `hours` east of UTC.
fn east(hours: i32) -> TimeZone {
    TimeZone::Fixed(UtcOffset::from_seconds(hours * 3600).expect("an offset"))
}

/// 2020-01-01 at `hour` UTC.
fn utc(hour: u32) -> DateTime {
    DateTime::new(2020, 1, 1, hour, 0, 0, 0).expect("a date-time")
}

/// The instant 2020-01-01 at `hour` UTC, in `tz`.
fn zoned(hour: u32, tz: &TimeZone) -> ZonedDateTime {
    ZonedDateTime::from_utc(utc(hour), tz.clone()).expect("a date-time in the zone")
}

/// The zone that is the unit of `axis`, if one is.
fn zone_unit(axis: &Axis) -> Option<TimeZone> {
    axis.units()
        .and_then(|unit| unit.downcast_ref::<TimeZone>().cloned())
}

/// The x tick labels of `ax`.
fn labels(ax: &Axes) -> Vec<String> {
    ax.xaxis().ticklabels(false).expect("tick labels")
}

#[test]
fn zoned_dates_plot_as_naive_dates_in_an_axis_of_their_zone() {
    let tz = east(5);
    let zoned_ax = Figure::new().subplot();
    let hours: Vec<ZonedDateTime> = (12..=18).map(|h| zoned(h, &tz)).collect();
    let line = zoned_ax.plot(&hours, [0.0; 7]).expect("a line");
    assert_eq!(zone_unit(&zoned_ax.xaxis()), Some(tz.clone()));

    let naive_ax = Figure::new().subplot();
    let naive = naive_ax.plot((12..=18).map(utc), [0.0; 7]).expect("a line");
    naive_ax.xaxis_date(&tz);
    assert_eq!(line.xdata(), naive.xdata(), "the instants, in UTC");
    assert_eq!(labels(&zoned_ax), labels(&naive_ax));
    assert_eq!(labels(&zoned_ax)[0], "01-01 17", "in the zone");
    assert_eq!(zoned_ax.xlim(), naive_ax.xlim());
}

#[test]
fn a_list_mixing_zones_takes_the_first_ones() {
    let ax = Figure::new().subplot();
    let mixed = [
        zoned(12, &east(-3)),
        zoned(15, &east(2)),
        zoned(18, &TimeZone::Utc),
    ];
    let line = ax.plot(mixed, [0.0; 3]).expect("a line");
    assert_eq!(zone_unit(&ax.xaxis()), Some(east(-3)));
    let instants = AxisValues::from([utc(12), utc(15), utc(18)]).into_unplaced_numbers();
    assert_eq!(line.xdata(), instants);
    assert_eq!(labels(&ax)[0], "01-01 09");
}

#[test]
fn an_axis_with_a_unit_keeps_it() {
    let ax = Figure::new().subplot();
    ax.xaxis_date(&TimeZone::Utc);
    ax.plot([zoned(12, &east(5)), zoned(18, &east(5))], [0.0, 1.0])
        .expect("a line");
    assert_eq!(zone_unit(&ax.xaxis()), Some(TimeZone::Utc));
    assert_eq!(labels(&ax)[0], "01-01 12");

    let first = Figure::new().subplot();
    first.plot([zoned(12, &east(1))], [0.0]).expect("a line");
    first.plot([zoned(12, &east(9))], [0.0]).expect("a line");
    assert_eq!(zone_unit(&first.xaxis()), Some(east(1)), "the first stays");
}

#[test]
fn plotting_takes_a_zone_after_naive_dates_and_other_methods_do_not() {
    let ax = Figure::new().subplot();
    ax.plot(vec![utc(12), utc(18)], [0.0, 1.0]).expect("a line");
    ax.scatter([zoned(13, &east(5))], [0.5]).expect("a scatter");
    assert_eq!(zone_unit(&ax.xaxis()), None, "the axis has a converter");
    ax.plot([zoned(14, &east(5))], [0.5]).expect("a line");
    assert_eq!(zone_unit(&ax.xaxis()), Some(east(5)));

    let updated = Figure::new().subplot();
    updated.plot(vec![utc(12)], [0.0]).expect("a line");
    assert!(updated.xaxis().update_units([zoned(12, &east(3))]));
    assert_eq!(zone_unit(&updated.xaxis()), Some(east(3)));
}

#[test]
fn limits_spans_bands_and_lines_take_the_zone_and_texts_do_not() {
    let tz = east(-5);
    let limits = Figure::new().subplot();
    limits
        .set_xlim(zoned(12, &tz), zoned(18, &tz))
        .expect("finite limits");
    assert_eq!(zone_unit(&limits.xaxis()), Some(tz.clone()));
    let span = Figure::new().subplot();
    span.axvspan(zoned(12, &tz), zoned(18, &tz));
    assert_eq!(zone_unit(&span.xaxis()), Some(tz.clone()));
    let line = Figure::new().subplot();
    let rule = line.axvline(zoned(12, &tz), LineProps::default());
    assert_eq!(zone_unit(&line.xaxis()), Some(tz.clone()));
    assert_eq!(rule.xdata()[0], limits.xlim().0);
    let bands = Figure::new().subplot();
    let six_hours = TimeDelta::from_hours(6);
    let band = bands.broken_barh(
        &[(zoned(12, &tz), six_hours)],
        (0.0, 1.0),
        BandAlign::Bottom,
    );
    assert_eq!(zone_unit(&bands.xaxis()), Some(tz.clone()));
    let corners = band.paths()[0].vertices().to_vec();
    assert_eq!((corners[0].x, corners[2].x), limits.xlim());

    let text = Figure::new().subplot();
    let note = text.text(zoned(12, &tz), 0.5, "noon");
    assert_eq!(note.position().x, limits.xlim().0);
    assert!(!text.xaxis().have_units(), "texts convert when drawn");
}

#[test]
fn axes_sharing_an_axis_share_the_zone() {
    let fig = Figure::new();
    let axs = fig
        .subplots_shared(2, 1, Share::All, Share::None)
        .expect("a grid");
    axs[(0, 0)]
        .plot([zoned(12, &east(5)), zoned(18, &east(5))], [0.0, 1.0])
        .expect("a line");
    let below = axs[(1, 0)].xaxis();
    assert_eq!(below.units(), Some(Unit::new(east(5))));
    assert_eq!(labels(&axs[(1, 0)])[0], "01-01 17", "ticks in the zone");
}

#[test]
fn three_d_axes_take_the_zone_along_their_axis() {
    let tz = east(5);
    let ax = Figure::new().subplot_3d();
    let hours: Vec<ZonedDateTime> = (12..=14).map(|h| zoned(h, &tz)).collect();
    ax.plot([0.0, 1.0, 2.0], [0.0; 3], &hours).expect("a line");
    let unit = ax.zaxis().units();
    assert_eq!(unit.as_ref().and_then(Unit::downcast_ref), Some(&tz));
    assert!(!ax.xaxis().have_units());
    let limits = Figure::new().subplot_3d();
    limits
        .set_ylim(&hours[0], &hours[2])
        .expect("finite limits");
    assert_eq!(limits.yaxis().units(), Some(Unit::new(tz)));
}
