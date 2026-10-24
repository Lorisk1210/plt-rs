//! Decorations of the current axes: titles, labels, limits, scales,
//! ticks, grids, legends, texts, tables and polar grids (matplotlib's
//! `title`, `xlabel`, `xlim`, `xscale`, `xticks`, `grid`, `axis`,
//! `tick_params`, `margins`, `box`, `legend`, `text`, `annotate`,
//! `table`, `rgrids`, `thetagrids`, ...).

use super::error::PyplotError;
use super::registry::gca;
use crate::axes::{
    Axis, AxisMode, AxisName, GridOptions, LocatorParams, PlotError, TickParams, TickSet,
    TicklabelFormat, TitleLocation,
};
use crate::coords::CoordError;
use crate::legend::{Legend, LegendError, LegendOptions};
use crate::polar::PolarAxes;
use crate::scale::ScaleError;
use crate::table::{Table, TableError, TableOptions};
use crate::text::{AnnotateOptions, Annotation, Text};
use crate::ticker::TickerError;
use crate::units::{AxisValue, AxisValues, DataPoint};

/// Sets the centre title of the current axes (matplotlib's `title`).
pub fn title(text: impl Into<String>) -> Text {
    gca().set_title(text)
}

/// Sets the title at `location` of the current axes (matplotlib's
/// `title(label, loc=...)`).
pub fn title_at(location: TitleLocation, text: impl Into<String>) -> Text {
    gca().set_title_at(location, text)
}

/// Sets the x label of the current axes (matplotlib's `xlabel`).
pub fn xlabel(text: impl Into<String>) {
    gca().set_xlabel(text);
}

/// Sets the y label of the current axes (matplotlib's `ylabel`).
pub fn ylabel(text: impl Into<String>) {
    gca().set_ylabel(text);
}

/// Sets the x limits of the current axes (matplotlib's
/// `xlim(left, right)`).
///
/// # Errors
///
/// As [`Axes::set_xlim`](crate::axes::Axes::set_xlim).
pub fn xlim(left: impl Into<AxisValue>, right: impl Into<AxisValue>) -> Result<(), PlotError> {
    gca().set_xlim(left, right).map(|_| ())
}

/// Sets the y limits of the current axes (matplotlib's
/// `ylim(bottom, top)`).
///
/// # Errors
///
/// As [`Axes::set_ylim`](crate::axes::Axes::set_ylim).
pub fn ylim(bottom: impl Into<AxisValue>, top: impl Into<AxisValue>) -> Result<(), PlotError> {
    gca().set_ylim(bottom, top).map(|_| ())
}

/// The x limits of the current axes (matplotlib's `xlim()`).
pub fn get_xlim() -> (f64, f64) {
    gca().xlim()
}

/// The y limits of the current axes (matplotlib's `ylim()`).
pub fn get_ylim() -> (f64, f64) {
    gca().ylim()
}

/// Sets the x scale of the current axes by name (matplotlib's `xscale`).
///
/// # Errors
///
/// When no scale is registered under `name`.
pub fn xscale(name: &str) -> Result<(), ScaleError> {
    gca().set_xscale(name).map(|_| ())
}

/// Sets the y scale of the current axes by name (matplotlib's `yscale`).
///
/// # Errors
///
/// When no scale is registered under `name`.
pub fn yscale(name: &str) -> Result<(), ScaleError> {
    gca().set_yscale(name).map(|_| ())
}

/// Puts the major ticks of `axis` (the minor ones when `minor`) at
/// `ticks`, labelled with `labels` unless that is empty.
fn place_ticks(
    axis: &Axis,
    ticks: AxisValues,
    labels: &[&str],
    minor: bool,
) -> Result<(), PlotError> {
    if minor {
        axis.set_minor_ticks(ticks);
        if !labels.is_empty() {
            axis.set_minor_ticklabels(labels.iter().copied())?;
        }
    } else {
        axis.set_ticks(ticks);
        if !labels.is_empty() {
            axis.set_ticklabels(labels.iter().copied())?;
        }
    }
    Ok(())
}

/// Puts the x ticks of the current axes at `ticks`, labelled with `labels`
/// unless that is empty (matplotlib's `xticks(ticks, labels)`;
/// `xticks([], &[])` removes the ticks).
///
/// # Errors
///
/// When there are labels but not one per tick.
pub fn xticks(ticks: impl Into<AxisValues>, labels: &[&str]) -> Result<(), PlotError> {
    place_ticks(&gca().xaxis(), ticks.into(), labels, false)
}

/// Puts the y ticks of the current axes at `ticks`, labelled with `labels`
/// unless that is empty (matplotlib's `yticks(ticks, labels)`;
/// `yticks([], &[])` removes the ticks).
///
/// # Errors
///
/// When there are labels but not one per tick.
pub fn yticks(ticks: impl Into<AxisValues>, labels: &[&str]) -> Result<(), PlotError> {
    place_ticks(&gca().yaxis(), ticks.into(), labels, false)
}

/// Puts the minor x ticks of the current axes at `ticks`, labelled with
/// `labels` unless that is empty (matplotlib's
/// `xticks(ticks, labels, minor=True)`).
///
/// ```
/// use plt::pyplot as plt;
/// plt::plot(vec![1.0, 2.0, 3.0], vec![1.0, 4.0, 9.0])?;
/// plt::xticks_minor(vec![1.1, 2.9], &["a", "b"])?;
/// assert_eq!(plt::gca().xaxis().ticklocs(true)?, [1.1, 2.9]);
/// assert_eq!(plt::gca().xaxis().ticklabels(true)?, ["a", "b"]);
/// plt::close_all();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When there are labels but not one per tick.
pub fn xticks_minor(ticks: impl Into<AxisValues>, labels: &[&str]) -> Result<(), PlotError> {
    place_ticks(&gca().xaxis(), ticks.into(), labels, true)
}

/// Puts the minor y ticks of the current axes at `ticks`, labelled with
/// `labels` unless that is empty (matplotlib's
/// `yticks(ticks, labels, minor=True)`).
///
/// # Errors
///
/// When there are labels but not one per tick.
pub fn yticks_minor(ticks: impl Into<AxisValues>, labels: &[&str]) -> Result<(), PlotError> {
    place_ticks(&gca().yaxis(), ticks.into(), labels, true)
}

/// The x ticks the current axes draws, with their labels (matplotlib's
/// `xticks()`).
///
/// # Errors
///
/// When the ticks cannot be computed (see [`Axes::ticks`]).
///
/// [`Axes::ticks`]: crate::axes::Axes::ticks
pub fn get_xticks() -> Result<TickSet, TickerError> {
    gca().ticks(AxisName::X)
}

/// The y ticks the current axes draws, with their labels (matplotlib's
/// `yticks()`).
///
/// # Errors
///
/// When the ticks cannot be computed (see [`Axes::ticks`]).
///
/// [`Axes::ticks`]: crate::axes::Axes::ticks
pub fn get_yticks() -> Result<TickSet, TickerError> {
    gca().ticks(AxisName::Y)
}

/// Shows or hides the major grid of the current axes (matplotlib's
/// `grid(visible)`).
pub fn grid(visible: bool) {
    gca().grid(visible);
}

/// Shows or hides the grid of the current axes for the ticks and axes
/// `options` selects, styled by it (matplotlib's
/// `grid(visible, which, axis, **kwargs)`).
pub fn grid_with(visible: bool, options: GridOptions) {
    gca().grid_with(visible, options);
}

/// Applies an axis preset (`On`, `Off`, `Equal`, `Tight`, ...) to the
/// current axes (matplotlib's `axis(mode)`).
pub fn axis(mode: AxisMode) {
    gca().axis_mode(mode);
}

/// Changes the appearance of the ticks and tick labels of the current
/// axes (matplotlib's `tick_params`).
pub fn tick_params(params: &TickParams) {
    gca().tick_params(params);
}

/// Configures the scalar tick label formatters of the current axes
/// (matplotlib's `ticklabel_format`).
///
/// # Errors
///
/// As [`Axes::ticklabel_format`](crate::axes::Axes::ticklabel_format).
pub fn ticklabel_format(format: &TicklabelFormat) -> Result<(), PlotError> {
    gca().ticklabel_format(format).map(|_| ())
}

/// Configures the tick locators of the current axes (matplotlib's
/// `locator_params`).
///
/// # Errors
///
/// As [`Axes::locator_params`](crate::axes::Axes::locator_params).
pub fn locator_params(params: &LocatorParams) -> Result<(), PlotError> {
    gca().locator_params(params).map(|_| ())
}

/// Shows minor ticks on the current axes (matplotlib's `minorticks_on`).
pub fn minorticks_on() {
    gca().minorticks_on();
}

/// Hides the minor ticks of the current axes (matplotlib's
/// `minorticks_off`).
pub fn minorticks_off() {
    gca().minorticks_off();
}

/// Sets the autoscaling margins of the current axes as fractions of the
/// data ranges (matplotlib's `margins(x, y)`).
///
/// # Errors
///
/// When a margin is -0.5 or less, or not finite.
pub fn margins(x: f64, y: f64) -> Result<(), PlotError> {
    gca().set_margins(x, y).map(|_| ())
}

/// Autoscales one axis, or both when `None`, of the current axes
/// (matplotlib's `autoscale`).
pub fn autoscale(axis: Option<AxisName>, tight: Option<bool>) {
    gca().autoscale(axis, tight);
}

/// Shows (`Some(true)`), hides (`Some(false)`) or toggles (`None`) the
/// frame of the current axes: its background patch and spines
/// (matplotlib's `box`).
pub fn r#box(on: Option<bool>) {
    let ax = gca();
    let on = on.unwrap_or_else(|| !ax.frame_on());
    ax.set_frame_on(on);
}

/// Places a legend on the current axes (matplotlib's `legend`).
///
/// # Errors
///
/// As [`Axes::legend`](crate::axes::Axes::legend).
pub fn legend(options: LegendOptions) -> Result<Legend, LegendError> {
    gca().legend(options)
}

/// Adds a text at `(x, y)` in data coordinates of the current axes
/// (matplotlib's `text`).
pub fn text(x: impl Into<AxisValue>, y: impl Into<AxisValue>, text: impl Into<String>) -> Text {
    gca().text(x, y, text)
}

/// Annotates the point `xy` of the current axes (matplotlib's
/// `annotate`).
///
/// # Errors
///
/// As [`Axes::annotate`](crate::axes::Axes::annotate).
pub fn annotate(
    text: impl Into<String>,
    xy: impl Into<DataPoint>,
    options: AnnotateOptions,
) -> Result<Annotation, CoordError> {
    gca().annotate(text, xy, options)
}

/// Adds a table to the current axes (matplotlib's `table`).
///
/// # Errors
///
/// As [`Axes::table`](crate::axes::Axes::table).
pub fn table(options: TableOptions) -> Result<Table, TableError> {
    gca().table(options)
}

/// Puts the radial gridlines of the current polar axes at `radii`,
/// labelled with `labels` unless that is empty, with the labels at `angle`
/// degrees (the current label angle when `None`; matplotlib's `rgrids`).
///
/// # Errors
///
/// [`PyplotError::NotPolar`] when the current axes is not polar, and
/// [`PyplotError::Plot`] when there are labels but not one per radius.
pub fn rgrids(radii: &[f64], labels: &[&str], angle: Option<f64>) -> Result<(), PyplotError> {
    let ax = current_polar()?;
    if labels.is_empty() {
        ax.set_rgrids(radii, angle);
    } else {
        ax.set_rgrids_labeled(radii, labels.iter().copied(), angle)?;
    }
    Ok(())
}

/// Puts the angular gridlines of the current polar axes at `angles`
/// degrees, labelled with `labels` unless that is empty (matplotlib's
/// `thetagrids`).
///
/// # Errors
///
/// [`PyplotError::NotPolar`] when the current axes is not polar, and
/// [`PyplotError::Plot`] when there are labels but not one per angle.
pub fn thetagrids(angles: &[f64], labels: &[&str]) -> Result<(), PyplotError> {
    let ax = current_polar()?;
    if labels.is_empty() {
        ax.set_thetagrids(angles);
    } else {
        ax.set_thetagrids_labeled(angles, labels.iter().copied())?;
    }
    Ok(())
}

/// The current axes as polar axes.
fn current_polar() -> Result<PolarAxes, PyplotError> {
    gca().as_polar().ok_or(PyplotError::NotPolar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyplot::{close_all, figure, gcf, plot, polar};

    #[test]
    fn labels_limits_and_scales() {
        close_all();
        plot(vec![1.0, 10.0], vec![1.0, 100.0]).unwrap();
        title("centre");
        title_at(TitleLocation::Left, "left");
        xlabel("x");
        ylabel("y");
        xlim(1.0, 10.0).unwrap();
        ylim(1.0, 100.0).unwrap();
        assert_eq!(
            xlim(f64::NAN, 1.0),
            Err(PlotError::NonFiniteLimit(AxisName::X))
        );
        yscale("log").unwrap();
        assert!(xscale("no such scale").is_err());
        let ax = gca();
        assert_eq!(ax.title(TitleLocation::Left), "left");
        assert_eq!(
            (ax.label(AxisName::X), ax.label(AxisName::Y)),
            ("x".into(), "y".into())
        );
        assert_eq!((get_xlim(), get_ylim()), ((1.0, 10.0), (1.0, 100.0)));
        assert_eq!(ax.yscale(), "log");
        close_all();
    }

    #[test]
    fn ticks_grids_and_frames() {
        close_all();
        plot(vec![0.0, 3.0], vec![0.0, 3.0]).unwrap();
        ticklabel_format(&TicklabelFormat::default()).unwrap();
        locator_params(&LocatorParams::default()).unwrap();
        xticks(vec![0.0, 1.5, 3.0], &["a", "b", "c"]).unwrap();
        assert!(yticks(vec![0.0, 1.0], &["only one"]).is_err());
        let labels: Vec<String> = get_xticks()
            .unwrap()
            .major
            .into_iter()
            .map(|t| t.label)
            .collect();
        assert_eq!(labels, ["a", "b", "c"]);
        yticks(Vec::<f64>::new(), &[]).unwrap();
        assert_eq!(get_yticks().unwrap().major.len(), 0);
        grid(true);
        grid_with(true, GridOptions::default());
        assert!(gca().has_grid(AxisName::X, true));
        tick_params(&TickParams::default());
        assert!(
            ticklabel_format(&TicklabelFormat::default()).is_err(),
            "fixed labels now"
        );
        let steps = LocatorParams {
            steps: Some(vec![1.0, 2.0]),
            ..LocatorParams::default()
        };
        assert!(
            locator_params(&steps).is_err(),
            "fixed ticks take only nbins now"
        );
        minorticks_on();
        minorticks_off();
        margins(0.1, 0.2).unwrap();
        autoscale(Some(AxisName::X), Some(true));
        axis(AxisMode::Equal);
        r#box(None);
        assert!(!gca().frame_on(), "box toggles the frame");
        r#box(Some(true));
        assert!(gca().frame_on());
        close_all();
    }

    #[test]
    fn legends_texts_annotations_and_tables() {
        use crate::artist::ArtistHandle;
        close_all();
        plot(vec![0.0, 1.0], vec![0.0, 1.0])
            .unwrap()
            .set_label("line");
        let shown = legend(LegendOptions::default()).unwrap();
        assert!(gcf().contains_artist(&shown));
        assert_eq!(text(0.5, 0.5, "note").text(), "note");
        let note = annotate("peak", (1.0, 1.0), AnnotateOptions::default()).unwrap();
        assert_eq!(note.text(), "peak");
        let options = TableOptions {
            cell_text: vec![vec!["1".into(), "2".into()]],
            ..TableOptions::default()
        };
        assert!(table(options).is_ok());
        close_all();
    }

    #[test]
    fn polar_grids_need_polar_axes() {
        close_all();
        plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
        assert_eq!(rgrids(&[0.5, 1.0], &[], None), Err(PyplotError::NotPolar));
        figure();
        polar(vec![0.0, 1.0], vec![1.0, 2.0]).unwrap();
        rgrids(&[0.5, 1.0], &["half", "one"], Some(45.0)).unwrap();
        thetagrids(&[0.0, 90.0], &[]).unwrap();
        assert!(matches!(
            thetagrids(&[0.0], &["a", "b"]),
            Err(PyplotError::Plot(_))
        ));
        assert_eq!(gca().as_polar().unwrap().rlabel_position(), 45.0);
        close_all();
    }
}
