//! Contour plots (matplotlib's `contour`, `contourf`, `tricontour` and
//! `tricontourf`).

use plt_core::{Affine2D, Path};

use super::children::DataUpdate;
use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle};
use crate::contour::{
    ContourError, ContourOptions, ContourPlan, ContourSet, ContourSetArtist, ContourSource,
    Generator, Levels, QuadField, given_source,
};
use crate::tri::{Triangulation, contour_source};

impl Axes {
    /// Contour lines of a field on a quadrilateral grid (matplotlib's
    /// `contour`).
    ///
    /// ```
    /// use plt::contour::{ContourOptions, QuadField};
    /// use plt::numeric::Grid;
    /// use plt::prelude::*;
    ///
    /// let z = Grid::from_fn(4, 5, |i, j| (i + j) as f64);
    /// let cs = Figure::new().subplot().contour(&QuadField::new(z), ContourOptions::default())?;
    /// assert_eq!(cs.levels(), [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
    /// # Ok::<(), plt::contour::ContourError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the shapes are inconsistent, the levels are invalid, or the
    /// colours cannot be mapped.
    pub fn contour(
        &self,
        field: &QuadField,
        options: ContourOptions,
    ) -> Result<ContourSet, ContourError> {
        let source = field.source(
            options.logscale(),
            &crate::rc::params(),
            self.grid_to_data(&options).as_ref(),
        )?;
        self.add_contour_set(false, options, source)
    }

    /// Filled contours of a field on a quadrilateral grid (matplotlib's
    /// `contourf`).
    ///
    /// # Errors
    ///
    /// As [`Axes::contour`], and when fewer than two levels are given.
    pub fn contourf(
        &self,
        field: &QuadField,
        options: ContourOptions,
    ) -> Result<ContourSet, ContourError> {
        let source = field.source(
            options.logscale(),
            &crate::rc::params(),
            self.grid_to_data(&options).as_ref(),
        )?;
        self.add_contour_set(true, options, source)
    }

    /// Contour lines of one z value per point of a triangulation
    /// (matplotlib's `tricontour`).
    ///
    /// # Errors
    ///
    /// When `z` does not match the points or is not finite on the
    /// triangulation, when log-scaled z is not positive, or as
    /// [`Axes::contour`].
    pub fn tricontour(
        &self,
        tri: &Triangulation,
        z: &[f64],
        options: ContourOptions,
    ) -> Result<ContourSet, ContourError> {
        let source = contour_source(tri, z, options.logscale())?;
        self.add_contour_set(false, options, source)
    }

    /// Filled contours on a triangulation (matplotlib's `tricontourf`).
    ///
    /// # Errors
    ///
    /// As [`Axes::tricontour`] and [`Axes::contourf`].
    pub fn tricontourf(
        &self,
        tri: &Triangulation,
        z: &[f64],
        options: ContourOptions,
    ) -> Result<ContourSet, ContourError> {
        let source = contour_source(tri, z, options.logscale())?;
        self.add_contour_set(true, options, source)
    }

    /// Contours given as segments instead of traced from a field
    /// (matplotlib's `ContourSet(ax, levels, allsegs, allkinds)`): for
    /// every level (lines) or band between consecutive levels (`filled`),
    /// the paths drawn there, joined into one compound path. They are
    /// coloured and styled like traced contours at `levels`, which replace
    /// `options.levels`.
    ///
    /// ```
    /// use plt::contour::ContourOptions;
    /// use plt::prelude::*;
    /// use plt::path::Path;
    /// use plt::transforms::Point;
    ///
    /// let ax = Figure::new().subplot();
    /// let line = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(3.0, 4.0)]);
    /// let cs = ax.contour_segments(&[1.0], &[vec![line]], false, ContourOptions::default())?;
    /// assert_eq!(cs.levels(), [1.0]);
    /// assert_eq!((ax.xlim(), ax.ylim()), ((0.0, 3.0), (0.0, 4.0)));
    /// # Ok::<(), plt::contour::ContourError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When there is not one list of segments per level or band, the
    /// segments have no points, or as [`Axes::contour`] and
    /// [`Axes::contourf`].
    pub fn contour_segments(
        &self,
        levels: &[f64],
        segments: &[Vec<Path>],
        filled: bool,
        options: ContourOptions,
    ) -> Result<ContourSet, ContourError> {
        let source = given_source(levels, segments, filled)?;
        let options = ContourOptions {
            levels: Levels::Values(levels.to_vec()),
            ..options
        };
        self.add_contour_set(filled, options, source)
    }

    /// The map from the coordinates of a quadrilateral grid to the data
    /// coordinates, when they differ (matplotlib's `trans_to_data` in
    /// `QuadContourSet._process_args`; triangulations are never mapped).
    fn grid_to_data(&self, options: &ContourOptions) -> Option<Affine2D> {
        let transform = options.transform.as_ref()?;
        transform
            .affine_before_data(self.id())
            .filter(|map| *map != Affine2D::IDENTITY)
    }

    /// Computes the contour set and adds its collection, extending the
    /// data limits to the grid and autoscaling tightly onto it.
    fn add_contour_set<G: Generator>(
        &self,
        filled: bool,
        options: ContourOptions,
        source: ContourSource<G>,
    ) -> Result<ContourSet, ContourError> {
        let transform = options
            .transform
            .clone()
            .unwrap_or_else(|| self.trans_data());
        let plan = ContourPlan::new(filled, options, source)?;
        let (mins, maxs) = plan.data_lim;
        let mut update = DataUpdate::data(vec![mins, maxs]);
        let mut collections = Vec::with_capacity(plan.collections.len());
        for mut collection in plan.collections {
            collection.base_mut().transform = Some(transform.clone());
            collections.push(self.adopt(collection, &update).id());
            update = DataUpdate::default();
        }
        let set = self.adopt(
            ContourSetArtist::new(plan.state, collections),
            &DataUpdate::default(),
        );
        self.raw().with_mut::<AxesState, _>(|a| a.tight = true);
        Ok(ContourSet::from_raw(self.clone(), set))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::numeric::Grid;
    use crate::transforms::Transform;

    #[test]
    fn transformed_grids_bound_the_data_where_they_land() {
        let ax = Figure::new().subplot();
        let times10 =
            Transform::Affine(Affine2D::identity().scale(10.0, 10.0)).then(ax.trans_data());
        let options = ContourOptions {
            transform: Some(times10),
            ..ContourOptions::default()
        };
        let z = Grid::from_fn(3, 4, |r, c| (r + c) as f64);
        ax.contourf(&QuadField::new(z), options).unwrap();
        assert_eq!((ax.xlim(), ax.ylim()), ((0.0, 30.0), (0.0, 20.0)));
    }
}
