//! Filling between two 3-D curves (matplotlib 3.10's
//! `Axes3D.fill_between`).

use std::cmp::Ordering;

use super::art3d::ZSort;
use super::collections3d::{NewPolys, PolyFaces, resolve_all};
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::{Vec3, cross, dot, norm, sub};
use super::plot::spread;
use super::shade::LightSource;
use crate::collections::{Collection, CollectionProps, EdgeColors};
use crate::numeric::contiguous_regions;
use crate::units::AxisValues;

/// The points of a curve in 3-D before unit conversion: x, y and z as
/// plotting values, each one value for all the points or one per point
/// (one curve of [`Axes3D::fill_between`]). Any triple of plotting values
/// converts into it.
///
/// ```
/// use plt::mplot3d::Curve3D;
/// use plt::units::AxisValues;
/// let curve = Curve3D::from((&[0.0, 1.0], ["a", "b"], [2.0]));
/// assert_eq!(curve.z, AxisValues::Numbers(vec![2.0]));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Curve3D {
    /// The x coordinates.
    pub x: AxisValues,
    /// The y coordinates.
    pub y: AxisValues,
    /// The z coordinates.
    pub z: AxisValues,
}

impl<X, Y, Z> From<(X, Y, Z)> for Curve3D
where
    X: Into<AxisValues>,
    Y: Into<AxisValues>,
    Z: Into<AxisValues>,
{
    fn from((x, y, z): (X, Y, Z)) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
        }
    }
}

/// How the area between the two curves is cut into polygons (matplotlib's
/// `mode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FillMode {
    /// `Polygon` when every point lies on one plane, `Quad` otherwise.
    #[default]
    Auto,
    /// One quadrilateral between each pair of consecutive points.
    Quad,
    /// One polygon per filled region joining the two curves.
    Polygon,
}

/// Options of [`Axes3D::fill_between`].
#[derive(Clone, Debug, Default)]
pub struct FillBetween3DOptions {
    /// Fill between points `i` and `i + 1` only where both are set, one
    /// flag per point (matplotlib's `where`; `None` fills everywhere).
    pub mask: Option<Vec<bool>>,
    /// How the area is cut into polygons.
    pub mode: FillMode,
    /// Whether the faces are shaded by the light; by default for
    /// [`FillMode::Quad`] only.
    pub shade: Option<bool>,
    /// The light shading the faces.
    pub lightsource: Option<LightSource>,
    /// Style of the polygons; without face colours the next colour of the
    /// patch property cycle is used. Face colours (and edge colours given
    /// as such) are shaded, a `color` is applied as is.
    pub props: CollectionProps,
    /// Whether the points outside the view limits are hidden, breaking the
    /// polygons reaching them (matplotlib's `axlim_clip`).
    pub axlim_clip: bool,
}

impl Axes3D {
    /// Fills the area between the curves `(x1, y1, z1)` and `(x2, y2,
    /// z2)`, each coordinate holding one value or one per point
    /// (matplotlib's `fill_between`); the coordinates are numbers, dates,
    /// category names or user values, converted on their axes as by
    /// [`Axes3D::plot`].
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::mplot3d::{FillBetween3DOptions, FillMode};
    /// let ax = Figure::new().subplot_3d();
    /// let x = [0.0, 1.0, 2.0, 3.0];
    /// let options = FillBetween3DOptions { mode: FillMode::Quad, ..Default::default() };
    /// ax.fill_between((&x, [0.0], [0.0]), (&x, [1.0], [1.0, 2.0, 1.0, 2.0]), options)?;
    /// assert_eq!(ax.zlim(), (-0.041666666666666664, 2.0416666666666665));
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates do not broadcast to one length, the mask does
    /// not have one flag per point, or a colour is invalid.
    pub fn fill_between(
        &self,
        first: impl Into<Curve3D>,
        second: impl Into<Curve3D>,
        options: FillBetween3DOptions,
    ) -> Result<Collection, Axes3DError> {
        let had_data = self.has_data();
        let curves = [first.into(), second.into()];
        let n = (curves.iter())
            .flat_map(|curve| [&curve.x, &curve.y, &curve.z])
            .map(AxisValues::len)
            .max()
            .unwrap_or(0);
        for coordinates in curves
            .iter()
            .flat_map(|curve| [&curve.x, &curve.y, &curve.z])
        {
            if coordinates.len() != 1 {
                check_len("fill_between coordinates", n, coordinates.len())?;
            }
        }
        let [line1, line2] = curves.map(|Curve3D { x, y, z }| {
            let [x, y, z] = self.convert_xyz([x, y, z]).map(|values| spread(values, n));
            (0..n).map(|i| [x[i], y[i], z[i]]).collect::<Vec<Vec3>>()
        });
        let mut props = options.props;
        let faces = match props.facecolors.take() {
            Some(colors) => colors,
            None => vec![self.plane().next_patch_color()],
        };
        let mut selected = match options.mask {
            Some(mask) => {
                check_len("where", n, mask.len())?;
                mask
            }
            None => vec![true; n],
        };
        for (flag, p) in selected.iter_mut().zip(&line1) {
            *flag &= !p[0].is_nan();
        }
        let mode = match options.mode {
            FillMode::Auto => {
                let points: Vec<Vec3> = line1
                    .iter()
                    .zip(&selected)
                    .chain(line2.iter().zip(&selected))
                    .filter_map(|(p, &on)| on.then_some(*p))
                    .collect();
                if all_points_on_plane(points, 1e-12) {
                    FillMode::Polygon
                } else {
                    FillMode::Quad
                }
            }
            mode => mode,
        };
        let polys = polygons(&line1, &line2, &selected, mode);
        let shade = options.shade.unwrap_or(mode == FillMode::Quad);
        let light = shade.then(|| options.lightsource.unwrap_or_else(LightSource::mplot3d));
        let shaded_edges = match (shade, props.edgecolors.take()) {
            (true, Some(EdgeColors::Colors(edges))) => Some(resolve_all(&edges)?),
            (_, edges) => {
                props.edgecolors = edges;
                None
            }
        };
        let collection = self.add_polys(NewPolys {
            faces: polys,
            colors: PolyFaces::Colors(resolve_all(&faces)?),
            shaded_edges,
            light,
            zsort: ZSort::Average,
            props,
            axlim_clip: options.axlim_clip,
        })?;
        let pairs = |i: usize| -> Vec<f64> { line1.iter().chain(&line2).map(|p| p[i]).collect() };
        self.auto_scale_xyz(&pairs(0), &pairs(1), Some(&pairs(2)), had_data)?;
        Ok(collection)
    }
}

/// The polygons filling each region of consecutive selected points.
fn polygons(line1: &[Vec3], line2: &[Vec3], selected: &[bool], mode: FillMode) -> Vec<Vec<Vec3>> {
    let mut polys = Vec::new();
    for (start, end) in contiguous_regions(selected) {
        let (a, b) = (&line1[start..end], &line2[start..end]);
        if mode == FillMode::Polygon {
            polys.push(a.iter().chain(b.iter().rev()).copied().collect());
        } else {
            polys.extend((0..a.len() - 1).map(|i| vec![a[i], a[i + 1], b[i + 1], b[i]]));
        }
    }
    polys
}

/// Lexicographic order of points, as `np.unique(..., axis=0)` sorts.
fn lexicographic(a: &Vec3, b: &Vec3) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.total_cmp(y))
        .find(|o| o.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// `points` sorted and without duplicates (`np.unique(points, axis=0)`).
fn unique(mut points: Vec<Vec3>) -> Vec<Vec3> {
    points.sort_by(lexicographic);
    points.dedup_by(|a, b| lexicographic(a, b).is_eq());
    points
}

/// Whether the points, NaN ones ignored, lie on one plane within `atol`
/// (matplotlib's `art3d._all_points_on_plane`).
fn all_points_on_plane(points: Vec<Vec3>, atol: f64) -> bool {
    let points = unique(
        points
            .into_iter()
            .filter(|p| !p.iter().any(|c| c.is_nan()))
            .collect(),
    );
    if points.len() <= 3 {
        return true;
    }
    let directions = unique(
        points[1..]
            .iter()
            .map(|&p| {
                let v = sub(p, points[0]);
                let length = norm(v);
                v.map(|c| c / length)
            })
            .collect(),
    );
    if directions.len() <= 2 {
        return true;
    }
    let first = directions[0];
    let independent: Vec<Vec3> = std::iter::once(first)
        .chain(
            directions[1..]
                .iter()
                .copied()
                .filter(|&v| norm(cross(first, v)).abs() > atol),
        )
        .collect();
    if independent.len() <= 2 {
        return true;
    }
    let normal = cross(independent[0], independent[1]);
    let normal = normal.map(|c| c / norm(normal));
    independent.iter().all(|&v| dot(normal, v).abs() <= atol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn planes_are_detected_like_matplotlib() {
        let flat: Vec<Vec3> = (0..6)
            .map(|i| {
                let t = f64::from(i);
                [t, 2.0 * t, t * t]
            })
            .map(|p| [p[0], p[0] + 1.0, p[2]])
            .collect();
        assert!(all_points_on_plane(flat, 1e-12), "x - y + 1 = 0");
        let helix: Vec<Vec3> = (0..8)
            .map(|i| {
                let t = f64::from(i);
                [t.cos(), t.sin(), t]
            })
            .collect();
        assert!(!all_points_on_plane(helix, 1e-12));
        assert!(all_points_on_plane(vec![[0.0, 0.0, f64::NAN]; 9], 1e-12));
    }

    #[test]
    fn quads_and_polygons() {
        let a = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        let b = [[0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [2.0, 1.0, 1.0]];
        assert_eq!(polygons(&a, &b, &[true; 3], FillMode::Quad).len(), 2);
        let polygon = polygons(&a, &b, &[true; 3], FillMode::Polygon);
        assert_eq!(polygon.len(), 1);
        assert_eq!(polygon[0][3], b[2], "the second curve runs backwards");
        assert_eq!(
            polygons(&a, &b, &[true, false, true], FillMode::Quad).len(),
            0
        );
    }

    #[test]
    fn fills_autoscale_to_both_curves() {
        let ax = Figure::new().subplot_3d();
        let x = [0.0, 1.0, 2.0];
        ax.fill_between(
            (&x, [0.0], [0.0]),
            (&x, [1.0], [5.0]),
            FillBetween3DOptions::default(),
        )
        .unwrap();
        let (lo, hi) = ax.zlim();
        assert!(lo < 0.0 && hi > 5.0);
        assert!(
            ax.fill_between(
                (&x, [0.0, 1.0], [0.0]),
                (&x, [0.0], [0.0]),
                FillBetween3DOptions::default()
            )
            .is_err()
        );
        let day = |d| crate::dates::DateTime::new(2020, 1, d, 0, 0, 0, 0).unwrap();
        ax.fill_between(
            (["a", "b", "c"], [0.0], vec![day(1), day(2), day(3)]),
            (["a", "b", "c"], [1.0], [18_262.0]),
            FillBetween3DOptions::default(),
        )
        .unwrap();
        assert_eq!(ax.xaxis().categories().unwrap().names(), ["a", "b", "c"]);
        assert_eq!(
            ax.zaxis().converter(),
            Some(crate::units::UnitConverter::Dates)
        );
    }
}
