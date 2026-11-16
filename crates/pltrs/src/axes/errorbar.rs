//! Error bars (matplotlib's `errorbar`).

use std::f64::consts::FRAC_PI_2;
use std::str::FromStr;

use super::broadcast::broadcast_flags;
use super::{Axes, AxesState, ErrorSize, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist};
use crate::colors::Color;
use crate::lines::{
    FormatString, FormatStringError, Line2D, LineArtist, LineProps, LineStyle, MarkEvery,
    PolarMarkerRotation,
};
use crate::markers::{Marker, MarkerSymbol};
use crate::orientation::Orientation;
use crate::ticker::AxisName;
use crate::transforms::Transform;
use crate::units::{AxisValue, AxisValues, CustomValues, UnitHandle, UnitSources};

/// How [`Axes::errorbar`] draws the data line (matplotlib's `fmt`);
/// matplotlib's strings parse with [`str::parse`].
///
/// ```
/// use pltrs::axes::ErrorbarFormat;
/// assert_eq!("".parse(), Ok(ErrorbarFormat::Default));
/// assert_eq!("none".parse(), Ok(ErrorbarFormat::None));
/// assert!(matches!("rs--".parse(), Ok(ErrorbarFormat::Format(_))));
/// assert!("not a format".parse::<ErrorbarFormat>().is_err());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ErrorbarFormat {
    /// The line in its default style (`fmt=""`).
    #[default]
    Default,
    /// No data line, the error bars only (`fmt="none"`).
    None,
    /// The style of a format string such as `"o"` or `"rs--"`.
    Format(FormatString),
}

impl ErrorbarFormat {
    /// Whether the data line is drawn.
    pub(crate) fn draws_line(&self) -> bool {
        *self != Self::None
    }

    /// The properties the format gives the data line: matplotlib parses
    /// `"none"` as the colour `none`.
    pub(crate) fn line_props(self) -> LineProps {
        match self {
            Self::Default => LineProps::default(),
            Self::None => LineProps::default().color("none"),
            Self::Format(format) => format.into(),
        }
    }
}

impl From<FormatString> for ErrorbarFormat {
    fn from(format: FormatString) -> Self {
        Self::Format(format)
    }
}

impl FromStr for ErrorbarFormat {
    type Err = FormatStringError;

    /// Parses `""`, `"none"` (in any case) or a format string.
    fn from_str(fmt: &str) -> Result<Self, FormatStringError> {
        if fmt.is_empty() {
            Ok(Self::Default)
        } else if fmt.eq_ignore_ascii_case("none") {
            Ok(Self::None)
        } else {
            fmt.parse().map(Self::Format)
        }
    }
}

/// Options of [`Axes::errorbar`] (matplotlib's `errorbar` keywords).
#[derive(Clone, Debug, Default)]
pub struct ErrorbarOptions {
    /// Horizontal error sizes.
    pub xerr: Option<ErrorSize>,
    /// Vertical error sizes.
    pub yerr: Option<ErrorSize>,
    /// Style of the data line, or no line.
    pub fmt: ErrorbarFormat,
    /// Properties of the data line; its colour, width, alpha, z-order and
    /// label also apply to the error bars and the container.
    pub line: LineProps,
    /// Colour of the error bars (default: the data line's colour).
    pub ecolor: Option<Color>,
    /// Width of the error bars in points.
    pub elinewidth: Option<f64>,
    /// Line style of the error bars.
    pub elinestyle: Option<LineStyle>,
    /// Length of the caps in points (default `errorbar.capsize`).
    pub capsize: Option<f64>,
    /// Thickness of the caps in points.
    pub capthick: Option<f64>,
    /// Draw the bars above the data line instead of below.
    pub barsabove: bool,
    /// Only upper y errors, marked with an upward caret (one or per point).
    pub lolims: Vec<bool>,
    /// Only lower y errors, marked with a downward caret.
    pub uplims: Vec<bool>,
    /// Only right x errors, marked with a rightward caret.
    pub xlolims: Vec<bool>,
    /// Only left x errors, marked with a leftward caret.
    pub xuplims: Vec<bool>,
    /// Which points get error bars (`None` = all).
    pub errorevery: Option<MarkEvery>,
}

/// The artists of an error bar plot (matplotlib's `ErrorbarContainer`).
#[derive(Clone, Debug)]
pub struct ErrorbarContainer {
    /// The data line, unless the format was `"none"`.
    pub data_line: Option<Line2D>,
    /// Caps and limit markers, those of horizontal bars first.
    pub caplines: Vec<Line2D>,
    /// The error bars, horizontal ones first.
    pub barlinecols: Vec<Collection>,
    /// Whether there are horizontal error bars.
    pub has_xerr: bool,
    /// Whether there are vertical error bars.
    pub has_yerr: bool,
    /// Legend label.
    pub label: Option<String>,
}

pltrs_core::archive_struct!(ErrorbarContainer {
    data_line,
    caplines,
    barlinecols,
    has_xerr,
    has_yerr,
    label
});

/// Styles shared by the bars and caps of one errorbar call.
struct BarStyle {
    /// The data line's style without its marker and line-style parts
    /// (matplotlib's `base_style`).
    base: LineProps,
    ecolor: Color,
    elinewidth: Option<f64>,
    elinestyle: Option<LineStyle>,
    capsize: f64,
    capthick: Option<f64>,
}

/// The points of a cap or limit-marker line, and the user values they
/// came from (matplotlib's caps are lines of the values with units).
struct CapPoints {
    x: Vec<f64>,
    y: Vec<f64>,
    sources: UnitSources,
}

/// Values along one axis of the points of an error bar plot: the numbers
/// drawn and the plotting values they came from.
#[derive(Clone, Copy)]
struct Series<'a> {
    numbers: &'a [f64],
    values: &'a [AxisValue],
}

impl Series<'_> {
    /// The numbers and the user values where `mask` is set.
    fn masked(self, mask: &[bool]) -> (Vec<f64>, Option<CustomValues>) {
        let values = self
            .values
            .iter()
            .zip(mask)
            .filter_map(|(value, &keep)| keep.then_some(value));
        (masked(self.numbers, mask), CustomValues::join(values))
    }
}

/// The cap points at `x` and `y` where `mask` is set.
fn cap_points(x: Series<'_>, y: Series<'_>, mask: &[bool]) -> CapPoints {
    let ((x, x_sources), (y, y_sources)) = (x.masked(mask), y.masked(mask));
    CapPoints {
        x,
        y,
        sources: UnitSources {
            x: x_sources,
            y: y_sources,
        },
    }
}

impl BarStyle {
    /// A cap or limit-marker line with `marker` (matplotlib's
    /// `eb_cap_style`).
    fn cap(&self, axes: &Axes, points: CapPoints, marker: MarkerSymbol) -> LineArtist {
        let mut props = self
            .base
            .clone()
            .linestyle(LineStyle::None)
            .marker(Marker::Symbol(marker))
            .markeredgecolor(self.ecolor.clone());
        if self.capsize > 0.0 {
            props.markersize = Some(2.0 * self.capsize);
        }
        props.markeredgewidth = self.capthick;
        let mut line = LineArtist::new(points.x, points.y, crate::rc::params());
        line.unit_sources = points.sources;
        props.apply(&mut line);
        line.base_mut().transform = Some(self.transform(axes));
        line
    }

    /// The coordinates of the bars and the caps: those the keywords give
    /// the data line, as matplotlib's `eb_lines_style` and `eb_cap_style`
    /// take them, or the data's.
    fn transform(&self, axes: &Axes) -> Transform {
        (self.base.base.transform.clone()).unwrap_or_else(|| axes.trans_data())
    }

    /// The cap or limit-marker lines with `marker` at `(x, y)` for error
    /// bars along `axis`: one line, or on polar axes one line per point
    /// whose `_` marker is turned perpendicular to its bar (matplotlib
    /// draws every polar cap with the `_` marker, whatever `marker` is).
    fn caps(
        &self,
        axes: &Axes,
        points: CapPoints,
        marker: MarkerSymbol,
        axis: AxisName,
    ) -> Vec<LineArtist> {
        if !axes
            .raw()
            .with::<AxesState, _>(|state| state.polar().is_some())
        {
            return vec![self.cap(axes, points, marker)];
        }
        let extra = if axis == AxisName::Y { FRAC_PI_2 } else { 0.0 };
        let CapPoints { x, y, sources } = points;
        x.into_iter()
            .zip(y)
            .enumerate()
            .map(|(i, (theta, r))| {
                let point = CapPoints {
                    x: vec![theta],
                    y: vec![r],
                    sources: sources.item(i),
                };
                let mut line = self.cap(axes, point, MarkerSymbol::HLine);
                line.marker_rotation = Some(PolarMarkerRotation { theta, extra });
                line
            })
            .collect()
    }

    /// Styles a family of error bars (matplotlib's `eb_lines_style`).
    fn style_bars(&self, bars: &Collection) {
        bars.set_edgecolor(self.ecolor.clone());
        if let Some(width) = self.elinewidth.or(self.base.linewidth) {
            bars.set_linewidths(vec![width]);
        }
        if let Some(style) = &self.elinestyle {
            bars.set_linestyles(vec![style.clone()]);
        }
        if let Some(antialiased) = self.base.antialiased {
            bars.set_antialiased(vec![antialiased]);
        }
        let base = &self.base.base;
        bars.set_alpha(base.alpha)
            .set_zorder(base.zorder.unwrap_or(2.0))
            .set_label("_nolegend_");
    }
}

/// The error bars along one axis.
struct Direction {
    axis: AxisName,
    err: ErrorSize,
    lolims: Vec<bool>,
    uplims: Vec<bool>,
}

impl Axes {
    /// Points with error bars (matplotlib's `errorbar`). The bars end at
    /// the values less and plus their errors in the units of the data, as
    /// matplotlib computes them (`x + -1 * xerr`): numbers add to numbers,
    /// durations to dates, and user errors to user values as their
    /// [`Convertible::scale`](crate::units::Convertible::scale) and
    /// [`add_delta`](crate::units::Convertible::add_delta) say; the ends
    /// convert on the axis. Like matplotlib's, the data line and the caps
    /// keep their user values, which convert again when the axis changes
    /// unit, and the bars do not.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::ErrorbarOptions;
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8])?;
    /// let options = ErrorbarOptions { yerr: Some(0.5.into()), fmt: "o".parse()?, ..Default::default() };
    /// let bars = ax.errorbar([1.0, 2.0], [3.0, 4.0], options)?;
    /// assert_eq!(bars.barlinecols.len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length, an error or limit list does not
    /// broadcast, errors are negative, or errors do not scale or add to
    /// the values in their units (a number to a date, a duration to a
    /// number, ...).
    pub fn errorbar(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        mut options: ErrorbarOptions,
    ) -> Result<ErrorbarContainer, PlotError> {
        let (x_values, y_values) = (x.into(), y.into());
        let x = self.xaxis().convert_plot_units(x_values.clone());
        let y = self.yaxis().convert_plot_units(y_values.clone());
        let (x, y) = (x.as_slice(), y.as_slice());
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let n = x.len();
        let every = match &options.errorevery {
            Some(every) => {
                let mut mask = vec![false; n];
                for i in every.indices(n).map_err(PlotError::ErrorEvery)? {
                    mask[i] = true;
                }
                mask
            }
            None => vec![true; n],
        };
        let (has_xerr, has_yerr) = (options.xerr.is_some(), options.yerr.is_some());
        let directions = Self::error_directions(&mut options, n)?;
        let mut kwargs = options.line;
        let label = kwargs.base.label.take();
        let zorder = *kwargs.base.zorder.get_or_insert(2.0);
        kwargs.base.label = Some("_nolegend_".into());
        let hidden = !options.fmt.draws_line();
        let format = options.fmt.line_props();
        let mut data = self.cycled_line(x.to_vec(), y.to_vec(), format.overlay(kwargs.clone()));
        data.unit_sources = UnitSources::of(&x_values, &y_values);
        let color = if hidden {
            kwargs.color.clone().unwrap_or_else(|| Color::from("C0"))
        } else {
            data.color.clone()
        };
        data.base_mut().zorder = if options.barsabove {
            zorder - 0.1
        } else {
            zorder + 0.1
        };
        let data_line = (!hidden).then(|| self.add_line_artist(data));

        let rc = crate::rc::params();
        let style = BarStyle {
            base: without_marker_style(kwargs.clone().color(color.clone())),
            ecolor: options.ecolor.unwrap_or(color),
            elinewidth: options.elinewidth.or(rc.errorbar_elinewidth),
            elinestyle: options.elinestyle,
            capsize: options.capsize.unwrap_or(rc.errorbar_capsize),
            capthick: kwargs
                .markeredgewidth
                .or(options.capthick)
                .or(rc.errorbar_capthick),
        };
        let item = |values: &AxisValues| (0..n).filter_map(|i| values.get(i)).collect::<Vec<_>>();
        let (x_items, y_items) = (item(&x_values), item(&y_values));
        let x = Series {
            numbers: x,
            values: &x_items,
        };
        let y = Series {
            numbers: y,
            values: &y_items,
        };
        let mut barlinecols = Vec::new();
        let mut caps = Vec::new();
        for direction in &directions {
            let (bars, new_caps) = self.error_direction(direction, x, y, &every, &style)?;
            barlinecols.push(bars);
            caps.extend(new_caps);
        }
        let caplines = caps
            .into_iter()
            .map(|line| self.add_line_artist(line))
            .collect();
        self.raw()
            .with_mut::<super::AxesState, _>(|axes| axes.request_autoscale(true, true));
        let container = ErrorbarContainer {
            data_line,
            caplines,
            barlinecols,
            has_xerr,
            has_yerr,
            label,
        };
        self.add_container(container.clone());
        Ok(container)
    }

    /// The requested error directions, horizontal first, taken out of
    /// `options` with their limit flags broadcast to `n` points.
    fn error_directions(
        options: &mut ErrorbarOptions,
        n: usize,
    ) -> Result<Vec<Direction>, PlotError> {
        let mut directions = Vec::new();
        if let Some(err) = options.xerr.take() {
            directions.push(Direction {
                axis: AxisName::X,
                err,
                lolims: broadcast_flags("xlolims", &options.xlolims, n)?,
                uplims: broadcast_flags("xuplims", &options.xuplims, n)?,
            });
        }
        if let Some(err) = options.yerr.take() {
            directions.push(Direction {
                axis: AxisName::Y,
                err,
                lolims: broadcast_flags("lolims", &options.lolims, n)?,
                uplims: broadcast_flags("uplims", &options.uplims, n)?,
            });
        }
        Ok(directions)
    }

    /// The bars, caps and limit markers along one axis.
    fn error_direction(
        &self,
        direction: &Direction,
        x: Series<'_>,
        y: Series<'_>,
        every: &[bool],
        style: &BarStyle,
    ) -> Result<(Collection, Vec<LineArtist>), PlotError> {
        let along_x = direction.axis == AxisName::X;
        let (dep, indep) = if along_x { (x, y) } else { (y, x) };
        let argument = if along_x { "xerr" } else { "yerr" };
        let (low_values, high_values) =
            direction
                .err
                .ends(argument, dep.values, &direction.lolims, &direction.uplims)?;
        let axis = self.axis_of(direction.axis);
        let convert = |ends: &[AxisValue]| -> Vec<f64> {
            ends.iter()
                .map(|end| axis.convert_without_update(end))
                .collect()
        };
        let (low, high) = (convert(&low_values), convert(&high_values));
        let low = Series {
            numbers: &low,
            values: &low_values,
        };
        let high = Series {
            numbers: &high,
            values: &high_values,
        };
        let orientation = if along_x {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        };
        let bars = self.segments_in(
            orientation,
            &masked(indep.numbers, every),
            &masked(low.numbers, every),
            &masked(high.numbers, every),
            style.transform(self),
        )?;
        style.style_bars(&bars);
        if along_x
            && self
                .raw()
                .with::<AxesState, _>(|state| state.polar().is_some())
        {
            // Angular error bars bend along their circle on polar axes.
            bars.raw().with_mut::<CollectionArtist, _>(|bars| {
                for path in &mut bars.paths {
                    *path = std::mem::take(path).with_interpolation_steps(2);
                }
            });
        }
        let caps = self.error_caps(direction, (x, y), (low, high), every, style);
        Ok((bars, caps))
    }

    /// The caps at the ends `low` and `high` of the bars of the points
    /// `(x, y)` along one axis, and the markers of the points that are
    /// limits.
    fn error_caps(
        &self,
        direction: &Direction,
        (x, y): (Series<'_>, Series<'_>),
        (low, high): (Series<'_>, Series<'_>),
        every: &[bool],
        style: &BarStyle,
    ) -> Vec<LineArtist> {
        let along_x = direction.axis == AxisName::X;
        let indep = if along_x { y } else { x };
        // Lines take `(x, y)`; the error values belong on the error axis.
        let place = |positions: Series<'_>, ends: Series<'_>, mask: &[bool]| {
            if along_x {
                cap_points(ends, positions, mask)
            } else {
                cap_points(positions, ends, mask)
            }
        };
        let (cap_marker, lo_marker, hi_marker) = if along_x {
            (
                MarkerSymbol::VLine,
                MarkerSymbol::CaretRightBase,
                MarkerSymbol::CaretLeftBase,
            )
        } else {
            (
                MarkerSymbol::HLine,
                MarkerSymbol::CaretUpBase,
                MarkerSymbol::CaretDownBase,
            )
        };
        let mut caps = Vec::new();
        let nolims: Vec<bool> = (0..every.len())
            .map(|i| !(direction.lolims[i] || direction.uplims[i]))
            .collect();
        if style.capsize > 0.0 && nolims.contains(&true) {
            let mask: Vec<bool> = nolims.iter().zip(every).map(|(&a, &b)| a && b).collect();
            for ends in [low, high] {
                let points = place(indep, ends, &mask);
                caps.extend(style.caps(self, points, cap_marker, direction.axis));
            }
        }
        for (upper_end, lims, ends) in [
            (false, &direction.lolims, high),
            (true, &direction.uplims, low),
        ] {
            if !lims.contains(&true) {
                continue;
            }
            // Read only for limit markers, as matplotlib reads it: reading
            // the view applies the autoscaling pending.
            let inverted = self.axis_of(direction.axis).inverted();
            let marker = if inverted ^ upper_end {
                hi_marker
            } else {
                lo_marker
            };
            let mask: Vec<bool> = lims.iter().zip(every).map(|(&a, &b)| a && b).collect();
            let points = place(indep, ends, &mask);
            caps.extend(style.caps(self, points, marker, direction.axis));
            if style.capsize > 0.0 {
                let points = cap_points(x, y, &mask);
                caps.extend(style.caps(self, points, cap_marker, direction.axis));
            }
        }
        caps
    }
}

/// `values` where `mask` is set (numpy's boolean indexing).
fn masked(values: &[f64], mask: &[bool]) -> Vec<f64> {
    values
        .iter()
        .zip(mask)
        .filter_map(|(&value, &keep)| keep.then_some(value))
        .collect()
}

/// `props` without the marker and line-style parts, which do not apply to
/// error bars and caps (matplotlib's `base_style` of `errorbar`).
pub(crate) fn without_marker_style(props: LineProps) -> LineProps {
    LineProps {
        marker: None,
        markersize: None,
        markerfacecolor: None,
        markerfacecoloralt: None,
        markeredgewidth: None,
        markeredgecolor: None,
        markevery: None,
        linestyle: None,
        fillstyle: None,
        drawstyle: None,
        dash_capstyle: None,
        dash_joinstyle: None,
        solid_capstyle: None,
        solid_joinstyle: None,
        ..props
    }
}
