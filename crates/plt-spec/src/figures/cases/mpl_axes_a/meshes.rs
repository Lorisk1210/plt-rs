//! `pcolormesh` and `pcolor`: curvilinear grids with masked values, edges
//! and Gouraud shading, alpha from the call and from the colormap, values
//! at the grid points, log axes, datetime coordinates and snapping.

use plt::axes::{Mesh, PcolorOptions, RgbaMesh, Shading, TickLabelProps, TickLevel};
use plt::collections::{CollectionProps, EdgeColors};
use plt::colors::{Colormap, colormaps};
use plt::dates::DateTime;
use plt::image::{ColorizerOptions, ScalarField};
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::text::HAlign;
use plt_core::Rgba;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;
use crate::rng::SpecRng;

/// The seed the tests give `np.random.seed`.
const SEED: u64 = 19_680_801;

/// `f(a[i, j], b[i, j])` for every cell.
fn zip_grids(a: &Grid<f64>, b: &Grid<f64>, f: impl Fn(f64, f64) -> f64) -> Grid<f64> {
    let (rows, cols) = a.shape();
    Grid::from_fn(rows, cols, |r, c| f(a[(r, c)], b[(r, c)]))
}

/// `a[:-1, :-1]`.
fn trimmed<T: Clone>(a: &Grid<T>) -> Grid<T> {
    let (rows, cols) = a.shape();
    Grid::from_fn(rows - 1, cols - 1, |r, c| a[(r, c)].clone())
}

/// `(z - z.min()) / np.ptp(z)`.
fn normalized(z: &Grid<f64>) -> Grid<f64> {
    let values = z.as_slice();
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    z.map(|v| (v - min) / (max - min))
}

/// The `n`-column, `2n`-row grid of the pcolormesh tests.
fn square_grid(n: usize) -> (Grid<f64>, Grid<f64>) {
    meshgrid(&linspace(-1.5, 1.5, n), &linspace(-1.5, 1.5, n * 2))
}

/// `_quad_grid(n)`: the curvilinear grid and the normalised radius.
fn quad_grid(n: usize) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (xx, yy) = square_grid(n);
    let qx = zip_grids(&xx, &yy, |x, y| y.cos() - x.cos() + 1.1);
    let qz = zip_grids(&xx, &yy, |x, y| y.sin() + x.sin());
    let z = normalized(&zip_grids(&xx, &yy, |x, y| x.hypot(y) / 5.0));
    (qx, qz, z)
}

/// `np.ma.masked_where(hide(qz), z)`, also trimmed to the cells when `flat`.
fn masked_where(
    z: &Grid<f64>,
    qz: &Grid<f64>,
    hide: impl Fn(f64) -> bool,
    flat: bool,
) -> Result<ScalarField, Box<dyn std::error::Error>> {
    let mask = qz.map(|&v| hide(v));
    Ok(if flat {
        ScalarField::masked(trimmed(z), &trimmed(&mask))?
    } else {
        ScalarField::masked(z.clone(), &mask)?
    })
}

/// Cell edges of `width` points in `colors`.
fn edged(width: f64, colors: &[&str]) -> PcolorOptions {
    PcolorOptions {
        props: CollectionProps {
            edgecolors: Some(EdgeColors::Colors(
                colors.iter().map(|&c| c.into()).collect(),
            )),
            linewidths: Some(vec![width]),
            ..CollectionProps::default()
        },
        ..PcolorOptions::default()
    }
}

/// Gouraud shading.
fn gouraud() -> PcolorOptions {
    PcolorOptions {
        shading: Some(Shading::Gouraud),
        ..PcolorOptions::default()
    }
}

pub(super) fn pcolormesh(fig: &Figure) -> CaseResult {
    let (qx, qz, z) = quad_grid(12);
    let qz_max = qz
        .as_slice()
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let hide = |v: f64| v.abs() < 0.5 * qz_max;
    let axs = fig.subplots(1, 3)?;
    let mesh = |flat| -> Result<Mesh, Box<dyn std::error::Error>> {
        Ok(Mesh::from_coords(
            qx.clone(),
            qz.clone(),
            masked_where(&z, &qz, hide, flat)?,
        ))
    };
    axs[(0, 0)].pcolormesh(mesh(true)?, edged(0.5, &["k"]))?;
    axs[(0, 1)].pcolormesh(mesh(true)?, edged(2.0, &["b", "w"]))?;
    axs[(0, 2)].pcolormesh(mesh(false)?, gouraud())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn pcolormesh_small(fig: &Figure) -> CaseResult {
    let (qx, qz, z) = quad_grid(3);
    let qz_max = qz
        .as_slice()
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let hide = |v: f64| v.abs() < 0.5 * qz_max;
    let hide_low = |v: f64| v < -0.5 * qz_max;
    let axs = fig.subplots(2, 2)?;
    let mesh = |values| Mesh::from_coords(qx.clone(), qz.clone(), values);
    axs[(0, 0)].pcolormesh(mesh(masked_where(&z, &qz, hide, true)?), edged(0.5, &["k"]))?;
    axs[(0, 1)].pcolormesh(
        mesh(masked_where(&z, &qz, hide, true)?),
        edged(2.0, &["b", "w"]),
    )?;
    axs[(1, 0)].pcolormesh(mesh(masked_where(&z, &qz, hide, false)?), gouraud())?;
    axs[(1, 1)].pcolormesh(mesh(masked_where(&z, &qz, hide_low, false)?), gouraud())?;
    for ax in fig.axes() {
        ax.set_axis_off();
    }
    Ok(())
}

pub(super) fn pcolormesh_alpha(fig: &Figure) -> CaseResult {
    let (xx, yy) = square_grid(12);
    let qy = zip_grids(&xx, &yy, |x, y| y + x.sin());
    let z = normalized(&zip_grids(&xx, &yy, |x, y| x.hypot(y) / 5.0));
    let vir = colormaps::get("viridis")?.resampled(16)?;
    let varying: Vec<Color> = (0..16_i32)
        .map(|i| {
            let mut color = vir.sample_index(i64::from(i));
            color.a = 0.5 + 0.5 * f64::from(i).sin();
            color.into()
        })
        .collect();
    let cmap = Colormap::listed("from_list", varying)?;
    let axs = fig.subplots(2, 2)?;
    for ax in axs.as_slice() {
        ax.add_patch(Rectangle::new((0.0, -1.5), 1.5, 3.0))
            .set_facecolor([0.7, 0.1, 0.1, 0.5])
            .set_zorder(0.0);
    }
    let options = |cmap: &Colormap, shading, alpha: Option<f64>| {
        let props = CollectionProps::default().zorder(1.0);
        PcolorOptions {
            colors: ColorizerOptions::default().cmap(cmap.clone()),
            shading: Some(shading),
            props: match alpha {
                Some(alpha) => props.alpha(alpha),
                None => props,
            },
            ..PcolorOptions::default()
        }
    };
    let flat = || Mesh::from_coords(xx.clone(), qy.clone(), trimmed(&z));
    let points = || Mesh::from_coords(xx.clone(), qy.clone(), z.clone());
    axs[(0, 0)].pcolormesh(flat(), options(&vir, Shading::Flat, Some(0.4)))?;
    axs[(0, 1)].pcolormesh(points(), options(&vir, Shading::Gouraud, Some(0.4)))?;
    axs[(1, 0)].pcolormesh(flat(), options(&cmap, Shading::Flat, None))?;
    axs[(1, 1)].pcolormesh(points(), options(&cmap, Shading::Gouraud, None))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `pcolormesh(np.ones((5, 6, channels)) / 2)`: mid-grey cells, half
/// transparent when the fourth channel is there too.
fn grey_rgba(fig: &Figure, alpha: f64) -> CaseResult {
    let colors = Grid::from_fn(5, 6, |_, _| Rgba::new(0.5, 0.5, 0.5, alpha));
    fig.subplot()
        .pcolormesh_rgba(RgbaMesh::new(colors), PcolorOptions::default())?;
    Ok(())
}

pub(super) fn pcolormesh_rgba_3(fig: &Figure) -> CaseResult {
    grey_rgba(fig, 1.0)
}

pub(super) fn pcolormesh_rgba_4(fig: &Figure) -> CaseResult {
    grey_rgba(fig, 0.5)
}

pub(super) fn pcolormesh_nearest_noargs(fig: &Figure) -> CaseResult {
    let c = Grid::from_fn(7, 4, |r, c| (r + c) as f64);
    let options = PcolorOptions {
        shading: Some(Shading::Nearest),
        ..PcolorOptions::default()
    };
    fig.subplot().pcolormesh(Mesh::new(c), options)?;
    Ok(())
}

/// `_log_scale_mesh`: a mesh from x = 0 to 1 on a log x axis, by
/// `pcolor` (cells) or `pcolormesh` (values at the points).
fn log_scale_mesh(fig: &Figure, cells: bool) -> CaseResult {
    let mut x = linspace(0.0, 1.0, 11);
    if cells {
        x[1] += 0.00001;
    }
    let (xx, yy) = meshgrid(&x, &linspace(1.0, 2.0, 5));
    let c = zip_grids(&xx, &yy, |x, y| x + y);
    let ax = fig.subplot();
    if cells {
        ax.pcolor(
            Mesh::from_coords(xx, yy, trimmed(&c)),
            PcolorOptions::default(),
        )?;
    } else {
        ax.pcolormesh(Mesh::from_coords(xx, yy, c), PcolorOptions::default())?;
    }
    ax.set_xscale("log")?;
    Ok(())
}

pub(super) fn pcolormesh_log_scale(fig: &Figure) -> CaseResult {
    log_scale_mesh(fig, false)
}

pub(super) fn pcolor_log_scale(fig: &Figure) -> CaseResult {
    log_scale_mesh(fig, true)
}

/// `n` datetimes `step_hours` apart from `start`.
fn datetimes(
    start: DateTime,
    step_hours: i64,
    n: i64,
) -> Result<Vec<DateTime>, Box<dyn std::error::Error>> {
    const HOUR: i64 = 3_600_000_000;
    let origin = start.timestamp_micros();
    Ok((0..n)
        .map(|i| DateTime::from_timestamp_micros(origin + i * step_hours * HOUR))
        .collect::<Result<_, _>>()?)
}

/// `_datetime_mesh`: 1-D and 2-D datetime coordinates, with and without
/// the extra row and column, under rotated tick labels.
fn datetime_mesh(fig: &Figure, cells: bool) -> CaseResult {
    fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        hspace: Some(0.4),
        top: Some(0.98),
        bottom: Some(0.15),
        ..plt::gridspec::SubplotOverrides::default()
    })?;
    let days = datetimes(DateTime::new(2013, 1, 1, 0, 0, 0, 0)?, 24, 21)?;
    let y = arange(0.0, 21.0, 1.0);
    let z = Grid::from_fn(20, 20, |r, c| (r * c) as f64);
    for index in 1..=4 {
        let ax = fig.add_subplot(2, 2, index)?;
        // pcolor and pcolormesh convert the dates as matplotlib's do.
        let x = ax.xaxis().convert_units(days.clone());
        let n = if index % 2 == 1 { 20 } else { 21 };
        let values = Grid::from_fn(n - 1, n - 1, |r, c| z[(r, c)]);
        let mesh = if index <= 2 {
            Mesh::from_coords(x[..n].to_vec(), y[..n].to_vec(), values)
        } else {
            let xx = Grid::from_fn(n, n, |_, c| x[c]);
            let yy = Grid::from_fn(n, n, |r, _| y[r]);
            Mesh::from_coords(xx, yy, values)
        };
        if cells {
            ax.pcolor(mesh, PcolorOptions::default())?;
        } else {
            ax.pcolormesh(mesh, PcolorOptions::default())?;
        }
    }
    let rotated = TickLabelProps {
        rotation: Some(30.0),
        halign: Some(HAlign::Right),
        ..TickLabelProps::default()
    };
    for ax in fig.axes() {
        ax.xaxis()
            .set_ticklabel_props(TickLevel::Major, rotated.clone());
    }
    Ok(())
}

pub(super) fn pcolormesh_datetime_axis(fig: &Figure) -> CaseResult {
    datetime_mesh(fig, false)
}

pub(super) fn pcolor_datetime_axis(fig: &Figure) -> CaseResult {
    datetime_mesh(fig, true)
}

/// `n` standard normal values in `rows` rows.
fn normal_grid(rows: usize, cols: usize) -> Result<Grid<f64>, Box<dyn std::error::Error>> {
    let values = SpecRng::new(SEED).normal(rows * cols, 0.0, 1.0);
    Ok(Grid::from_vec(rows, cols, values)?)
}

/// Flat shading.
fn flat() -> PcolorOptions {
    PcolorOptions {
        shading: Some(Shading::Flat),
        ..PcolorOptions::default()
    }
}

pub(super) fn pcolornearest(fig: &Figure) -> CaseResult {
    let mesh = Mesh::from_coords(
        arange(0.0, 10.0, 1.0),
        arange(0.0, 3.0, 1.0),
        normal_grid(2, 9)?,
    );
    fig.subplot().pcolormesh(mesh, flat())?;
    Ok(())
}

pub(super) fn pcolornearestunits(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let hours = datetimes(DateTime::new(2020, 1, 1, 0, 0, 0, 0)?, 1, 10)?;
    let x = ax.xaxis().convert_units(hours);
    let mesh = Mesh::from_coords(x, arange(0.0, 3.0, 1.0), normal_grid(2, 9)?);
    ax.pcolormesh(mesh, flat())?;
    Ok(())
}

/// `_pcolorauto`: automatic shading of cells, snapped or not.
fn pcolorauto(fig: &Figure, snap: bool) -> CaseResult {
    let mesh = Mesh::from_coords(
        arange(0.0, 10.0, 1.0),
        arange(0.0, 4.0, 1.0),
        normal_grid(3, 9)?,
    );
    let options = PcolorOptions {
        snap: Some(snap),
        ..PcolorOptions::default()
    };
    fig.subplot().pcolormesh(mesh, options)?;
    Ok(())
}

pub(super) fn pcolorauto_snap_false(fig: &Figure) -> CaseResult {
    pcolorauto(fig, false)
}

pub(super) fn pcolorauto_snap_true(fig: &Figure) -> CaseResult {
    pcolorauto(fig, true)
}
