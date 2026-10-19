//! Axes steps that plot lines, bars, spans and patches: the artists whose
//! data limits and sticky edges the scenarios autoscale around.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, BarLabelOptions, BarLabelType, BarOptions, ErrorSize, ErrorbarOptions};
use plt::lines::LineProps;
use plt::patches::{FancyArrow, PathShape};
use plt::units::AxisValues;
use plt_core::{Path, PathCode};

use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Object, Produced, Scene};
use crate::json::Json;
use crate::probes::ProbeError;

/// Runs a plotting method of `ax`.
pub(super) fn call(scene: &Scene, ax: &Axes, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "plot" | "loglog" | "semilogx" | "semilogy" => plot(ax, call),
        "bar" | "barh" => bars(ax, call),
        "bar_label" => bar_label(scene, ax, call),
        "axvline" | "axhline" => call.f64(0, "pos").map(|pos| {
            let line = if call.method == "axvline" {
                ax.axvline(pos, LineProps::default())
            } else {
                ax.axhline(pos, LineProps::default())
            };
            artist(&line)
        }),
        "axvspan" | "axhspan" => {
            let span = (call.f64(0, "min"), call.f64(1, "max"));
            span.0.and_then(|lo| {
                let hi = span.1?;
                Ok(artist(&if call.method == "axvspan" {
                    ax.axvspan(lo, hi)
                } else {
                    ax.axhspan(lo, hi)
                }))
            })
        }
        "errorbar" => errorbar(ax, call),
        "arrow" => (0..4)
            .map(|i| call.f64(i, ""))
            .collect::<Result<Vec<_>, _>>()
            .map(|v| artist(&ax.arrow(FancyArrow::new(v[0], v[1], v[2], v[3])))),
        "add_patch" => add_patch(ax, call),
        _ => return None,
    })
}

/// An artist for later steps.
pub(super) fn artist(handle: &impl ArtistHandle) -> Produced {
    Produced::Object(Object::Artist(handle.raw().clone()))
}

/// Data for an axis: a number, a string (a category) or a list of either.
pub(super) fn axis_values(
    call: &Call<'_>,
    index: usize,
    keyword: &str,
) -> Result<AxisValues, ProbeError> {
    let value = call.required(index, keyword)?;
    let items = value.as_array().unwrap_or(std::slice::from_ref(value));
    if let Some(numbers) = items.iter().map(Json::as_f64).collect::<Option<Vec<_>>>() {
        return Ok(numbers.into());
    }
    items
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect::<Option<Vec<String>>>()
        .map(AxisValues::from)
        .ok_or_else(|| call.bad(&format!("{keyword} is not data")))
}

/// The `x` and `y` of a plotting call.
pub(super) fn xy(call: &Call<'_>) -> Result<(AxisValues, AxisValues), ProbeError> {
    Ok((axis_values(call, 0, "x")?, axis_values(call, 1, "y")?))
}

/// matplotlib's `plot([x], y, [fmt])` and its log-scaled variants.
fn plot(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let format_at = |index| match call.arg(index, "fmt") {
        Some(Json::String(fmt)) => Some(fmt.as_str()),
        _ => None,
    };
    let (x, y, fmt) = match call.arity() {
        1 => (None, axis_values(call, 0, "y")?, None),
        2 if format_at(1).is_some() => (None, axis_values(call, 0, "y")?, format_at(1)),
        2 => (
            Some(axis_values(call, 0, "x")?),
            axis_values(call, 1, "y")?,
            None,
        ),
        3 => (
            Some(axis_values(call, 0, "x")?),
            axis_values(call, 1, "y")?,
            format_at(2),
        ),
        _ => return Err(call.bad("unsupported plot arguments")),
    };
    let x = x.unwrap_or_else(|| (0..y.len()).map(|i| i as f64).collect::<Vec<_>>().into());
    let mut props = fmt
        .map(LineProps::fmt)
        .transpose()
        .map_err(ProbeError::rejected)?
        .unwrap_or_default();
    if let Some(label) = call.opt_str(KEYWORD_ONLY, "label")? {
        props = props.label(label);
    }
    if let Some(color) = call.opt_str(KEYWORD_ONLY, "color")? {
        props = props.color(color);
    }
    if let Some(color) = call.opt_str(KEYWORD_ONLY, "markeredgecolor")? {
        props = props.markeredgecolor(color);
    }
    if let Some(color) = call.opt_str(KEYWORD_ONLY, "markerfacecolor")? {
        props = props.markerfacecolor(color);
    }
    match call.method {
        "loglog" => scales(ax, true, true)?,
        "semilogx" => scales(ax, true, false)?,
        "semilogy" => scales(ax, false, true)?,
        _ => {}
    }
    ax.plot_with(x, y, props)
        .map(|line| artist(&line))
        .map_err(ProbeError::rejected)
}

fn scales(ax: &Axes, x: bool, y: bool) -> Result<(), ProbeError> {
    if x {
        ax.set_xscale("log").map_err(ProbeError::rejected)?;
    }
    if y {
        ax.set_yscale("log").map_err(ProbeError::rejected)?;
    }
    Ok(())
}

/// matplotlib's `errorbar(x, y, yerr=..., xerr=...)` with symmetric
/// errors.
fn errorbar(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let size = |keyword| {
        call.keyword(keyword)
            .map(|_| {
                call.f64_vec(KEYWORD_ONLY, keyword)
                    .map(ErrorSize::Symmetric)
            })
            .transpose()
    };
    let options = ErrorbarOptions {
        xerr: size("xerr")?,
        yerr: size("yerr")?,
        ..ErrorbarOptions::default()
    };
    ax.errorbar(call.f64_vec(0, "x")?, call.f64_vec(1, "y")?, options)
        .map(|_| Produced::Nothing)
        .map_err(ProbeError::rejected)
}

/// matplotlib's `bar(x, height, ...)`/`barh(y, width, left=..., ...)`.
fn bars(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let at = axis_values(call, 0, "x")?;
    let size = match call.required(1, "height")? {
        Json::Array(_) => call.f64_vec(1, "height")?,
        _ => vec![call.f64(1, "height")?],
    };
    let base_keyword = if call.method == "bar" {
        "bottom"
    } else {
        "left"
    };
    let mut options = BarOptions {
        label: call.opt_str(KEYWORD_ONLY, "label")?.map(str::to_owned),
        ..BarOptions::default()
    };
    if let Some(base) = call.opt_f64(KEYWORD_ONLY, base_keyword)? {
        options.base = vec![base];
    }
    let made = if call.method == "bar" {
        ax.bar(at, size, options)
    } else {
        ax.barh(at, size, options)
    };
    made.map(|bars| Produced::Object(Object::Bars(bars)))
        .map_err(ProbeError::rejected)
}

fn bar_label(scene: &Scene, ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let reference = call.str(0, "container")?;
    let Object::Bars(bars) = scene.reference(reference)? else {
        return Err(call.bad("not a bar container"));
    };
    let label_type = match call.opt_str(KEYWORD_ONLY, "label_type")? {
        None | Some("edge") => BarLabelType::Edge,
        Some("center") => BarLabelType::Center,
        Some(other) => return Err(ProbeError::rejected(format!("bad label_type {other:?}"))),
    };
    let options = BarLabelOptions {
        label_type,
        ..BarLabelOptions::default()
    };
    ax.bar_label(&bars, &options)
        .map(|_| Produced::Nothing)
        .map_err(ProbeError::rejected)
}

/// `add_patch(PathPatch(Path(vertices, codes)))`.
fn add_patch(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let Some(("PathPatch", [vertices, codes])) = call.constructed(0, "p") else {
        return Err(call.bad("only PathPatch(vertices, codes) is supported"));
    };
    let vertices = crate::probes::encode::parse_points(vertices)?;
    let codes = codes
        .as_f64_vec()
        .ok_or_else(|| call.bad("bad path codes"))?
        .into_iter()
        .map(|code| PathCode::try_from(code as u8).map_err(ProbeError::rejected))
        .collect::<Result<Vec<_>, _>>()?;
    let path = Path::new(vertices, codes).map_err(ProbeError::rejected)?;
    Ok(artist(&ax.add_patch(PathShape(path))))
}
