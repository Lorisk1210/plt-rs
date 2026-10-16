//! Contour plots (matplotlib's `contour`, `contourf`, `tricontour` and
//! `tricontourf`).

use super::children::DataUpdate;
use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::Collection;
use crate::contour::{
    ContourError, ContourOptions, ContourPlan, ContourSet, ContourSource, Generator, QuadField,
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
        let source = field.source(options.logscale(), &crate::rc::params())?;
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
        let source = field.source(options.logscale(), &crate::rc::params())?;
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

    /// Computes the contour set and adds its collection, extending the
    /// data limits to the grid and autoscaling tightly onto it.
    fn add_contour_set<G: Generator>(
        &self,
        filled: bool,
        options: ContourOptions,
        source: ContourSource<G>,
    ) -> Result<ContourSet, ContourError> {
        let plan = ContourPlan::new(filled, options, source)?;
        let (mins, maxs) = plan.data_lim;
        let mut update = DataUpdate::data(vec![mins, maxs]);
        let mut collections = Vec::with_capacity(plan.collections.len());
        for mut collection in plan.collections {
            collection.base_mut().transform = Some(self.trans_data());
            collections.push(Collection::from_raw(self.adopt(collection, &update)));
            update = DataUpdate::default();
        }
        self.raw().with_mut::<AxesState, _>(|a| a.tight = true);
        Ok(ContourSet::new(self.clone(), collections, plan.state))
    }
}
