//! Voxels (matplotlib's `Axes3D.voxels`), whose arguments the Python side
//! resolved and broadcast as matplotlib does.

use pltrs::collections::Collection;
use pltrs::colors::LightSource;
use pltrs::mplot3d::{Array3, Axes3D, VoxelColors, VoxelOptions};

use super::super::collections::rgba_rows;
use crate::binding::prelude::*;

/// The shape of a 3-D array, or `name`'s matplotlib error.
fn shape3(shape: &[usize], name: &str) -> PyResult<[usize; 3]> {
    match *shape {
        [nx, ny, nz] => Ok([nx, ny, nz]),
        _ => Err(PyErr::value_error(format!(
            "Argument {name} must be 3-dimensional"
        ))),
    }
}

/// Cell values in C order as an array of `shape`.
fn cells<T: Clone>(shape: [usize; 3], values: &[T]) -> Array3<T> {
    let [_, ny, nz] = shape;
    Array3::from_fn(shape, |[i, j, k]| values[(i * ny + j) * nz + k].clone())
}

/// One colour per cell of `shape` from RGBA rows of shape `(nx, ny, nz,
/// 4)`.
fn cell_colors(shape: [usize; 3], array: &NdArray) -> PyResult<VoxelColors> {
    let rows = NdArray::with_shape(array.data.clone(), vec![array.data.len() / 4, 4])?;
    let colors = rgba_rows(&rows)?;
    if colors.len() != shape.iter().product::<usize>() {
        return Err(PyErr::value_error(
            "When multidimensional, colors must match the shape of filled",
        ));
    }
    Ok(VoxelColors::PerCell(cells(shape, &colors)))
}

/// The corners of the cells: one point per corner from the `x`, `y` and
/// `z` grids, one larger than `filled` along each axis.
fn corners((x, y, z): &(NdArray, NdArray, NdArray)) -> PyResult<Array3<[f64; 3]>> {
    let shape = shape3(&x.shape, "x")?;
    let points: Vec<[f64; 3]> = (x.data.iter().zip(&y.data).zip(&z.data))
        .map(|((&x, &y), &z)| [x, y, z])
        .collect();
    Ok(cells(shape, &points))
}

/// The voxels made: each one's cell and collection.
type Voxels = Vec<((usize, usize, usize), Collection)>;

/// Cubes in the filled cells of `filled` (matplotlib's `voxels`): unit
/// cubes, or the hexahedra between `corners`; `facecolors` (`None`: the
/// next patch colour) and `edgecolors` are RGBA rows, one per cell; the
/// faces are shaded when `shade`, by the light at `(azdeg, altdeg)` if
/// given. Returns each voxel's cell and collection.
fn voxels(
    axes: Axes3D,
    filled: NdArray<bool>,
    corner_grids: Option<(NdArray, NdArray, NdArray)>,
    (facecolors, edgecolors): (Option<NdArray>, Option<NdArray>),
    (shade, light): (bool, Option<(f64, f64)>),
    axlim_clip: bool,
) -> PyResult<Voxels> {
    let shape = shape3(&filled.shape, "filled")?;
    let options = VoxelOptions {
        facecolors: facecolors
            .map(|colors| cell_colors(shape, &colors))
            .transpose()?,
        edgecolors: edgecolors
            .map(|colors| cell_colors(shape, &colors))
            .transpose()?,
        shade,
        lightsource: light.map(|(azdeg, altdeg)| LightSource::new(azdeg, altdeg)),
        axlim_clip,
        corners: corner_grids.as_ref().map(corners).transpose()?,
        ..VoxelOptions::default()
    };
    let made = axes.voxels(&cells(shape, &filled.data), options)?;
    Ok(made
        .into_iter()
        .map(|([i, j, k], collection)| ((i, j, k), collection))
        .collect())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("voxels", voxels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn voxels_return_their_cells() {
        let ax = Figure::new().subplot_3d();
        let filled =
            NdArray::<bool>::with_shape(vec![true, false, false, true], vec![2, 2, 1]).unwrap();
        let made = voxels(ax.clone(), filled, None, (None, None), (true, None), false).unwrap();
        let cells: Vec<_> = made.iter().map(|(cell, _)| *cell).collect();
        assert_eq!(cells, [(0, 0, 0), (1, 1, 0)]);
        let flat = NdArray::<bool>::with_shape(vec![true], vec![1]).unwrap();
        assert!(voxels(ax, flat, None, (None, None), (true, None), false).is_err());
    }

    #[test]
    fn cells_are_c_ordered() {
        let array = cells([2, 1, 2], &[0, 1, 2, 3]);
        assert_eq!(array.get([1, 0, 0]), Some(&2));
    }
}
