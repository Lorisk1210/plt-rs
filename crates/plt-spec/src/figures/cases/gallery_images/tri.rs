//! Triangular grids: `tricontour_demo`, `tricontour_smooth_delaunay`,
//! `tricontour_smooth_user`, `trigradient_demo`, `triinterp_demo`,
//! `tripcolor_demo` and `triplot_demo`.

use plt::artist::BaseProps;
use plt::axes::{Axes, TriShading, TriValues, TripcolorOptions};
use plt::collections::{CollectionProps, EdgeColors};
use plt::colorbar::ColorbarOptions;
use plt::colors::{Color, ColorizerOptions, colormaps};
use plt::contour::{ContourOptions, Levels, QuadCoords, QuadField};
use plt::figure::Figure;
use plt::layout::TightLayout;
use plt::legend::LegendOptions;
use plt::lines::{LineProps, LineStyle};
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::quiver::{ArrowShape, QuiverOptions, QuiverUnits, VectorField};
use plt::rc::Aspect;
use plt::ticker::pyfmt;
use plt::tri::{
    CubicKind, CubicTriInterpolator, LinearTriInterpolator, TriAnalyzer, TriInterpolator,
    Triangulation, UniformTriRefiner,
};

use super::data::{SEED, britain, britain_triangulation, wales_bump};
use crate::figures::cases::common::{Annulus, colors, hatches, values};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// An axes with an equal aspect.
fn equal_axes(fig: &Figure) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    Ok(ax)
}

/// The axis labels and title of the plots over Great Britain.
fn britain_labels(ax: &Axes, title: &str) {
    ax.set_title(title);
    ax.set_xlabel("Longitude (degrees)");
    ax.set_ylabel("Latitude (degrees)");
}

/// The triangulation and field of `tricontour_demo` (48 angles),
/// `tripcolor_demo` and `triplot_demo` (36 angles): eight rings from 0.25
/// out, the hole left untriangulated.
fn rings(n_angles: usize) -> Result<(Triangulation, Vec<f64>), Box<dyn std::error::Error>> {
    let annulus = Annulus::new(n_angles, 8, 0.25);
    Ok((annulus.masked_delaunay(0.25)?, annulus.waves()))
}

fn tricontour_demo_1(fig: &Figure) -> CaseResult {
    let (triang, z) = rings(48)?;
    let ax = equal_axes(fig)?;
    let tcf = ax.tricontourf(&triang, &z, ContourOptions::default())?;
    fig.colorbar(&tcf, ColorbarOptions::default())?;
    ax.tricontour(
        &triang,
        &z,
        ContourOptions {
            colors: Some(colors(&["k"])),
            ..ContourOptions::default()
        },
    )?;
    ax.set_title("Contour plot of Delaunay triangulation");
    Ok(())
}

fn tricontour_demo_2(fig: &Figure) -> CaseResult {
    let (triang, z) = rings(48)?;
    let ax = equal_axes(fig)?;
    let tcf = ax.tricontourf(
        &triang,
        &z,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("cividis")?),
                ..ColorizerOptions::default()
            },
            hatches: hatches(&[
                Some("*"),
                Some("-"),
                Some("/"),
                Some("//"),
                Some("\\"),
                None,
            ]),
            ..ContourOptions::default()
        },
    )?;
    fig.colorbar(&tcf, ColorbarOptions::default())?;
    ax.tricontour(
        &triang,
        &z,
        ContourOptions {
            linestyles: Some(vec![LineStyle::Solid]),
            colors: Some(colors(&["k"])),
            linewidths: Some(vec![2.0]),
            ..ContourOptions::default()
        },
    )?;
    ax.set_title("Hatched Contour plot of Delaunay triangulation");
    Ok(())
}

fn tricontour_demo_3(fig: &Figure) -> CaseResult {
    let (triang, z) = rings(48)?;
    let ax = fig.subplot();
    let tcf = ax.tricontourf(
        &triang,
        &z,
        ContourOptions {
            levels: Levels::Count(7),
            colors: Some(vec![Color::none()]),
            hatches: hatches(&[
                Some("."),
                Some("/"),
                Some("\\"),
                None,
                Some("\\\\"),
                Some("*"),
            ]),
            ..ContourOptions::default()
        },
    )?;
    ax.tricontour(
        &triang,
        &z,
        ContourOptions {
            levels: Levels::Count(7),
            colors: Some(colors(&["black"])),
            linestyles: Some(vec![LineStyle::Solid]),
            ..ContourOptions::default()
        },
    )?;
    let (handles, labels) = tcf.legend_elements("x", |v| format!("{:>2}", pyfmt::fixed(v, 1)));
    ax.legend(LegendOptions {
        handles: Some(handles),
        labels: Some(labels),
        handleheight: Some(2.0),
        framealpha: Some(1.0),
        ..LegendOptions::default()
    })?;
    Ok(())
}

fn tricontour_demo_4(fig: &Figure) -> CaseResult {
    let triang = britain_triangulation()?;
    let z: Vec<f64> = triang
        .x()
        .iter()
        .zip(triang.y())
        .map(|(&x, &y)| wales_bump(x, y))
        .collect();
    let ax = equal_axes(fig)?;
    let tcf = ax.tricontourf(&triang, &z, ContourOptions::default())?;
    fig.colorbar(&tcf, ColorbarOptions::default())?;
    britain_labels(&ax, "Contour plot of user-specified triangulation");
    Ok(())
}

/// `(max(z) - z) / (max(z) - min(z))` of `z`.
fn rescaled(z: &[f64]) -> Vec<f64> {
    let max = z.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = z.iter().copied().fold(f64::INFINITY, f64::min);
    z.iter().map(|v| (max - v) / (max - min)).collect()
}

/// The two-pole field of the smooth tricontouring examples: `petals`
/// about `(0.5, 0.5)` and `(-0.2, -0.2)`, weighted, plus a paraboloid.
fn two_poles(x: f64, y: f64, weight: f64, petals: (f64, f64), bowl: f64) -> f64 {
    let r1 = ((0.5 - x) * (0.5 - x) + (0.5 - y) * (0.5 - y)).sqrt();
    let theta1 = (0.5 - x).atan2(0.5 - y);
    let r2 = ((-x - 0.2) * (-x - 0.2) + (-y - 0.2) * (-y - 0.2)).sqrt();
    let theta2 = (-x - 0.2).atan2(-y - 0.2);
    let ring = |r: f64| ((r / 10.0) * (r / 10.0)).exp() - 1.0;
    weight * ring(r1) * 30.0 * (petals.0 * theta1).cos()
        + ring(r2) * 30.0 * (petals.1 * theta2).cos()
        + bowl * (x * x + y * y)
}

/// The values of a refined field, which cover every refined point.
fn refined_values(z: &[Option<f64>]) -> Vec<f64> {
    z.iter().map(|v| v.unwrap_or(f64::NAN)).collect()
}

fn tricontour_smooth_delaunay(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let x_test = rng.uniform(200, -1.0, 1.0);
    let y_test = rng.uniform(200, -1.0, 1.0);
    let z_test = rescaled(
        &x_test
            .iter()
            .zip(&y_test)
            .map(|(&x, &y)| two_poles(2.0 * x, y, 4.0, (3.0, 5.0), 2.0))
            .collect::<Vec<f64>>(),
    );
    let tri = Triangulation::delaunay(&x_test, &y_test)?;
    let mask = TriAnalyzer::new(&tri).flat_tri_mask(0.01, true);
    let tri = tri.with_mask(Some(mask.clone()))?;
    let refined = UniformTriRefiner::new(&tri).refine_field(&z_test, None, 3)?;
    let flat_tri = Triangulation::delaunay(&x_test, &y_test)?
        .with_mask(Some(mask.iter().map(|m| !m).collect()))?;
    let ax = equal_axes(fig)?;
    ax.set_title("Filtering a Delaunay mesh\n(application to high-resolution tricontouring)");
    ax.tricontour(
        &refined.triangulation,
        &refined_values(&refined.z),
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("Blues")?),
                ..ColorizerOptions::default()
            },
            linewidths: Some(vec![2.0, 0.5, 1.0, 0.5]),
            ..values(&arange(0.0, 1.0, 0.025))
        },
    )?;
    ax.triplot(&tri, "", LineProps::default().color("0.7"))?;
    ax.triplot(&flat_tri, "", LineProps::default().color("red"))?;
    Ok(())
}

fn tricontour_smooth_user(fig: &Figure) -> CaseResult {
    let annulus = Annulus::new(20, 10, 0.15);
    let z = rescaled(
        &annulus
            .x
            .iter()
            .zip(&annulus.y)
            .map(|(&x, &y)| -two_poles(x, y, 2.0, (7.0, 11.0), 0.7))
            .collect::<Vec<f64>>(),
    );
    let triang = annulus.masked_delaunay(0.15)?;
    let refined = UniformTriRefiner::new(&triang).refine_field(&z, None, 3)?;
    let z_refi = refined_values(&refined.z);
    let ax = equal_axes(fig)?;
    ax.triplot(
        &triang,
        "",
        LineProps::default().linewidth(0.5).color("white"),
    )?;
    let levels = arange(0.0, 1.0, 0.025);
    ax.tricontourf(
        &refined.triangulation,
        &z_refi,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("terrain")?),
                ..ColorizerOptions::default()
            },
            ..values(&levels)
        },
    )?;
    ax.tricontour(
        &refined.triangulation,
        &z_refi,
        ContourOptions {
            colors: Some(colors(&["0.25", "0.5", "0.5", "0.5", "0.5"])),
            linewidths: Some(vec![1.0, 0.5, 0.5, 0.5, 0.5]),
            ..values(&levels)
        },
    )?;
    ax.set_title("High-resolution tricontouring");
    Ok(())
}

fn trigradient_demo(fig: &Figure) -> CaseResult {
    let annulus = Annulus::new(30, 10, 0.2);
    let potential: Vec<f64> = annulus
        .x
        .iter()
        .zip(&annulus.y)
        .map(|(&x, &y)| y.atan2(x).cos() / (x * x + y * y))
        .collect();
    let v = rescaled(&potential);
    let triang = annulus.masked_delaunay(0.2)?;
    let refined = UniformTriRefiner::new(&triang).refine_field(&v, None, 3)?;
    let minus_v: Vec<f64> = v.iter().map(|v| -v).collect();
    let tci = CubicTriInterpolator::new(&triang, &minus_v, &CubicKind::MinE)?;
    let (mut ex, mut ey) = (Vec::new(), Vec::new());
    for (&x, &y) in triang.x().iter().zip(triang.y()) {
        let [gx, gy] = tci.gradient(x, y).unwrap_or([f64::NAN; 2]);
        let norm = (gx * gx + gy * gy).sqrt();
        ex.push(gx / norm);
        ey.push(gy / norm);
    }
    let ax = equal_axes(fig)?;
    ax.set_use_sticky_edges(false);
    ax.set_margins(0.07, 0.07)?;
    ax.triplot(&triang, "", LineProps::default().color("0.8"))?;
    ax.tricontour(
        &refined.triangulation,
        &refined_values(&refined.z),
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("hot")?),
                ..ColorizerOptions::default()
            },
            linewidths: Some(vec![2.0, 1.0, 1.0, 1.0]),
            ..values(&arange(0.0, 1.0, 0.01))
        },
    )?;
    let options = QuiverOptions {
        units: QuiverUnits::Xy,
        scale: Some(10.0),
        color: "blue".into(),
        width: Some(0.007),
        shape: ArrowShape {
            head_width: 3.0,
            head_length: 4.0,
            ..ArrowShape::default()
        },
        props: CollectionProps {
            base: BaseProps {
                zorder: Some(3.0),
                ..BaseProps::default()
            },
            ..CollectionProps::default()
        },
        ..QuiverOptions::default()
    };
    let field = VectorField::new(triang.x().to_vec(), triang.y().to_vec(), ex, ey);
    ax.quiver(field, options)?;
    ax.set_title("Gradient plot: an electrical dipole");
    Ok(())
}

fn triinterp_demo(fig: &Figure) -> CaseResult {
    let x = [0.0, 1.0, 2.0, 3.0, 0.5, 1.5, 2.5, 1.0, 2.0, 1.5];
    let y = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 3.0];
    let triangles = vec![
        [0, 1, 4],
        [1, 2, 5],
        [2, 3, 6],
        [1, 5, 4],
        [2, 6, 5],
        [4, 5, 7],
        [5, 6, 8],
        [5, 8, 7],
        [7, 8, 9],
    ];
    let triang = Triangulation::new(&x, &y, triangles)?;
    let z: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(&x, &y)| (1.5 * x).cos() * (1.5 * y).cos())
        .collect();
    let grid = linspace(0.0, 3.0, 20);
    let (xi, yi) = meshgrid(&grid, &grid);
    let axs = fig.subplots(2, 2)?;
    let axs = axs.as_slice();
    axs[0].tricontourf(&triang, &z, ContourOptions::default())?;
    axs[0].triplot(&triang, "ko-", LineProps::default())?;
    axs[0].set_title("Triangular grid");
    let linear = LinearTriInterpolator::new(&triang, &z)?;
    let geom = CubicTriInterpolator::new(&triang, &z, &CubicKind::Geom)?;
    let min_e = CubicTriInterpolator::new(&triang, &z, &CubicKind::MinE)?;
    let interpolators: [(&dyn TriInterpolator, &str); 3] = [
        (&linear, "Linear interpolation"),
        (&geom, "Cubic interpolation,\nkind='geom'"),
        (&min_e, "Cubic interpolation,\nkind='min_E'"),
    ];
    let faint = || LineProps::fmt("k-").map(|p| p.linewidth(0.5).alpha(0.5));
    for (ax, (interpolator, title)) in axs[1..].iter().zip(interpolators) {
        let zi = Grid::from_fn(20, 20, |r, c| interpolator.value(xi[(r, c)], yi[(r, c)]));
        let field = QuadField::new(zi.map(|v| v.unwrap_or(f64::NAN)))
            .with_coords(QuadCoords::Mesh {
                x: xi.clone(),
                y: yi.clone(),
            })
            .with_mask(zi.map(Option::is_none));
        ax.contourf(&field, ContourOptions::default())?;
        // `plot(xi, yi)` and `plot(xi.T, yi.T)`: one line per column of
        // each.
        for (xs, ys) in [(&xi, &yi), (&xi.transposed(), &yi.transposed())] {
            for c in 0..20 {
                ax.plot_with(xs.column(c), ys.column(c), faint()?)?;
            }
        }
        ax.set_title(title);
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn tripcolor_demo_1(fig: &Figure) -> CaseResult {
    tripcolor_rings(
        fig,
        TriShading::Flat,
        "tripcolor of Delaunay triangulation, flat shading",
    )
}

fn tripcolor_demo_2(fig: &Figure) -> CaseResult {
    tripcolor_rings(
        fig,
        TriShading::Gouraud,
        "tripcolor of Delaunay triangulation, gouraud shading",
    )
}

/// The rings coloured with `shading`, with a colorbar.
fn tripcolor_rings(fig: &Figure, shading: TriShading, title: &str) -> CaseResult {
    let (triang, z) = rings(36)?;
    let ax = equal_axes(fig)?;
    let options = TripcolorOptions {
        shading,
        ..TripcolorOptions::default()
    };
    let tpc = ax.tripcolor(&triang, TriValues::Auto(z), options)?;
    fig.colorbar(&tpc, ColorbarOptions::default())?;
    ax.set_title(title);
    Ok(())
}

fn tripcolor_demo_3(fig: &Figure) -> CaseResult {
    let triang = britain_triangulation()?;
    let (x, y) = britain();
    let zfaces: Vec<f64> = triang
        .triangles()
        .iter()
        .map(|t| {
            let mean = |v: &[f64]| (v[t[0]] + v[t[1]] + v[t[2]]) / 3.0;
            wales_bump(mean(&x), mean(&y))
        })
        .collect();
    let ax = equal_axes(fig)?;
    let options = TripcolorOptions {
        props: CollectionProps {
            edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
            ..CollectionProps::default()
        },
        ..TripcolorOptions::default()
    };
    let tpc = ax.tripcolor(&triang, TriValues::Faces(zfaces), options)?;
    fig.colorbar(&tpc, ColorbarOptions::default())?;
    britain_labels(&ax, "tripcolor of user-specified triangulation");
    Ok(())
}

fn triplot_demo_1(fig: &Figure) -> CaseResult {
    let (triang, _) = rings(36)?;
    let ax = equal_axes(fig)?;
    ax.triplot(&triang, "bo-", LineProps::default().linewidth(1.0))?;
    ax.set_title("triplot of Delaunay triangulation");
    Ok(())
}

fn triplot_demo_2(fig: &Figure) -> CaseResult {
    let triang = britain_triangulation()?;
    let ax = equal_axes(fig)?;
    ax.triplot(&triang, "go-", LineProps::default().linewidth(1.0))?;
    britain_labels(&ax, "triplot of user-specified triangulation");
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_images/tricontour_demo_1", tricontour_demo_1),
    FigureCase::new("gallery_images/tricontour_demo_2", tricontour_demo_2),
    FigureCase::new("gallery_images/tricontour_demo_3", tricontour_demo_3),
    FigureCase::new("gallery_images/tricontour_demo_4", tricontour_demo_4),
    FigureCase::new(
        "gallery_images/tricontour_smooth_delaunay",
        tricontour_smooth_delaunay,
    ),
    FigureCase::new(
        "gallery_images/tricontour_smooth_user",
        tricontour_smooth_user,
    ),
    FigureCase::new("gallery_images/trigradient_demo", trigradient_demo),
    FigureCase::new("gallery_images/triinterp_demo", triinterp_demo),
    FigureCase::new("gallery_images/tripcolor_demo_1", tripcolor_demo_1),
    FigureCase::new("gallery_images/tripcolor_demo_2", tripcolor_demo_2),
    FigureCase::new("gallery_images/tripcolor_demo_3", tripcolor_demo_3),
    FigureCase::new("gallery_images/triplot_demo_1", triplot_demo_1),
    FigureCase::new("gallery_images/triplot_demo_2", triplot_demo_2),
];
