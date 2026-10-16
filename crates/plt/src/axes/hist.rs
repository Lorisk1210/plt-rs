//! Histograms (matplotlib's `hist`).

use super::broadcast::broadcast;
use super::{Axes, BarAlign, BarContainer, BarOptions, PlotError};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::patches::{Patch, PatchProps, PatchShape, Polygon};
use crate::scale::{LogScale, NonPositive};
use crate::stats::{
    BinSample, HistBars, HistOptions, HistOrientation, HistPatches, HistType, StepOutline, hist,
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
    /// let plot = ax.hist(&[[0.0, 1.0, 1.5, 2.0]], &HistPlotOptions::default())?;
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
        options: &HistPlotOptions<'_>,
    ) -> Result<HistPlot, PlotError> {
        let colors = if options.color.is_empty() {
            datasets.iter().map(|_| self.next_line_color()).collect()
        } else {
            broadcast("color", &options.color, datasets.len())?
        };
        let computed = hist(datasets, &options.layout).map_err(PlotError::Stats)?;
        let artists = match computed.patches {
            HistPatches::Bars(groups) => {
                HistArtists::Bars(self.hist_bars(groups, &colors, options)?)
            }
            HistPatches::Steps(outlines) => {
                HistArtists::Steps(self.hist_steps(outlines, &colors, options))
            }
        };
        artists.style(options);
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
    /// Applies the shared properties to every patch and labels the first
    /// patch of each dataset (matplotlib's styling loop at the end of
    /// `hist`).
    fn style(&self, options: &HistPlotOptions<'_>) {
        let label = |i: usize| options.label.get(i);
        match self {
            Self::Bars(containers) => {
                for (i, container) in containers.iter().enumerate() {
                    style_dataset(&container.patches, label(i), &options.props);
                }
            }
            Self::Steps(outlines) => {
                for (i, outline) in outlines.iter().enumerate() {
                    style_dataset(std::slice::from_ref(outline), label(i), &options.props);
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
