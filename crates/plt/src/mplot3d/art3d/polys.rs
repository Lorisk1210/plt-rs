//! Polygons in 3-D, drawn back to front (matplotlib's `Poly3DCollection`).

use plt_core::{Path, Point, Rgba};

use crate::collections::{CollectionArtist, EdgeColors};
use crate::colors::Color;
use crate::mplot3d::camera::Projection;
use crate::mplot3d::math::Vec3;
use crate::numeric::argsort;

/// How the depth of a polygon is computed from the projected depths of
/// its vertices, for drawing polygons back to front (matplotlib's
/// `zsort`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ZSort {
    /// The mean depth.
    #[default]
    Average,
    /// The nearest vertex.
    Min,
    /// The farthest vertex.
    Max,
}

/// numpy's default fill value for floats (`np.ma.default_fill_value(1.0)`),
/// which its masked `min` and `max` leave in the data of a fully masked row.
const MASKED_FILL: f64 = 1e20;

impl ZSort {
    /// The depth of a polygon whose vertices have depths `zs`, ignoring
    /// the vertices that could not be projected (matplotlib's masked
    /// vertices). When none could, it is what numpy's masked reductions
    /// leave in the data matplotlib sorts by: 0 for the mean and
    /// [`MASKED_FILL`] for the extremes.
    fn reduce(self, zs: impl Iterator<Item = f64>) -> f64 {
        let (count, sum, min, max) = zs.filter(|z| !z.is_nan()).fold(
            (0_u32, 0.0, f64::INFINITY, f64::NEG_INFINITY),
            |(n, sum, min, max), z| (n + 1, sum + z, min.min(z), max.max(z)),
        );
        if count == 0 {
            return match self {
                Self::Average => 0.0,
                Self::Min | Self::Max => MASKED_FILL,
            };
        }
        match self {
            Self::Average => sum / f64::from(count),
            Self::Min => min,
            Self::Max => max,
        }
    }
}

/// The 3-D state of a polygon collection.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Polys3D {
    /// The polygons, possibly with different numbers of vertices.
    pub(crate) faces: Vec<Vec<Vec3>>,
    pub(crate) zsort: ZSort,
    /// A fixed z position whose depth replaces that of the polygons in
    /// computed z-order (matplotlib's `set_sort_zpos`).
    pub(crate) sort_zpos: Option<f64>,
    /// Face colours in polygon order (matplotlib's `_facecolor3d`).
    pub(crate) facecolors: Vec<Rgba>,
    /// Edge colours in polygon order (matplotlib's `_edgecolor3d`).
    pub(crate) edgecolors: Vec<Rgba>,
    /// The 2-D paths whose codes the projected polygons keep, one per
    /// face with one vertex per face point, for polygons lifted from a
    /// 2-D collection (matplotlib's `_codes3d`); `None` closes every
    /// polygon instead.
    pub(crate) templates: Option<Vec<Path>>,
}

impl Polys3D {
    /// Writes the projected polygons, deepest first, and their colours
    /// into `collection` and returns the collection's depth.
    pub(crate) fn project(
        &self,
        collection: &mut CollectionArtist,
        projection: &Projection,
    ) -> f64 {
        let projected: Vec<Vec<Vec3>> = self
            .faces
            .iter()
            .map(|face| face.iter().map(|&p| projection.project(p)).collect())
            .collect();
        let order = self.depth_order(&projected);
        collection.paths = match &self.templates {
            Some(templates) => order
                .iter()
                .map(|&i| {
                    let mut path = templates[i].clone();
                    for (vertex, p) in path.vertices_mut().iter_mut().zip(&projected[i]) {
                        *vertex = Point::new(p[0], p[1]);
                    }
                    path
                })
                .collect(),
            None => closed_polygons(&projected, &order),
        };
        let (faces, edges) = self.sorted_colors(&order);
        collection.facecolors = Some(faces.into_iter().map(Color::from).collect());
        collection.edgecolors = EdgeColors::Colors(edges.into_iter().map(Color::from).collect());
        match self.sort_zpos {
            // matplotlib projects the sort position without the scales.
            Some(z) => projection.project_transformed([0.0, 0.0, z])[2],
            None => projected
                .iter()
                .flatten()
                .map(|p| p[2])
                .reduce(f64::min)
                .unwrap_or(f64::NAN),
        }
    }

    /// The order the `projected` faces are drawn in, deepest first
    /// (matplotlib's `argsort(face_z)[::-1]`, faces of equal depth in
    /// numpy's order).
    fn depth_order(&self, projected: &[Vec<Vec3>]) -> Vec<usize> {
        let face_z: Vec<f64> = projected
            .iter()
            .map(|face| self.zsort.reduce(face.iter().map(|p| p[2])))
            .collect();
        let mut order = argsort(&face_z);
        order.reverse();
        order
    }

    /// The face and edge colours of the polygons drawn in `order`
    /// (matplotlib's colour handling in `do_3d_projection`).
    fn sorted_colors(&self, order: &[usize]) -> (Vec<Rgba>, Vec<Rgba>) {
        let n = self.faces.len();
        let faces = repeat_to(&self.facecolors, n);
        let edges = match self.edgecolors.len() {
            len if len == n => self.edgecolors.clone(),
            0 => faces.clone(),
            _ => repeat_to(&self.edgecolors, n),
        };
        let pick = |colors: &[Rgba]| -> Vec<Rgba> { order.iter().map(|&i| colors[i]).collect() };
        let sorted_faces = if faces.is_empty() {
            faces.clone()
        } else {
            pick(&faces)
        };
        let sorted_edges = if self.edgecolors.len() == faces.len() && !edges.is_empty() {
            pick(&edges)
        } else {
            self.edgecolors.clone()
        };
        (sorted_faces, sorted_edges)
    }
}

/// The projected faces in `order` as closed polygons. matplotlib stores
/// ragged polygons in one array padded with NaN up to the longest one,
/// then closes every polygon: the padding makes path conversion drop the
/// closure of the shorter ones.
fn closed_polygons(projected: &[Vec<Vec3>], order: &[usize]) -> Vec<Path> {
    let longest = projected.iter().map(Vec::len).max().unwrap_or(0);
    order
        .iter()
        .map(|&i| {
            let mut vertices: Vec<Point> = projected[i]
                .iter()
                .map(|p| Point::new(p[0], p[1]))
                .collect();
            if let Some(&first) = vertices.first() {
                vertices.resize(longest, Point::new(f64::NAN, f64::NAN));
                vertices.push(first);
            }
            Path::closed(vertices)
        })
        .collect()
}

/// `colors` itself when it has `n` entries, otherwise each colour
/// repeated `n` times (numpy's `repeat(n, axis=0)`).
fn repeat_to(colors: &[Rgba], n: usize) -> Vec<Rgba> {
    if colors.len() == n {
        colors.to_vec()
    } else {
        colors
            .iter()
            .flat_map(|&c| std::iter::repeat_n(c, n))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsort_reductions() {
        let zs = [1.0, 2.0, 6.0];
        assert_eq!(ZSort::Average.reduce(zs.into_iter()), 3.0);
        assert_eq!(ZSort::Min.reduce(zs.into_iter()), 1.0);
        assert_eq!(ZSort::Max.reduce(zs.into_iter()), 6.0);
        let masked = [f64::NAN, 2.0, 4.0];
        assert_eq!(ZSort::Average.reduce(masked.into_iter()), 3.0);
        // numpy: np.average(masked row).data == 0, np.min/np.max == 1e20.
        let hidden = [f64::NAN, f64::NAN];
        assert_eq!(ZSort::Average.reduce(hidden.into_iter()), 0.0);
        assert_eq!(ZSort::Min.reduce(hidden.into_iter()), 1e20);
        assert_eq!(ZSort::Max.reduce(std::iter::empty()), 1e20);
    }

    #[test]
    fn hidden_polygons_sort_by_numpys_masked_depths() {
        // Triangles at depths 1 and -2 around one hidden entirely, whose
        // mean depth 0 puts it between them and whose extreme depth 1e20
        // puts it first.
        let polys = |zsort| Polys3D {
            faces: vec![Vec::new(); 3],
            zsort,
            sort_zpos: None,
            facecolors: Vec::new(),
            edgecolors: Vec::new(),
            templates: None,
        };
        let projected = [
            vec![[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
            vec![[f64::NAN; 3]; 3],
            vec![[0.0, 0.0, -2.0], [1.0, 0.0, -2.0], [0.0, 1.0, -2.0]],
        ];
        assert_eq!(polys(ZSort::Average).depth_order(&projected), [0, 1, 2]);
        assert_eq!(polys(ZSort::Max).depth_order(&projected), [1, 0, 2]);
    }

    #[test]
    fn colours_follow_matplotlib() {
        let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
        let blue = Rgba::new(0.0, 0.0, 1.0, 1.0);
        let polys = Polys3D {
            faces: vec![Vec::new(); 2],
            zsort: ZSort::Average,
            sort_zpos: None,
            facecolors: vec![red, blue],
            edgecolors: Vec::new(),
            templates: None,
        };
        let (faces, edges) = polys.sorted_colors(&[1, 0]);
        assert_eq!(faces, [blue, red]);
        assert!(edges.is_empty(), "no edge colours stay none");
        let single_edge = Polys3D {
            edgecolors: vec![red],
            ..polys.clone()
        };
        assert_eq!(single_edge.sorted_colors(&[1, 0]).1, [red]);
        let per_face = Polys3D {
            edgecolors: vec![red, blue],
            ..polys
        };
        assert_eq!(per_face.sorted_colors(&[1, 0]).1, [blue, red]);
    }

    #[test]
    fn single_colours_repeat() {
        let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
        assert_eq!(repeat_to(&[red], 3).len(), 3);
        assert_eq!(repeat_to(&[red, red], 2).len(), 2);
        assert_eq!(repeat_to(&[], 3), Vec::<Rgba>::new());
    }
}
