//! Probe operations for the `axes3d_behaviour` area: limits under aspect
//! changes, 3-D collections, margins and scales, bounds and inversion,
//! argument checks, box aspects, scatter plots of non-finite points,
//! every tick of each axis (dates included), the axis position flags and
//! the coordinate readout, ported from mplot3d's `test_axes3d.py` (see
//! `spec/reference/probes/test_axes3d_behaviour.py`). Every op runs under
//! the reference configuration.

use plt::axes::{Adjustable, ScatterColors, ScatterOptions, TickParams};
use plt::collections::CollectionProps;
use plt::figure::Figure;
use plt::mplot3d::{
    Aspect3D, Axes3D, Axes3DOptions, Axis3, Axis3D, ProjType, Sampling, Scatter3DOptions,
    WireframeOptions,
};
use plt::numeric::{Grid, linspace, meshgrid};
use plt::ticker::{MultipleLocator, NullFormatter};

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Op, Outcome, ProbeError, made};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("axes3d.limits", |args| with_rc(args, || limits(args))),
    Op::new("axes3d.zbounds", |args| with_rc(args, zbounds)),
    Op::new("axes3d.set_limit", |args| with_rc(args, || set_limit(args))),
    Op::new("axes3d.errors", |args| with_rc(args, || errors(args))),
    Op::new("axes3d.margins", |args| with_rc(args, margins)),
    Op::new("axes3d.box_aspect", |args| {
        with_rc(args, || box_aspect(args))
    }),
    Op::new("axes3d.scatter_nonfinite", |args| {
        with_rc(args, scatter_nonfinite)
    }),
    Op::new("axes3d.ticks", |args| with_rc(args, || ticks(args))),
    Op::new("axes3d.axis_position", |args| {
        with_rc(args, || axis_position(args))
    }),
    Op::new("axes3d.format_data", |args| {
        with_rc(args, || format_data(args))
    }),
];

const AXES: [Axis3; 3] = [Axis3::X, Axis3::Y, Axis3::Z];

/// An axis named `"x"`, `"y"` or `"z"`.
fn axis_arg(args: Args<'_>, key: &str) -> Result<Axis3, ProbeError> {
    match args.str(key)? {
        "x" => Ok(Axis3::X),
        "y" => Ok(Axis3::Y),
        "z" => Ok(Axis3::Z),
        other => Err(ProbeError::BadArgs(format!("unknown axis {other:?}"))),
    }
}

fn lims(ax: &Axes3D) -> Json {
    Json::Array(
        AXES.iter()
            .map(|&axis| {
                let (low, high) = ax.limits(axis);
                Json::from(vec![low, high])
            })
            .collect(),
    )
}

fn plot(ax: &Axes3D, x: [f64; 2], y: [f64; 2], z: [f64; 2]) -> Result<(), ProbeError> {
    made(ax.plot(x, y, z))?;
    Ok(())
}

fn aspect_datalim(fig: &Figure, ax: &Axes3D) -> Result<Vec<Json>, ProbeError> {
    plot(ax, [0.0, 1.0], [0.0, 2.0], [0.0, 3.0])?;
    let mut observed = Vec::new();
    for aspect in [None, Some(Aspect3D::Equal), Some(Aspect3D::Auto)] {
        if let Some(aspect) = aspect {
            made(ax.set_aspect(aspect, Adjustable::Datalim))?;
        }
        fig.layout();
        observed.push(lims(ax));
    }
    Ok(observed)
}

fn aspect_datalim_untouched(ax: &Axes3D) -> Result<Vec<Json>, ProbeError> {
    plot(ax, [1.0, 2.0], [3.0, 4.0], [5.0, 6.0])?;
    let before = lims(ax);
    made(ax.set_aspect(Aspect3D::Equal, Adjustable::Datalim))?;
    made(ax.set_xlim(-50.0, 50.0))?;
    made(ax.set_aspect(Aspect3D::Auto, Adjustable::Datalim))?;
    Ok(vec![before, lims(ax)])
}

fn autoscale(ax: &Axes3D) -> Result<Vec<Json>, ProbeError> {
    made(ax.set_margins([Some(0.0), Some(0.1), Some(0.2)], None))?;
    plot(ax, [0.0, 1.0], [0.0, 1.0], [0.0, 1.0])?;
    let mut observed = vec![lims(ax)];
    for axis in AXES {
        ax.set_axis_autoscale_on(axis, false);
    }
    ax.set_axis_autoscale_on(Axis3::Z, true);
    plot(ax, [0.0, 2.0], [0.0, 2.0], [0.0, 2.0])?;
    observed.push(lims(ax));
    ax.set_axis_autoscale_on(Axis3::X, true);
    plot(ax, [0.0, 2.0], [0.0, 2.0], [0.0, 2.0])?;
    observed.push(lims(ax));
    Ok(observed)
}

fn shared_retick(fig: &Figure, ax: &Axes3D) -> Result<Vec<Json>, ProbeError> {
    let other = made(fig.add_subplot_3d(2, 1, 2, Axes3DOptions::default()))?;
    made(other.share_z(ax))?;
    plot(ax, [0.0, 1.0], [0.0, 1.0], [0.0, 2.0])?;
    plot(&other, [0.0, 1.0], [0.0, 1.0], [0.0, 2.0])?;
    ax.zaxis().set_ticks([-0.5, 0.0, 2.0, 2.5]);
    Ok(vec![lims(ax), lims(&other)])
}

/// The limits after each step of a scene.
fn limits(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    let scene = args.str("scene")?;
    let observed = match scene {
        "aspect_datalim" => aspect_datalim(&fig, &ax)?,
        "aspect_datalim_untouched" => aspect_datalim_untouched(&ax)?,
        "lines3d" | "lines3d_ragged" => {
            let lines = if scene == "lines3d" {
                vec![
                    vec![[0.0, 0.0, 0.0], [1.0, 4.0, 2.0]],
                    vec![[1.0, 1.0, 3.0], [2.0, 0.0, 2.0]],
                    vec![[1.0, 0.0, 4.0], [1.0, 4.0, 5.0]],
                ]
            } else {
                vec![
                    vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]],
                    vec![[0.0, 1.0, 0.0], [1.0, 2.0, 1.0]],
                    vec![
                        [1.0, 0.0, 1.0],
                        [2.0, 1.0, 2.0],
                        [3.0, 2.0, 3.0],
                        [4.0, 3.0, 4.0],
                    ],
                ]
            };
            made(ax.add_line_collection3d(lines, CollectionProps::default(), true))?;
            vec![lims(&ax)]
        }
        "polys3d" => {
            let triangle = vec![vec![[0.0, 0.0, 0.0], [1.0, 1.0, 3.0], [1.0, 0.0, 4.0]]];
            made(ax.add_poly_collection3d(triangle, CollectionProps::default(), true))?;
            vec![lims(&ax)]
        }
        "autoscale" => autoscale(&ax)?,
        "shared_retick" => shared_retick(&fig, &ax)?,
        other => match other.strip_prefix("scale:") {
            Some(name) => {
                made(ax.set_xscale(name))?;
                made(ax.set_yscale(name))?;
                made(ax.set_zscale(name))?;
                fig.layout();
                vec![lims(&ax)]
            }
            None => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
        },
    };
    Ok(Json::Array(observed))
}

/// The z limits, bounds and inversion after each bound change.
fn zbounds() -> Outcome {
    let ax = Figure::new().subplot_3d();
    let observe = |ax: &Axes3D| {
        let (z0, z1) = ax.zlim();
        let (b0, b1) = ax.bound(Axis3::Z);
        object([
            ("zlim", Json::from(vec![z0, z1])),
            ("zbound", Json::from(vec![b0, b1])),
            ("inverted", ax.inverted(Axis3::Z).into()),
        ])
    };
    made(ax.set_zlim(0.0, 1.0))?;
    let mut observed = vec![observe(&ax)];
    made(ax.set_zbound(Some(0.0), Some(2.0)))?;
    observed.push(observe(&ax));
    made(ax.invert(Axis3::Z))?;
    observed.push(observe(&ax));
    made(ax.set_zbound(None, Some(1.0)))?;
    observed.push(observe(&ax));
    made(ax.set_zbound(Some(2.0), None))?;
    observed.push(observe(&ax));
    Ok(Json::Array(observed))
}

/// One limit set, the other kept (`set_xlim3d(left=...)` and friends).
fn set_limit(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot_3d();
    let axis = axis_arg(args, "axis")?;
    let value = args.f64("value")?;
    let (low, high) = ax.limits(axis);
    let (low, high) = match args.str("side")? {
        "left" | "bottom" => (value, high),
        "right" | "top" => (low, value),
        other => return Err(ProbeError::BadArgs(format!("unknown side {other:?}"))),
    };
    made(ax.set_limits(axis, low, high))?;
    Ok(lims(&ax))
}

/// Whether a call is accepted.
fn errors(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot_3d();
    match args.str("case")? {
        "focal_zero" | "focal_half" => {
            let focal_length = if args.str("case")? == "focal_zero" {
                0.0
            } else {
                0.5
            };
            made(ax.set_proj_type(ProjType::Persp { focal_length }))?;
        }
        "wireframe_strides" | "wireframe_ok" => {
            let stride = usize::from(args.str("case")? == "wireframe_ok");
            let axis = linspace(-1.0, 1.0, 5);
            let (x, y) = meshgrid(&axis, &axis);
            let z = Grid::from_fn(5, 5, |r, c| x[(r, c)] * y[(r, c)]);
            let options = WireframeOptions {
                sampling: Sampling::Strides {
                    rows: stride,
                    cols: stride,
                },
                ..WireframeOptions::default()
            };
            made(ax.plot_wireframe(&x, &y, &z, options))?;
        }
        "margin" | "margin_ok" => {
            let margin = if args.str("case")? == "margin" {
                -1.0
            } else {
                -0.4
            };
            made(ax.set_margins([Some(margin); 3], None))?;
        }
        other => return Err(ProbeError::BadArgs(format!("unknown case {other:?}"))),
    }
    Ok(Json::Bool(true))
}

/// The margins after each change.
fn margins() -> Outcome {
    let ax = Figure::new().subplot_3d();
    let changes = [
        [Some(0.2); 3],
        [Some(0.1), Some(0.2), Some(0.3)],
        [Some(0.0), None, None],
        [None, Some(0.1), None],
        [None, None, Some(0.0)],
    ];
    let mut observed = Vec::new();
    for change in changes {
        made(ax.set_margins(change, None))?;
        observed.push(Json::from(AXES.map(|axis| ax.margin(axis)).to_vec()));
    }
    Ok(Json::Array(observed))
}

/// The box aspect `set_box_aspect(None)` gives for a vertical axis.
fn box_aspect(args: Args<'_>) -> Outcome {
    let vertical = axis_arg(args, "vertical_axis")?;
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    ax.view_init(0.0, 0.0, 0.0).set_vertical_axis(vertical);
    fig.layout();
    made(ax.set_box_aspect(None, 1.0))?;
    Ok(Json::from(ax.box_aspect().to_vec()))
}

/// How many points of a scatter plot of non-finite points are drawn, and
/// whether the face colours stay aligned with them.
fn scatter_nonfinite() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            colors: ScatterColors::PerPoint(vec![(0.0, 0.0, 0.0, 1.0).into(); 4]),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    let points = made(ax.scatter_with(
        [f64::NAN, 1.0, 2.0, 1.0],
        [0.0, f64::INFINITY, 2.0, 1.0],
        [0.0, 1.0, f64::NEG_INFINITY, 1.0],
        options,
    ))?;
    fig.layout();
    let offsets = points.offsets();
    let finite = offsets.iter().filter(|p| p.is_finite()).count();
    let colors = made(points.colors())?.faces.len();
    Ok(object([
        (
            "finite_offsets",
            Json::Integer(i64::try_from(finite).expect("few points")),
        ),
        ("aligned", (offsets.len() == colors).into()),
    ]))
}

/// The laid-out axes of a scene of `axes3d.ticks`.
fn tick_scene(scene: &str) -> Result<Axes3D, ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    match scene {
        "default" => {}
        "inset" => {
            made(ax.set_zlim(0.05, 0.95))?;
        }
        "minor" => {
            made(ax.set_xlim(0.0, 1.0))?;
            ax.xaxis()
                .set_minor_locator(made(MultipleLocator::new(0.1))?);
            ax.yaxis().set_tick_params(TickParams {
                labelright: Some(true),
                ..TickParams::default()
            });
        }
        "log" => {
            made(ax.set_zscale("log"))?;
            made(ax.set_zlim(1.0, 1000.0))?;
        }
        "dates" => {
            plot(&ax, [0.0, 1.0], [0.0, 1.0], [19_000.0, 19_010.0])?;
            ax.zaxis().axis_date(None);
        }
        "dates_zlim" => {
            made(ax.set_zlim(19_000.0, 19_003.0))?;
            ax.zaxis().axis_date(None);
        }
        "dates_kept_formatter" => {
            ax.zaxis().set_major_formatter(NullFormatter);
            ax.zaxis().axis_date(None);
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
    fig.layout();
    Ok(ax)
}

/// Every tick location and label of each axis, major and minor.
fn ticks(args: Args<'_>) -> Outcome {
    let ax = tick_scene(args.str("scene")?)?;
    let observe = |axis: Axis3D| -> Outcome {
        let level = |minor: bool| -> Result<(Json, Json), ProbeError> {
            Ok((
                Json::from(made(axis.ticklocs(minor))?),
                Json::from(made(axis.ticklabels(minor))?),
            ))
        };
        let ((locs, labels), (minor_locs, minor_labels)) = (level(false)?, level(true)?);
        Ok(object([
            ("locs", locs),
            ("labels", labels),
            ("minor_locs", minor_locs),
            ("minor_labels", minor_labels),
        ]))
    };
    Ok(object([
        ("x", observe(ax.xaxis())?),
        ("y", observe(ax.yaxis())?),
        ("z", observe(ax.zaxis())?),
    ]))
}

/// `get_axis_position()` in a view.
fn axis_position(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    if args.opt_bool("inverted")?.unwrap_or(false) {
        made(ax.invert(Axis3::X))?;
    }
    ax.view_init(args.f64("elev")?, args.f64("azim")?, args.f64("roll")?);
    fig.layout();
    Ok(Json::from(ax.axis_highs().to_vec()))
}

/// `format_<axis>data(value)` in a scene of `axes3d.ticks`.
fn format_data(args: Args<'_>) -> Outcome {
    let ax = tick_scene(args.str("scene")?)?;
    ax.format_data(axis_arg(args, "axis")?, args.f64("value")?)
        .map(Json::from)
        .ok_or_else(|| ProbeError::rejected("the major formatter failed"))
}
