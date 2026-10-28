//! Error bars in 3-D (matplotlib's `Axes3D.errorbar`): a bar along x, y
//! and z for each point, flat caps at its ends, and arrows marking upper
//! and lower limits.

use super::art3d::Art3D;
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::columns;
use super::quiver::Quiver3DOptions;
use super::state::Axes3DState;
use super::units::Sources3D;
use crate::artist::{Artist, ArtistHandle};
use crate::axes::{ErrorSize, ErrorbarContainer, ErrorbarFormat, PlotError, without_marker_style};
use crate::collections::{Collection, CollectionProps};
use crate::colors::Color;
use crate::lines::{Line2D, LineArtist, LineProps, LineStyle, MarkEvery};
use crate::markers::{Marker, MarkerSymbol};
use crate::transforms::{TransformContext, unit_to_bbox};
use crate::units::{AxisDelta, AxisDeltas, AxisValue, AxisValues, UnitHandle};

/// The tangent of a quiver head's 15° barb angle, inverted: arrow length
/// over half the width of its base (matplotlib's `1.8660254037844388`).
const ARROW_LENGTH_PER_BASE: f64 = 1.866_025_403_784_438_8;

/// Label of the parts kept out of legends.
const NO_LEGEND: &str = "_nolegend_";

/// Options of [`Axes3D::errorbar`] (matplotlib's `errorbar` keywords on
/// 3-D axes).
#[derive(Clone, Debug, Default)]
pub struct Errorbar3DOptions {
    /// Error sizes along x, y and z: numbers, one or one per point;
    /// nothing draws no bars along that axis.
    pub errors: [Option<ErrorSize>; 3],
    /// Style of the data line, or no line.
    pub fmt: ErrorbarFormat,
    /// Properties of the data line; its colour, width, alpha, z-order and
    /// label also apply to the error bars.
    pub line: LineProps,
    /// Colour of the error bars (default: the data line's colour).
    pub ecolor: Option<Color>,
    /// Width of the error bars in points.
    pub elinewidth: Option<f64>,
    /// Length of the caps in points (default `errorbar.capsize`).
    pub capsize: Option<f64>,
    /// Thickness of the caps in points.
    pub capthick: Option<f64>,
    /// Draw the bars above the data line instead of below.
    pub barsabove: bool,
    /// Per axis, points whose value is a lower limit: only the upper bar
    /// is drawn, ending in an arrow (one flag or one per point).
    pub lolims: [Vec<bool>; 3],
    /// Per axis, points whose value is an upper limit.
    pub uplims: [Vec<bool>; 3],
    /// Which points get error bars (`None` = all).
    pub errorevery: Option<MarkEvery>,
    /// Whether the data line, the bars and the caps hide their points
    /// outside the view limits; the limit arrows are not hidden, as in
    /// matplotlib (`axlim_clip`, which matplotlib 3.11 fails to draw
    /// once a cap is hidden).
    pub axlim_clip: bool,
}

/// The artists of a 3-D error bar plot.
#[derive(Clone, Debug)]
pub struct Errorbar3D {
    /// The data line, unless the format was `"none"`.
    pub data_line: Option<Line2D>,
    /// The bars, one collection per axis with errors (x, y, z order).
    pub errlines: Vec<Collection>,
    /// The caps at the lower then upper bar ends, per axis.
    pub caplines: Vec<Line2D>,
    /// The arrows marking limits (matplotlib draws them with `quiver` but
    /// returns an empty `limmarks`).
    pub limit_arrows: Vec<Collection>,
}

/// The ends of the bars of one axis for every point.
struct BarEnds {
    lows: Vec<Vec3>,
    highs: Vec<Vec3>,
}

impl Axes3D {
    /// Points `(x, y, z)` with error bars along the axes that have errors
    /// (matplotlib's `errorbar` on 3-D axes). The caps are flat markers
    /// (`|` on x and y bars, `_` on z bars); limits end in arrows sized
    /// like the caps seen face on. The points are numbers, dates, category
    /// names or user values, converted on their axes as by
    /// [`Axes3D::plot`] (matplotlib's `_process_unit_info`); the bars end
    /// at the values less and plus their errors, computed in the units of
    /// the values, such as dates and durations.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::mplot3d::Errorbar3DOptions;
    /// let ax = Figure::new().subplot_3d();
    /// let options = Errorbar3DOptions {
    ///     errors: [None, None, Some(0.5.into())],
    ///     capsize: Some(2.0),
    ///     ..Default::default()
    /// };
    /// let bars = ax.errorbar(&[0.0, 1.0], &[0.0, 1.0], &[1.0, 2.0], options)?;
    /// assert_eq!(bars.errlines.len(), 1);
    /// assert_eq!(bars.caplines.len(), 2);
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates differ in length, an error or limit list does
    /// not broadcast, `errorevery` is invalid, or errors do not add to the
    /// coordinates, such as durations to numbers
    /// ([`Axes3DError::UnaddableErrors`]; matplotlib fails on them too).
    pub fn errorbar(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
        mut options: Errorbar3DOptions,
    ) -> Result<Errorbar3D, Axes3DError> {
        let values = [x.into(), y.into(), z.into()];
        check_len("y", values[0].len(), values[1].len())?;
        check_len("z", values[0].len(), values[2].len())?;
        let n = values[0].len();
        let had_data = self.has_data();
        let sources = Sources3D::of(values.each_ref());
        let [x, y, z] = self.convert_xyz(values.clone());
        let every = every_mask(options.errorevery.as_ref(), n)?;
        let (has_xyerr, has_zerr) = (
            options.errors[0].is_some() || options.errors[1].is_some(),
            options.errors[2].is_some(),
        );
        let mut kwargs = options.line;
        let label = kwargs.base.label.take();
        kwargs.base.zorder.get_or_insert(2.0);
        kwargs.base.label = Some(NO_LEGEND.into());
        let hidden = !options.fmt.draws_line();
        let format = options.fmt.line_props();
        let plane = self.plane();
        let mut data = plane.cycled_line(x.clone(), y.clone(), format.overlay(kwargs.clone()));
        let color = if hidden {
            kwargs.color.clone().unwrap_or_else(|| Color::from("C0"))
        } else {
            data.color.clone()
        };
        let zorder = kwargs.base.zorder.unwrap_or(2.0);
        data.base_mut().zorder = zorder + if options.barsabove { -0.1 } else { 0.1 };
        let points: Vec<Vec3> = (0..n).map(|i| [x[i], y[i], z[i]]).collect();
        let data_line = (!hidden).then(|| {
            let raw = self.add_artist(data, Art3D::Line(points.clone()), options.axlim_clip);
            self.keep_unit_sources(raw.id(), sources);
            Line2D::from_raw(raw)
        });
        let ecolor = options.ecolor.take().unwrap_or(color);
        let capsize = options
            .capsize
            .unwrap_or_else(|| crate::rc::params().errorbar_capsize);
        let mut style = CapStyle {
            base: without_marker_style(kwargs.clone()),
            ecolor,
            capsize,
            capthick: options.capthick,
            bar_width: options
                .elinewidth
                .filter(|&w| w != 0.0)
                .or(kwargs.linewidth),
            arrow_length: f64::NAN,
            axlim_clip: options.axlim_clip,
        };
        style.arrow_length = self.limit_arrow_length(style.markersize());
        let mut bars = Errorbar3D {
            data_line,
            errlines: Vec::new(),
            caplines: Vec::new(),
            limit_arrows: Vec::new(),
        };
        let mut extremes: Vec<Vec3> = Vec::new();
        for axis in Axis3::ALL {
            let Some(err) = options.errors[axis.index()].take() else {
                continue;
            };
            let direction = ErrorDirection {
                axis,
                err,
                lolims: broadcast_flags("lolims", &options.lolims[axis.index()], n)?,
                uplims: broadcast_flags("uplims", &options.uplims[axis.index()], n)?,
            };
            let ends =
                self.error_direction(&direction, (&values, &points), &every, &style, &mut bars)?;
            extremes.extend(ends.lows.iter().chain(&ends.highs));
        }
        // matplotlib autoscales to the bar ends; without bars it would
        // fail, here the points are used.
        let span = if extremes.is_empty() {
            &points
        } else {
            &extremes
        };
        let [xs, ys, zs] = columns(span).map(|values| nan_extremes(&values));
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        // matplotlib's container takes z as the vertical direction.
        plane.add_container(ErrorbarContainer {
            data_line: bars.data_line.clone(),
            caplines: bars.caplines.clone(),
            barlinecols: bars.errlines.clone(),
            has_xerr: has_xyerr,
            has_yerr: has_zerr,
            label,
        });
        Ok(bars)
    }

    /// Adds the caps, limit arrows and bars of one axis to `bars` and
    /// returns the bar ends, for the points `points` converted from
    /// `values`.
    fn error_direction(
        &self,
        direction: &ErrorDirection,
        (values, points): (&[AxisValues; 3], &[Vec3]),
        every: &[bool],
        style: &CapStyle,
        bars: &mut Errorbar3D,
    ) -> Result<BarEnds, Axes3DError> {
        let axis = direction.axis;
        let (lolims, uplims) = (&direction.lolims, &direction.uplims);
        let (lows, highs) = end_values(
            &values[axis.index()],
            axis,
            &direction.err,
            (lolims, uplims),
            every,
        )?;
        let along = self.axis(axis);
        let place = |ends: &[AxisValue]| -> Vec<Vec3> {
            (points.iter().zip(ends))
                .map(|(&point, end)| {
                    let mut moved = point;
                    moved[axis.index()] = along.convert_without_update(end);
                    moved
                })
                .collect()
        };
        let ends = BarEnds {
            lows: place(&lows),
            highs: place(&highs),
        };
        let nolims: Vec<bool> = lolims
            .iter()
            .zip(uplims)
            .map(|(&lo, &up)| !(lo || up))
            .collect();
        if style.capsize > 0.0 && nolims.contains(&true) {
            let marker = if axis == Axis3::Z {
                MarkerSymbol::HLine
            } else {
                MarkerSymbol::VLine
            };
            let mask = and(&nolims, every);
            for side in [&ends.lows, &ends.highs] {
                bars.caplines
                    .push(self.cap_line(style, &masked(side, &mask), marker));
            }
        }
        for (lims, starts, sign) in [(lolims, &ends.highs, 1.0), (uplims, &ends.lows, -1.0)] {
            if lims.contains(&true) {
                let positions = masked(starts, &and(lims, every));
                let vectors = vec![axis.unit().map(|c| sign * c); positions.len()];
                let options = Quiver3DOptions {
                    length: style.arrow_length,
                    arrow_length_ratio: 1.0,
                    props: style.arrow_props(),
                    ..Quiver3DOptions::default()
                };
                bars.limit_arrows
                    .push(self.quiver(&positions, &vectors, options)?);
            }
        }
        let segments: Vec<Vec<Vec3>> = ends
            .lows
            .iter()
            .zip(&ends.highs)
            .map(|(&low, &high)| vec![low, high])
            .collect();
        bars.errlines
            .push(self.add_segments(segments, style.bar_props(), style.axlim_clip));
        Ok(ends)
    }

    /// Adds a line of markers at `points` styled as a cap.
    fn cap_line(&self, style: &CapStyle, points: &[Vec3], marker: MarkerSymbol) -> Line2D {
        let [xs, ys, _] = columns(points);
        let mut line = LineArtist::new(xs, ys, crate::rc::params());
        style.cap_props(marker).apply(&mut line);
        Line2D::from_raw(self.add_artist(line, Art3D::Line(points.to_vec()), style.axlim_clip))
    }

    /// The length of the limit arrows: a marker of `markersize` points
    /// across the axes seen face on, mapped back to data along y and
    /// stretched so that the arrow head's base is that wide (matplotlib's
    /// `quiversize`, measured on the axes' position before it is made
    /// square).
    fn limit_arrow_length(&self, markersize: f64) -> f64 {
        let state = self.raw().figure_state().borrow();
        let Some(axes) = state.store.get_as::<Axes3DState>(self.id()) else {
            return f64::NAN;
        };
        let frame = axes.base().parent.and_then(|f| state.frame_bbox(f));
        let Some(frame) = frame else {
            return f64::NAN;
        };
        let display = axes.position.transformed(&unit_to_bbox(&frame));
        let size = markersize * markersize * state.dpi() / 72.0;
        let fraction = f64::midpoint(size / display.width(), size / display.height());
        let mut camera = axes.camera;
        camera.angles.elev = 0.0;
        camera.angles.azim = 0.0;
        camera.angles.roll = 0.0;
        let matrix = camera.projection(axes.scaled_limits()).matrix;
        matrix
            .inverse()
            .map_or(f64::NAN, |inverse| inverse.rows()[1][0] * fraction)
            * ARROW_LENGTH_PER_BASE
    }
}

/// The error bars requested along one axis.
struct ErrorDirection {
    axis: Axis3,
    err: ErrorSize,
    /// One flag per point.
    lolims: Vec<bool>,
    /// One flag per point.
    uplims: Vec<bool>,
}

/// The styles shared by the bars, caps and arrows of one call.
struct CapStyle {
    /// The data line's style without its marker and line-style parts
    /// (matplotlib's `base_style`).
    base: LineProps,
    ecolor: Color,
    capsize: f64,
    capthick: Option<f64>,
    /// Width of the bars: the error width, or the line width given.
    bar_width: Option<f64>,
    /// Length of the limit arrows in data units.
    arrow_length: f64,
    /// Whether the bars and caps hide their points outside the view
    /// limits; the arrows never do, as in matplotlib.
    axlim_clip: bool,
}

impl CapStyle {
    /// The marker size of the caps, which also sizes the arrows.
    fn markersize(&self) -> f64 {
        if self.capsize > 0.0 {
            2.0 * self.capsize
        } else {
            crate::rc::params().lines_markersize
        }
    }

    /// matplotlib's `eb_cap_style` with the cap `marker`.
    fn cap_props(&self, marker: MarkerSymbol) -> LineProps {
        let mut props = self
            .base
            .clone()
            .linestyle(LineStyle::None)
            .marker(Marker::Symbol(marker))
            .color(self.ecolor.clone());
        if self.capsize > 0.0 {
            props.markersize = Some(2.0 * self.capsize);
        }
        props.markeredgewidth = self.capthick;
        props
    }

    /// matplotlib's `eb_lines_style`: the bars take the error colour, the
    /// error width (or the line width given), and the line's alpha and
    /// z-order.
    fn bar_props(&self) -> CollectionProps {
        let mut props = CollectionProps::default().color(self.ecolor.clone());
        props.base = self.base.base.clone();
        props.linewidths = self.bar_width.map(|w| vec![w]);
        props.antialiased = self.base.antialiased.map(|a| vec![a]);
        props
    }

    /// The arrows' style: the cap style as a line collection (matplotlib's
    /// `eb_quiver_style`, whose `'None'` line style draws solid lines).
    fn arrow_props(&self) -> CollectionProps {
        let mut props = CollectionProps::default().color(self.ecolor.clone());
        props.base = self.base.base.clone();
        props.linewidths = self.base.linewidth.map(|w| vec![w]);
        props.antialiased = self.base.antialiased.map(|a| vec![a]);
        props
    }
}

/// The lower and upper bar ends along `axis` of every point, in the
/// units of `values`, the coordinates of the points along it: the value
/// itself on the limited side and for points without bars, else the value
/// less or plus its error (matplotlib's `_extract_errs`, which computes
/// `data - err` and `data + err` on the data as given).
fn end_values(
    values: &AxisValues,
    axis: Axis3,
    err: &ErrorSize,
    (lolims, uplims): (&[bool], &[bool]),
    every: &[bool],
) -> Result<(Vec<AxisValue>, Vec<AxisValue>), Axes3DError> {
    let items: Vec<AxisValue> = (0..values.len()).filter_map(|j| values.get(j)).collect();
    let n = items.len();
    let (lower, upper) = match err {
        ErrorSize::Symmetric(errors) => (errors, errors),
        ErrorSize::Asymmetric { lower, upper } => (lower, upper),
    };
    let per_point = |errors: &AxisDeltas| -> Result<Vec<AxisDelta>, Axes3DError> {
        let errors: Vec<AxisDelta> = errors.iter().collect();
        if let [single] = errors.as_slice() {
            return Ok(vec![single.clone(); n]);
        }
        check_len("errors", n, errors.len())?;
        Ok(errors)
    };
    let (lower, upper) = (per_point(lower)?, per_point(upper)?);
    let end = |j: usize, error: &AxisDelta, factor: i64, keep: bool| {
        if keep {
            return Ok(items[j].clone());
        }
        items[j]
            .offset(error, factor)
            .ok_or(Axes3DError::UnaddableErrors(axis))
    };
    let lows = (0..n)
        .map(|j| end(j, &lower[j], -1, lolims[j] || !every[j]))
        .collect::<Result<_, _>>()?;
    let highs = (0..n)
        .map(|j| end(j, &upper[j], 1, uplims[j] || !every[j]))
        .collect::<Result<_, _>>()?;
    Ok((lows, highs))
}

/// One flag per point from one flag or one per point.
fn broadcast_flags(name: &'static str, flags: &[bool], n: usize) -> Result<Vec<bool>, Axes3DError> {
    match flags {
        [] => Ok(vec![false; n]),
        [single] => Ok(vec![*single; n]),
        _ => {
            check_len(name, n, flags.len())?;
            Ok(flags.to_vec())
        }
    }
}

/// The points getting error bars (matplotlib's `_errorevery_to_mask`).
fn every_mask(every: Option<&MarkEvery>, n: usize) -> Result<Vec<bool>, Axes3DError> {
    let Some(every) = every else {
        return Ok(vec![true; n]);
    };
    let mut mask = vec![false; n];
    for i in every.indices(n).map_err(PlotError::ErrorEvery)? {
        mask[i] = true;
    }
    Ok(mask)
}

/// Element-wise `a & b`.
fn and(a: &[bool], b: &[bool]) -> Vec<bool> {
    a.iter().zip(b).map(|(&a, &b)| a && b).collect()
}

/// The points where `mask` is set.
fn masked(points: &[Vec3], mask: &[bool]) -> Vec<Vec3> {
    points
        .iter()
        .zip(mask)
        .filter_map(|(p, &keep)| keep.then_some(*p))
        .collect()
}

/// The smallest and largest non-NaN value, as matplotlib's `nanmin` and
/// `nanmax` (none when every value is NaN).
fn nan_extremes(values: &[f64]) -> Vec<f64> {
    let finite = values.iter().copied().filter(|v| !v.is_nan());
    let low = finite.clone().reduce(f64::min);
    let high = finite.reduce(f64::max);
    low.into_iter().chain(high).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn limits_drop_one_side_and_every_skips_points() {
        let z = AxisValues::from([1.0, 2.0, 3.0]);
        let err = ErrorSize::Asymmetric {
            lower: 0.5.into(),
            upper: 1.0.into(),
        };
        let (lows, highs) = end_values(
            &z,
            Axis3::Z,
            &err,
            (&[true, false, false], &[false, false, false]),
            &[true, true, false],
        )
        .unwrap();
        let number = AxisValue::Number;
        assert_eq!(lows[0], number(1.0), "a lower limit keeps its low end");
        assert_eq!(highs[0], number(2.0));
        assert_eq!((&lows[1], &highs[1]), (&number(1.5), &number(3.0)));
        assert_eq!(
            (&lows[2], &highs[2]),
            (&number(3.0), &number(3.0)),
            "no bar"
        );
    }

    #[test]
    fn date_bars_end_a_duration_away() {
        use crate::dates::{DateTime, TimeDelta};
        let ax = Figure::new().subplot_3d();
        let day = |d| DateTime::new(2020, 1, d, 0, 0, 0, 0).unwrap();
        let half_day = TimeDelta::new(0, 43_200, 0).unwrap();
        let options = Errorbar3DOptions {
            errors: [None, None, Some(half_day.into())],
            ..Errorbar3DOptions::default()
        };
        let bars = ax
            .errorbar([0.0, 1.0], [0.0, 1.0], vec![day(1), day(2)], options)
            .unwrap();
        let ends = ax.with(|axes| {
            let id = bars.errlines[0].id();
            let entry = axes.artists.iter().find(|a| a.id == id).cloned();
            entry.map(|a| a.art.points())
        });
        let z: Vec<f64> = ends.unwrap().iter().map(|p| p[2]).collect();
        assert_eq!(z, [18_261.5, 18_262.5, 18_262.5, 18_263.5]);
    }

    #[test]
    fn errors_with_units_are_refused_as_in_matplotlib() {
        use crate::dates::TimeDelta;
        use crate::units::{ConversionError, ConversionInterface, Convertible, UnitAxis};

        /// A length in some unit.
        #[derive(Clone, Copy, Debug, PartialEq)]
        struct Length(f64);

        struct LengthConverter;

        impl ConversionInterface for LengthConverter {
            type Value = Length;
            type Unit = ();
            fn convert(
                &self,
                values: &[Length],
                _: Option<&()>,
                _: &UnitAxis,
            ) -> Result<Vec<f64>, ConversionError> {
                Ok(values.iter().map(|length| length.0).collect())
            }
        }

        impl Convertible for Length {
            type Converter = LengthConverter;
            fn converter() -> LengthConverter {
                LengthConverter
            }
        }

        let values = AxisValues::from([0.0]);
        let flags: (&[bool], &[bool]) = (&[false], &[false]);
        let half_day = TimeDelta::new(0, 43_200, 0).unwrap();
        assert_eq!(
            end_values(&values, Axis3::X, &half_day.into(), flags, &[true]).err(),
            Some(Axes3DError::UnaddableErrors(Axis3::X)),
            "a number plus a duration fails in Python"
        );
        let lengths = ErrorSize::from(AxisDeltas::from(vec![Length(1.0)]));
        assert_eq!(
            end_values(&values, Axis3::Y, &lengths, flags, &[true]).err(),
            Some(Axes3DError::UnaddableErrors(Axis3::Y))
        );
    }

    #[test]
    fn errorbars_autoscale_to_the_bar_ends_and_draw_arrows() {
        let ax = Figure::new().subplot_3d();
        let options = Errorbar3DOptions {
            errors: [Some(0.25.into()), None, Some(1.0.into())],
            lolims: [Vec::new(), Vec::new(), vec![true, false]],
            capsize: Some(3.0),
            ..Errorbar3DOptions::default()
        };
        let bars = ax
            .errorbar([0.0, 1.0], [0.0, 1.0], [0.0, 1.0], options)
            .unwrap();
        assert_eq!(bars.errlines.len(), 2);
        assert_eq!(bars.limit_arrows.len(), 1);
        assert_eq!(bars.caplines.len(), 4);
        let (lo, hi) = ax.xlim();
        assert!(lo < -0.25 && hi > 1.25);
        // The lower limit at z = 0 keeps its low end; the other bar spans
        // 0 to 2.
        let (lo, hi) = ax.zlim();
        assert!(lo < 0.0 && lo > -0.1 && hi > 2.0);
        assert!(
            ax.errorbar([0.0], [0.0, 1.0], [0.0], Errorbar3DOptions::default())
                .is_err()
        );
    }

    #[test]
    fn nan_extremes_ignore_nan() {
        assert_eq!(nan_extremes(&[f64::NAN, 3.0, -1.0]), [-1.0, 3.0]);
        assert_eq!(nan_extremes(&[f64::NAN]).len(), 0);
    }
}
