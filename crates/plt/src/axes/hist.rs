//! Histograms (matplotlib's `hist`).

use super::broadcast::broadcast;
use super::{Axes, BarAlign, BarContainer, BarOptions, DatasetLabels, PlotError};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::patches::{Patch, PatchProps, PatchShape, Polygon};
use crate::rc::HistBins;
use crate::scale::LogScale;
use crate::stats::{
    BinSample, Bins, Cumulative, Datasets, HistAlign, HistBars, HistBottom, HistOptions,
    HistOrientation, HistPatches, HistType, StepOutline, hist,
};
use crate::ticker::{AxisName, LogBase};

/// The most bars a bar histogram draws, its datasets together: each is a
/// patch, and matplotlib makes as many as the bins ask for (a million bins
/// take it minutes). Step histograms draw one outline per dataset and have
/// no such bound.
pub const MAX_HIST_BARS: usize = 1 << 16;

/// Options of [`Axes::hist`] (matplotlib's keywords): the binning and
/// layout [`stats::hist`](crate::stats::hist) takes as a
/// [`HistOptions`], whose fields come first and are documented there,
/// then the styling. A [`HistOptions`] converts into them, with the
/// default styling.
#[derive(Clone, Debug, Default)]
pub struct HistPlotOptions<'a> {
    /// The bins ([`HistOptions::bins`]); `None` takes `hist.bins`.
    pub bins: Option<Bins>,
    /// The range of equal-width bins ([`HistOptions::range`]).
    pub range: Option<(f64, f64)>,
    /// Normalise to densities ([`HistOptions::density`]).
    pub density: bool,
    /// One weight array per dataset ([`HistOptions::weights`]).
    pub weights: Option<&'a [Vec<f64>]>,
    /// Accumulate counts along the bins ([`HistOptions::cumulative`]).
    pub cumulative: Cumulative,
    /// Stack datasets on top of each other ([`HistOptions::stacked`]).
    pub stacked: bool,
    /// Bars or steps ([`HistOptions::histtype`]).
    pub histtype: HistType,
    /// Placement relative to the bin edges ([`HistOptions::align`]).
    pub align: HistAlign,
    /// Which way the bars grow ([`HistOptions::orientation`]).
    pub orientation: HistOrientation,
    /// The relative bar width ([`HistOptions::rwidth`]).
    pub rwidth: Option<f64>,
    /// The baseline of the first dataset ([`HistOptions::bottom`]).
    pub bottom: HistBottom,
    /// Use a log scale (non-positive counts clipped) on the count axis.
    pub log: bool,
    /// One colour for every dataset or one per dataset (default: the next
    /// colours of the line property cycle).
    pub color: Vec<Color>,
    /// Legend labels: one for the first dataset (`"label".into()`), or
    /// one per dataset (`["first", "second"].into()`).
    pub label: DatasetLabels,
    /// Properties applied to every patch.
    pub props: PatchProps,
    /// Face colours cycled over the datasets (matplotlib's sequence of
    /// `facecolor`s); empty keeps the dataset colours.
    pub facecolors: Vec<Color>,
    /// Edge colours cycled over the datasets; empty keeps the patch
    /// default (the dataset colours for `step` histograms).
    pub edgecolors: Vec<Color>,
    /// Hatch patterns cycled over the datasets.
    pub hatches: Vec<String>,
    /// Edge widths in points cycled over the datasets.
    pub linewidths: Vec<f64>,
    /// Edge styles cycled over the datasets.
    pub linestyles: Vec<LineStyle>,
}

impl<'a> From<HistOptions<'a>> for HistPlotOptions<'a> {
    /// The binning and layout of `options`, with the default styling.
    fn from(options: HistOptions<'a>) -> Self {
        let HistOptions {
            bins,
            range,
            density,
            weights,
            cumulative,
            stacked,
            histtype,
            align,
            orientation,
            rwidth,
            bottom,
        } = options;
        Self {
            bins,
            range,
            density,
            weights,
            cumulative,
            stacked,
            histtype,
            align,
            orientation,
            rwidth,
            bottom,
            ..Self::default()
        }
    }
}

impl<'a> HistPlotOptions<'a> {
    /// The binning and layout, as [`stats::hist`](crate::stats::hist)
    /// takes them.
    fn binning(&self) -> HistOptions<'a> {
        HistOptions {
            bins: self.bins.clone(),
            range: self.range,
            density: self.density,
            weights: self.weights,
            cumulative: self.cumulative,
            stacked: self.stacked,
            histtype: self.histtype,
            align: self.align,
            orientation: self.orientation,
            rwidth: self.rwidth,
            bottom: self.bottom.clone(),
        }
    }

    /// The properties of the patches of dataset `index`: the shared
    /// [`props`](Self::props) with the entries of the style sequences for
    /// that dataset on top (matplotlib's `style_generator`).
    fn dataset_props(&self, index: usize) -> PatchProps {
        let mut props = self.props.clone();
        if let Some(color) = cycled(&self.facecolors, index) {
            props.facecolor = Some(color.clone());
        }
        if let Some(color) = cycled(&self.edgecolors, index) {
            props.edgecolor = Some(color.clone());
        }
        if let Some(hatch) = cycled(&self.hatches, index) {
            props.hatch = Some(hatch.clone());
        }
        if let Some(&width) = cycled(&self.linewidths, index) {
            props.linewidth = Some(width);
        }
        if let Some(style) = cycled(&self.linestyles, index) {
            props.linestyle = Some(style.clone());
        }
        props
    }
}

/// Entry `index` of `values` repeated cyclically (`None` when empty).
fn cycled<T>(values: &[T], index: usize) -> Option<&T> {
    values.iter().cycle().nth(index)
}

/// What [`Axes::hist`] computed and drew.
#[derive(Clone, Debug)]
pub struct HistPlot {
    /// The bar heights of every dataset (stacked and cumulated as asked).
    pub tops: Vec<Vec<f64>>,
    /// The bin edges.
    pub edges: Vec<f64>,
    /// The patches of every dataset.
    pub artists: HistArtists,
}

/// The patches of a histogram.
#[derive(Clone, Debug)]
pub enum HistArtists {
    /// The bars of every dataset (bar types).
    Bars(Vec<BarContainer>),
    /// One outline per dataset (step types).
    Steps(Vec<Patch<Polygon>>),
}

impl Axes {
    /// Histograms of one dataset, or of several side by side (see
    /// [`Datasets`]; matplotlib's `hist`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::stats::Bins;
    /// let ax = Figure::new().subplot();
    /// let samples = [0.0, 0.5, 1.5, 2.0];
    /// let plot = ax.hist(&samples, HistPlotOptions::default())?;
    /// assert_eq!(plot.edges.len(), 11);
    /// let two = HistPlotOptions { bins: Some(Bins::Count(2)), ..HistPlotOptions::default() };
    /// let stacked = ax.hist(&[&samples[..2], &samples[2..]], two)?;
    /// assert_eq!(stacked.tops, [[2.0, 0.0], [0.0, 2.0]]);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the binning is invalid (more than [`MAX_EQUAL_BINS`] bins
    /// included), weights do not match the data, the colours do not match
    /// the datasets, or a bar histogram would draw more than
    /// [`MAX_HIST_BARS`] bars.
    ///
    /// [`MAX_EQUAL_BINS`]: crate::stats::MAX_EQUAL_BINS
    pub fn hist<T: BinSample, D: Datasets<T> + ?Sized>(
        &self,
        datasets: &D,
        options: HistPlotOptions<'_>,
    ) -> Result<HistPlot, PlotError> {
        let datasets = datasets.datasets();
        let colors = if options.color.is_empty() {
            datasets.iter().map(|_| self.next_line_color()).collect()
        } else {
            broadcast("color", &options.color, datasets.len())?
        };
        let mut layout = options.binning();
        if layout.bins.is_none() {
            layout.bins = Some(rc_bins(&crate::rc::params().hist_bins)?);
        }
        let computed = hist(datasets.as_slice(), layout).map_err(PlotError::Stats)?;
        let artists = match computed.patches {
            HistPatches::Bars(groups) => {
                let bars = groups.iter().map(|group| group.positions.len()).sum();
                if bars > MAX_HIST_BARS {
                    return Err(PlotError::TooManyBars(bars));
                }
                HistArtists::Bars(self.hist_bars(groups, &colors, &options)?)
            }
            HistPatches::Steps(outlines) => {
                HistArtists::Steps(self.hist_steps(outlines, &colors, &options))
            }
        };
        artists.style(&options);
        Ok(HistPlot {
            tops: computed.tops,
            edges: computed.edges,
            artists,
        })
    }

    /// One `bar`/`barh` call per dataset; only the lowest bars keep their
    /// sticky edges so margins can grow past the inner ones.
    fn hist_bars(
        &self,
        groups: Vec<HistBars>,
        colors: &[Color],
        options: &HistPlotOptions<'_>,
    ) -> Result<Vec<BarContainer>, PlotError> {
        let horizontal = options.orientation == HistOrientation::Horizontal;
        let containers = groups
            .into_iter()
            .zip(colors)
            .map(|(group, color)| {
                let bars = BarOptions {
                    thickness: group.thickness.into(),
                    base: group.bases.into(),
                    align: BarAlign::Center,
                    color: vec![color.clone()],
                    log: options.log,
                    ..BarOptions::default()
                };
                if horizontal {
                    self.barh(group.positions, group.lengths, bars)
                } else {
                    self.bar(group.positions, group.lengths, bars)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        for patch in containers.iter().skip(1).flat_map(|c| &c.patches) {
            patch.set_sticky_edges(Vec::new(), Vec::new());
        }
        Ok(containers)
    }

    /// One polygon per dataset, added in reverse so lower stacked outlines
    /// are drawn on top; every outline sticks to zero counts.
    fn hist_steps(
        &self,
        outlines: Vec<StepOutline>,
        colors: &[Color],
        options: &HistPlotOptions<'_>,
    ) -> Vec<Patch<Polygon>> {
        if options.log {
            let axis = match options.orientation {
                HistOrientation::Vertical => AxisName::Y,
                HistOrientation::Horizontal => AxisName::X,
            };
            self.set_scale(axis, Box::new(LogScale::from_base(LogBase::TEN)));
        }
        let filled = options.histtype == HistType::StepFilled;
        let mut patches: Vec<Patch<Polygon>> = outlines
            .into_iter()
            .zip(colors)
            .rev()
            .map(|(outline, color)| {
                let mut props = PatchProps::default().facecolor(color.clone()).fill(filled);
                if !filled {
                    props = props.edgecolor(color.clone()).zorder(2.0);
                }
                let patch = self.add_polygon(Polygon::new(outline.vertices, outline.closed), props);
                match options.orientation {
                    HistOrientation::Vertical => patch.set_sticky_edges(Vec::new(), vec![0.0]),
                    HistOrientation::Horizontal => patch.set_sticky_edges(vec![0.0], Vec::new()),
                };
                patch
            })
            .collect();
        patches.reverse();
        patches
    }
}

impl HistArtists {
    /// Applies the shared properties and the style sequences to every
    /// patch and labels the first patch of each dataset (matplotlib's
    /// styling loop at the end of `hist`).
    fn style(&self, options: &HistPlotOptions<'_>) {
        let label = |i: usize| options.label.get(i);
        match self {
            Self::Bars(containers) => {
                for (i, container) in containers.iter().enumerate() {
                    style_dataset(&container.patches, label(i), &options.dataset_props(i));
                }
            }
            Self::Steps(outlines) => {
                for (i, outline) in outlines.iter().enumerate() {
                    let props = options.dataset_props(i);
                    style_dataset(std::slice::from_ref(outline), label(i), &props);
                }
            }
        }
    }
}

/// Styles the patches of one dataset; only the first carries the label.
fn style_dataset<S: PatchShape>(patches: &[Patch<S>], label: Option<&str>, props: &PatchProps) {
    for (i, patch) in patches.iter().enumerate() {
        patch.update_props(props.clone());
        match (i, label) {
            (0, Some(label)) => {
                patch.set_label(label);
            }
            (0, None) => {}
            _ => {
                patch.set_label("_nolegend_");
            }
        }
    }
}

/// The bins `hist.bins` asks for.
fn rc_bins(bins: &HistBins) -> Result<Bins, PlotError> {
    Ok(match bins {
        HistBins::Count(count) => Bins::Count(*count),
        HistBins::Rule(rule) => Bins::Estimator(rule.parse().map_err(PlotError::Stats)?),
        HistBins::Edges(edges) => Bins::Edges(edges.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::patches::PatchStyle;

    fn three_datasets() -> Vec<Vec<f64>> {
        vec![vec![0.0, 1.0], vec![1.0, 2.0], vec![2.0, 3.0]]
    }

    #[test]
    fn style_sequences_cycle_over_the_datasets() {
        let ax = Figure::new().subplot();
        let options = HistPlotOptions {
            facecolors: vec![Color::from("b"), Color::from("g")],
            hatches: vec!["/".into()],
            linewidths: vec![1.0, 1.5, 2.0],
            linestyles: vec![LineStyle::Dashed, LineStyle::Dotted],
            props: PatchProps::default().edgecolor("k"),
            ..HistPlotOptions::default()
        };
        let plot = ax.hist(&three_datasets(), options).unwrap();
        let HistArtists::Bars(containers) = plot.artists else {
            panic!("bars expected");
        };
        let styles: Vec<PatchStyle> = containers
            .iter()
            .map(|container| container.patches[0].style())
            .collect();
        let faces: Vec<_> = styles.iter().map(|s| s.facecolor.clone()).collect();
        assert_eq!(faces, ["b", "g", "b"].map(|c| Some(Color::from(c))));
        assert!(styles.iter().all(|s| s.hatch.as_deref() == Some("/")));
        assert!(styles.iter().all(|s| s.edgecolor == Some(Color::from("k"))));
        let widths: Vec<_> = styles.iter().map(|s| s.linewidth).collect();
        assert_eq!(widths, [Some(1.0), Some(1.5), Some(2.0)]);
        let dashes: Vec<_> = styles.iter().map(|s| s.linestyle.clone()).collect();
        assert_eq!(
            dashes,
            [LineStyle::Dashed, LineStyle::Dotted, LineStyle::Dashed]
        );
    }

    #[test]
    fn bar_counts_are_bounded_but_step_outlines_are_not() {
        // Found by fuzzing: a million bins of three stacked datasets made
        // three million patches.
        let ax = Figure::new().subplot();
        let with = |histtype| HistPlotOptions {
            bins: Some(crate::stats::Bins::Count(1 << 20)),
            histtype,
            ..HistPlotOptions::default()
        };
        assert_eq!(
            ax.hist(&three_datasets(), with(HistType::BarStacked)).err(),
            Some(PlotError::TooManyBars(3 << 20))
        );
        assert!(ax.hist(&three_datasets(), with(HistType::Step)).is_ok());
    }

    #[test]
    fn step_outlines_take_edge_sequences_over_the_dataset_colours() {
        let ax = Figure::new().subplot();
        let step = |edgecolors: Vec<Color>| HistPlotOptions {
            histtype: HistType::Step,
            color: vec![Color::from("r")],
            edgecolors,
            ..HistPlotOptions::default()
        };
        let second_edge = |options: HistPlotOptions<'_>| {
            let plot = ax.hist(&three_datasets(), options).unwrap();
            let HistArtists::Steps(outlines) = plot.artists else {
                panic!("step outlines expected");
            };
            outlines[1].style().edgecolor
        };
        assert_eq!(second_edge(step(Vec::new())), Some(Color::from("r")));
        let sequence = vec![Color::from("b"), Color::from("g")];
        assert_eq!(second_edge(step(sequence)), Some(Color::from("g")));
    }
}
