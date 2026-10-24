//! Contours: `contour3d`, `contour3d_2`, `contour3d_3`, `contourf3d`,
//! `contourf3d_2` and `box3d`.

use std::num::NonZeroUsize;

use plt::collections::CollectionProps;
use plt::colorbar::ColorbarOptions;
use plt::colors::{ColorizerOptions, ColormapError, colormaps};
use plt::contour::{ContourOptions, ContourSet, Levels};
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::mplot3d::{
    Axes3D, Axes3DError, Axis3, Contour3DOptions, Sampling, SurfaceOptions, Vec3, test_data,
};
use plt::numeric::{Grid, linspace};

use super::{Fill, label_xyz};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/contour3d", contour3d),
    FigureCase::new("gallery_3d/contour3d_2", contour3d_2),
    FigureCase::new("gallery_3d/contour3d_3", contour3d_3),
    FigureCase::new("gallery_3d/contourf3d", contourf3d),
    FigureCase::new("gallery_3d/contourf3d_2", contourf3d_2),
    FigureCase::new("gallery_3d/box3d", box3d).sized(5.0, 4.0, 100.0),
];

/// `cmap="coolwarm"` contours.
fn coolwarm() -> Result<Contour3DOptions, ColormapError> {
    Ok(Contour3DOptions {
        contour: ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("coolwarm")?),
                ..ColorizerOptions::default()
            },
            ..ContourOptions::default()
        },
        ..Contour3DOptions::default()
    })
}

fn contour3d(fig: &Figure) -> CaseResult {
    let (x, y, z) = test_data(0.05);
    fig.subplot_3d().contour(&x, &y, &z, coolwarm()?)?;
    Ok(())
}

fn contour3d_2(fig: &Figure) -> CaseResult {
    let (x, y, z) = test_data(0.05);
    let options = Contour3DOptions {
        extend3d: NonZeroUsize::new(5),
        ..coolwarm()?
    };
    fig.subplot_3d().contour(&x, &y, &z, options)?;
    Ok(())
}

fn contourf3d(fig: &Figure) -> CaseResult {
    let (x, y, z) = test_data(0.05);
    fig.subplot_3d().contourf(&x, &y, &z, coolwarm()?)?;
    Ok(())
}

/// `contour3d_3` and `contourf3d_2`: a translucent surface and its
/// contours projected onto three panes.
fn projected_contours(fig: &Figure, fill: Fill) -> CaseResult {
    let ax = fig.subplot_3d();
    let (x, y, z) = test_data(0.05);
    let surface = SurfaceOptions {
        sampling: Sampling::Strides { rows: 8, cols: 8 },
        props: CollectionProps::default()
            .edgecolor("royalblue")
            .linewidth(0.5)
            .alpha(0.3),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, surface)?;
    for (zdir, offset) in [(Axis3::Z, -100.0), (Axis3::X, -40.0), (Axis3::Y, 40.0)] {
        let options = Contour3DOptions {
            zdir,
            offset: Some(offset),
            ..coolwarm()?
        };
        match fill {
            Fill::Lines => ax.contour(&x, &y, &z, options)?,
            Fill::Filled => ax.contourf(&x, &y, &z, options)?,
        };
    }
    ax.set_xlim(-40.0, 40.0)?
        .set_ylim(-40.0, 40.0)?
        .set_zlim(-100.0, 100.0)?;
    label_xyz(&ax, ["X", "Y", "Z"]);
    Ok(())
}

fn contour3d_3(fig: &Figure) -> CaseResult {
    projected_contours(fig, Fill::Lines)
}

fn contourf3d_2(fig: &Figure) -> CaseResult {
    projected_contours(fig, Fill::Filled)
}

/// The number of grid points of `box3d` along x, y and z.
const BOX: [usize; 3] = [100, 300, 500];

/// `box3d`'s field `((X + 100)**2 + (Y - 20)**2 + 2 Z) / 1000 + 1`; on
/// the grid its terms are whole numbers, which add up exactly.
fn box_field([x, y, z]: Vec3) -> f64 {
    ((x + 100.0).powi(2) + (y - 20.0).powi(2) + 2.0 * z) / 1000.0 + 1.0
}

/// A `rows × cols` face of the box: the coordinates of its grid points
/// (`point(i, j)`) and the field on them.
fn box_face(rows: usize, cols: usize, point: impl Fn(f64, f64) -> Vec3) -> [Grid<f64>; 4] {
    let points = Grid::from_fn(rows, cols, |i, j| point(i as f64, j as f64));
    [
        points.map(|p| p[0]),
        points.map(|p| p[1]),
        points.map(|p| p[2]),
        points.map(|&p| box_field(p)),
    ]
}

/// One of `box3d`'s faces: filled contours of the field at `levels`,
/// projected onto the plane at `offset` along `zdir`.
fn box_contours(
    ax: &Axes3D,
    [x, y, z]: [&Grid<f64>; 3],
    zdir: Axis3,
    offset: f64,
    levels: &[f64],
) -> Result<ContourSet, Axes3DError> {
    let options = Contour3DOptions {
        zdir,
        offset: Some(offset),
        contour: ContourOptions {
            colorizer: ColorizerOptions {
                vmin: levels.first().copied(),
                vmax: levels.last().copied(),
                ..ColorizerOptions::default()
            },
            levels: Levels::Values(levels.to_vec()),
            ..ContourOptions::default()
        },
        ..Contour3DOptions::default()
    };
    ax.contourf(x, y, z, options)
}

fn box3d(fig: &Figure) -> CaseResult {
    let [xmax, ymax, zmax] = BOX.map(|n| (n - 1) as f64);
    // The field is smallest at x = 0, y = 20 and the bottom, largest at
    // the far corner of the top.
    let (low, high) = (box_field([0.0, 20.0, -zmax]), box_field([xmax, ymax, 0.0]));
    let levels = linspace(low, high, 10);
    let ax = fig.subplot_3d();
    let [nx, ny, nz] = BOX;
    let [x, y, _, field] = box_face(ny, nx, |j, i| [i, j, 0.0]);
    box_contours(&ax, [&x, &y, &field], Axis3::Z, 0.0, &levels)?;
    let [x, _, z, field] = box_face(nx, nz, |i, k| [i, 0.0, -k]);
    box_contours(&ax, [&x, &field, &z], Axis3::Y, 0.0, &levels)?;
    let [_, y, z, field] = box_face(ny, nz, |j, k| [xmax, j, -k]);
    let set = box_contours(&ax, [&field, &y, &z], Axis3::X, xmax, &levels)?;
    ax.set_xlim(0.0, xmax)?
        .set_ylim(0.0, ymax)?
        .set_zlim(-zmax, 0.0)?;
    let edge = || LineProps::default().color("0.4").linewidth(1.0).zorder(1e3);
    ax.plot_with(&[xmax, xmax], &[0.0, ymax], &[0.0], Axis3::Z, edge())?;
    ax.plot_with(&[0.0, xmax], &[0.0, 0.0], &[0.0], Axis3::Z, edge())?;
    ax.plot_with(&[xmax, xmax], &[0.0, 0.0], &[-zmax, 0.0], Axis3::Z, edge())?;
    label_xyz(&ax, ["X [km]", "Y [km]", "Z [m]"]);
    ax.zaxis().set_ticks(&[0.0, -150.0, -300.0, -450.0]);
    ax.view_init(40.0, -30.0, 0.0);
    ax.set_box_aspect(None, 0.9)?;
    let colorbar = ColorbarOptions {
        fraction: 0.02,
        pad: Some(0.1),
        label: "Name [units]".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&set, colorbar)?;
    Ok(())
}
