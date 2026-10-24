//! Containers: the artists of one bar, error bar or stem plot kept
//! together under one legend label (matplotlib.container).
//!
//! The plotting methods return their container and record it in the axes
//! ([`Axes::containers`], matplotlib's `ax.containers`), where legends find
//! it. The axes keeps the artists' ids rather than handles, so that a
//! figure never holds references to itself.

use std::cell::RefCell;
use std::rc::Rc;

use crate::artist::{ArtistHandle, ArtistId, RawHandle};
use crate::axes::{Axes, AxesState, BarContainer, ErrorbarContainer, StemContainer};
use crate::figure::FigureState;
use crate::orientation::Orientation;

/// The artists of one plotting call (an entry of matplotlib's
/// `ax.containers`).
#[derive(Clone, Debug)]
pub enum Container {
    /// Bars, possibly with error bars (`bar`, `barh`, bar histograms).
    Bar(BarContainer),
    /// Error bars (`errorbar`).
    Errorbar(ErrorbarContainer),
    /// Stems (`stem`).
    Stem(StemContainer),
}

impl Container {
    /// The legend label; `None` and labels starting with `_` keep the
    /// container out of legends.
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Bar(bars) => bars.label.as_deref(),
            Self::Errorbar(errors) => errors.label.as_deref(),
            Self::Stem(stems) => stems.label.as_deref(),
        }
    }

    /// The handles of the container's artists.
    fn artists(&self) -> Vec<RawHandle> {
        match self {
            Self::Bar(bars) => bars
                .patches
                .iter()
                .map(|patch| patch.raw().clone())
                .chain(bars.errorbar.iter().flat_map(errorbar_artists))
                .collect(),
            Self::Errorbar(errors) => errorbar_artists(errors),
            Self::Stem(stems) => vec![
                stems.markerline.raw().clone(),
                stems.stemlines.raw().clone(),
                stems.baseline.raw().clone(),
            ],
        }
    }

    /// Removes the container's artists from their figure and the container
    /// from the [`Axes::containers`] of its axes (matplotlib's
    /// `Container.remove`). A container without any artist (the bars of
    /// no data, error bars with neither bars nor a data line) is found
    /// through none, so it stays recorded.
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let stems = ax.stem([1.0, 2.0], [1.0, 2.0], Default::default())?;
    /// stems.remove();
    /// assert!(ax.lines().is_empty() && ax.collections().is_empty());
    /// assert!(ax.containers().is_empty());
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn remove(&self) {
        let artists = self.artists();
        let Some(first) = artists.first() else {
            return;
        };
        let recorded = ContainerIds::from(self);
        let mut state = first.figure_state().borrow_mut();
        let ids: Vec<ArtistId> = state.store.ids().collect();
        for id in ids {
            if let Some(axes) = state.store.get_as_mut::<AxesState>(id) {
                axes.containers.retain(|ids| *ids != recorded);
            }
        }
        for artist in &artists {
            state.remove_artist(artist.id());
        }
    }
}

/// The handles of an error bar plot's artists.
fn errorbar_artists(errors: &ErrorbarContainer) -> Vec<RawHandle> {
    errors
        .data_line
        .iter()
        .map(|line| line.raw().clone())
        .chain(errors.caplines.iter().map(|line| line.raw().clone()))
        .chain(errors.barlinecols.iter().map(|bars| bars.raw().clone()))
        .collect()
}

impl BarContainer {
    /// Removes the bars and their error bars (see [`Container::remove`]).
    pub fn remove(&self) {
        Container::Bar(self.clone()).remove();
    }
}

impl ErrorbarContainer {
    /// Removes the data line, caps and bars (see [`Container::remove`]).
    pub fn remove(&self) {
        Container::Errorbar(self.clone()).remove();
    }
}

impl StemContainer {
    /// Removes the heads, stems and baseline (see [`Container::remove`]).
    pub fn remove(&self) {
        Container::Stem(self.clone()).remove();
    }
}

impl From<BarContainer> for Container {
    fn from(bars: BarContainer) -> Self {
        Self::Bar(bars)
    }
}

impl From<ErrorbarContainer> for Container {
    fn from(errors: ErrorbarContainer) -> Self {
        Self::Errorbar(errors)
    }
}

impl From<StemContainer> for Container {
    fn from(stems: StemContainer) -> Self {
        Self::Stem(stems)
    }
}

/// The ids of an error bar plot's artists.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ErrorbarIds {
    pub(crate) data_line: Option<ArtistId>,
    pub(crate) caplines: Vec<ArtistId>,
    pub(crate) barlinecols: Vec<ArtistId>,
    pub(crate) has_xerr: bool,
    pub(crate) has_yerr: bool,
    pub(crate) label: Option<String>,
}

/// A [`Container`] as its axes records it: by artist id.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ContainerIds {
    /// See [`BarContainer`].
    Bar {
        patches: Vec<ArtistId>,
        errorbar: Option<ErrorbarIds>,
        datavalues: Vec<f64>,
        orientation: Orientation,
        label: Option<String>,
    },
    /// See [`ErrorbarContainer`].
    Errorbar(ErrorbarIds),
    /// See [`StemContainer`].
    Stem {
        markerline: ArtistId,
        stemlines: ArtistId,
        baseline: ArtistId,
        label: Option<String>,
    },
}

impl ContainerIds {
    /// The legend label.
    pub(crate) fn label(&self) -> Option<&str> {
        match self {
            Self::Bar { label, .. }
            | Self::Errorbar(ErrorbarIds { label, .. })
            | Self::Stem { label, .. } => label.as_deref(),
        }
    }

    /// The handles again, in `figure`.
    fn to_container(&self, figure: &Rc<RefCell<FigureState>>) -> Container {
        let raw = |id: ArtistId| RawHandle::new(Rc::clone(figure), id);
        match self {
            Self::Bar {
                patches,
                errorbar,
                datavalues,
                orientation,
                label,
            } => Container::Bar(BarContainer {
                patches: patches
                    .iter()
                    .map(|&id| crate::patches::Patch::from_raw(raw(id)))
                    .collect(),
                errorbar: errorbar.as_ref().map(|ids| ids.to_container(figure)),
                datavalues: datavalues.clone(),
                orientation: *orientation,
                label: label.clone(),
            }),
            Self::Errorbar(ids) => Container::Errorbar(ids.to_container(figure)),
            Self::Stem {
                markerline,
                stemlines,
                baseline,
                label,
            } => Container::Stem(StemContainer {
                markerline: crate::lines::Line2D::from_raw(raw(*markerline)),
                stemlines: crate::collections::Collection::from_raw(raw(*stemlines)),
                baseline: crate::lines::Line2D::from_raw(raw(*baseline)),
                label: label.clone(),
            }),
        }
    }
}

impl ErrorbarIds {
    fn to_container(&self, figure: &Rc<RefCell<FigureState>>) -> ErrorbarContainer {
        let raw = |id: ArtistId| RawHandle::new(Rc::clone(figure), id);
        ErrorbarContainer {
            data_line: self
                .data_line
                .map(|id| crate::lines::Line2D::from_raw(raw(id))),
            caplines: self
                .caplines
                .iter()
                .map(|&id| crate::lines::Line2D::from_raw(raw(id)))
                .collect(),
            barlinecols: self
                .barlinecols
                .iter()
                .map(|&id| crate::collections::Collection::from_raw(raw(id)))
                .collect(),
            has_xerr: self.has_xerr,
            has_yerr: self.has_yerr,
            label: self.label.clone(),
        }
    }
}

impl From<&ErrorbarContainer> for ErrorbarIds {
    fn from(errors: &ErrorbarContainer) -> Self {
        Self {
            data_line: errors.data_line.as_ref().map(ArtistHandle::id),
            caplines: errors.caplines.iter().map(ArtistHandle::id).collect(),
            barlinecols: errors.barlinecols.iter().map(ArtistHandle::id).collect(),
            has_xerr: errors.has_xerr,
            has_yerr: errors.has_yerr,
            label: errors.label.clone(),
        }
    }
}

impl From<&Container> for ContainerIds {
    fn from(container: &Container) -> Self {
        match container {
            Container::Bar(bars) => Self::Bar {
                patches: bars.patches.iter().map(ArtistHandle::id).collect(),
                errorbar: bars.errorbar.as_ref().map(ErrorbarIds::from),
                datavalues: bars.datavalues.clone(),
                orientation: bars.orientation,
                label: bars.label.clone(),
            },
            Container::Errorbar(errors) => Self::Errorbar(errors.into()),
            Container::Stem(stems) => Self::Stem {
                markerline: stems.markerline.id(),
                stemlines: stems.stemlines.id(),
                baseline: stems.baseline.id(),
                label: stems.label.clone(),
            },
        }
    }
}

impl Axes {
    /// Records a container so that legends find it (matplotlib's
    /// `add_container`); the plotting methods that return containers do
    /// this themselves.
    pub fn add_container(&self, container: impl Into<Container>) -> &Self {
        let ids = ContainerIds::from(&container.into());
        self.raw()
            .with_mut::<AxesState, _>(|axes| axes.containers.push(ids));
        self
    }

    /// The containers of the axes in the order they were added
    /// (matplotlib's `ax.containers`).
    pub fn containers(&self) -> Vec<Container> {
        let figure = self.raw().figure_state();
        let ids = self
            .raw()
            .with::<AxesState, _>(|axes| axes.containers.clone());
        ids.iter().map(|ids| ids.to_container(figure)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::{BarOptions, ErrorbarOptions, StemOptions};
    use crate::figure::Figure;

    #[test]
    fn plotting_calls_record_their_containers_in_order() {
        let ax = Figure::new().subplot();
        let errors = ErrorbarOptions {
            yerr: Some(0.5.into()),
            ..ErrorbarOptions::default()
        };
        let bars = BarOptions {
            label: Some("bars".into()),
            errors,
            ..BarOptions::default()
        };
        ax.bar([1.0, 2.0], [3.0, 4.0], bars).unwrap();
        let stems = StemOptions {
            label: Some("stems".into()),
            ..StemOptions::default()
        };
        ax.stem([1.0], [2.0], stems).unwrap();
        let containers = ax.containers();
        let labels: Vec<Option<&str>> = containers.iter().map(Container::label).collect();
        assert_eq!(labels, [Some("_nolegend_"), Some("bars"), Some("stems")]);
        let Container::Bar(recorded) = &containers[1] else {
            panic!("the second container holds the bars");
        };
        assert_eq!(recorded.patches.len(), 2);
        assert_eq!(recorded.datavalues, [3.0, 4.0]);
        assert!(recorded.errorbar.is_some());
    }

    /// `[lines, collections, patches, containers]` of `ax`.
    fn counts(ax: &Axes) -> [usize; 4] {
        [
            ax.lines().len(),
            ax.collections().len(),
            ax.patches().len(),
            ax.containers().len(),
        ]
    }

    #[test]
    fn removing_a_container_removes_its_artists_and_entry() {
        let ax = Figure::new().subplot();
        let kept = ax
            .stem([1.0], [2.0], StemOptions::default())
            .expect("stems");
        let errors = ErrorbarOptions {
            xerr: Some(1.0.into()),
            fmt: crate::axes::ErrorbarFormat::None,
            ..ErrorbarOptions::default()
        };
        let bars = ax.errorbar([1.0], [1.0], errors).expect("error bars");
        assert_eq!(counts(&ax), [2, 2, 0, 2]);
        bars.remove();
        assert_eq!(counts(&ax), [2, 1, 0, 1], "the stems stay");
        let Container::Stem(left) = &ax.containers()[0] else {
            panic!("the stems remain recorded");
        };
        assert_eq!(left, &kept);
        kept.remove();
        assert_eq!(counts(&ax), [0, 0, 0, 0]);
    }

    #[test]
    fn removing_bars_leaves_their_error_bar_entry() {
        let ax = Figure::new().subplot();
        let options = BarOptions {
            errors: ErrorbarOptions {
                yerr: Some(0.5.into()),
                ..ErrorbarOptions::default()
            },
            ..BarOptions::default()
        };
        let bars = ax.bar([1.0, 2.0], [3.0, 4.0], options).expect("bars");
        assert_eq!(counts(&ax), [0, 1, 2, 2]);
        bars.remove();
        // matplotlib removes the error bars' artists with the bars but
        // keeps their container in `ax.containers`.
        assert_eq!(counts(&ax), [0, 0, 0, 1]);
    }
}
