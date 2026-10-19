//! Probe operations for the `quiver_behaviour` area: data limits, argument
//! checks and drawing of quiver and barbs plots, ported from matplotlib's
//! `test_quiver.py` and `test_collections.py` (see
//! `spec/reference/probes/test_quiver_behaviour.py`). Every op runs under
//! the reference configuration.

use plt::axes::MappedColors;
use plt::figure::Figure;
use plt::numeric::{Grid, linspace};
use plt::quiver::{Angles, ArrowShape, BarbsOptions, QuiverOptions, QuiverUnits, VectorField};
use plt::transforms::Transform;
use plt_core::Affine2D;

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{bbox, object};
use crate::probes::{Args, Op, Outcome, ProbeError};
use crate::rng::SpecRng;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("quiver.limits", |args| with_rc(args, || limits(args))),
    Op::new("quiver.arguments", |args| with_rc(args, || arguments(args))),
    Op::new("quiver.draw", |args| with_rc(args, || draw(args))),
];

fn made<T>(result: Result<T, impl std::fmt::Display>) -> Result<T, ProbeError> {
    result.map_err(ProbeError::rejected)
}

/// `sin(X)`, `cos(Y)` on the 20 × 10 grid of `linspace(-5, 10, 20)` and
/// `linspace(-2, 4, 10)`, rows along x as `meshgrid(y, x)` builds them.
fn translated_field() -> VectorField {
    let (xs, ys) = (linspace(-5.0, 10.0, 20), linspace(-2.0, 4.0, 10));
    let x = Grid::from_fn(20, 10, |r, _| xs[r]);
    let y = Grid::from_fn(20, 10, |_, c| ys[c]);
    VectorField::on_mesh(&x, &y, &x.map(|v| v.sin()), &y.map(|v| v.cos()))
}

fn limits(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let shifted = || Transform::Affine(Affine2D::translation(25.0, 32.0)).then(ax.trans_data());
    match args.str("scene")? {
        "grid" => {
            let u = Grid::from_fn(10, 8, |r, c| 10.0 * (8 * r + c) as f64 / 79.0);
            let x: Vec<f64> = (0..8).map(f64::from).collect();
            let y: Vec<f64> = (0..10).map(f64::from).collect();
            let field = VectorField::on_axes(&x, &y, &u, &u);
            made(ax.quiver(field, QuiverOptions::default()))?;
        }
        "translated" => {
            let options = QuiverOptions {
                transform: Some(shifted()),
                ..QuiverOptions::default()
            };
            made(ax.quiver(translated_field(), options))?;
        }
        "barbs_translated" => {
            let options = BarbsOptions {
                transform: Some(shifted()),
                ..BarbsOptions::default()
            };
            made(ax.barbs(translated_field(), options))?;
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
    let (x0, x1) = ax.xlim();
    let (y0, y1) = ax.ylim();
    Ok(object([
        ("datalim", bbox(&ax.data_lim().bbox)),
        ("xlim", Json::from(vec![x0, x1])),
        ("ylim", Json::from(vec![y0, y1])),
    ]))
}

/// Whether `quiver` or `barbs` accepts the arrays.
fn arguments(args: Args<'_>) -> Outcome {
    let mut field = VectorField::new(
        args.f64_vec("x")?,
        args.f64_vec("y")?,
        args.f64_vec("u")?,
        args.f64_vec("v")?,
    );
    if args.get("c").is_some() {
        field = field.with_colors(MappedColors::new(args.f64_vec("c")?));
    }
    let ax = Figure::new().subplot();
    match args.str("plot")? {
        "quiver" => {
            made(ax.quiver(field, QuiverOptions::default()))?;
        }
        "barbs" => {
            made(ax.barbs(field, BarbsOptions::default()))?;
        }
        other => return Err(ProbeError::BadArgs(format!("unknown plot {other:?}"))),
    }
    Ok(Json::Bool(true))
}

/// Whether a quiver plot draws.
fn draw(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    match args.str("scene")? {
        "zero_head" => {
            let u = Grid::from_fn(10, 10, |_, c| (c as f64).cos());
            let v = Grid::from_fn(10, 10, |r, _| (r as f64).sin());
            let options = QuiverOptions {
                shape: ArrowShape {
                    head_length: 0.0,
                    head_axis_length: 0.0,
                    ..ArrowShape::default()
                },
                ..QuiverOptions::default()
            };
            made(ax.quiver(VectorField::on_index_grid(&u, &v), options))?;
        }
        "angles_xy" => {
            let phi: Vec<f64> = SpecRng::new(4)
                .uniform(150, 0.0, 1.0)
                .into_iter()
                .map(|p| (p - 0.5) * 150.0)
                .collect();
            let ones = Grid::filled(10, 15, 1.0);
            let x: Vec<f64> = (0..15).map(f64::from).collect();
            let y: Vec<f64> = (0..10).map(f64::from).collect();
            let options = QuiverOptions {
                angles: Angles::Degrees(phi),
                scale_units: Some(QuiverUnits::Xy),
                ..QuiverOptions::default()
            };
            made(ax.quiver(VectorField::on_axes(&x, &y, &ones, &ones), options))?;
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
    made(fig.render_raster())?;
    Ok("drawn".into())
}
