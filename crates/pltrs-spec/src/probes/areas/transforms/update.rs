//! Data-limit updates of a mutable `Bbox` (`update_from_data_*`,
//! `update_from_path`, `ignore`) and `frozen` snapshots.

use pltrs::transforms::{Point, TrackedBbox};

use super::bbox_arg;
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `{"points", "minpos", "mutatedx", "mutatedy"}` after applying `steps`
/// to `Bbox(bbox)` (with the optional initial `minpos`).
pub(super) fn update(args: Args<'_>) -> Outcome {
    let mut bbox = TrackedBbox::new(bbox_arg(args, "bbox")?);
    if args.get("minpos").is_some() {
        let minpos = args.f64_vec("minpos")?;
        let &[x, y] = minpos.as_slice() else {
            return Err(ProbeError::BadArgs(format!(
                "minpos {minpos:?} is not [x, y]"
            )));
        };
        bbox.set_minpos(Point::new(x, y));
    }
    apply_steps(&mut bbox, args)?;
    Ok(encode::object([
        ("points", encode::bbox(&bbox.bbox())),
        ("minpos", encode::point(bbox.minpos())),
        ("mutatedx", Json::from(bbox.mutated_x())),
        ("mutatedy", Json::from(bbox.mutated_y())),
    ]))
}

/// `{"points", "minpos"}` of `frozen()` taken after `steps`.
pub(super) fn frozen(args: Args<'_>) -> Outcome {
    let mut bbox = TrackedBbox::new(bbox_arg(args, "bbox")?);
    apply_steps(&mut bbox, args)?;
    let frozen = bbox.limits();
    Ok(encode::object([
        ("points", encode::bbox(&frozen.bbox)),
        ("minpos", encode::point(frozen.minpos)),
    ]))
}

/// Applies each `{"method": ..., ...}` step of the `steps` argument.
fn apply_steps(bbox: &mut TrackedBbox, args: Args<'_>) -> Result<(), ProbeError> {
    let steps = args.json("steps")?;
    let steps = steps
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("steps is not a list: {steps}")))?;
    steps
        .iter()
        .try_for_each(|step| apply_step(bbox, Args::new(step)))
}

fn apply_step(bbox: &mut TrackedBbox, step: Args<'_>) -> Result<(), ProbeError> {
    let ignore = step.opt_bool("ignore")?;
    let update_x = step.opt_bool("updatex")?.unwrap_or(true);
    let update_y = step.opt_bool("updatey")?.unwrap_or(true);
    match step.str("method")? {
        "ignore" => bbox.set_ignore(step.bool("value")?),
        "update_from_data_xy" => {
            let points = encode::parse_points(step.json("xy")?)?;
            bbox.update_from_points(&points, ignore, update_x, update_y);
        }
        "update_from_data_x" => bbox.update_from_x(&step.f64_vec("x")?, ignore),
        "update_from_data_y" => bbox.update_from_y(&step.f64_vec("y")?, ignore),
        "update_from_path" => {
            let path = encode::parse_mpl_path(step.json("path")?)?;
            bbox.update_from_path(&path, ignore, update_x, update_y);
        }
        other => {
            return Err(ProbeError::BadArgs(format!(
                "unknown update method {other:?}"
            )));
        }
    }
    Ok(())
}
