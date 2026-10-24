//! The gallery's contour examples: filled contours with every extension
//! and their colorbars, hatching with contour legends, log norms, label
//! formats and placements, rc line styles and degenerate levels.

use plt::colorbar::ColorbarOptions;
use plt::colors::{Color, ColorizerOptions, Extend, LogNorm, colormaps};
use plt::contour::{ClabelFormat, ClabelOptions, ContourOptions, QuadCoords, QuadField};
use plt::figure::Figure;
use plt::fonts::FontSize;
use plt::layout::LayoutEngine;
use plt::legend::{LegendLoc, LegendOptions};
use plt::lines::LineStyle;
use plt::numeric::{Grid, arange, linspace, logspace, meshgrid};
use plt::offsetbox::AnchorLoc;
use plt::ticker::pyfmt;
use plt::ticker::{FuncFormatter, LogFormatter, LogLabelStyle, LogLocator};

use super::{bumps, colors, count, hatches, values};
use crate::figures::CaseResult;
use crate::figures::cases::common::percent_label;
use std::rc::Rc;

/// `_wave()`: `cos(x) + sin(y)` on a 150 × 120 grid over `[-3, 5]²`.
fn wave() -> QuadField {
    let (xx, yy) = meshgrid(&linspace(-3.0, 5.0, 150), &linspace(-3.0, 5.0, 120));
    let z = Grid::from_fn(120, 150, |i, j| xx[(i, j)].cos() + yy[(i, j)].sin());
    QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy })
}

pub(super) fn contourf_extend_variants_colorbars(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 2)?;
    let cmap = colormaps::get("winter")?
        .with_under("yellow")?
        .with_over("cyan")?;
    let field = bumps(40, 30);
    for (ax, (extend, name)) in axs.as_slice().iter().zip([
        (Extend::Neither, "neither"),
        (Extend::Both, "both"),
        (Extend::Min, "min"),
        (Extend::Max, "max"),
    ]) {
        let options = ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(cmap.clone()),
                ..ColorizerOptions::default()
            },
            extend,
            ..values(&[-1.5, -1.0, -0.5, 0.0, 0.5, 1.0])
        };
        let cs = ax.contourf(&field, options)?;
        fig.colorbar(
            &cs,
            ColorbarOptions {
                shrink: 0.9,
                ..ColorbarOptions::beside(ax)
            },
        )?;
        ax.set_title(format!("extend = {name}"));
    }
    Ok(())
}

pub(super) fn contourf_hatching_legend(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let field = wave();
    let options = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("gray")?),
            ..ColorizerOptions::default()
        },
        hatches: hatches(&[Some("-"), Some("/"), Some("\\"), Some("//")]),
        extend: Extend::Both,
        alpha: Some(0.5),
        ..ContourOptions::default()
    };
    let cs = ax1.contourf(&field, options)?;
    fig.colorbar(&cs, ColorbarOptions::beside(ax1))?;
    let lines = ContourOptions {
        colors: Some(colors(&["black"])),
        linestyles: Some(vec![LineStyle::Solid]),
        ..count(6)
    };
    ax2.contour(&field, lines)?;
    let fills = ContourOptions {
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
    };
    let cs = ax2.contourf(&field, fills)?;
    let (handles, labels) = cs.legend_elements("x", |v| format!("{:>2}", pyfmt::fixed(v, 1)));
    ax2.legend(LegendOptions {
        handles: Some(handles),
        labels: Some(labels),
        handleheight: Some(2.0),
        framealpha: Some(1.0),
        ..LegendOptions::default()
    })?;
    Ok(())
}

pub(super) fn line_contour_legend(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("viridis")?),
            ..ColorizerOptions::default()
        },
        linewidths: Some(vec![2.0]),
        ..count(5)
    };
    let cs = ax.contour(&bumps(60, 45), options)?;
    let (handles, labels) = cs.legend_elements("x", pyfmt::repr);
    ax.legend(LegendOptions {
        handles: Some(handles),
        labels: Some(labels),
        loc: Some(LegendLoc::Anchored(AnchorLoc::UpperLeft)),
        fontsize: Some(FontSize::Points(8.0)),
        ..LegendOptions::default()
    })?;
    Ok(())
}

pub(super) fn contourf_lognorm_levels_colorbar(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(-3.0, 3.0, 60);
    let (xx, yy) = meshgrid(&x, &x);
    let z = Grid::from_fn(60, 60, |i, j| {
        let (x, y) = (xx[(i, j)], yy[(i, j)]);
        (-(x * x) - y * y).exp() * 1e3
            + (-((x - 1.5) * (x - 1.5)) - (y - 1.0) * (y - 1.0)).exp() * 50.0
            + 1e-2
    });
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy });
    let options = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("PuBu_r")?),
            norm: Some(LogNorm::default().into()),
            ..ColorizerOptions::default()
        },
        ..values(&logspace(-2.0, 3.0, 11, 10.0))
    };
    let cs = ax.contourf(&field, options)?;
    fig.colorbar(&cs, ColorbarOptions::beside(&ax))?;
    Ok(())
}

pub(super) fn clabel_label_demo(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let (xx, yy) = meshgrid(&arange(-3.0, 3.0, 0.025), &arange(-2.0, 2.0, 0.025));
    let (rows, cols) = xx.shape();
    let z = Grid::from_fn(rows, cols, |i, j| {
        let (x, y) = (xx[(i, j)], yy[(i, j)]);
        ((-(x * x) - y * y).exp() - (-((x - 1.0) * (x - 1.0)) / 1.5 - (y - 1.0) * (y - 1.0)).exp())
            * 2.0
    });
    let coords = QuadCoords::Mesh { x: xx, y: yy };
    let field = QuadField::new(z.clone()).with_coords(coords.clone());
    let cs = axs[(0, 0)].contour(&field, ContourOptions::default())?;
    cs.clabel(ClabelOptions {
        levels: Some(cs.levels()),
        fmt: ClabelFormat::Formatter(Rc::new(FuncFormatter::new(|x, _| percent_label(x)))),
        fontsize: Some(FontSize::Points(10.0)),
        ..ClabelOptions::default()
    })?;
    let cs = axs[(0, 1)].contour(&field, ContourOptions::default())?;
    let names = [
        "first", "second", "third", "fourth", "fifth", "sixth", "seventh",
    ];
    let levels = cs.levels();
    cs.clabel(ClabelOptions {
        levels: Some(levels.iter().copied().step_by(2).collect()),
        fmt: ClabelFormat::Map(
            levels
                .iter()
                .zip(names)
                .map(|(&level, name)| (level, name.to_owned()))
                .collect(),
        ),
        fontsize: Some(FontSize::Points(10.0)),
        ..ClabelOptions::default()
    })?;
    let powers = QuadField::new(z.map(|v| 100f64.powf(*v))).with_coords(coords);
    let cs = axs[(0, 2)].contour(
        &powers,
        ContourOptions {
            locator: Some(Box::new(LogLocator::new(10.0)?)),
            ..ContourOptions::default()
        },
    )?;
    cs.clabel(ClabelOptions {
        levels: Some(cs.levels()),
        fmt: ClabelFormat::Formatter(Rc::new(LogFormatter::new(LogLabelStyle::Mathtext, 10.0))),
        ..ClabelOptions::default()
    })?;
    Ok(())
}

pub(super) fn clabel_manual_rightside_up(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let field = bumps(60, 45);
    let options = ContourOptions {
        colors: Some(colors(&["k"])),
        ..values(&[-1.5, -0.5, 0.5, 1.5])
    };
    let cs = axs[(0, 0)].contour(&field, options)?;
    cs.clabel(ClabelOptions {
        manual: Some(vec![(-1.0, 0.5), (0.5, -0.5), (1.5, 1.2), (-2.0, -1.5)]),
        inline_spacing: 12.0,
        ..ClabelOptions::default()
    })?;
    let options = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("coolwarm")?),
            ..ColorizerOptions::default()
        },
        ..values(&[-1.0, 0.0, 1.0])
    };
    let cs = axs[(0, 1)].contour(&field, options)?;
    cs.clabel(ClabelOptions {
        rightside_up: false,
        fontsize: Some(FontSize::Points(8.0)),
        fmt: ClabelFormat::Label("%.1f".into()),
        ..ClabelOptions::default()
    })?;
    Ok(())
}

pub(super) fn negative_linestyle_and_width_rc(fig: &Figure) -> CaseResult {
    let options = ContourOptions {
        colors: Some(colors(&["k"])),
        ..count(8)
    };
    fig.subplot().contour(&bumps(60, 45), options)?;
    Ok(())
}

pub(super) fn levels_outside_and_constant_field(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let field = bumps(30, 20);
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax1.contourf(
        &field,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("viridis")?),
                ..ColorizerOptions::default()
            },
            ..values(&[-0.5, 0.0, 0.5, 5.0, 6.0])
        },
    )?;
    ax1.contour(
        &field,
        ContourOptions {
            colors: Some(colors(&["r"])),
            ..values(&[5.0, 6.0])
        },
    )?;
    let (x, y) = meshgrid(&linspace(-3.0, 3.0, 30), &linspace(-2.0, 2.0, 20));
    let flat = QuadField::new(Grid::filled(20, 30, 0.5)).with_coords(QuadCoords::Mesh { x, y });
    ax2.contourf(&flat, ContourOptions::default())?;
    ax2.contour(
        &flat,
        ContourOptions {
            colors: Some(colors(&["k"])),
            ..ContourOptions::default()
        },
    )?;
    Ok(())
}
