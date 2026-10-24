//! `test_contour.py`: replaced paths, manual and nearby labels, given
//! colours with extensions, hatch colours, log levels, date axes, corner
//! masks, log extensions, colorbar lines, closed loops, contours given as
//! segments, algorithms, clipping and line style aliases.

use std::f64::consts::PI;

use plt::artist::ClipPathSpec;
use plt::axes::{Mesh, PcolorOptions, TickLabelProps, TickLevel};
use plt::colorbar::{ColorbarOptions, Spacing};
use plt::colors::{ColorizerOptions, Extend, LogNorm, colormaps};
use plt::contour::{ClabelOptions, ContourOptions, ContourSet, Levels, QuadCoords, QuadField};
use plt::dates::DateTime;
use plt::fonts::{FontSize, RelativeSize};
use plt::gridspec::SubplotOverrides;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::text::HAlign;
use plt::ticker::{FixedLocator, LogLocator};
use plt::transforms::Transform;
use plt_core::{Affine2D, Path, PathCode, Point};

use super::{colors, remove_ticks_and_titles};
use crate::figures::CaseResult;
use crate::rng::SpecRng;

/// A grid from nested rows of numbers.
fn rows<const C: usize>(values: &[[f64; C]]) -> Result<Grid<f64>, plt::numeric::GridError> {
    Grid::from_rows(values.iter().map(|row| row.iter().copied()))
}

/// Explicit levels.
fn levels(values: &[f64]) -> ContourOptions {
    ContourOptions {
        levels: Levels::Values(values.to_vec()),
        ..ContourOptions::default()
    }
}

/// `z(x, y)` at the points of the `x` × `y` mesh, one row per y.
fn mesh_values(x: &[f64], y: &[f64], z: impl Fn(f64, f64) -> f64) -> Grid<f64> {
    Grid::from_fn(y.len(), x.len(), |r, c| z(x[c], y[r]))
}

/// `z(x, y)` at the points of the `x` × `y` mesh, placed there.
fn on_mesh(x: &[f64], y: &[f64], z: impl Fn(f64, f64) -> f64) -> QuadField {
    let (xx, yy) = meshgrid(x, y);
    QuadField::new(mesh_values(x, y, z)).with_coords(QuadCoords::Mesh { x: xx, y: yy })
}

pub(super) fn contour_set_paths(fig: &Figure) -> CaseResult {
    let cs_test = fig.subplot().contour(
        &QuadField::new(rows(&[[0.0, 1.0], [1.0, 2.0]])?),
        ContourOptions::default(),
    )?;
    let cs_ref = Figure::new().subplot().contour(
        &QuadField::new(rows(&[[1.0, 0.0], [2.0, 1.0]])?),
        ContourOptions::default(),
    )?;
    cs_test.set_paths(cs_ref.paths());
    Ok(())
}

/// `z = max(|x|, |y|)` on the 10 × 10 integer grid.
fn max_norm_grid() -> QuadField {
    let x = arange(0.0, 10.0, 1.0);
    on_mesh(&x, &x, |x, y| x.abs().max(y.abs()))
}

/// Manual label positions.
fn manual(points: &[(f64, f64)]) -> ClabelOptions {
    ClabelOptions {
        manual: Some(points.to_vec()),
        ..ClabelOptions::default()
    }
}

pub(super) fn contour_manual_labels(fig: &Figure) -> CaseResult {
    let cs = fig
        .subplot()
        .contour(&max_norm_grid(), ContourOptions::default())?;
    cs.clabel(manual(&[(1.0, 3.0), (1.0, 4.4), (1.0, 6.0)]))?;
    cs.clabel(ClabelOptions {
        fontsize: Some(FontSize::Relative(RelativeSize::Small)),
        colors: Some(colors(&["r", "g"])),
        ..manual(&[(2.0, 3.0), (2.0, 4.4), (2.0, 6.0)])
    })?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn contour_label_with_disconnected_segments(fig: &Figure) -> CaseResult {
    // `np.mgrid[-1:1:21j, -1:1:21j]`: x down the rows, y along them.
    let t = linspace(-1.0, 1.0, 21);
    let x = Grid::from_fn(21, 21, |r, _| t[r]);
    let y = Grid::from_fn(21, 21, |_, c| t[c]);
    let z = Grid::from_fn(21, 21, |r, c| {
        let (x, y) = (x[(r, c)], y[(r, c)]);
        1.0 / (0.01 + (x + 0.3).powi(2) + y * y).sqrt()
            + 1.0 / (0.01 + (x - 0.3).powi(2) + y * y).sqrt()
    });
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x, y });
    let cs = fig.subplot().contour(&field, levels(&[7.0]))?;
    cs.clabel(manual(&[(0.2, 0.1)]))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn given_colors_levels_and_extends(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 4)?;
    let data = QuadField::new(Grid::from_fn(3, 4, |r, c| (r * 4 + c) as f64));
    let names = ["red", "yellow", "pink", "blue", "black"];
    let extends = [Extend::Neither, Extend::Min, Extend::Max, Extend::Both];
    for (i, ax) in axs.as_slice().iter().enumerate() {
        let extend = extends[i / 2];
        let cs = if i % 2 == 0 {
            // Three colours without extensions, one more per extension.
            let first = usize::from(matches!(extend, Extend::Max | Extend::Neither));
            let last = names.len() - usize::from(matches!(extend, Extend::Min | Extend::Neither));
            ax.contourf(
                &data,
                ContourOptions {
                    colors: Some(colors(&names[first..last])),
                    extend,
                    ..levels(&[2.0, 4.0, 8.0, 10.0])
                },
            )?
        } else {
            ax.contour(
                &data,
                ContourOptions {
                    colors: Some(colors(&names[..4])),
                    extend,
                    ..levels(&[2.0, 4.0, 8.0, 10.0])
                },
            )?
        };
        fig.colorbar(&cs, ColorbarOptions::beside(ax))?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn hatch_colors(fig: &Figure) -> CaseResult {
    let options = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("gray")?),
            ..ColorizerOptions::default()
        },
        hatches: ["-", "/", "\\", "//"]
            .into_iter()
            .map(|h| Some(h.to_owned()))
            .collect(),
        ..ContourOptions::default()
    };
    let cf = fig
        .subplot()
        .contourf(&QuadField::new(rows(&[[0.0, 1.0], [1.0, 2.0]])?), options)?;
    cf.set_edgecolors(colors(&["blue", "grey", "yellow", "red"]));
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn log_locator_levels(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = mesh_values(
        &linspace(-3.0, 3.0, 100),
        &linspace(-2.0, 2.0, 100),
        |x, y| (-x * x - y * y).exp() + 50.0 * (-(x * 10.0).powi(2) - (y * 10.0).powi(2)).exp(),
    );
    let options = ContourOptions {
        locator: Some(Box::new(LogLocator::new(10.0)?)),
        ..ContourOptions::default()
    };
    let c = ax.contourf(&QuadField::new(data), options)?;
    fig.colorbar(&c, ColorbarOptions::beside(&ax))?;
    Ok(())
}

pub(super) fn contour_datetime_axis(fig: &Figure) -> CaseResult {
    const DAY: i64 = 86_400_000_000;
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(0.4),
        top: Some(0.98),
        bottom: Some(0.15),
        ..SubplotOverrides::default()
    })?;
    let base = DateTime::new(2013, 1, 1, 0, 0, 0, 0)?.timestamp_micros();
    let days = (0..20)
        .map(|d| DateTime::from_timestamp_micros(base + d * DAY))
        .collect::<Result<Vec<_>, _>>()?;
    let y = arange(0.0, 20.0, 1.0);
    let z = Grid::from_fn(20, 20, |r, c| (r * c) as f64);
    for index in 1..=4 {
        let ax = fig.add_subplot(2, 2, index)?;
        let x = ax.xaxis().convert_units(days.clone());
        let coords = if index <= 2 {
            QuadCoords::Axes { x, y: y.clone() }
        } else {
            QuadCoords::Mesh {
                x: Grid::from_fn(20, 20, |_, c| x[c]),
                y: Grid::from_fn(20, 20, |r, _| y[r]),
            }
        };
        let field = QuadField::new(z.clone()).with_coords(coords);
        if index % 2 == 1 {
            ax.contour(&field, ContourOptions::default())?;
        } else {
            ax.contourf(&field, ContourOptions::default())?;
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

pub(super) fn labels(fig: &Figure) -> CaseResult {
    let field = on_mesh(
        &arange(-3.0, 3.0, 0.025),
        &arange(-2.0, 2.0, 0.025),
        |x, y| {
            let z1 = (-(x * x + y * y) / 2.0).exp() / (2.0 * PI);
            let z2 = (-(((x - 1.0) / 1.5).powi(2) + ((y - 1.0) / 0.5).powi(2)) / 2.0).exp()
                / (2.0 * PI * 0.5 * 1.5);
            10.0 * (z2 - z1)
        },
    );
    let ax = fig.subplot();
    let cs = ax.contour(&field, ContourOptions::default())?;
    cs.clabel(ClabelOptions::default())?;
    let label_near = |cs: &ContourSet, points: &[(f64, f64)], transform: &Transform| {
        points.iter().try_for_each(|&(x, y)| {
            cs.add_label_near(Point::new(x, y), transform, true, 5.0)
                .map(drop)
        })
    };
    label_near(
        &cs,
        &[(-2.0, 0.5), (0.0, -1.5), (2.8, 1.0)],
        &ax.trans_data(),
    )?;
    label_near(
        &cs,
        &[(216.0, 177.0), (359.0, 290.0), (521.0, 406.0)],
        &Transform::Display,
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_corner_mask`: noisy waves with 5% of the points masked.
fn corner_mask(fig: &Figure, corner_mask: bool) -> CaseResult {
    const N: usize = 60;
    let mut stream = SpecRng::new(1);
    let t = linspace(0.0, 2.0, N);
    let noise = stream.uniform(N * N, 0.0, 1.0);
    let z = Grid::from_fn(N, N, |r, c| {
        (7.0 * t[c]).cos() * (8.0 * t[r]).sin() + noise[r * N + c]
    });
    let mask = Grid::from_vec(
        N,
        N,
        stream
            .uniform(N * N, 0.0, 1.0)
            .into_iter()
            .map(|u| u >= 0.95)
            .collect(),
    )?;
    let field = QuadField {
        corner_mask: Some(corner_mask),
        ..QuadField::new(z).with_mask(mask)
    };
    fig.subplot().contourf(&field, ContourOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn corner_mask_1(fig: &Figure) -> CaseResult {
    corner_mask(fig, false)
}

pub(super) fn corner_mask_2(fig: &Figure) -> CaseResult {
    corner_mask(fig, true)
}

pub(super) fn contourf_log_extension(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.05),
        right: Some(0.95),
        ..SubplotOverrides::default()
    })?;
    let exponents = linspace(-7.5, 9.5, 1200);
    let data = QuadField::new(Grid::from_fn(30, 40, |r, c| {
        10f64.powf(exponents[r * 40 + c])
    }));
    let level_values: Vec<f64> = arange(-4.0, 7.0, 1.0)
        .into_iter()
        .map(|e| 10f64.powf(e))
        .collect();
    let log_norm = |vmin: f64, vmax: f64| ContourOptions {
        colorizer: ColorizerOptions {
            norm: Some(Box::new(LogNorm::default())),
            vmin: Some(vmin),
            vmax: Some(vmax),
            ..ColorizerOptions::default()
        },
        ..ContourOptions::default()
    };
    let (zmin, zmax) = (10f64.powf(-7.5), 10f64.powf(9.5));
    let fixed: Vec<f64> = arange(-8.0, 12.0, 2.0)
        .into_iter()
        .map(|e| 10f64.powf(e))
        .collect();
    let c1 = axs[(0, 0)].contourf(
        &data,
        ContourOptions {
            locator: Some(Box::new(FixedLocator::new(fixed))),
            ..log_norm(zmin, zmax)
        },
    )?;
    let banded = |extend| ContourOptions {
        levels: Levels::Values(level_values.clone()),
        extend,
        ..log_norm(1e-4, 1e6)
    };
    let c2 = axs[(0, 1)].contourf(&data, banded(Extend::Neither))?;
    let c3 = axs[(0, 2)].contourf(&data, banded(Extend::Both))?;
    for (cs, ax) in [c1, c2, c3].iter().zip(axs.as_slice()) {
        fig.colorbar(cs, ColorbarOptions::beside(ax))?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn contour_addlines(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = Grid::from_vec(10, 10, SpecRng::new(19_680_812).uniform(100, 0.0, 1.0))?
        .map(|v| v * 10000.0);
    let pcm = ax.pcolormesh(Mesh::new(x.clone()), PcolorOptions::default())?;
    let cont = ax.contour(
        &QuadField::new(x.map(|v| v + 1000.0)),
        ContourOptions::default(),
    )?;
    fig.colorbar(&pcm, ColorbarOptions::default())?
        .add_lines(&cont, true)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn contour_uneven(fig: &Figure) -> CaseResult {
    let z = QuadField::new(Grid::from_fn(4, 6, |r, c| (r * 6 + c) as f64));
    let axs = fig.subplots(1, 2)?;
    for (ax, spacing) in axs
        .as_slice()
        .iter()
        .zip([Spacing::Proportional, Spacing::Uniform])
    {
        let cs = ax.contourf(&z, levels(&[2.0, 4.0, 6.0, 10.0, 20.0]))?;
        fig.colorbar(
            &cs,
            ColorbarOptions {
                spacing,
                ..ColorbarOptions::beside(ax)
            },
        )?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn contour_closed_line_loop(fig: &Figure) -> CaseResult {
    let z = rows(&[
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        [2.0, 1.0, 2.0],
    ])?;
    let ax = fig.subplot();
    let options = ContourOptions {
        linewidths: Some(vec![20.0]),
        alpha: Some(0.7),
        ..levels(&[0.5])
    };
    ax.contour(&QuadField::new(z), options)?;
    ax.set_xlim(-0.1, 2.1)?.set_ylim(-0.1, 3.1)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// A segment through `points`, without path codes.
fn segment(points: &[(f64, f64)]) -> Path {
    Path::polyline(points.iter().map(|&(x, y)| Point::new(x, y)).collect())
}

pub(super) fn contour_manual(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let viridis = || colormaps::get("viridis");
    let lines0 = vec![segment(&[(2.0, 0.0), (1.0, 2.0), (1.0, 3.0)])];
    let lines1 = vec![
        segment(&[(3.0, 0.0), (3.0, 2.0)]),
        segment(&[(3.0, 3.0), (3.0, 4.0)]),
    ];
    let filled01 = vec![segment(&[
        (0.0, 0.0),
        (0.0, 4.0),
        (1.0, 3.0),
        (1.0, 2.0),
        (2.0, 0.0),
    ])];
    let filled12 = vec![
        segment(&[(2.0, 0.0), (3.0, 0.0), (3.0, 2.0), (1.0, 3.0), (1.0, 2.0)]),
        segment(&[(1.0, 4.0), (3.0, 4.0), (3.0, 3.0)]),
    ];
    let filled = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(viridis()?),
            ..ColorizerOptions::default()
        },
        ..ContourOptions::default()
    };
    ax.contour_segments(&[0.0, 1.0, 2.0], &[filled01, filled12], true, filled)?;
    let thick = |names: &[&str]| ContourOptions {
        linewidths: Some(vec![3.0]),
        colors: Some(colors(names)),
        ..ContourOptions::default()
    };
    ax.contour_segments(&[1.0, 2.0], &[lines0, lines1], false, thick(&["r", "k"]))?;
    // A polygon with a hole, given with path codes.
    let (move_to, line_to, close) = (PathCode::MoveTo, PathCode::LineTo, PathCode::ClosePoly);
    let vertices = [
        (4.0, 0.0),
        (7.0, 0.0),
        (7.0, 3.0),
        (4.0, 3.0),
        (4.0, 0.0),
        (5.0, 1.0),
        (5.0, 2.0),
        (6.0, 2.0),
        (6.0, 1.0),
        (5.0, 1.0),
    ];
    let codes = [
        move_to, line_to, line_to, line_to, close, move_to, line_to, line_to, line_to, close,
    ];
    let segs = Path::new(
        vertices.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        codes.to_vec(),
    )?;
    let hole = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(viridis()?),
            ..ColorizerOptions::default()
        },
        ..ContourOptions::default()
    };
    let segs = [vec![segs]];
    ax.contour_segments(&[2.0, 3.0], &segs, true, hole)?;
    ax.contour_segments(&[2.0], &segs, false, thick(&["k"]))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn contour_line_start_on_corner_edge(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let field = on_mesh(&[0.0, 1.0, 2.0, 3.0, 4.0], &[0.0, 1.0, 2.0], |x, y| {
        1.2 - (x - 2.0).powi(2) + (y - 1.0).powi(2)
    });
    let mask = Grid::from_fn(3, 5, |r, c| r == 1 && (c == 1 || c == 3));
    let field = QuadField {
        corner_mask: Some(true),
        ..field.with_mask(mask)
    };
    let filled = ax.contourf(&field, ContourOptions::default())?;
    let cbar = fig.colorbar(&filled, ColorbarOptions::default())?;
    let lines = ax.contour(
        &field,
        ContourOptions {
            colors: Some(colors(&["k"])),
            ..ContourOptions::default()
        },
    )?;
    cbar.add_lines(&lines, true)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn contourf_rasterize(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let circle =
        Path::unit_circle().transformed(&Affine2D::identity().scale(0.5, 0.5).translate(0.5, 0.5));
    let cs = ax.contourf(
        &QuadField::new(rows(&[[0.0, 1.0], [1.0, 0.0]])?),
        ContourOptions::default(),
    )?;
    for collection in cs.collections() {
        collection
            .set_clip_path(Some(ClipPathSpec::Path {
                path: circle.clone(),
                transform: ax.trans_axes(),
            }))
            .set_rasterized(true);
    }
    Ok(())
}

pub(super) fn contour_aliases(fig: &Figure) -> CaseResult {
    let data = Grid::from_fn(10, 10, |r, c| ((r * 10 + c) as f64).powi(2));
    let options = ContourOptions {
        linestyles: Some(vec![LineStyle::Dotted]),
        ..ContourOptions::default()
    };
    fig.subplot().contour(&QuadField::new(data), options)?;
    Ok(())
}
