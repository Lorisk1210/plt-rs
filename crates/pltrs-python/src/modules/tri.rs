//! `_native.tri`: unstructured triangular grids (matplotlib's `tri`):
//! triangulations, point location, interpolation, refinement and mesh
//! analysis, done by `pltrs::tri`.
//!
//! A triangulation crosses as a native value; interpolators, finders,
//! refiners and analyzers borrow it in pltrs (Rust), so each query here
//! makes the one it needs from the triangulation the Python object keeps.

use pltrs::tri::{
    CubicKind, CubicTriInterpolator, LinearTriInterpolator, TriAnalyzer, TriError, TriFinder,
    TriInterpolator, Triangulation, UniformTriRefiner,
};

use crate::binding::prelude::*;

native_value!(Triangulation);

impl From<TriError> for PyErr {
    /// matplotlib's `ValueError` for invalid triangulations and fields.
    fn from(err: TriError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// Indices as an `(N, 3)` array of triangles.
fn triangle_rows(triangles: &NdArray<i64>) -> PyResult<Vec<[usize; 3]>> {
    if triangles.data.is_empty() {
        return Ok(Vec::new());
    }
    if triangles.shape.len() != 2 || triangles.shape[1] != 3 {
        return Err(PyErr::value_error(
            "triangles must be a (N, 3) int array, not shaped otherwise",
        ));
    }
    triangles
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .map(|row| {
            let index = |v: i64| {
                usize::try_from(v).map_err(|_| {
                    PyErr::value_error(format!(
                        "triangles are indices into the points and must be in the range 0 <= \
                         i < N but found value {v}"
                    ))
                })
            };
            Ok([index(row[0])?, index(row[1])?, index(row[2])?])
        })
        .collect()
}

/// `(N, 3)` triangles back as an int array.
fn triangles_array(triangles: &[[usize; 3]]) -> PyResult<NdArray<i64>> {
    let data = triangles
        .iter()
        .flatten()
        .map(|&v| i64::try_from(v).unwrap_or(i64::MAX))
        .collect();
    NdArray::with_shape(data, vec![triangles.len(), 3])
}

/// A triangulation of the points `(x, y)`: of the given `triangles`, else
/// Delaunay's; masked by `mask`.
fn new(
    x: NdArray,
    y: NdArray,
    triangles: Option<NdArray<i64>>,
    mask: Option<Vec<bool>>,
) -> PyResult<Triangulation> {
    let (x, y) = (x.into_vec()?, y.into_vec()?);
    let tri = match triangles {
        Some(triangles) => Triangulation::new(&x, &y, triangle_rows(&triangles)?)?,
        None => Triangulation::delaunay(&x, &y)?,
    };
    Ok(tri.with_mask(mask)?)
}

/// `(x, y)` of the points.
fn points(tri: Triangulation) -> PyResult<(NdArray, NdArray)> {
    Ok((
        NdArray::from_vec(tri.x().to_vec()),
        NdArray::from_vec(tri.y().to_vec()),
    ))
}

/// Every triangle, masked or not, in the vertex order given.
fn triangles(tri: Triangulation) -> PyResult<NdArray<i64>> {
    triangles_array(tri.given_triangles())
}

/// The unmasked triangles, in the vertex order given.
fn masked_triangles(tri: Triangulation) -> PyResult<NdArray<i64>> {
    let given = tri.given_triangles();
    let unmasked: Vec<[usize; 3]> = tri.unmasked().map(|t| given[t]).collect();
    triangles_array(&unmasked)
}

/// The mask, `None` when unset.
fn mask(tri: Triangulation) -> PyResult<Option<Vec<bool>>> {
    Ok(tri.mask().map(<[bool]>::to_vec))
}

/// Sets (or, `None`, removes) the mask in place.
fn set_mask(tri: Native<Triangulation>, mask: Option<Vec<bool>>) -> PyResult<()> {
    tri.borrow_mut().set_mask(mask)?;
    Ok(())
}

/// The edges of the unmasked triangles, as an `(N, 2)` int array.
fn edges(tri: Triangulation) -> PyResult<NdArray<i64>> {
    let edges = tri.edges();
    let data = edges
        .iter()
        .flatten()
        .map(|&v| i64::try_from(v).unwrap_or(i64::MAX))
        .collect();
    NdArray::with_shape(data, vec![edges.len(), 2])
}

/// The neighbouring triangle across each edge, `-1` for none.
fn neighbors(tri: Triangulation) -> PyResult<NdArray<i64>> {
    let neighbors = tri.neighbors();
    let data = neighbors
        .iter()
        .flatten()
        .map(|n| n.map_or(-1, |n| i64::try_from(n).unwrap_or(i64::MAX)))
        .collect();
    NdArray::with_shape(data, vec![neighbors.len(), 3])
}

/// Whether the triangles are Delaunay's.
fn is_delaunay(tri: Triangulation) -> PyResult<bool> {
    Ok(tri.is_delaunay())
}

/// The plane `z = a·x + b·y + c` through each triangle, as an `(N, 3)`
/// array.
fn plane_coefficients(tri: Triangulation, z: NdArray) -> PyResult<NdArray> {
    let planes = tri.plane_coefficients(&z.into_vec()?)?;
    let rows = planes.len();
    NdArray::with_shape(planes.into_iter().flatten().collect(), vec![rows, 3])
}

/// The triangle holding each point `(x[i], y[i])`, `-1` outside.
fn find(tri: Triangulation, x: NdArray, y: NdArray) -> PyResult<NdArray<i64>> {
    let finder = TriFinder::new(&tri)?;
    Ok(NdArray::from_vec(
        x.data
            .iter()
            .zip(&y.data)
            .map(|(&x, &y)| {
                finder
                    .find(x, y)
                    .map_or(-1, |t| i64::try_from(t).unwrap_or(i64::MAX))
            })
            .collect(),
    ))
}

/// How a field is interpolated: `"linear"`, `"min_E"`, `"geom"`, or
/// `"user"` with the gradients `(dzdx, dzdy)` at the points.
type Kind = (String, Option<(NdArray, NdArray)>);

/// The cubic kind of a [`Kind`].
fn cubic_kind((name, gradients): Kind) -> PyResult<CubicKind> {
    Ok(match (name.as_str(), gradients) {
        ("geom", _) => CubicKind::Geom,
        ("user", Some((dzdx, dzdy))) => CubicKind::User {
            dzdx: dzdx.into_vec()?,
            dzdy: dzdy.into_vec()?,
        },
        _ => CubicKind::MinE,
    })
}

/// Runs `f` with the interpolator of `z` of `kind` on `tri`.
fn with_interpolator<R>(
    tri: &Triangulation,
    z: &[f64],
    kind: Kind,
    f: impl FnOnce(&dyn TriInterpolator) -> R,
) -> PyResult<R> {
    if kind.0 == "linear" {
        let interpolator = LinearTriInterpolator::new(tri, z)?;
        Ok(f(&interpolator))
    } else {
        let interpolator = CubicTriInterpolator::new(tri, z, &cubic_kind(kind)?)?;
        Ok(f(&interpolator))
    }
}

/// The interpolated values (`NaN` outside the triangulation) at the points
/// `(x, y)`, and the gradients `(dz/dx, dz/dy)`.
fn interpolate(
    tri: Triangulation,
    z: NdArray,
    kind: Kind,
    (x, y): (NdArray, NdArray),
) -> PyResult<(NdArray, NdArray, NdArray)> {
    let z = z.into_vec()?;
    let (values, dzdx, dzdy) = with_interpolator(&tri, &z, kind, |interp| {
        let mut values = Vec::with_capacity(x.data.len());
        let (mut dzdx, mut dzdy) = (Vec::new(), Vec::new());
        for (&x, &y) in x.data.iter().zip(&y.data) {
            values.push(interp.value(x, y).unwrap_or(f64::NAN));
            let [gx, gy] = interp.gradient(x, y).unwrap_or([f64::NAN; 2]);
            dzdx.push(gx);
            dzdy.push(gy);
        }
        (values, dzdx, dzdy)
    })?;
    Ok((
        NdArray::from_vec(values),
        NdArray::from_vec(dzdx),
        NdArray::from_vec(dzdy),
    ))
}

/// The triangulation refined `subdiv` times, and the original triangle
/// of each new point (`-1` for none).
fn refine(tri: Triangulation, subdiv: usize) -> PyResult<(Triangulation, NdArray<i64>)> {
    let (refined, found) = UniformTriRefiner::new(&tri).refine_triangulation_with_index(subdiv);
    let found = found
        .into_iter()
        .map(|t| t.map_or(-1, |t| i64::try_from(t).unwrap_or(i64::MAX)))
        .collect();
    Ok((refined, NdArray::from_vec(found)))
}

/// The triangulation refined `subdiv` times and `z` interpolated (as
/// `kind` says) at its points, `NaN` where none.
fn refine_field(
    tri: Triangulation,
    z: NdArray,
    kind: Kind,
    subdiv: usize,
) -> PyResult<(Triangulation, NdArray)> {
    let z = z.into_vec()?;
    let refined = with_interpolator(&tri, &z, kind, |interp| {
        UniformTriRefiner::new(&tri).refine_field(&z, Some(interp), subdiv)
    })??;
    let values = refined
        .z
        .into_iter()
        .map(|v| v.unwrap_or(f64::NAN))
        .collect();
    Ok((refined.triangulation, NdArray::from_vec(values)))
}

/// The factors scaling the triangulation to a unit square.
fn scale_factors(tri: Triangulation) -> PyResult<(f64, f64)> {
    let [sx, sy] = TriAnalyzer::new(&tri).scale_factors();
    Ok((sx, sy))
}

/// The ratio of the incircle to the circumcircle radius of each triangle
/// (`NaN` for masked ones), after rescaling when `rescale`.
fn circle_ratios(tri: Triangulation, rescale: bool) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(
        TriAnalyzer::new(&tri)
            .circle_ratios(rescale)
            .into_iter()
            .map(|r| r.unwrap_or(f64::NAN))
            .collect(),
    ))
}

/// The mask removing flat border triangles (matplotlib's
/// `get_flat_tri_mask`).
fn flat_tri_mask(tri: Triangulation, min_circle_ratio: f64, rescale: bool) -> PyResult<Vec<bool>> {
    Ok(TriAnalyzer::new(&tri).flat_tri_mask(min_circle_ratio, rescale))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("points", points)?;
    module.add("triangles", triangles)?;
    module.add("masked_triangles", masked_triangles)?;
    module.add("mask", mask)?;
    module.add("set_mask", set_mask)?;
    module.add("edges", edges)?;
    module.add("neighbors", neighbors)?;
    module.add("is_delaunay", is_delaunay)?;
    module.add("plane_coefficients", plane_coefficients)?;
    module.add("find", find)?;
    module.add("interpolate", interpolate)?;
    module.add("refine", refine)?;
    module.add("refine_field", refine_field)?;
    module.add("scale_factors", scale_factors)?;
    module.add("circle_ratios", circle_ratios)?;
    module.add("flat_tri_mask", flat_tri_mask)
}
