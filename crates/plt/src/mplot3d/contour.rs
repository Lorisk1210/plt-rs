//! Contours in 3-D: lines at their level (or extended into ribbons),
//! fills at their band's middle, or both projected onto a plane
//! (matplotlib's `Axes3D.contour`, `contourf`, `tricontour`,
//! `tricontourf` and `_3d_extend_contour`).

use std::num::NonZeroUsize;

use plt_core::path::PathCode;
use plt_core::{Path, Point};

use super::art3d::{Art3D, Path3D, ZSort};
use super::collections3d::{NewPolys, PolyFaces};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::columns;
use super::shade::LightSource;
use super::surface::SurfaceGrid;
use crate::artist::ArtistHandle;
use crate::collections::CollectionProps;
use crate::colors::Extend;
use crate::contour::{ContourOptions, ContourSet, QuadCoords, QuadField};
use crate::numeric::{Grid, nanmax, nanmin};
use crate::tri::Triangulation;

/// Options of [`Axes3D::contour`], [`Axes3D::contourf`],
/// [`Axes3D::tricontour`] and [`Axes3D::tricontourf`].
#[derive(Debug, Default)]
pub struct Contour3DOptions {
    /// The axis whose values are contoured; the contours lie in planes
    /// normal to it.
    pub zdir: Axis3,
    /// Places every contour in the plane at this position along `zdir`
    /// instead of at its level (a projection of the contours).
    pub offset: Option<f64>,
    /// Line contours only: replaces each level's first contour line by a
    /// ribbon of shaded quadrilaterals reaching half-way to the next
    /// level, one per this many points (matplotlib's `extend3d=True` with
    /// `stride`, by default 5). The ribbons stand along z whatever
    /// `zdir`, as in matplotlib; filled contours have no such option.
    pub extend3d: Option<NonZeroUsize>,
    /// Whether the contour points outside the view limits are hidden,
    /// breaking the lines and fills there (matplotlib's `axlim_clip`,
    /// which matplotlib 3.11 fails to draw once a point is hidden); the
    /// ribbons of `extend3d` are never hidden, as in matplotlib.
    pub axlim_clip: bool,
    /// The 2-D contour options: levels, colours, widths, ...
    pub contour: ContourOptions,
}

/// Where the contour paths of a set go in 3-D, and whether they are
/// clipped to the view limits.
#[derive(Clone, Copy, Debug)]
struct Placement {
    zdir: Axis3,
    offset: Option<f64>,
    axlim_clip: bool,
}

impl Contour3DOptions {
    /// The placement these options ask for.
    fn placement(&self) -> Placement {
        Placement {
            zdir: self.zdir,
            offset: self.offset,
            axlim_clip: self.axlim_clip,
        }
    }
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
        let placement = options.placement();
        let field = rotated_field(&grid, options.zdir);
        let set = self.plane().contour(&field, options.contour)?;
        self.add_contour_set(&set, placement, options.extend3d)?;
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
        let placement = options.placement();
        let field = rotated_field(&grid, options.zdir);
        let set = self.plane().contourf(&field, options.contour)?;
        let middles = self.add_contourf_set(&set, extend, placement);
        let data = [x.as_slice(), y.as_slice(), z.as_slice()];
        self.auto_scale_contourf(data, options.zdir, &middles, had_data)?;
        Ok(set)
    }

    /// Contour lines of one `z` per point of a triangulation, each at its
    /// level along `zdir` (matplotlib's `ax.tricontour(tri, Z,
    /// zdir=..., offset=...)`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::mplot3d::Contour3DOptions;
    /// use plt::tri::Triangulation;
    /// let (x, y) = ([0.0, 1.0, 0.0, 1.0, 0.5], [0.0, 0.0, 1.0, 1.0, 0.5]);
    /// let tri = Triangulation::delaunay(&x, &y)?;
    /// let ax = Figure::new().subplot_3d();
    /// let set = ax.tricontour(&tri, &[0.0, 1.0, 1.0, 2.0, 3.0], Contour3DOptions::default())?;
    /// assert!(!set.levels().is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `z` does not have one value per point, or as
    /// [`crate::axes::Axes::tricontour`].
    pub fn tricontour(
        &self,
        tri: &Triangulation,
        z: &[f64],
        options: Contour3DOptions,
    ) -> Result<ContourSet, Axes3DError> {
        check_len("z", tri.npoints(), z.len())?;
        let had_data = self.has_data();
        let placement = options.placement();
        let (rotated, jz) = rotated_triangulation(tri, z, options.zdir)?;
        let set = self.plane().tricontour(&rotated, &jz, options.contour)?;
        self.add_contour_set(&set, placement, options.extend3d)?;
        self.auto_scale_xyz(tri.x(), tri.y(), Some(z), had_data)?;
        Ok(set)
    }

    /// Filled contours on a triangulation, each band at its middle along
    /// `zdir` (matplotlib's `ax.tricontourf(tri, Z, zdir=...,
    /// offset=...)`).
    ///
    /// # Errors
    ///
    /// As [`Axes3D::tricontour`] and [`crate::axes::Axes::tricontourf`].
    pub fn tricontourf(
        &self,
        tri: &Triangulation,
        z: &[f64],
        options: Contour3DOptions,
    ) -> Result<ContourSet, Axes3DError> {
        check_len("z", tri.npoints(), z.len())?;
        let had_data = self.has_data();
        let extend = options.contour.extend;
        let placement = options.placement();
        let (rotated, jz) = rotated_triangulation(tri, z, options.zdir)?;
        let set = self.plane().tricontourf(&rotated, &jz, options.contour)?;
        let middles = self.add_contourf_set(&set, extend, placement);
        self.auto_scale_contourf([tri.x(), tri.y(), z], options.zdir, &middles, had_data)?;
        Ok(set)
    }

    /// Places line contours in 3-D, or extends them into ribbons
    /// (matplotlib's `add_contour_set`).
    fn add_contour_set(
        &self,
        set: &ContourSet,
        placement: Placement,
        extend3d: Option<NonZeroUsize>,
    ) -> Result<(), Axes3DError> {
        if let Some(stride) = extend3d {
            return self.extend_contour(set, stride);
        }
        self.lift(set, placement, &set.levels());
        Ok(())
    }

    /// Places filled contours in 3-D at the middles of their bands, the
    /// extended ends half a band beyond the outer levels, and returns
    /// those positions (matplotlib's `_add_contourf_set`).
    fn add_contourf_set(&self, set: &ContourSet, extend: Extend, placement: Placement) -> Vec<f64> {
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
        self.lift(set, placement, &middles);
        middles
    }

    /// Autoscales to the data, with the band middles along `zdir`
    /// (matplotlib's `_auto_scale_contourf`).
    fn auto_scale_contourf(
        &self,
        data: [&[f64]; 3],
        zdir: Axis3,
        middles: &[f64],
        had_data: bool,
    ) -> Result<(), Axes3DError> {
        let [xs, ys, zs] = Axis3::ALL.map(|axis| {
            let values = if axis == zdir {
                middles
            } else {
                data[axis.index()]
            };
            let (low, high) = (
                nanmin(values.iter().copied()),
                nanmax(values.iter().copied()),
            );
            low.into_iter().chain(high).collect::<Vec<f64>>()
        });
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)
    }

    /// Places the contour paths of `set` in 3-D: path `i` in the plane at
    /// the placement's offset, or at `positions[i]`, along its `zdir`
    /// (matplotlib's `collection_2d_to_3d`).
    fn lift(&self, set: &ContourSet, placement: Placement, positions: &[f64]) {
        let mut index = 0;
        for collection in set.collections() {
            let paths = collection
                .paths()
                .into_iter()
                .map(|path| {
                    let along = placement
                        .offset
                        .or_else(|| positions.get(index).copied())
                        .unwrap_or(f64::NAN);
                    index += 1;
                    let vertices = path
                        .vertices()
                        .iter()
                        .map(|v| placement.zdir.unrotate([v.x, v.y, along]))
                        .collect();
                    Path3D::new(path, vertices)
                })
                .collect();
            self.register(collection.id(), Art3D::Paths(paths), placement.axlim_clip);
        }
    }

    /// Replaces the line contours of `set` by shaded ribbons, one
    /// quadrilateral per `stride` points of each level's first line,
    /// reaching half the level step above and below it (matplotlib's
    /// `_3d_extend_contour`).
    fn extend_contour(&self, set: &ContourSet, stride: NonZeroUsize) -> Result<(), Axes3DError> {
        let levels = set.levels();
        let &[first, second, ..] = levels.as_slice() else {
            return Err(Axes3DError::Shape {
                argument: "contour levels",
                expected: 2,
                actual: levels.len(),
            });
        };
        let dz = (second - first) / 2.0;
        let colors = set.colors();
        let mut faces: Vec<Vec<Vec3>> = Vec::new();
        let mut face_colors = Vec::new();
        for ((&level, path), &color) in levels.iter().zip(set.paths()).zip(&colors) {
            let line = first_component(&path);
            let ribbon = ribbon(&line, level - dz, level + dz, stride);
            face_colors.extend(std::iter::repeat_n(color, ribbon.len()));
            faces.extend(ribbon);
        }
        let points: Vec<Vec3> = faces.iter().flatten().copied().collect();
        self.add_polys(NewPolys {
            faces,
            colors: PolyFaces::Colors(face_colors.clone()),
            shaded_edges: Some(face_colors),
            light: Some(LightSource::mplot3d()),
            zsort: ZSort::Average,
            props: CollectionProps::default(),
            axlim_clip: false,
        })?;
        if !points.is_empty() {
            let [xs, ys, zs] = columns(&points);
            // matplotlib's `add_collection3d` sees the 2-D contours as data.
            self.auto_scale_xyz(&xs, &ys, Some(&zs), true)?;
        }
        let figure = self.raw().figure_state();
        for collection in set.collections() {
            collection.remove();
        }
        figure.borrow_mut().remove_artist(set.id());
        Ok(())
    }
}

/// The vertices of the first connected part of `path`, up to its second
/// move (matplotlib's first `_iter_connected_components` subpath through
/// `iter_segments`).
fn first_component(path: &Path) -> Vec<Point> {
    let vertices = path.vertices();
    let Some(codes) = path.codes() else {
        return vertices.to_vec();
    };
    let is_move = |c: &PathCode| *c == PathCode::MoveTo;
    let Some(start) = codes.iter().position(is_move) else {
        return Vec::new();
    };
    let end = codes[start + 1..]
        .iter()
        .position(is_move)
        .map_or(codes.len(), |i| start + 1 + i);
    vertices[start..end].to_vec()
}

/// The quadrilaterals between the line lifted to `low` and to `high`,
/// joining every `stride`-th point (rounded like Python).
fn ribbon(line: &[Point], low: f64, high: f64, stride: NonZeroUsize) -> Vec<Vec<Vec3>> {
    if line.is_empty() {
        return Vec::new();
    }
    let n = line.len();
    let steps = ((n as f64 / stride.get() as f64).round_ties_even() as usize).max(2);
    let step = (n - 1) as f64 / (steps - 1) as f64;
    let at = |k: usize, z: f64| {
        let p = line[(k as f64 * step).round_ties_even() as usize];
        [p.x, p.y, z]
    };
    (0..steps - 1)
        .map(|k| vec![at(k, low), at(k + 1, low), at(k + 1, high), at(k, high)])
        .collect()
}

/// The field contoured for `zdir`: the grid rotated so that `zdir`
/// becomes z.
fn rotated_field(grid: &SurfaceGrid<'_>, zdir: Axis3) -> QuadField {
    let [x, y, z] = grid.rotated(zdir);
    QuadField::new(z).with_coords(QuadCoords::Mesh { x, y })
}

/// The triangulation and values contoured for `zdir`: the points rotated
/// so that `zdir` becomes z, keeping the triangles and the mask
/// (matplotlib's `Triangulation(jX, jY, tri.triangles, tri.mask)`).
fn rotated_triangulation(
    tri: &Triangulation,
    z: &[f64],
    zdir: Axis3,
) -> Result<(Triangulation, Vec<f64>), Axes3DError> {
    let points: Vec<Vec3> = (0..tri.npoints())
        .map(|i| zdir.rotate([tri.x()[i], tri.y()[i], z[i]]))
        .collect();
    let [jx, jy, jz] = columns(&points);
    let rotated = Triangulation::new(&jx, &jy, tri.triangles().to_vec())?
        .with_mask(tri.mask().map(<[bool]>::to_vec))?;
    Ok((rotated, jz))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::numeric::{linspace, meshgrid};

    fn stride(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).expect("a positive stride")
    }

    #[test]
    fn ribbons_follow_pythons_rounding() {
        let line: Vec<Point> = (0..7).map(|i| Point::new(f64::from(i), 0.0)).collect();
        let quads = ribbon(&line, -1.0, 1.0, stride(5));
        // round(7 / 5) = 1 → 2 steps of 6: one quad from point 0 to 6.
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0][1], [6.0, 0.0, -1.0]);
        let quads = ribbon(&line, -1.0, 1.0, stride(2));
        // round(3.5) = 4 steps → 3 quads at 0, 2, 4, 6.
        assert_eq!(quads.len(), 3);
        assert_eq!(quads[1][0], [2.0, 0.0, -1.0]);
        assert_eq!(ribbon(&[], 0.0, 1.0, stride(5)).len(), 0);
    }

    #[test]
    fn the_first_component_stops_at_the_next_move() {
        let path = Path::builder()
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(1.0, 0.0))
            .move_to(Point::new(5.0, 5.0))
            .line_to(Point::new(6.0, 5.0))
            .build();
        assert_eq!(first_component(&path).len(), 2);
        assert_eq!(first_component(&Path::polyline(Vec::new())).len(), 0);
    }

    #[test]
    fn extended_contours_replace_the_lines() {
        let x = linspace(-2.0, 2.0, 12);
        let (xx, yy) = meshgrid(&x, &x);
        let zz = Grid::from_fn(12, 12, |i, j| xx[(i, j)] * xx[(i, j)] + yy[(i, j)]);
        let ax = Figure::new().subplot_3d();
        let options = Contour3DOptions {
            extend3d: NonZeroUsize::new(5),
            ..Contour3DOptions::default()
        };
        let set = ax.contour(&xx, &yy, &zz, options).unwrap();
        assert!(!set.is_alive(), "the 2-D contours are removed");
        assert_eq!(
            ax.with(|axes| axes.artists.len()),
            1,
            "one ribbon collection"
        );
    }

    #[test]
    fn tricontours_rotate_the_triangulation() {
        let (x, y) = ([0.0, 1.0, 0.0, 1.0, 0.5], [0.0, 0.0, 1.0, 1.0, 0.5]);
        let tri = Triangulation::delaunay(&x, &y).unwrap();
        let z = [0.0, 1.0, 1.0, 2.0, 3.0];
        let ax = Figure::new().subplot_3d();
        let options = Contour3DOptions {
            zdir: Axis3::Y,
            offset: Some(2.0),
            ..Contour3DOptions::default()
        };
        ax.tricontourf(&tri, &z, options).unwrap();
        assert!(ax.ylim().1 > 0.5);
        assert!(
            ax.tricontour(&tri, &z[..2], Contour3DOptions::default())
                .is_err()
        );
    }
}
