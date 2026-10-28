//! Axes: limits, scales, ticks, locators, formatters, spines, twin and
//! secondary axes, units and dates.
//!
//! An [`Axes`] handle has the methods matplotlib puts on `Axes`; the
//! [`Axis`] handles from [`Axes::xaxis`] and [`Axes::yaxis`] have those of
//! `ax.xaxis` and `ax.yaxis`: locators and formatters, tick labels and
//! lines, the axis label, inversion and units. Per-axis methods of
//! `Axes` take an [`AxisName`] (see [`super::conventions`]).
//!
//! A few things look different:
//!
//! * `set_xlim(left=2)` (one limit) is [`Axes::set_limits_with`] with a
//!   [`Limits`]; inverting an axis is [`Axis::set_inverted`].
//! * Scales are chosen by name as in matplotlib, `"log"`, `"symlog"`,
//!   `"logit"`, `"asinh"`, `"function"`, with their keywords in a
//!   [`ScaleOptions`] ([`Axes::set_scale_with`]), or as a [`Scale`] value
//!   ([`Axes::set_scale`]).
//! * Locators and formatters are values implementing [`Locator`] and
//!   [`Formatter`]; constructors that can fail (`MultipleLocator(0)`, an
//!   invalid format string) return a `Result`.
//! * `tick_params(**kwargs)` takes a [`TickParams`], `grid(**kwargs)` a
//!   [`GridOptions`], `ticklabel_format(**kwargs)` a [`TicklabelFormat`].
//! * Axis labels are styled through the [`Axis`]: `set_xlabel("x",
//!   loc="left", labelpad=4)` is `set_xlabel("x")` followed by
//!   [`Axis::set_label_loc`] and [`Axis::set_label_pad`].
//! * Spines are fields of [`Spines`]; positions are a [`SpinePosition`].
//!
//! # Equivalences
//!
//! | matplotlib                                     | plt-rs                                           |
//! |------------------------------------------------|--------------------------------------------------|
//! | `ax.set_xlim(a, b)`, `ax.get_xlim()`           | [`Axes::set_xlim`], [`Axes::xlim`]               |
//! | `ax.set_xlim(left=a)`, `ax.set_xlim(a, b, auto=None)` | [`Axes::set_limits_with`]`(AxisName::X, Limits { .. })` |
//! | `ax.set_xbound(a, b)`, `ax.get_xbound()`       | [`Axes::set_bound`], [`Axes::bound`]             |
//! | `ax.invert_xaxis()`, `ax.xaxis_inverted()`     | [`Axis::set_inverted`], [`Axis::inverted`]       |
//! | `ax.margins(x, y)`, `ax.set_xmargin(m)`        | [`Axes::set_margins`], [`Axes::set_margin`]      |
//! | `ax.autoscale(axis="x", tight=True)`           | [`Axes::autoscale`]`(Some(AxisName::X), Some(true))` |
//! | `ax.autoscale(enable=False)`                   | [`Axes::set_autoscale_on`]`(false)`              |
//! | `ax.autoscale_view()`, `ax.relim()`            | [`Axes::autoscale_view`], [`Axes::relim`]        |
//! | `ax.has_data()`                                | [`Axes::has_data`]                               |
//! | `ax.axis("equal")`, `ax.axis("off")`           | [`Axes::axis_mode`]`(AxisMode::Equal)`, [`Axes::set_axis_off`] |
//! | `ax.set_aspect("equal")`, `ax.set_aspect(2)`   | [`Axes::set_aspect`]`(Aspect::Equal)`, `Aspect::Ratio(2.0)` ([`Aspect`]) |
//! | `ax.set_adjustable("datalim")`, `ax.set_box_aspect(1)` | [`Axes::set_adjustable`], [`Axes::set_box_aspect`] |
//! | `ax.set_xscale("log")`, `ax.get_xscale()`      | [`Axes::set_xscale`]`(ScaleName::Log)`, [`Axes::xscale`] ([`ScaleName`]) |
//! | `ax.set_xscale("log", base=2)`                 | [`Axes::set_scale_with`] with [`ScaleOptions`]   |
//! | `ax.set_xscale("function", functions=(f, g))`  | [`ScaleOptions::functions`] with a [`FuncTransform`] |
//! | `ax.set_xticks(ticks)`, `ax.set_xticks(ticks, minor=True)` | [`Axes::set_xticks`], [`Axis::set_minor_ticks`] |
//! | `ax.set_xticks(ticks, labels)`, `ax.set_xticklabels(labels)` | [`Axes::set_xticks`] then [`Axes::set_xticklabels`] |
//! | `ax.get_xticks()`, `ax.get_xticklabels()`      | [`Axis::ticklocs`]`(false)`, [`Axis::ticklabels`]`(false)` |
//! | `ax.xaxis.set_major_locator(MultipleLocator(0.5))` | [`Axis::set_major_locator`]`(MultipleLocator::new(0.5)?)` |
//! | `ax.xaxis.set_major_formatter("{x:.1f}")`      | [`Axis::set_major_formatter`]`(StrMethodFormatter::new("{x:.1f}")?)` |
//! | `MaxNLocator`, `AutoMinorLocator`, `LogLocator`, `FixedLocator`, `NullLocator` | [`MaxNLocator`], [`AutoMinorLocator`], [`LogLocator`], [`FixedLocator`], [`NullLocator`] |
//! | `LogLocator(base=2)`, `LogScale(axis, base=2)` | [`LogLocator::new`]`(2.0)?`, [`LogScale::new`]`(2.0)?`, or `from_base` with a [`LogBase`] checked once |
//! | `FormatStrFormatter`, `FuncFormatter`, `PercentFormatter`, `EngFormatter`, `NullFormatter` | [`FormatStrFormatter`], [`FuncFormatter`], [`PercentFormatter`], [`EngFormatter`], [`NullFormatter`] |
//! | `ax.minorticks_on()`                           | [`Axes::minorticks_on`]                          |
//! | `ax.tick_params(axis="x", labelrotation=45)`   | [`Axes::tick_params`]`(TickParams { .. })`       |
//! | `ax.ticklabel_format(style="sci", scilimits=(0, 0))` | [`Axes::ticklabel_format`]`(TicklabelFormat { .. })` |
//! | `ax.locator_params(nbins=4)`                   | [`Axes::locator_params`]`(LocatorParams { .. })`  |
//! | `ax.xaxis.tick_top()`, `set_ticks_position("both")` | [`Axis::tick_to`]`(SpineSide::Top)`, [`Axis::set_ticks_position`] |
//! | `ax.yaxis.set_offset_position("right")`             | [`Axis::set_offset_position`]`(SpineSide::Right)?`, [`Axis::offset_position`] |
//! | `ax.xaxis.set_picker(True)`, `ax.xaxis.contains(event)` | [`Axis::set_picker`]`(Some(Picker::Default))`, [`Axis::contains`]`(point)`; picks name the axis in [`PickEvent::axis`] |
//! | `ax.xaxis.set_pickradius(r)`, `get_pickradius()`, `pickradius` | [`Axis::set_pickradius`]`(r)?`, [`Axis::pickradius`] |
//! | `LockableBbox(bbox, y0=0)`, `locked.locked_y0 = None` | [`LockableBbox`]` { locked_y0: Some(0.0), ..LockableBbox::new(bbox) }`, the public `locked_*` fields; [`LockableBbox::bbox`] is `get_points()` |
//! | `ax.grid(True, which="both", axis="y", ...)`   | [`Axes::grid`], [`Axes::grid_with`] with [`GridOptions`] |
//! | `ax.set_axisbelow(True)`                       | [`Axes::set_axisbelow`]`(AxisBelow::Below)` ([`AxisBelow`]) |
//! | `ax.set_xlabel(s)`, `ax.get_xlabel()`          | [`Axes::set_xlabel`], [`Axes::label`]`(AxisName::X)` |
//! | `set_xlabel(..., loc=, labelpad=, fontsize=, color=)` | [`Axis::set_label_loc`], [`Axis::set_label_pad`], [`Axis::set_label_fontsize`], [`Axis::set_label_color`] |
//! | `set_xlabel(s, loc="left")`, `set_ylabel(s, loc="top")` | [`Axis::set_label_loc`] with [`XLabelLoc::Left`] or [`YLabelLoc::Top`], matplotlib's names, or the [`LabelLoc`] of either axis |
//! | `ax.xaxis.set_in_layout(False)`                | [`Axis::set_in_layout`]`(false)`: the layout engines leave that axis out |
//! | `ax.xaxis.set_label_position("top")`           | [`Axis::set_label_position`]`(SpineSide::Top)`   |
//! | `ax.xaxis.set_label_coords(x, y)`              | [`Axis::set_label_coords`]                       |
//! | `ax.set_ylabel(s, y=1, ha="left")`             | [`Axis::set_label_loc`]`(LabelLoc::End)` then [`Axis::set_label_halign`]`(HAlign::Left)` |
//! | `ax.spines["top"].set_visible(False)`          | `ax.spines().top.set_visible(false)` ([`Spines`]) |
//! | `ax.spines["left"].set_position(("outward", 10))` | [`Spine::set_position`]`(SpinePosition::Outward(10.0))` |
//! | `ax.spines["bottom"].set_position("zero")`     | [`Spine::set_position`]`(SpinePosition::Data(0.0))` |
//! | `ax.spines["left"].set_bounds(lo, hi)`         | [`Spine::set_bounds`]                            |
//! | `spine.set_bounds(lo, None)`, `spine.get_bounds()` | [`Spine::set_bound_ends`], [`Spine::bounds`]   |
//! | `ax.twinx()`, `ax.twiny()`                     | [`Axes::twinx`], [`Axes::twiny`]                 |
//! | `ax.secondary_xaxis("top", functions=(f, g))`  | [`Axes::secondary_xaxis`]`(SecondaryLocation::Side(SpineSide::Top), Some(..))` |
//! | `ax.secondary_xaxis("top", color="r")`, `sec.set_color("r")` | [`Axis::set_color`] on the secondary axes' [`Axes::xaxis`] |
//! | `ax.sharex(other)`, `ax.get_shared_x_axes()`   | [`Axes::share_x`], [`Axes::shared_axes`]         |
//! | `ax.label_outer()`                             | [`Axes::label_outer`]                            |
//! | `ax.xaxis_date()`, `ax.xaxis.axis_date(tz)`    | [`Axes::xaxis_date`], [`Axis::axis_date`]: ticks of the kind of the axis' converter of dates |
//! | `ax.xaxis.convert_units(values)`               | [`Axis::convert_units`]                          |
//! | `ax.xaxis.set_units(u)`, `ax.xaxis.get_units()` | [`Axis::set_units`], [`Axis::units`] ([`Unit`])  |
//! | `ax.xaxis.update_units(data)`                  | [`Axis::update_units`]                           |
//! | `ax.xaxis.set_converter(c)`, `get_converter()` | [`Axis::set_converter`], [`Axis::converter`]     |
//! | `ax.xaxis.set_converter(mdates.ConciseDateConverter())`, `DateConverter()` | [`Axis::set_converter`]`(`[`DateConverter::Concise`]`)?`, `(DateConverter::Auto)`: a [`UnitConverter::DatesBy`], whose ticks ignore `date.converter` |
//! | the converter plotted dates get (`units.registry[datetime]`, a `_SwitchableDateConverter`) | [`UnitConverter::Dates`], whose ticks follow `date.converter`; [`UnitConverter::is_dates`] tells either converter of dates from the others |
//! | `class C(units.ConversionInterface)`, `units.registry[T] = C()` | `impl` [`ConversionInterface`] `for C`, `impl` [`Convertible`] `for T` |
//! | `def convert(value, unit, axis)` (and `axisinfo`, `default_units`) | `fn convert(&self, values, unit, axis: &`[`UnitAxis`]`)`: the axis of 2-D or of 3-D axes |
//! | `units.AxisInfo(majloc=, label=, default_limits=)` | [`UnitAxisInfo`] `{ majloc, label, default_limits, .. }` |
//! | `units.ConversionError`                        | [`ConversionError`]                              |
//! | `mdates.MonthLocator()`, `DayLocator`, `WeekdayLocator` | [`RRuleLocator::months`], [`RRuleLocator::days`], [`RRuleLocator::weekdays`] |
//! | `mdates.YearLocator`, `AutoDateLocator`        | [`YearLocator`], [`AutoDateLocator`]             |
//! | `mdates.DateFormatter("%b %Y")`, `ConciseDateFormatter` | [`DateFormatter::new`], [`ConciseDateFormatter`] |
//! | `mdates.date2num`, `num2date`, `datestr2num`   | [`date2num`], [`num2date`], [`datestr2num`]      |
//! | `mdates.set_epoch`, `get_epoch`                | [`set_epoch`], [`get_epoch`]                     |
//!
//! # Limits, margins and aspect
//!
//! ```python
//! ax.plot([0, 10], [0, 5])
//! ax.set_xlim(left=2)            # the right limit stays
//! ax.set_ylim(5, 0)              # an inverted axis
//! assert ax.get_xlim()[0] == 2 and ax.yaxis_inverted()
//! ax.margins(0.1, 0)
//! ax.autoscale(axis="x", tight=True)
//! assert ax.get_xlim() == (0, 10)
//! ax.set_aspect("equal", adjustable="datalim")
//! assert ax.get_aspect() == 1
//! ```
//!
//! ```
//! use plt::axes::{Adjustable, Limits};
//! use plt::prelude::*;
//! use plt::rc::Aspect;
//! use plt::ticker::AxisName;
//!
//! let ax = Figure::new().subplot();
//! ax.plot([0.0, 10.0], [0.0, 5.0])?;
//! let (_, right) = ax.xlim();
//! ax.set_limits_with(AxisName::X, Limits { low: Some(2.0), high: None, auto: None })?;
//! ax.set_ylim(5.0, 0.0)?;
//! assert_eq!(ax.xlim(), (2.0, right));
//! assert!(ax.yaxis().inverted());
//!
//! ax.set_margins(0.1, 0.0)?;
//! ax.autoscale(Some(AxisName::X), Some(true));
//! assert_eq!(ax.xlim(), (0.0, 10.0));
//!
//! ax.set_aspect(Aspect::Equal)?;
//! ax.set_adjustable(Adjustable::Datalim)?;
//! assert_eq!(ax.aspect(), Aspect::Ratio(1.0), "as in matplotlib, equal reads back as 1");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Scales
//!
//! ```python
//! ax.set_xscale("log", base=2)
//! ax.set_yscale("symlog", linthresh=1)
//! assert ax.get_xscale() == "log"
//! ax.set_yscale("function", functions=(np.sqrt, np.square))
//! ```
//!
//! A scale name is a [`ScaleName`]; matplotlib's strings convert to it,
//! and an unknown name is refused when the scale is set.
//!
//! ```
//! use plt::prelude::*;
//! use plt::scale::{FuncTransform, ScaleName, ScaleOptions};
//! use plt::ticker::AxisName;
//!
//! let ax = Figure::new().subplot();
//! let base2 = ScaleOptions { base: Some(2.0), ..ScaleOptions::default() };
//! ax.set_scale_with(AxisName::X, ScaleName::Log, base2)?;
//! let symlog = ScaleOptions { linthresh: Some(1.0), ..ScaleOptions::default() };
//! ax.set_scale_with(AxisName::Y, "symlog", symlog)?;
//! assert_eq!(ax.xscale(), ScaleName::Log);
//! assert_eq!(ax.yscale().to_string(), "symlog");
//!
//! let root = ScaleOptions {
//!     functions: Some(FuncTransform::new(f64::sqrt, |v| v * v)),
//!     ..ScaleOptions::default()
//! };
//! ax.set_scale_with(AxisName::Y, ScaleName::Function, root)?;
//! assert!(ax.set_xscale("loglog").is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Ticks, locators and formatters
//!
//! ```python
//! ax.set_xticks([0, 1, 2], ["zero", "one", "two"])
//! ax.set_ylim(0, 1)
//! ax.yaxis.set_major_locator(MultipleLocator(0.5))
//! ax.yaxis.set_major_formatter(FormatStrFormatter("%.1f"))
//! ax.yaxis.set_minor_locator(AutoMinorLocator())
//! ax.tick_params(axis="x", labelrotation=45, labelsize=8, direction="in")
//! # like the locator, the tick lists reach one step beyond the view
//! assert list(ax.get_yticks()) == [-0.5, 0, 0.5, 1, 1.5]
//! assert [t.get_text() for t in ax.get_yticklabels()] == ["-0.5", "0.0", "0.5", "1.0", "1.5"]
//! ```
//!
//! ```
//! use plt::axes::{TickDirection, TickParams};
//! use plt::fonts::FontSize;
//! use plt::prelude::*;
//! use plt::ticker::{AutoMinorLocator, AxisName, FormatStrFormatter, MultipleLocator};
//!
//! let ax = Figure::new().subplot();
//! ax.set_xticks([0.0, 1.0, 2.0]).set_xticklabels(["zero", "one", "two"])?;
//! ax.set_ylim(0.0, 1.0)?;
//! ax.yaxis()
//!     .set_major_locator(MultipleLocator::new(0.5)?)
//!     .set_major_formatter(FormatStrFormatter::new("%.1f")?)
//!     .set_minor_locator(AutoMinorLocator::new(None));
//! ax.tick_params(TickParams {
//!     axis: Some(AxisName::X),
//!     labelrotation: Some(45.0),
//!     labelsize: Some(FontSize::Points(8.0)),
//!     direction: Some(TickDirection::In),
//!     ..TickParams::default()
//! });
//! assert_eq!(ax.yaxis().ticklocs(false)?, [-0.5, 0.0, 0.5, 1.0, 1.5]);
//! assert_eq!(ax.yaxis().ticklabels(false)?, ["-0.5", "0.0", "0.5", "1.0", "1.5"]);
//! assert_eq!(ax.xaxis().ticklabels(false)?, ["zero", "one", "two"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Grids, labels and spines
//!
//! ```python
//! ax.grid(True, which="major", axis="y", color="0.8", linestyle="--")
//! ax.set_xlabel("time", loc="left", labelpad=8)
//! ax.spines[["top", "right"]].set_visible(False)
//! ax.spines["left"].set_position(("outward", 10))
//! ax.spines["bottom"].set_position("zero")
//! ```
//!
//! ```
//! use plt::axes::{GridOptions, LabelLoc, TickLevel, XLabelLoc};
//! use plt::prelude::*;
//! use plt::spines::{SpinePosition, SpineSide};
//! use plt::ticker::AxisName;
//!
//! let ax = Figure::new().subplot();
//! let options = GridOptions {
//!     which: TickLevel::Major,
//!     axis: Some(AxisName::Y),
//!     color: Some("0.8".into()),
//!     linestyle: Some(LineStyle::Dashed),
//!     ..GridOptions::default()
//! };
//! ax.grid(false).grid_with(true, options);
//! assert!(ax.has_grid(AxisName::Y, true));
//! assert!(!ax.has_grid(AxisName::X, true));
//!
//! ax.set_xlabel("time");
//! ax.xaxis().set_label_loc(XLabelLoc::Left).set_label_pad(8.0);
//! assert_eq!(ax.label(AxisName::X), "time");
//! assert_eq!(ax.xaxis().label_loc(), LabelLoc::Start, "the start of any axis");
//!
//! for side in [SpineSide::Top, SpineSide::Right] {
//!     ax.spines().get(side).set_visible(false);
//! }
//! ax.spines().left.set_position(SpinePosition::Outward(10.0));
//! ax.spines().bottom.set_position(SpinePosition::Data(0.0));
//! assert!(!ax.spines().top.visible());
//! assert_eq!(ax.spines().left.position(), SpinePosition::Outward(10.0));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Twin, secondary and shared axes
//!
//! ```python
//! twin = ax.twinx()
//! twin.set_ylabel("°F")
//! sec = ax.secondary_xaxis("top", functions=(np.deg2rad, np.rad2deg))
//! other = fig.add_subplot(2, 1, 2)
//! other.sharex(ax)
//! ```
//!
//! ```
//! use plt::axes::SecondaryLocation;
//! use plt::figure::Figure;
//! use plt::scale::FuncTransform;
//! use plt::spines::SpineSide;
//! use plt::ticker::AxisName;
//!
//! let fig = Figure::new();
//! let ax = fig.add_subplot(2, 1, 1)?;
//! ax.set_xlim(0.0, 180.0)?;
//! let twin = ax.twinx()?;
//! twin.set_ylabel("°F");
//! assert_eq!(twin.xlim(), ax.xlim(), "a twin shares the x axis");
//!
//! let radians = FuncTransform::new(f64::to_radians, f64::to_degrees);
//! let sec = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(radians))?;
//! assert!((sec.xlim().1 - std::f64::consts::PI).abs() < 1e-12);
//!
//! let other = fig.add_subplot(2, 1, 2)?;
//! other.share_x(&ax)?;
//! assert!(ax.shared_axes(AxisName::X).contains(&other));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Dates
//!
//! ```python
//! ax.plot([datetime(2024, 1, 1), datetime(2024, 6, 1)], [0, 1])
//! ax.xaxis.set_major_locator(mdates.MonthLocator(bymonthday=1, interval=2))
//! ax.xaxis.set_major_formatter(mdates.DateFormatter("%b %Y"))
//! ax.set_xlim(mdates.datestr2num("2024-01-01"), mdates.datestr2num("2024-06-01"))
//! assert mdates.datestr2num("2024-01-01") == 19723
//! assert [t.get_text() for t in ax.get_xticklabels()] == ["Feb 2024", "Apr 2024", "Jun 2024"]
//! ```
//!
//! ```
//! use plt::dates::{DateFormatter, DateTime, RRuleLocator, datestr2num};
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let days = [DateTime::new(2024, 1, 1, 0, 0, 0, 0)?, DateTime::new(2024, 6, 1, 0, 0, 0, 0)?];
//! ax.plot(&days[..], [0.0, 1.0])?;
//! ax.xaxis()
//!     .set_major_locator(RRuleLocator::months(&[], &[1], 2))
//!     .set_major_formatter(DateFormatter::new("%b %Y"));
//! ax.set_xlim(datestr2num("2024-01-01")?, datestr2num("2024-06-01")?)?;
//! assert_eq!(datestr2num("2024-01-01")?, 19723.0);
//! // The rule starts a step before the view, as matplotlib's does.
//! assert_eq!(ax.xaxis().ticklabels(false)?, ["Feb 2024", "Apr 2024", "Jun 2024"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Axes`]: crate::axes::Axes
//! [`Axis`]: crate::axes::Axis
//! [`AxisName`]: crate::ticker::AxisName
//! [`Limits`]: crate::axes::Limits
//! [`AxisMode`]: crate::axes::AxisMode
//! [`Aspect`]: crate::rc::Aspect
//! [`AxisBelow`]: crate::rc::AxisBelow
//! [`ScaleOptions`]: crate::scale::ScaleOptions
//! [`ScaleName`]: crate::scale::ScaleName
//! [`ScaleOptions::functions`]: field@crate::scale::ScaleOptions::functions
//! [`FuncTransform`]: crate::scale::FuncTransform
//! [`Scale`]: crate::scale::Scale
//! [`Locator`]: crate::ticker::Locator
//! [`Formatter`]: crate::ticker::Formatter
//! [`TickParams`]: crate::axes::TickParams
//! [`GridOptions`]: crate::axes::GridOptions
//! [`TicklabelFormat`]: crate::axes::TicklabelFormat
//! [`MaxNLocator`]: crate::ticker::MaxNLocator
//! [`AutoMinorLocator`]: crate::ticker::AutoMinorLocator
//! [`LogLocator`]: crate::ticker::LogLocator
//! [`LogLocator::new`]: crate::ticker::LogLocator::new
//! [`LogScale::new`]: crate::scale::LogScale::new
//! [`LogBase`]: crate::ticker::LogBase
//! [`LabelLoc`]: crate::axes::LabelLoc
//! [`XLabelLoc::Left`]: crate::axes::XLabelLoc::Left
//! [`YLabelLoc::Top`]: crate::axes::YLabelLoc::Top
//! [`Axis::set_in_layout`]: crate::axes::Axis::set_in_layout
//! [`FixedLocator`]: crate::ticker::FixedLocator
//! [`NullLocator`]: crate::ticker::NullLocator
//! [`FormatStrFormatter`]: crate::ticker::FormatStrFormatter
//! [`FuncFormatter`]: crate::ticker::FuncFormatter
//! [`PercentFormatter`]: crate::ticker::PercentFormatter
//! [`EngFormatter`]: crate::ticker::EngFormatter
//! [`NullFormatter`]: crate::ticker::NullFormatter
//! [`Spines`]: crate::spines::Spines
//! [`Spine::set_position`]: crate::spines::Spine::set_position
//! [`Spine::set_bounds`]: crate::spines::Spine::set_bounds
//! [`Spine::set_bound_ends`]: crate::spines::Spine::set_bound_ends
//! [`Spine::bounds`]: crate::spines::Spine::bounds
//! [`SpinePosition`]: crate::spines::SpinePosition
//! [`RRuleLocator::months`]: crate::dates::RRuleLocator::months
//! [`RRuleLocator::days`]: crate::dates::RRuleLocator::days
//! [`RRuleLocator::weekdays`]: crate::dates::RRuleLocator::weekdays
//! [`YearLocator`]: crate::dates::YearLocator
//! [`AutoDateLocator`]: crate::dates::AutoDateLocator
//! [`DateFormatter::new`]: crate::dates::DateFormatter::new
//! [`ConciseDateFormatter`]: crate::dates::ConciseDateFormatter
//! [`date2num`]: crate::dates::date2num
//! [`num2date`]: crate::dates::num2date
//! [`datestr2num`]: crate::dates::datestr2num
//! [`set_epoch`]: crate::dates::set_epoch
//! [`get_epoch`]: crate::dates::get_epoch
//! [`Axes::xaxis`]: crate::axes::Axes::xaxis
//! [`Axes::yaxis`]: crate::axes::Axes::yaxis
//! [`Axes::set_xlim`]: crate::axes::Axes::set_xlim
//! [`Axes::xlim`]: crate::axes::Axes::xlim
//! [`Axes::set_limits_with`]: crate::axes::Axes::set_limits_with
//! [`Axes::set_bound`]: crate::axes::Axes::set_bound
//! [`Axes::bound`]: crate::axes::Axes::bound
//! [`Axes::set_margins`]: crate::axes::Axes::set_margins
//! [`Axes::set_margin`]: crate::axes::Axes::set_margin
//! [`Axes::autoscale`]: crate::axes::Axes::autoscale
//! [`Axes::set_autoscale_on`]: crate::axes::Axes::set_autoscale_on
//! [`Axes::autoscale_view`]: crate::axes::Axes::autoscale_view
//! [`Axes::relim`]: crate::axes::Axes::relim
//! [`Axes::axis_mode`]: crate::axes::Axes::axis_mode
//! [`Axes::set_axis_off`]: crate::axes::Axes::set_axis_off
//! [`Axes::set_aspect`]: crate::axes::Axes::set_aspect
//! [`Axes::set_adjustable`]: crate::axes::Axes::set_adjustable
//! [`Axes::set_box_aspect`]: crate::axes::Axes::set_box_aspect
//! [`Axes::set_xscale`]: crate::axes::Axes::set_xscale
//! [`Axes::xscale`]: crate::axes::Axes::xscale
//! [`Axes::set_scale_with`]: crate::axes::Axes::set_scale_with
//! [`Axes::set_scale`]: crate::axes::Axes::set_scale
//! [`Axes::set_xticks`]: crate::axes::Axes::set_xticks
//! [`Axes::set_xticklabels`]: crate::axes::Axes::set_xticklabels
//! [`Axes::minorticks_on`]: crate::axes::Axes::minorticks_on
//! [`Axes::tick_params`]: crate::axes::Axes::tick_params
//! [`Axes::ticklabel_format`]: crate::axes::Axes::ticklabel_format
//! [`Axes::locator_params`]: crate::axes::Axes::locator_params
//! [`Axes::grid`]: crate::axes::Axes::grid
//! [`Axes::grid_with`]: crate::axes::Axes::grid_with
//! [`Axes::set_axisbelow`]: crate::axes::Axes::set_axisbelow
//! [`Axes::set_xlabel`]: crate::axes::Axes::set_xlabel
//! [`Axes::label`]: crate::axes::Axes::label
//! [`Axes::twinx`]: crate::axes::Axes::twinx
//! [`Axes::twiny`]: crate::axes::Axes::twiny
//! [`Axes::secondary_xaxis`]: crate::axes::Axes::secondary_xaxis
//! [`Axes::share_x`]: crate::axes::Axes::share_x
//! [`Axes::shared_axes`]: crate::axes::Axes::shared_axes
//! [`Axes::label_outer`]: crate::axes::Axes::label_outer
//! [`Axes::xaxis_date`]: crate::axes::Axes::xaxis_date
//! [`Axis::set_inverted`]: crate::axes::Axis::set_inverted
//! [`Axis::inverted`]: crate::axes::Axis::inverted
//! [`Axis::set_minor_ticks`]: crate::axes::Axis::set_minor_ticks
//! [`Axis::ticklocs`]: crate::axes::Axis::ticklocs
//! [`Axis::ticklabels`]: crate::axes::Axis::ticklabels
//! [`Axis::set_major_locator`]: crate::axes::Axis::set_major_locator
//! [`Axis::set_major_formatter`]: crate::axes::Axis::set_major_formatter
//! [`Axis::tick_to`]: crate::axes::Axis::tick_to
//! [`Axis::set_ticks_position`]: crate::axes::Axis::set_ticks_position
//! [`Axis::set_label_loc`]: crate::axes::Axis::set_label_loc
//! [`Axis::set_label_pad`]: crate::axes::Axis::set_label_pad
//! [`Axis::set_label_fontsize`]: crate::axes::Axis::set_label_fontsize
//! [`Axis::set_label_color`]: crate::axes::Axis::set_label_color
//! [`Axis::set_label_position`]: crate::axes::Axis::set_label_position
//! [`Axis::set_label_coords`]: crate::axes::Axis::set_label_coords
//! [`Axis::set_label_halign`]: crate::axes::Axis::set_label_halign
//! [`Axis::set_color`]: crate::axes::Axis::set_color
//! [`Axis::axis_date`]: crate::axes::Axis::axis_date
//! [`Axis::convert_units`]: crate::axes::Axis::convert_units
//! [`Axis::set_units`]: crate::axes::Axis::set_units
//! [`Axis::units`]: crate::axes::Axis::units
//! [`Axis::update_units`]: crate::axes::Axis::update_units
//! [`Axis::set_converter`]: crate::axes::Axis::set_converter
//! [`Axis::converter`]: crate::axes::Axis::converter
//! [`DateConverter::Concise`]: crate::units::DateConverter::Concise
//! [`UnitConverter::DatesBy`]: crate::units::UnitConverter::DatesBy
//! [`UnitConverter::Dates`]: crate::units::UnitConverter::Dates
//! [`UnitConverter::is_dates`]: crate::units::UnitConverter::is_dates
//! [`Unit`]: crate::units::Unit
//! [`ConversionInterface`]: crate::units::ConversionInterface
//! [`Convertible`]: crate::units::Convertible
//! [`UnitAxisInfo`]: crate::units::UnitAxisInfo
//! [`UnitAxis`]: crate::units::UnitAxis
//! [`ConversionError`]: crate::units::ConversionError
//! [`Axes::has_data`]: crate::axes::Axes::has_data
//! [`Axis::set_offset_position`]: crate::axes::Axis::set_offset_position
//! [`Axis::offset_position`]: crate::axes::Axis::offset_position
//! [`Axis::set_picker`]: crate::axes::Axis::set_picker
//! [`Axis::contains`]: crate::axes::Axis::contains
//! [`Axis::set_pickradius`]: crate::axes::Axis::set_pickradius
//! [`Axis::pickradius`]: crate::axes::Axis::pickradius
//! [`PickEvent::axis`]: crate::events::PickEvent::axis
//! [`LockableBbox`]: crate::transforms::LockableBbox
//! [`LockableBbox::bbox`]: crate::transforms::LockableBbox::bbox
