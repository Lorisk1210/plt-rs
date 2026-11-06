//! The 3-D geometry of artists a 3-D axes made (matplotlib's
//! `Line3D.get_data_3d`/`set_data_3d`, the offsets of
//! `Path3DCollection`, `Text3D.get_position_3d`), and the projection math
//! of matplotlib's `proj3d`.

use pltrs::artist::RawHandle;
use pltrs::collections::Collection;
use pltrs::lines::Line2D;
use pltrs::mplot3d::math::{
    Matrix4, ortho_transformation, persp_transformation, view_axes, view_transformation,
};
use pltrs::mplot3d::{Axes3D, Geometry3D, LightSource, Vec3, ZSort, all_points_on_plane};
use pltrs::text::Text;
use pltrs_core::{Point, Rgba};

use super::super::path::{PathArrays, PyPath, path_arrays};
use super::super::transforms::bbox_array;
use super::zdir_axis;
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

/// Lets the markers of a scatter plot fade with depth or not, down to the
/// opacity `min_alpha` (`Path3DCollection.set_depthshade`; Python gives
/// `axes3d.depthshade_minalpha` when none is given, as matplotlib does).
fn set_depthshade(
    axes: Axes3D,
    scatter: Collection,
    depthshade: bool,
    min_alpha: f64,
) -> PyResult<()> {
    axes.set_depthshade(&scatter, depthshade)?
        .set_depthshade_minalpha(&scatter, min_alpha)?;
    Ok(())
}

/// Takes the face (`faces`) and edge (`edges`) colours a polygon
/// collection of the axes has now as those of its polygons
/// (`Poly3DCollection.set_facecolor`, `set_edgecolor`, `set_alpha`).
fn refresh_poly_colors(
    axes: Axes3D,
    collection: Collection,
    faces: bool,
    edges: bool,
) -> PyResult<()> {
    axes.refresh_poly_colors(&collection, faces, edges)?;
    Ok(())
}

/// The face and edge colours the polygons of a polygon collection of the
/// axes are drawn with, as `(N, 4)` arrays (`Poly3DCollection._facecolor3d`
/// and `_edgecolor3d`).
fn poly_colors(axes: Axes3D, collection: Collection) -> PyResult<(NdArray, NdArray)> {
    let (faces, edges) = axes.poly_colors(&collection)?;
    let array = |colors: Vec<Rgba>| {
        let n = colors.len();
        NdArray::with_shape(
            colors.iter().flat_map(|c| [c.r, c.g, c.b, c.a]).collect(),
            vec![n, 4],
        )
    };
    Ok((array(faces)?, array(edges)?))
}

/// Whether a polygon collection of the axes was projected
/// (`Poly3DCollection` before its `_facecolors2d` exist).
fn poly_projected(axes: Axes3D, collection: Collection) -> PyResult<bool> {
    Ok(axes.poly_projected(&collection)?)
}

/// Turns the patch `patch` of the axes' projection plane into a flat 3-D
/// patch following `outline`, an `(N, 3)` array (`patch_2d_to_3d`).
fn patch_2d_to_3d(
    axes: Axes3D,
    patch: RawHandle,
    outline: NdArray,
    axlim_clip: bool,
) -> PyResult<()> {
    let outline = vertices(std::slice::from_ref(&outline))?.concat();
    axes.patch_2d_to_3d(&patch, outline, axlim_clip)?;
    Ok(())
}

/// Turns the patch `patch` of the axes' projection plane into a 3-D path
/// patch: its outline `path` in data coordinates, codes kept, with its
/// vertices at `vertices`, an `(N, 3)` array (`pathpatch_2d_to_3d`).
fn pathpatch_2d_to_3d(
    axes: Axes3D,
    patch: RawHandle,
    path: PyPath,
    vertices3d: NdArray,
) -> PyResult<()> {
    let vertices3d = vertices(std::slice::from_ref(&vertices3d))?.concat();
    axes.pathpatch_2d_to_3d(&patch, path.path, vertices3d)?;
    Ok(())
}

/// The path a 3-D patch of the axes is drawn with, projected in the
/// current layout (`Patch3D.get_path`).
fn patch_path(axes: Axes3D, patch: RawHandle) -> PyResult<PathArrays> {
    path_arrays(&axes.patch_path(&patch)?, false)
}

/// Includes the points `(x, y, z)` in the data limits (`z` `None`: the z
/// limits stay), replacing them unless `had_data`, and autoscales
/// (`auto_scale_xyz`).
fn auto_scale_xyz(
    axes: Axes3D,
    (x, y, z): (Vec<f64>, Vec<f64>, Option<Vec<f64>>),
    had_data: bool,
) -> PyResult<()> {
    axes.auto_scale_xyz(&x, &y, z.as_deref(), had_data)?;
    Ok(())
}

/// Projects a 3-D artist of the axes onto its 2-D artist through the
/// projection matrix of the last draw and returns its depth, `None`
/// before the first draw (`do_3d_projection`, which reads `M`).
fn do_3d_projection(axes: Axes3D, artist: RawHandle) -> PyResult<Option<f64>> {
    Ok(axes.do_3d_projection_as_drawn(&artist)?)
}

/// Whether data artists were added to the axes (`has_data`).
fn has_data(axes: Axes3D) -> PyResult<bool> {
    Ok(axes.has_data())
}

/// Turns the text `text` of the axes' projection plane into a 3-D text at
/// `position`, its baseline along `direction` (`text_2d_to_3d`,
/// `Text3D.set_3d_properties`).
fn text_2d_to_3d(
    axes: Axes3D,
    text: Text,
    (position, direction): (Triple, Triple),
    axlim_clip: bool,
) -> PyResult<()> {
    axes.text_2d_to_3d(&text, vec3(position), vec3(direction), axlim_clip)?;
    Ok(())
}

/// Moves a 3-D text of the axes to `position`, its baseline along
/// `direction` if given (`Text3D.set_position_3d`).
fn set_text_position(
    axes: Axes3D,
    text: Text,
    position: Triple,
    direction: Option<Triple>,
) -> PyResult<()> {
    axes.set_text_position(&text, vec3(position), direction.map(vec3))?;
    Ok(())
}

/// Turns the line `line` of the axes' projection plane into a 3-D line
/// through `(xs, ys, zs)` (`line_2d_to_3d`, `Line3D.set_3d_properties`).
fn line_2d_to_3d(
    axes: Axes3D,
    line: Line2D,
    coordinates: Columns,
    axlim_clip: bool,
) -> PyResult<()> {
    axes.line_2d_to_3d(&line, points(coordinates), axlim_clip)?;
    Ok(())
}

/// `colors` (RGBA rows: one, or one per face) shaded by the normals of
/// the `faces` (`(N, 3)` arrays) as the light at `(azdeg, altdeg)`
/// (`None`: mplot3d's) lights them (a shaded `Poly3DCollection`).
fn shade_faces(
    faces: Vec<NdArray>,
    colors: NdArray,
    light: Option<(f64, f64)>,
) -> PyResult<NdArray> {
    let faces = vertices(&faces)?;
    let colors: Vec<Rgba> = colors
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| Rgba::new(r, g, b, a))
        .collect();
    let light = light.map_or_else(LightSource::mplot3d, |(azdeg, altdeg)| {
        LightSource::new(azdeg, altdeg)
    });
    let shaded = pltrs::mplot3d::shade_faces(&faces, &colors, &light);
    let n = shaded.len();
    NdArray::with_shape(
        shaded.iter().flat_map(|c| [c.r, c.g, c.b, c.a]).collect(),
        vec![n, 4],
    )
}

/// Whether the 3-D artist `artist` of the axes hides its parts outside
/// the view limits; `None` when it is no 3-D artist of the axes
/// (`_axlim_clip`).
fn get_axlim_clip(axes: Axes3D, artist: RawHandle) -> PyResult<Option<bool>> {
    Ok(axes.axlim_clip(&artist))
}

/// The 3-D position of a text of the axes (`Text3D.get_position_3d`).
fn text_position(axes: Axes3D, text: Text) -> PyResult<(f64, f64, f64)> {
    let [x, y, z] = axes.text_position(&text)?;
    Ok((x, y, z))
}

/// The extent of a 3-D text of the axes where matplotlib measures it, at
/// the x and y of its 3-D position, turned by its own `rotation`
/// (`Text3D.get_window_extent`): `[[x0, y0], [x1, y1]]`, or `None`.
fn unprojected_text_extent(axes: Axes3D, text: Text, rotation: f64) -> PyResult<Option<NdArray>> {
    let [x, y, _] = axes.text_position(&text)?;
    (text.extent_placed(Point::new(x, y), rotation))
        .map(bbox_array)
        .transpose()
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
pub(super) fn matrix_array(matrix: &Matrix4) -> PyResult<NdArray> {
    NdArray::with_shape(
        matrix.rows().iter().flatten().copied().collect(),
        vec![4, 4],
    )
}

/// A 4 × 4 numpy array as a matrix.
pub(super) fn array_matrix(array: &NdArray) -> PyResult<Matrix4> {
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
/// divided by their `w` (`proj_transform`).
fn transform(matrix: NdArray, coordinates: Columns) -> PyResult<Columns> {
    let projected = array_matrix(&matrix)?.project_all(&points(coordinates));
    Ok(point_columns(&projected))
}

/// The points `(xs, ys, zs)` through the inverse of a projection
/// `matrix`, divided by their `w` unless it is zero (`inv_transform`).
fn inverse_transform(matrix: NdArray, coordinates: Columns) -> PyResult<Columns> {
    let unprojected = array_matrix(&matrix)?.unproject_all(&points(coordinates));
    Ok(point_columns(&unprojected))
}

/// The x, y and z columns of `points`.
fn point_columns(points: &[Vec3]) -> Columns {
    (
        points.iter().map(|p| p[0]).collect(),
        points.iter().map(|p| p[1]).collect(),
        points.iter().map(|p| p[2]).collect(),
    )
}

/// Whether the points `(xs, ys, zs)`, NaN ones ignored, lie on one plane
/// within `atol` (`_all_points_on_plane`).
fn on_plane(coordinates: Columns, atol: f64) -> PyResult<bool> {
    Ok(all_points_on_plane(points(coordinates), atol))
}

/// The vertices of each polygon or polyline: `(N, 3)` arrays.
pub(super) fn vertices(arrays: &[NdArray]) -> PyResult<Vec<Vec<Vec3>>> {
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
pub(super) fn zsort(name: &str) -> PyResult<ZSort> {
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

/// The 3-D geometry of a collection made outside any figure, as Python
/// describes it: `("polygons", (verts, zsort, closed))`, `("lines",
/// (verts,))` or `("patches", (zs, zdir, depthshade, min_alpha))`.
struct PyGeometry(Geometry3D);

impl FromPy<'_> for PyGeometry {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (kind, payload): (String, Bound<'_>) = object.extract()?;
        Ok(Self(match kind.as_str() {
            "polygons" => {
                let (verts, zsort_name, closed, from_2d): (Vec<NdArray>, String, bool, bool) =
                    payload.extract()?;
                Geometry3D::Polygons {
                    faces: vertices(&verts)?,
                    zsort: zsort(&zsort_name)?,
                    closed,
                    from_2d,
                }
            }
            "lines" => {
                let (verts,): (Vec<NdArray>,) = payload.extract()?;
                Geometry3D::Lines(vertices(&verts)?)
            }
            "paths" => {
                let (verts,): (Vec<NdArray>,) = payload.extract()?;
                Geometry3D::Paths(vertices(&verts)?)
            }
            "patches" => {
                let (zs, zdir, depthshade, min_alpha): (Vec<f64>, String, bool, f64) =
                    payload.extract()?;
                Geometry3D::Patches {
                    zs,
                    zdir: zdir_axis(&zdir)?,
                    depthshade,
                    min_alpha,
                }
            }
            _ => {
                return Err(PyErr::value_error(format!(
                    "no 3-D collection geometry named {kind:?}"
                )));
            }
        }))
    }
}

/// Adds the loose collection `collection` (a `Poly3DCollection`,
/// `Line3DCollection` or `Patch3DCollection` made with its properties)
/// with its 3-D `geometry` (`add_collection3d`).
fn adopt_collection3d(
    axes: Axes3D,
    collection: RawHandle,
    geometry: PyGeometry,
    autolim: bool,
    axlim_clip: bool,
) -> PyResult<Collection> {
    Ok(axes.adopt_collection3d(&collection, geometry.0, autolim, axlim_clip)?)
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("line_data", line_data)?;
    module.add("set_line_data", set_line_data)?;
    module.add("scatter_offsets", scatter_offsets)?;
    module.add("set_scatter_offsets", set_scatter_offsets)?;
    module.add("text_position", text_position)?;
    module.add("unprojected_text_extent", unprojected_text_extent)?;
    module.add("text_2d_to_3d", text_2d_to_3d)?;
    module.add("line_2d_to_3d", line_2d_to_3d)?;
    module.add("shade_faces", shade_faces)?;
    module.add("get_axlim_clip", get_axlim_clip)?;
    module.add("set_text_position", set_text_position)?;
    module.add("get_depthshade", get_depthshade)?;
    module.add("set_depthshade", set_depthshade)?;
    module.add("refresh_poly_colors", refresh_poly_colors)?;
    module.add("poly_colors", poly_colors)?;
    module.add("poly_projected", poly_projected)?;
    module.add("patch_2d_to_3d", patch_2d_to_3d)?;
    module.add("pathpatch_2d_to_3d", pathpatch_2d_to_3d)?;
    module.add("patch_path", patch_path)?;
    module.add("auto_scale_xyz", auto_scale_xyz)?;
    module.add("has_data", has_data)?;
    module.add("do_3d_projection", do_3d_projection)?;
    module.add("view_axes", view_frame)?;
    module.add("view_transformation", view_matrix)?;
    module.add("persp_transformation", persp)?;
    module.add("ortho_transformation", ortho)?;
    module.add("transform", transform)?;
    module.add("inverse_transform", inverse_transform)?;
    module.add("all_points_on_plane", on_plane)?;
    module.add("adopt_collection3d", adopt_collection3d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::mplot3d::math::world_transformation;

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
