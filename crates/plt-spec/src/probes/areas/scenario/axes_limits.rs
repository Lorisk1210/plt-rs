//! Axes steps about view and data limits: explicit limits and bounds,
//! autoscaling, margins, sticky edges and `relim`.

use plt::axes::{Axes, AxisMode, AxisName, Limits};

use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Produced};
use crate::json::Json;
use crate::probes::ProbeError;

/// Runs a limits method of `ax`.
pub(super) fn call(ax: &Axes, call: &Call<'_>) -> Handled {
    use AxisName::{X, Y};
    Some(match call.method {
        "get_xlim" => Ok(Produced::pair(ax.xlim())),
        "get_ylim" => Ok(Produced::pair(ax.ylim())),
        "set_xlim" => set_lim(ax, X, call, ["left", "right", "xmin", "xmax"]),
        "set_ylim" => set_lim(ax, Y, call, ["bottom", "top", "ymin", "ymax"]),
        "get_xbound" => Ok(Produced::pair(ax.bound(X))),
        "get_ybound" => Ok(Produced::pair(ax.bound(Y))),
        "set_xbound" => set_bound(ax, X, call),
        "set_ybound" => set_bound(ax, Y, call),
        "invert_xaxis" => invert(ax, X),
        "invert_yaxis" => invert(ax, Y),
        "get_autoscalex_on" => Ok(Produced::Value(ax.autoscale_on(Some(X)).into())),
        "get_autoscaley_on" => Ok(Produced::Value(ax.autoscale_on(Some(Y)).into())),
        "get_autoscale_on" => Ok(Produced::Value(ax.autoscale_on(None).into())),
        "set_autoscalex_on" => set_autoscale_on(ax, X, call),
        "set_autoscaley_on" => set_autoscale_on(ax, Y, call),
        "autoscale" => autoscale(ax, call),
        "autoscale_view" => call.opt_bool(0, "tight").map(|tight| {
            ax.autoscale_view(tight);
            Produced::Nothing
        }),
        "use_sticky_edges" => use_sticky_edges(ax, call),
        "margins" => margins(ax, call),
        "get_xmargin" => Ok(Produced::Value(ax.margin(X).into())),
        "get_ymargin" => Ok(Produced::Value(ax.margin(Y).into())),
        "set_xmargin" => set_margin(ax, X, call),
        "set_ymargin" => set_margin(ax, Y, call),
        "relim" => call.opt_bool(0, "visible_only").map(|visible_only| {
            ax.relim(visible_only.unwrap_or(false));
            Produced::Nothing
        }),
        "dataLim" => Ok(Produced::Value(bbox(ax.data_lim().bbox))),
        "dataLim.minpos" => {
            let minpos = ax.data_lim().minpos;
            Ok(Produced::numbers(vec![minpos.x, minpos.y]))
        }
        "viewLim" => {
            let ((x0, x1), (y0, y1)) = (ax.xlim(), ax.ylim());
            Ok(Produced::Value(Json::from(vec![
                vec![x0, y0],
                vec![x1, y1],
            ])))
        }
        "axis" => axis(ax, call),
        _ => return None,
    })
}

/// `[[x0, y0], [x1, y1]]`.
pub(super) fn bbox(b: plt_core::Bbox) -> Json {
    Json::from(vec![vec![b.x0(), b.y0()], vec![b.x1(), b.y1()]])
}

/// matplotlib's `set_xlim(left=None, right=None, *, emit=True,
/// auto=False, xmin=None, xmax=None)`; `left` may hold both limits.
fn set_lim(
    ax: &Axes,
    axis: AxisName,
    call: &Call<'_>,
    [low_name, high_name, min_name, max_name]: [&str; 4],
) -> Result<Produced, ProbeError> {
    let mut low = call.arg(0, low_name);
    let mut high = call.arg(1, high_name);
    if let (Some(Json::Array(both)), None) = (low, high) {
        let [first, second] = both.as_slice() else {
            return Err(ProbeError::rejected("limits need two values"));
        };
        (low, high) = (Some(first), Some(second));
    }
    for (value, alias) in [(&mut low, min_name), (&mut high, max_name)] {
        if let Some(aliased) = call.keyword(alias) {
            if value.is_some() {
                return Err(ProbeError::rejected(format!("cannot pass {alias} twice")));
            }
            *value = Some(aliased);
        }
    }
    let limit = |value: Option<&Json>| {
        value
            .filter(|v| !v.is_null())
            .map(|v| v.as_f64().ok_or_else(|| call.bad("limits are numbers")))
            .transpose()
    };
    let auto = if call.has_keyword("auto") {
        call.opt_bool(KEYWORD_ONLY, "auto")?
    } else {
        Some(false)
    };
    let limits = Limits {
        low: limit(low)?,
        high: limit(high)?,
        auto,
    };
    ax.set_limits_with(axis, limits)
        .map_err(ProbeError::rejected)?;
    Ok(Produced::Nothing)
}

/// matplotlib's `set_xbound(lower=None, upper=None)`.
fn set_bound(ax: &Axes, axis: AxisName, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let lower = call.opt_f64(0, "lower")?;
    let upper = call.opt_f64(1, "upper")?;
    ax.set_bound(axis, lower, upper)
        .map_err(ProbeError::rejected)?;
    Ok(Produced::Nothing)
}

fn invert(ax: &Axes, axis: AxisName) -> Result<Produced, ProbeError> {
    let axis = ax.axis_of(axis);
    axis.set_inverted(!axis.inverted());
    Ok(Produced::Nothing)
}

fn set_autoscale_on(ax: &Axes, axis: AxisName, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let on = call
        .opt_bool(0, "b")?
        .ok_or_else(|| call.bad("missing switch"))?;
    ax.set_axis_autoscale_on(axis, on);
    Ok(Produced::Nothing)
}

/// matplotlib's `autoscale(enable=True, axis='both', tight=None)`.
fn autoscale(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let enable = if call.arity() > 0 || call.has_keyword("enable") {
        call.opt_bool(0, "enable")?
    } else {
        Some(true)
    };
    let axis = match call.opt_str(1, "axis")? {
        None | Some("both") => None,
        Some("x") => Some(AxisName::X),
        Some("y") => Some(AxisName::Y),
        Some(other) => return Err(ProbeError::rejected(format!("unknown axis {other:?}"))),
    };
    ax.autoscale_with(enable, axis, call.opt_bool(2, "tight")?);
    Ok(Produced::Nothing)
}

fn use_sticky_edges(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    Ok(match call.opt_bool(0, "value")? {
        Some(on) => {
            ax.set_use_sticky_edges(on);
            Produced::Nothing
        }
        None => Produced::Value(ax.use_sticky_edges().into()),
    })
}

/// matplotlib's `margins(*margins, x=None, y=None, tight=True)`.
fn margins(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let (x, y) = match call.arity() {
        0 => (
            call.opt_f64(KEYWORD_ONLY, "x")?,
            call.opt_f64(KEYWORD_ONLY, "y")?,
        ),
        1 => {
            let both = call.f64(0, "margins")?;
            (Some(both), Some(both))
        }
        2 => (Some(call.f64(0, "x")?), Some(call.f64(1, "y")?)),
        _ => return Err(call.bad("too many margins")),
    };
    if x.is_none() && y.is_none() {
        // Unprefixed queries are not recorded; scenarios use get_xmargin.
        return Err(call.bad("query margins with get_xmargin and get_ymargin"));
    }
    ax.set_margins(x, y).map_err(ProbeError::rejected)?;
    Ok(Produced::Nothing)
}

fn set_margin(ax: &Axes, axis: AxisName, call: &Call<'_>) -> Result<Produced, ProbeError> {
    ax.set_margin(axis, call.f64(0, "m")?)
        .map_err(ProbeError::rejected)?;
    Ok(Produced::Nothing)
}

/// matplotlib's `axis(arg)`: a mode string, `[xmin, xmax, ymin, ymax]`, or
/// the limits by keyword.
fn axis(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let limits = match call.arg(0, "arg") {
        Some(Json::String(mode)) => {
            ax.axis_mode(axis_mode(mode)?)
                .map_err(ProbeError::rejected)?;
            return Ok(Produced::Nothing);
        }
        Some(extent) => {
            let values = extent
                .as_f64_vec()
                .filter(|v| v.len() == 4)
                .ok_or_else(|| ProbeError::rejected("axis() needs four limits"))?;
            values.into_iter().map(Some).collect::<Vec<_>>()
        }
        None => ["xmin", "xmax", "ymin", "ymax"]
            .into_iter()
            .map(|name| call.opt_f64(KEYWORD_ONLY, name))
            .collect::<Result<_, _>>()?,
    };
    for (axis, pair) in [AxisName::X, AxisName::Y].into_iter().zip(limits.chunks(2)) {
        ax.set_limits_with(
            axis,
            Limits {
                low: pair[0],
                high: pair[1],
                auto: Some(false),
            },
        )
        .map_err(ProbeError::rejected)?;
    }
    Ok(Produced::Nothing)
}

/// The preset matplotlib's `axis` names `mode`.
pub(crate) fn axis_mode(mode: &str) -> Result<AxisMode, ProbeError> {
    Ok(match mode {
        "on" => AxisMode::On,
        "off" => AxisMode::Off,
        "equal" => AxisMode::Equal,
        "scaled" => AxisMode::Scaled,
        "tight" => AxisMode::Tight,
        "auto" => AxisMode::Auto,
        "image" => AxisMode::Image,
        "square" => AxisMode::Square,
        other => return Err(ProbeError::rejected(format!("unknown axis mode {other:?}"))),
    })
}
