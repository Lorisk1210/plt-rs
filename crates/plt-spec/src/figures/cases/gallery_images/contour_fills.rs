//! Filled contours: `contourf_demo`, `contourf_hatching`, `contourf_log`,
//! `contour_image` and `irregulardatagrid`.

use plt::axes::{ImshowOptions, LocatorParams};
use plt::colorbar::ColorbarOptions;
use plt::colors::{Color, Colorize, ColorizerOptions, Extend, Normalize, colormaps};
use plt::contour::{ClabelFormat, ClabelOptions, ContourOptions, Origin, QuadCoords, QuadField};
use plt::figure::Figure;
use plt::fonts::FontSize;
use plt::gridspec::SubplotOverrides;
use plt::image::{Extent, Interpolation};
use plt::layout::{LayoutEngine, TightLayout};
use plt::legend::LegendOptions;
use plt::lines::{LineProps, LineStyle};
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::ticker::pyfmt;
use plt::ticker::{LogLocator, Nbins};
use plt::tri::{LinearTriInterpolator, TriInterpolator, Triangulation};

use super::data::{SEED, mesh_field, spike};
use crate::figures::cases::common::{colors, count, hatches, two_bumps, values};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `_holey_field()`: `two_bumps` over `[-3, 3.01)²` with a NaN corner, a
/// masked corner and a masked disc.
fn holey_field() -> QuadField {
    let x = arange(-3.0, 3.01, 0.025);
    let (xx, yy, z) = mesh_field(&x, &x, two_bumps);
    let n = x.len();
    // `-n // 6` floors towards minus infinity.
    let nan_from = n - n.div_ceil(6);
    let z = Grid::from_fn(n, n, |r, c| {
        if r >= nan_from && c >= nan_from {
            f64::NAN
        } else {
            z[(r, c)]
        }
    });
    let mask = Grid::from_fn(n, n, |r, c| {
        (r < n / 6 && c < n / 6) || (xx[(r, c)] * xx[(r, c)] + yy[(r, c)] * yy[(r, c)]).sqrt() < 0.5
    });
    QuadField::new(z)
        .with_coords(QuadCoords::Mesh { x: xx, y: yy })
        .with_mask(mask)
}

/// `contourf_demo`'s levels.
fn holey_levels() -> Vec<f64> {
    vec![-1.5, -1.0, -0.5, 0.0, 0.5, 1.0]
}

fn contourf_demo_1(fig: &Figure) -> CaseResult {
    let field = holey_field();
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let ax = fig.subplot();
    let cs = ax.contourf(
        &field,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("bone")?),
                ..ColorizerOptions::default()
            },
            ..count(10)
        },
    )?;
    // `ax.contour(CS, levels=CS.levels[::2], ...)`: the same field.
    let every_other: Vec<f64> = cs.levels().into_iter().step_by(2).collect();
    let cs2 = ax.contour(
        &field,
        ContourOptions {
            colors: Some(colors(&["r"])),
            ..values(&every_other)
        },
    )?;
    ax.set_title("Nonsense (3 masked regions)");
    ax.set_xlabel("word length anomaly");
    ax.set_ylabel("sentence length anomaly");
    let cbar = fig.colorbar(&cs, ColorbarOptions::default())?;
    cbar.ax().set_ylabel("verbosity coefficient");
    cbar.add_lines(&cs2, true)?;
    Ok(())
}

fn contourf_demo_2(fig: &Figure) -> CaseResult {
    let field = holey_field();
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let ax = fig.subplot();
    let cs3 = ax.contourf(
        &field,
        ContourOptions {
            colors: Some(colors(&["r", "g", "b"])),
            extend: Extend::Both,
            ..values(&holey_levels())
        },
    )?;
    let cmap = cs3.cmap().ok_or("explicit colours make a colormap")?;
    cs3.set_cmap(cmap.with_under("yellow")?.with_over("cyan")?);
    let cs4 = ax.contour(
        &field,
        ContourOptions {
            colors: Some(colors(&["k"])),
            linewidths: Some(vec![3.0]),
            ..values(&holey_levels())
        },
    )?;
    ax.set_title("Listed colors (3 masked regions)");
    cs4.clabel(ClabelOptions {
        fmt: ClabelFormat::Label("%2.1f".into()),
        colors: Some(colors(&["w"])),
        fontsize: Some(FontSize::Points(14.0)),
        ..ClabelOptions::default()
    })?;
    fig.colorbar(&cs3, ColorbarOptions::default())?;
    Ok(())
}

fn contourf_demo_3(fig: &Figure) -> CaseResult {
    let field = holey_field();
    let cmap = colormaps::get("winter")?
        .with_under("magenta")?
        .with_over("yellow")?;
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(2, 2)?;
    for (ax, (extend, name)) in axs.as_slice().iter().zip([
        (Extend::Neither, "neither"),
        (Extend::Both, "both"),
        (Extend::Min, "min"),
        (Extend::Max, "max"),
    ]) {
        let cs = ax.contourf(
            &field,
            ContourOptions {
                colorizer: ColorizerOptions {
                    cmap: Some(cmap.clone()),
                    ..ColorizerOptions::default()
                },
                extend,
                ..values(&holey_levels())
            },
        )?;
        fig.colorbar(
            &cs,
            ColorbarOptions {
                shrink: 0.9,
                ..ColorbarOptions::beside(ax)
            },
        )?;
        ax.set_title(format!("extend = {name}"));
        ax.locator_params(&LocatorParams {
            nbins: Some(Nbins::Fixed(4)),
            ..LocatorParams::default()
        })?;
    }
    Ok(())
}

fn contourf_demo_4(fig: &Figure) -> CaseResult {
    let h = Grid::from_fn(9, 9, |r, c| ((r + 1) * (c + 1)) as f64);
    let axs = fig.subplots(1, 2)?;
    for (ax, (origin, title)) in axs.as_slice().iter().zip([
        (Origin::Upper, "origin='upper'"),
        (Origin::Lower, "origin='lower'"),
    ]) {
        ax.set_title(title);
        let field = QuadField::new(h.clone()).with_coords(QuadCoords::Implicit {
            origin: Some(origin),
            extent: None,
        });
        ax.contourf(
            &field,
            ContourOptions {
                extend: Extend::Both,
                ..values(&arange(5.0, 70.0, 5.0))
            },
        )?;
    }
    Ok(())
}

/// `contourf_hatching`'s `cos x + sin y` on 150 × 120 points over
/// `[-3, 5]²`.
fn hatching_wave() -> QuadField {
    let x = linspace(-3.0, 5.0, 150);
    let y = linspace(-3.0, 5.0, 120);
    let z = Grid::from_fn(120, 150, |r, c| x[c].cos() + y[r].sin());
    QuadField::new(z).with_coords(QuadCoords::Axes { x, y })
}

fn contourf_hatching_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let cs = ax.contourf(
        &hatching_wave(),
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("gray")?),
                ..ColorizerOptions::default()
            },
            hatches: hatches(&[Some("-"), Some("/"), Some("\\"), Some("//")]),
            extend: Extend::Both,
            alpha: Some(0.5),
            ..ContourOptions::default()
        },
    )?;
    fig.colorbar(&cs, ColorbarOptions::default())?;
    Ok(())
}

fn contourf_hatching_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let field = hatching_wave();
    ax.contour(
        &field,
        ContourOptions {
            colors: Some(colors(&["black"])),
            linestyles: Some(vec![LineStyle::Solid]),
            ..count(6)
        },
    )?;
    // The example's `extend='lower'` is not an extension matplotlib
    // knows, which leaves the levels unextended.
    let cs = ax.contourf(
        &field,
        ContourOptions {
            colors: Some(vec![Color::none()]),
            hatches: hatches(&[
                Some("."),
                Some("/"),
                Some("\\"),
                None,
                Some("\\\\"),
                Some("*"),
            ]),
            ..count(6)
        },
    )?;
    let (handles, labels) = cs.legend_elements("x", |v| format!("{:>2}", pyfmt::fixed(v, 1)));
    ax.legend(LegendOptions {
        handles: Some(handles),
        labels: Some(labels),
        handleheight: Some(2.0),
        framealpha: Some(1.0),
        ..LegendOptions::default()
    })?;
    Ok(())
}

fn contourf_log(fig: &Figure) -> CaseResult {
    let (x, y, z) = mesh_field(&linspace(-3.0, 3.0, 100), &linspace(-2.0, 2.0, 100), spike);
    let z = Grid::from_fn(
        100,
        100,
        |r, c| if r < 5 && c < 5 { -1.0 } else { z[(r, c)] },
    );
    let mask = z.map(|&v| v <= 0.0);
    let field = QuadField::new(z)
        .with_coords(QuadCoords::Mesh { x, y })
        .with_mask(mask);
    let ax = fig.subplot();
    let cs = ax.contourf(
        &field,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("PuBu_r")?),
                ..ColorizerOptions::default()
            },
            locator: Some(Box::new(LogLocator::new(10.0)?)),
            ..ContourOptions::default()
        },
    )?;
    fig.colorbar(&cs, ColorbarOptions::default())?;
    Ok(())
}

fn contour_image(fig: &Figure) -> CaseResult {
    let extent = [-3.0, 4.0, -4.0, 3.0];
    let (x, y, z) = mesh_field(
        &arange(-3.0, 4.001, 0.5),
        &arange(-4.0, 3.001, 0.5),
        two_bumps,
    );
    let levels = arange(-2.0, 1.601, 0.4);
    let top = z.as_slice().iter().fold(0.0, |m: f64, v| m.max(v.abs()));
    let norm = || Normalize::new(-top, top);
    let cmap = colormaps::get("PRGn")?;
    let axs = fig.subplots(2, 2)?;
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(0.3),
        ..SubplotOverrides::default()
    })?;
    let axs = axs.as_slice();
    let mesh = QuadField::new(z.clone()).with_coords(QuadCoords::Mesh { x, y });
    let cset1 = axs[0].contourf(
        &mesh,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(cmap.resampled(levels.len() - 1)?),
                norm: Some(norm().into()),
                ..ColorizerOptions::default()
            },
            ..values(&levels)
        },
    )?;
    let cset2 = axs[0].contour(
        &mesh,
        ContourOptions {
            colors: Some(colors(&["k"])),
            ..values(&cset1.levels())
        },
    )?;
    cset2.set_linestyles(&[LineStyle::Solid]);
    axs[0].contour(
        &mesh,
        ContourOptions {
            colors: Some(colors(&["g"])),
            linewidths: Some(vec![2.0]),
            ..values(&[0.0])
        },
    )?;
    axs[0].set_title("Filled contours");
    fig.colorbar(&cset1, ColorbarOptions::beside(&axs[0]))?;

    let image =
        |origin: Option<plt::image::Origin>, interpolation: Option<Interpolation>| ImshowOptions {
            colorizer: ColorizerOptions::default().cmap(cmap.clone()).norm(norm()),
            extent: Some(Extent::new(extent[0], extent[1], extent[2], extent[3])),
            origin,
            interpolation,
            ..ImshowOptions::default()
        };
    let black_lines = || ContourOptions {
        colors: Some(colors(&["k"])),
        ..values(&levels)
    };
    let placed = |origin: Origin| {
        QuadField::new(z.clone()).with_coords(QuadCoords::Implicit {
            origin: Some(origin),
            extent: Some(extent),
        })
    };
    axs[1].imshow(z.clone(), image(None, None))?;
    axs[1].contour(&placed(Origin::Upper), black_lines())?;
    axs[1].set_title("Image, origin 'upper'");
    axs[2].imshow(z.clone(), image(Some(plt::image::Origin::Lower), None))?;
    axs[2].contour(&placed(Origin::Lower), black_lines())?;
    axs[2].set_title("Image, origin 'lower'");
    let im = axs[3].imshow(z.clone(), image(None, Some(Interpolation::Nearest)))?;
    axs[3].contour(&placed(Origin::Image), black_lines())?;
    let (bottom, top) = axs[3].ylim();
    axs[3].set_ylim(top, bottom)?;
    axs[3].set_title("Origin from rc, reversed y-axis");
    fig.colorbar(&im, ColorbarOptions::beside(&axs[3]))?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn irregulardatagrid(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let (npts, ngridx, ngridy) = (200, 100, 200);
    let x = rng.uniform(npts, -2.0, 2.0);
    let y = rng.uniform(npts, -2.0, 2.0);
    let z: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(&x, &y)| x * (-(x * x) - y * y).exp())
        .collect();
    let axs = fig.subplots(2, 1)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    let xi = linspace(-2.1, 2.1, ngridx);
    let yi = linspace(-2.1, 2.1, ngridy);
    let triang = Triangulation::delaunay(&x, &y)?;
    let interpolator = LinearTriInterpolator::new(&triang, &z)?;
    let (gx, gy) = meshgrid(&xi, &yi);
    let zi = Grid::from_fn(ngridy, ngridx, |r, c| {
        interpolator.value(gx[(r, c)], gy[(r, c)])
    });
    let field = QuadField::new(zi.map(|v| v.unwrap_or(f64::NAN)))
        .with_coords(QuadCoords::Axes {
            x: xi.clone(),
            y: yi.clone(),
        })
        .with_mask(zi.map(Option::is_none));
    let thin_black = || ContourOptions {
        linewidths: Some(vec![0.5]),
        colors: Some(colors(&["k"])),
        ..count(14)
    };
    let rdbu = || -> Result<ContourOptions, plt::colors::ColormapError> {
        Ok(ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("RdBu_r")?),
                ..ColorizerOptions::default()
            },
            ..count(14)
        })
    };
    let dots = || LineProps::fmt("ko").map(|props| props.markersize(3.0));
    ax1.contour(&field, thin_black())?;
    let cntr1 = ax1.contourf(&field, rdbu()?)?;
    fig.colorbar(&cntr1, ColorbarOptions::beside(ax1))?;
    ax1.plot_with(&x, &y, dots()?)?;
    ax1.set_xlim(-2.0, 2.0)?;
    ax1.set_ylim(-2.0, 2.0)?;
    ax1.set_title(format!(
        "grid and contour ({npts} points, {} grid points)",
        ngridx * ngridy
    ));
    ax2.tricontour(&triang, &z, thin_black())?;
    let cntr2 = ax2.tricontourf(&triang, &z, rdbu()?)?;
    fig.colorbar(&cntr2, ColorbarOptions::beside(ax2))?;
    ax2.plot_with(&x, &y, dots()?)?;
    ax2.set_xlim(-2.0, 2.0)?;
    ax2.set_ylim(-2.0, 2.0)?;
    ax2.set_title(format!("tricontour ({npts} points)"));
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(0.5),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_images/contourf_demo_1", contourf_demo_1),
    FigureCase::new("gallery_images/contourf_demo_2", contourf_demo_2),
    FigureCase::new("gallery_images/contourf_demo_3", contourf_demo_3),
    FigureCase::new("gallery_images/contourf_demo_4", contourf_demo_4),
    FigureCase::new("gallery_images/contourf_hatching_1", contourf_hatching_1),
    FigureCase::new("gallery_images/contourf_hatching_2", contourf_hatching_2),
    FigureCase::new("gallery_images/contourf_log", contourf_log),
    FigureCase::new("gallery_images/contour_image", contour_image),
    FigureCase::new("gallery_images/irregulardatagrid", irregulardatagrid),
];
