//! Bars in 3-D: boxes (matplotlib's `bar3d`) and flat 2-D bars placed in
//! a plane (matplotlib's `Axes3D.bar`).

use std::cmp::Ordering;

use plt_core::{Path, PathCode, Point};

use super::art3d::{Art3D, ProjectedOutline, ZSort};
use super::collections3d::{NewPolys, PolyFaces, resolve_all};
use super::coords::Axis3;
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::{broadcast, columns};
use super::shade::LightSource;
use crate::artist::{Artist, ArtistHandle};
use crate::axes::BarOptions;
use crate::collections::{Collection, CollectionProps};
use crate::colors::Color;
use crate::patches::{Patch, PatchArtist, PatchShape, Rectangle};

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

/// The geometry of the boxes of [`Axes3D::bar3d`]: anchor corners and
/// sizes, each holding one value per bar or one for all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boxes<'a> {
    /// x of the anchor corners.
    pub x: &'a [f64],
    /// y of the anchor corners.
    pub y: &'a [f64],
    /// z of the anchor corners.
    pub z: &'a [f64],
    /// Widths along x.
    pub dx: &'a [f64],
    /// Depths along y.
    pub dy: &'a [f64],
    /// Heights along z.
    pub dz: &'a [f64],
}

impl Boxes<'_> {
    /// The anchors and sizes broadcast to one entry per bar.
    fn broadcast(&self) -> Result<[Vec<f64>; 6], Axes3DError> {
        let arrays = [self.x, self.y, self.z, self.dx, self.dy, self.dz];
        let n = arrays.iter().map(|a| a.len()).max().unwrap_or(0);
        let names = ["x", "y", "z", "dx", "dy", "dz"];
        let mut out: [Vec<f64>; 6] = Default::default();
        for ((slot, values), name) in out.iter_mut().zip(arrays).zip(names) {
            *slot = broadcast(name, values, n)?;
        }
        Ok(out)
    }
}

impl Axes3D {
    /// Boxes with corners at `(x, y, z)` and sizes `(dx, dy, dz)`
    /// (matplotlib's `bar3d`).
    ///
    /// # Errors
    ///
    /// When the arrays do not broadcast or a colour is invalid.
    pub fn bar3d(
        &self,
        boxes: Boxes<'_>,
        options: Bar3DOptions,
    ) -> Result<Collection, Axes3DError> {
        let had_data = self.has_data();
        let [x, y, z, dx, dy, dz] = boxes.broadcast()?;
        let n = x.len();
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
    /// 3.11 fails to draw once a corner is hidden).
    ///
    /// # Errors
    ///
    /// When `zs` does not broadcast, or as [`crate::axes::Axes::bar`].
    pub fn bar(
        &self,
        left: &[f64],
        height: &[f64],
        zs: &[f64],
        zdir: Axis3,
        options: BarOptions,
    ) -> Result<Vec<Patch<ProjectedOutline>>, Axes3DError> {
        let had_data = self.has_data();
        check_len("height", left.len(), height.len())?;
        let zs = broadcast("zs", zs, left.len())?;
        let container = self.plane().bar(left.to_vec(), height, options)?;
        let mut all_points = Vec::new();
        let mut patches = Vec::with_capacity(container.patches.len());
        for (rectangle, &z) in container.patches.iter().zip(&zs) {
            let outline: Vec<Vec3> = polygon(&rectangle.shape())
                .into_iter()
                .map(|p| zdir.juggle([p.x, p.y, z]))
                .collect();
            all_points.extend_from_slice(&outline);
            patches.push(self.lift_patch(rectangle, outline));
        }
        let [xs, ys, zs] = columns(&all_points);
        self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
        Ok(patches)
    }

    /// Replaces a 2-D rectangle of the plane by a patch of the same style
    /// following the 3-D `outline` (matplotlib's `patch_2d_to_3d`).
    fn lift_patch(
        &self,
        rectangle: &Patch<Rectangle>,
        outline: Vec<Vec3>,
    ) -> Patch<ProjectedOutline> {
        let (style, base) = rectangle
            .raw()
            .with::<PatchArtist<Rectangle>, _>(|r| (r.style.clone(), r.base().clone()));
        rectangle.remove();
        let mut patch = PatchArtist::new(ProjectedOutline(Path::default()), crate::rc::params());
        patch.style = style;
        *patch.base_mut() = base;
        Patch::from_raw(self.add_artist(patch, Art3D::Patch(outline), false))
    }
}

/// The outline of a rectangle as a closed polygon whose last vertex
/// repeats the first (matplotlib's `_get_patch_verts`).
fn polygon(rectangle: &Rectangle) -> Vec<Point> {
    let path = rectangle.path();
    let placement = rectangle.patch_transform();
    let first = path.vertices().first().copied().unwrap_or(Point::ORIGIN);
    path.vertices()
        .iter()
        .zip(path.resolved_codes())
        .map(|(&v, code)| {
            let v = if code == PathCode::ClosePoly {
                first
            } else {
                v
            };
            placement.transform_point(v)
        })
        .collect()
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
    }

    #[test]
    fn boxes_broadcast_single_values() {
        let boxes = Boxes {
            x: &[0.0, 1.0, 2.0],
            y: &[0.0],
            z: &[0.0],
            dx: &[0.5],
            dy: &[0.5],
            dz: &[3.0, 1.0, 2.0],
        };
        let [x, y, _, _, _, dz] = boxes.broadcast().expect("broadcastable");
        assert_eq!((x.len(), y, dz), (3, vec![0.0; 3], vec![3.0, 1.0, 2.0]));
        let bad = Boxes {
            y: &[0.0, 1.0],
            ..boxes
        };
        assert!(bad.broadcast().is_err());
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
