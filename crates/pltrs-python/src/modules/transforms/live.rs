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
//! `("affine_delta", t)`, or `("native", transform)` for a value made
//! before. [`describe`] gives the same tuples back, with the artist ids of
//! the figure (`_native.artist.id`) in place of the handles.

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs::transforms::{Point, ResolvedTransform, Transform, TransformError};

use super::{affine, matrix, points, points_array};
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
            "blended" => Transform::blended(child(1)?, child(2)?),
            "composite" => child(1)?.then(child(2)?),
            "translation" => Transform::Translation {
                dx: item(1)?.extract()?,
                dy: item(2)?.extract()?,
                units: Box::new(child(3)?),
            },
            "inverted" => child(1)?.inverted(),
            "affine_delta" => Transform::AffineDelta(Box::new(child(1)?)),
            other => {
                return Err(PyErr::value_error(format!(
                    "unknown kind of transform {other:?}"
                )));
            }
        };
        Ok(Self(transform))
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
    Ok(figure.resolve_transform(transform)?)
}

/// The points of `values` (`(2,)` or `(N, 2)`) mapped by `transform` in
/// `figure`'s current layout: entirely (`part` `"all"`), by its affine
/// part, or by its non-affine part.
fn apply(figure: Figure, transform: Transform, values: NdArray, part: String) -> PyResult<NdArray> {
    let resolved = resolved(&figure, &transform)?;
    let map = |p: Point| match part.as_str() {
        "affine" => resolved.affine_part().transform_point(p),
        "non_affine" => resolved.transform_point_non_affine(p),
        _ => resolved.transform_point(p),
    };
    let mapped: Vec<Point> = points(&values)?.into_iter().map(map).collect();
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

/// The data transform of the axes (`ax.transData`).
fn trans_data(axes: Axes) -> PyResult<Transform> {
    Ok(axes.trans_data())
}

/// The axes fractions of the axes (`ax.transAxes`).
fn trans_axes(axes: Axes) -> PyResult<Transform> {
    Ok(axes.trans_axes())
}

/// The fractions of the figure or subfigure (`fig.transFigure`,
/// `subfig.transSubfigure`).
fn trans_figure(figure: Figure) -> PyResult<Transform> {
    Ok(Transform::Figure(figure.id()))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("native", native)?;
    module.add("describe", describe)?;
    module.add("apply", apply)?;
    module.add("affine_matrix", affine_matrix)?;
    module.add("is_affine", is_affine)?;
    module.add("inverted", inverted)?;
    module.add("get_transform", get_transform)?;
    module.add("set_transform", set_transform)?;
    module.add("trans_data", trans_data)?;
    module.add("trans_axes", trans_axes)?;
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
}
