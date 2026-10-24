//! Probe operations for the `tri` area: triangulations, point location,
//! interpolation, refinement and mesh analysis.
//!
//! Canonical forms follow `spec/reference/probes/test_tri.py`: triangles as
//! index triples rotated to start at their smallest index (sorted for
//! Delaunay output), neighbours aligned with them, refined meshes as sorted
//! coordinate triangles, and masked values as `null`.

pub(crate) mod fields;

use plt::tri::{TriAnalyzer, TriError, Triangulation};

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("tri.Triangulation.delaunay", delaunay),
    Op::new("tri.Triangulation.delaunay_counts", delaunay_counts),
    Op::new("tri.Triangulation.delaunay_order", delaunay_order),
    Op::new("tri.Triangulation.explicit", explicit),
    Op::new(
        "tri.Triangulation.calculate_plane_coefficients",
        plane_coefficients,
    ),
    Op::new("tri.TrapezoidMapTriFinder.find", find),
    Op::new("tri.LinearTriInterpolator.__call__", fields::linear_call),
    Op::new(
        "tri.LinearTriInterpolator.gradient",
        fields::linear_gradient,
    ),
    Op::new("tri.CubicTriInterpolator.__call__", fields::cubic_call),
    Op::new("tri.CubicTriInterpolator.gradient", fields::cubic_gradient),
    Op::new(
        "tri.UniformTriRefiner.refine_triangulation",
        fields::refine_triangulation,
    ),
    Op::new("tri.UniformTriRefiner.refine_field", fields::refine_field),
    Op::new("tri.TriAnalyzer.scale_factors", scale_factors),
    Op::new("tri.TriAnalyzer.circle_ratios", circle_ratios),
    Op::new("tri.TriAnalyzer.get_flat_tri_mask", flat_tri_mask),
];

/// A flat list of numbers; nested lists (non-1-D arrays) are rejected.
pub(crate) fn floats(args: Args<'_>, key: &str) -> Result<Vec<f64>, ProbeError> {
    let value = args.json(key)?;
    match value.as_array() {
        Some(items) if items.iter().any(|v| v.as_array().is_some()) => {
            Err(ProbeError::rejected(format!("{key} must be a 1-D array")))
        }
        _ => value
            .as_f64_vec()
            .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list of numbers"))),
    }
}

/// Triangles as given (truncated to integers) and as validated indices.
type ParsedTriangles = (Vec<[i64; 3]>, Vec<[usize; 3]>);

/// `np.array(triangles, dtype=np.int32)` validated as an `(N, 3)` array of
/// non-negative indices (float indices truncate).
fn parse_triangles(value: &Json) -> Result<ParsedTriangles, ProbeError> {
    let shape_error = || ProbeError::rejected("triangles must be a (N, 3) int array");
    let rows = value.as_array().ok_or_else(shape_error)?;
    if rows.is_empty() {
        return Err(shape_error());
    }
    let raw = rows
        .iter()
        .map(|row| match row.as_f64_vec().as_deref() {
            Some(&[a, b, c]) => Ok([a, b, c].map(|v| v.trunc() as i64)),
            _ => Err(shape_error()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let indices = raw
        .iter()
        .map(|t| {
            t.iter()
                .map(|&v| usize::try_from(v).map_err(|_| shape_error()))
                .collect::<Result<Vec<_>, _>>()
                .map(|v| [v[0], v[1], v[2]])
        })
        .collect::<Result<_, _>>()?;
    Ok((raw, indices))
}

/// `null` or a list with one truthy value per triangle; a bare scalar is
/// not a valid mask.
fn parse_mask(args: Args<'_>) -> Result<Option<Vec<bool>>, ProbeError> {
    let Some(value) = args.get("mask") else {
        return Ok(None);
    };
    let items = value
        .as_array()
        .ok_or_else(|| ProbeError::rejected("mask array must have same length as triangles"))?;
    items
        .iter()
        .map(|v| {
            v.as_bool()
                .or_else(|| v.as_f64().map(|n| n != 0.0))
                .ok_or_else(|| ProbeError::BadArgs(format!("bad mask value {v}")))
        })
        .collect::<Result<_, _>>()
        .map(Some)
}

/// The triangulation described by `x`, `y`, `triangles` (or Delaunay when
/// `null`) and `mask`.
pub(crate) fn triangulation(args: Args<'_>) -> Result<Triangulation, ProbeError> {
    let (x, y) = (floats(args, "x")?, floats(args, "y")?);
    let tri = match args.get("triangles") {
        None => Triangulation::delaunay(&x, &y),
        Some(value) => Triangulation::new(&x, &y, parse_triangles(value)?.1),
    }
    .map_err(ProbeError::rejected)?;
    tri.with_mask(parse_mask(args)?)
        .map_err(ProbeError::rejected)
}

fn int(v: usize) -> Json {
    Json::Integer(v as i64)
}

fn index_list(values: &[usize]) -> Json {
    Json::Array(values.iter().map(|&v| int(v)).collect())
}

/// A triple rotated so that its smallest index comes first.
fn canon_triple(t: [usize; 3]) -> [usize; 3] {
    let r = (0..3).fold(0, |best, i| if t[i] < t[best] { i } else { best });
    [t[r], t[(r + 1) % 3], t[(r + 2) % 3]]
}

fn triple_json(t: [usize; 3]) -> Json {
    index_list(&t)
}

/// Canonical triangles and neighbours, sorted by canonical triple.
fn canonical_topology(tri: &Triangulation) -> (Json, Json) {
    let mut rows: Vec<([usize; 3], Json)> = tri
        .triangles()
        .iter()
        .zip(tri.neighbors())
        .map(|(&t, nb)| {
            let r = (0..3).fold(0, |best, i| if t[i] < t[best] { i } else { best });
            let neighbors = (0..3)
                .map(|i| {
                    nb[(r + i) % 3].map_or(Json::Null, |n| {
                        triple_json(canon_triple(tri.triangles()[n]))
                    })
                })
                .collect();
            (canon_triple(t), Json::Array(neighbors))
        })
        .collect();
    rows.sort_by_key(|(t, _)| *t);
    let triangles = rows.iter().map(|(t, _)| triple_json(*t)).collect();
    let neighbors = rows.into_iter().map(|(_, n)| n).collect();
    (Json::Array(triangles), Json::Array(neighbors))
}

fn edges_json(tri: &Triangulation) -> Json {
    Json::Array(tri.edges().iter().map(|e| index_list(e)).collect())
}

/// Triangles as stored, in order.
fn triangles_json(triangles: &[[usize; 3]]) -> Json {
    Json::Array(triangles.iter().map(|t| triple_json(*t)).collect())
}

/// Neighbours as matplotlib stores them, in triangle order, `-1` for none.
fn raw_neighbors_json(tri: &Triangulation) -> Json {
    Json::Array(
        tri.neighbors()
            .iter()
            .map(|nb| {
                Json::Array(
                    nb.iter()
                        .map(|n| n.map_or(Json::Integer(-1), int))
                        .collect(),
                )
            })
            .collect(),
    )
}

fn object<const N: usize>(entries: [(&str, Json); N]) -> Json {
    Json::Object(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

/// A finite value, or `null` (numpy's `masked_invalid`).
pub(crate) fn masked(value: Option<f64>) -> Json {
    value
        .filter(|v| v.is_finite())
        .map_or(Json::Null, Json::from)
}

fn delaunay(args: Args<'_>) -> Outcome {
    let tri = Triangulation::delaunay(&floats(args, "x")?, &floats(args, "y")?)
        .map_err(ProbeError::rejected)?;
    let (triangles, neighbors) = canonical_topology(&tri);
    Ok(object([
        ("triangles", triangles),
        ("neighbors", neighbors),
        ("edges", edges_json(&tri)),
        ("is_delaunay", Json::from(tri.is_delaunay())),
    ]))
}

pub(crate) fn delaunay_counts(args: Args<'_>) -> Outcome {
    let tri = Triangulation::delaunay(&floats(args, "x")?, &floats(args, "y")?)
        .map_err(ProbeError::rejected)?;
    let mut used: Vec<usize> = tri.triangles().iter().flatten().copied().collect();
    used.sort_unstable();
    used.dedup();
    Ok(object([
        ("ntri", int(tri.ntri())),
        ("nedges", int(tri.edges().len())),
        ("points_used", index_list(&used)),
    ]))
}

/// The triangles and neighbours exactly as matplotlib's qhull wrapper
/// orders them.
pub(crate) fn delaunay_order(args: Args<'_>) -> Outcome {
    let tri = Triangulation::delaunay(&floats(args, "x")?, &floats(args, "y")?)
        .map_err(ProbeError::rejected)?;
    Ok(object([
        ("triangles", triangles_json(tri.triangles())),
        ("neighbors", raw_neighbors_json(&tri)),
    ]))
}

pub(crate) fn explicit(args: Args<'_>) -> Outcome {
    let (x, y) = (floats(args, "x")?, floats(args, "y")?);
    let (raw, indices) = parse_triangles(args.json("triangles")?)?;
    let mut tri = Triangulation::new(&x, &y, indices).map_err(ProbeError::rejected)?;
    tri.set_mask(parse_mask(args)?)
        .map_err(ProbeError::rejected)?;
    Ok(object([
        (
            "triangles_input",
            Json::Array(
                raw.iter()
                    .map(|t| Json::Array(t.iter().map(|&v| Json::Integer(v)).collect()))
                    .collect(),
            ),
        ),
        ("triangles", triangles_json(tri.triangles())),
        ("masked_triangles", triangles_json(&tri.masked_triangles())),
        ("edges", edges_json(&tri)),
        ("neighbors", raw_neighbors_json(&tri)),
        ("is_delaunay", Json::from(tri.is_delaunay())),
    ]))
}

fn plane_coefficients(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let planes = tri
        .plane_coefficients(&floats(args, "z")?)
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        planes.into_iter().map(|p| Json::from(p.to_vec())).collect(),
    ))
}

/// Query points `qx`, `qy` of equal length.
pub(crate) fn queries(args: Args<'_>) -> Result<Vec<(f64, f64)>, ProbeError> {
    let (qx, qy) = (floats(args, "qx")?, floats(args, "qy")?);
    if qx.len() != qy.len() {
        return Err(ProbeError::rejected("x and y must have the same shape"));
    }
    Ok(qx.into_iter().zip(qy).collect())
}

fn find(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let points = queries(args)?;
    let finder = tri.trifinder().map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        points
            .iter()
            .map(|&(x, y)| {
                finder.find(x, y).map_or(Json::Null, |t| {
                    triple_json(canon_triple(tri.triangles()[t]))
                })
            })
            .collect(),
    ))
}

pub(crate) fn scale_factors(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    Ok(Json::from(TriAnalyzer::new(&tri).scale_factors().to_vec()))
}

pub(crate) fn circle_ratios(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let ratios = TriAnalyzer::new(&tri).circle_ratios(args.bool("rescale")?);
    Ok(Json::Array(ratios.into_iter().map(masked).collect()))
}

pub(crate) fn flat_tri_mask(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let mask =
        TriAnalyzer::new(&tri).flat_tri_mask(args.f64("min_circle_ratio")?, args.bool("rescale")?);
    Ok(Json::from(mask))
}

/// Maps triangulation errors to probe rejections.
pub(crate) fn rejected(err: TriError) -> ProbeError {
    ProbeError::rejected(err)
}
