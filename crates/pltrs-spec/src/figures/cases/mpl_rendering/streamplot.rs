//! The cases from `test_streamplot.py`: streamlines from given start
//! points, coloured, of varying width, through masks and NaNs, of limited
//! length, traced backwards, and integrated with other tolerances.

use pltrs::colorbar::ColorbarOptions;
use pltrs::colors::{ColorizerOptions, colormaps};
use pltrs::layout::TightLayout;
use pltrs::numeric::{Grid, linspace, meshgrid};
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::streamplot::{
    IntegrationDirection, StreamColorMap, StreamColors, StreamField, StreamWidths,
    StreamplotOptions,
};
use pltrs::transforms::Point;

use super::super::mpl_tests::remove_ticks_and_titles;
use crate::figures::{CaseResult, FigureCase};

/// The test module's `velocity_field`: `u = -1 - x² + y`, `v = 1 + x - y²`
/// on 200 × 100 points of `[-3, 3]²`.
fn velocity_field() -> StreamField {
    let (x, y) = (linspace(-3.0, 3.0, 200), linspace(-3.0, 3.0, 100));
    let u = Grid::from_fn(100, 200, |i, j| -1.0 - x[j] * x[j] + y[i]);
    let v = Grid::from_fn(100, 200, |i, j| 1.0 + x[j] - y[i] * y[i]);
    StreamField::new(x, y, u, v)
}

/// The test module's `swirl_velocity_field`: a rotation turned by 0.1 rad
/// on 200 × 100 points of `[-3, 3]²`.
fn swirl_velocity_field() -> StreamField {
    let (x, y) = (linspace(-3.0, 3.0, 200), linspace(-3.0, 3.0, 100));
    let (sin, cos) = 0.1_f64.sin_cos();
    let u = Grid::from_fn(100, 200, |i, j| cos * -y[i] - sin * x[j]);
    let v = Grid::from_fn(100, 200, |i, j| sin * -y[i] + cos * x[j]);
    StreamField::new(x, y, u, v)
}

/// Colours mapped from `values` through the colormap `cmap`.
fn mapped(values: Grid<f64>, cmap: &str) -> Result<StreamColors, pltrs::colors::ColormapError> {
    Ok(StreamColors::Mapped(Box::new(StreamColorMap {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get(cmap)?),
            norm: None,
            ..ColorizerOptions::default()
        },
        values,
    })))
}

/// `test_startpoints`: streamlines from a 5 × 5 grid of start points,
/// four arrows each, the start points drawn one column per line.
fn startpoints(fig: &Figure) -> CaseResult {
    let field = velocity_field();
    let starts_x = linspace(-3.0, 3.0, 5);
    let starts_y = linspace(-3.0, 3.0, 5);
    let start_points = starts_y
        .iter()
        .flat_map(|&y| starts_x.iter().map(move |&x| Point::new(x, y)))
        .collect();
    let ax = fig.subplot();
    let options = StreamplotOptions {
        start_points: Some(start_points),
        num_arrows: 4,
        ..StreamplotOptions::default()
    };
    ax.streamplot(&field, options)?;
    for &x in &starts_x {
        ax.plot_fmt(vec![x; 5], starts_y.clone(), "ok")?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_colormap`: streamlines coloured by `u` through `autumn`, with a
/// colorbar.
fn colormap(fig: &Figure) -> CaseResult {
    let field = velocity_field();
    let options = StreamplotOptions {
        color: Some(mapped(field.u.clone(), "autumn")?),
        density: (0.6, 0.6),
        linewidth: Some(StreamWidths::Uniform(2.0)),
        ..StreamplotOptions::default()
    };
    let stream = fig.subplot().streamplot(&field, options)?;
    fig.colorbar(&stream.lines, ColorbarOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_linewidth`: black streamlines as wide as the speed, two arrows
/// each.
fn linewidth(fig: &Figure) -> CaseResult {
    let field = velocity_field();
    let speed = Grid::from_fn(100, 200, |i, j| field.u[(i, j)].hypot(field.v[(i, j)]));
    let max = speed.as_slice().iter().copied().fold(f64::MIN, f64::max);
    let options = StreamplotOptions {
        density: (0.5, 1.0),
        color: Some(StreamColors::Single("k".into())),
        linewidth: Some(StreamWidths::Varying(speed.map(|s| 5.0 * s / max))),
        num_arrows: 2,
        ..StreamplotOptions::default()
    };
    fig.subplot().streamplot(&field, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_masks_and_nans`: a masked block and a NaN corner in `u`, which
/// also colours the streamlines.
fn masks_and_nans(fig: &Figure) -> CaseResult {
    let mut field = velocity_field();
    field.u = Grid::from_fn(100, 200, |i, j| {
        let masked = (40..60).contains(&i) && (80..120).contains(&j);
        let nan = i < 20 && j < 40;
        if masked || nan {
            f64::NAN
        } else {
            field.u[(i, j)]
        }
    });
    let options = StreamplotOptions {
        color: Some(mapped(field.u.clone(), "Blues")?),
        ..StreamplotOptions::default()
    };
    fig.subplot().streamplot(&field, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_maxlength` and `test_maxlength_no_broken`: one long streamline
/// of the swirl, with the old test image's upper limits.
fn maxlength(fig: &Figure, broken_streamlines: bool) -> CaseResult {
    let options = StreamplotOptions {
        maxlength: 10.0,
        start_points: Some(vec![Point::new(0.0, 1.5)]),
        linewidth: Some(StreamWidths::Uniform(2.0)),
        density: (2.0, 2.0),
        broken_streamlines,
        ..StreamplotOptions::default()
    };
    let ax = fig.subplot();
    ax.streamplot(&swirl_velocity_field(), options)?;
    ax.set_xlim(ax.xlim().0, 3.255_598_802_188_230_5)?
        .set_ylim(ax.ylim().0, 3.078_326_760_195_413)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_direction`: one streamline of the swirl traced backwards.
fn direction(fig: &Figure) -> CaseResult {
    let options = StreamplotOptions {
        integration_direction: IntegrationDirection::Backward,
        maxlength: 1.5,
        start_points: Some(vec![Point::new(1.5, 0.0)]),
        linewidth: Some(StreamWidths::Uniform(2.0)),
        density: (2.0, 2.0),
        ..StreamplotOptions::default()
    };
    fig.subplot().streamplot(&swirl_velocity_field(), options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The potential flow over a lifting cylinder of `test_integration_options`
/// on a 50 × 50 grid of `[-2, 2] × [-3, 3]`.
fn cylinder_flow() -> Result<StreamField, pltrs::streamplot::StreamplotError> {
    let (xx, yy) = meshgrid(&linspace(-2.0, 2.0, 50), &linspace(-3.0, 3.0, 50));
    let velocity = |x: f64, y: f64| {
        let th = y.atan2(x);
        let r = x.hypot(y);
        let vr = -th.cos() / (r * r);
        let vt = -th.sin() / (r * r) - 1.0 / r;
        (
            vr * th.cos() - vt * th.sin() + 1.0,
            vr * th.sin() + vt * th.cos(),
        )
    };
    let vx = Grid::from_fn(50, 50, |i, j| velocity(xx[(i, j)], yy[(i, j)]).0);
    let vy = Grid::from_fn(50, 50, |i, j| velocity(xx[(i, j)], yy[(i, j)]).1);
    StreamField::from_mesh(&xx, &yy, vx, vy)
}

/// `test_integration_options`: unbroken streamlines integrated with three
/// step and error tolerances around a cylinder, each with a zoomed inset.
fn integration_options(fig: &Figure) -> CaseResult {
    let field = cylinder_flow()?;
    let seeds: Vec<Point> = linspace(-2.0, 2.0, 50)
        .into_iter()
        .map(|y| Point::new(-1.75, y))
        .collect();
    let theta = linspace(0.0, 2.0 * std::f64::consts::PI, 100);
    let (circle_x, circle_y): (Vec<f64>, Vec<f64>) =
        theta.iter().map(|t| (t.cos(), t.sin())).unzip();
    let axs = fig.subplots(3, 1)?;
    for (ax, max_val) in axs.as_slice().iter().zip([0.05, 1.0, 5.0]) {
        let inset = ax.inset_axes([0.0, 0.7, 0.3, 0.35]);
        for (ax_curr, is_inset) in [(ax, false), (&inset, true)] {
            let options = StreamplotOptions {
                start_points: Some(seeds.clone()),
                broken_streamlines: false,
                arrowsize: 1e-10,
                linewidth: Some(StreamWidths::Uniform(if is_inset { 2.0 } else { 0.6 })),
                color: Some(StreamColors::Single("k".into())),
                integration_max_step_scale: max_val,
                integration_max_error_scale: max_val,
                ..StreamplotOptions::default()
            };
            ax_curr.streamplot(&field, options)?;
            let disc = PatchProps::default()
                .color("w")
                .edgecolor("k")
                .linewidth(if is_inset { 6.0 } else { 2.0 });
            ax_curr.fill(&circle_x, &circle_y, disc)?;
            ax_curr.set_aspect(Aspect::Equal)?;
        }
        inset.set_xlim(-1.2, -0.7)?.set_ylim(-0.8, -0.4)?;
        inset.set_yticks(Vec::<f64>::new());
        inset.set_xticks(Vec::<f64>::new());
        ax.set_ylim(-1.5, 1.5)?;
        ax.set_axis_off();
        ax.indicate_inset_zoom(&inset, PatchProps::default().edgecolor("k"));
    }
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_rendering/startpoints", startpoints),
    FigureCase::new("mpl_rendering/colormap", colormap),
    FigureCase::new("mpl_rendering/linewidth", linewidth),
    FigureCase::new("mpl_rendering/masks_and_nans", masks_and_nans),
    FigureCase::new("mpl_rendering/maxlength", |fig| maxlength(fig, true)),
    FigureCase::new("mpl_rendering/maxlength_no_broken", |fig| {
        maxlength(fig, false)
    }),
    FigureCase::new("mpl_rendering/direction", direction),
    FigureCase::new("mpl_rendering/integration_options", integration_options)
        .sized(6.0, 14.0, 100.0),
];
