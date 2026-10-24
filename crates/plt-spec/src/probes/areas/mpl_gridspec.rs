//! Probe operations for the `mpl_gridspec` area: matplotlib's own tests of
//! grids, cell blocks and subplot parameters (see
//! `spec/reference/probes/test_mpl_gridspec.py`).

use std::ops::Range;

use plt::figure::Figure;
use plt::gridspec::{GridSpec, SubplotOverrides, SubplotParams, SubplotSpec};
use plt::rc::RcParams;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("SubplotSpec.__eq__", subplotspec_eq),
    Op::new("GridSpec.update", gridspec_update),
    Op::new("GridSpec.__repr__", gridspec_repr),
    Op::new("SubplotSpec.__repr__", subplotspec_repr),
    Op::new("SubplotParams", subplot_params),
    Op::new("SubplotParams.update", subplot_params_update),
    Op::new("SubplotParams.reset", subplot_params_reset),
    Op::new(
        "GridSpecFromSubplotSpec.get_topmost_subplotspec",
        topmost_subplotspec,
    ),
];

/// The names of the subplot parameters, in the order they are recorded.
const PARAMS: [&str; 6] = ["left", "bottom", "right", "top", "wspace", "hspace"];

/// A count from a JSON number.
fn count(value: &Json) -> Result<usize, ProbeError> {
    value
        .as_i64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| ProbeError::BadArgs(format!("{value} is not a count")))
}

/// A pair of counts, `[a, b]`.
fn count_pair(value: &Json) -> Result<(usize, usize), ProbeError> {
    match value.as_array() {
        Some([a, b]) => Ok((count(a)?, count(b)?)),
        _ => Err(ProbeError::BadArgs(format!("{value} is not a pair"))),
    }
}

/// The grid `GridSpec(*shape)` of argument `shape`.
fn grid(args: Args<'_>) -> Result<GridSpec, ProbeError> {
    let (nrows, ncols) = count_pair(args.json("shape")?)?;
    GridSpec::new(nrows, ncols).map_err(ProbeError::rejected)
}

/// The block of `grid` given as `[[row_start, row_stop], [col_start,
/// col_stop]]`.
fn block(grid: &GridSpec, value: &Json) -> Result<SubplotSpec, ProbeError> {
    let range = |pair: &Json| count_pair(pair).map(|(start, end)| Range { start, end });
    match value.as_array() {
        Some([rows, cols]) => grid
            .span(range(rows)?, range(cols)?)
            .map_err(ProbeError::rejected),
        _ => Err(ProbeError::BadArgs(format!("{value} is not a block"))),
    }
}

/// The field of `changes` holding parameter `name`.
fn param_slot<'a>(
    changes: &'a mut SubplotOverrides,
    name: &str,
) -> Result<&'a mut Option<f64>, ProbeError> {
    Ok(match name {
        "left" => &mut changes.left,
        "bottom" => &mut changes.bottom,
        "right" => &mut changes.right,
        "top" => &mut changes.top,
        "wspace" => &mut changes.wspace,
        "hspace" => &mut changes.hspace,
        other => return Err(ProbeError::BadArgs(format!("unknown parameter {other}"))),
    })
}

/// Subplot parameter changes from an object of numbers.
fn overrides(value: &Json) -> Result<SubplotOverrides, ProbeError> {
    let fields = value
        .as_object()
        .ok_or_else(|| ProbeError::BadArgs(format!("{value} is not an object")))?;
    let mut changes = SubplotOverrides::default();
    for (key, value) in fields {
        *param_slot(&mut changes, key)? = Some(
            value
                .as_f64()
                .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a number")))?,
        );
    }
    Ok(changes)
}

/// Optional parameters as recorded.
fn optional_params(o: &SubplotOverrides) -> Json {
    Json::Array(
        [o.left, o.bottom, o.right, o.top, o.wspace, o.hspace]
            .into_iter()
            .map(|value| value.map_or(Json::Null, Json::from))
            .collect(),
    )
}

/// Parameters as recorded.
fn params(p: &SubplotParams) -> Json {
    Json::Array(
        [p.left, p.bottom, p.right, p.top, p.wspace, p.hspace]
            .into_iter()
            .map(Json::from)
            .collect(),
    )
}

/// matplotlib's `SubplotParams(**changes)`: the rcParams' values, then
/// the changes, validated.
fn subplot_params_from(changes: &SubplotOverrides) -> Result<SubplotParams, ProbeError> {
    SubplotParams::from_rc(&RcParams::matplotlib())
        .updated(changes)
        .map_err(ProbeError::rejected)
}

/// `SubplotParams(*params)` of argument `params`.
fn positional_params(args: Args<'_>) -> Result<SubplotParams, ProbeError> {
    let given = args.f64_vec("params")?;
    if given.len() > PARAMS.len() {
        return Err(ProbeError::BadArgs("too many parameters".into()));
    }
    let mut changes = SubplotOverrides::default();
    for (name, value) in PARAMS.into_iter().zip(given) {
        *param_slot(&mut changes, name)? = Some(value);
    }
    subplot_params_from(&changes)
}

fn subplotspec_eq(args: Args<'_>) -> Outcome {
    let grid = grid(args)?;
    let first = block(&grid, args.json("first")?)?;
    let second = block(&grid, args.json("second")?)?;
    Ok(Json::from(first == second))
}

fn gridspec_update(args: Args<'_>) -> Outcome {
    let grid = grid(args)?;
    let fig = Figure::new();
    grid.update(&fig, &overrides(args.json("changes")?)?)
        .map_err(ProbeError::rejected)?;
    Ok(optional_params(&grid.overrides()))
}

fn gridspec_repr(args: Args<'_>) -> Outcome {
    let nrows = count(args.json("nrows")?)?;
    let ncols = count(args.json("ncols")?)?;
    let ratios = |key| args.get(key).map(|_| args.f64_vec(key)).transpose();
    let grid = GridSpec::with_ratios(
        nrows,
        ncols,
        ratios("width_ratios")?,
        ratios("height_ratios")?,
        SubplotOverrides::default(),
    )
    .map_err(ProbeError::rejected)?;
    Ok(Json::String(grid.to_string()))
}

fn subplotspec_repr(args: Args<'_>) -> Outcome {
    let grid = grid(args)?;
    Ok(Json::String(block(&grid, args.json("block")?)?.to_string()))
}

fn subplot_params(args: Args<'_>) -> Outcome {
    let mut changes = SubplotOverrides::default();
    for name in PARAMS {
        *param_slot(&mut changes, name)? = args.opt_f64(name)?;
    }
    Ok(params(&subplot_params_from(&changes)?))
}

fn subplot_params_update(args: Args<'_>) -> Outcome {
    let updated = positional_params(args)?
        .updated(&overrides(args.json("changes")?)?)
        .map_err(ProbeError::rejected)?;
    Ok(params(&updated))
}

/// matplotlib's `reset` brings back the rcParams' values, which
/// `SubplotParams::from_rc` gives.
fn subplot_params_reset(args: Args<'_>) -> Outcome {
    positional_params(args)?;
    Ok(params(&SubplotParams::from_rc(&RcParams::matplotlib())))
}

fn topmost_subplotspec(args: Args<'_>) -> Outcome {
    let (nrows, ncols) = count_pair(args.json("shape")?)?;
    let (inner_rows, inner_cols) = count_pair(args.json("nested")?)?;
    let fig = Figure::new();
    let axes = fig.subplots(nrows, ncols).map_err(ProbeError::rejected)?;
    let ax = axes
        .as_slice()
        .get(count(args.json("index")?)?)
        .ok_or_else(|| ProbeError::BadArgs("no such axes".into()))?;
    let spec = ax
        .subplot_spec()
        .ok_or_else(|| ProbeError::rejected("the axes has no subplot spec"))?;
    let nested = GridSpec::nested(&spec, inner_rows, inner_cols, None, None, (None, None))
        .map_err(ProbeError::rejected)?;
    let topmost = nested
        .topmost_subplotspec()
        .ok_or_else(|| ProbeError::rejected("a nested grid has a topmost block"))?;
    Ok(Json::Array(vec![
        Json::String(topmost.to_string()),
        Json::from(topmost == spec),
    ]))
}
