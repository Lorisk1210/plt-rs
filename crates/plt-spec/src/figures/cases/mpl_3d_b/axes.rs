//! The tests of the axes themselves: label pads, clearing, views and
//! projections, inversion, tick label formats, minor ticks, box aspects,
//! shared views and pane colours.

use plt::axes::TicklabelFormat;
use plt::mplot3d::{Axes3D, Axes3DError, Axes3DOptions, Axis3, ProjType, SurfaceOptions};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::ticker::OffsetMode;

use super::{cube_edges, remove_ticks_and_titles, subplots_3d};
use crate::figures::CaseResult;

pub(super) fn axes3d_labelpad(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes_3d([0.0, 0.0, 1.0, 1.0], Axes3DOptions::default())?;
    ax.set_xlabel("X LABEL");
    ax.xaxis().set_label_pad(10.0);
    ax.set_ylabel("Y LABEL");
    ax.set_zlabel("Z LABEL");
    ax.zaxis().set_label_pad(20.0);
    ax.yaxis().set_label_pad(20.0);
    ax.zaxis().set_label_pad(-40.0);
    let pads: Vec<f64> = (ax.yaxis().major_tick_pads()?.into_iter().enumerate())
        .map(|(i, pad)| pad + 5.0 - i as f64 * 5.0)
        .collect();
    ax.yaxis().set_major_tick_pads(pads);
    Ok(())
}

pub(super) fn axes3d_cla(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    ax.set_axis_off();
    ax.cla()?;
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn axes3d_rotated(fig: &Figure) -> CaseResult {
    fig.subplot_3d().view_init(90.0, 45.0, 0.0);
    Ok(())
}

pub(super) fn culling(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let xs = linspace(0.0, -100.0, 101);
    ax.plot_fmt(&xs, [1.0; 101], [0.0; 101], "k")?;
    ax.set_xlim(-5.0, 5.0)?;
    ax.set_ylim(-5.0, 5.0)?;
    ax.set_zlim(-5.0, 5.0)?;
    ax.view_init(5.0, 180.0, 0.0);
    Ok(())
}

pub(super) fn axes3d_focal_length(fig: &Figure) -> CaseResult {
    let axs = subplots_3d(fig, 1, 2)?;
    axs[0].set_proj_type(ProjType::Persp {
        focal_length: f64::INFINITY,
    })?;
    axs[1].set_proj_type(ProjType::Persp { focal_length: 0.15 })?;
    Ok(())
}

pub(super) fn axes3d_ortho(fig: &Figure) -> CaseResult {
    fig.subplot_3d().set_proj_type(ProjType::Ortho)?;
    Ok(())
}

pub(super) fn axes3d_isometric(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d_with(Axes3DOptions {
        proj_type: ProjType::Ortho,
        box_aspect: Some([4.0; 3]),
        ..Axes3DOptions::default()
    })?;
    cube_edges(&ax, [-1.0, 1.0], &LineProps::default().color("k"))?;
    let elev = (1.0 / 2.0_f64.sqrt()).atan().to_degrees();
    ax.view_init(elev, -45.0, 0.0);
    ax.grid(true);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The line of the `inverted` tests.
fn inverted_line(ax: &Axes3D) -> Result<(), Axes3DError> {
    let line = ax.plot(
        [1.0, 1.0, 10.0, 10.0],
        [1.0, 10.0, 10.0, 10.0],
        [1.0, 1.0, 1.0, 10.0],
    );
    line.map(drop)
}

pub(super) fn inverted(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    inverted_line(&ax)?;
    ax.invert(Axis3::Y)?;
    Ok(())
}

pub(super) fn inverted_ref(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    ax.invert(Axis3::Y)?;
    inverted_line(&ax)?;
    Ok(())
}

pub(super) fn ticklabel_format(fig: &Figure) -> CaseResult {
    let axs = subplots_3d(fig, 4, 5)?;
    for ax in &axs {
        ax.set_xlim(1e7, 1e7 + 10.0)?;
    }
    let rc = plt::rc::params();
    let formats = [
        TicklabelFormat {
            scientific: Some(false),
            ..TicklabelFormat::default()
        },
        TicklabelFormat {
            scilimits: Some((-2, 2)),
            ..TicklabelFormat::default()
        },
        TicklabelFormat {
            use_offset: Some(if rc.axes_formatter_useoffset {
                OffsetMode::Off
            } else {
                OffsetMode::Auto
            }),
            ..TicklabelFormat::default()
        },
        // `useLocale` changes nothing in the C locale.
        TicklabelFormat::default(),
        TicklabelFormat {
            use_mathtext: Some(!rc.axes_formatter_use_mathtext),
            ..TicklabelFormat::default()
        },
    ];
    let rows = [Some(Axis3::X), Some(Axis3::Y), Some(Axis3::Z), None];
    for (row, axis) in axs.chunks(5).zip(rows) {
        for (ax, format) in row.iter().zip(&formats) {
            ax.ticklabel_format(axis, format)?;
        }
    }
    Ok(())
}

pub(super) fn minor_ticks(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let ticks = [(Axis3::X, 0.25, "quarter"), (Axis3::Y, 0.33, "third")];
    for (dir, tick, label) in ticks.into_iter().chain([(Axis3::Z, 0.5, "half")]) {
        ax.axis(dir)
            .set_minor_ticks(&[tick])
            .set_minor_ticklabels([label])?;
    }
    Ok(())
}

pub(super) fn equal_box_aspect(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let u = linspace(0.0, 2.0 * std::f64::consts::PI, 100);
    let v = linspace(0.0, std::f64::consts::PI, 100);
    let outer = |f: &dyn Fn(f64, f64) -> f64| Grid::from_fn(100, 100, |i, j| f(u[i], v[j]));
    let x = outer(&|u, v| u.cos() * v.sin());
    let y = outer(&|u, v| u.sin() * v.sin());
    let z = outer(&|_, v| v.cos());
    ax.plot_surface(&x, &y, &z, SurfaceOptions::default())?;
    cube_edges(&ax, [-1.0, 1.0], &LineProps::default().color("b"))?;
    let (lows, highs): (Vec<f64>, Vec<f64>) = [ax.xlim(), ax.ylim(), ax.zlim()].into_iter().unzip();
    let low = lows.iter().copied().fold(f64::INFINITY, f64::min);
    let high = highs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for dir in Axis3::ALL {
        ax.set_limits(dir, low, high)?;
    }
    ax.set_axis_off();
    ax.set_box_aspect(Some([1.0; 3]), 1.0)?;
    Ok(())
}

pub(super) fn shared_view(fig: &Figure) -> CaseResult {
    let ax1 = fig.add_subplot_3d(1, 3, 1, Axes3DOptions::default())?;
    let ax2 = fig.add_subplot_3d(
        1,
        3,
        2,
        Axes3DOptions {
            shareview: Some(ax1.clone()),
            ..Axes3DOptions::default()
        },
    )?;
    let ax3 = fig.add_subplot_3d(1, 3, 3, Axes3DOptions::default())?;
    ax3.share_view(&ax1)?;
    ax2.view_init_shared(5.0, 20.0, 30.0);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn panecolor_rcparams(fig: &Figure) -> CaseResult {
    fig.subplot_3d();
    remove_ticks_and_titles(fig);
    Ok(())
}
