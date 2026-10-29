//! Steps acting on the figure: adding, listing and clearing axes, the
//! subplot parameters and drawing.

use pltrs::axes::{Axes, ProjectionKind};
use pltrs::figure::Share;
use pltrs::gridspec::{GridSpec, SubplotOverrides, SubplotSpec};

use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Object, Produced, Scene};
use crate::json::Json;
use crate::probes::ProbeError;

/// Runs a figure method.
pub(super) fn call(scene: &Scene, call: &Call<'_>) -> Handled {
    let fig = &scene.fig;
    Some(match call.method {
        "subplots" => subplots(scene, call),
        "gca" => Ok(Produced::Object(Object::Axes(fig.gca()))),
        "add_subplot" => add_subplot(scene, call),
        "add_axes" => rect(call).and_then(|rect| {
            let ax = fig.add_axes(rect).map_err(ProbeError::rejected)?;
            share_from_keywords(scene, &ax, call)?;
            Ok(Produced::Object(Object::Axes(ax)))
        }),
        "get_axes" => Ok(Produced::Value(Json::Array(
            fig.axes()
                .iter()
                .filter_map(|ax| scene.number(ax))
                .map(|n| Json::from(n as f64))
                .collect(),
        ))),
        "clear" | "clf" => {
            fig.clear();
            Ok(Produced::Nothing)
        }
        "subplots_adjust" => subplots_adjust(scene, call),
        param if param.starts_with("subplotpars.") => subplot_param(scene, call),
        "draw" => fig
            .render_raster()
            .map(|_| Produced::Nothing)
            .map_err(ProbeError::rejected),
        _ => return None,
    })
}

/// matplotlib's `fig.subplots(nrows, ncols, sharex=..., sharey=...,
/// width_ratios=..., height_ratios=...)`.
fn subplots(scene: &Scene, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let count = |index, keyword| {
        call.opt_f64(index, keyword)?.map_or(Ok(1), |n| {
            grid_count(n).ok_or_else(|| call.bad("bad grid size"))
        })
    };
    let (nrows, ncols) = (count(0, "nrows")?, count(1, "ncols")?);
    let share = |keyword| match call.keyword(keyword) {
        None | Some(Json::Bool(false)) => Ok(Share::None),
        Some(Json::Bool(true)) => Ok(Share::All),
        Some(Json::String(mode)) => match mode.as_str() {
            "all" => Ok(Share::All),
            "row" => Ok(Share::Row),
            "col" => Ok(Share::Col),
            "none" => Ok(Share::None),
            _ => Err(ProbeError::rejected(format!(
                "{mode:?} is not a sharing mode"
            ))),
        },
        Some(other) => Err(call.bad(&format!("bad {keyword} {other}"))),
    };
    let ratios = |keyword| {
        call.keyword(keyword)
            .map(|_| call.f64_vec(KEYWORD_ONLY, keyword))
            .transpose()
    };
    let grid = GridSpec::with_ratios(
        nrows,
        ncols,
        ratios("width_ratios")?,
        ratios("height_ratios")?,
        SubplotOverrides::default(),
    )
    .map_err(ProbeError::rejected)?;
    let axes = scene
        .fig
        .subplots_on(&grid, share("sharex")?, share("sharey")?)
        .map_err(ProbeError::rejected)?;
    Ok(match axes.as_slice() {
        [single] => Produced::Object(Object::Axes(single.clone())),
        _ => Produced::Nothing,
    })
}

fn grid_count(n: f64) -> Option<usize> {
    (n.fract() == 0.0 && n >= 1.0).then_some(n as usize)
}

/// matplotlib's `fig.add_subplot()`, `add_subplot(211)`,
/// `add_subplot(2, 1, 1)`, `add_subplot(3, 2, (3, 5))` and
/// `add_subplot(gs[...])`, with the `projection` named and sharing axes
/// given by keyword.
fn add_subplot(scene: &Scene, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let fig = &scene.fig;
    let spec = match call.arity() {
        0 => None,
        1 if call.constructed(0, "spec").is_some() => Some(subplot_spec(call)?),
        1 => {
            let code = call.f64(0, "code")?;
            let digits = grid_count(code)
                .filter(|code| (111..=999).contains(code))
                .ok_or_else(|| ProbeError::rejected("not a three-digit subplot code"))?;
            Some(numbered(digits / 100, digits / 10 % 10, digits % 10)?)
        }
        3 => Some(numbered_cell(call)?),
        _ => return Err(call.bad("unsupported subplot arguments")),
    };
    let projection = call
        .opt_str(KEYWORD_ONLY, "projection")?
        .map(|name| {
            ProjectionKind::from_name(name)
                .ok_or_else(|| call.bad(&format!("unknown projection {name:?}")))
        })
        .transpose()?;
    let ax = match (spec, projection) {
        (None, None) => fig.subplot(),
        (Some(spec), None) => fig.add_subplot_spec(&spec).map_err(ProbeError::rejected)?,
        (None, Some(kind)) => fig
            .subplot_with_projection(kind)
            .map_err(ProbeError::rejected)?,
        (Some(spec), Some(kind)) => fig
            .add_subplot_spec_with_projection(&spec, kind)
            .map_err(ProbeError::rejected)?,
    };
    share_from_keywords(scene, &ax, call)?;
    Ok(Produced::Object(Object::Axes(ax)))
}

fn numbered(nrows: usize, ncols: usize, index: usize) -> Result<SubplotSpec, ProbeError> {
    GridSpec::new(nrows, ncols)
        .and_then(|grid| grid.numbered(index))
        .map_err(ProbeError::rejected)
}

/// `(nrows, ncols, index)` or `(nrows, ncols, (first, last))`.
fn numbered_cell(call: &Call<'_>) -> Result<SubplotSpec, ProbeError> {
    let whole = |value: f64| grid_count(value).ok_or_else(|| ProbeError::rejected("not a count"));
    let (nrows, ncols) = (whole(call.f64(0, "nrows")?)?, whole(call.f64(1, "ncols")?)?);
    let Some(("tuple", [first, last])) = call.constructed(2, "index") else {
        return numbered(nrows, ncols, whole(call.f64(2, "index")?)?);
    };
    let cell = |value: &Json| {
        value
            .as_f64()
            .ok_or_else(|| call.bad("bad cell"))
            .and_then(whole)
    };
    let (first, last) = (cell(first)?, cell(last)?);
    GridSpec::new(nrows, ncols)
        .and_then(|grid| grid.numbered_span(first, last))
        .map_err(ProbeError::rejected)
}

/// `{"new": "SubplotSpec", "args": [nrows, ncols, [row0, row1], [col0,
/// col1]]}`: `GridSpec(nrows, ncols)[row0:row1, col0:col1]`.
fn subplot_spec(call: &Call<'_>) -> Result<SubplotSpec, ProbeError> {
    let Some(("SubplotSpec", [nrows, ncols, rows, cols])) = call.constructed(0, "spec") else {
        return Err(call.bad("expected a SubplotSpec"));
    };
    let count = |v: &Json| {
        v.as_f64()
            .and_then(grid_count)
            .ok_or_else(|| call.bad("bad grid"))
    };
    let range = |v: &Json| match v.as_f64_vec().as_deref() {
        Some(&[start, end]) if start >= 0.0 && end >= start => Ok(start as usize..end as usize),
        _ => Err(call.bad("bad span")),
    };
    let (rows, cols) = (range(rows)?, range(cols)?);
    GridSpec::new(count(nrows)?, count(ncols)?)
        .and_then(|grid| grid.span(rows, cols))
        .map_err(ProbeError::rejected)
}

/// Applies `sharex=`/`sharey=` keywords naming other axes.
fn share_from_keywords(scene: &Scene, ax: &Axes, call: &Call<'_>) -> Result<(), ProbeError> {
    if let Some(other) = call.keyword("sharex") {
        ax.share_x(&scene.axes_argument(other)?)
            .map_err(ProbeError::rejected)?;
    }
    if let Some(other) = call.keyword("sharey") {
        ax.share_y(&scene.axes_argument(other)?)
            .map_err(ProbeError::rejected)?;
    }
    Ok(())
}

/// A `[left, bottom, width, height]` rectangle.
pub(super) fn rect(call: &Call<'_>) -> Result<[f64; 4], ProbeError> {
    call.f64_vec(0, "rect")?
        .try_into()
        .map_err(|_| call.bad("a rectangle has four numbers"))
}

fn subplots_adjust(scene: &Scene, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let overrides = SubplotOverrides {
        left: call.opt_f64(0, "left")?,
        bottom: call.opt_f64(1, "bottom")?,
        right: call.opt_f64(2, "right")?,
        top: call.opt_f64(3, "top")?,
        wspace: call.opt_f64(4, "wspace")?,
        hspace: call.opt_f64(5, "hspace")?,
    };
    scene
        .fig
        .subplots_adjust(overrides)
        .map_err(ProbeError::rejected)?;
    Ok(Produced::Nothing)
}

/// `fig.subplotpars.<name>`.
fn subplot_param(scene: &Scene, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let params = scene.fig.subplot_params();
    let value = match call.method.trim_start_matches("subplotpars.") {
        "left" => params.left,
        "right" => params.right,
        "bottom" => params.bottom,
        "top" => params.top,
        "wspace" => params.wspace,
        "hspace" => params.hspace,
        _ => return Err(call.bad("unknown subplot parameter")),
    };
    Ok(Produced::Value(value.into()))
}
