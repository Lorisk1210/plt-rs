//! The current image of an axes: the colour-mapped artist that pyplot's
//! colormap functions, `clim` and `colorbar` act on (matplotlib's
//! `Axes._sci`, `Axes._gci` and `Figure._gci`).

use std::rc::Rc;

use super::{Axes, AxesState, PlotError, Tripcolor};
use crate::artist::{ArtistHandle, ArtistId, RawHandle};
use crate::collections::{Collection, TriMesh};
use crate::colorbar::{Colorbar, Mappable};
use crate::colors::Colormap;
use crate::contour::ContourSet;
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
    /// A Gouraud-shaded triangle mesh (`tripcolor` with Gouraud shading).
    TriMesh(TriMesh),
    /// Line or filled contours.
    Contour(ContourSet),
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

impl From<TriMesh> for ColorizingArtist {
    fn from(mesh: TriMesh) -> Self {
        Self::TriMesh(mesh)
    }
}

impl From<&TriMesh> for ColorizingArtist {
    fn from(mesh: &TriMesh) -> Self {
        Self::TriMesh(mesh.clone())
    }
}

impl From<&Tripcolor> for ColorizingArtist {
    fn from(plot: &Tripcolor) -> Self {
        match plot {
            Tripcolor::Flat(collection) => collection.into(),
            Tripcolor::Gouraud(mesh) => mesh.into(),
        }
    }
}

impl From<ContourSet> for ColorizingArtist {
    fn from(contours: ContourSet) -> Self {
        Self::Contour(contours)
    }
}

impl From<&ContourSet> for ColorizingArtist {
    fn from(contours: &ContourSet) -> Self {
        Self::Contour(contours.clone())
    }
}

impl<'a> From<&'a ColorizingArtist> for Mappable<'a> {
    fn from(artist: &'a ColorizingArtist) -> Self {
        match artist {
            ColorizingArtist::Image(image) => Self::Image(image),
            ColorizingArtist::Collection(collection) => Self::Collection(collection),
            ColorizingArtist::TriMesh(mesh) => Self::TriMesh(mesh),
            ColorizingArtist::Contour(contours) => Self::Contour(contours),
        }
    }
}

/// What an axes remembers of its current image: artist ids rather than
/// handles, which would keep the figure alive from inside itself.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CurrentImage {
    Image(ArtistId),
    Collection(ArtistId),
    TriMesh(ArtistId),
    /// The artist keeping the contour set's state.
    Contour(ArtistId),
}

impl ColorizingArtist {
    /// The colorbar made last for the artist, while it shows the artist
    /// (matplotlib's `mappable.colorbar`).
    ///
    /// ```
    /// use plt::axes::{ColorizingArtist, ImshowOptions};
    /// use plt::colorbar::ColorbarOptions;
    /// use plt::numeric::Grid;
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let image = ColorizingArtist::from(ax.imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())?);
    /// assert!(image.colorbar().is_none());
    /// let colorbar = fig.colorbar(&image, ColorbarOptions::default())?;
    /// assert_eq!(image.colorbar().map(|c| c.ax().clone()), Some(colorbar.ax().clone()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn colorbar(&self) -> Option<Colorbar> {
        crate::colorbar::colorbar_of(self.into())
    }

    /// Replaces the colormap (matplotlib's `set_cmap`); the artist and
    /// its colorbars are recoloured.
    pub fn set_cmap(&self, cmap: Colormap) -> &Self {
        match self {
            Self::Image(image) => {
                image.set_cmap(cmap);
            }
            Self::Collection(collection) => {
                collection.set_cmap(cmap);
            }
            Self::TriMesh(mesh) => {
                mesh.set_cmap(cmap);
            }
            Self::Contour(contours) => {
                contours.set_cmap(cmap);
            }
        }
        self
    }

    /// Sets the data values mapped to the ends of the colormap
    /// (matplotlib's `set_clim`); `None` leaves a limit unchanged. The
    /// artist and its colorbars are recoloured.
    pub fn set_clim(&self, vmin: Option<f64>, vmax: Option<f64>) -> &Self {
        match self {
            Self::Image(image) => {
                image.set_clim(vmin, vmax);
            }
            Self::Collection(collection) => {
                collection.set_clim(vmin, vmax);
            }
            Self::TriMesh(mesh) => {
                mesh.set_clim(vmin, vmax);
            }
            Self::Contour(contours) => {
                contours.set_clim(vmin, vmax);
            }
        }
        self
    }

    /// Whether the artist is drawn in `axes`.
    fn is_in(&self, axes: &Axes) -> bool {
        let child_of = |raw: &RawHandle| {
            Rc::ptr_eq(raw.figure_state(), axes.raw().figure_state())
                && raw.is_attached()
                && raw.with_base(|base| base.parent) == Some(axes.id())
        };
        match self {
            Self::Image(image) => child_of(image.raw()),
            Self::Collection(collection) => child_of(collection.raw()),
            Self::TriMesh(mesh) => child_of(mesh.raw()),
            Self::Contour(contours) => contours.axes() == axes && contours.is_attached(),
        }
    }

    /// The record an axes keeps of this artist.
    fn record(&self) -> CurrentImage {
        match self {
            Self::Image(image) => CurrentImage::Image(image.id()),
            Self::Collection(collection) => CurrentImage::Collection(collection.id()),
            Self::TriMesh(mesh) => CurrentImage::TriMesh(mesh.id()),
            Self::Contour(contours) => CurrentImage::Contour(contours.id()),
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
        let record = self.raw().with::<AxesState, _>(|axes| axes.current_image)?;
        let figure = self.raw().figure_state();
        let handle = |id: ArtistId| {
            let raw = RawHandle::new(Rc::clone(figure), id);
            raw.is_attached().then_some(raw)
        };
        Some(match record {
            CurrentImage::Image(id) => ColorizingArtist::Image(AxesImage::from_raw(handle(id)?)),
            CurrentImage::Collection(id) => {
                ColorizingArtist::Collection(Collection::from_raw(handle(id)?))
            }
            CurrentImage::TriMesh(id) => ColorizingArtist::TriMesh(TriMesh::from_raw(handle(id)?)),
            CurrentImage::Contour(id) => {
                ColorizingArtist::Contour(ContourSet::from_raw(self.clone(), handle(id)?))
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
    fn contour_sets_are_remembered_with_their_state() {
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
