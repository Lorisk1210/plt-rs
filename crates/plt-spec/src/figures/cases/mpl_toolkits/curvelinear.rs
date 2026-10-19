//! The tests of `axisartist/tests/test_grid_helper_curvelinear.py` and
//! `test_floating_axes.py`: curvilinear grids with floating axes, and
//! floating axes over a range of polar coordinates.

use crate::figures::cases::common::polar_degrees;

use plt::artist::ClipPathSpec;
use plt::axes::{AxisName, GridOptions};
use plt::axisartist::angle_helper::{ExtremeFinderCycle, FormatterDms, LocatorDms};
use plt::axisartist::floating_axes::{self, FloatingAxes};
use plt::axisartist::grid_finder::{
    FixedLocator, GridError, GridFinderOptions, GridFormatter, MaxNLocator,
};
use plt::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
use plt::axisartist::{self, AuxTransform, Toggle};
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::numeric::linspace;
use plt::prelude::ArtistHandle;
use plt::rc::Aspect;
use plt::spines::SpineSide;

use super::axislines::artist;
use crate::figures::{CaseResult, FigureCase};

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_toolkits/custom_transform", custom_transform)
        .with_rc(&[("xtick.direction", "in"), ("ytick.direction", "inout")]),
    FigureCase::new("mpl_toolkits/polar_box", polar_box)
        .sized(5.0, 5.0, 100.0)
        .with_rc(&[("xtick.direction", "inout"), ("ytick.direction", "out")]),
    FigureCase::new("mpl_toolkits/axis_direction", axis_direction).sized(5.0, 5.0, 100.0),
    FigureCase::new("mpl_toolkits/curvelinear3", curvelinear3).sized(5.0, 5.0, 100.0),
    FigureCase::new("mpl_toolkits/curvelinear4", curvelinear4).sized(5.0, 5.0, 100.0),
];

/// The extreme finder of the polar tests: angles cycle over 360°, radii
/// stay positive.
fn cycle_extremes() -> ExtremeFinderCycle {
    ExtremeFinderCycle {
        lat_minmax: Some((0.0, f64::INFINITY)),
        ..ExtremeFinderCycle::new(20, 20)
    }
}

/// `ax1.grid(True)`.
fn grid_on(ax: &axisartist::Axes) {
    ax.grid(true, &GridOptions::default());
}

fn custom_transform(fig: &Figure) -> CaseResult {
    // `MyTransform(1)`: a shear whose paths are not interpolated.
    let tr = AuxTransform::from_fns(|x, y| (x, y - x), |x, y| (x, y + x));
    let helper = GridHelperCurveLinear::new(tr.clone(), GridFinderOptions::default());
    let ax1 = axisartist::Axes::with_grid_helper(fig.subplot(), helper);
    ax1.get_aux_axes(&tr).plot([3.0, 6.0], [5.0, 10.0])?;
    ax1.ax().set_aspect(Aspect::Equal)?;
    ax1.ax().set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    grid_on(&ax1);
    Ok(())
}

/// `test_polar_box`'s radius formatter:
/// `FuncFormatter(lambda x, p: "eight" if x == 8 else f"{int(x)}")`.
#[derive(Debug)]
struct EightFormatter;

impl GridFormatter for EightFormatter {
    #[expect(clippy::float_cmp, reason = "the test's formatter compares exactly")]
    fn labels(
        &self,
        _direction: SpineSide,
        _factor: f64,
        values: &[f64],
    ) -> Result<Vec<String>, GridError> {
        Ok(values
            .iter()
            .map(|&x| {
                if x == 8.0 {
                    "eight".to_owned()
                } else {
                    // `+ 0.0` turns the -0 of truncated small negatives
                    // into 0, as `int` does.
                    format!("{}", x.trunc() + 0.0)
                }
            })
            .collect())
    }
}

fn polar_box(fig: &Figure) -> CaseResult {
    let tr = polar_degrees();
    let helper = GridHelperCurveLinear::new(
        tr.clone(),
        GridFinderOptions {
            extreme_finder: Some(Box::new(cycle_extremes())),
            grid_locator1: Some(Box::new(LocatorDms::new(12))),
            tick_formatter1: Some(Box::new(FormatterDms)),
            tick_formatter2: Some(Box::new(EightFormatter)),
            ..GridFinderOptions::default()
        },
    );
    let ax1 = axisartist::Axes::with_grid_helper(fig.subplot(), helper);
    for name in ["right", "top"] {
        artist(&ax1, name)?.major_ticklabels().set_visible(true);
    }
    artist(&ax1, "right")?.set_nth_coord_ticks(AxisName::X);
    artist(&ax1, "bottom")?.set_nth_coord_ticks(AxisName::Y);
    let lat = ax1.new_floating_axis("lat", AxisName::X, 45.0);
    lat.label().set_text("Test").set_visible(true);
    lat.set_extremes(2.0, 12.0);
    let lon = ax1.new_floating_axis("lon", AxisName::Y, 6.0);
    lon.label().set_text("Test 2");
    lon.set_extremes(-180.0, 90.0);
    ax1.get_aux_axes(&tr)
        .plot(linspace(0.0, 30.0, 50), linspace(10.0, 10.0, 50))?;
    ax1.ax().set_aspect(Aspect::Equal)?;
    ax1.ax().set_xlim(-5.0, 12.0).set_ylim(-5.0, 10.0);
    grid_on(&ax1);
    Ok(())
}

fn axis_direction(fig: &Figure) -> CaseResult {
    // The test sets `nbins=5` on both grid locators after making them.
    let mut grid_locator1 = LocatorDms::new(12);
    grid_locator1.set_params(5);
    let helper = GridHelperCurveLinear::new(
        polar_degrees(),
        GridFinderOptions {
            extreme_finder: Some(Box::new(cycle_extremes())),
            grid_locator1: Some(Box::new(grid_locator1)),
            grid_locator2: Some(Box::new(MaxNLocator::new(5))),
            tick_formatter1: Some(Box::new(FormatterDms)),
            ..GridFinderOptions::default()
        },
    );
    let ax1 = axisartist::Axes::with_grid_helper(fig.subplot(), helper);
    for axis in ax1.axis_artists() {
        axis.set_visible(false);
    }
    for (name, value, direction) in [
        ("lat1", 130.0, SpineSide::Left),
        ("lat2", 50.0, SpineSide::Right),
    ] {
        let axis = ax1.new_floating_axis_facing(name, AxisName::X, value, direction);
        axis.label().set_text("Test").set_visible(true);
        axis.set_extremes(0.001, 10.0);
    }
    let lon = ax1.new_floating_axis_facing("lon", AxisName::Y, 10.0, SpineSide::Bottom);
    lon.label().set_text("Test 2");
    lon.set_extremes(50.0, 130.0);
    lon.major_ticklabels().set_axis_direction(SpineSide::Top);
    lon.label().set_axis_direction(SpineSide::Top);
    ax1.ax().set_aspect(Aspect::Equal)?;
    ax1.ax().set_xlim(-8.0, 8.0).set_ylim(-4.0, 12.0);
    grid_on(&ax1);
    Ok(())
}

/// The floating axes of `test_curvelinear3` and `test_curvelinear4`: the
/// polar range `extremes` in degrees, angles every `degrees_step`-ish
/// degrees, radii every 2.
fn floating_polar(
    fig: &Figure,
    extremes: (f64, f64, f64, f64),
    degrees_step: usize,
) -> Result<FloatingAxes, Box<dyn std::error::Error>> {
    let helper = floating_axes::GridHelperCurveLinear::new(
        polar_degrees(),
        extremes,
        GridFinderOptions {
            grid_locator1: Some(Box::new(LocatorDms::new(degrees_step))),
            grid_locator2: Some(Box::new(FixedLocator::new([2.0, 4.0, 6.0, 8.0, 10.0]))),
            tick_formatter1: Some(Box::new(FormatterDms)),
            ..GridFinderOptions::default()
        },
    );
    Ok(FloatingAxes::new(fig.subplot(), helper)?)
}

/// The floating axis `z` at radius `radius`, labelled and greyed, of both
/// floating axes tests; the label faces `label_direction` when given.
fn z_axis(ax1: &FloatingAxes, radius: f64, label_direction: Option<SpineSide>) {
    let axis = ax1.new_floating_axis("z", AxisName::Y, radius);
    axis.toggle(&Toggle {
        all: Some(true),
        label: Some(true),
        ..Toggle::default()
    });
    if let Some(direction) = label_direction {
        axis.label().set_axis_direction(direction);
    }
    axis.label().set_text("z = ?").set_visible(true);
    axis.line().set_color(Some("0.5".into()));
}

/// The data of both floating axes tests, the line clipped to the
/// background of `ax1`.
fn scatter_path(ax1: &FloatingAxes) -> CaseResult {
    let ax2 = ax1.get_aux_axes(&polar_degrees());
    let (xx, yy) = ([67.0, 90.0, 75.0, 30.0], [2.0, 5.0, 8.0, 4.0]);
    ax2.scatter(xx, yy)?;
    let line = ax2.plot_with(xx, yy, LineProps::default().color("k"))?;
    line.set_clip_path(Some(ClipPathSpec::Path {
        path: ax1.patch_path(),
        transform: ax1.ax().trans_data(),
    }));
    Ok(())
}

fn curvelinear3(fig: &Figure) -> CaseResult {
    let ax1 = floating_polar(fig, (0.0, 360.0, 10.0, 3.0), 15)?;
    // The test puts an axis of another grid helper (radii scaled by 10)
    // on the right, then hides it with the left one: only the labels of
    // both are set, and never drawn.
    artist(&ax1, "left")?.label().set_text("Test 1");
    artist(&ax1, "right")?.label().set_text("Test 2");
    for name in ["left", "right"] {
        artist(&ax1, name)?.set_visible(false);
    }
    z_axis(&ax1, 7.0, None);
    scatter_path(&ax1)
}

fn curvelinear4(fig: &Figure) -> CaseResult {
    let ax1 = floating_polar(fig, (120.0, 30.0, 10.0, 0.0), 5)?;
    ax1.clear();
    artist(&ax1, "left")?.label().set_text("Test 1");
    artist(&ax1, "right")?.label().set_text("Test 2");
    artist(&ax1, "top")?.set_visible(false);
    z_axis(&ax1, 70.0, Some(SpineSide::Top));
    scatter_path(&ax1)
}
