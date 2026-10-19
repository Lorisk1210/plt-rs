//! `test_polar.py`: annotations on and off polar axes, label alignment,
//! radial limits, origins and inversion, θ positions and sectors, wrapped
//! and interpolated lines, shared axes removed, log radii and error bars.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use plt::axes::{
    AxisName, BarOptions, Limits, LocatorParams, PlotError, TickDirection, TickLabelProps,
    TickLevel, TickParams,
};
use plt::patches::Ellipse;
use plt::polar::{Compass, PolarAxes, ThetaDirection};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{
    AnnotateOptions, ArrowProps, HorizontalAlignment, LegacyArrowProps, VerticalAlignment,
};
use plt::ticker::FuncFormatter;
use plt::ticker::pyfmt::FormatSpec;
use plt_core::Point;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

/// `r = np.arange(start, stop, 0.01)` and `θ = 2πr`.
fn spiral(start: f64, stop: f64) -> (Vec<f64>, Vec<f64>) {
    let r = arange(start, stop, 0.01);
    (r.iter().map(|r| TAU * r).collect(), r)
}

/// Polar axes at `(0.1, 0.1, 0.8, 0.8)` with the spiral plotted.
fn spiral_axes(fig: &Figure, start: f64, stop: f64) -> Result<PolarAxes, PlotError> {
    let ax = fig.add_polar_axes([0.1, 0.1, 0.8, 0.8]);
    let (theta, r) = spiral(start, stop);
    ax.plot(theta, r)?;
    Ok(ax)
}

/// The legacy annotation arrow of the tests: black, shortened by 5%.
fn black_arrow() -> ArrowProps {
    ArrowProps::Legacy(LegacyArrowProps {
        shrink: 0.05,
        patch: PatchProps::default().facecolor("black"),
        ..LegacyArrowProps::default()
    })
}

/// Degrees to radians, element-wise.
fn radians(degrees: &[f64]) -> Vec<f64> {
    degrees.iter().map(|d| d.to_radians()).collect()
}

pub(super) fn polar_annotations(fig: &Figure) -> CaseResult {
    let r = arange(0.0, 1.0, 0.001);
    let theta: Vec<f64> = r.iter().map(|r| 2.0 * 2.0 * PI * r).collect();
    let ax = fig.polar_subplot();
    ax.plot_with(
        &theta,
        &r,
        LineProps::default().color("#ee8d18").linewidth(3.0),
    )?;
    ax.plot_with(
        [0.0, 0.0],
        [0.0, 1.0],
        LineProps::default().color("#0000ff").linewidth(1.0),
    )?;
    let (thisr, thistheta) = (r[800], theta[800]);
    ax.plot_fmt([thistheta], [thisr], "o")?;
    let options = AnnotateOptions {
        xytext: Some(Point::new(0.05, 0.05)),
        textcoords: Some("figure fraction".parse()?),
        arrowprops: Some(black_arrow()),
        ..AnnotateOptions::default()
    };
    ax.annotate("a polar annotation", (thistheta, thisr), options)?
        .set_halign(HorizontalAlignment::Left)
        .set_valign(VerticalAlignment::Baseline);
    ax.tick_params(&TickParams {
        axis: Some(AxisName::X),
        bottom: Some(true),
        top: Some(true),
        direction: Some(TickDirection::Out),
        ..TickParams::default()
    });
    Ok(())
}

pub(super) fn polar_coord_annotations(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    ax.add_artist(Ellipse::new((0.0, 0.0), 10.0, 20.0))
        .update_props(PatchProps::default().facecolor("r").alpha(0.5));
    let options = AnnotateOptions {
        xytext: Some(Point::new(PI / 3.0, 20.0)),
        xycoords: "polar".parse()?,
        textcoords: Some("polar".parse()?),
        arrowprops: Some(black_arrow()),
        ..AnnotateOptions::default()
    };
    ax.annotate("the top", (FRAC_PI_2, 10.0), options)?
        .set_halign(HorizontalAlignment::Left)
        .set_valign(VerticalAlignment::Baseline)
        .set_clip_on(true);
    ax.set_xlim(-20.0, 20.0).set_ylim(-20.0, 20.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn polar_alignment(fig: &Figure) -> CaseResult {
    let angles = [0.0, 90.0, 180.0, 270.0];
    let grid_values = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let rect = [0.1, 0.1, 0.8, 0.8];
    let horizontal = fig.add_polar_axes(rect);
    horizontal.set_thetagrids(&angles);
    let vertical = fig.add_polar_axes(rect);
    vertical.set_patch_visible(false);
    let aligned = TickLabelProps {
        halign: Some(HorizontalAlignment::Left),
        valign: Some(VerticalAlignment::Top),
        ..TickLabelProps::default()
    };
    for (ax, angle) in [horizontal, vertical].iter().zip(angles) {
        ax.set_rgrids(&grid_values, Some(angle));
        ax.yaxis().set_ticklabel_props(TickLevel::Major, &aligned);
    }
    Ok(())
}

pub(super) fn polar_wrap(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    ax.plot(radians(&[179.0, -179.0]), [0.2, 0.1])?;
    ax.plot(radians(&[2.0, -2.0]), [0.2, 0.1])?;
    Ok(())
}

/// The `fig_ref` sides of the unit tests: the points in radians.
fn unit_points(fig: &Figure) -> Result<PolarAxes, PlotError> {
    let ax = fig.polar_subplot();
    ax.plot(radians(&[30.0, 45.0, 60.0, 90.0]), [1.0, 2.0, 3.0, 4.0])?;
    Ok(ax)
}

pub(super) fn polar_units_1_ref(fig: &Figure) -> CaseResult {
    unit_points(fig)?.set_xlabel("deg");
    Ok(())
}

pub(super) fn polar_units_2_ref(fig: &Figure) -> CaseResult {
    let ax = unit_points(fig)?;
    // `"{:.12}".format`
    let spec = FormatSpec::parse(".12")?;
    ax.xaxis()
        .set_major_formatter(FuncFormatter::new(move |x, _| {
            spec.format_float(x).unwrap_or_default()
        }));
    ax.set_xlabel("rad");
    ax.set_ylabel("km");
    Ok(())
}

pub(super) fn polar_rmin(fig: &Figure) -> CaseResult {
    spiral_axes(fig, 0.0, 3.0)?.set_rmax(2.0).set_rmin(0.5);
    Ok(())
}

pub(super) fn polar_negative_rmin(fig: &Figure) -> CaseResult {
    spiral_axes(fig, -3.0, 0.0)?.set_rmax(0.0).set_rmin(-3.0);
    Ok(())
}

pub(super) fn polar_rorigin(fig: &Figure) -> CaseResult {
    spiral_axes(fig, 0.0, 3.0)?
        .set_rmax(2.0)
        .set_rmin(0.5)
        .set_rorigin(Some(0.0));
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn polar_invertedylim(fig: &Figure) -> CaseResult {
    fig.add_polar_axes([0.1, 0.1, 0.8, 0.8]).set_ylim(2.0, 0.0);
    Ok(())
}

pub(super) fn polar_invertedylim_rorigin(fig: &Figure) -> CaseResult {
    let ax = fig.add_polar_axes([0.1, 0.1, 0.8, 0.8]);
    ax.yaxis().set_inverted(true);
    ax.plot_with([0.0, 0.0], [0.0, 2.0], LineProps::default().color("none"))?;
    ax.set_margins(0.0, 0.0)?;
    ax.set_rorigin(Some(3.0));
    Ok(())
}

pub(super) fn polar_theta_position(fig: &Figure) -> CaseResult {
    spiral_axes(fig, 0.0, 3.0)?
        .set_theta_zero_location(Compass::NW, 30.0)
        .set_theta_direction(ThetaDirection::Clockwise);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn polar_rlabel_position(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    ax.set_rlabel_position(315.0);
    ax.tick_params(&TickParams {
        labelrotation_auto: Some(true),
        ..TickParams::default()
    });
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn polar_title_position(fig: &Figure) -> CaseResult {
    fig.polar_subplot().set_title("foo");
    Ok(())
}

pub(super) fn polar_theta_limits(fig: &Figure) -> CaseResult {
    let (theta, r) = spiral(0.0, 3.0);
    let theta_mins = arange(15.0, 361.0, 90.0);
    let theta_maxs = arange(50.0, 361.0, 90.0);
    let directions = [TickDirection::Out, TickDirection::In, TickDirection::InOut];
    let (rows, cols) = (theta_mins.len(), theta_maxs.len());
    for (i, &start) in theta_mins.iter().enumerate() {
        for (j, &end) in theta_maxs.iter().enumerate() {
            let ax = fig.add_polar_subplot(rows, cols, i * cols + j + 1)?;
            ax.plot(&theta, &r)?;
            if start < end {
                ax.set_thetamin(start).set_thetamax(end);
            } else {
                ax.set_thetamin(end)
                    .set_thetamax(start)
                    .set_theta_direction(ThetaDirection::Clockwise);
            }
            ax.tick_params(&TickParams {
                bottom: Some(true),
                top: Some(true),
                left: Some(true),
                right: Some(true),
                direction: Some(directions[i % directions.len()]),
                labelrotation_auto: Some(true),
                ..TickParams::default()
            });
            ax.yaxis().set_tick_params(&TickParams {
                labelright: Some(true),
                labelrotation_auto: Some(true),
                ..TickParams::default()
            });
            ax.locator_params(&LocatorParams {
                axis: Some(AxisName::X),
                steps: Some(vec![1.0, 2.0, 2.5, 5.0, 10.0]),
                ..LocatorParams::default()
            })?;
        }
    }
    Ok(())
}

pub(super) fn polar_rlim(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    ax.set_limits_with(
        AxisName::Y,
        Limits {
            high: Some(10.0),
            ..Limits::default()
        },
    )?;
    ax.set_limits_with(
        AxisName::Y,
        Limits {
            low: Some(0.5),
            ..Limits::default()
        },
    )?;
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn polar_rlim_bottom(fig: &Figure) -> CaseResult {
    fig.polar_subplot().set_rlim(0.5, 10.0);
    Ok(())
}

pub(super) fn polar_interpolation_steps_constant_r(fig: &Figure) -> CaseResult {
    for (index, width) in [(1, 3.0 * PI), (2, -3.0 * PI)] {
        let options = BarOptions {
            thickness: vec![width],
            edgecolor: vec![Color::from("none")],
            props: PatchProps::default().antialiased(false),
            ..BarOptions::default()
        };
        fig.add_polar_subplot(1, 2, index)?
            .bar([0.0], [1.0], options)?;
    }
    Ok(())
}

pub(super) fn polar_interpolation_steps_variable_r(fig: &Figure) -> CaseResult {
    let line = fig.polar_subplot().plot([0.0, FRAC_PI_2], [1.0, 2.0])?;
    line.set_interpolation_steps(100_usize);
    Ok(())
}

/// A 2 × 2 grid of polar axes sharing their θ (`x`) or radial axes, of
/// which only the lower left one is kept.
fn remove_all_but_lower_left(fig: &Figure, name: AxisName) -> CaseResult {
    let axs = (1..=4)
        .map(|index| fig.add_polar_subplot(2, 2, index))
        .collect::<Result<Vec<_>, _>>()?;
    for ax in &axs[1..] {
        match name {
            AxisName::X => ax.share_x(&axs[0])?,
            AxisName::Y => ax.share_y(&axs[0])?,
        };
    }
    for index in [0, 1, 3] {
        axs[index].remove();
    }
    Ok(())
}

pub(super) fn remove_shared_polar(fig: &Figure) -> CaseResult {
    remove_all_but_lower_left(fig, AxisName::Y)
}

pub(super) fn remove_shared_polar_ref(fig: &Figure) -> CaseResult {
    remove_all_but_lower_left(fig, AxisName::X)
}

pub(super) fn polar_log(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    ax.set_rscale("log")?;
    ax.set_rlim(1.0, 1000.0);
    ax.plot(linspace(0.0, TAU, 100), logspace(0.0, 2.0, 100, 10.0))?;
    Ok(())
}

pub(super) fn polar_log_rorigin(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    ax.set_facecolor("red");
    ax.set_rscale("log")?;
    ax.set_rlim(1.0, 100.0)
        .set_rorigin(Some(1e-3))
        .set_rticks(&logspace(0.0, 2.0, 5, 10.0));
    ax.tick_params(&TickParams {
        labelleft: Some(false),
        ..TickParams::default()
    });
    Ok(())
}

/// `test_polar_errorbar`: θ = 0 at the top running clockwise, set before
/// or after the error bars are drawn.
fn polar_errorbar(fig: &Figure, before: bool) -> CaseResult {
    let theta = arange(0.0, TAU, PI / 8.0);
    let r: Vec<f64> = theta.iter().map(|t| t / PI / 2.0 + 0.5).collect();
    let ax = fig.polar_subplot();
    let turn = |ax: &PolarAxes| {
        ax.set_theta_zero_location(Compass::N, 0.0)
            .set_theta_direction(ThetaDirection::Clockwise);
    };
    if before {
        turn(&ax);
    }
    let options = ErrorbarOptions {
        xerr: Some(0.1.into()),
        yerr: Some(0.1.into()),
        capsize: Some(7.0),
        fmt: "o".to_owned(),
        line: LineProps::default().color("seagreen"),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(theta, r, options)?;
    if !before {
        turn(&ax);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn polar_errorbar_before(fig: &Figure) -> CaseResult {
    polar_errorbar(fig, true)
}

pub(super) fn polar_errorbar_after(fig: &Figure) -> CaseResult {
    polar_errorbar(fig, false)
}
