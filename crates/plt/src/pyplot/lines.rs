//! Lines, markers, fills and reference lines on the current axes
//! (matplotlib's `plot`, `scatter`, `step`, `stem`, `errorbar`,
//! `fill_between`, `axhline`, `axvspan`, ...).

use plt_core::Point;

use super::mappables::make_current;
use super::registry::gca;
use crate::axes::{
    EcdfError, EcdfOptions, ErrorbarContainer, ErrorbarOptions, EventplotOptions,
    FillBetweenOptions, PlotError, ScatterOptions, StackplotOptions, StairsOptions, StemContainer,
    StemOptions,
};
use crate::collections::Collection;
use crate::lines::{AxLineThrough, Line2D, LineProps, StepWhere};
use crate::patches::{FancyArrow, Patch, PatchProps, Polygon, Rectangle, StepPatch};
use crate::units::{AxisValue, AxisValues};

/// Plots `y` against `x` on the current axes (matplotlib's `plot`).
///
/// # Errors
///
/// As [`Axes::plot`](crate::axes::Axes::plot).
pub fn plot(x: impl Into<AxisValues>, y: impl Into<AxisValues>) -> Result<Line2D, PlotError> {
    gca().plot(x, y)
}

/// Plots with a format string such as `"r--o"` on the current axes
/// (matplotlib's `plot(x, y, fmt)`).
///
/// # Errors
///
/// As [`Axes::plot_fmt`](crate::axes::Axes::plot_fmt).
pub fn plot_fmt(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
    format: &str,
) -> Result<Line2D, PlotError> {
    gca().plot_fmt(x, y, format)
}

/// Plots with explicit line properties on the current axes (matplotlib's
/// `plot(x, y, **kwargs)`).
///
/// # Errors
///
/// As [`Axes::plot_with`](crate::axes::Axes::plot_with).
pub fn plot_with(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
    props: LineProps,
) -> Result<Line2D, PlotError> {
    gca().plot_with(x, y, props)
}

/// Plots `y` against its indices on the current axes (matplotlib's
/// `plot(y)`).
pub fn plot_y(y: impl AsRef<[f64]>) -> Line2D {
    gca().plot_y(y)
}

/// Plots on log scales on both axes of the current axes (matplotlib's
/// `loglog`).
///
/// # Errors
///
/// As [`Axes::loglog`](crate::axes::Axes::loglog).
pub fn loglog(x: impl Into<AxisValues>, y: impl Into<AxisValues>) -> Result<Line2D, PlotError> {
    gca().loglog(x, y)
}

/// Plots with a log x scale on the current axes (matplotlib's
/// `semilogx`).
///
/// # Errors
///
/// As [`Axes::semilogx`](crate::axes::Axes::semilogx).
pub fn semilogx(x: impl Into<AxisValues>, y: impl Into<AxisValues>) -> Result<Line2D, PlotError> {
    gca().semilogx(x, y)
}

/// Plots with a log y scale on the current axes (matplotlib's
/// `semilogy`).
///
/// # Errors
///
/// As [`Axes::semilogy`](crate::axes::Axes::semilogy).
pub fn semilogy(x: impl Into<AxisValues>, y: impl Into<AxisValues>) -> Result<Line2D, PlotError> {
    gca().semilogy(x, y)
}

/// Scatters markers on the current axes and makes them the current image
/// (matplotlib's `scatter`).
///
/// # Errors
///
/// As [`Axes::scatter`](crate::axes::Axes::scatter).
pub fn scatter(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
) -> Result<Collection, PlotError> {
    scatter_with(x, y, ScatterOptions::default())
}

/// Scatters markers with explicit sizes, colours and styles on the current
/// axes and makes them the current image (matplotlib's
/// `scatter(x, y, s=..., c=..., ...)`).
///
/// # Errors
///
/// As [`Axes::scatter_with`](crate::axes::Axes::scatter_with).
pub fn scatter_with(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
    options: ScatterOptions,
) -> Result<Collection, PlotError> {
    let ax = gca();
    let points = ax.scatter_with(x, y, options)?;
    make_current(&ax, &points);
    Ok(points)
}

/// Plots a step line on the current axes (matplotlib's `step`).
///
/// # Errors
///
/// As [`Axes::step`](crate::axes::Axes::step).
pub fn step(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
    at: StepWhere,
    props: LineProps,
) -> Result<Line2D, PlotError> {
    gca().step(x, y, at, props)
}

/// Draws a stepwise constant function on the current axes (matplotlib's
/// `stairs`).
///
/// # Errors
///
/// As [`Axes::stairs`](crate::axes::Axes::stairs).
pub fn stairs(
    values: impl Into<Vec<f64>>,
    options: StairsOptions,
) -> Result<Patch<StepPatch>, PlotError> {
    gca().stairs(values, options)
}

/// Draws a stem plot on the current axes (matplotlib's `stem`).
///
/// # Errors
///
/// As [`Axes::stem`](crate::axes::Axes::stem).
pub fn stem(
    locs: impl Into<AxisValues>,
    heads: impl Into<AxisValues>,
    options: StemOptions,
) -> Result<StemContainer, PlotError> {
    gca().stem(locs, heads, options)
}

/// Plots data with error bars on the current axes (matplotlib's
/// `errorbar`).
///
/// # Errors
///
/// As [`Axes::errorbar`](crate::axes::Axes::errorbar).
pub fn errorbar(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
    options: ErrorbarOptions,
) -> Result<ErrorbarContainer, PlotError> {
    gca().errorbar(x, y, options)
}

/// Draws rows of event lines on the current axes (matplotlib's
/// `eventplot`).
///
/// # Errors
///
/// As [`Axes::eventplot`](crate::axes::Axes::eventplot).
pub fn eventplot<P: Clone + Into<AxisValues>>(
    positions: &[P],
    options: EventplotOptions,
) -> Result<Vec<Collection>, PlotError> {
    gca().eventplot(positions, options)
}

/// Draws horizontal lines from `xmin` to `xmax` at each `y` on the current
/// axes (matplotlib's `hlines`).
///
/// # Errors
///
/// As [`Axes::hlines`](crate::axes::Axes::hlines).
pub fn hlines(
    y: impl Into<AxisValues>,
    xmin: impl Into<AxisValues>,
    xmax: impl Into<AxisValues>,
) -> Result<Collection, PlotError> {
    gca().hlines(y, xmin, xmax)
}

/// Draws vertical lines from `ymin` to `ymax` at each `x` on the current
/// axes (matplotlib's `vlines`).
///
/// # Errors
///
/// As [`Axes::vlines`](crate::axes::Axes::vlines).
pub fn vlines(
    x: impl Into<AxisValues>,
    ymin: impl Into<AxisValues>,
    ymax: impl Into<AxisValues>,
) -> Result<Collection, PlotError> {
    gca().vlines(x, ymin, ymax)
}

/// A horizontal line across the current axes at `y` (matplotlib's
/// `axhline`).
pub fn axhline(y: impl Into<AxisValue>, props: LineProps) -> Line2D {
    gca().axhline(y, props)
}

/// A horizontal line at `y` from `xmin` to `xmax` in axes fractions on the
/// current axes (matplotlib's `axhline(y, xmin, xmax)`).
pub fn axhline_between(y: impl Into<AxisValue>, xmin: f64, xmax: f64, props: LineProps) -> Line2D {
    gca().axhline_between(y, xmin, xmax, props)
}

/// A vertical line across the current axes at `x` (matplotlib's
/// `axvline`).
pub fn axvline(x: impl Into<AxisValue>, props: LineProps) -> Line2D {
    gca().axvline(x, props)
}

/// A vertical line at `x` from `ymin` to `ymax` in axes fractions on the
/// current axes (matplotlib's `axvline(x, ymin, ymax)`).
pub fn axvline_between(x: impl Into<AxisValue>, ymin: f64, ymax: f64, props: LineProps) -> Line2D {
    gca().axvline_between(x, ymin, ymax, props)
}

/// An infinite line through `xy1` on the current axes (matplotlib's
/// `axline`).
///
/// # Errors
///
/// When a slope is given while either axis has a non-linear scale.
pub fn axline(xy1: Point, through: AxLineThrough, props: LineProps) -> Result<Line2D, PlotError> {
    gca().axline(xy1, through, props)
}

/// A horizontal band from `ymin` to `ymax` across the current axes
/// (matplotlib's `axhspan`).
pub fn axhspan(ymin: impl Into<AxisValue>, ymax: impl Into<AxisValue>) -> Patch<Rectangle> {
    gca().axhspan(ymin, ymax)
}

/// A horizontal band from `ymin` to `ymax`, `xmin` to `xmax` in axes
/// fractions, on the current axes (matplotlib's
/// `axhspan(ymin, ymax, xmin, xmax)`).
pub fn axhspan_between(
    ymin: impl Into<AxisValue>,
    ymax: impl Into<AxisValue>,
    xmin: f64,
    xmax: f64,
) -> Patch<Rectangle> {
    gca().axhspan_between(ymin, ymax, xmin, xmax)
}

/// A vertical band from `xmin` to `xmax` across the current axes
/// (matplotlib's `axvspan`).
pub fn axvspan(xmin: impl Into<AxisValue>, xmax: impl Into<AxisValue>) -> Patch<Rectangle> {
    gca().axvspan(xmin, xmax)
}

/// A vertical band from `xmin` to `xmax`, `ymin` to `ymax` in axes
/// fractions, on the current axes (matplotlib's
/// `axvspan(xmin, xmax, ymin, ymax)`).
pub fn axvspan_between(
    xmin: impl Into<AxisValue>,
    xmax: impl Into<AxisValue>,
    ymin: f64,
    ymax: f64,
) -> Patch<Rectangle> {
    gca().axvspan_between(xmin, xmax, ymin, ymax)
}

/// Fills the polygon through `(x, y)` on the current axes (matplotlib's
/// `fill`).
///
/// # Errors
///
/// As [`Axes::fill`](crate::axes::Axes::fill).
pub fn fill(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
    props: PatchProps,
) -> Result<Patch<Polygon>, PlotError> {
    gca().fill(x, y, props)
}

/// Fills between the curves `y1` and `y2` over `x` on the current axes
/// (matplotlib's `fill_between`).
///
/// # Errors
///
/// As [`Axes::fill_between`](crate::axes::Axes::fill_between).
pub fn fill_between(
    x: impl Into<AxisValues>,
    y1: impl Into<AxisValues>,
    y2: impl Into<AxisValues>,
    options: FillBetweenOptions,
) -> Result<Collection, PlotError> {
    gca().fill_between(x, y1, y2, options)
}

/// Fills between the curves `x1` and `x2` over `y` on the current axes
/// (matplotlib's `fill_betweenx`).
///
/// # Errors
///
/// As [`Axes::fill_betweenx`](crate::axes::Axes::fill_betweenx).
pub fn fill_betweenx(
    y: impl Into<AxisValues>,
    x1: impl Into<AxisValues>,
    x2: impl Into<AxisValues>,
    options: FillBetweenOptions,
) -> Result<Collection, PlotError> {
    gca().fill_betweenx(y, x1, x2, options)
}

/// Stacks the curves `ys` over `x` on the current axes (matplotlib's
/// `stackplot`).
///
/// # Errors
///
/// As [`Axes::stackplot`](crate::axes::Axes::stackplot).
pub fn stackplot(
    x: impl Into<AxisValues>,
    ys: &[Vec<f64>],
    options: StackplotOptions,
) -> Result<Vec<Collection>, PlotError> {
    gca().stackplot(x, ys, options)
}

/// Draws an arrow on the current axes (matplotlib's `arrow`).
pub fn arrow(arrow: FancyArrow) -> Patch<FancyArrow> {
    gca().arrow(arrow)
}

/// Plots the empirical cumulative distribution of `x` on the current axes
/// (matplotlib's `ecdf`).
///
/// # Errors
///
/// As [`Axes::ecdf`](crate::axes::Axes::ecdf).
pub fn ecdf(x: &[f64], options: EcdfOptions) -> Result<Line2D, EcdfError> {
    gca().ecdf(x, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ColorizingArtist;
    use crate::pyplot::{close_all, gcf, gci};

    #[test]
    fn line_plots_land_on_the_current_axes() {
        close_all();
        let lines = [
            plot(vec![0.0, 1.0], vec![0.0, 2.0]).unwrap(),
            plot_fmt(vec![0.0, 1.0], vec![1.0, 0.0], "r--").unwrap(),
            plot_with(vec![0.0, 1.0], vec![2.0, 2.0], LineProps::default()).unwrap(),
            plot_y([3.0, 1.0]),
            step(
                vec![0.0, 1.0],
                vec![1.0, 2.0],
                StepWhere::Mid,
                LineProps::default(),
            )
            .unwrap(),
            axhline(0.5, LineProps::default()),
            axvline_between(0.5, 0.0, 0.5, LineProps::default()),
            axline(
                Point::new(0.0, 0.0),
                AxLineThrough::Slope(1.0),
                LineProps::default(),
            )
            .unwrap(),
        ];
        let ax = gca();
        for line in &lines {
            assert!(ax.lines().contains(line));
        }
        assert_eq!(gcf().axes().len(), 1);
        close_all();
    }

    #[test]
    fn log_plots_set_the_scales() {
        close_all();
        loglog(vec![1.0, 10.0], vec![1.0, 100.0]).unwrap();
        assert_eq!((gca().xscale(), gca().yscale()), ("log", "log"));
        crate::pyplot::cla();
        semilogy(vec![1.0, 2.0], vec![1.0, 100.0]).unwrap();
        assert_eq!((gca().xscale(), gca().yscale()), ("linear", "log"));
        crate::pyplot::cla();
        semilogx(vec![1.0, 100.0], vec![1.0, 2.0]).unwrap();
        assert_eq!(gca().xscale(), "log");
        close_all();
    }

    #[test]
    fn scatter_becomes_the_current_image() {
        close_all();
        let points = scatter(vec![0.0, 1.0], vec![1.0, 0.0]).unwrap();
        assert!(matches!(gci(), Some(ColorizingArtist::Collection(c)) if c == points));
        close_all();
    }

    #[test]
    fn fills_spans_and_collections() {
        close_all();
        let x = [0.0, 1.0, 2.0];
        fill(x, [0.0, 1.0, 0.0], PatchProps::default()).unwrap();
        fill_between(x, [0.0, 1.0, 0.5], [0.0; 3], FillBetweenOptions::default()).unwrap();
        fill_betweenx(x, [0.0, 1.0, 0.5], [0.0; 3], FillBetweenOptions::default()).unwrap();
        stackplot(x, &[vec![1.0, 2.0, 3.0]], StackplotOptions::default()).unwrap();
        hlines([1.0], [0.0], [2.0]).unwrap();
        vlines([1.0], [0.0], [2.0]).unwrap();
        eventplot(&[vec![0.5, 1.5]], EventplotOptions::default()).unwrap();
        axhspan(0.2, 0.4);
        axhspan_between(0.2, 0.4, 0.1, 0.9);
        axvspan(0.5, 0.6);
        axvspan_between(0.5, 0.6, 0.0, 0.5);
        axhline_between(1.0, 0.2, 0.8, LineProps::default());
        axvline(1.0, LineProps::default());
        stairs(vec![1.0, 2.0], StairsOptions::default()).unwrap();
        stem(x, [1.0, 2.0, 1.0], StemOptions::default()).unwrap();
        errorbar(x, [1.0, 2.0, 1.0], ErrorbarOptions::default()).unwrap();
        ecdf(&[3.0, 1.0, 2.0], EcdfOptions::default()).unwrap();
        arrow(FancyArrow::new(0.0, 0.0, 1.0, 1.0));
        assert!(gcf().to_png().is_ok());
        close_all();
    }
}
