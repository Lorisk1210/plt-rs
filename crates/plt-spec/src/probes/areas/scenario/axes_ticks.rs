//! Axes steps about scales and ticks.

use plt::axes::{Axes, Axis, AxisName, LocatorParams};
use plt::dates::TimeZone;
use plt::scale::{ScaleOptions, scale_factory};
use plt::ticker::{MultipleLocator, Nbins};

use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Produced};
use crate::json::Json;
use crate::probes::ProbeError;

/// Runs a scale or tick method of `ax`.
pub(super) fn call(ax: &Axes, call: &Call<'_>) -> Handled {
    use AxisName::{X, Y};
    let method = call.method;
    Some(match method {
        "get_xscale" | "xaxis.get_scale" => Ok(Produced::Value(ax.xscale().to_string().into())),
        "get_yscale" | "yaxis.get_scale" => Ok(Produced::Value(ax.yscale().to_string().into())),
        "set_xscale" => set_scale(ax, X, call),
        "set_yscale" => set_scale(ax, Y, call),
        "get_xticks" | "get_yticks" | "get_xticklabels" | "get_yticklabels" => {
            let axis = ax.axis_of(if method.starts_with("get_x") { X } else { Y });
            call.opt_bool(KEYWORD_ONLY, "minor").and_then(|minor| {
                let minor = minor.unwrap_or(false);
                Ok(if method.ends_with("labels") {
                    let labels = axis.ticklabels(minor).map_err(ProbeError::rejected)?;
                    Produced::Value(Json::from(labels))
                } else {
                    Produced::numbers(axis.ticklocs(minor).map_err(ProbeError::rejected)?)
                })
            })
        }
        "xaxis.get_ticklocs"
        | "yaxis.get_ticklocs"
        | "xaxis.get_majorticklocs"
        | "yaxis.get_majorticklocs"
        | "xaxis.get_minorticklocs"
        | "yaxis.get_minorticklocs" => {
            let axis = ax.axis_of(if method.starts_with('x') { X } else { Y });
            let minor = match method.split_once(".get_") {
                Some((_, "minorticklocs")) => Ok(true),
                Some((_, "ticklocs")) => call
                    .opt_bool(KEYWORD_ONLY, "minor")
                    .map(Option::unwrap_or_default),
                _ => Ok(false),
            };
            minor.and_then(|minor| {
                axis.ticklocs(minor)
                    .map(Produced::numbers)
                    .map_err(ProbeError::rejected)
            })
        }
        "xaxis.get_ticklabels" | "yaxis.get_ticklabels" => ticklabels(
            &ax.axis_of(if method.starts_with('x') { X } else { Y }),
            call,
        ),
        "set_xticks" | "set_yticks" | "xaxis.set_ticks" | "yaxis.set_ticks" => {
            let axis = ax.axis_of(if method.contains('x') { X } else { Y });
            call.f64_vec(0, "ticks").and_then(|ticks| {
                if call.opt_bool(KEYWORD_ONLY, "minor")? == Some(true) {
                    axis.set_minor_ticks(ticks);
                } else {
                    axis.set_ticks(ticks);
                }
                Ok(Produced::Nothing)
            })
        }
        "xaxis.set_ticklabels" | "yaxis.set_ticklabels" => {
            let axis = ax.axis_of(if method.starts_with('x') { X } else { Y });
            labels(call).and_then(|labels| {
                axis.set_ticklabels(labels).map_err(ProbeError::rejected)?;
                Ok(Produced::Nothing)
            })
        }
        "xaxis.set_major_locator" | "yaxis.set_major_locator" => {
            let axis = ax.axis_of(if method.starts_with('x') { X } else { Y });
            multiple_locator(call).map(|locator| {
                axis.set_major_locator(locator);
                Produced::Nothing
            })
        }
        "xaxis.get_remove_overlapping_locs" | "yaxis.get_remove_overlapping_locs" => {
            let axis = ax.axis_of(if method.starts_with('x') { X } else { Y });
            Ok(Produced::Value(axis.remove_overlapping_locs().into()))
        }
        "xaxis.set_remove_overlapping_locs" | "yaxis.set_remove_overlapping_locs" => {
            let axis = ax.axis_of(if method.starts_with('x') { X } else { Y });
            call.opt_bool(0, "val").and_then(|remove| {
                let remove = remove.ok_or_else(|| call.bad("missing val"))?;
                axis.set_remove_overlapping_locs(remove);
                Ok(Produced::Nothing)
            })
        }
        "xaxis_date" | "yaxis_date" => {
            ax.axis_of(if method.starts_with('x') { X } else { Y })
                .axis_date(&TimeZone::Utc);
            Ok(Produced::Nothing)
        }
        "locator_params" => locator_params(ax, call),
        "minorticks_on" => {
            ax.minorticks_on();
            Ok(Produced::Nothing)
        }
        _ => return None,
    })
}

/// matplotlib's `Axis.get_ticklabels(which=...)`: the major labels, the
/// minor ones, or both after each other.
fn ticklabels(axis: &Axis, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let levels = match call.opt_str(KEYWORD_ONLY, "which")? {
        None | Some("major") => &[false][..],
        Some("minor") => &[true],
        Some("both") => &[false, true],
        Some(other) => return Err(ProbeError::rejected(format!("bad which {other:?}"))),
    };
    levels
        .iter()
        .map(|&minor| axis.ticklabels(minor).map_err(ProbeError::rejected))
        .collect::<Result<Vec<_>, _>>()
        .map(|labels| Produced::Value(Json::from(labels.concat())))
}

/// matplotlib's `set_xscale(name, base=...)`.
fn set_scale(ax: &Axes, axis: AxisName, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let options = ScaleOptions {
        base: call.opt_f64(KEYWORD_ONLY, "base")?,
        ..ScaleOptions::default()
    };
    let scale = scale_factory(call.str(0, "value")?, options).map_err(ProbeError::rejected)?;
    ax.set_scale(axis, scale);
    Ok(Produced::Nothing)
}

fn labels(call: &Call<'_>) -> Result<Vec<String>, ProbeError> {
    call.required(0, "labels")?
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| call.bad("labels are strings"))
}

/// `{"new": "MultipleLocator", "args": [base]}`.
fn multiple_locator(call: &Call<'_>) -> Result<MultipleLocator, ProbeError> {
    match call.constructed(0, "locator") {
        Some(("MultipleLocator", [base])) => base
            .as_f64()
            .ok_or_else(|| call.bad("bad base"))
            .and_then(|base| MultipleLocator::new(base).map_err(ProbeError::rejected)),
        _ => Err(call.bad("only MultipleLocator(base) is supported")),
    }
}

/// matplotlib's `locator_params(axis='both', tight=None, nbins=...)`.
fn locator_params(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let axis = match call.opt_str(0, "axis")? {
        None | Some("both") => None,
        Some("x") => Some(AxisName::X),
        Some("y") => Some(AxisName::Y),
        Some(other) => return Err(ProbeError::rejected(format!("bad axis {other:?}"))),
    };
    let nbins = match call.keyword("nbins") {
        None => None,
        Some(Json::String(auto)) if auto == "auto" => Some(Nbins::Auto),
        Some(n) => Some(Nbins::Fixed(
            n.as_i64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| call.bad("bad nbins"))?,
        )),
    };
    let params = LocatorParams {
        axis,
        nbins,
        ..LocatorParams::default()
    };
    ax.locator_params(&params).map_err(ProbeError::rejected)?;
    Ok(Produced::Nothing)
}
