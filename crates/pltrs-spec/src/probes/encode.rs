//! JSON encodings shared by several probe areas (paths, matrices,
//! transform operation lists), as documented in the spec's Python modules.

use std::borrow::Borrow;

use pltrs::colors::Color;
use pltrs_core::{Affine2D, Bbox, Path, PathCode, Point, Rgba};

use super::{Outcome, ProbeError};
use crate::json::Json;

/// `[x, ...]`: a list of numbers, given as values or references.
pub(crate) fn numbers<T: Borrow<f64>>(values: impl IntoIterator<Item = T>) -> Json {
    Json::Array(
        values
            .into_iter()
            .map(|v| Json::from(*v.borrow()))
            .collect(),
    )
}

/// `[a, b]`.
pub(crate) fn pair((a, b): (f64, f64)) -> Json {
    Json::from(vec![a, b])
}

/// A count, as Python's `len` and friends report it (a JSON number).
pub(crate) fn count(n: usize) -> Json {
    Json::from(n as f64)
}

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

/// `[r, g, b, a]`.
pub(crate) fn rgba(color: Rgba) -> Json {
    Json::from(color.to_array().to_vec())
}

/// `[r, g, b, a]` of `color` resolved, or the rejection of a colour that
/// does not resolve.
pub(crate) fn resolved_rgba(color: &Color) -> Outcome {
    color.resolve().map(rgba).map_err(ProbeError::rejected)
}

/// `[[x, y], ...]`.
pub(crate) fn points(points: &[Point]) -> Json {
    Json::Array(points.iter().map(|p| point(*p)).collect())
}

/// A JSON object with the given entries, in order.
pub(crate) fn object<const N: usize>(entries: [(&str, Json); N]) -> Json {
    Json::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
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
    apply_affine_ops(Affine2D::IDENTITY, value)
}

/// Applies a TRANSFORM OP LIST to `start`: `rotate`, `rotate_deg`,
/// `rotate_around`, `rotate_deg_around`, `scale`, `translate`, `skew`,
/// `skew_deg`, `clear` and `set_matrix`.
///
/// matplotlib evaluates angles with Python's `math.cos`/`math.tan`, which
/// raise `ValueError` for infinite arguments, so infinite angles are
/// rejected; NaN angles give NaN coefficients.
pub(crate) fn apply_affine_ops(start: Affine2D, value: &Json) -> Result<Affine2D, ProbeError> {
    let ops = value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("expected an op list, got {value}")))?;
    ops.iter().try_fold(start, apply_affine_op)
}

fn apply_affine_op(t: Affine2D, op: &Json) -> Result<Affine2D, ProbeError> {
    let parts = op.as_array().unwrap_or(&[]);
    let name = parts.first().and_then(Json::as_str).unwrap_or_default();
    let args = &parts[1.min(parts.len())..];
    if let ("set_matrix", [matrix]) = (name, args) {
        return parse_matrix(matrix);
    }
    let params: Vec<f64> = args.iter().filter_map(Json::as_f64).collect();
    let angle = |a: f64| {
        if a.is_infinite() {
            Err(ProbeError::rejected("math domain error"))
        } else {
            Ok(a)
        }
    };
    Ok(match (name, params.as_slice()) {
        ("rotate", [rad]) => t.rotate(angle(*rad)?),
        ("rotate_deg", [deg]) => t.rotate_deg(angle(*deg)?),
        ("rotate_around", [x, y, rad]) => t.rotate_around(*x, *y, angle(*rad)?),
        ("rotate_deg_around", [x, y, deg]) => t.rotate_deg_around(*x, *y, angle(*deg)?),
        ("scale", [s]) => t.scale(*s, *s),
        ("scale", [sx, sy]) => t.scale(*sx, *sy),
        ("translate", [tx, ty]) => t.translate(*tx, *ty),
        ("skew", [x, y]) => t.skew(angle(*x)?, angle(*y)?),
        ("skew_deg", [x, y]) => t.skew_deg(angle(*x)?, angle(*y)?),
        ("clear", []) => Affine2D::IDENTITY,
        _ => return Err(ProbeError::BadArgs(format!("unknown transform op {op}"))),
    })
}

/// Decodes a 3×3 matrix `[[a, c, e], [b, d, f], [0, 0, 1]]`.
pub(crate) fn parse_matrix(value: &Json) -> Result<Affine2D, ProbeError> {
    let rows: Option<Vec<Vec<f64>>> = value
        .as_array()
        .and_then(|rows| rows.iter().map(Json::as_f64_vec).collect());
    match rows.as_deref() {
        Some([top, middle, bottom]) if bottom.as_slice() == [0.0, 0.0, 1.0] => {
            match (top.as_slice(), middle.as_slice()) {
                ([a, c, e], [b, d, f]) => Ok(Affine2D::from_values(*a, *b, *c, *d, *e, *f)),
                _ => Err(ProbeError::BadArgs(format!("not a 3x3 matrix: {value}"))),
            }
        }
        _ => Err(ProbeError::BadArgs(format!(
            "not an affine 3x3 matrix: {value}"
        ))),
    }
}

/// `[[x0, y0], [x1, y1]]`.
pub(crate) fn bbox(b: &Bbox) -> Json {
    Json::Array(vec![point(b.p0), point(b.p1)])
}

/// Decodes `[[x0, y0], [x1, y1]]`.
pub(crate) fn parse_bbox(value: &Json) -> Result<Bbox, ProbeError> {
    match parse_points(value)?.as_slice() {
        [p0, p1] => Ok(Bbox::new(*p0, *p1)),
        _ => Err(ProbeError::BadArgs(format!("expected a bbox, got {value}"))),
    }
}

/// Decodes a path dict the way `Path(np.asarray(vertices).reshape(-1, 2),
/// codes)` builds it (so `[]` is the empty path). pltrs paths have no
/// `STOP` code: matplotlib's algorithms end a path at its first `STOP`, so
/// the path is truncated there.
pub(crate) fn parse_mpl_path(value: &Json) -> Result<Path, ProbeError> {
    let raw_vertices = value.get("vertices").unwrap_or(&Json::Null);
    let mut vertices = if raw_vertices.as_array().is_some_and(<[Json]>::is_empty) {
        Vec::new()
    } else {
        parse_points(raw_vertices)?
    };
    let Some(codes) = value.get("codes").filter(|c| !c.is_null()) else {
        return Ok(Path::polyline(vertices));
    };
    let mut codes = codes
        .as_f64_vec()
        .ok_or_else(|| ProbeError::BadArgs(format!("bad path codes {codes}")))?;
    if let Some(stop) = codes.iter().position(|c| *c as u8 == 0) {
        codes.truncate(stop);
        vertices.truncate(stop);
    }
    let codes = codes
        .into_iter()
        .map(|c| PathCode::try_from(c as u8).map_err(ProbeError::rejected))
        .collect::<Result<Vec<_>, _>>()?;
    Path::new(vertices, codes).map_err(ProbeError::rejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ops(text: &str) -> Result<Affine2D, ProbeError> {
        parse_affine_ops(&Json::parse(text).unwrap())
    }

    #[test]
    fn op_lists_post_compose() {
        assert_eq!(
            ops(r#"[["scale", 2, 3], ["translate", 1, 0]]"#),
            Ok(Affine2D::scaling(2.0, 3.0).translate(1.0, 0.0))
        );
        assert_eq!(
            ops(r#"[["scale", 3, 5], ["clear"], ["translate", 1, 1]]"#),
            Ok(Affine2D::translation(1.0, 1.0))
        );
        assert_eq!(
            ops(r#"[["rotate_deg", 10], ["set_matrix", [[1, 2, 3], [4, 5, 6], [0, 0, 1]]]]"#),
            Ok(Affine2D::from_values(1.0, 4.0, 2.0, 5.0, 3.0, 6.0))
        );
        assert_eq!(
            ops(r#"[["rotate_deg_around", 1, 2, 90]]"#),
            Ok(Affine2D::identity().rotate_deg_around(1.0, 2.0, 90.0))
        );
    }

    #[test]
    fn infinite_angles_are_rejected() {
        for text in [
            r#"[["rotate", Infinity]]"#,
            r#"[["rotate_deg", -Infinity]]"#,
            r#"[["skew", Infinity, 0]]"#,
            r#"[["rotate_around", 0, 0, Infinity]]"#,
        ] {
            assert!(matches!(ops(text), Err(ProbeError::Rejected(_))), "{text}");
        }
        let nan = ops(r#"[["rotate", NaN]]"#).expect("NaN angles are allowed");
        assert!(nan.values()[0].is_nan());
    }

    #[test]
    fn malformed_ops_and_matrices() {
        assert!(matches!(
            ops(r#"[["shear", 1]]"#),
            Err(ProbeError::BadArgs(_))
        ));
        assert!(matches!(
            ops(r#"[["scale", 1, 2, 3]]"#),
            Err(ProbeError::BadArgs(_))
        ));
        let projective = Json::parse("[[1, 0, 0], [0, 1, 0], [1, 0, 1]]").unwrap();
        assert!(parse_matrix(&projective).is_err());
    }
}
