//! JSON encodings shared by several probe areas (paths, matrices,
//! transform operation lists), as documented in the spec's Python modules.

use plt_core::{Affine2D, Path, PathCode, Point};

use super::ProbeError;
use crate::json::Json;

/// `{"vertices": [[x, y], ...], "codes": [int, ...] | null}`.
pub(crate) fn path(path: &Path) -> Json {
    let vertices = path
        .vertices()
        .iter()
        .map(|p| point(*p))
        .collect::<Vec<_>>();
    let codes = path.codes().map(|codes| {
        Json::Array(
            codes
                .iter()
                .map(|c| Json::from(f64::from(*c as u8)))
                .collect(),
        )
    });
    Json::Object(vec![
        ("vertices".into(), Json::Array(vertices)),
        ("codes".into(), codes.unwrap_or(Json::Null)),
    ])
}

/// `[x, y]`.
pub(crate) fn point(p: Point) -> Json {
    Json::from(vec![p.x, p.y])
}

/// The 3×3 matrix `[[a, c, e], [b, d, f], [0, 0, 1]]`.
pub(crate) fn matrix(t: &Affine2D) -> Json {
    Json::Array(
        t.matrix()
            .iter()
            .map(|row| Json::from(row.to_vec()))
            .collect(),
    )
}

/// Decodes `[[x, y], ...]`.
pub(crate) fn parse_points(value: &Json) -> Result<Vec<Point>, ProbeError> {
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| match item.as_f64_vec()?.as_slice() {
                    [x, y] => Some(Point::new(*x, *y)),
                    _ => None,
                })
                .collect()
        })
        .ok_or_else(|| ProbeError::BadArgs(format!("expected [[x, y], ...], got {value}")))
}

/// Decodes the path encoding produced by [`path`].
pub(crate) fn parse_path(value: &Json) -> Result<Path, ProbeError> {
    let vertices = parse_points(value.get("vertices").unwrap_or(&Json::Null))?;
    match value.get("codes").filter(|c| !c.is_null()) {
        None => Ok(Path::polyline(vertices)),
        Some(codes) => {
            let codes = codes
                .as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad path codes {codes}")))?
                .into_iter()
                .map(|c| PathCode::try_from(c as u8).map_err(ProbeError::rejected))
                .collect::<Result<Vec<_>, _>>()?;
            Path::new(vertices, codes).map_err(ProbeError::rejected)
        }
    }
}

/// Applies a TRANSFORM OP LIST (`[["rotate_deg", 30], ["scale", 2], ...]`)
/// to a fresh identity transform, each op post-composing.
pub(crate) fn parse_affine_ops(value: &Json) -> Result<Affine2D, ProbeError> {
    let ops = value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("expected an op list, got {value}")))?;
    ops.iter().try_fold(Affine2D::IDENTITY, |t, op| {
        let parts = op.as_array().unwrap_or(&[]);
        let name = parts.first().and_then(Json::as_str).unwrap_or_default();
        let params: Vec<f64> = parts[1.min(parts.len())..]
            .iter()
            .filter_map(Json::as_f64)
            .collect();
        Ok(match (name, params.as_slice()) {
            ("rotate_deg", [deg]) => t.rotate_deg(*deg),
            ("rotate", [rad]) => t.rotate(*rad),
            ("scale", [s]) => t.scale(*s, *s),
            ("scale", [sx, sy]) => t.scale(*sx, *sy),
            ("translate", [tx, ty]) => t.translate(*tx, *ty),
            ("skew_deg", [x, y]) => t.skew_deg(*x, *y),
            ("skew", [x, y]) => t.skew(*x, *y),
            _ => return Err(ProbeError::BadArgs(format!("unknown transform op {op}"))),
        })
    })
}
