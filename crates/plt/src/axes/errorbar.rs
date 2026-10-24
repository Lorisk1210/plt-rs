//! Error bars (matplotlib's `errorbar`).

use std::f64::consts::FRAC_PI_2;

use super::broadcast::{broadcast, broadcast_flags};
use super::{Axes, AxesState, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist};
use crate::colors::Color;
use crate::dates::TimeDelta;
use crate::lines::{Line2D, LineArtist, LineProps, LineStyle, MarkEvery, PolarMarkerRotation};
use crate::markers::{Marker, MarkerSymbol};
use crate::orientation::Orientation;
use crate::ticker::AxisName;
use crate::units::{AxisDelta, AxisValues, convert_dx};

/// Error sizes along one axis (matplotlib's `xerr`/`yerr`); every list
/// holds one value or one per point.
#[derive(Clone, Debug, PartialEq)]
pub enum ErrorSize {
    /// The same error below and above.
    Symmetric(Vec<f64>),
    /// Separate errors below and above.
    Asymmetric {
        /// Errors below the values.
        lower: Vec<f64>,
        /// Errors above the values.
        upper: Vec<f64>,
    },
}

impl From<f64> for ErrorSize {
    fn from(error: f64) -> Self {
        Self::Symmetric(vec![error])
    }
}

impl From<Vec<f64>> for ErrorSize {
    fn from(errors: Vec<f64>) -> Self {
        Self::Symmetric(errors)
    }
}

impl From<TimeDelta> for ErrorSize {
    /// The same duration below and above, on a date axis (matplotlib's
    /// `timedelta` errors).
    fn from(error: TimeDelta) -> Self {
        Self::Symmetric(vec![AxisDelta::from(error).to_number()])
    }
}

impl ErrorSize {
    /// The sizes rounded through the coordinates they extend from
    /// ([`convert_dx`]).
    pub(crate) fn convert_dx(self, origins: &[f64]) -> Self {
        match self {
            Self::Symmetric(errors) => Self::Symmetric(convert_dx(origins, &errors)),
            Self::Asymmetric { lower, upper } => Self::Asymmetric {
                lower: convert_dx(origins, &lower),
                upper: convert_dx(origins, &upper),
            },
        }
    }

    /// `(lower, upper)` errors for `n` points.
    fn bounds(&self, argument: &'static str, n: usize) -> Result<(Vec<f64>, Vec<f64>), PlotError> {
        let (lower, upper) = match self {
            Self::Symmetric(errors) => (errors, errors),
            Self::Asymmetric { lower, upper } => (lower, upper),
        };
        let (lower, upper) = (
            broadcast(argument, lower, n)?,
            broadcast(argument, upper, n)?,
        );
        // NaN skips a bar; negative sizes are rejected.
        if lower.iter().chain(&upper).any(|&e| e < 0.0) {
            return Err(PlotError::NegativeErrors(argument));
        }
        Ok((lower, upper))
    }
}

/// Options of [`Axes::errorbar`] (matplotlib's `errorbar` keywords).
#[derive(Clone, Debug, Default)]
pub struct ErrorbarOptions {
    /// Horizontal error sizes.
    pub xerr: Option<ErrorSize>,
    /// Vertical error sizes.
    pub yerr: Option<ErrorSize>,
    /// Format string of the data line: `""` for the default style,
    /// `"none"` for error bars only.
    pub fmt: String,
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

impl BarStyle {
    /// A cap or limit-marker line with `marker` (matplotlib's
    /// `eb_cap_style`).
    fn cap(&self, axes: &Axes, x: Vec<f64>, y: Vec<f64>, marker: MarkerSymbol) -> LineArtist {
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
        let mut line = LineArtist::new(x, y, crate::rc::params());
        props.apply(&mut line);
        line.base_mut().transform = Some(axes.trans_data());
        line
    }

    /// The cap or limit-marker lines with `marker` at `(x, y)` for error
    /// bars along `axis`: one line, or on polar axes one line per point
    /// whose `_` marker is turned perpendicular to its bar (matplotlib
    /// draws every polar cap with the `_` marker, whatever `marker` is).
    fn caps(
        &self,
        axes: &Axes,
        (x, y): (Vec<f64>, Vec<f64>),
        marker: MarkerSymbol,
        axis: AxisName,
    ) -> Vec<LineArtist> {
        if !axes
            .raw()
            .with::<AxesState, _>(|state| state.polar().is_some())
        {
            return vec![self.cap(axes, x, y, marker)];
        }
        let extra = if axis == AxisName::Y { FRAC_PI_2 } else { 0.0 };
        x.into_iter()
            .zip(y)
            .map(|(theta, r)| {
                let mut line = self.cap(axes, vec![theta], vec![r], MarkerSymbol::HLine);
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
    /// Points with error bars (matplotlib's `errorbar`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::ErrorbarOptions;
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8]);
    /// let options = ErrorbarOptions { yerr: Some(0.5.into()), fmt: "o".into(), ..Default::default() };
    /// let bars = ax.errorbar([1.0, 2.0], [3.0, 4.0], options)?;
    /// assert_eq!(bars.barlinecols.len(), 1);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length, an error or limit list does not
    /// broadcast, errors are negative, or the format string is invalid.
    pub fn errorbar(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        mut options: ErrorbarOptions,
    ) -> Result<ErrorbarContainer, PlotError> {
        let (x, y) = (self.xaxis().convert_units(x), self.yaxis().convert_units(y));
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
        let hidden = options.fmt.eq_ignore_ascii_case("none");
        let format = if options.fmt.is_empty() {
            LineProps::default()
        } else {
            LineProps::fmt(&options.fmt)?
        };
        let mut data = self.cycled_line(x.to_vec(), y.to_vec(), format.overlay(kwargs.clone()));
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
        x: &[f64],
        y: &[f64],
        every: &[bool],
        style: &BarStyle,
    ) -> Result<(Collection, Vec<LineArtist>), PlotError> {
        let along_x = direction.axis == AxisName::X;
        let (dep, indep) = if along_x { (x, y) } else { (y, x) };
        let (lower, upper) = direction
            .err
            .bounds(if along_x { "xerr" } else { "yerr" }, x.len())?;
        // `dep - lower * !lolims` written so that NaN errors stay NaN.
        let low: Vec<f64> = (0..x.len())
            .map(|i| dep[i] + -(1.0 - indicator(direction.lolims[i])) * lower[i])
            .collect();
        let high: Vec<f64> = (0..x.len())
            .map(|i| dep[i] + (1.0 - indicator(direction.uplims[i])) * upper[i])
            .collect();
        let orientation = if along_x {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        };
        let bars = self.segments(
            orientation,
            &masked(indep, every),
            &masked(&low, every),
            &masked(&high, every),
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

        // Lines take `(x, y)`; the error values belong on the error axis.
        let place = |positions: Vec<f64>, values: Vec<f64>| {
            if along_x {
                (values, positions)
            } else {
                (positions, values)
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
        let nolims: Vec<bool> = (0..x.len())
            .map(|i| !(direction.lolims[i] || direction.uplims[i]))
            .collect();
        if style.capsize > 0.0 && nolims.contains(&true) {
            let mask: Vec<bool> = nolims.iter().zip(every).map(|(&a, &b)| a && b).collect();
            for ends in [&low, &high] {
                let points = place(masked(indep, &mask), masked(ends, &mask));
                caps.extend(style.caps(self, points, cap_marker, direction.axis));
            }
        }
        let inverted = self.axis_of(direction.axis).inverted();
        for (upper_end, lims, ends) in [
            (false, &direction.lolims, &high),
            (true, &direction.uplims, &low),
        ] {
            if !lims.contains(&true) {
                continue;
            }
            let marker = if inverted ^ upper_end {
                hi_marker
            } else {
                lo_marker
            };
            let mask: Vec<bool> = lims.iter().zip(every).map(|(&a, &b)| a && b).collect();
            let points = place(masked(indep, &mask), masked(ends, &mask));
            caps.extend(style.caps(self, points, marker, direction.axis));
            if style.capsize > 0.0 {
                let points = (masked(x, &mask), masked(y, &mask));
                caps.extend(style.caps(self, points, cap_marker, direction.axis));
            }
        }
        Ok((bars, caps))
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

/// 1 for set flags, 0 otherwise.
fn indicator(flag: bool) -> f64 {
    f64::from(u8::from(flag))
}

/// `props` without the marker and line-style parts, which do not apply to
/// error bars and caps.
fn without_marker_style(props: LineProps) -> LineProps {
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
