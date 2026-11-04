//! Bars in 3-D: boxes (matplotlib's `bar3d`) and flat 2-D bars placed in
//! a plane (matplotlib's `Axes3D.bar`).

use std::cmp::Ordering;

use pltrs_core::{PathCode, Point};

use super::art3d::{ProjectedOutline, ZSort};
use super::collections3d::{NewPolys, PolyFaces, resolve_all};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::{columns, spread};
use super::shade::LightSource;
use crate::artist::ArtistHandle;
use crate::axes::BarOptions;
use crate::collections::{Collection, CollectionProps};
use crate::colors::Color;
use crate::patches::{Patch, PatchShape, Rectangle};
use crate::units::{AxisDeltas, AxisValues, UnitHandle};

/// The faces of the unit cube, each counter-clockwise seen from outside:
/// -z, +z, -y, +y, -x, +x.
const CUBOID: [[[f64; 3]; 4]; 6] = [
    [
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
    ],
    [
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    ],
    [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ],
    [
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
    ],
    [
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
    ],
    [
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
    ],
];

/// Options of [`Axes3D::bar3d`].
#[derive(Clone, Debug)]
pub struct Bar3DOptions {
    /// One colour for all bars, one per bar, one per face kind (6, in
    /// matplotlib's cuboid face order: bottom, top, front, back, left,
    /// right) or one per face; empty takes the next colour of
    /// the patch property cycle.
    pub color: Vec<Color>,
    /// How faces are ordered by depth.
    pub zsort: ZSort,
    /// Whether faces turned away from the light are darkened.
    pub shade: bool,
    /// The light shading the faces.
    pub lightsource: Option<LightSource>,
    /// Further collection properties (alpha, edge colour, width, ...).
    pub props: CollectionProps,
    /// Whether the points outside the view limits are hidden, breaking the
    /// faces reaching them (matplotlib's `axlim_clip`).
    pub axlim_clip: bool,
}

impl Default for Bar3DOptions {
    fn default() -> Self {
        Self {
            color: Vec::new(),
            zsort: ZSort::Average,
            shade: true,
            lightsource: None,
            props: CollectionProps::default(),
            axlim_clip: false,
        }
    }
}

/// The geometry of the boxes of [`Axes3D::bar3d`]: anchor corners as
/// plotting values (numbers, dates, category names or user values) and
/// sizes as extents from them (numbers, durations or user values), each
/// holding one value per bar or one for all.
///
/// ```
/// use pltrs::mplot3d::Boxes;
/// let boxes = Boxes::new(["a", "b"], [0.0, 1.0], [0.0], [0.5], [0.5], [2.0, 3.0]);
/// assert_eq!(boxes.dz.len(), 2);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Boxes {
    /// x of the anchor corners.
    pub x: AxisValues,
    /// y of the anchor corners.
    pub y: AxisValues,
    /// z of the anchor corners.
    pub z: AxisValues,
    /// Widths along x.
    pub dx: AxisDeltas,
    /// Depths along y.
    pub dy: AxisDeltas,
    /// Heights along z.
    pub dz: AxisDeltas,
}

impl Boxes {
    /// Boxes with corners at `(x, y, z)` and sizes `(dx, dy, dz)`.
    pub fn new(
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        z: impl Into<AxisValues>,
        dx: impl Into<AxisDeltas>,
        dy: impl Into<AxisDeltas>,
        dz: impl Into<AxisDeltas>,
    ) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            dx: dx.into(),
            dy: dy.into(),
            dz: dz.into(),
        }
    }

    /// The number of boxes: the length of the longest list.
    fn len(&self) -> usize {
        [self.x.len(), self.y.len(), self.z.len()]
            .into_iter()
            .chain([self.dx.len(), self.dy.len(), self.dz.len()])
            .max()
            .unwrap_or(0)
    }
}

impl Axes3D {
    /// Boxes with corners at `(x, y, z)` and sizes `(dx, dy, dz)`
    /// (matplotlib's `bar3d`). The corners convert on their axes as
    /// [`Axes3D::plot`] converts its points, the sizes through the
    /// corners they extend from (matplotlib's `_convert_dx`).
    ///
    /// # Errors
    ///
    /// When the arrays do not broadcast or a colour is invalid.
    pub fn bar3d(&self, boxes: Boxes, options: Bar3DOptions) -> Result<Collection, Axes3DError> {
        let had_data = self.has_data();
        let n = boxes.len();
        let corners = [&boxes.x, &boxes.y, &boxes.z].map(AxisValues::len);
        let sizes = [&boxes.dx, &boxes.dy, &boxes.dz].map(AxisDeltas::len);
        let names = ["x", "y", "z", "dx", "dy", "dz"];
        for (name, len) in names.into_iter().zip(corners.into_iter().chain(sizes)) {
            if len != 1 {
                check_len(name, n, len)?;
            }
        }
        let Boxes {
            x,
            y,
            z,
            dx,
            dy,
            dz,
        } = boxes;
        let starts = self.convert_xyz([x.clone(), y.clone(), z.clone()]);
        let extents = [(dx, &x), (dy, &y), (dz, &z)];
        let [dx, dy, dz] = std::array::from_fn(|i| {
            let (deltas, origin) = &extents[i];
            self.axis(Axis3::ALL[i])
                .convert_extents(deltas, origin, &starts[i])
        });
        let [x, y, z] = starts;
        let [x, y, z, dx, dy, dz] = [x, y, z, dx, dy, dz].map(|values| spread(values, n));
        let span = |p: &[f64], dp: &[f64]| {
            let low = p.iter().copied().fold(f64::INFINITY, f64::min);
            let high = p
                .iter()
                .zip(dp)
                .map(|(p, d)| p + d)
                .fold(f64::NEG_INFINITY, f64::max);
            vec![low, high]
        };
        let limits = [span(&x, &dx), span(&y, &dy), span(&z, &dz)];
        let faces: Vec<Vec<Vec3>> = (0..n)
            .flat_map(|i| {
                let (corner, size) = ([x[i], y[i], z[i]], [dx[i], dy[i], dz[i]]);
                CUBOID.iter().map(move |face| {
                    face.iter()
                        .map(|unit| [0, 1, 2].map(|k| corner[k] + size[k] * unit[k]))
                        .collect()
                })
            })
            .collect();
        let colors = if options.color.is_empty() {
            vec![self.plane().next_patch_color()]
        } else {
            options.color
        };
        let colors = resolve_all(&colors)?;
        let facecolors = match colors.len().cmp(&n) {
            Ordering::Equal => colors
                .iter()
                .flat_map(|&c| std::iter::repeat_n(c, 6))
                .collect(),
            Ordering::Less => colors.repeat(6 * n),
            Ordering::Greater => colors,
        };
        let collection = self.add_polys(NewPolys {
            faces,
            colors: PolyFaces::Colors(facecolors),
            shaded_edges: None,
            light: options
                .shade
                .then(|| options.lightsource.unwrap_or_else(LightSource::mplot3d)),
            zsort: options.zsort,
            props: options.props,
            axlim_clip: options.axlim_clip,
        })?;
        let [xs, ys, zs] = limits;
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        Ok(collection)
    }

    /// Flat vertical bars at `left` with heights `height`, placed in the
    /// plane normal to `zdir` at `zs` (one per bar or one for all;
    /// matplotlib's `ax.bar(left, height, zs=..., zdir=...)`).
    /// [`Axes3D::set_axlim_clip`] on a bar hides it where it reaches
    /// beyond the view limits (matplotlib's `axlim_clip`, which matplotlib
    /// 3.11 fails to draw once a corner is hidden). The positions, the
    /// bases in `options` and `zs` convert on the axes they lie along
    /// once placed, as [`Axes3D::plot`] converts its points, and the
    /// heights and thicknesses that are not numbers through them
    /// (matplotlib's `_convert_dx`).
    ///
    /// # Errors
    ///
    /// When `zs` does not broadcast, or as [`crate::axes::Axes::bar`].
    pub fn bar(
        &self,
        left: impl Into<AxisValues>,
        height: impl Into<AxisDeltas>,
        zs: impl Into<AxisValues>,
        zdir: Axis3,
        mut options: BarOptions,
    ) -> Result<Vec<Patch<ProjectedOutline>>, Axes3DError> {
        let had_data = self.has_data();
        let (left, height, zs) = (left.into(), height.into(), zs.into());
        check_len("height", left.len(), height.len())?;
        if zs.len() != 1 {
            check_len("zs", left.len(), zs.len())?;
        }
        // The axes the 2-D positions, the 2-D values and `zs` lie along.
        let [along_left, along_height, along_zs] =
            zdir.unjuggle(Axis3::ALL).map(|dir| self.axis(dir));
        let positions = along_left.convert_values(left.clone());
        options.thickness = along_left
            .convert_extents(&options.thickness, &left, &positions)
            .into();
        let bases = along_height.convert_values(options.base.clone());
        let height = along_height.convert_extents(&height, &options.base, &bases);
        options.base = bases.into();
        let zs = spread(along_zs.convert_values(zs), positions.len());
        let container =
            self.on_plane_keeping_limits(|plane| plane.bar(positions, height, options))?;
        let mut all_points = Vec::new();
        let mut patches = Vec::with_capacity(container.patches.len());
        for (rectangle, &z) in container.patches.iter().zip(&zs) {
            let outline: Vec<Vec3> = polygon(&rectangle.shape())
                .into_iter()
                .map(|p| zdir.juggle([p.x, p.y, z]))
                .collect();
            all_points.extend_from_slice(&outline);
            patches.push(self.patch_2d_to_3d(rectangle.raw(), outline, false)?);
        }
        let [xs, ys, zs] = columns(&all_points);
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        Ok(patches)
    }
}

/// The outline of a rectangle as a closed polygon, whose first vertex is
/// repeated at the end unless the last one already is the first, as for
/// a rectangle of no height (matplotlib's `_get_patch_verts`, through
/// `Path.to_polygons`).
fn polygon(rectangle: &Rectangle) -> Vec<Point> {
    let path = rectangle.path();
    let placement = rectangle.patch_transform();
    let mut points: Vec<Point> = path
        .vertices()
        .iter()
        .zip(path.resolved_codes())
        .filter(|&(_, code)| code != PathCode::ClosePoly)
        .map(|(&v, _)| placement.transform_point(v))
        .collect();
    if let (Some(&first), Some(&last)) = (points.first(), points.last())
        && first != last
    {
        points.push(first);
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_become_closed_polygons() {
        let outline = polygon(&Rectangle::new((1.0, 2.0), 3.0, 4.0));
        assert_eq!(outline.first(), outline.last());
        assert_eq!(outline.len(), 5);
        assert!(outline.contains(&Point::new(4.0, 6.0)));
        // A bar of no height ends where it starts already.
        let flat = polygon(&Rectangle::new((1.0, 2.0), 3.0, 0.0));
        assert_eq!(flat.len(), 4);
        assert_eq!(flat.first(), flat.last());
    }

    #[test]
    fn boxes_broadcast_single_values() {
        let boxes = Boxes::new([0.0, 1.0, 2.0], [0.0], [0.0], [0.5], [0.5], [3.0, 1.0, 2.0]);
        assert_eq!(boxes.len(), 3);
        let fig = crate::figure::Figure::new();
        let ax = fig.subplot_3d();
        let bars = ax
            .bar3d(boxes.clone(), Bar3DOptions::default())
            .expect("broadcastable");
        fig.layout();
        assert_eq!(bars.paths().len(), 18, "six faces per box");
        let bad = Boxes {
            y: vec![0.0, 1.0].into(),
            ..boxes
        };
        assert!(ax.bar3d(bad, Bar3DOptions::default()).is_err());
    }

    #[test]
    fn categorical_boxes_size_through_their_corners() {
        use crate::dates::{DateTime, TimeDelta};
        let ax = crate::figure::Figure::new().subplot_3d();
        let day = DateTime::new(2020, 1, 1, 0, 0, 0, 0).expect("a date");
        let half_day = TimeDelta::new(0, 43_200, 0).expect("a duration");
        let boxes = Boxes::new(["a", "b"], [day], [0.0], [0.5], half_day, [1.0, 2.0]);
        ax.bar3d(boxes, Bar3DOptions::default()).expect("boxes");
        assert_eq!(ax.xaxis().ticklabels(false).expect("ticks"), ["a", "b"]);
        let (low, high) = ax.ylim();
        // The boxes span half a day from 2020-01-01 (day 18262).
        assert!(
            low < 18_262.0 && high > 18_262.5 && high < 18_263.0,
            "{low} {high}"
        );
    }

    #[test]
    fn cuboid_faces_point_outwards() {
        use crate::mplot3d::shade::polygon_normal;
        let center = [0.5, 0.5, 0.5];
        for face in CUBOID {
            let n = polygon_normal(&face);
            let mid = [0, 1, 2].map(|k| face.iter().map(|p| p[k]).sum::<f64>() / 4.0);
            let outward: f64 = (0..3).map(|k| n[k] * (mid[k] - center[k])).sum();
            assert!(outward > 0.0);
        }
    }
}
