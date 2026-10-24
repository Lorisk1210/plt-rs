//! `axisartist`: axes through the origin, parasite axes with offset
//! spines, a curvilinear grid and floating axes over a polar sector.

use std::f64::consts::PI;

use plt::artist::ClipPathSpec;
use plt::axes::{AxisName, GridOptions};
use plt::axisartist::angle_helper::{ExtremeFinderCycle, FormatterDms, LocatorDms};
use plt::axisartist::floating_axes::{FloatingAxes, FloatingGridHelper};
use plt::axisartist::grid_finder::{GridFinderOptions, GridFixedLocator, GridMaxNLocator};
use plt::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
use plt::axisartist::{AuxTransform, AxisArtist, AxisArtistAxes, Toggle};
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::transforms::Affine2D;

use super::data::Samples;
use super::lines::map;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "axes_zero",
        "Axis lines through the origin, with arrow heads",
        axes_zero,
    ),
    Entry::figure(
        "parasite_axes",
        "Parasite axes: three y axes, one offset",
        parasite_axes,
    ),
    Entry::figure(
        "curvilinear_grid",
        "A polar grid on Cartesian axes",
        curvilinear_grid,
    ),
    Entry::figure(
        "floating_axes",
        "Floating axes over a polar sector",
        floating_axes,
    ),
];

/// The named axis artist of `ax`.
fn axis(ax: &AxisArtistAxes, name: &str) -> Result<AxisArtist, &'static str> {
    ax.axis(name).ok_or("an axis artist of this name")
}

/// Polar coordinates with the angle in degrees, as the transform from
/// the grid's coordinates to the axes' data.
fn polar_degrees() -> AuxTransform {
    AuxTransform::affine(Affine2D::identity().scale(PI / 180.0, 1.0)).then(AuxTransform::polar())
}

/// A cubic and its derivative on axes whose axis lines cross at the
/// origin, ending in arrow heads, as in a textbook.
fn axes_zero() -> Built {
    let fig = Figure::with_size(6.0, 4.5, 100.0);
    let ax = AxisArtistAxes::zero(fig.subplot());
    for name in ["xzero", "yzero"] {
        let line = axis(&ax, name)?;
        line.set_axisline_style(Some("-|>".parse()?));
        line.set_visible(true);
    }
    for name in ["left", "right", "bottom", "top"] {
        axis(&ax, name)?.set_visible(false);
    }
    let x = linspace(-2.0, 2.5, 200);
    ax.ax().plot_with(
        &x,
        map(&x, |x| x * x * x - 2.0 * x),
        LineProps::default().label("f(x) = x³ − 2x"),
    )?;
    ax.ax().plot_with(
        &x,
        map(&x, |x| 3.0 * x * x - 2.0),
        LineProps::fmt("--")?.label("f′(x)"),
    )?;
    ax.ax().set_ylim(-4.0, 8.0)?;
    ax.ax().grid(false);
    ax.ax().legend(LegendOptions::default())?;
    Ok(fig)
}

/// Three quantities against one x: two parasite axes share the host's x
/// axis, and the third's y axis sits 60 points to the right.
fn parasite_axes() -> Built {
    let fig = Figure::with_size(6.4, 4.2, 100.0);
    let host = AxisArtistAxes::new(fig.subplot());
    let (temperature, velocity) = (host.twinx(), host.twinx());
    velocity.new_fixed_axis("right", SpineSide::Right, (60.0, 0.0));
    let everything = Toggle {
        all: Some(true),
        ..Toggle::default()
    };
    for parasite in [&temperature, &velocity] {
        axis(parasite, "right")?.toggle(&everything);
    }
    let x = linspace(0.0, 2.0, 50);
    let datasets = [
        (&host, map(&x, |x| 1.0 + 0.5 * x), "density"),
        (
            &temperature,
            map(&x, |x| 3.0 * (x * 1.5).sin()),
            "temperature",
        ),
        (&velocity, map(&x, |x| 50.0 - 18.0 * x), "velocity"),
    ];
    for (ax, y, label) in datasets {
        let line = ax
            .ax()
            .plot_with(&x, y, LineProps::default().label(label))?;
        ax.ax().set_ylabel(label);
        let side = if ax == &host { "left" } else { "right" };
        axis(ax, side)?.label().set_color(Some(line.color()));
    }
    host.ax().set_xlabel("distance");
    host.ax().legend(LegendOptions::default())?;
    Ok(fig)
}

/// A grid of constant angle and radius drawn on ordinary x/y axes: the
/// grid helper maps polar coordinates to the data, and a floating axis
/// runs along the 45° ray.
fn curvilinear_grid() -> Built {
    let fig = Figure::with_size(5.5, 5.0, 100.0);
    let finder = GridFinderOptions {
        extreme_finder: Some(Box::new(ExtremeFinderCycle {
            lat_minmax: Some((0.0, f64::INFINITY)),
            ..ExtremeFinderCycle::new(20, 20)
        })),
        grid_locator1: Some(Box::new(LocatorDms::new(12))),
        grid_locator2: Some(Box::new(GridMaxNLocator::new(5))),
        tick_formatter1: Some(Box::new(FormatterDms)),
        ..GridFinderOptions::default()
    };
    let helper = GridHelperCurveLinear::new(polar_degrees(), finder);
    let ax = AxisArtistAxes::with_grid_helper(fig.subplot(), helper);
    axis(&ax, "right")?.major_ticklabels().set_visible(true);
    axis(&ax, "top")?.major_ticklabels().set_visible(true);
    axis(&ax, "right")?.set_nth_coord_ticks(AxisName::X);
    let ray = ax.new_floating_axis("ray", AxisName::X, 45.0);
    ray.label().set_text("θ = 45°").set_visible(true);
    ray.set_extremes(1.0, 9.0);
    // A spiral given in degrees and radius, drawn through the transform.
    let angle = linspace(0.0, 540.0, 300);
    ax.aux_axes(&polar_degrees())
        .plot(&angle, map(&angle, |a| 1.0 + a / 90.0))?;
    ax.ax().set_aspect(Aspect::Equal)?;
    ax.ax().set_xlim(-8.0, 8.0)?.set_ylim(-8.0, 8.0)?;
    ax.grid(true, GridOptions::default());
    Ok(fig)
}

/// Axes shaped like the sector of angles 20°–110° and radii 2–10: the
/// frame follows the polar grid, and points given in degrees and radius
/// are drawn through an auxiliary axes, clipped to the sector.
fn floating_axes() -> Built {
    let fig = Figure::with_size(5.5, 5.0, 100.0);
    let finder = GridFinderOptions {
        grid_locator1: Some(Box::new(LocatorDms::new(6))),
        grid_locator2: Some(Box::new(GridFixedLocator::new([2.0, 4.0, 6.0, 8.0, 10.0]))),
        tick_formatter1: Some(Box::new(FormatterDms)),
        ..GridFinderOptions::default()
    };
    let helper = FloatingGridHelper::new(polar_degrees(), (20.0, 110.0, 2.0, 10.0), finder);
    let ax = FloatingAxes::new(fig.subplot(), helper)?;
    // The edges are named after the grid: "left" is the 20° ray, "top"
    // the outer arc. Their labels face away from the sector; the inner
    // arc stays bare.
    let ray = axis(&ax, "left")?;
    ray.set_axis_direction(SpineSide::Bottom);
    ray.label().set_text("radius").set_visible(true);
    let outer = axis(&ax, "top")?;
    outer.set_axis_direction(SpineSide::Bottom);
    outer.toggle(&Toggle {
        ticklabels: Some(true),
        label: Some(true),
        ..Toggle::default()
    });
    outer.major_ticklabels().set_axis_direction(SpineSide::Top);
    outer.label().set_axis_direction(SpineSide::Top);
    outer.label().set_text("angle");
    axis(&ax, "bottom")?.toggle(&Toggle {
        ticklabels: Some(false),
        ..Toggle::default()
    });
    let mut samples = Samples::new(59);
    let angles: Vec<f64> = (0..120).map(|_| 20.0 + 90.0 * samples.uniform()).collect();
    let radii: Vec<f64> = angles
        .iter()
        .map(|a| 6.0 + 2.5 * (a / 15.0).sin() + samples.normal(0.0, 0.7))
        .collect();
    let data = ax.aux_axes(&polar_degrees());
    let points = data.scatter_with(
        &angles,
        &radii,
        ScatterOptions {
            sizes: Some(vec![14.0]),
            ..ScatterOptions::default()
        },
    )?;
    points.set_clip_path(Some(ClipPathSpec::Path {
        path: ax.patch_path(),
        transform: ax.ax().trans_data(),
    }));
    ax.grid(true, GridOptions::default());
    Ok(fig)
}
