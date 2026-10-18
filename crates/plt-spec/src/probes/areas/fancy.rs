//! Probe operations for the `fancy` area: the paths of fancy box, arrow
//! and connection styles parsed from `"name,key=value"` strings.

use plt::patches::{ArrowStyle, BoxStyle, ConnectionStyle};
use plt_core::{Path, PathCode, Point};

use crate::json::Json;
use crate::probes::encode;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("patches.BoxStyle", box_style),
    Op::new("patches.ArrowStyle", arrow_style),
    Op::new("patches.ConnectionStyle", connection_style),
];

/// A path, with codes that only say "move, then lines" written as null
/// (the same drawing as no codes; see `test_fancy.py`).
fn canonical(path: &Path) -> Json {
    let codes = path.resolved_codes();
    let polyline = codes.split_first().is_some_and(|(first, rest)| {
        *first == PathCode::MoveTo && rest.iter().all(|&code| code == PathCode::LineTo)
    });
    if polyline {
        encode::path(&Path::polyline(path.vertices().to_vec()))
    } else {
        encode::path(path)
    }
}

/// The outline of `BoxStyle(style)(x0, y0, width, height, mutation_size)`.
fn box_style(args: Args<'_>) -> Outcome {
    let style: BoxStyle = args.str("style")?.parse().map_err(ProbeError::rejected)?;
    let [x0, y0, width, height] = <[f64; 4]>::try_from(args.f64_vec("rect")?)
        .map_err(|_| ProbeError::BadArgs("rect needs four numbers".into()))?;
    Ok(canonical(&style.path(
        x0,
        y0,
        width,
        height,
        args.f64("mutation_size")?,
    )))
}

/// The pieces of `ArrowStyle(style)(path, ...)`, each with whether it is
/// filled.
fn arrow_style(args: Args<'_>) -> Outcome {
    let style: ArrowStyle = args.str("style")?.parse().map_err(ProbeError::rejected)?;
    let path = encode::parse_path(args.json("path")?)?;
    let pieces = style
        .paths(
            &path,
            args.f64("mutation_size")?,
            args.f64("linewidth")?,
            args.f64("aspect")?,
        )
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        pieces
            .iter()
            .map(|(piece, filled)| Json::Array(vec![canonical(piece), Json::Bool(*filled)]))
            .collect(),
    ))
}

/// The raw path of `ConnectionStyle(style).connect(a, b)`.
fn connection_style(args: Args<'_>) -> Outcome {
    let style: ConnectionStyle = args.str("style")?.parse().map_err(ProbeError::rejected)?;
    let point = |key: &str| -> Result<Point, ProbeError> {
        let [x, y] = <[f64; 2]>::try_from(args.f64_vec(key)?)
            .map_err(|_| ProbeError::BadArgs(format!("{key} needs two numbers")))?;
        Ok(Point::new(x, y))
    };
    style
        .connect(point("a")?, point("b")?)
        .map(|path| canonical(&path))
        .map_err(ProbeError::rejected)
}
