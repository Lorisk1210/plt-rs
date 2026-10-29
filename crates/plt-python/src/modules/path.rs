//! `_native.path`: the geometry of matplotlib's paths (its C++ `_path`
//! module, and the shapes `Path` makes), done by plt-core's `Path`.
//!
//! A path arrives as the Python `Path` object itself: its `vertices`,
//! `codes`, `should_simplify` and `simplify_threshold` are read from it. A
//! transform arrives as its 3×3 matrix, or `None` for the identity. A path
//! goes back as `(vertices, codes)` arrays, `codes` `None` for a polyline.

use plt_core::hatch::pattern_path;
use plt_core::path::convert::{CleanupOptions, SimplifySettings, SketchParams, SnapMode};
use plt_core::path::convert::{StringCodes, StringOptions};
use plt_core::path::{CollectionError, CollectionGeometry, PolygonOptions};
use plt_core::{Affine2D, Bbox, Path, PathCode, PathError, Point};

use super::transforms::{affine, bbox_array, points, points_array};
use crate::binding::dict_value;
use crate::binding::prelude::*;

impl From<PathError> for PyErr {
    /// matplotlib's `ValueError`s for malformed paths.
    fn from(err: PathError) -> Self {
        match err {
            PathError::LengthMismatch { vertices, codes } => PyErr::value_error(format!(
                "'codes' must be a 1D list or array with the same length of 'vertices'. Your \
                 vertices have shape ({vertices}, 2) but your codes have shape ({codes},)"
            )),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

impl From<CollectionError> for PyErr {
    /// `ValueError` for collections without paths or offsets.
    fn from(err: CollectionError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// A path for Python: its `(N, 2)` vertices and its codes (`None` for a
/// polyline).
pub(super) type PathArrays = (NdArray, Option<NdArray<u8>>);

/// matplotlib's `STOP` code, which ends the paths `cleanup_path` returns.
const STOP: u8 = 0;

/// A matplotlib `Path` read from Python, with its simplification settings.
pub(super) struct PyPath {
    pub(super) path: Path,
    simplify: SimplifySettings,
}

impl FromPy<'_> for PyPath {
    /// A `Path` object: anything with `vertices`, `codes`,
    /// `should_simplify` and `simplify_threshold`.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let vertices: NdArray = object.getattr("vertices")?.extract()?;
        let codes: Option<NdArray<u8>> = object.getattr("codes")?.extract()?;
        let simplify = SimplifySettings {
            enabled: object.getattr("should_simplify")?.extract()?,
            threshold: object.getattr("simplify_threshold")?.extract()?,
        };
        Ok(Self {
            path: path(&vertices, codes.as_ref())?,
            simplify,
        })
    }
}

/// The path of `vertices` (`(N, 2)`) and `codes` (`None` for a polyline);
/// `STOP` codes end it, as in matplotlib's iteration.
fn path(vertices: &NdArray, codes: Option<&NdArray<u8>>) -> PyResult<Path> {
    let mut vertices = points(vertices)?;
    let Some(codes) = codes else {
        return Ok(Path::polyline(vertices));
    };
    let mut codes = &codes.data[..];
    if let Some(end) = codes.iter().position(|&code| code == STOP) {
        codes = &codes[..end];
        vertices.truncate(end);
    }
    let codes = codes
        .iter()
        .map(|&code| PathCode::try_from(code))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Path::new(vertices, codes)?)
}

/// A path as `(vertices, codes)`; `stop` ends it with matplotlib's `STOP`
/// vertex.
pub(super) fn path_arrays(path: &Path, stop: bool) -> PyResult<PathArrays> {
    let mut vertices = path.vertices().to_vec();
    let codes = if stop || path.codes().is_some() {
        let mut codes: Vec<u8> = path.resolved_codes().iter().map(|&c| c as u8).collect();
        if stop {
            vertices.push(Point::ORIGIN);
            codes.push(STOP);
        }
        Some(NdArray::from_vec(codes))
    } else {
        None
    };
    let shape = vec![vertices.len(), 2];
    Ok((points_array(&vertices, shape)?, codes))
}

/// The affine map of an optional 3×3 matrix (the identity for `None`).
fn transform(matrix: Option<&NdArray>) -> PyResult<Option<Affine2D>> {
    matrix.map(affine).transpose()
}

/// Polygons as a list of `(N, 2)` arrays.
fn polygon_arrays(polygons: Vec<Vec<Point>>) -> PyResult<Vec<NdArray>> {
    polygons
        .into_iter()
        .map(|polygon| {
            let shape = vec![polygon.len(), 2];
            points_array(&polygon, shape)
        })
        .collect()
}

/// matplotlib's tri-state `snap`: `True`, `False` or `None` (automatic).
fn snap_mode(snap: Option<bool>) -> SnapMode {
    match snap {
        None => SnapMode::Auto,
        Some(true) => SnapMode::Always,
        Some(false) => SnapMode::Never,
    }
}

/// The sketch `(scale, length, randomness)`, if any.
fn sketch(params: Option<(f64, f64, f64)>) -> Option<SketchParams> {
    params
        .map(|(scale, length, randomness)| SketchParams::new(scale, Some(length), Some(randomness)))
}

/// A clipping rectangle `(x0, y0, x1, y1)`, if any.
fn clip_rect(clip: Option<(f64, f64, f64, f64)>) -> Option<Bbox> {
    clip.map(|(x0, y0, x1, y1)| Bbox::from_extents(x0, y0, x1, y1))
}

/// `cleanup_path(path, trans, options)`: the path transformed, cleaned of
/// NaNs, clipped, snapped, simplified, with curves flattened unless kept,
/// ending with a `STOP` vertex. `options` holds `remove_nans`, `clip`,
/// `snap`, `stroke_width`, `simplify` (`None`: the path's
/// `should_simplify`), `curves` and `sketch`.
fn cleanup_path(py_path: PyPath, trans: Option<NdArray>, options: Owned) -> PyResult<PathArrays> {
    let options = Python::with_gil(|py| -> PyResult<CleanupOptions> {
        let options = options.bind(py);
        let simplify: Option<bool> = dict_value(&options, "simplify")?;
        let simplify = match simplify {
            None => py_path.simplify.for_path(&py_path.path),
            Some(true) => Some(py_path.simplify.threshold),
            Some(false) => None,
        };
        Ok(CleanupOptions {
            transform: transform(trans.as_ref())?.unwrap_or_default(),
            remove_nans: dict_value(&options, "remove_nans")?,
            clip: clip_rect(dict_value(&options, "clip")?),
            snap: snap_mode(dict_value(&options, "snap")?),
            stroke_width: dict_value(&options, "stroke_width")?,
            simplify,
            curves: dict_value(&options, "curves")?,
            sketch: sketch(dict_value(&options, "sketch")?),
        })
    })?;
    path_arrays(&py_path.path.cleaned(&options), true)
}

/// `point_in_path(x, y, r, path, trans)`.
fn point_in_path(
    x: f64,
    y: f64,
    radius: f64,
    py_path: PyPath,
    trans: Option<NdArray>,
) -> PyResult<bool> {
    let transform = transform(trans.as_ref())?;
    Ok(py_path
        .path
        .contains_point(Point::new(x, y), transform.as_ref(), radius))
}

/// `points_in_path(points, r, path, trans)`: one bool per point.
fn points_in_path(
    query: NdArray,
    radius: f64,
    py_path: PyPath,
    trans: Option<NdArray>,
) -> PyResult<NdArray<bool>> {
    let transform = transform(trans.as_ref())?;
    let inside = py_path
        .path
        .contains_points(&points(&query)?, transform.as_ref(), radius);
    Ok(NdArray::from_vec(inside))
}

/// `path_in_path(a, atrans, b, btrans)`: whether `a` contains `b`.
fn path_in_path(
    a: PyPath,
    atrans: Option<NdArray>,
    b: PyPath,
    btrans: Option<NdArray>,
) -> PyResult<bool> {
    let container = match transform(atrans.as_ref())? {
        Some(t) => a.path.transformed(&t),
        None => a.path,
    };
    let transform = transform(btrans.as_ref())?;
    Ok(container.contains_path(&b.path, transform.as_ref()))
}

/// `path_intersects_path(p1, p2, filled)`.
fn path_intersects_path(p1: PyPath, p2: PyPath, filled: bool) -> PyResult<bool> {
    Ok(p1.path.intersects_path(&p2.path, filled))
}

/// `path_intersects_rectangle(path, x0, y0, x1, y1, filled)`.
fn path_intersects_rectangle(
    py_path: PyPath,
    (x0, y0, x1, y1): (f64, f64, f64, f64),
    filled: bool,
) -> PyResult<bool> {
    let rect = Bbox::from_extents(x0, y0, x1, y1);
    Ok(py_path.path.intersects_bbox(&rect, filled))
}

/// `convert_path_to_polygons(path, trans, width, height, closed_only)`.
fn convert_path_to_polygons(
    py_path: PyPath,
    trans: Option<NdArray>,
    width: f64,
    height: f64,
    closed_only: bool,
) -> PyResult<Vec<NdArray>> {
    let options = PolygonOptions {
        transform: transform(trans.as_ref())?,
        width,
        height,
        closed_only,
        simplify: py_path.simplify,
    };
    polygon_arrays(py_path.path.to_polygons(&options))
}

/// `clip_path_to_rect(path, bbox, inside)`: the closed polygons of the
/// path clipped to the box.
fn clip_path_to_rect(py_path: PyPath, rect: NdArray, inside: bool) -> PyResult<Vec<NdArray>> {
    let rect = super::transforms::bbox(&rect)?;
    polygon_arrays(py_path.path.clip_polygons(&rect, inside))
}

/// The paths, per-item matrices, offsets and offset matrix of a
/// collection, read for [`CollectionGeometry`].
struct Collection {
    master: Affine2D,
    paths: Vec<Path>,
    transforms: Vec<Affine2D>,
    offsets: Vec<Point>,
    offset_transform: Affine2D,
}

impl Collection {
    fn new(
        master: &NdArray,
        paths: Vec<PyPath>,
        transforms: &[NdArray],
        offsets: &NdArray,
        offset_transform: &NdArray,
    ) -> PyResult<Self> {
        Ok(Self {
            master: affine(master)?,
            paths: paths.into_iter().map(|p| p.path).collect(),
            transforms: transforms.iter().map(affine).collect::<PyResult<_>>()?,
            offsets: points(offsets)?,
            offset_transform: affine(offset_transform)?,
        })
    }

    fn geometry(&self) -> CollectionGeometry<'_> {
        CollectionGeometry {
            master_transform: self.master,
            paths: &self.paths,
            transforms: &self.transforms,
            offsets: &self.offsets,
            offset_transform: self.offset_transform,
        }
    }
}

/// `get_path_collection_extents(master_transform, paths, transforms,
/// offsets, offset_transform)`: the extents and `minpos`.
fn get_path_collection_extents(
    master: NdArray,
    paths: Vec<PyPath>,
    transforms: Vec<NdArray>,
    offsets: NdArray,
    offset_transform: NdArray,
) -> PyResult<(NdArray, (f64, f64))> {
    let collection = Collection::new(&master, paths, &transforms, &offsets, &offset_transform)?;
    let limits = collection.geometry().extents()?;
    Ok((bbox_array(limits.bbox)?, (limits.minpos.x, limits.minpos.y)))
}

/// `point_in_path_collection(x, y, radius, master_transform, paths,
/// transforms, offsets, offset_transform, filled)`, with `(x, y, radius)`
/// and `(offsets, offset_transform)` paired: the indices of the items hit.
fn point_in_path_collection(
    (x, y, radius): (f64, f64, f64),
    master: NdArray,
    paths: Vec<PyPath>,
    transforms: Vec<NdArray>,
    (offsets, offset_transform): (NdArray, NdArray),
    filled: bool,
) -> PyResult<NdArray<i64>> {
    let collection = Collection::new(&master, paths, &transforms, &offsets, &offset_transform)?;
    let hits = collection
        .geometry()
        .pick(Point::new(x, y), radius, filled)
        .into_iter()
        .map(|i| i64::try_from(i).unwrap_or(i64::MAX))
        .collect();
    Ok(NdArray::from_vec(hits))
}

/// `convert_to_string(path, trans, clip, simplify, sketch, precision,
/// codes, postfix)`: the path as text, one command a line.
#[expect(
    clippy::too_many_arguments,
    reason = "the positional signature of matplotlib's `_path.convert_to_string`"
)]
fn convert_to_string(
    py_path: PyPath,
    trans: Option<NdArray>,
    clip: Option<(f64, f64, f64, f64)>,
    simplify: Option<bool>,
    sketch_params: Option<(f64, f64, f64)>,
    precision: usize,
    codes: Vec<String>,
    postfix: bool,
) -> PyResult<Bytes> {
    let [move_to, line_to, quad_to, cubic_to, close] = &codes[..] else {
        return Err(PyErr::value_error(
            "convert_to_string needs five command tokens",
        ));
    };
    let simplify = match simplify {
        None => py_path.simplify.for_path(&py_path.path),
        Some(true) => Some(py_path.simplify.threshold),
        Some(false) => None,
    };
    let options = StringOptions {
        transform: transform(trans.as_ref())?.unwrap_or_default(),
        clip: clip_rect(clip),
        simplify,
        sketch: sketch(sketch_params),
        precision,
        codes: StringCodes {
            move_to,
            line_to,
            quad_to,
            cubic_to,
            close,
        },
        postfix,
    };
    Ok(Bytes(py_path.path.convert_to_string(&options).into_bytes()))
}

/// `is_sorted_and_has_non_nan(array)`: whether the finite values are in
/// increasing order and there is at least one.
fn is_sorted_and_has_non_nan(values: NdArray) -> PyResult<bool> {
    let mut finite = values.data.iter().filter(|v| !v.is_nan());
    let Some(&first) = finite.next() else {
        return Ok(false);
    };
    let mut last = first;
    for &value in finite {
        if value < last {
            return Ok(false);
        }
        last = value;
    }
    Ok(true)
}

/// `Path.arc(theta1, theta2, n, is_wedge)` as `(vertices, codes)`.
fn arc(theta1: f64, theta2: f64, n: Option<usize>, is_wedge: bool) -> PyResult<PathArrays> {
    let shape = if is_wedge {
        Path::wedge(theta1, theta2, n)
    } else {
        Path::arc(theta1, theta2, n)
    };
    path_arrays(&shape, false)
}

/// The unit shapes of `Path`: `"rectangle"`, `"circle"`,
/// `"circle_righthalf"`, or a regular `"polygon"`, `"star"` or
/// `"asterisk"` of `vertices` points (a star's inner radius
/// `inner_radius`).
fn unit_shape(name: String, vertices: usize, inner_radius: f64) -> PyResult<PathArrays> {
    let shape = match name.as_str() {
        "rectangle" => Path::unit_rectangle(),
        "circle" => Path::unit_circle(),
        "circle_righthalf" => Path::unit_circle_righthalf(),
        "polygon" => Path::unit_regular_polygon(vertices),
        "star" => Path::unit_regular_star(vertices, inner_radius),
        "asterisk" => Path::unit_regular_asterisk(vertices),
        _ => return Err(PyErr::value_error(format!("no unit shape named {name:?}"))),
    };
    path_arrays(&shape, false)
}

/// `Path.circle(center, radius)` as `(vertices, codes)`.
fn circle(center: (f64, f64), radius: f64) -> PyResult<PathArrays> {
    path_arrays(&Path::circle(Point::new(center.0, center.1), radius), false)
}

/// The hatch path of `pattern` in the unit square, with `density` lines
/// a unit (`hatch.get_path`), as `(vertices, codes)`.
fn hatch(pattern: String, density: i32) -> PyResult<PathArrays> {
    let path =
        pattern_path(&pattern, density).map_err(|err| PyErr::value_error(err.to_string()))?;
    path_arrays(&path, false)
}

/// `Path.interpolated(steps)` of a path, as `(vertices, codes)`.
fn interpolated(py_path: PyPath, steps: usize) -> PyResult<PathArrays> {
    path_arrays(&py_path.path.interpolated(steps), false)
}

/// The exact extents of the path transformed by `trans`, curve extrema
/// included (`Path.get_extents`), as a box's 2×2 points.
fn get_extents(py_path: PyPath, trans: Option<NdArray>) -> PyResult<NdArray> {
    let path = match transform(trans.as_ref())? {
        Some(t) => py_path.path.transformed(&t),
        None => py_path.path,
    };
    bbox_array(path.extents())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("cleanup_path", cleanup_path)?;
    module.add("point_in_path", point_in_path)?;
    module.add("points_in_path", points_in_path)?;
    module.add("path_in_path", path_in_path)?;
    module.add("path_intersects_path", path_intersects_path)?;
    module.add("path_intersects_rectangle", path_intersects_rectangle)?;
    module.add("convert_path_to_polygons", convert_path_to_polygons)?;
    module.add("clip_path_to_rect", clip_path_to_rect)?;
    module.add("get_path_collection_extents", get_path_collection_extents)?;
    module.add("point_in_path_collection", point_in_path_collection)?;
    module.add("convert_to_string", convert_to_string)?;
    module.add("is_sorted_and_has_non_nan", is_sorted_and_has_non_nan)?;
    module.add("arc", arc)?;
    module.add("unit_shape", unit_shape)?;
    module.add("circle", circle)?;
    module.add("hatch", hatch)?;
    module.add("interpolated", interpolated)?;
    module.add("get_extents", get_extents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertices(data: &[f64]) -> NdArray {
        NdArray::with_shape(data.to_vec(), vec![data.len() / 2, 2]).unwrap()
    }

    #[test]
    fn a_stop_code_ends_the_path() {
        let codes = NdArray::from_vec(vec![1, 2, STOP, 2]);
        let stopped = path(
            &vertices(&[0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 5.0, 5.0]),
            Some(&codes),
        )
        .unwrap();
        assert_eq!(stopped.len(), 2);
        assert!(path(&vertices(&[0.0, 0.0]), Some(&NdArray::from_vec(vec![2]))).is_err());
        assert!(path(&vertices(&[0.0, 0.0]), Some(&NdArray::from_vec(vec![1, 2]))).is_err());
    }

    #[test]
    fn cleaned_paths_end_with_stop_and_polylines_have_no_codes() {
        let polyline = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
        let (_, codes) = path_arrays(&polyline, false).unwrap();
        assert!(codes.is_none());
        let (vertices, codes) = path_arrays(&polyline, true).unwrap();
        assert_eq!(vertices.shape, [3, 2]);
        assert_eq!(codes.unwrap().data, [1, 2, STOP]);
    }

    #[test]
    fn sortedness_ignores_nan_and_needs_a_value() {
        let sorted =
            |data: &[f64]| is_sorted_and_has_non_nan(NdArray::from_vec(data.to_vec())).unwrap();
        assert!(sorted(&[1.0, f64::NAN, 2.0, 2.0]));
        assert!(!sorted(&[2.0, 1.0]));
        assert!(!sorted(&[f64::NAN]));
        assert!(!sorted(&[]));
    }
}
