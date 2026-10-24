//! Legend labels of box plots (matplotlib's `label` of `bxp` and
//! `boxplot`).

use super::PlotError;
use super::bxp::{BoxArtist, BoxplotArtists};
use crate::artist::{ArtistHandle, RawHandle};

/// The legend labels of a box plot (matplotlib's `label` of `bxp` and
/// `boxplot`). They go to the boxes when these are patches (matplotlib's
/// `patch_artist`), else to the median lines.
///
/// ```
/// use plt::prelude::*;
/// use plt::axes::{BoxLabel, BoxplotOptions};
/// let ax = Figure::new().subplot();
/// let mut options = BoxplotOptions::default();
/// options.draw.label = Some(BoxLabel::Each(vec!["A".into(), "B".into()]));
/// let parts = ax.boxplot(&[vec![1.0, 2.0, 3.0], vec![2.0, 3.0, 4.0]], options)?;
/// assert_eq!(parts.medians[1].label(), "B");
/// assert_eq!(ax.legend_handles_labels().1, ["A", "B"]);
/// # Ok::<(), plt::axes::PlotError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoxLabel {
    /// One label for the first box only, giving a single legend entry for
    /// boxes styled alike (a string `label`).
    First(String),
    /// One label per box (a list `label`).
    Each(Vec<String>),
}

impl BoxLabel {
    /// Checks that the labels fit `boxes` boxes.
    pub(super) fn check(&self, boxes: usize) -> Result<(), PlotError> {
        match self {
            Self::Each(labels) if labels.len() != boxes => Err(PlotError::DatasetCount {
                argument: "label",
                len: labels.len(),
                datasets: boxes,
            }),
            _ => Ok(()),
        }
    }

    /// Labels the boxes of `artists` if they are patches, else its medians.
    pub(super) fn apply(&self, artists: &BoxplotArtists) {
        let patches = artists
            .boxes
            .iter()
            .any(|drawn| matches!(drawn, BoxArtist::Patch(_)));
        let targets: Vec<&RawHandle> = if patches {
            artists
                .boxes
                .iter()
                .map(|drawn| match drawn {
                    BoxArtist::Line(line) => line.raw(),
                    BoxArtist::Patch(patch) => patch.raw(),
                })
                .collect()
        } else {
            artists.medians.iter().map(ArtistHandle::raw).collect()
        };
        match self {
            Self::First(label) => {
                if let Some(first) = targets.first() {
                    first.set_label(label.clone());
                }
            }
            Self::Each(labels) => {
                for (target, label) in targets.into_iter().zip(labels) {
                    target.set_label(label.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::{BoxStyle, BoxplotOptions};
    use crate::figure::Figure;
    use crate::patches::PatchProps;

    fn data() -> Vec<Vec<f64>> {
        (0..3)
            .map(|k| (0..5).map(|i| f64::from(i + k)).collect())
            .collect()
    }

    #[test]
    fn patch_boxes_carry_the_labels() {
        let ax = Figure::new().subplot();
        let mut options = BoxplotOptions::default();
        options.draw.boxes = Some(BoxStyle::Patch(PatchProps::default()));
        options.draw.label = Some(BoxLabel::Each(vec!["a".into(), "b".into(), "c".into()]));
        let parts = ax.boxplot(&data(), options).unwrap();
        let labels: Vec<String> = parts
            .boxes
            .iter()
            .map(|drawn| match drawn {
                BoxArtist::Patch(patch) => patch.label(),
                BoxArtist::Line(line) => line.label(),
            })
            .collect();
        assert_eq!(labels, ["a", "b", "c"]);
        assert!(parts.medians.iter().all(|m| m.label() == "_nolegend_"));
    }

    #[test]
    fn one_label_goes_to_the_first_median_only() {
        let ax = Figure::new().subplot();
        let mut options = BoxplotOptions::default();
        options.draw.label = Some(BoxLabel::First("all".into()));
        let parts = ax.boxplot(&data(), options).unwrap();
        assert_eq!(parts.medians[0].label(), "all");
        assert!(
            parts.medians[1..]
                .iter()
                .all(|m| m.label().starts_with('_'))
        );
        assert_eq!(ax.legend_handles_labels().1, ["all"]);
    }

    #[test]
    fn a_label_per_box_must_match_the_boxes() {
        let ax = Figure::new().subplot();
        let mut options = BoxplotOptions::default();
        options.draw.label = Some(BoxLabel::Each(vec!["a".into()]));
        assert_eq!(
            ax.boxplot(&data(), options),
            Err(PlotError::DatasetCount {
                argument: "label",
                len: 1,
                datasets: 3
            })
        );
        assert_eq!(ax.lines(), Vec::new(), "nothing is drawn");
    }
}
