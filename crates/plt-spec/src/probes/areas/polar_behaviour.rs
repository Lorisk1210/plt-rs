//! Probe operations for the `polar_behaviour` area: polar limits, θ
//! limits, θ ticks, tight bounding boxes and the cursor readout, ported
//! from matplotlib's `test_polar.py` (see
//! `spec/reference/probes/test_polar_behaviour.py`). Every op runs under
//! the reference configuration.

use plt::figure::Figure;
use plt::gridspec::GridSpec;
use plt::lines::LineProps;
use plt::navigation::Navigation;
use plt::polar::{Compass, PolarAxes, ThetaDirection};
use plt::ticker::AxisName;

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{bbox, object, parse_points};
use crate::probes::{Args, Op, Outcome, ProbeError, made};

/// The θ offset of `test_polar_outer_spine_not_collapsed`: π/2 rounded to
/// nine decimals, which once left the outer spine just short of a turn.
#[expect(
    clippy::approx_constant,
    reason = "the upstream test's value, deliberately not exactly π/2"
)]
const NEARLY_QUARTER_TURN: f64 = 1.570_796_327;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("polar.limits", |args| with_rc(args, || limits(args))),
    Op::new("polar.thetalim", |args| with_rc(args, || thetalim(args))),
    Op::new("polar.ticks", |args| with_rc(args, || ticks(args))),
    Op::new("polar.tightbbox", |args| with_rc(args, || tightbbox(args))),
    Op::new("polar.cursor", |args| with_rc(args, || cursor(args))),
];

fn limits_of(polar: &PolarAxes) -> Json {
    let (t0, t1) = polar.axes().xlim();
    let (r0, r1) = polar.axes().ylim();
    object([
        ("thetalim", Json::from(vec![t0, t1])),
        ("rlim", Json::from(vec![r0, r1])),
    ])
}

/// The θ and r limits after each step of a scene.
fn limits(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let polar = fig.polar_subplot();
    let ax = polar.axes();
    let plot_y = |y: &[f64]| ax.plot_y(y);
    match args.str("scene")? {
        "rlim_zero" => {
            let x: Vec<f64> = (0..10).map(f64::from).collect();
            let y: Vec<f64> = x.iter().map(|v| v + 0.01).collect();
            made(ax.plot(x, y))?;
        }
        "no_data" => {}
        "log" => {
            made(polar.set_rscale("log"))?;
        }
        "spans" => {
            ax.axvspan(0.0, 1.0);
            ax.axvline(0.5, LineProps::default());
            plot_y(&[0.1, 0.2]);
        }
        "radial" => {
            let mut observed = vec![limits_of(&polar)];
            polar.set_rticks(&[1.0, 2.0, 3.0, 4.0]);
            observed.push(limits_of(&polar));
            made(ax.plot([1.0, 2.0], [1.0, 2.0]))?;
            observed.push(limits_of(&polar));
            polar.set_rticks(&[-1.0, 0.0, 1.0, 2.0]);
            observed.push(limits_of(&polar));
            made(ax.plot([1.0, 2.0], [-1.0, -2.0]))?;
            observed.push(limits_of(&polar));
            return Ok(Json::Array(observed));
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
    Ok(Json::Array(vec![limits_of(&polar)]))
}

/// `set_thetalim` in radians or degrees.
fn thetalim(args: Args<'_>) -> Outcome {
    let (low, high) = (args.f64("min")?, args.f64("max")?);
    let (low, high) = if args.bool("degrees")? {
        (low.to_radians(), high.to_radians())
    } else {
        (low, high)
    };
    let polar = Figure::new().polar_subplot();
    made(polar.set_thetalim(low, high))?;
    Ok(Json::from(vec![polar.thetamin(), polar.thetamax()]))
}

/// The major θ ticks of each axes, in degrees.
fn ticks(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let axes = match args.str("scene")? {
        "negative" => {
            let polar = fig.polar_subplot();
            made(polar.set_thetalim(-std::f64::consts::PI, std::f64::consts::PI))?;
            vec![polar]
        }
        "mosaic" => {
            let grid = made(GridSpec::new(1, 7))?;
            let spans = [(0, 4), (4, 6)].map(|(start, end)| grid.span(0..1, start..end));
            let mut axes = Vec::new();
            for span in spans {
                let polar = made(fig.add_polar_subplot_spec(&made(span)?))?;
                made(polar.set_thetalim(0.0, std::f64::consts::PI))?;
                axes.push(polar);
            }
            axes
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    };
    fig.layout();
    let per_axes = axes
        .iter()
        .map(|polar| {
            let ticks = made(polar.axes().ticks(AxisName::X))?;
            Ok(Json::Array(
                ticks
                    .major
                    .iter()
                    .map(|tick| {
                        Json::Array(vec![
                            tick.loc.to_degrees().into(),
                            tick.label.clone().into(),
                        ])
                    })
                    .collect(),
            ))
        })
        .collect::<Result<_, ProbeError>>()?;
    Ok(Json::Array(per_axes))
}

/// `fig.get_tightbbox()` of a polar axes turned or offset.
fn tightbbox(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let polar = fig.polar_subplot();
    match args.str("scene")? {
        "default" => {}
        "N20" | "N30" => {
            let offset = if args.str("scene")? == "N20" {
                20.0
            } else {
                30.0
            };
            polar
                .set_theta_direction(ThetaDirection::Clockwise)
                .set_theta_zero_location(Compass::N, offset);
        }
        "offset" => {
            polar.set_theta_offset(NEARLY_QUARTER_TURN);
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
    Ok(bbox(&fig.tight_bbox()))
}

/// The status bar message at each display pixel.
fn cursor(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    fig.polar_subplot();
    fig.layout();
    let navigation = Navigation::new();
    let messages = parse_points(args.json("pixels")?)?
        .into_iter()
        .map(|pixel| Json::String(navigation.readout(&fig, pixel)))
        .collect();
    Ok(Json::Array(messages))
}
