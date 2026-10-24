//! `figures/cases_contour.py`.

mod gallery;

use plt::axes::{ColorizingArtist, ImshowOptions, TriShading, TriValues, TripcolorOptions};
use plt::collections::{CollectionProps, EdgeColors};
use plt::colorbar::ColorbarOptions;
use plt::colors::{Extend, colormaps};
use plt::contour::{ClabelOptions, ContourOptions, LabelFormat, Origin, QuadCoords, QuadField};
use plt::fonts::FontSize;
use plt::image::{ColorizerOptions, Extent};
use plt::lines::{LineProps, LineStyle};
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::rc::Aspect;
use plt::ticker::LogLocator;
use plt::ticker::pyfmt::PrintfFormat;
use plt::tri::Triangulation;

use crate::figures::FigureCase;
use crate::figures::cases::common::{colors, count, hatches, values};
use crate::rng::SpecRng;

/// Two Gaussian bumps of opposite sign on an `nx × ny` grid (`_xyz`).
fn bumps(nx: usize, ny: usize) -> QuadField {
    let (xx, yy) = meshgrid(&linspace(-3.0, 3.0, nx), &linspace(-2.0, 2.0, ny));
    let z = Grid::from_fn(ny, nx, |i, j| {
        let (x, y) = (xx[(i, j)], yy[(i, j)]);
        let z1 = (-(x * x) - y * y).exp();
        let z2 = (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp();
        (z1 - z2) * 2.0
    });
    QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy })
}

/// 60 random points in `[-3, 3] × [-2, 2]` plus the corners, Delaunay
/// triangulated (`_triangulation`).
fn random_triangulation(n: usize, seed: u64) -> Result<Triangulation, plt::tri::TriError> {
    let mut rng = SpecRng::new(seed);
    let mut x = rng.uniform(n, -3.0, 3.0);
    let mut y = rng.uniform(n, -2.0, 2.0);
    x.extend([-3.0, 3.0, 3.0, -3.0]);
    y.extend([-2.0, -2.0, 2.0, 2.0]);
    Triangulation::delaunay(&x, &y)
}

/// The bumps at the points of a triangulation (`_tri_z`).
fn tri_bumps(tri: &Triangulation) -> Vec<f64> {
    tri.x()
        .iter()
        .zip(tri.y())
        .map(|(&x, &y)| {
            (-(x * x) - y * y).exp() * 2.0
                - (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp() * 2.0
        })
        .collect()
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("contour/contour_default", |fig| {
        fig.subplot()
            .contour(&QuadField::new(bumps(60, 45).z), ContourOptions::default())?;
        Ok(())
    }),
    FigureCase::new("contour/contour_xy_levels_int", |fig| {
        fig.subplot().contour(&bumps(60, 45), count(12))?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/single_color_negative_dashed", |fig| {
        let options = ContourOptions {
            colors: Some(colors(&["k"])),
            ..count(10)
        };
        fig.subplot().contour(&bumps(60, 45), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/negative_linestyles_dotted", |fig| {
        let options = ContourOptions {
            colors: Some(colors(&["tab:blue"])),
            negative_linestyles: Some(LineStyle::Dotted),
            ..count(10)
        };
        fig.subplot().contour(&bumps(60, 45), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/linestyles_linewidths_lists", |fig| {
        let options = ContourOptions {
            colors: Some(colors(&["r", "g", "b"])),
            linestyles: Some(vec![LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted]),
            linewidths: Some(vec![0.5, 1.5, 3.0]),
            ..values(&[-1.5, -0.5, 0.0, 0.5, 1.5])
        };
        fig.subplot().contour(&bumps(60, 45), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/single_level_cmap", |fig| {
        let options = ContourOptions {
            cmap: Some(colormaps::get("plasma")?),
            linewidths: Some(vec![2.0]),
            ..values(&[0.0])
        };
        fig.subplot().contour(&bumps(60, 45), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/contourf_default", |fig| {
        fig.subplot()
            .contourf(&bumps(60, 45), ContourOptions::default())?;
        Ok(())
    }),
    FigureCase::new("contour/contourf_levels_cmap", |fig| {
        let options = ContourOptions {
            cmap: Some(colormaps::get("RdBu_r")?),
            ..values(&[-2.0, -1.0, -0.25, 0.0, 0.25, 1.0, 2.0])
        };
        fig.subplot().contourf(&bumps(60, 45), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/contourf_extend_both", |fig| {
        let cmap = colormaps::get("coolwarm")?
            .with_under("black")?
            .with_over("yellow")?;
        let options = ContourOptions {
            cmap: Some(cmap),
            extend: Extend::Both,
            ..values(&linspace(-0.8, 0.8, 9))
        };
        fig.subplot().contourf(&bumps(60, 45), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/contourf_alpha_overlay_lines", |fig| {
        let ax = fig.subplot();
        let field = bumps(60, 45);
        let fills = ContourOptions {
            alpha: Some(0.6),
            ..count(8)
        };
        ax.contourf(&field, fills)?;
        let lines = ContourOptions {
            colors: Some(colors(&["k"])),
            linewidths: Some(vec![0.6]),
            ..count(8)
        };
        ax.contour(&field, lines)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/contourf_hatches", |fig| {
        let options = ContourOptions {
            hatches: hatches(&[Some("/"), Some("\\"), None, Some("x")]),
            cmap: Some(colormaps::get("Pastel1")?),
            ..values(&[-2.0, -0.5, 0.0, 0.5, 2.0])
        };
        fig.subplot().contourf(&bumps(60, 45), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/contourf_hatches_only", |fig| {
        let ax = fig.subplot();
        let field = bumps(60, 45);
        let fills = ContourOptions {
            colors: Some(colors(&["none"])),
            hatches: hatches(&[Some(".."), None, Some("//")]),
            ..values(&[-2.0, -0.5, 0.5, 2.0])
        };
        ax.contourf(&field, fills)?;
        let lines = ContourOptions {
            colors: Some(colors(&["k"])),
            ..values(&[-0.5, 0.5])
        };
        ax.contour(&field, lines)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/clabel_default", |fig| {
        let cs = fig.subplot().contour(&bumps(60, 45), count(8))?;
        cs.clabel(ClabelOptions::default())?;
        Ok(())
    })
    .sized(5.0, 3.5, 100.0),
    FigureCase::new("contour/clabel_fmt_fontsize_not_inline", |fig| {
        let options = ContourOptions {
            colors: Some(colors(&["k"])),
            ..values(&[-1.0, -0.5, 0.5, 1.0])
        };
        let cs = fig.subplot().contour(&bumps(60, 45), options)?;
        cs.clabel(ClabelOptions {
            fmt: LabelFormat::Printf(PrintfFormat::parse("%1.2f m")?),
            fontsize: Some(FontSize::Points(7.0)),
            inline: false,
            ..ClabelOptions::default()
        })?;
        Ok(())
    })
    .sized(5.0, 3.5, 100.0),
    FigureCase::new("contour/clabel_dict_colors", |fig| {
        let cs = fig
            .subplot()
            .contour(&bumps(60, 45), values(&[-1.0, 0.0, 1.0]))?;
        let texts = [(-1.0, "low"), (0.0, "zero"), (1.0, "high")];
        cs.clabel(ClabelOptions {
            fmt: LabelFormat::Map(texts.iter().map(|&(l, t)| (l, t.to_owned())).collect()),
            colors: Some(colors(&["k"])),
            fontsize: Some(FontSize::Points(11.0)),
            inline_spacing: 8.0,
            ..ClabelOptions::default()
        })?;
        Ok(())
    })
    .sized(5.0, 3.5, 100.0),
    FigureCase::new("contour/masked_corner_mask_true_false", |fig| {
        let axes = fig.subplots(1, 2)?;
        let mask = Grid::from_fn(10, 13, |i, j| (i * 13 + j) % 7 == 0);
        let field = bumps(13, 10).with_mask(mask);
        for (ax, corner_mask) in axes.as_slice().iter().zip([true, false]) {
            let field = QuadField {
                corner_mask: Some(corner_mask),
                ..field.clone()
            };
            ax.contourf(&field, count(8))?;
            let lines = ContourOptions {
                colors: Some(colors(&["k"])),
                ..count(8)
            };
            ax.contour(&field, lines)?;
            ax.set_title(format!(
                "corner_mask={}",
                if corner_mask { "True" } else { "False" }
            ));
        }
        Ok(())
    })
    .sized(6.0, 2.5, 100.0),
    FigureCase::new("contour/nan_region", |fig| {
        let mut field = bumps(60, 45);
        if let QuadCoords::Mesh { x, y } = &field.coords {
            let holes = Grid::from_fn(45, 60, |i, j| x[(i, j)] > 0.5 && y[(i, j)] < 0.0);
            for (z, hole) in field.z.as_mut_slice().iter_mut().zip(holes.as_slice()) {
                if *hole {
                    *z = f64::NAN;
                }
            }
        }
        let options = ContourOptions {
            cmap: Some(colormaps::get("viridis")?),
            ..count(10)
        };
        fig.subplot().contourf(&field, options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/log_locator", |fig| {
        let x = linspace(-3.0, 3.0, 60);
        let (xx, yy) = meshgrid(&x, &x);
        let z = Grid::from_fn(60, 60, |i, j| {
            let (x, y) = (xx[(i, j)], yy[(i, j)]);
            (-(x * x) - y * y).exp() * 100.0
                + (-((x - 1.5) * (x - 1.5)) - (y - 1.0) * (y - 1.0)).exp() * 1e4
                + 1e-3
        });
        let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy });
        let options = ContourOptions {
            locator: Some(Box::new(LogLocator::new(10.0)?)),
            cmap: Some(colormaps::get("PuBu_r")?),
            ..ContourOptions::default()
        };
        fig.subplot().contourf(&field, options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/origin_extent_image", |fig| {
        let ax = fig.subplot();
        let z = bumps(30, 20).z;
        let image = ImshowOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("gray")?),
            origin: Some(plt::image::Origin::Lower),
            extent: Some(Extent::new(0.0, 6.0, 0.0, 4.0)),
            ..ImshowOptions::default()
        };
        ax.imshow(z.clone(), image)?;
        let field = QuadField::new(z).with_coords(QuadCoords::Implicit {
            origin: Some(Origin::Lower),
            extent: Some([0.0, 6.0, 0.0, 4.0]),
        });
        let options = ContourOptions {
            cmap: Some(colormaps::get("autumn")?),
            ..count(6)
        };
        ax.contour(&field, options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/tricontour", |fig| {
        let tri = random_triangulation(60, 21)?;
        fig.subplot()
            .tricontour(&tri, &tri_bumps(&tri), count(10))?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/tricontourf_levels", |fig| {
        let tri = random_triangulation(60, 21)?;
        let options = ContourOptions {
            cmap: Some(colormaps::get("RdYlBu")?),
            ..values(&[-2.0, -1.0, -0.3, 0.3, 1.0, 2.0])
        };
        fig.subplot().tricontourf(&tri, &tri_bumps(&tri), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/tripcolor_flat", |fig| {
        let tri = random_triangulation(60, 21)?;
        let options = TripcolorOptions {
            props: CollectionProps {
                edgecolors: Some(EdgeColors::Colors(colors(&["w"]))),
                linewidths: Some(vec![0.3]),
                ..CollectionProps::default()
            },
            ..TripcolorOptions::default()
        };
        fig.subplot()
            .tripcolor(&tri, TriValues::Auto(tri_bumps(&tri)), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/tripcolor_gouraud", |fig| {
        let tri = random_triangulation(60, 21)?;
        let options = TripcolorOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("magma")?),
            shading: TriShading::Gouraud,
            ..TripcolorOptions::default()
        };
        fig.subplot()
            .tripcolor(&tri, TriValues::Auto(tri_bumps(&tri)), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/tripcolor_facecolors", |fig| {
        let tri = random_triangulation(30, 22)?;
        let faces = arange(0.0, tri.ntri() as f64, 1.0);
        let options = TripcolorOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("tab20")?),
            ..TripcolorOptions::default()
        };
        fig.subplot()
            .tripcolor(&tri, TriValues::Faces(faces), options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/triplot_masked", |fig| {
        let ax = fig.subplot();
        let mut tri = random_triangulation(30, 23)?;
        tri.set_mask(Some((0..tri.ntri()).map(|t| t % 4 == 0).collect()))?;
        let props = LineProps::default()
            .color("tab:green")
            .linewidth(1.0)
            .markersize(3.0);
        ax.triplot(&tri, "o-", props)?;
        ax.set_aspect(Aspect::Equal)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("contour/recolor_set_cmap", |fig| {
        let ax = fig.subplot();
        let field = bumps(60, 45);
        let fills = ax.contourf(&field, count(8))?;
        let lines = ax.contour(&field, values(&fills.levels()))?;
        lines.clabel(ClabelOptions::default())?;
        fig.colorbar(&fills, ColorbarOptions::beside(&ax))?;
        fills.set_cmap(colormaps::get("RdBu_r")?);
        fills.set_clim(Some(-1.5), Some(1.5));
        lines.set_cmap(colormaps::get("Greys")?);
        Ok(())
    })
    .sized(5.0, 3.5, 100.0),
    FigureCase::new("contour/tripcolor_gouraud_colorbar", |fig| {
        let ax = fig.subplot();
        let tri = random_triangulation(60, 21)?;
        let options = TripcolorOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("magma")?),
            shading: TriShading::Gouraud,
            ..TripcolorOptions::default()
        };
        let plot = ax.tripcolor(&tri, TriValues::Auto(tri_bumps(&tri)), options)?;
        fig.colorbar(&plot, ColorbarOptions::beside(&ax))?;
        ColorizingArtist::from(&plot).set_clim(Some(-1.5), Some(1.5));
        Ok(())
    })
    .sized(5.0, 3.5, 100.0),
    FigureCase::new(
        "contour/contourf_extend_variants_colorbars",
        gallery::contourf_extend_variants_colorbars,
    )
    .sized(6.4, 5.0, 100.0),
    FigureCase::new(
        "contour/contourf_hatching_legend",
        gallery::contourf_hatching_legend,
    )
    .sized(7.0, 3.5, 100.0),
    FigureCase::new("contour/line_contour_legend", gallery::line_contour_legend)
        .sized(5.0, 3.5, 100.0),
    FigureCase::new(
        "contour/contourf_lognorm_levels_colorbar",
        gallery::contourf_lognorm_levels_colorbar,
    )
    .sized(5.0, 3.5, 100.0),
    FigureCase::new("contour/clabel_label_demo", gallery::clabel_label_demo).sized(7.0, 3.0, 100.0),
    FigureCase::new(
        "contour/clabel_manual_rightside_up",
        gallery::clabel_manual_rightside_up,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "contour/negative_linestyle_and_width_rc",
        gallery::negative_linestyle_and_width_rc,
    )
    .sized(4.0, 3.0, 100.0)
    .with_rc(&[
        ("contour.negative_linestyle", "dashdot"),
        ("contour.linewidth", "2.0"),
    ]),
    FigureCase::new(
        "contour/levels_outside_and_constant_field",
        gallery::levels_outside_and_constant_field,
    )
    .sized(6.0, 2.5, 100.0),
];
