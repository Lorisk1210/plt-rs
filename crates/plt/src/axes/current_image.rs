//! The current image of an axes: the colour-mapped artist that pyplot's
//! colormap functions, `clim` and `colorbar` act on (matplotlib's
//! `Axes._sci`, `Axes._gci` and `Figure._gci`).

use std::rc::Rc;

use super::{Axes, AxesState, PlotError};
use crate::artist::{ArtistHandle, ArtistId, RawHandle};
use crate::collections::Collection;
use crate::colorbar::Mappable;
use crate::contour::{ContourSet, ContourState};
use crate::figure::Figure;
use crate::image::AxesImage;

/// A colour-mapped artist that can be the current image of an axes
/// (matplotlib's `ColorizingArtist`).
#[derive(Clone, Debug)]
pub enum ColorizingArtist {
    /// An image (`imshow`, `matshow`, `spy`, `specgram`).
    Image(AxesImage),
    /// A collection (`scatter`, `pcolor`, `pcolormesh`, `hexbin`,
    /// `hist2d`, `quiver`, `streamplot`, flat `tripcolor`).
    Collection(Collection),
    /// Line or filled contours as they were when made current: labels
    /// added afterwards are not part of the copy.
    Contour(Box<ContourSet>),
}

impl From<AxesImage> for ColorizingArtist {
    fn from(image: AxesImage) -> Self {
        Self::Image(image)
    }
}

impl From<&AxesImage> for ColorizingArtist {
    fn from(image: &AxesImage) -> Self {
        Self::Image(image.clone())
    }
}

impl From<Collection> for ColorizingArtist {
    fn from(collection: Collection) -> Self {
        Self::Collection(collection)
    }
}

impl From<&Collection> for ColorizingArtist {
    fn from(collection: &Collection) -> Self {
        Self::Collection(collection.clone())
    }
}

impl From<ContourSet> for ColorizingArtist {
    fn from(contours: ContourSet) -> Self {
        Self::Contour(Box::new(contours))
    }
}

impl From<&ContourSet> for ColorizingArtist {
    fn from(contours: &ContourSet) -> Self {
        Self::Contour(Box::new(contours.clone()))
    }
}

impl<'a> From<&'a ColorizingArtist> for Mappable<'a> {
    fn from(artist: &'a ColorizingArtist) -> Self {
        match artist {
            ColorizingArtist::Image(image) => Self::Image(image),
            ColorizingArtist::Collection(collection) => Self::Collection(collection),
            ColorizingArtist::Contour(contours) => Self::Contour(contours),
        }
    }
}

/// What an axes remembers of its current image: artist ids rather than
/// handles, which would keep the figure alive from inside itself.
#[derive(Clone, Debug)]
pub(crate) enum CurrentImage {
    Image(ArtistId),
    Collection(ArtistId),
    Contour {
        collections: Vec<ArtistId>,
        /// The levels and colours, without the labels (whose texts are
        /// handles).
        state: Box<ContourState>,
    },
}

impl ColorizingArtist {
    /// Whether the artist is drawn in `axes`.
    fn is_in(&self, axes: &Axes) -> bool {
        let child_of = |raw: &RawHandle| {
            Rc::ptr_eq(raw.figure_state(), axes.raw().figure_state())
                && raw.is_alive()
                && raw.with_base(|base| base.parent) == Some(axes.id())
        };
        match self {
            Self::Image(image) => child_of(image.raw()),
            Self::Collection(collection) => child_of(collection.raw()),
            Self::Contour(contours) => contours.axes() == axes,
        }
    }

    /// The record an axes keeps of this artist.
    fn record(&self) -> CurrentImage {
        match self {
            Self::Image(image) => CurrentImage::Image(image.id()),
            Self::Collection(collection) => CurrentImage::Collection(collection.id()),
            Self::Contour(contours) => {
                let mut state = contours.state().clone();
                state.labels.clear();
                CurrentImage::Contour {
                    collections: contours.collections().iter().map(Collection::id).collect(),
                    state: Box::new(state),
                }
            }
        }
    }
}

impl Axes {
    /// Makes `image` the current image of the axes (matplotlib's
    /// `Axes._sci`): the artist that pyplot's colormap functions, `clim`
    /// and `colorbar` act on.
    ///
    /// # Errors
    ///
    /// [`PlotError::NotInAxes`] when the artist is not drawn in this axes.
    pub fn set_current_image(
        &self,
        image: impl Into<ColorizingArtist>,
    ) -> Result<&Self, PlotError> {
        let image = image.into();
        if !image.is_in(self) {
            return Err(PlotError::NotInAxes);
        }
        let record = image.record();
        self.raw()
            .with_mut::<AxesState, _>(|axes| axes.current_image = Some(record));
        Ok(self)
    }

    /// The current image of the axes while it is still drawn here
    /// (matplotlib's `Axes._gci`); clearing the axes forgets it.
    pub fn current_image(&self) -> Option<ColorizingArtist> {
        let record = self
            .raw()
            .with::<AxesState, _>(|axes| axes.current_image.clone())?;
        let figure = self.raw().figure_state();
        let handle = |id: ArtistId| {
            let raw = RawHandle::new(Rc::clone(figure), id);
            raw.is_alive().then_some(raw)
        };
        Some(match record {
            CurrentImage::Image(id) => ColorizingArtist::Image(AxesImage::from_raw(handle(id)?)),
            CurrentImage::Collection(id) => {
                ColorizingArtist::Collection(Collection::from_raw(handle(id)?))
            }
            CurrentImage::Contour { collections, state } => {
                let collections = collections
                    .into_iter()
                    .map(|id| handle(id).map(Collection::from_raw))
                    .collect::<Option<Vec<_>>>()?;
                ColorizingArtist::Contour(Box::new(ContourSet::new(
                    self.clone(),
                    collections,
                    *state,
                )))
            }
        })
    }
}

impl Figure {
    /// The current image of the figure (matplotlib's `Figure._gci`): that
    /// of the current axes, or else of the latest axes that has one;
    /// `None` without axes.
    pub fn current_image(&self) -> Option<ColorizingArtist> {
        self.current_axes()?
            .current_image()
            .or_else(|| self.axes().iter().rev().find_map(Axes::current_image))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ImshowOptions;
    use crate::contour::{ContourOptions, QuadField};
    use crate::numeric::Grid;

    fn ramp() -> Grid<f64> {
        Grid::from_fn(3, 4, |row, col| (row * 4 + col) as f64)
    }

    #[test]
    fn images_become_current_explicitly() {
        let fig = Figure::new();
        let ax = fig.subplot();
        assert!(fig.current_image().is_none());
        let image = ax.imshow(ramp(), ImshowOptions::default()).unwrap();
        assert!(
            ax.current_image().is_none(),
            "plotting alone does not track"
        );
        ax.set_current_image(&image).unwrap();
        assert!(matches!(ax.current_image(), Some(ColorizingArtist::Image(im)) if im == image));
        image.remove();
        assert!(
            ax.current_image().is_none(),
            "removed artists are forgotten"
        );
    }

    #[test]
    fn foreign_artists_are_rejected() {
        let fig = Figure::new();
        let (left, right) = (
            fig.add_subplot(1, 2, 1).unwrap(),
            fig.add_subplot(1, 2, 2).unwrap(),
        );
        let points = left.scatter(vec![0.0, 1.0], vec![1.0, 0.0]).unwrap();
        assert_eq!(
            right.set_current_image(&points).unwrap_err(),
            PlotError::NotInAxes
        );
        left.set_current_image(&points).unwrap();
        assert!(
            matches!(fig.current_image(), Some(ColorizingArtist::Collection(c)) if c == points)
        );
        right.clear();
        left.clear();
        assert!(fig.current_image().is_none(), "clearing forgets the image");
    }

    #[test]
    fn contour_sets_are_remembered_without_their_labels() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let field = QuadField::new(ramp());
        let contours = ax.contourf(&field, ContourOptions::default()).unwrap();
        ax.set_current_image(&contours).unwrap();
        let Some(ColorizingArtist::Contour(current)) = fig.current_image() else {
            panic!("the contour set is current");
        };
        assert_eq!(current.levels(), contours.levels());
        assert_eq!(current.collections(), contours.collections());
        assert!(
            Mappable::from(&ColorizingArtist::Contour(current))
                .source()
                .is_ok()
        );
    }
}
