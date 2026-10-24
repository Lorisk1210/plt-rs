//! Curvilinear grids and floating axes in `figures/cases_axisartist.py`.

use std::f64::consts::PI;

use plt::axes::{AxisName, BarOptions, GridOptions, ImshowOptions, Mesh, PcolorOptions};
use plt::axisartist::angle_helper::{FormatterHms, LocatorHms};
use plt::axisartist::floating_axes::{self, FloatingAxes};
use plt::axisartist::grid_finder::{
    DictFormatter, ExtremeFinderSimple, FixedLocator, GridFinderOptions, MaxNLocator,
};
use plt::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
use plt::axisartist::{self, AuxTransform, Toggle};
use plt::colors::colormaps;
use plt::contour::{ContourOptions, QuadCoords, QuadField};
use plt::gridspec::SubplotOverrides;
use plt::image::ColorizerOptions;
use plt::image::Origin;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::text::AnnotateOptions;
use plt::text::{HAlign, VAlign};
use plt::transforms::Affine2D;
use plt_core::Point;

use crate::figures::CaseResult;
use crate::figures::cases::common::{
    axis_artist, polar_box, polar_degrees, polar_grid_helper, show_label,
};
use crate::rng::SpecRng;

/// numpy's `sign`.
fn sign(x: f64) -> f64 {
    if x == 0.0 || x.is_nan() {
        x
    } else {
        x.signum()
    }
}

/// `np.arange(rows * cols).reshape(rows, cols)` as floats.
fn arange_grid(rows: usize, cols: usize) -> Grid<f64> {
    Grid::from_fn(rows, cols, |r, c| (r * cols + c) as f64)
}

/// The grid of matplotlib's `demo_curvelinear_grid`: `(x, y - x)`.
fn shear() -> AuxTransform {
    AuxTransform::from_fns(|x, y| (x, y - x), |x, y| (x, y + x))
}

/// A change to the floating axis of `simple_axis_pad`.
type Adjustment = fn(&axisartist::AxisArtist);

pub(super) fn simple_axis_pad(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.01),
        right: Some(0.99),
        bottom: Some(0.01),
        top: Some(0.99),
        wspace: Some(0.01),
        hspace: Some(0.01),
    })?;
    let adjustments: [(&str, Adjustment); 4] = [
        ("default", |_| {}),
        ("ticklabels.set_pad(10)", |axis| {
            axis.major_ticklabels().set_pad(10.0);
        }),
        ("label.set_pad(20)", |axis| {
            axis.label().set_pad(20.0);
        }),
        ("ticks.set_tick_out(True)", |axis| {
            axis.major_ticks().set_tick_out(true);
        }),
    ];
    for (i, (note, adjust)) in adjustments.into_iter().enumerate() {
        let ax = polar_box(fig, (1, 4, i + 1))?;
        for axis in ax.axis_artists() {
            axis.set_visible(false);
        }
        let axis = ax.new_floating_axis("lat", AxisName::X, 30.0);
        show_label(&axis, r"$\theta = 30^{\circ}$");
        adjust(&axis);
        ax.ax()
            .annotate(
                note,
                (0.5, 1.0),
                AnnotateOptions {
                    xytext: Some(Point::new(5.0, -5.0)),
                    xycoords: "axes fraction".parse()?,
                    textcoords: Some("offset points".parse()?),
                    ..AnnotateOptions::default()
                },
            )?
            .set_valign(VAlign::Top)
            .set_halign(HAlign::Center);
    }
    Ok(())
}

pub(super) fn curvelinear_grid(fig: &Figure) -> CaseResult {
    let helper = GridHelperCurveLinear::new(shear(), GridFinderOptions::default());
    let ax1 = axisartist::Axes::with_grid_helper(fig.add_subplot(1, 2, 1)?, helper);
    ax1.ax().plot([3.0, 6.0], [2.0, 4.0])?;
    ax1.ax().set_aspect(Aspect::Equal)?;
    ax1.ax().set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    ax1.new_floating_axis("t", AxisName::X, 3.0);
    ax1.new_floating_axis("t2", AxisName::Y, 7.0);
    grid_below(&ax1);

    let ax2 =
        axisartist::Axes::with_grid_helper(fig.add_subplot(1, 2, 2)?, polar_grid_helper(None));
    for name in ["right", "top"] {
        axis_artist(&ax2, name)?
            .major_ticklabels()
            .set_visible(true);
    }
    axis_artist(&ax2, "right")?.set_nth_coord_ticks(AxisName::X);
    axis_artist(&ax2, "bottom")?.set_nth_coord_ticks(AxisName::Y);
    ax2.ax().set_aspect(Aspect::Equal)?;
    ax2.ax().set_xlim(-5.0, 12.0).set_ylim(-5.0, 10.0);
    grid_below(&ax2);
    let aux = ax2.aux_axes(&polar_degrees());
    aux.plot_with(
        linspace(0.0, 30.0, 51),
        vec![10.0; 51],
        LineProps::default().linewidth(2.0),
    )?;
    let (x, y) = (linspace(0.0, 90.0, 4), linspace(0.0, 10.0, 4));
    aux.pcolor(
        Mesh::from_coords(x.clone(), y.clone(), arange_grid(3, 3)),
        PcolorOptions::default(),
    )?;
    aux.contour(
        &QuadField::new(arange_grid(4, 4)).with_coords(QuadCoords::Axes { x, y }),
        ContourOptions {
            colors: Some(vec!["k".into()]),
            ..ContourOptions::default()
        },
    )?;
    Ok(())
}

/// `ax.grid(True, zorder=0)`.
fn grid_below(ax: &axisartist::Axes) {
    ax.grid(true, GridOptions::default());
    if let Some(gridlines) = ax.gridlines() {
        gridlines.set_zorder(0.0);
    }
}

pub(super) fn curvelinear_grid2(fig: &Figure) -> CaseResult {
    let tr = AuxTransform::from_fns(
        |x, y| (sign(x) * x.abs().sqrt(), y),
        |x, y| (sign(x) * (x * x), y),
    );
    let helper = GridHelperCurveLinear::new(
        tr,
        GridFinderOptions {
            extreme_finder: Some(Box::new(ExtremeFinderSimple::new(20, 20))),
            grid_locator1: Some(Box::new(MaxNLocator::new(6))),
            grid_locator2: Some(Box::new(MaxNLocator::new(6))),
            ..GridFinderOptions::default()
        },
    );
    let ax = axisartist::Axes::with_grid_helper(fig.subplot(), helper);
    ax.ax().imshow(
        arange_grid(5, 5),
        ImshowOptions {
            colors: ColorizerOptions::default()
                .cmap(colormaps::get("gray_r")?)
                .limits(None, Some(50.0)),
            origin: Some(Origin::Lower),
            ..ImshowOptions::default()
        },
    )?;
    Ok(())
}

pub(super) fn floating_axes(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        wspace: Some(0.3),
        left: Some(0.05),
        right: Some(0.95),
        ..SubplotOverrides::default()
    })?;
    let mut data = SpecRng::new(19_680_801);

    let tr1 = AuxTransform::affine(Affine2D::scaling(2.0, 1.0).rotate_deg(30.0));
    let helper1 = floating_axes::GridHelperCurveLinear::new(
        tr1.clone(),
        (-0.5, 3.5, 0.0, 4.0),
        GridFinderOptions {
            grid_locator1: Some(Box::new(MaxNLocator::new(4))),
            grid_locator2: Some(Box::new(MaxNLocator::new(4))),
            ..GridFinderOptions::default()
        },
    );
    let ax1 = FloatingAxes::new(fig.add_subplot(1, 3, 1)?, helper1)?;
    ax1.grid(true, GridOptions::default());
    ax1.aux_axes(&tr1).bar(
        [0.0, 1.0, 2.0, 3.0],
        [3.0, 2.0, 1.0, 3.0],
        BarOptions::default(),
    )?;

    let tr2 = AuxTransform::polar();
    let angle_ticks = [
        (0.0, r"$0$"),
        (0.25 * PI, r"$\frac{1}{4}\pi$"),
        (0.5 * PI, r"$\frac{1}{2}\pi$"),
    ];
    let helper2 = floating_axes::GridHelperCurveLinear::new(
        tr2.clone(),
        (0.5 * PI, 0.0, 2.0, 1.0),
        GridFinderOptions {
            grid_locator1: Some(Box::new(FixedLocator::new(angle_ticks.map(|(v, _)| v)))),
            grid_locator2: Some(Box::new(MaxNLocator::new(2))),
            tick_formatter1: Some(Box::new(DictFormatter::new(
                angle_ticks.map(|(v, s)| (v, s.to_owned())),
                None,
            ))),
            ..GridFinderOptions::default()
        },
    );
    let ax2 = FloatingAxes::new(fig.add_subplot(1, 3, 2)?, helper2)?;
    ax2.grid(true, GridOptions::default());
    let aux2 = ax2.aux_axes(&tr2);
    // `aux2.patch = ax2.patch` and `ax2.patch.zorder = 0.9`.
    ax2.share_patch(&aux2);
    ax2.ax().set_patch_zorder(0.9);
    let theta: Vec<f64> = data
        .uniform(10, 0.0, 1.0)
        .iter()
        .map(|u| u * 0.5 * PI)
        .collect();
    let radius: Vec<f64> = data.uniform(10, 0.0, 1.0).iter().map(|u| u + 1.0).collect();
    aux2.scatter(theta, radius)?;

    let tr3 = AuxTransform::affine(Affine2D::translation(-95.0, 0.0))
        .then(AuxTransform::affine(Affine2D::scaling(PI / 180.0, 1.0)))
        .then(AuxTransform::polar());
    let helper3 = floating_axes::GridHelperCurveLinear::new(
        tr3.clone(),
        (8.0 * 15.0, 14.0 * 15.0, 0.0, 14000.0),
        GridFinderOptions {
            grid_locator1: Some(Box::new(LocatorHms::new(4))),
            tick_formatter1: Some(Box::new(FormatterHms)),
            grid_locator2: Some(Box::new(MaxNLocator::new(3))),
            ..GridFinderOptions::default()
        },
    );
    let ax3 = FloatingAxes::new(fig.add_subplot(1, 3, 3)?, helper3)?;
    let left = axis_artist(&ax3, "left")?;
    left.set_axis_direction(SpineSide::Bottom);
    axis_artist(&ax3, "right")?.set_axis_direction(SpineSide::Top);
    axis_artist(&ax3, "bottom")?.set_visible(false);
    let top = axis_artist(&ax3, "top")?;
    top.set_axis_direction(SpineSide::Bottom);
    top.toggle(&Toggle {
        ticklabels: Some(true),
        label: Some(true),
        ..Toggle::default()
    });
    top.major_ticklabels().set_axis_direction(SpineSide::Top);
    top.label().set_axis_direction(SpineSide::Top);
    left.label().set_text(r"cz [km$^{-1}$]");
    top.label().set_text(r"$\alpha_{1950}$");
    ax3.grid(true, GridOptions::default());
    let aux3 = ax3.aux_axes(&tr3);
    ax3.share_patch(&aux3);
    ax3.ax().set_patch_zorder(0.9);
    let theta: Vec<f64> = data
        .uniform(10, 0.0, 1.0)
        .iter()
        .map(|u| (8.0 + u * (14.0 - 8.0)) * 15.0)
        .collect();
    let radius: Vec<f64> = data
        .uniform(10, 0.0, 1.0)
        .iter()
        .map(|u| u * 14000.0)
        .collect();
    aux3.scatter(theta, radius)?;
    Ok(())
}

pub(super) fn floating_axis(fig: &Figure) -> CaseResult {
    let ax = axisartist::Axes::with_grid_helper(fig.subplot(), polar_grid_helper(None));
    let lat = ax.new_floating_axis("lat", AxisName::X, 60.0);
    show_label(&lat, r"$\theta = 60^{\circ}$");
    let lon = ax.new_floating_axis("lon", AxisName::Y, 6.0);
    lon.label().set_text(r"$r = 6$");
    ax.ax().set_aspect(Aspect::Equal)?;
    ax.ax().set_xlim(-5.0, 12.0).set_ylim(-5.0, 10.0);
    ax.grid(true, GridOptions::default());
    Ok(())
}
