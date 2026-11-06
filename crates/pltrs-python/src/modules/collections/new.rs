//! The constructors of matplotlib's collection classes
//! (`LineCollection(...)`, `PatchCollection(...)`, ...): each makes a
//! loose pltrs (Rust) collection ([`Collection::loose`]), which an axes
//! or a figure then adopts (`_native.artist.adopt_into_axes`).

use pltrs::collections::{Collection, CollectionProps, CollectionSpec, EllipseUnits, MeshShading};
use pltrs::numeric::Grid;
use pltrs::orientation::Orientation;
use pltrs::tri::Triangulation;
use pltrs_core::Point;

use super::super::patches::collection_patches;
use super::super::path::PyPath;
use super::super::plotting::fills::fill_between_options;
use super::super::transforms::points;
use crate::binding::prelude::*;

/// `(N, 2)` arrays as point lists.
fn point_lists(arrays: &[NdArray]) -> PyResult<Vec<Vec<Point>>> {
    arrays.iter().map(points).collect()
}

/// A loose line collection of polylines (matplotlib's `LineCollection`).
fn line_collection(segments: Vec<NdArray>) -> PyResult<Collection> {
    Ok(Collection::loose(CollectionSpec::lines(point_lists(
        &segments,
    )?)))
}

/// A loose collection of closed polygons (matplotlib's `PolyCollection`).
fn poly_collection(verts: Vec<NdArray>) -> PyResult<Collection> {
    Ok(Collection::loose(CollectionSpec::polygons(point_lists(
        &verts,
    )?)))
}

/// A loose collection of paths (matplotlib's `PathCollection`).
fn path_collection(paths: Vec<PyPath>) -> PyResult<Collection> {
    let paths = paths.into_iter().map(|path| path.path).collect();
    Ok(Collection::loose(CollectionSpec::paths(paths)))
}

/// A loose event collection (matplotlib's `EventCollection`): event lines
/// at `positions` along x (along y when `vertical`), centred on
/// `lineoffset` and `linelength` long.
fn event_collection(
    positions: NdArray,
    vertical: bool,
    lineoffset: f64,
    linelength: f64,
) -> PyResult<Collection> {
    let orientation = if vertical {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    };
    let spec = CollectionSpec::events(&positions.into_vec()?, orientation, lineoffset, linelength);
    Ok(Collection::loose(spec))
}

/// A loose collection of the outlines of `patches` (native patch objects;
/// matplotlib's `PatchCollection`), each keeping its own look when
/// `match_original`.
fn patch_collection(patches: Vec<Owned>, match_original: bool) -> PyResult<Collection> {
    let items = collection_patches(&patches)?;
    let spec = CollectionSpec::patches(&items, match_original)?;
    Ok(Collection::loose(spec))
}

/// A loose collection filling between the curves `f1(t)` and `f2(t)`
/// (matplotlib's `FillBetweenPolyCollection`): along y when `horizontal`
/// (`t_direction='y'`), where `mask` allows (`None` everywhere), with
/// interpolated crossings and steps (`None`, `"pre"`, `"mid"` or
/// `"post"`).
fn fill_between_collection(
    horizontal: bool,
    (t, f1, f2): (NdArray, NdArray, NdArray),
    mask: Option<Vec<bool>>,
    interpolate: bool,
    step: Option<String>,
) -> PyResult<Collection> {
    let orientation = if horizontal {
        Orientation::Horizontal
    } else {
        Orientation::Vertical
    };
    let spec = CollectionSpec::fill_between(
        orientation,
        (&t.data, &f1.data, &f2.data),
        fill_between_options(mask, interpolate, step.as_deref()),
    )?;
    Ok(Collection::loose(spec))
}

/// A loose collection of circles of areas `sizes` in points²
/// (matplotlib's `CircleCollection`).
fn circle_collection(sizes: NdArray) -> PyResult<Collection> {
    Ok(Collection::loose(CollectionSpec::circles(
        sizes.into_vec()?,
    )))
}

/// A loose collection of regular polygons (matplotlib's
/// `RegularPolyCollection`).
fn regular_poly_collection(numsides: usize, rotation: f64, sizes: NdArray) -> PyResult<Collection> {
    let spec = CollectionSpec::regular_polygons(numsides, rotation, sizes.into_vec()?);
    Ok(Collection::loose(spec))
}

/// A loose collection of ellipses (matplotlib's `EllipseCollection`), with
/// the units of their sizes.
fn ellipse_collection(
    (widths, heights, angles): (NdArray, NdArray, NdArray),
    units: String,
) -> PyResult<Collection> {
    let units = match units.as_str() {
        "xy" => EllipseUnits::Xy,
        "x" => EllipseUnits::X,
        "y" => EllipseUnits::Y,
        "inches" => EllipseUnits::Inches,
        "width" => EllipseUnits::Width,
        "height" => EllipseUnits::Height,
        "dots" => EllipseUnits::Dots,
        _ => EllipseUnits::Points,
    };
    let spec = CollectionSpec::ellipses(
        &widths.into_vec()?,
        &heights.into_vec()?,
        &angles.into_vec()?,
        units,
    );
    Ok(Collection::loose(spec))
}

/// A loose quad mesh over the vertices `coordinates`, an `(M, N, 2)` array
/// (NaN where masked), Gouraud-shaded with `gouraud`, drawn `antialiased`
/// or not (matplotlib's `QuadMesh(coordinates, antialiased=...,
/// shading=...)`).
fn quad_mesh(coordinates: NdArray, gouraud: bool, antialiased: bool) -> PyResult<Collection> {
    let grid = vertex_grid(&coordinates)?;
    let shading = if gouraud {
        MeshShading::Gouraud
    } else {
        MeshShading::Flat
    };
    let props = CollectionProps {
        antialiased: Some(vec![antialiased]),
        ..CollectionProps::default()
    };
    Ok(Collection::loose(
        CollectionSpec::quad_mesh(grid, shading).with_props(props),
    ))
}

/// A loose quadrilateral mesh drawn cell by cell on the vertices
/// `coordinates`, an `(M, N, 2)` array with NaN for masked vertices
/// (matplotlib's `PolyQuadMesh(coordinates)`).
fn poly_quad_mesh(coordinates: NdArray) -> PyResult<Collection> {
    Ok(Collection::loose(CollectionSpec::poly_quad_mesh(
        vertex_grid(&coordinates)?,
    )))
}

/// The vertex grid of a mesh from its `(M, N, 2)` array of coordinates.
fn vertex_grid(coordinates: &NdArray) -> PyResult<Grid<Point>> {
    let &[rows, cols, 2] = coordinates.shape.as_slice() else {
        return Err(PyErr::value_error(format!(
            "'coordinates' must be 3D with shape (M, N, 2), but your input has shape {:?}",
            coordinates.shape
        )));
    };
    let (pairs, _) = coordinates.data.as_chunks::<2>();
    let vertices = pairs.iter().map(|&[x, y]| Point::new(x, y)).collect();
    Grid::from_vec(rows, cols, vertices).map_err(|err| PyErr::value_error(err.to_string()))
}

/// A loose triangle mesh over the unmasked triangles of `triangulation`
/// (matplotlib's `TriMesh(triangulation)`).
fn tri_mesh(triangulation: Native<Triangulation>) -> PyResult<Collection> {
    Ok(Collection::loose(CollectionSpec::tri_mesh(
        &triangulation.borrow(),
    )))
}

/// Registers the constructors, as `new_<kind>`.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new_line_collection", line_collection)?;
    module.add("new_poly_collection", poly_collection)?;
    module.add("new_path_collection", path_collection)?;
    module.add("new_event_collection", event_collection)?;
    module.add("new_patch_collection", patch_collection)?;
    module.add("new_circle_collection", circle_collection)?;
    module.add("new_fill_between_collection", fill_between_collection)?;
    module.add("new_regular_poly_collection", regular_poly_collection)?;
    module.add("new_ellipse_collection", ellipse_collection)?;
    module.add("new_quad_mesh", quad_mesh)?;
    module.add("new_poly_quad_mesh", poly_quad_mesh)?;
    module.add("new_tri_mesh", tri_mesh)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poly_quad_mesh_takes_a_grid_of_vertices() {
        let coordinates: Vec<f64> = (0..12).map(f64::from).collect();
        let grid = NdArray::with_shape(coordinates.clone(), vec![2, 3, 2]).unwrap();
        assert_eq!(poly_quad_mesh(grid).unwrap().paths().len(), 2);
        let flat = NdArray::with_shape(coordinates, vec![6, 2]).unwrap();
        assert!(poly_quad_mesh(flat).is_err());
    }
}
