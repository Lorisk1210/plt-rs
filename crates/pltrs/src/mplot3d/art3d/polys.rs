//! Polygons in 3-D, drawn back to front (matplotlib's `Poly3DCollection`).

use pltrs_core::{Path, Point, Rgba};

use crate::collections::{CollectionArtist, DepthSorted};
use crate::mplot3d::camera::Projection;
use crate::mplot3d::math::Vec3;
use crate::numeric::argsort;
use crate::stats::pairwise_sum;

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
    /// [`MASKED_FILL`] for the extremes. The mean sums like numpy's: in
    /// pairwise order, the hidden depths filled with zeros.
    fn reduce(self, zs: impl Iterator<Item = f64>) -> f64 {
        let zs: Vec<f64> = zs.collect();
        let (count, min, max) = zs.iter().filter(|z| !z.is_nan()).fold(
            (0_u32, f64::INFINITY, f64::NEG_INFINITY),
            |(n, min, max), &z| (n + 1, min.min(z), max.max(z)),
        );
        if count == 0 {
            return match self {
                Self::Average => 0.0,
                Self::Min | Self::Max => MASKED_FILL,
            };
        }
        match self {
            Self::Average => {
                let filled: Vec<f64> = zs
                    .iter()
                    .map(|&z| if z.is_nan() { 0.0 } else { z })
                    .collect();
                pairwise_sum(&filled) / f64::from(count)
            }
            Self::Min => min,
            Self::Max => max,
        }
    }
}

/// The 3-D state of a polygon collection.
///
/// The 2-D collection keeps its colours (and colour mapping) in polygon
/// order; projecting writes its paths sorted by depth and its colours
/// sorted alike ([`DepthSorted`]).
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
    /// 2-D collection (matplotlib's `_codes3d`); `None` makes polygons
    /// of the faces instead.
    pub(crate) templates: Option<Vec<Path>>,
    /// Whether the polygons made of the faces are closed (matplotlib's
    /// `closed`); open ones are filled but not stroked along their last
    /// edge.
    pub(crate) closed: bool,
}

impl Polys3D {
    /// Writes the projected polygons, deepest first, and their colours
    /// into `collection` and returns the collection's depth.
    pub(crate) fn project(
        &self,
        collection: &mut CollectionArtist,
        projection: &Projection,
    ) -> f64 {
        // matplotlib projects the vertices of every face at once.
        let mut vertices = projection.project_all(&self.faces.concat()).into_iter();
        let projected: Vec<Vec<Vec3>> = (self.faces.iter())
            .map(|face| vertices.by_ref().take(face.len()).collect())
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
            None => polygons(&projected, &order, self.closed),
        };
        collection.depth_sorted = Some(DepthSorted {
            colors: Some(self.sorted_colors(collection, &order)),
            ..DepthSorted::default()
        });
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
    /// numpy's order). matplotlib pads shorter faces with masked vertices
    /// up to the longest, which its reductions see.
    fn depth_order(&self, projected: &[Vec<Vec3>]) -> Vec<usize> {
        let longest = projected.iter().map(Vec::len).max().unwrap_or(0);
        let face_z: Vec<f64> = projected
            .iter()
            .map(|face| {
                let padding = std::iter::repeat_n(f64::NAN, longest - face.len());
                self.zsort.reduce(face.iter().map(|p| p[2]).chain(padding))
            })
            .collect();
        let mut order = argsort(&face_z);
        order.reverse();
        order
    }

    /// The face and edge colours of the polygons drawn in `order`
    /// (matplotlib's colour handling in `do_3d_projection`): those of a
    /// colour-mapped `collection` worked out afresh, as matplotlib's
    /// `update_scalarmappable`, or else the ones kept.
    fn sorted_colors(
        &self,
        collection: &CollectionArtist,
        order: &[usize],
    ) -> (Vec<Rgba>, Vec<Rgba>) {
        let mapped = (collection.mapping.is_some())
            .then(|| collection.base_colors().ok())
            .flatten();
        let (facecolors, edgecolors) = mapped.as_ref().map_or(
            (self.facecolors.as_slice(), self.edgecolors.as_slice()),
            |(faces, edges)| (faces.as_slice(), edges.as_slice()),
        );
        sort_colors(self.faces.len(), order, facecolors, edgecolors)
    }
}

/// The face and edge colours of `n` polygons drawn in `order`, from one
/// colour or one per polygon (no edge colours: none).
fn sort_colors(
    n: usize,
    order: &[usize],
    facecolors: &[Rgba],
    edgecolors: &[Rgba],
) -> (Vec<Rgba>, Vec<Rgba>) {
    let faces = repeat_to(facecolors, n);
    let edges = match edgecolors.len() {
        len if len == n => edgecolors.to_vec(),
        0 => faces.clone(),
        _ => repeat_to(edgecolors, n),
    };
    let pick = |colors: &[Rgba]| -> Vec<Rgba> { order.iter().map(|&i| colors[i]).collect() };
    let sorted_faces = if faces.is_empty() {
        faces.clone()
    } else {
        pick(&faces)
    };
    let sorted_edges = if edgecolors.len() == faces.len() && !edges.is_empty() {
        pick(&edges)
    } else {
        edgecolors.to_vec()
    };
    (sorted_faces, sorted_edges)
}

/// The projected faces in `order` as closed or open polygons
/// (matplotlib's `PolyCollection.set_verts(..., closed)`). matplotlib
/// stores ragged polygons in one array padded with NaN up to the longest
/// one, then closes every polygon: the padding makes path conversion drop
/// the closure of the shorter ones.
fn polygons(projected: &[Vec<Vec3>], order: &[usize], closed: bool) -> Vec<Path> {
    let longest = projected.iter().map(Vec::len).max().unwrap_or(0);
    order
        .iter()
        .map(|&i| {
            let mut vertices: Vec<Point> = projected[i]
                .iter()
                .map(|p| Point::new(p[0], p[1]))
                .collect();
            let Some(&first) = vertices.first() else {
                return Path::polyline(vertices);
            };
            vertices.resize(longest, Point::new(f64::NAN, f64::NAN));
            if closed {
                vertices.push(first);
                Path::closed(vertices)
            } else {
                Path::polyline(vertices)
            }
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
    use pltrs_core::PathCode;

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
        // numpy sums eight depths pairwise, where adding them in turn
        // would keep the second 1 (matplotlib's `np.average(pzs, -1)`).
        let cancelling = [1e16, 1.0, -1e16, 1.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(ZSort::Average.reduce(cancelling.into_iter()), 0.0);
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
            closed: true,
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
        let (faces, edges) = sort_colors(2, &[1, 0], &[red, blue], &[]);
        assert_eq!(faces, [blue, red]);
        assert!(edges.is_empty(), "no edge colours stay none");
        assert_eq!(sort_colors(2, &[1, 0], &[red, blue], &[red]).1, [red]);
        assert_eq!(
            sort_colors(2, &[1, 0], &[red, blue], &[red, blue]).1,
            [blue, red]
        );
    }

    #[test]
    fn open_polygons_are_not_closed() {
        let triangle = vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]];
        let closed = polygons(&triangle, &[0], true);
        assert_eq!(closed[0].len(), 4);
        assert_eq!(
            closed[0].resolved_codes().last(),
            Some(&PathCode::ClosePoly)
        );
        let open = polygons(&triangle, &[0], false);
        assert_eq!(open[0].len(), 3);
        assert!(!open[0].resolved_codes().contains(&PathCode::ClosePoly));
    }

    #[test]
    fn single_colours_repeat() {
        let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
        assert_eq!(repeat_to(&[red], 3).len(), 3);
        assert_eq!(repeat_to(&[red, red], 2).len(), 2);
        assert_eq!(repeat_to(&[], 3), Vec::<Rgba>::new());
    }
}
