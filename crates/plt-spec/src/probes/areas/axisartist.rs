//! Probe operations for the `axisartist` area (grid levels, labels,
//! extremes and grid information of curvilinear grids).

use std::f64::consts::PI;

use plt::axisartist::AuxTransform;
use plt::axisartist::angle_helper::{
    self, AngleUnit, ExtremeFinderCycle, FormatterDms, FormatterHms, LocatorD, LocatorDm,
    LocatorDms, LocatorH, LocatorHm, LocatorHms,
};
use plt::axisartist::grid_finder::{
    CoordinateGrid, DictFormatter, ExtremeFinder, ExtremeFinderSimple, FixedLocator,
    FormatterPrettyPrint, GridError, GridFinder, GridFinderOptions, GridFormatter, GridLevels,
    GridLocator, MaxNLocator,
};
use plt::spines::SpineSide;
use plt::ticker::{Nbins, Prune};
use plt::transforms::Affine2D;
use plt_core::Bbox;
use plt_core::numeric::np_sign;

use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Op, Outcome, ProbeError, malformed};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("angle_helper.select_step", select_step),
    Op::new("angle_helper.select_step24", select_step24),
    Op::new("angle_helper.select_step360", select_step360),
    Op::new("angle_helper.select_step_degree", select_step_degree),
    Op::new("angle_helper.select_step_hour", select_step_hour),
    Op::new("angle_helper.select_step_sub", select_step_sub),
    Op::new("angle_helper.Locator", locator),
    Op::new("angle_helper.Formatter", formatter),
    Op::new("angle_helper.ExtremeFinderCycle", extreme_finder_cycle),
    Op::new("grid_finder.ExtremeFinderSimple", extreme_finder_simple),
    Op::new("grid_finder.MaxNLocator", max_n_locator),
    Op::new("grid_finder.FixedLocator", fixed_locator),
    Op::new("grid_finder.FormatterPrettyPrint", formatter_pretty_print),
    Op::new("grid_finder.DictFormatter", dict_formatter),
    Op::new("grid_finder.get_grid_info", grid_info),
];

/// `[levels, n, factor]`.
fn levels(levels: Result<GridLevels, GridError>) -> Outcome {
    let GridLevels { levels, n, factor } = levels.map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        Json::from(levels),
        Json::from(n as f64),
        Json::from(factor),
    ]))
}

/// A non-negative integer argument.
fn count(args: Args<'_>, key: &str) -> Result<usize, ProbeError> {
    usize::try_from(args.i64(key)?).map_err(|_| malformed(key))
}

/// `(v1, v2, nv, include_last, threshold_factor)` of the `select_step`
/// family.
fn step_args(args: Args<'_>) -> Result<(f64, f64, usize, bool, f64), ProbeError> {
    Ok((
        args.f64("v1")?,
        args.f64("v2")?,
        count(args, "nv")?,
        args.bool("include_last")?,
        args.f64("threshold_factor")?,
    ))
}

fn select_step(args: Args<'_>) -> Outcome {
    let (v1, v2, nv, include_last, threshold) = step_args(args)?;
    let unit = if args.bool("hour")? {
        AngleUnit::Hour
    } else {
        AngleUnit::Degree
    };
    levels(angle_helper::select_step(
        v1,
        v2,
        nv,
        unit,
        include_last,
        threshold,
    ))
}

fn select_step24(args: Args<'_>) -> Outcome {
    let (v1, v2, nv, include_last, threshold) = step_args(args)?;
    levels(angle_helper::select_step24(
        v1,
        v2,
        nv,
        include_last,
        threshold,
    ))
}

fn select_step360(args: Args<'_>) -> Outcome {
    let (v1, v2, nv, include_last, threshold) = step_args(args)?;
    levels(angle_helper::select_step360(
        v1,
        v2,
        nv,
        include_last,
        threshold,
    ))
}

/// `[step, factor]`.
fn step_factor((step, factor): (f64, f64)) -> Json {
    Json::from(vec![step, factor])
}

fn select_step_degree(args: Args<'_>) -> Outcome {
    angle_helper::select_step_degree(args.f64("dv")?)
        .map(step_factor)
        .map_err(ProbeError::rejected)
}

fn select_step_hour(args: Args<'_>) -> Outcome {
    angle_helper::select_step_hour(args.f64("dv")?)
        .map(step_factor)
        .map_err(ProbeError::rejected)
}

fn select_step_sub(args: Args<'_>) -> Outcome {
    Ok(step_factor(angle_helper::select_step_sub(args.f64("dv")?)))
}

fn locator(args: Args<'_>) -> Outcome {
    let nbins = count(args, "nbins")?;
    let include_last = args.opt_bool("include_last")?.unwrap_or(true);
    let locator: Box<dyn GridLocator> = match args.str("kind")? {
        "DMS" => Box::new(LocatorDms::new(nbins).with_include_last(include_last)),
        "DM" => Box::new(LocatorDm::new(nbins).with_include_last(include_last)),
        "D" => Box::new(LocatorD::new(nbins).with_include_last(include_last)),
        "HMS" => Box::new(LocatorHms::new(nbins).with_include_last(include_last)),
        "HM" => Box::new(LocatorHm::new(nbins).with_include_last(include_last)),
        "H" => Box::new(LocatorH::new(nbins).with_include_last(include_last)),
        _ => return Err(malformed("kind")),
    };
    levels(locator.levels(args.f64("v1")?, args.f64("v2")?))
}

fn side(name: &str) -> Result<SpineSide, ProbeError> {
    SpineSide::ALL
        .into_iter()
        .find(|side| side.name() == name)
        .ok_or_else(|| malformed("direction"))
}

/// The labels of `formatter`.
fn labels(
    formatter: &dyn GridFormatter,
    direction: SpineSide,
    factor: f64,
    values: &[f64],
) -> Outcome {
    formatter
        .labels(direction, factor, values)
        .map(Json::from)
        .map_err(ProbeError::rejected)
}

fn formatter(args: Args<'_>) -> Outcome {
    let formatter: &dyn GridFormatter = match args.str("kind")? {
        "DMS" => &FormatterDms,
        "HMS" => &FormatterHms,
        _ => return Err(malformed("kind")),
    };
    labels(
        formatter,
        side(args.str("direction")?)?,
        args.f64("factor")?,
        &args.f64_vec("values")?,
    )
}

/// The grid transform named `name` (see the Python module).
fn transform(name: &str) -> Result<AuxTransform, ProbeError> {
    Ok(match name {
        "polar_degrees" => {
            AuxTransform::affine(Affine2D::scaling(PI / 180.0, 1.0)).then(AuxTransform::polar())
        }
        "polar" => AuxTransform::polar(),
        "shear" => AuxTransform::from_fns(|x, y| (x, y - x), |x, y| (x, y + x)),
        "sqrt" => AuxTransform::from_fns(
            |x, y| (np_sign(x) * x.abs().sqrt(), y),
            |x, y| (np_sign(x) * (x * x), y),
        ),
        "rotated" => AuxTransform::affine(Affine2D::scaling(2.0, 1.0).rotate_deg(30.0)),
        _ => return Err(malformed("transform")),
    })
}

fn bbox(args: Args<'_>) -> Result<Bbox, ProbeError> {
    match args.f64_vec("bbox")?.as_slice() {
        &[x0, y0, x1, y1] => Ok(Bbox::from_extents(x0, y0, x1, y1)),
        _ => Err(malformed("bbox")),
    }
}

/// `[x0, y0, x1, y1]`.
fn extents(bbox: Bbox) -> Json {
    let (x0, y0, x1, y1) = bbox.extents();
    Json::from(vec![x0, y0, x1, y1])
}

/// The extents found by `finder` for the box of `args` through the
/// inverse of its transform.
fn find(finder: &dyn ExtremeFinder, args: Args<'_>) -> Outcome {
    let inverse = transform(args.str("transform")?)?
        .inverted()
        .ok_or_else(|| ProbeError::rejected("singular transform"))?;
    Ok(extents(finder.find_transformed_bbox(&inverse, bbox(args)?)))
}

/// An optional `[low, high]` pair.
fn range(args: Args<'_>, key: &str) -> Result<Option<(f64, f64)>, ProbeError> {
    args.get(key)
        .map(|_| match args.f64_vec(key)?.as_slice() {
            &[low, high] => Ok((low, high)),
            _ => Err(malformed(key)),
        })
        .transpose()
}

fn extreme_finder_cycle(args: Args<'_>) -> Outcome {
    let finder = ExtremeFinderCycle {
        lon_cycle: args.opt_f64("lon_cycle")?,
        lat_cycle: args.opt_f64("lat_cycle")?,
        lon_minmax: range(args, "lon_minmax")?,
        lat_minmax: range(args, "lat_minmax")?,
        ..ExtremeFinderCycle::new(count(args, "nx")?, count(args, "ny")?)
    };
    find(&finder, args)
}

fn extreme_finder_simple(args: Args<'_>) -> Outcome {
    find(
        &ExtremeFinderSimple::new(count(args, "nx")?, count(args, "ny")?),
        args,
    )
}

fn max_n_locator(args: Args<'_>) -> Outcome {
    let mut locator = plt::ticker::MaxNLocator::new(Nbins::Fixed(count(args, "nbins")?))
        .with_integer(args.opt_bool("integer")?.unwrap_or(false))
        .with_symmetric(args.opt_bool("symmetric")?.unwrap_or(false))
        .with_prune(match args.opt_str("prune")? {
            None => None,
            Some("lower") => Some(Prune::Lower),
            Some("upper") => Some(Prune::Upper),
            Some("both") => Some(Prune::Both),
            Some(_) => return Err(malformed("prune")),
        });
    if args.get("steps").is_some() {
        locator = locator
            .with_steps(&args.f64_vec("steps")?)
            .map_err(ProbeError::rejected)?;
    }
    levels(MaxNLocator::from_locator(locator).levels(args.f64("v1")?, args.f64("v2")?))
}

fn fixed_locator(args: Args<'_>) -> Outcome {
    levels(FixedLocator::new(args.f64_vec("locs")?).levels(args.f64("v1")?, args.f64("v2")?))
}

fn formatter_pretty_print(args: Args<'_>) -> Outcome {
    let formatter = FormatterPrettyPrint::new(args.opt_bool("useMathText")?.unwrap_or(true));
    labels(&formatter, SpineSide::Bottom, 1.0, &args.f64_vec("values")?)
}

fn dict_formatter(args: Args<'_>) -> Outcome {
    let table = args
        .json("format_dict")?
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .map(|entry| match entry.as_array()? {
                    [value, label] => Some((value.as_f64()?, label.as_str()?.to_owned())),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| malformed("format_dict"))?;
    let fallback: Option<Box<dyn GridFormatter>> = args
        .bool("fallback")?
        .then(|| Box::new(FormatterPrettyPrint::default()) as Box<dyn GridFormatter>);
    labels(
        &DictFormatter::new(table, fallback),
        SpineSide::Bottom,
        1.0,
        &args.f64_vec("values")?,
    )
}

/// The polar grid of the figure cases in degrees with `grid_locator2`.
fn polar_grid(grid_locator2: Option<Box<dyn GridLocator>>) -> Result<GridFinder, ProbeError> {
    Ok(GridFinder::new(
        transform("polar_degrees")?,
        GridFinderOptions {
            extreme_finder: Some(Box::new(ExtremeFinderCycle {
                lat_minmax: Some((0.0, f64::INFINITY)),
                ..ExtremeFinderCycle::new(20, 20)
            })),
            grid_locator1: Some(Box::new(LocatorDms::new(12))),
            grid_locator2,
            tick_formatter1: Some(Box::new(FormatterDms)),
            ..GridFinderOptions::default()
        },
    ))
}

/// The grid named `name` (see `GRIDS` in the Python module).
fn grid(name: &str) -> Result<GridFinder, ProbeError> {
    match name {
        "shear" => Ok(GridFinder::new(
            transform("shear")?,
            GridFinderOptions::default(),
        )),
        "polar" => polar_grid(None),
        "polar5" => polar_grid(Some(Box::new(MaxNLocator::new(5)))),
        "sqrt" => Ok(GridFinder::new(
            transform("sqrt")?,
            GridFinderOptions {
                extreme_finder: Some(Box::new(ExtremeFinderSimple::new(20, 20))),
                grid_locator1: Some(Box::new(MaxNLocator::new(6))),
                grid_locator2: Some(Box::new(MaxNLocator::new(6))),
                ..GridFinderOptions::default()
            },
        )),
        _ => Err(malformed("grid")),
    }
}

/// `{lines: [[first, last], ...], ticks: {side: [[x, y, angle, label],
/// ...]}}`.
fn coordinate(grid: &CoordinateGrid) -> Json {
    let point = |p: plt_core::Point| Json::from(vec![p.x, p.y]);
    let lines = grid
        .lines
        .iter()
        .filter_map(|line| {
            Some(Json::Array(vec![
                point(*line.first()?),
                point(*line.last()?),
            ]))
        })
        .collect();
    let ticks = SpineSide::ALL
        .into_iter()
        .map(|side| {
            let ticks = grid
                .side_ticks(side)
                .iter()
                .map(|tick| {
                    Json::Array(vec![
                        Json::from(tick.loc.x),
                        Json::from(tick.loc.y),
                        Json::from(tick.angle),
                        Json::from(tick.label.as_str()),
                    ])
                })
                .collect();
            (side.name().to_owned(), Json::Array(ticks))
        })
        .collect();
    object([
        ("lines", Json::Array(lines)),
        ("ticks", Json::Object(ticks)),
    ])
}

fn grid_info(args: Args<'_>) -> Outcome {
    let info = grid(args.str("grid")?)?
        .grid_info(bbox(args)?)
        .map_err(ProbeError::rejected)?;
    Ok(object([
        ("extremes", extents(info.extremes)),
        ("lon", coordinate(&info.lon)),
        ("lat", coordinate(&info.lat)),
    ]))
}
