//! Histograms (matplotlib's `hist`).

use super::broadcast::broadcast;
use super::{Axes, BarAlign, BarContainer, BarOptions, PlotError};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::patches::{Patch, PatchProps, PatchShape, Polygon};
use crate::rc::HistBins;
use crate::scale::{LogScale, NonPositive};
use crate::stats::{
    BinSample, Bins, HistBars, HistOptions, HistOrientation, HistPatches, HistType, StepOutline,
    hist,
};
use crate::ticker::AxisName;

/// Options of [`Axes::hist`]: binning and layout plus styling.
#[derive(Clone, Debug, Default)]
pub struct HistPlotOptions<'a> {
    /// Bins, normalisation, stacking, histogram type and orientation.
    pub layout: HistOptions<'a>,
    /// Use a log scale (non-positive counts clipped) on the count axis.
    pub log: bool,
    /// One colour for every dataset or one per dataset (default: the next
    /// colours of the line property cycle).
    pub color: Vec<Color>,
    /// Legend labels, one per dataset.
    pub label: Vec<String>,
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

impl HistPlotOptions<'_> {
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
    /// Histograms of one or more datasets (matplotlib's `hist`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::HistPlotOptions;
    /// let ax = Figure::new().subplot();
    /// let plot = ax.hist(&[[0.0, 1.0, 1.5, 2.0]], HistPlotOptions::default())?;
    /// assert_eq!(plot.edges.len(), 11);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the binning is invalid, weights do not match the data, or the
    /// colours do not match the datasets.
    pub fn hist<T: BinSample, D: AsRef<[T]>>(
        &self,
        datasets: &[D],
        options: HistPlotOptions<'_>,
    ) -> Result<HistPlot, PlotError> {
        let colors = if options.color.is_empty() {
            datasets.iter().map(|_| self.next_line_color()).collect()
        } else {
            broadcast("color", &options.color, datasets.len())?
        };
        let mut layout = options.layout.clone();
        if layout.bins.is_none() {
            layout.bins = Some(rc_bins(&crate::rc::params().hist_bins)?);
        }
        let computed = hist(datasets, layout).map_err(PlotError::Stats)?;
        let artists = match computed.patches {
            HistPatches::Bars(groups) => {
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
        let horizontal = options.layout.orientation == HistOrientation::Horizontal;
        let containers = groups
            .into_iter()
            .zip(colors)
            .map(|(group, color)| {
                let bars = BarOptions {
                    thickness: group.thickness,
                    base: group.bases,
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
            let axis = match options.layout.orientation {
                HistOrientation::Vertical => AxisName::Y,
                HistOrientation::Horizontal => AxisName::X,
            };
            let log = LogScale::new(10.0)
                .expect("10 is a valid logarithm base")
                .with_nonpositive(NonPositive::Clip);
            self.set_scale(axis, Box::new(log));
        }
        let filled = options.layout.histtype == HistType::StepFilled;
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
                match options.layout.orientation {
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
fn style_dataset<S: PatchShape>(patches: &[Patch<S>], label: Option<&String>, props: &PatchProps) {
    for (i, patch) in patches.iter().enumerate() {
        patch.update_props(props.clone());
        match (i, label) {
            (0, Some(label)) => {
                patch.set_label(label.clone());
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
    fn step_outlines_take_edge_sequences_over_the_dataset_colours() {
        let ax = Figure::new().subplot();
        let step = |edgecolors: Vec<Color>| HistPlotOptions {
            layout: HistOptions {
                histtype: HistType::Step,
                ..HistOptions::default()
            },
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
