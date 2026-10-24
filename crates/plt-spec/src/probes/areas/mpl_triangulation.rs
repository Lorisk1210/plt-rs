//! Probe operations for the `mpl_triangulation` area: matplotlib's own
//! tests of triangular grids (see
//! `spec/reference/probes/test_mpl_triangulation.py`, which documents every
//! op). Most operations are the `tri` and `contour` areas'.

use plt::tri::{
    CubicKind, CubicTriInterpolator, LinearTriInterpolator, TriInterpolator, Triangulation,
};
use plt_core::{Path, Point};

use super::contour;
use super::tri::fields::{
    cubic_call, cubic_gradient, linear_call, refine_field, refine_triangulation,
};
use super::tri::{
    circle_ratios, delaunay_counts, delaunay_order, explicit, flat_tri_mask, floats, masked,
    queries, rejected, scale_factors, triangulation,
};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("tri.Triangulation.explicit", explicit),
    Op::new("tri.Triangulation.delaunay_counts", delaunay_counts),
    Op::new("tri.Triangulation.delaunay_order", delaunay_order),
    Op::new("tri.Triangulation.tris_contain_point", tris_contain_point),
    Op::new("tri.Triangulation.neighbors", neighbors_match),
    Op::new("tri.TrapezoidMapTriFinder.__call__", find_indices),
    Op::new("tri.LinearTriInterpolator.__call__", linear_call),
    Op::new("tri.CubicTriInterpolator.__call__", cubic_call),
    Op::new("tri.CubicTriInterpolator.gradient", cubic_gradient),
    Op::new("TriInterpolator._interpolate_multikeys", multikeys),
    Op::new(
        "tri.UniformTriRefiner.refine_triangulation",
        refine_triangulation,
    ),
    Op::new("tri.UniformTriRefiner.refine_field", refine_field),
    Op::new("tri.TriAnalyzer.scale_factors", scale_factors),
    Op::new("tri.TriAnalyzer.circle_ratios", circle_ratios),
    Op::new("tri.TriAnalyzer.get_flat_tri_mask", flat_tri_mask),
    Op::new("Axes.tricontourf", contour::tricontourf),
    Op::new("ContourSet.get_paths", contour::raw_paths),
];

/// The Delaunay triangulation of `x`, `y`.
fn delaunay(args: Args<'_>) -> Result<Triangulation, ProbeError> {
    Triangulation::delaunay(&floats(args, "x")?, &floats(args, "y")?).map_err(rejected)
}

/// How many Delaunay triangles contain each point.
fn tris_contain_point(args: Args<'_>) -> Outcome {
    let tri = delaunay(args)?;
    let outlines: Vec<Path> = tri
        .triangles()
        .iter()
        .map(|t| Path::polyline(t.map(|v| tri.point(v)).to_vec()))
        .collect();
    let points = encode::parse_points(args.json("points")?)?;
    Ok(Json::Array(
        points
            .into_iter()
            .map(|p: Point| {
                let count = outlines
                    .iter()
                    .filter(|outline| outline.contains_point(p, None, 0.0))
                    .count();
                Json::Integer(i64::try_from(count).unwrap_or(i64::MAX))
            })
            .collect(),
    ))
}

/// Whether the Delaunay neighbours equal those computed again from the
/// triangles.
fn neighbors_match(args: Args<'_>) -> Outcome {
    let tri = delaunay(args)?;
    let again = Triangulation::new(tri.x(), tri.y(), tri.triangles().to_vec()).map_err(rejected)?;
    Ok(Json::Bool(again.neighbors() == tri.neighbors()))
}

/// The triangle indices the finder returns, `-1` outside, after an
/// optional new mask.
fn find_indices(args: Args<'_>) -> Outcome {
    let mut tri = triangulation(args)?;
    if let Some(mask) = args.get("set_mask") {
        let mask = mask
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs("set_mask must be a list".into()))?
            .iter()
            .map(|v| v.as_bool().or_else(|| v.as_f64().map(|n| n != 0.0)))
            .collect::<Option<Vec<bool>>>()
            .ok_or_else(|| ProbeError::BadArgs("bad set_mask value".into()))?;
        tri.set_mask(Some(mask)).map_err(rejected)?;
    }
    let finder = tri.trifinder().map_err(rejected)?;
    Ok(Json::Array(
        queries(args)?
            .into_iter()
            .map(|(x, y)| {
                finder.find(x, y).map_or(Json::Integer(-1), |t| {
                    Json::Integer(i64::try_from(t).unwrap_or(i64::MAX))
                })
            })
            .collect(),
    ))
}

/// The values of an interpolator at the query points, all evaluated in
/// one given triangle (matplotlib's `_interpolate_multikeys` with
/// `tri_index`).
fn multikeys(args: Args<'_>) -> Outcome {
    let tri = triangulation(args)?;
    let z = floats(args, "z")?;
    let index = usize::try_from(args.i64("tri")?)
        .ok()
        .filter(|&t| t < tri.ntri())
        .ok_or_else(|| ProbeError::BadArgs("tri is not a triangle index".into()))?;
    let interp: Box<dyn TriInterpolator + '_> = match args.str("interpolator")? {
        "linear" => Box::new(LinearTriInterpolator::new(&tri, &z).map_err(rejected)?),
        "cubic_min_E" => {
            Box::new(CubicTriInterpolator::new(&tri, &z, &CubicKind::MinE).map_err(rejected)?)
        }
        "cubic_geom" => {
            Box::new(CubicTriInterpolator::new(&tri, &z, &CubicKind::Geom).map_err(rejected)?)
        }
        other => {
            return Err(ProbeError::BadArgs(format!(
                "unknown interpolator {other:?}"
            )));
        }
    };
    Ok(Json::Array(
        queries(args)?
            .into_iter()
            .map(|(x, y)| masked(Some(interp.value_in(index, x, y))))
            .collect(),
    ))
}
