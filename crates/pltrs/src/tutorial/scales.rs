//! Chapter 8: scales, ticks and dates.
//!
//! An axis has *limits* (the range that it shows), a *scale* (linear,
//! logarithmic, ...) and *ticks*. pltrs selects all three from the data,
//! and you can set each of them.
//!
//! # Limits
//!
//! An axes fits its limits to its data, with a margin of 5% by default.
//! [`Axes::set_xlim`] and [`Axes::set_ylim`] set the limits in any values
//! of the axis (numbers, dates, categories). The larger value first
//! inverts the axis. [`Axes::set_margins`] changes the margin.
//!
//! ```
//! use pltrs::prelude::*;
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
//! A *logarithmic* scale (`"log"`) cannot show zero or negative values. The *symmetric log* scale
//! (`"symlog"`) is linear in a band around zero and logarithmic outside
//! it. The *logit* scale stretches the ends of the range 0 to 1, for
//! probabilities.
//!
//! [`Axes::set_xscale`] and [`Axes::set_yscale`] select a scale by name.
//! [`Axes::set_scale_with`] also takes options, such as the log base or
//! the width of the symlog band.
//! [`Axes::semilogy`], [`Axes::semilogx`] and [`Axes::loglog`] plot and
//! set the scales in one call.
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::scale::{ScaleName, ScaleOptions};
//! use pltrs::ticker::AxisName;
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
//! Gallery: `axis_scales` in `lines.rs`.
//!
//! # Ticks
//!
//! A [`Locator`](crate::ticker::Locator) selects the tick values, and a
//! [`Formatter`](crate::ticker::Formatter) writes the labels. Set them on
//! the [`Axis`](crate::axes::Axis):
//!
//! * [`Axes::set_xticks`] and [`Axes::set_xticklabels`] set exact values
//!   and labels.
//! * [`MultipleLocator`](crate::ticker::MultipleLocator) uses multiples of
//!   a step, [`MaxNLocator`](crate::ticker::MaxNLocator) at most n round
//!   values, and [`AutoMinorLocator`](crate::ticker::AutoMinorLocator)
//!   adds minor ticks.
//! * [`StrMethodFormatter`](crate::ticker::StrMethodFormatter) uses a
//!   Python format pattern, [`PercentFormatter`](crate::ticker::PercentFormatter)
//!   writes percentages and [`FuncFormatter`](crate::ticker::FuncFormatter)
//!   calls your closure.
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::ticker::{AutoMinorLocator, FuncFormatter, MultipleLocator, PercentFormatter};
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
//! [`Axes::tick_params`] sets the length, direction, label size and
//! rotation of the ticks, and the sides that have them. [`Axes::grid`]
//! and [`Axes::grid_with`] set the grid.
//!
//! # Dates
//!
//! [`DateTime`] values make a date axis (chapter [`data`](super::data)).
//! Its locator and formatter select years, months, days or hours for the
//! range. A [`ConciseDateFormatter`](crate::dates::ConciseDateFormatter)
//! writes short labels and moves the year to the end of the axis. A
//! [`RRuleLocator`](crate::dates::RRuleLocator) uses a calendar rule, for
//! example the first day of every third month. A
//! [`DateFormatter`](crate::dates::DateFormatter) uses a `strftime`
//! pattern.
//!
//! ```
//! use std::f64::consts::TAU;
//!
//! use pltrs::dates::{ConciseDateFormatter, DateFormatter, DateRange, DateTime, RRuleLocator, TimeDelta};
//! use pltrs::prelude::*;
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
//! [`Axes::set_xlim`] accepts dates. A [`DateRange`] (series of date-times
//! a [`TimeDelta`] apart, like `drange`) plots directly or collects into a
//! `Vec`. Gallery: `concise_dates` in `units.rs`.
//!
//! # Time zones
//!
//! A [`DateTime`] has no time zone, and pltrs plots it as UTC. A
//! [`ZonedDateTime`] is a time in a [`TimeZone`]. An axis without a unit
//! takes the zone of the first `ZonedDateTime` that it plots. Its ticks
//! then show that zone, with daylight saving time. Values in other zones
//! plot at their instants. [`TimeZone::named`]
//! reads a zone from the time zone database of the system.
//!
//! ```
//! use pltrs::dates::{DateRange, DateTime, TimeDelta, TimeZone, ZonedDateTime};
//! use pltrs::prelude::*;
//! use pltrs::units::Unit;
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
//! For times in UTC, [`Axes::xaxis_date`] or [`Axis::axis_date`] gives the
//! axis a zone for its ticks. The axis keeps the zone for later plots.
//! Without a zone, an axis uses the setting `timezone` (UTC by default).
//!
//! The setting `date.converter` selects automatic or concise labels for
//! all date axes. [`Axis::set_converter`] selects them for one axis. As in
//! matplotlib, `set_converter` alone does not change the ticks. A later
//! `axis_date` with a zone gives the axis the ticks of the converter. For
//! UTC, pass `&TimeZone::Utc`.
//!
//! ```
//! use pltrs::dates::{DateRange, DateTime, TimeDelta, TimeZone};
//! use pltrs::prelude::*;
//! use pltrs::units::DateConverter;
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
//! use pltrs::rc;
//! use pltrs::units::DateConverter;
//!
//! rc::update(|params| {
//!     params.timezone = "Europe/Zurich".into();
//!     params.date_converter = DateConverter::Concise;
//! });
//! # rc::restore_startup();
//! ```
//!
//! The page [`porting::axes`](crate::porting::axes) lists all locators,
//! formatters, scales and tick settings.
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
