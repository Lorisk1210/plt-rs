//! Probe operations on fields defined over a triangulation: linear and
//! cubic interpolators and uniform refinement.

use std::cmp::Ordering;

use plt::tri::{
    CubicKind, CubicTriInterpolator, LinearTriInterpolator, TriInterpolator, Triangulation,
    UniformTriRefiner,
};

use super::{canon_triple, floats, int, masked, object, queries, rejected, triangulation};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

pub(super) fn linear_call(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let interp = LinearTriInterpolator::new(&tri, &floats(args, "z")?).map_err(rejected)?;
    Ok(values(&interp, &queries(args)?))
}

pub(super) fn linear_gradient(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let interp = LinearTriInterpolator::new(&tri, &floats(args, "z")?).map_err(rejected)?;
    Ok(gradients(&interp, &queries(args)?))
}

pub(super) fn cubic_call(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let interp = cubic(&tri, args, &cubic_kind(args.str("kind")?, args)?)?;
    Ok(values(&interp, &queries(args)?))
}

pub(super) fn cubic_gradient(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let interp = cubic(&tri, args, &cubic_kind(args.str("kind")?, args)?)?;
    Ok(gradients(&interp, &queries(args)?))
}

pub(super) fn refine_triangulation(args: Args<'_>) -> Outcome {
    let base = triangulation(args)?;
    let refiner = UniformTriRefiner::new(&base);
    let subdiv = subdiv(args)?;
    let (refined, parents) = if args.bool("return_tri_index")? {
        let (refined, found) = refiner.refine_triangulation_with_index(subdiv);
        let parents: Vec<Option<[usize; 3]>> = found
            .iter()
            .map(|parent| parent.map(|t| canon_triple(base.triangles()[t])))
            .collect();
        (refined, Some(parents))
    } else {
        (refiner.refine_triangulation(subdiv), None)
    };
    let mut order: Vec<usize> = (0..refined.npoints()).collect();
    let point_key = |i: usize| [refined.x()[i], refined.y()[i]];
    match &parents {
        Some(parents) => order.sort_by(|&a, &b| {
            cmp_floats(&point_key(a), &point_key(b)).then(parents[a].cmp(&parents[b]))
        }),
        None => order.sort_by(|&a, &b| cmp_floats(&point_key(a), &point_key(b))),
    }
    let (masked_tris, unmasked_tris): (Vec<_>, Vec<_>) = (0..refined.ntri())
        .map(|t| (refined.is_masked(t), xy_triangle(&refined, t)))
        .partition(|(is_masked, _)| *is_masked);
    let mut entries = vec![
        ("npoints", int(refined.npoints())),
        (
            "points",
            Json::Array(
                order
                    .iter()
                    .map(|&i| Json::from(point_key(i).to_vec()))
                    .collect(),
            ),
        ),
        ("triangles", xy_triangles_json(unmasked_tris)),
        ("masked_triangles", xy_triangles_json(masked_tris)),
        ("has_mask", Json::from(refined.mask().is_some())),
    ];
    if let Some(parents) = parents {
        let aligned = order
            .iter()
            .map(|&i| Json::from(parents[i].map(|t| t.map(int).to_vec())))
            .collect();
        entries.push(("parents", Json::Array(aligned)));
    }
    Ok(Json::Object(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    ))
}

pub(super) fn refine_field(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let z = floats(args, "z")?;
    let refiner = UniformTriRefiner::new(&tri);
    let subdiv = subdiv(args)?;
    let refined = match args.opt_str("interpolator")? {
        None => refiner.refine_field(&z, None, subdiv),
        Some("linear") => {
            let interp = LinearTriInterpolator::new(&tri, &z).map_err(rejected)?;
            refiner.refine_field(&z, Some(&interp), subdiv)
        }
        Some(name) => {
            let kind = name
                .strip_prefix("cubic_")
                .ok_or_else(|| ProbeError::BadArgs(format!("unknown interpolator {name:?}")))?;
            let interp = cubic(&tri, args, &cubic_kind(kind, args)?)?;
            refiner.refine_field(&z, Some(&interp), subdiv)
        }
    }
    .map_err(rejected)?;
    let field = &refined.triangulation;
    let mut rows: Vec<(f64, f64, Option<f64>)> = (0..field.npoints())
        .map(|i| (field.x()[i], field.y()[i], refined.z[i]))
        .collect();
    rows.sort_by(|a, b| {
        cmp_floats(&[a.0, a.1], &[b.0, b.1])
            .then(a.2.is_none().cmp(&b.2.is_none()))
            .then(cmp_floats(&[a.2.unwrap_or(0.0)], &[b.2.unwrap_or(0.0)]))
    });
    Ok(Json::Array(
        rows.into_iter()
            .map(|(x, y, z)| Json::Array(vec![Json::from(x), Json::from(y), masked(z)]))
            .collect(),
    ))
}

/// `subdiv`, where non-positive values mean no refinement.
fn subdiv(args: Args<'_>) -> Result<usize, ProbeError> {
    Ok(usize::try_from(args.i64("subdiv")?).unwrap_or(0))
}

/// The cubic interpolator of `z` over `tri`.
fn cubic<'a>(
    tri: &'a Triangulation,
    args: Args<'_>,
    kind: &CubicKind,
) -> Result<CubicTriInterpolator<'a>, ProbeError> {
    CubicTriInterpolator::new(tri, &floats(args, "z")?, kind).map_err(rejected)
}

/// The `CubicTriInterpolator` kind named `name`; `'user'` takes its nodal
/// derivatives from `dz = [dzdx, dzdy]`.
fn cubic_kind(name: &str, args: Args<'_>) -> Result<CubicKind, ProbeError> {
    match name {
        "geom" => Ok(CubicKind::Geom),
        "min_E" => Ok(CubicKind::MinE),
        "user" => {
            let dz = args
                .get("dz")
                .ok_or_else(|| ProbeError::rejected("kind 'user' requires dz"))?;
            let list = |v: &Json| {
                v.as_f64_vec()
                    .ok_or_else(|| ProbeError::BadArgs("dz entries must be number lists".into()))
            };
            match dz.as_array() {
                Some([dzdx, dzdy]) => Ok(CubicKind::User {
                    dzdx: list(dzdx)?,
                    dzdy: list(dzdy)?,
                }),
                _ => Err(ProbeError::BadArgs("dz must be [dzdx, dzdy]".into())),
            }
        }
        other => Err(ProbeError::rejected(format!(
            "invalid cubic kind {other:?}"
        ))),
    }
}

/// Interpolated values at `points`, `null` outside or where undefined.
fn values(interp: &dyn TriInterpolator, points: &[(f64, f64)]) -> Json {
    Json::Array(
        points
            .iter()
            .map(|&(x, y)| masked(interp.value(x, y)))
            .collect(),
    )
}

/// Interpolated gradients at `points` as `{"dzdx", "dzdy"}` lists.
fn gradients(interp: &dyn TriInterpolator, points: &[(f64, f64)]) -> Json {
    let grads: Vec<Option<[f64; 2]>> = points.iter().map(|&(x, y)| interp.gradient(x, y)).collect();
    let component = |k: usize| Json::Array(grads.iter().map(|g| masked(g.map(|g| g[k]))).collect());
    object([("dzdx", component(0)), ("dzdy", component(1))])
}

/// Lexicographic comparison with Python's float semantics (NaN compares
/// equal to everything, `-0.0 == 0.0`).
fn cmp_floats(a: &[f64], b: &[f64]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.partial_cmp(y).unwrap_or(Ordering::Equal))
        .find(|o| o.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// The corners of triangle `t`, rotated to start at the smallest `(x, y)`.
fn xy_triangle(tri: &Triangulation, t: usize) -> [[f64; 2]; 3] {
    let corners = tri.triangles()[t].map(|v| [tri.x()[v], tri.y()[v]]);
    let first = (1..3).fold(0, |best, k| {
        if cmp_floats(&corners[k], &corners[best]).is_lt() {
            k
        } else {
            best
        }
    });
    [0, 1, 2].map(|k| corners[(first + k) % 3])
}

/// Sorted canonical xy-triangles.
fn xy_triangles_json(tris: Vec<(bool, [[f64; 2]; 3])>) -> Json {
    let mut tris: Vec<[[f64; 2]; 3]> = tris.into_iter().map(|(_, t)| t).collect();
    tris.sort_by(|a, b| cmp_floats(a.as_flattened(), b.as_flattened()));
    Json::Array(
        tris.into_iter()
            .map(|t| Json::Array(t.iter().map(|p| Json::from(p.to_vec())).collect()))
            .collect(),
    )
}
