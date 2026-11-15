//! Live transforms: `ax.transData`, `ax.transAxes`, `fig.transFigure`,
//! `fig.dpi_scale_trans` and the transforms artists are drawn with. They
//! cross as pltrs (Rust) [`Transform`] values, which a figure resolves in
//! its current layout whenever Python maps points through one.
//!
//! Python describes a transform as nested tuples, the first item naming
//! the kind: `("display",)`, `("inches",)`, `("affine", matrix)`,
//! `("data", axes)`, `("axes", axes)`, `("figure", figure)`,
//! `("blended", x, y)`, `("composite", first, then)`,
//! `("translation", dx, dy, units)`, `("inverted", t)`,
//! `("affine_delta", t)`, `("native", transform)` for a value made
//! before, or `("python", transform)` for a non-affine Python transform
//! of another kind (a `Transform` subclass of the program's), which
//! becomes a map calling it. [`describe`] gives the same tuples back, with
//! the artist ids of the figure (`_native.artist.id`) in place of the
//! handles.

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs::transforms::{Point, PointMap, ResolvedTransform, Transform, TransformError};
use pltrs_core::Path;

use super::super::path::{PyPath, path_arrays};
use super::{affine, map_points, matrix, points, points_array};
use crate::binding::prelude::*;
use crate::binding::registered_error;

native_value!(Transform);

impl From<TransformError> for PyErr {
    /// numpy's `LinAlgError` for a singular transform, as matplotlib's
    /// inversion raises; `ValueError` otherwise.
    fn from(err: TransformError) -> Self {
        match err {
            TransformError::Singular => registered_error("LinAlgError", "Singular matrix"),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// A transform described by Python's nested tuples (see the module
/// documentation).
struct Description(Transform);

impl FromPy<'_> for Description {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let items: Vec<Bound<'_>> = object.extract()?;
        let item = |index: usize| {
            items.get(index).ok_or_else(|| {
                PyErr::value_error(format!("a transform description lacks item {index}"))
            })
        };
        let child =
            |index: usize| -> PyResult<Transform> { Ok(item(index)?.extract::<Description>()?.0) };
        let kind: String = item(0)?.extract()?;
        let transform = match kind.as_str() {
            "native" => item(1)?.extract::<Transform>()?,
            "display" => Transform::Display,
            "inches" => Transform::Inches,
            "affine" => Transform::Affine(affine(&item(1)?.extract()?)?),
            "data" => Transform::Data(item(1)?.extract::<Axes>()?.id()),
            "axes" => Transform::Axes(item(1)?.extract::<Axes>()?.id()),
            "figure" => Transform::Figure(item(1)?.extract::<Figure>()?.id()),
            "figure_box" => Transform::FigureBox(item(1)?.extract::<Figure>()?.id()),
            "blended" => Transform::blended(child(1)?, child(2)?),
            "composite" => child(1)?.then(child(2)?),
            "translation" => Transform::Translation {
                dx: item(1)?.extract()?,
                dy: item(2)?.extract()?,
                units: Box::new(child(3)?),
            },
            "inverted" => child(1)?.inverted(),
            "affine_delta" => Transform::AffineDelta(Box::new(child(1)?)),
            "python" => Transform::custom(PyPointMap::new(item(1)?)?),
            other => {
                return Err(PyErr::value_error(format!(
                    "unknown kind of transform {other:?}"
                )));
            }
        };
        Ok(Self(transform))
    }
}

/// A non-affine Python transform as a map of the plane (the
/// `_ProgramMap` of `_live_transforms` around it): its `transform`
/// forward, the `transform` of its inverse back, and its `transform_path`,
/// if any, for whole paths, which matplotlib's caching keeps until the
/// transform is invalidated.
#[derive(Debug)]
pub(crate) struct PyPointMap {
    /// The Python map (what an archive holds in place of the map).
    object: Owned,
    forward: Callback,
    inverse: Option<Callback>,
    path: Option<Callback>,
    /// The Python map's `transform_outline`, if any: a patch's outline
    /// mapped anew (matplotlib's `transform_path_non_affine`).
    outline: Option<Callback>,
    /// The Python map's `transform_members`, if any: a collection's paths
    /// mapped anew, all in one call.
    members: Option<Callback>,
    /// The address of the Python map when it answers alike for as long
    /// as it lives (its `_answers_alike`), which names its code (see
    /// [`PointMap::program_key`]).
    key: Option<usize>,
}

impl PyPointMap {
    /// The map `map`; one without an inverse maps back to NaN.
    pub(crate) fn new(map: &Bound<'_>) -> PyResult<Self> {
        let forward = map.getattr("transform")?.extract()?;
        let inverse = map
            .call_method("inverted", &[])
            .and_then(|inverted| inverted.getattr("transform")?.extract())
            .ok();
        let alike = map
            .getattr("_answers_alike")
            .and_then(|alike| alike.extract::<bool>())
            .unwrap_or(false);
        Ok(Self {
            object: map.clone().unbind(),
            forward,
            inverse,
            path: map
                .getattr("transform_path")
                .and_then(|method| method.extract())
                .ok(),
            outline: map
                .getattr("transform_outline")
                .and_then(|method| method.extract())
                .ok(),
            members: map
                .getattr("transform_members")
                .and_then(|method| method.extract())
                .ok(),
            key: alike.then(|| map.as_ptr().addr()),
        })
    }

    /// The Python transform.
    pub(crate) fn object(&self) -> &Owned {
        &self.object
    }

    /// `p` through `callback`; NaN when it fails (the exception is raised
    /// when the native call running returns).
    fn apply(callback: Option<&Callback>, p: Point) -> Point {
        callback
            .and_then(|callback| callback.call::<(f64, f64)>(((p.x, p.y),)))
            .map_or(Point::new(f64::NAN, f64::NAN), |(x, y)| Point::new(x, y))
    }
}

impl PointMap for PyPointMap {
    fn forward(&self, p: Point) -> Point {
        Self::apply(Some(&self.forward), p)
    }

    fn calls_program(&self) -> bool {
        true
    }

    fn program_key(&self) -> Option<usize> {
        self.key
    }

    fn inverse(&self, p: Point) -> Point {
        Self::apply(self.inverse.as_ref(), p)
    }

    /// The points through the Python map's `transform`, called once with
    /// all of them.
    fn forward_all(&self, points: &mut [Point]) {
        map_points(&self.forward, points);
    }

    /// The points through the `transform` of the Python map's inverse,
    /// called once with all of them; NaN without an inverse.
    fn inverse_all(&self, points: &mut [Point]) {
        match &self.inverse {
            Some(inverse) => map_points(inverse, points),
            None => points.fill(Point::new(f64::NAN, f64::NAN)),
        }
    }

    /// The path through the Python map's `transform_path`, else its
    /// vertices through its `transform`; NaN vertices when it fails (the
    /// exception is raised when the native call running returns).
    fn transform_path(&self, path: &Path) -> Path {
        self.path_through(self.path.as_ref(), path)
    }

    /// The outline through the Python map's `transform_outline`, else as
    /// [`transform_path`](Self::transform_path) maps it.
    fn transform_outline(&self, path: &Path) -> Path {
        self.path_through(self.outline.as_ref().or(self.path.as_ref()), path)
    }

    /// The paths through the Python map's `transform_members`, called
    /// once with all of them, else each as
    /// [`transform_outline`](Self::transform_outline) maps it; NaN
    /// vertices when it fails (the exception is raised when the native
    /// call running returns).
    fn transform_members(&self, paths: &[Path]) -> Vec<Path> {
        let Some(members) = &self.members else {
            return paths
                .iter()
                .map(|path| self.transform_outline(path))
                .collect();
        };
        let given = paths
            .iter()
            .map(|path| {
                path_arrays(path, false)
                    .map(|(vertices, codes)| (vertices, codes, path.interpolation_steps()))
            })
            .collect::<PyResult<Vec<_>>>();
        given
            .ok()
            .and_then(|given| members.call::<Vec<PyPath>>((given,)))
            .filter(|mapped| mapped.len() == paths.len())
            .map_or_else(
                || paths.iter().map(failed_path).collect(),
                |mapped| mapped.into_iter().map(|mapped| mapped.path).collect(),
            )
    }
}

/// `path` with every vertex NaN: what a map of the program's that failed
/// gives (its exception is raised when the native call running returns).
fn failed_path(path: &Path) -> Path {
    let mut failed = path.clone();
    failed.vertices_mut().fill(Point::new(f64::NAN, f64::NAN));
    failed
}

impl PyPointMap {
    /// `path` through `path_map`, a method of the Python map taking a
    /// path's arrays, else its vertices through its `transform`.
    fn path_through(&self, path_map: Option<&Callback>, path: &Path) -> Path {
        let Some(path_map) = path_map else {
            let mut mapped = path.clone();
            self.forward_all(mapped.vertices_mut());
            return mapped;
        };
        // With its interpolation steps, which gridlines have many of so
        // that a curving map can bend them (matplotlib's
        // `_interpolation_steps`).
        path_arrays(path, false)
            .ok()
            .and_then(|(vertices, codes)| {
                path_map.call::<PyPath>((vertices, codes, path.interpolation_steps()))
            })
            .map_or_else(|| failed_path(path), |mapped| mapped.path)
    }
}

/// The transform a description gives.
fn native(description: Description) -> PyResult<Transform> {
    Ok(description.0)
}

/// A transform as Python's nested tuples, artists named by their ids; a
/// custom map comes back as `("native", transform)`.
struct Described(Transform);

impl IntoPy for Described {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let wrap = |inner: Box<Transform>| Described(*inner);
        match self.0 {
            Transform::Display => ("display",).into_py(py),
            Transform::Inches => ("inches",).into_py(py),
            Transform::Affine(map) => ("affine", matrix(map)?).into_py(py),
            Transform::Data(axes) => ("data", axes.index()).into_py(py),
            Transform::Axes(axes) => ("axes", axes.index()).into_py(py),
            Transform::Figure(frame) => ("figure", frame.index()).into_py(py),
            Transform::FigureBox(frame) => ("figure_box", frame.index()).into_py(py),
            Transform::Blended(x, y) => ("blended", wrap(x), wrap(y)).into_py(py),
            Transform::Composite(first, then) => ("composite", wrap(first), wrap(then)).into_py(py),
            Transform::Translation { dx, dy, units } => {
                ("translation", dx, dy, wrap(units)).into_py(py)
            }
            Transform::Inverted(inner) => ("inverted", wrap(inner)).into_py(py),
            Transform::AffineDelta(inner) => ("affine_delta", wrap(inner)).into_py(py),
            custom @ Transform::Custom(_) => ("native", custom).into_py(py),
        }
    }
}

/// The description of `transform` (see [`Described`]).
fn describe(transform: Transform) -> PyResult<Described> {
    Ok(Described(transform))
}

/// `transform` resolved in `figure`'s current layout.
fn resolved(figure: &Figure, transform: &Transform) -> PyResult<ResolvedTransform> {
    Ok(figure.resolve_transform_as_is(transform)?)
}

/// The points of `values` (`(2,)` or `(N, 2)`) mapped by `transform` in
/// `figure`'s current layout: entirely (`part` `"all"`), by its affine
/// part, or by its non-affine part; all at once, so that a transform of
/// the program's is called once with all of them, as matplotlib calls it.
fn apply(figure: Figure, transform: Transform, values: NdArray, part: String) -> PyResult<NdArray> {
    let resolved = resolved(&figure, &transform)?;
    let mut mapped = points(&values)?;
    match part.as_str() {
        "affine" => {
            let affine = resolved.affine_part();
            for p in &mut mapped {
                *p = affine.transform_point(*p);
            }
        }
        "non_affine" => resolved.transform_points_non_affine(&mut mapped),
        _ => resolved.transform_points(&mut mapped),
    }
    points_array(&mapped, values.shape)
}

/// The 3×3 matrix of the affine part of `transform` in `figure`'s current
/// layout (matplotlib's `get_matrix`).
fn affine_matrix(figure: Figure, transform: Transform) -> PyResult<NdArray> {
    matrix(resolved(&figure, &transform)?.affine_part())
}

/// Whether `transform` is affine in `figure`'s current layout (linear
/// scales).
fn is_affine(figure: Figure, transform: Transform) -> PyResult<bool> {
    Ok(resolved(&figure, &transform)?.is_affine())
}

/// Whether `transform` maps x and y independently in `figure`'s current
/// layout (`is_separable`).
fn is_separable(figure: Figure, transform: Transform) -> PyResult<bool> {
    Ok(resolved(&figure, &transform)?.is_separable())
}

/// The inverse of `transform`.
fn inverted(transform: Transform) -> PyResult<Transform> {
    Ok(transform.inverted())
}

/// The transform set on the artist, `None` while it keeps the default of
/// the function that made it.
fn get_transform(artist: RawHandle) -> PyResult<Option<Transform>> {
    Ok(artist.transform())
}

/// Places the artist in the coordinates of `transform` (display
/// coordinates for `None`, matplotlib's identity transform).
fn set_transform(artist: RawHandle, transform: Option<Transform>) -> PyResult<()> {
    artist.set_transform(transform.unwrap_or(Transform::Display));
    Ok(())
}

/// Forgets the transform set on `artist` (matplotlib's `_transformSet =
/// False`): what adds the artist gives it its own.
fn unset_transform(artist: RawHandle) -> PyResult<()> {
    artist.unset_transform();
    Ok(())
}

/// The data transform of the axes (`ax.transData`).
fn trans_data(axes: Axes) -> PyResult<Transform> {
    Ok(axes.trans_data())
}

/// The axes fractions of the axes (`ax.transAxes`).
fn trans_axes(axes: Axes) -> PyResult<Transform> {
    Ok(axes.trans_axes())
}

/// x in data, y in axes coordinates (`ax.get_xaxis_transform()`).
fn trans_xaxis(axes: Axes) -> PyResult<Transform> {
    Ok(axes.trans_xaxis())
}

/// x in axes, y in data coordinates (`ax.get_yaxis_transform()`).
fn trans_yaxis(axes: Axes) -> PyResult<Transform> {
    Ok(axes.trans_yaxis())
}

/// The fractions of the figure or subfigure (`fig.transFigure`,
/// `subfig.transSubfigure`).
fn trans_figure(figure: Figure) -> PyResult<Transform> {
    Ok(Transform::Figure(figure.id()))
}

/// The fractions of the root `figure`'s own box, which a print cut to a
/// window leaves in place (matplotlib's `BboxTransformTo(fig.bbox)`).
fn figure_box(figure: Figure) -> PyResult<Transform> {
    Ok(Transform::FigureBox(figure.id()))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("native", native)?;
    module.add("figure_box", figure_box)?;
    module.add("describe", describe)?;
    module.add("apply", apply)?;
    module.add("affine_matrix", affine_matrix)?;
    module.add("is_affine", is_affine)?;
    module.add("is_separable", is_separable)?;
    module.add("inverted", inverted)?;
    module.add("get_transform", get_transform)?;
    module.add("set_transform", set_transform)?;
    module.add("unset_transform", unset_transform)?;
    module.add("trans_data", trans_data)?;
    module.add("trans_axes", trans_axes)?;
    module.add("trans_xaxis", trans_xaxis)?;
    module.add("trans_yaxis", trans_yaxis)?;
    module.add("trans_figure", trans_figure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};

    #[test]
    fn live_transforms_follow_the_layout() {
        let figure = Figure::with_size(4.0, 2.0, 50.0);
        let fractions = trans_figure(figure.clone()).unwrap();
        let corner = NdArray::with_shape(vec![1.0, 1.0], vec![2]).unwrap();
        let mapped = apply(figure.clone(), fractions.clone(), corner, "all".into()).unwrap();
        assert_eq!(mapped.data, [200.0, 100.0]);
        figure.set_size_inches(2.0, 2.0).unwrap();
        let mtx = affine_matrix(figure.clone(), fractions.clone()).unwrap();
        assert_eq!(mtx.data[0], 100.0, "the layout of now");
        let inverse = inverted(fractions).unwrap();
        assert!(is_affine(figure, inverse).unwrap());
    }

    #[test]
    fn descriptions_round_trip() {
        with_python(|py| {
            let source = "('composite', ('inches',), ('blended', ('display',), ('inches',)))";
            let description: Description = eval(py, "", source).extract().unwrap();
            let described = describe(description.0).unwrap().into_py(py).unwrap();
            assert_eq!(described.repr().unwrap(), source);
            let unknown = eval(py, "", "('sideways',)").extract::<Description>();
            assert!(unknown.is_err());
            let short = eval(py, "", "('blended', ('display',))").extract::<Description>();
            assert!(short.is_err());
        });
    }

    /// A non-affine transform of the program's and its inverse.
    const DOUBLE: &str = r"
class Halve:
    def transform(self, xy):
        return (xy[0] / 2, xy[1])

class Double:
    def transform(self, xy):
        return (xy[0] * 2, xy[1])
    def inverted(self):
        return Halve()

class OneWay:
    def transform(self, xy):
        return (xy[0] * 2, xy[1])
";

    #[test]
    fn python_transforms_become_maps_calling_them() {
        with_python(|py| {
            let description: Description =
                eval(py, DOUBLE, "('python', Double())").extract().unwrap();
            let Transform::Custom(map) = description.0 else {
                panic!("a custom map, not {:?}", description.0);
            };
            let map = map.map();
            assert_eq!(map.forward(Point::new(3.0, 1.0)), Point::new(6.0, 1.0));
            assert_eq!(map.inverse(Point::new(3.0, 1.0)), Point::new(1.5, 1.0));
            let one_way: Description = eval(py, DOUBLE, "('python', OneWay())").extract().unwrap();
            let Transform::Custom(map) = one_way.0 else {
                panic!("a custom map, not {:?}", one_way.0);
            };
            assert!(map.map().inverse(Point::new(1.0, 1.0)).x.is_nan());
            let mut none = [Point::new(1.0, 1.0)];
            map.map().inverse_all(&mut none);
            assert!(none[0].x.is_nan(), "no inverse to call with the points");
        });
    }

    #[test]
    fn axis_transforms_blend_data_and_axes() {
        let figure = Figure::with_size(4.0, 4.0, 100.0);
        let axes = figure.add_axes([0.0, 0.0, 1.0, 1.0]).unwrap();
        axes.set_xlim(0.0, 4.0).unwrap().set_ylim(0.0, 2.0).unwrap();
        let point = NdArray::with_shape(vec![1.0, 0.5], vec![2]).unwrap();
        let xaxis = trans_xaxis(axes.clone()).unwrap();
        let mapped = apply(figure.clone(), xaxis, point.clone(), "all".into()).unwrap();
        assert_eq!(mapped.data, [100.0, 200.0]);
        let yaxis = trans_yaxis(axes).unwrap();
        let mapped = apply(figure, yaxis, point, "all".into()).unwrap();
        assert_eq!(mapped.data, [400.0, 100.0]);
    }
}
