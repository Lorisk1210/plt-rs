//! Chapter 8: scales, ticks and dates.
//!
//! An axis maps data values to positions. Its *limits* say which range of
//! values is shown, its *scale* how values map to positions in between
//! (evenly, logarithmically, ...), and its *ticks* mark chosen values
//! with labels. All three are chosen automatically from the data, and
//! each can be set by hand.
//!
//! # Limits
//!
//! An axes fits its limits to the data it holds, with a small margin
//! (5% of the range by default). [`Axes::set_xlim`] and
//! [`Axes::set_ylim`] fix them, in any values the axis takes (numbers of
//! any type, dates, categories); giving the larger value first inverts
//! the axis. [`Axes::set_margins`] changes the margin instead, keeping
//! the limits automatic.
//!
//! ```
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! ax.plot([0.0, 10.0], [0.0, 5.0])?;
//! ax.set_margins(0.0, 0.1)?;
//! assert_eq!(ax.xlim(), (0.0, 10.0));
//! assert_eq!(ax.ylim(), (-0.5, 5.5));
//!
//! ax.set_xlim(0, 20)?.set_ylim(6.0, 0.0)?; // y grows downwards now
//! assert!(ax.yaxis().inverted());
//! assert!(ax.set_xlim(f64::NAN, 1.0).is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Scales
//!
//! On a *logarithmic* scale, equal distances are equal ratios: 1 to 10
//! takes as much room as 10 to 100. Use it for data spanning several
//! orders of magnitude, or when ratios matter more than differences;
//! exponential growth becomes a straight line. It cannot show zero or
//! negative values. The *symmetric log* scale (`"symlog"`) is logarithmic away
//! from zero and linear in a band around it, for data of both signs. The
//! *logit* scale stretches the ends of the range 0 to 1, for
//! probabilities.
//!
//! [`Axes::set_xscale`] and [`Axes::set_yscale`] choose a scale by name;
//! [`Axes::set_scale_with`] adds its options, such as the base of a log
//! scale or the width of the linear band of a symlog one.
//! [`Axes::semilogy`], [`Axes::semilogx`] and [`Axes::loglog`] plot and
//! set the scales in one call.
//!
//! ```
//! use plt::prelude::*;
//! use plt::scale::{ScaleName, ScaleOptions};
//! use plt::ticker::AxisName;
//!
//! let fig = Figure::new();
//! let axs = fig.subplots(1, 2)?;
//!
//! axs[(0, 0)].plot(0..20, (0..20).map(|n| 2f64.powi(n)))?;
//! axs[(0, 0)].set_yscale("log")?; // 2ⁿ is a straight line
//! assert_eq!(axs[(0, 0)].yscale(), ScaleName::Log);
//!
//! let x = linspace(-100.0, 100.0, 201);
//! axs[(0, 1)].plot(&x, &x)?;
//! let band = ScaleOptions { linthresh: Some(1.0), ..ScaleOptions::default() };
//! axs[(0, 1)].set_scale_with(AxisName::Y, ScaleName::Symlog, band)?;
//!
//! assert!(axs[(0, 0)].set_xscale("loglog").is_err(), "no such scale");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `axis_scales` in `lines.rs` draws the same data on linear,
//! log, symlog and logit axes.
//!
//! # Ticks
//!
//! Two objects decide an axis' ticks. A [`Locator`](crate::ticker::Locator)
//! chooses the values; a [`Formatter`](crate::ticker::Formatter) writes
//! their labels. The defaults pick a few round values and write them as
//! briefly as they allow. To choose:
//!
//! * [`Axes::set_xticks`] and [`Axes::set_xticklabels`] give the exact
//!   values and labels;
//! * [`MultipleLocator`](crate::ticker::MultipleLocator) puts ticks at
//!   multiples of a step, [`MaxNLocator`](crate::ticker::MaxNLocator) at
//!   most a given number of round values,
//!   [`AutoMinorLocator`](crate::ticker::AutoMinorLocator) minor ticks
//!   between the major ones;
//! * [`StrMethodFormatter`](crate::ticker::StrMethodFormatter) writes a
//!   label from a pattern in Python's format syntax,
//!   [`PercentFormatter`](crate::ticker::PercentFormatter) as a
//!   percentage, [`FuncFormatter`](crate::ticker::FuncFormatter) with a
//!   closure of yours.
//!
//! Set them on the [`Axis`](crate::axes::Axis), whose setters chain.
//!
//! ```
//! use plt::prelude::*;
//! use plt::ticker::{AutoMinorLocator, FuncFormatter, MultipleLocator, PercentFormatter};
//!
//! let ax = Figure::new().subplot();
//! ax.plot([0.0, 12.0], [0.0, 0.8])?;
//! ax.set_xlim(0.0, 12.0)?.set_ylim(0.0, 1.0)?;
//!
//! ax.set_xticks([0.0, 3.0, 6.0, 9.0, 12.0]);
//! ax.xaxis()
//!     .set_major_formatter(FuncFormatter::new(|hours, _| format!("{hours:02}:00")))
//!     .set_minor_locator(AutoMinorLocator::new(None));
//! ax.yaxis()
//!     .set_major_locator(MultipleLocator::new(0.25)?)
//!     .set_major_formatter(PercentFormatter::new(1.0));
//!
//! assert_eq!(ax.xaxis().ticklabels(false)?, ["00:00", "03:00", "06:00", "09:00", "12:00"]);
//! // A locator also returns the ticks just beyond the limits; they are not drawn.
//! assert_eq!(ax.yaxis().ticklocs(false)?, [-0.25, 0.0, 0.25, 0.5, 0.75, 1.0, 1.25]);
//! assert_eq!(ax.yaxis().ticklabels(false)?[1..6], ["0%", "25%", "50%", "75%", "100%"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! How ticks look (length, direction, label size and rotation, which
//! sides carry them) is set with [`Axes::tick_params`]; the grid with
//! [`Axes::grid`] and [`Axes::grid_with`].
//!
//! # Dates
//!
//! Plotting [`DateTime`] values makes a date axis (chapter
//! [`data`](super::data)): the values become days since 1970, and the
//! axis takes a date locator and formatter, which choose years, months,
//! days or hours to suit the range shown. Replace the formatter with a
//! [`ConciseDateFormatter`](crate::dates::ConciseDateFormatter) to write
//! each label as briefly as its neighbours allow and move the rest (the
//! year, say) to the end of the axis. For a fixed rhythm, a
//! [`RRuleLocator`](crate::dates::RRuleLocator) places ticks by a
//! calendar rule, such as the first day of every third month, and a
//! [`DateFormatter`](crate::dates::DateFormatter) writes them with a
//! `strftime` pattern.
//!
//! ```
//! use std::f64::consts::TAU;
//!
//! use plt::dates::{ConciseDateFormatter, DateFormatter, DateRange, DateTime, RRuleLocator, TimeDelta};
//! use plt::prelude::*;
//!
//! let start = DateTime::new(2024, 1, 1, 0, 0, 0, 0)?;
//! let end = DateTime::new(2025, 1, 1, 0, 0, 0, 0)?;
//! let days: Vec<DateTime> = DateRange::new(start, end, TimeDelta::from_days(1)?)?.collect();
//! let temperature: Vec<f64> = (0..366)
//!     .map(|day| 10.0 - 8.0 * (TAU * f64::from(day) / 366.0).cos())
//!     .collect();
//!
//! let fig = Figure::with_size(7.0, 5.0, 100.0);
//! let axs = fig.subplots(2, 1)?;
//! axs[(0, 0)].plot(&days[..], &temperature)?;
//! axs[(0, 0)].xaxis().set_major_formatter(ConciseDateFormatter::new());
//! assert_eq!(axs[(0, 0)].xaxis().ticklabels(false)?[..3], ["2024", "Mar", "May"]);
//!
//! let quarters = &axs[(1, 0)];
//! quarters.plot(&days[..], &temperature)?;
//! quarters
//!     .xaxis()
//!     .set_major_locator(RRuleLocator::months(&[], &[1], 3))
//!     .set_major_formatter(DateFormatter::new("%b %Y"));
//! quarters.set_xlim(start, end)?;
//! assert_eq!(
//!     quarters.xaxis().ticklabels(false)?,
//!     ["Jan 2024", "Apr 2024", "Jul 2024", "Oct 2024", "Jan 2025"]
//! );
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Axes::set_xlim`] takes dates as it takes numbers. A [`DateRange`]
//! is the series of date-times from a start up to an end, a
//! [`TimeDelta`] apart, as matplotlib's `drange` but of date-times; it
//! plots directly, or collects into a `Vec`.
//! Gallery: `concise_dates` in `units.rs`.
//!
//! # Time zones
//!
//! A [`DateTime`] has no zone: plotting takes it as UTC, and date
//! numbers count days in UTC. A [`ZonedDateTime`] is a time on the
//! clocks of a [`TimeZone`]. It plots as the instant it stands for, and
//! an axis without a unit takes the zone of the first one it plots as
//! its unit, so its ticks read in that zone, daylight saving time
//! included, as matplotlib's do with aware `datetime`s. Values of other
//! zones in the same list plot as their instants. [`TimeZone::named`]
//! reads a zone from the system's time zone database.
//!
//! ```
//! use plt::dates::{DateRange, DateTime, TimeDelta, TimeZone, ZonedDateTime};
//! use plt::prelude::*;
//! use plt::units::Unit;
//!
//! # if TimeZone::named("Europe/Zurich").is_err() {
//! #     return Ok(()); // no time zone database on this system
//! # }
//! let zurich = TimeZone::named("Europe/Zurich")?;
//! // Readings of a clock in Zurich, every hour from 13:00.
//! let start = DateTime::new(2024, 3, 30, 13, 0, 0, 0)?;
//! let end = DateTime::new(2024, 3, 30, 19, 0, 0, 0)?;
//! let readings = DateRange::new(start, end, TimeDelta::from_hours(1))?
//!     .map(|local| ZonedDateTime::from_local(local, zurich.clone()));
//!
//! let ax = Figure::new().subplot();
//! ax.plot(readings, [3.0, 4.0, 4.5, 4.0, 3.0, 2.0])?;
//! assert_eq!(ax.xaxis().units(), Some(Unit::new(zurich)));
//! assert_eq!(ax.xaxis().ticklabels(false)?[..3], ["03-30 13", "03-30 14", "03-30 15"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Times recorded in UTC plot as they are; [`Axes::xaxis_date`]
//! ([`Axis::axis_date`] on any axis) gives the axis the zone to show
//! them in. Either way the axis keeps its zone for later plots; without
//! one, it writes its ticks in the zone the `timezone` setting names,
//! UTC by default.
//!
//! The kind of label, automatic or concise, follows the `date.converter`
//! setting, for every date axis; [`Axis::set_converter`] chooses it for
//! one axis. As in matplotlib, an axis given its converter that way
//! takes the converter's ticks when it is given a zone, so pass
//! `&TimeZone::Utc` for UTC.
//!
//! ```
//! use plt::dates::{DateRange, DateTime, TimeDelta, TimeZone};
//! use plt::prelude::*;
//! use plt::units::DateConverter;
//!
//! # if TimeZone::named("Europe/Zurich").is_err() {
//! #     return Ok(()); // no time zone database on this system
//! # }
//! let zurich = TimeZone::named("Europe/Zurich")?;
//! let noon = DateTime::new(2024, 3, 30, 12, 0, 0, 0)?; // UTC
//! let evening = DateTime::new(2024, 3, 30, 18, 0, 0, 0)?;
//! let hours = DateRange::new(noon, evening, TimeDelta::from_hours(1))?;
//!
//! let ax = Figure::new().subplot();
//! ax.plot(hours, [3.0, 4.0, 4.5, 4.0, 3.0, 2.0])?;
//! ax.xaxis().set_converter(DateConverter::Concise)?.axis_date(&zurich);
//! assert_eq!(ax.xaxis().ticklabels(false)?[..3], ["13:00", "14:00", "15:00"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! To set both for a whole program, change the settings (chapter
//! [`styling`](super::styling)):
//!
//! ```
//! use plt::rc;
//! use plt::units::DateConverter;
//!
//! rc::update(|params| {
//!     params.timezone = "Europe/Zurich".into();
//!     params.date_converter = DateConverter::Concise;
//! });
//! # rc::restore_startup();
//! ```
//!
//! The [`porting::axes`](crate::porting::axes) page lists the locators,
//! formatters, scales, tick settings and date converters.
//!
//! Next: [saving and output formats](super::saving).
//!
//! [`Axes::set_xlim`]: crate::axes::Axes::set_xlim
//! [`Axes::set_ylim`]: crate::axes::Axes::set_ylim
//! [`Axes::set_margins`]: crate::axes::Axes::set_margins
//! [`Axes::set_xscale`]: crate::axes::Axes::set_xscale
//! [`Axes::set_yscale`]: crate::axes::Axes::set_yscale
//! [`Axes::set_scale_with`]: crate::axes::Axes::set_scale_with
//! [`Axes::semilogy`]: crate::axes::Axes::semilogy
//! [`Axes::semilogx`]: crate::axes::Axes::semilogx
//! [`Axes::loglog`]: crate::axes::Axes::loglog
//! [`Axes::set_xticks`]: crate::axes::Axes::set_xticks
//! [`Axes::set_xticklabels`]: crate::axes::Axes::set_xticklabels
//! [`Axes::tick_params`]: crate::axes::Axes::tick_params
//! [`Axes::grid`]: crate::axes::Axes::grid
//! [`Axes::grid_with`]: crate::axes::Axes::grid_with
//! [`DateTime`]: crate::dates::DateTime
//! [`DateRange`]: crate::dates::DateRange
//! [`TimeDelta`]: crate::dates::TimeDelta
//! [`ZonedDateTime`]: crate::dates::ZonedDateTime
//! [`TimeZone`]: crate::dates::TimeZone
//! [`TimeZone::named`]: crate::dates::TimeZone::named
//! [`Axes::xaxis_date`]: crate::axes::Axes::xaxis_date
//! [`Axis::axis_date`]: crate::axes::Axis::axis_date
//! [`Axis::set_converter`]: crate::axes::Axis::set_converter
