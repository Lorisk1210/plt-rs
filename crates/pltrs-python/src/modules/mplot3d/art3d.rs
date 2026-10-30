//! The 3-D geometry of artists a 3-D axes made (matplotlib's
//! `Line3D.get_data_3d`/`set_data_3d`, the offsets of
//! `Path3DCollection`, `Text3D.get_position_3d`), and the projection math
//! of matplotlib's `proj3d`.

use pltrs::artist::RawHandle;
use pltrs::collections::Collection;
use pltrs::lines::Line2D;
use pltrs::mplot3d::math::{
    Matrix4, ortho_transformation, persp_transformation, view_axes, view_transformation,
    world_transformation,
};
use pltrs::mplot3d::{Axes3D, Geometry3D, Vec3, ZSort, all_points_on_plane};
use pltrs::text::Text;

use crate::binding::prelude::*;

/// Three coordinate columns.
type Columns = (Vec<f64>, Vec<f64>, Vec<f64>);

/// A 3-vector as Python passes it.
type Triple = (f64, f64, f64);

/// `[x, y, z]` columns as a tuple.
fn columns([x, y, z]: [Vec<f64>; 3]) -> Columns {
    (x, y, z)
}

/// The 3-D points of a line of the axes (`Line3D.get_data_3d`).
fn line_data(axes: Axes3D, line: Line2D) -> PyResult<Columns> {
    Ok(columns(axes.line_data(&line)?))
}

/// Replaces the 3-D points of a line of the axes (`set_data_3d`).
fn set_line_data(axes: Axes3D, line: Line2D, (x, y, z): Columns) -> PyResult<()> {
    axes.set_line_data(&line, x, y, z)?;
    Ok(())
}

/// The 3-D positions of the markers of a scatter plot of the axes
/// (`Path3DCollection._offsets3d`).
fn scatter_offsets(axes: Axes3D, scatter: Collection) -> PyResult<Columns> {
    Ok(columns(axes.scatter_offsets(&scatter)?))
}

/// Replaces the 3-D positions of the markers of a scatter plot.
fn set_scatter_offsets(axes: Axes3D, scatter: Collection, (x, y, z): Columns) -> PyResult<()> {
    axes.set_scatter_offsets(&scatter, x, y, z)?;
    Ok(())
}

/// Whether the markers of a scatter plot fade with depth
/// (`Path3DCollection.get_depthshade`).
fn get_depthshade(axes: Axes3D, scatter: Collection) -> PyResult<bool> {
    Ok(axes.depthshade(&scatter)?)
}

/// Lets the markers of a scatter plot fade with depth or not
/// (`Path3DCollection.set_depthshade`).
fn set_depthshade(axes: Axes3D, scatter: Collection, depthshade: bool) -> PyResult<()> {
    axes.set_depthshade(&scatter, depthshade)?;
    Ok(())
}

/// The 3-D position of a text of the axes (`Text3D.get_position_3d`).
fn text_position(axes: Axes3D, text: Text) -> PyResult<(f64, f64, f64)> {
    let [x, y, z] = axes.text_position(&text)?;
    Ok((x, y, z))
}

/// The points of three coordinate columns.
fn points((xs, ys, zs): Columns) -> Vec<Vec3> {
    xs.iter()
        .zip(&ys)
        .zip(&zs)
        .map(|((&x, &y), &z)| [x, y, z])
        .collect()
}

/// A 4 × 4 matrix as a numpy array.
fn matrix_array(matrix: &Matrix4) -> PyResult<NdArray> {
    NdArray::with_shape(
        matrix.rows().iter().flatten().copied().collect(),
        vec![4, 4],
    )
}

/// A 4 × 4 numpy array as a matrix.
fn array_matrix(array: &NdArray) -> PyResult<Matrix4> {
    if array.shape[..] != [4, 4] {
        return Err(PyErr::value_error("the projection matrix must be 4 x 4"));
    }
    let row = |i: usize| [0, 1, 2, 3].map(|j| array.data[4 * i + j]);
    Ok(Matrix4::from_rows([row(0), row(1), row(2), row(3)]))
}

/// A 3-vector of a sequence of three numbers.
fn vec3((x, y, z): (f64, f64, f64)) -> Vec3 {
    [x, y, z]
}

/// The matrix mapping the box `limits` (`xmin, xmax, ymin, ymax, zmin,
/// zmax`) onto `[0, aspect]` along each axis (`world_transformation`).
fn world(limits: (f64, f64, f64, f64, f64, f64), aspect: (f64, f64, f64)) -> PyResult<NdArray> {
    let (x0, x1, y0, y1, z0, z1) = limits;
    matrix_array(&world_transformation(
        [x0, x1, y0, y1, z0, z1],
        vec3(aspect),
    ))
}

/// The unit vectors right, up and out of the screen for an eye at `eye`
/// looking at `center` with `vertical` up, rolled by `roll` radians
/// (`_view_axes`).
fn view_frame(
    eye: (f64, f64, f64),
    center: (f64, f64, f64),
    vertical: (f64, f64, f64),
    roll: f64,
) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let [u, v, w] = view_axes(vec3(eye), vec3(center), vec3(vertical), roll);
    Ok((u.to_vec(), v.to_vec(), w.to_vec()))
}

/// The world-to-view matrix for the view axes and the eye position
/// (`_view_transformation_uvw`).
fn view_matrix((u, v, w): (Triple, Triple, Triple), eye: Triple) -> PyResult<NdArray> {
    matrix_array(&view_transformation(vec3(u), vec3(v), vec3(w), vec3(eye)))
}

/// The perspective projection between `zfront` and `zback`
/// (`_persp_transformation`).
fn persp(zfront: f64, zback: f64, focal_length: f64) -> PyResult<NdArray> {
    matrix_array(&persp_transformation(zfront, zback, focal_length))
}

/// The orthographic projection between `zfront` and `zback`
/// (`_ortho_transformation`).
fn ortho(zfront: f64, zback: f64) -> PyResult<NdArray> {
    matrix_array(&ortho_transformation(zfront, zback))
}

/// The points `(xs, ys, zs)` through the homogeneous matrix `matrix`,
/// divided by their `w` (`proj_transform`, `inv_transform`).
fn transform(matrix: NdArray, coordinates: Columns) -> PyResult<Columns> {
    let projected = array_matrix(&matrix)?.project_all(&points(coordinates));
    Ok((
        projected.iter().map(|p| p[0]).collect(),
        projected.iter().map(|p| p[1]).collect(),
        projected.iter().map(|p| p[2]).collect(),
    ))
}

/// Whether the points `(xs, ys, zs)`, NaN ones ignored, lie on one plane
/// within `atol` (`_all_points_on_plane`).
fn on_plane(coordinates: Columns, atol: f64) -> PyResult<bool> {
    Ok(all_points_on_plane(points(coordinates), atol))
}

/// The vertices of each polygon or polyline: `(N, 3)` arrays.
fn vertices(arrays: &[NdArray]) -> PyResult<Vec<Vec<Vec3>>> {
    arrays
        .iter()
        .map(|array| match array.shape[..] {
            [_, 3] => Ok(array.data.as_chunks::<3>().0.to_vec()),
            _ => Err(PyErr::value_error(
                "verts must be a list of (N, 3) array-like",
            )),
        })
        .collect()
}

/// How a polygon's depth is computed from its vertices (`zsort`:
/// `"average"`, `"min"` or `"max"`).
fn zsort(name: &str) -> PyResult<ZSort> {
    match name {
        "average" => Ok(ZSort::Average),
        "min" => Ok(ZSort::Min),
        "max" => Ok(ZSort::Max),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for zsort; supported values are 'average', 'min', \
             'max'"
        ))),
    }
}

/// Adds the loose collection `collection` (a `Poly3DCollection` or
/// `Line3DCollection` made with its properties) through the 3-D
/// `verts`: polygons ordered by the depth `zsort` picks, closed or not,
/// or (`zsort` `None`) polylines (`add_collection3d`).
fn adopt_collection3d(
    axes: Axes3D,
    collection: RawHandle,
    verts: Vec<NdArray>,
    (zsort_name, closed): (Option<String>, bool),
    autolim: bool,
    axlim_clip: bool,
) -> PyResult<Collection> {
    let verts = vertices(&verts)?;
    let geometry = match zsort_name {
        Some(name) => Geometry3D::Polygons {
            faces: verts,
            zsort: zsort(&name)?,
            closed,
        },
        None => Geometry3D::Lines(verts),
    };
    Ok(axes.adopt_collection3d(&collection, geometry, autolim, axlim_clip)?)
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("line_data", line_data)?;
    module.add("set_line_data", set_line_data)?;
    module.add("scatter_offsets", scatter_offsets)?;
    module.add("set_scatter_offsets", set_scatter_offsets)?;
    module.add("text_position", text_position)?;
    module.add("get_depthshade", get_depthshade)?;
    module.add("set_depthshade", set_depthshade)?;
    module.add("world_transformation", world)?;
    module.add("view_axes", view_frame)?;
    module.add("view_transformation", view_matrix)?;
    module.add("persp_transformation", persp)?;
    module.add("ortho_transformation", ortho)?;
    module.add("transform", transform)?;
    module.add("all_points_on_plane", on_plane)?;
    module.add("adopt_collection3d", adopt_collection3d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_world_maps_the_box_onto_the_aspect() {
        let world = world_transformation([100.0, 120.0, -100.0, 100.0, 0.1, 0.2], [1.0; 3]);
        let (x, y, z) = transform(
            matrix_array(&world).unwrap(),
            (vec![120.0], vec![-100.0], vec![0.15]),
        )
        .unwrap();
        assert!((x[0] - 1.0).abs() < 1e-12 && y[0].abs() < 1e-12);
        assert!((z[0] - 0.5).abs() < 1e-9);
        assert!(array_matrix(&NdArray::from_vec(vec![1.0; 3])).is_err());
    }
}
