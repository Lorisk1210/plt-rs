//! Bars and statistical plots on the current axes (matplotlib's `bar`,
//! `barh`, `broken_barh`, `bar_label`, `grouped_bar`, `hist`, `boxplot`,
//! `violinplot`, `pie` and `pie_label`).

use super::registry::gca;
use crate::axes::{
    BandAlign, BandRange, BarContainer, BarLabelError, BarLabelOptions, BarOptions, BoxplotArtists,
    BoxplotOptions, GroupedBarData, GroupedBarError, GroupedBarOptions, HistPlot, HistPlotOptions,
    PieContainer, PieLabelOptions, PieLabels, PieOptions, PlotError, ViolinParts,
    ViolinplotOptions,
};
use crate::collections::Collection;
use crate::stats::BinSample;
use crate::text::{Annotation, Text};
use crate::units::{AxisDelta, AxisValue, AxisValues};

/// Draws vertical bars on the current axes (matplotlib's `bar`).
///
/// # Errors
///
/// As [`Axes::bar`](crate::axes::Axes::bar).
pub fn bar(
    x: impl Into<AxisValues>,
    height: impl AsRef<[f64]>,
    options: BarOptions,
) -> Result<BarContainer, PlotError> {
    gca().bar(x, height, options)
}

/// Draws horizontal bars on the current axes (matplotlib's `barh`).
///
/// # Errors
///
/// As [`Axes::barh`](crate::axes::Axes::barh).
pub fn barh(
    y: impl Into<AxisValues>,
    width: impl AsRef<[f64]>,
    options: BarOptions,
) -> Result<BarContainer, PlotError> {
    gca().barh(y, width, options)
}

/// Draws horizontal bands `xranges` (start, width) at the band's position
/// with its height on the current axes (matplotlib's `broken_barh`).
pub fn broken_barh<X, W>(
    xranges: &[(X, W)],
    band: impl Into<BandRange>,
    align: BandAlign,
) -> Collection
where
    X: Clone + Into<AxisValue>,
    W: Clone + Into<AxisDelta>,
{
    gca().broken_barh(xranges, band, align)
}

/// Labels the bars of `container` on the current axes (matplotlib's
/// `bar_label`).
///
/// # Errors
///
/// As [`Axes::bar_label`](crate::axes::Axes::bar_label).
pub fn bar_label(
    container: &BarContainer,
    options: &BarLabelOptions,
) -> Result<Vec<Annotation>, BarLabelError> {
    gca().bar_label(container, options)
}

/// Draws groups of bars side by side on the current axes (matplotlib's
/// `grouped_bar`).
///
/// # Errors
///
/// As [`Axes::grouped_bar`](crate::axes::Axes::grouped_bar).
pub fn grouped_bar(
    heights: impl Into<GroupedBarData>,
    options: GroupedBarOptions,
) -> Result<Vec<BarContainer>, GroupedBarError> {
    gca().grouped_bar(heights, options)
}

/// Draws histograms of `datasets` on the current axes (matplotlib's
/// `hist`).
///
/// # Errors
///
/// As [`Axes::hist`](crate::axes::Axes::hist).
pub fn hist<T: BinSample, D: AsRef<[T]>>(
    datasets: &[D],
    options: &HistPlotOptions<'_>,
) -> Result<HistPlot, PlotError> {
    gca().hist(datasets, options)
}

/// Draws box-and-whisker plots of `datasets` on the current axes
/// (matplotlib's `boxplot`).
///
/// # Errors
///
/// As [`Axes::boxplot`](crate::axes::Axes::boxplot).
pub fn boxplot<D: AsRef<[f64]>>(
    datasets: &[D],
    options: &BoxplotOptions,
) -> Result<BoxplotArtists, PlotError> {
    gca().boxplot(datasets, options)
}

/// Draws violin plots of `datasets` on the current axes (matplotlib's
/// `violinplot`).
///
/// # Errors
///
/// As [`Axes::violinplot`](crate::axes::Axes::violinplot).
pub fn violinplot<D: AsRef<[f64]>>(
    datasets: &[D],
    options: &ViolinplotOptions,
) -> Result<ViolinParts, PlotError> {
    gca().violinplot(datasets, options)
}

/// Draws a pie chart of `values` on the current axes (matplotlib's
/// `pie`).
///
/// # Errors
///
/// As [`Axes::pie`](crate::axes::Axes::pie).
pub fn pie(values: &[f64], options: &PieOptions) -> Result<PieContainer, PlotError> {
    gca().pie(values, options)
}

/// Labels the wedges of a pie chart on the current axes (matplotlib's
/// `pie_label`).
///
/// # Errors
///
/// As [`Axes::pie_label`](crate::axes::Axes::pie_label).
pub fn pie_label(
    container: &mut PieContainer,
    labels: impl Into<PieLabels>,
    options: &PieLabelOptions,
) -> Result<Vec<Text>, PlotError> {
    gca().pie_label(container, labels, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyplot::{close_all, gcf, subplot};

    #[test]
    fn bar_charts_and_labels() {
        close_all();
        let bars = bar(vec![1.0, 2.0], [3.0, 4.0], BarOptions::default()).unwrap();
        assert_eq!(
            bar_label(&bars, &BarLabelOptions::default()).unwrap().len(),
            2
        );
        subplot(2, 2, 2).unwrap();
        barh(vec![1.0, 2.0], [3.0, 4.0], BarOptions::default()).unwrap();
        broken_barh(&[(1.0, 2.0), (4.0, 1.0)], (0.0, 1.0), BandAlign::Bottom);
        subplot(2, 2, 3).unwrap();
        let groups = grouped_bar(
            vec![vec![1.0, 2.0], vec![2.0, 1.0]],
            GroupedBarOptions::default(),
        )
        .unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(gcf().axes().len(), 3);
        close_all();
    }

    #[test]
    fn distributions() {
        close_all();
        let data = [vec![1.0, 2.0, 2.0, 3.0, 5.0], vec![2.0, 3.0, 3.0, 4.0]];
        hist(&data, &HistPlotOptions::default()).unwrap();
        subplot(1, 3, 2).unwrap();
        boxplot(&data, &BoxplotOptions::default()).unwrap();
        subplot(1, 3, 3).unwrap();
        violinplot(&data, &ViolinplotOptions::default()).unwrap();
        assert!(gcf().to_png().is_ok());
        close_all();
    }

    #[test]
    fn pies_and_their_labels() {
        close_all();
        let mut wedges = pie(&[1.0, 2.0, 3.0], &PieOptions::default()).unwrap();
        let texts = pie_label(
            &mut wedges,
            ["a", "b", "c"].as_slice(),
            &PieLabelOptions::default(),
        )
        .unwrap();
        assert_eq!(texts.len(), 3);
        close_all();
    }
}
