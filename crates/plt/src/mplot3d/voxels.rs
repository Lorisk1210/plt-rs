//! Voxels: unit cubes filling the cells of a 3-D grid, with only the
//! faces between filled and empty cells drawn (matplotlib's `voxels`).

use std::collections::HashMap;

use plt_core::Rgba;

use super::art3d::ZSort;
use super::collections3d::{NewPolys, PolyFaces};
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::Vec3;
use super::plot::columns;
use super::shade::LightSource;
use crate::collections::{Collection, CollectionProps};
use crate::colors::Color;

/// A value per cell of a 3-D grid, stored with the last index varying
/// fastest (numpy's C order).
#[derive(Clone, Debug, PartialEq)]
pub struct Array3<T> {
    shape: [usize; 3],
    data: Vec<T>,
}

impl<T> Array3<T> {
    /// An array of `shape` with `f([i, j, k])` in every cell.
    pub fn from_fn(shape: [usize; 3], mut f: impl FnMut([usize; 3]) -> T) -> Self {
        let mut data = Vec::with_capacity(shape.iter().product());
        for i in 0..shape[0] {
            for j in 0..shape[1] {
                for k in 0..shape[2] {
                    data.push(f([i, j, k]));
                }
            }
        }
        Self { shape, data }
    }

    /// The number of cells along each axis.
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }

    /// The value of a cell, `None` outside the array.
    pub fn get(&self, [i, j, k]: [usize; 3]) -> Option<&T> {
        let [_, nj, nk] = self.shape;
        (i < self.shape[0] && j < nj && k < nk).then(|| &self.data[(i * nj + j) * nk + k])
    }
}

/// The colours of the voxels: one for all, or one per cell.
#[derive(Clone, Debug, PartialEq)]
pub enum VoxelColors {
    /// The same colour everywhere.
    Single(Color),
    /// One colour per cell, shaped like the filled cells.
    PerCell(Array3<Color>),
}

impl VoxelColors {
    fn at(&self, cell: [usize; 3]) -> Option<&Color> {
        match self {
            Self::Single(color) => Some(color),
            Self::PerCell(colors) => colors.get(cell),
        }
    }
}

/// Options of [`Axes3D::voxels`].
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelOptions {
    /// Face colours; `None` takes the next colour of the patch property
    /// cycle.
    pub facecolors: Option<VoxelColors>,
    /// Edge colours, shaded like the faces; `None` leaves the default
    /// edges (see also `props`).
    pub edgecolors: Option<VoxelColors>,
    /// Whether faces turned away from the light are darkened.
    pub shade: bool,
    /// The light shading the faces.
    pub lightsource: Option<LightSource>,
    /// Further properties of every voxel's collection.
    pub props: CollectionProps,
}

impl Default for VoxelOptions {
    fn default() -> Self {
        Self {
            facecolors: None,
            edgecolors: None,
            shade: true,
            lightsource: None,
            props: CollectionProps::default(),
        }
    }
}

/// The corners of a unit square in the plane of the first two axes.
const SQUARE: [[usize; 3]; 4] = [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]];

/// The cyclic axis permutations matplotlib walks: `P · [p, q, r]` for
/// the identity and its two rotations (`np.roll(eye, n, axis=0)`).
const PERMUTATIONS: [fn([usize; 3]) -> [usize; 3]; 3] = [
    |[p, q, r]| [p, q, r],
    |[p, q, r]| [r, p, q],
    |[p, q, r]| [q, r, p],
];

/// The visible faces of every filled voxel, in the order matplotlib finds
/// them: voxels in order of their first face.
fn visible_faces(filled: &Array3<bool>) -> Vec<([usize; 3], Vec<Vec3>)> {
    let size = filled.shape();
    let is_filled = |cell: [usize; 3]| filled.get(cell).copied().unwrap_or(false);
    let mut voxels: Vec<([usize; 3], Vec<Vec3>)> = Vec::new();
    let mut index: HashMap<[usize; 3], usize> = HashMap::new();
    let mut add = |cell: [usize; 3], corner: [usize; 3], square: &[[usize; 3]; 4]| {
        let slot = *index.entry(cell).or_insert_with(|| {
            voxels.push((cell, Vec::new()));
            voxels.len() - 1
        });
        let face = square
            .iter()
            .map(|v| [0, 1, 2].map(|i| (corner[i] + v[i]) as f64));
        voxels[slot].1.extend(face);
    };
    for permute in PERMUTATIONS {
        // P^T · size: the extents along the permuted axes.
        let extent = |axis: usize| {
            let mut unit = [0; 3];
            unit[axis] = 1;
            size[permute(unit).iter().position(|&u| u == 1).unwrap_or(axis)]
        };
        let (pc, qc, rc) = (extent(0), extent(1), extent(2));
        let positive = SQUARE.map(permute);
        let mut negative = positive;
        negative.reverse();
        for p in 0..pc {
            for q in 0..qc {
                if rc == 0 {
                    continue;
                }
                let first = permute([p, q, 0]);
                if is_filled(first) {
                    add(first, first, &negative);
                }
                for r in 0..rc - 1 {
                    let (lower, upper) = (permute([p, q, r]), permute([p, q, r + 1]));
                    match (is_filled(lower), is_filled(upper)) {
                        (true, false) => add(lower, upper, &positive),
                        (false, true) => add(upper, upper, &negative),
                        _ => {}
                    }
                }
                let last = permute([p, q, rc - 1]);
                if is_filled(last) {
                    add(last, permute([p, q, rc]), &positive);
                }
            }
        }
    }
    voxels
}

impl Axes3D {
    /// Unit cubes in the cells of `filled`, `filled[0, 0, 0]` with its
    /// lower corner at the origin; faces between two filled cells are not
    /// drawn. Returns one collection per voxel with visible faces, with
    /// its cell (matplotlib's `voxels(filled, ...)`).
    ///
    /// # Errors
    ///
    /// When per-cell colours do not cover the filled cells or a colour is
    /// invalid.
    pub fn voxels(
        &self,
        filled: &Array3<bool>,
        options: VoxelOptions,
    ) -> Result<Vec<([usize; 3], Collection)>, Axes3DError> {
        let facecolors = match options.facecolors {
            Some(colors) => colors,
            None => VoxelColors::Single(self.plane().next_patch_color()),
        };
        let [nx, ny, nz] = filled.shape();
        // Scale to the whole grid of corners, even if only its centre is filled.
        let corners = Array3::from_fn([nx + 1, ny + 1, nz + 1], |c| c.map(|v| v as f64));
        let points: Vec<Vec3> = corners.data;
        let [xs, ys, zs] = columns(&points);
        self.auto_scale_xyz(&xs, &ys, Some(&zs), false)?;
        let light = options
            .shade
            .then(|| options.lightsource.unwrap_or_else(LightSource::mplot3d));
        let color_at = |colors: &VoxelColors, cell: [usize; 3]| -> Result<Rgba, Axes3DError> {
            let color = colors.at(cell).ok_or(Axes3DError::Shape {
                argument: "voxel colours",
                expected: nx * ny * nz,
                actual: 0,
            })?;
            Ok(color.resolve()?)
        };
        let mut collections = Vec::new();
        for (cell, vertices) in visible_faces(filled) {
            let had_data = self.has_data();
            let faces: Vec<Vec<Vec3>> = vertices.chunks(4).map(<[Vec3]>::to_vec).collect();
            let shaded_edges = options
                .edgecolors
                .as_ref()
                .map(|edges| color_at(edges, cell).map(|c| vec![c]))
                .transpose()?;
            let collection = self.add_polys(NewPolys {
                faces,
                colors: PolyFaces::Colors(vec![color_at(&facecolors, cell)?]),
                shaded_edges,
                light,
                zsort: ZSort::Average,
                props: options.props.clone(),
            })?;
            let [xs, ys, zs] = columns(&vertices);
            self.auto_scale_xyz(&xs, &ys, Some(&zs), had_data)?;
            collections.push((cell, collection));
        }
        Ok(collections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_voxel_shows_six_faces() {
        let filled = Array3::from_fn([1, 1, 1], |_| true);
        let voxels = visible_faces(&filled);
        assert_eq!(voxels.len(), 1);
        assert_eq!(voxels[0].1.len(), 6 * 4);
    }

    #[test]
    fn shared_faces_are_hidden() {
        let filled = Array3::from_fn([2, 1, 1], |_| true);
        let faces: usize = visible_faces(&filled)
            .iter()
            .map(|(_, f)| f.len() / 4)
            .sum();
        assert_eq!(faces, 10);
        let empty = Array3::from_fn([2, 2, 2], |_| false);
        assert_eq!(visible_faces(&empty).len(), 0);
    }

    #[test]
    fn array_indexing_is_c_ordered() {
        let a = Array3::from_fn([2, 3, 4], |[i, j, k]| i * 100 + j * 10 + k);
        assert_eq!(a.get([1, 2, 3]), Some(&123));
        assert_eq!(a.get([2, 0, 0]), None);
        assert_eq!(a.shape(), [2, 3, 4]);
    }
}
