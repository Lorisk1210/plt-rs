//! `figures/cases_mplot3d_api.py`, settings part: equal aspects, ticks and
//! tick parameters, bounds and margins, non-linear scales and shared
//! axes.

use std::f64::consts::PI;

use pltrs::axes::{Adjustable, AxisName, TickLevel, TickParams};
use pltrs::collections::CollectionProps;
use pltrs::colors::{Colorize, ColorizerOptions, colormaps};
use pltrs::fonts::FontSize;
use pltrs::lines::LineProps;
use pltrs::mplot3d::{
    Aspect3D, Axes3D, Axes3DError, Axes3DOptions, Axis3, FaceColors, Sampling, SurfaceOptions,
    WireframeOptions,
};
use pltrs::numeric::{Grid, linspace, pow};
use pltrs::ticker::{AutoMinorLocator, MinorDivisions, MultipleLocator};

use super::surface;
use crate::figures::FigureCase;

/// `_ellipse_helix()`: two turns of an elliptic helix rising from 0 to 3.
pub(super) fn ellipse_helix() -> [Vec<f64>; 3] {
    let t = linspace(0.0, 4.0 * PI, 120);
    [
        t.iter().map(|t| 2.0 * t.cos()).collect(),
        t.iter().map(|t| 0.5 * t.sin()).collect(),
        t.iter().map(|t| 3.0 * t / (4.0 * PI)).collect(),
    ]
}

/// `ax.plot(*_ellipse_helix(), color=...)`.
fn plot_ellipse_helix(ax: &Axes3D, color: Option<&str>) -> Result<(), Axes3DError> {
    let [x, y, z] = ellipse_helix();
    let props = color.map_or_else(LineProps::default, |c| LineProps::default().color(c));
    ax.plot_with(&x, &y, &z, Axis3::Z, props).map(drop)
}

/// `ax.plot_wireframe(*_surface(), rstride=n, cstride=n, **props)`.
fn wireframe(ax: &Axes3D, stride: usize, props: CollectionProps) -> Result<(), Axes3DError> {
    let (x, y, z) = surface(20);
    let options = WireframeOptions {
        sampling: Sampling::Strides {
            rows: stride,
            cols: stride,
        },
        props,
        ..WireframeOptions::default()
    };
    ax.plot_wireframe(&x, &y, &z, options).map(drop)
}

/// `ax.plot_surface(x, y, z, cmap=name)`.
fn mapped_surface(
    ax: &Axes3D,
    (x, y, z): (Grid<f64>, Grid<f64>, Grid<f64>),
    cmap: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = SurfaceOptions {
        colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get(cmap)?)),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d/api_aspect_equal_box", |fig| {
        let ax = fig.subplot_3d();
        plot_ellipse_helix(&ax, None)?;
        ax.set_aspect(Aspect3D::Equal, Adjustable::Box)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_aspect_equal_datalim", |fig| {
        let ax = fig.subplot_3d();
        plot_ellipse_helix(&ax, None)?;
        ax.set_aspect(Aspect3D::Equal, Adjustable::Datalim)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_aspect_equalxz_box", |fig| {
        let ax = fig.subplot_3d();
        plot_ellipse_helix(&ax, Some("tab:green"))?;
        ax.set_aspect(Aspect3D::EqualXZ, Adjustable::Box)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_custom_ticks", |fig| {
        let ax = fig.subplot_3d();
        wireframe(
            &ax,
            2,
            CollectionProps::default().color("0.4").linewidth(0.5),
        )?;
        ax.xaxis()
            .set_ticks([-3.0, 0.0, 2.0])
            .set_ticklabels(["low", "zero", "two"])?;
        ax.yaxis()
            .set_major_locator(MultipleLocator::new(1.5)?)
            .set_minor_locator(AutoMinorLocator::new(Some(MinorDivisions::Count(3))));
        ax.zaxis().set_major_locator(MultipleLocator::new(0.25)?);
        ax.tick_params(TickParams {
            axis: Some(AxisName::X),
            colors: Some("tab:red".into()),
            labelsize: Some(FontSize::Points(7.0)),
            ..TickParams::default()
        });
        ax.zaxis().set_tick_params(TickParams {
            pad: Some(6.0),
            labelcolor: Some("tab:blue".into()),
            labelrotation: Some(20.0),
            ..TickParams::default()
        });
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_tick_params_hidden", |fig| {
        let ax = fig.subplot_3d();
        mapped_surface(&ax, surface(20), "viridis")?;
        ax.tick_params(TickParams {
            labelleft: Some(false),
            labelsize: Some(FontSize::Points(6.0)),
            which: TickLevel::Both,
            ..TickParams::default()
        });
        ax.yaxis().minorticks_on();
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_bounds_margins", |fig| {
        let ax = fig.subplot_3d();
        plot_ellipse_helix(&ax, Some("tab:purple"))?;
        ax.set_margins([Some(0.2); 3], Some(true))?;
        ax.set_zbound(Some(-1.0), None)?
            .set_ybound(Some(1.0), Some(-1.0))?
            .set_inverted(Axis3::Z, true)
            .set_margin(Axis3::X, 0.0)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_log_z", |fig| {
        let ax = fig.subplot_3d();
        ax.set_zscale("log")?;
        let (xx, yy, _) = surface(15);
        let zz = Grid::from_fn(15, 15, |i, j| {
            pow(10.0, 1.5 + xx[(i, j)].cos() + 0.5 * yy[(i, j)])
        });
        mapped_surface(&ax, (xx, yy, zz), "magma")
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_symlog_x", |fig| {
        let ax = fig.subplot_3d();
        let t = linspace(-1.0, 1.0, 60);
        let x: Vec<f64> = t.iter().map(|t| 100.0 * t.powf(3.0)).collect();
        let y: Vec<f64> = t.iter().map(|t| (3.0 * t).cos()).collect();
        let props = LineProps::fmt("o-")?.markersize(3.0);
        ax.plot_with(&x, &y, &t, Axis3::Z, props)?;
        ax.set_xscale("symlog")?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/api_shared_views", |fig| {
        let ax1 = fig.add_subplot_3d(1, 2, 1, Axes3DOptions::default())?;
        plot_ellipse_helix(&ax1, None)?;
        let ax2 = fig.add_subplot_3d(
            1,
            2,
            2,
            Axes3DOptions {
                shareview: Some(ax1.clone()),
                sharez: Some(ax1.clone()),
                ..Axes3DOptions::default()
            },
        )?;
        wireframe(&ax2, 3, CollectionProps::default())?;
        ax1.view_init_shared(15.0, 40.0, 10.0);
        ax2.set_zlim(-1.0, 2.0)?;
        Ok(())
    })
    .sized(8.0, 4.0, 100.0),
];
