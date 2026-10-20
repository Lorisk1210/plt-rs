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
    /// Whether the corners outside the view limits are hidden, breaking
    /// the faces reaching them (matplotlib's `axlim_clip`).
    pub axlim_clip: bool,
    /// Where the corners of the cells are: one point per corner, one more
    /// along each axis than `filled` has cells (matplotlib's `x, y, z`
    /// arguments); `None` puts corner `[i, j, k]` at `(i, j, k)`.
    pub corners: Option<Array3<Vec3>>,
}

impl Default for VoxelOptions {
    fn default() -> Self {
        Self {
            facecolors: None,
            edgecolors: None,
            shade: true,
            lightsource: None,
            props: CollectionProps::default(),
            axlim_clip: false,
            corners: None,
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

/// The visible faces of every filled voxel as the indices of their
/// corners, four per face, in the order matplotlib finds them: voxels in
/// order of their first face.
fn visible_faces(filled: &Array3<bool>) -> Vec<([usize; 3], Vec<[usize; 3]>)> {
    let size = filled.shape();
    let is_filled = |cell: [usize; 3]| filled.get(cell).copied().unwrap_or(false);
    let mut voxels: Vec<([usize; 3], Vec<[usize; 3]>)> = Vec::new();
    let mut index: HashMap<[usize; 3], usize> = HashMap::new();
    let mut add = |cell: [usize; 3], corner: [usize; 3], square: &[[usize; 3]; 4]| {
        let slot = *index.entry(cell).or_insert_with(|| {
            voxels.push((cell, Vec::new()));
            voxels.len() - 1
        });
        let face = square.iter().map(|v| [0, 1, 2].map(|i| corner[i] + v[i]));
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
    /// Cubes in the cells of `filled`: unit cubes with `filled[0, 0, 0]`'s
    /// lower corner at the origin, or the hexahedra between the
    /// [`VoxelOptions::corners`]; faces between two filled cells are not
    /// drawn. Returns one collection per voxel with visible faces, with
    /// its cell (matplotlib's `voxels([x, y, z,] filled, ...)`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::mplot3d::{Array3, VoxelOptions};
    /// let ax = Figure::new().subplot_3d();
    /// let filled = Array3::from_fn([2, 1, 1], |[i, _, _]| i == 1);
    /// // Cells half a unit wide along x.
    /// let corners = Array3::from_fn([3, 2, 2], |[i, j, k]| [i as f64 / 2.0, j as f64, k as f64]);
    /// let options = VoxelOptions { corners: Some(corners), ..VoxelOptions::default() };
    /// let voxels = ax.voxels(&filled, options)?;
    /// assert_eq!(voxels.len(), 1);
    /// assert!(ax.xlim().1 < 1.1);
    /// # Ok::<(), plt::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the corners are not one more than the cells along each axis,
    /// per-cell colours do not cover the filled cells or a colour is
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
        let corner_shape = [nx + 1, ny + 1, nz + 1];
        let corners = match options.corners {
            Some(corners) if corners.shape() == corner_shape => corners,
            Some(corners) => {
                return Err(Axes3DError::Shape {
                    argument: "voxel corners",
                    expected: corner_shape.iter().product(),
                    actual: corners.data.len(),
                });
            }
            None => Array3::from_fn(corner_shape, |c| c.map(|v| v as f64)),
        };
        let position = |corner: &[usize; 3]| {
            *corners
                .get(*corner)
                .expect("faces only reach the corners of the grid")
        };
        // Scale to the whole grid of corners, even if only its centre is filled.
        let [xs, ys, zs] = columns(&corners.data);
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
        for (cell, face_corners) in visible_faces(filled) {
            let vertices: Vec<Vec3> = face_corners.iter().map(position).collect();
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
                axlim_clip: options.axlim_clip,
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
    fn corners_place_the_cubes() {
        let ax = crate::figure::Figure::new().subplot_3d();
        let filled = Array3::from_fn([1, 1, 1], |_| true);
        let corners = Array3::from_fn([2, 2, 2], |[i, j, k]| {
            [10.0 + i as f64, 2.0 * j as f64, -(k as f64)]
        });
        let options = VoxelOptions {
            corners: Some(corners),
            ..VoxelOptions::default()
        };
        assert_eq!(ax.voxels(&filled, options).expect("corners fit").len(), 1);
        let inside = |(low, high): (f64, f64), from: f64, to: f64| {
            low <= from && high >= to && high - low < 1.2 * (to - from)
        };
        assert!(inside(ax.xlim(), 10.0, 11.0));
        assert!(inside(ax.ylim(), 0.0, 2.0));
        assert!(inside(ax.zlim(), -1.0, 0.0));
    }

    #[test]
    fn corners_must_surround_the_cells() {
        let ax = crate::figure::Figure::new().subplot_3d();
        let filled = Array3::from_fn([2, 1, 1], |_| true);
        let options = VoxelOptions {
            corners: Some(Array3::from_fn([2, 2, 2], |c| c.map(|v| v as f64))),
            ..VoxelOptions::default()
        };
        assert!(matches!(
            ax.voxels(&filled, options),
            Err(Axes3DError::Shape {
                argument: "voxel corners",
                expected: 12,
                actual: 8,
            })
        ));
    }

    #[test]
    fn array_indexing_is_c_ordered() {
        let a = Array3::from_fn([2, 3, 4], |[i, j, k]| i * 100 + j * 10 + k);
        assert_eq!(a.get([1, 2, 3]), Some(&123));
        assert_eq!(a.get([2, 0, 0]), None);
        assert_eq!(a.shape(), [2, 3, 4]);
    }
}
