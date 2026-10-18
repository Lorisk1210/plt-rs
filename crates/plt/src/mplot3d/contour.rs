//! Contours in 3-D: lines at their level, fills at their band's middle,
//! or both projected onto a plane (matplotlib's `Axes3D.contour` and
//! `contourf`).

use super::art3d::{Art3D, Path3D};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::surface::SurfaceGrid;
use crate::artist::ArtistHandle;
use crate::contour::{ContourOptions, ContourSet, QuadCoords, QuadField};
use crate::numeric::{Grid, nanmax, nanmin};

/// Options of [`Axes3D::contour`] and [`Axes3D::contourf`].
#[derive(Debug, Default)]
pub struct Contour3DOptions {
    /// The axis whose values are contoured; the contours lie in planes
    /// normal to it.
    pub zdir: Axis3,
    /// Places every contour in the plane at this position along `zdir`
    /// instead of at its level (a projection of the contours).
    pub offset: Option<f64>,
    /// The 2-D contour options: levels, colours, widths, ...
    pub contour: ContourOptions,
}

impl Axes3D {
    /// Contour lines of the surface `(x, y, z)`, each at its level along
    /// `zdir` (matplotlib's `ax.contour(X, Y, Z, zdir=..., offset=...)`).
    ///
    /// # Errors
    ///
    /// When the grids differ in shape, or as
    /// [`crate::axes::Axes::contour`].
    pub fn contour(
        &self,
        x: &Grid<f64>,
        y: &Grid<f64>,
        z: &Grid<f64>,
        options: Contour3DOptions,
    ) -> Result<ContourSet, Axes3DError> {
        let grid = SurfaceGrid::new(x, y, z)?;
        let had_data = self.has_data();
        let field = rotated_field(&grid, options.zdir);
        let set = self.plane().contour(&field, options.contour)?;
        self.lift(&set, options.zdir, options.offset, &set.levels());
        self.auto_scale_xyz(x.as_slice(), y.as_slice(), Some(z.as_slice()), had_data)?;
        Ok(set)
    }

    /// Filled contours of the surface `(x, y, z)`, each band at its middle
    /// along `zdir` (matplotlib's `ax.contourf(X, Y, Z, zdir=...,
    /// offset=...)`).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::contour`] and [`crate::axes::Axes::contourf`].
    pub fn contourf(
        &self,
        x: &Grid<f64>,
        y: &Grid<f64>,
        z: &Grid<f64>,
        options: Contour3DOptions,
    ) -> Result<ContourSet, Axes3DError> {
        let grid = SurfaceGrid::new(x, y, z)?;
        let had_data = self.has_data();
        let extend = options.contour.extend;
        let field = rotated_field(&grid, options.zdir);
        let set = self.plane().contourf(&field, options.contour)?;
        let levels = set.levels();
        let mut middles: Vec<f64> = levels
            .windows(2)
            .map(|w| w[0] + (w[1] - w[0]) / 2.0)
            .collect();
        if let ([first, second, ..], true) = (levels.as_slice(), extend.extends_min()) {
            middles.insert(0, first - (second - first) / 2.0);
        }
        if let ([.., before, last], true) = (levels.as_slice(), extend.extends_max()) {
            middles.push(last + (last - before) / 2.0);
        }
        self.lift(&set, options.zdir, options.offset, &middles);
        // The limits along `zdir` follow the band middles.
        let span = |axis: Axis3, values: &[f64]| -> Vec<f64> {
            let values = if axis == options.zdir {
                &middles[..]
            } else {
                values
            };
            let (low, high) = (
                nanmin(values.iter().copied()),
                nanmax(values.iter().copied()),
            );
            low.into_iter().chain(high).collect()
        };
        let xs = span(Axis3::X, x.as_slice());
        let ys = span(Axis3::Y, y.as_slice());
        let zs = span(Axis3::Z, z.as_slice());
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        Ok(set)
    }

    /// Places the contour paths of `set` in 3-D: path `i` in the plane at
    /// `offset`, or at `positions[i]`, along `zdir` (matplotlib's
    /// `collection_2d_to_3d`).
    fn lift(&self, set: &ContourSet, zdir: Axis3, offset: Option<f64>, positions: &[f64]) {
        let mut index = 0;
        for collection in set.collections() {
            let paths = collection
                .paths()
                .into_iter()
                .map(|path| {
                    let along = offset
                        .or_else(|| positions.get(index).copied())
                        .unwrap_or(f64::NAN);
                    index += 1;
                    let vertices = path
                        .vertices()
                        .iter()
                        .map(|v| zdir.unrotate([v.x, v.y, along]))
                        .collect();
                    Path3D::new(path, vertices)
                })
                .collect();
            self.register(collection.id(), Art3D::Paths(paths));
        }
    }
}

/// The field contoured for `zdir`: the grid rotated so that `zdir`
/// becomes z.
fn rotated_field(grid: &SurfaceGrid<'_>, zdir: Axis3) -> QuadField {
    let [x, y, z] = grid.rotated(zdir);
    QuadField::new(z).with_coords(QuadCoords::Mesh { x, y })
}
