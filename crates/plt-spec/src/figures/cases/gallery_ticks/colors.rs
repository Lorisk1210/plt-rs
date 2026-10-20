//! Colours: colorbars of masked and clipped images, a histogram used as a
//! colorbar, a line coloured by its values, the default colour cycle,
//! colours in every format, the built-in colour sequences, colours with
//! their own alpha, and colours picked from colormaps (`colorbar_basics`,
//! `colorbar_histogram`, `color_by_yvalue`, `color_cycle_default`,
//! `color_demo`, `color_sequences`, `set_alpha` and
//! `individual_colors_from_cmap`).

use plt::axes::{AxisName, ImshowOptions, TickLevel, TickParams};
use plt::collections::EdgeColors;
use plt::colorbar::ColorbarOptions;
use plt::colors::{BoundaryNorm, ColorTable, Colormap, Extend, Norm, color_sequence, colormaps};
use plt::image::{ColorizerOptions, Extent, Interpolation, Origin, ScalarField};
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::stats::{Bins, HistogramOptions, histogram};
use plt::text::VerticalAlignment as VAlign;
use plt::transforms::Anchor;

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// An image in a colormap, its pixels drawn unsmoothed.
fn unsmoothed(cmap: &str) -> Result<ImshowOptions, plt::colors::ColormapError> {
    Ok(ImshowOptions::default()
        .cmap(colormaps::get(cmap)?)
        .interpolation(Interpolation::None))
}

fn colorbar_basics(fig: &Figure) -> CaseResult {
    let n = 37;
    let z = Grid::from_fn(n, n, |i, j| (i as f64 * 0.2).cos() + (j as f64 * 0.3).sin());
    let axs = fig.subplots(1, 3)?;
    let (ax1, ax2, ax3) = (&axs[(0, 0)], &axs[(0, 1)], &axs[(0, 2)]);
    let positive = ScalarField::masked(z.clone(), &z.map(|&v| v < 0.0))?;
    let pos = ax1.imshow(positive, unsmoothed("Blues")?)?;
    fig.colorbar(&pos, ColorbarOptions::beside(ax1))?;
    let negative = ScalarField::masked(z.clone(), &z.map(|&v| v > 0.0))?;
    let neg = ax2.imshow(negative, unsmoothed("Reds_r")?)?;
    fig.colorbar(
        &neg,
        ColorbarOptions {
            location: Some(SpineSide::Right),
            anchor: Some(Anchor::Fraction(0.0, 0.3)),
            shrink: 0.7,
            ..ColorbarOptions::beside(ax2)
        },
    )?;
    let clipped = ax3.imshow(
        z,
        ImshowOptions::default()
            .interpolation(Interpolation::None)
            .colors(ColorizerOptions {
                cmap: Some(colormaps::get("RdBu")?),
                vmin: Some(-1.2),
                vmax: Some(1.2),
                ..ColorizerOptions::default()
            }),
    )?;
    let cbar = fig.colorbar(
        &clipped,
        ColorbarOptions {
            extend: Some(Extend::Both),
            ..ColorbarOptions::beside(ax3)
        },
    )?;
    cbar.minorticks_on();
    Ok(())
}

fn colorbar_histogram(fig: &Figure) -> CaseResult {
    let coords = arange(-2.0, 2.0, 0.025);
    let n = coords.len();
    let z = Grid::from_fn(n, n, |row, col| {
        let (x, y) = (coords[col], coords[row]);
        let z1 = (-((x + 1.0) * 1.3).powi(2) - ((y + 1.0) * 1.3).powi(2)).exp();
        let z2 = 2.5 * (-(x - 1.0).powi(2) - (y - 1.0).powi(2)).exp();
        z1.powf(0.25) - z2.sqrt()
    });
    let values = z.as_slice();
    let (low, high) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let cmap = colormaps::get("RdYlBu_r")?;
    let bin_edges = linspace(low, high, 31);
    let norm = BoundaryNorm::new(bin_edges.clone(), cmap.n(), Extend::Neither)?;
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    ax.imshow(
        z.clone(),
        ImshowOptions {
            colors: ColorizerOptions {
                cmap: Some(cmap.clone()),
                norm: Some(Box::new(norm.clone())),
                ..ColorizerOptions::default()
            },
            origin: Some(Origin::Lower),
            extent: Some(Extent {
                left: -3.0,
                right: 3.0,
                bottom: -3.0,
                top: 3.0,
            }),
            ..ImshowOptions::default()
        },
    )?;
    let cax = ax.inset_axes([1.18, 0.02, 0.25, 0.95]);
    let counts = histogram(
        values,
        &HistogramOptions {
            bins: Bins::Edges(bin_edges.clone()),
            ..HistogramOptions::default()
        },
    )?
    .counts;
    let midpoints: Vec<f64> = bin_edges
        .windows(2)
        .map(|w| f64::midpoint(w[0], w[1]))
        .collect();
    let distance = midpoints[1] - midpoints[0];
    let colors = norm
        .clone()
        .normalize(&midpoints)?
        .into_iter()
        .map(|index| Color::from(cmap.lookup(index, None)))
        .collect();
    cax.barh(
        &midpoints,
        counts,
        BarOptions {
            thickness: vec![0.8 * distance],
            color: colors,
            ..BarOptions::default()
        },
    )?;
    for spine in cax.spines().iter() {
        spine.set_visible(false);
    }
    cax.set_yticks(bin_edges);
    cax.tick_params(&TickParams {
        which: TickLevel::Both,
        length: Some(0.0),
        ..TickParams::default()
    });
    Ok(())
}

fn color_by_yvalue(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 2.0, 0.01);
    let s = map(&t, |t| (2.0 * std::f64::consts::PI * t).sin());
    let (upper, lower) = (0.77, -0.77);
    // The masked parts of each line are not drawn.
    let keep = |inside: fn(f64, f64, f64) -> bool| {
        map(&s, |v| if inside(v, lower, upper) { v } else { f64::NAN })
    };
    let ax = fig.subplot();
    ax.plot(&t, keep(|v, lower, upper| v >= lower && v <= upper))?;
    ax.plot(&t, keep(|v, lower, _| v <= lower))?;
    ax.plot(&t, keep(|v, _, upper| v >= upper))?;
    Ok(())
}

fn color_cycle_default(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    ax.set_title("Colors in the default property cycle");
    let colors = plt::rc::params().axes_prop_cycle.colors();
    let x = linspace(-4.0, 4.0, 200);
    let names = ColorTable::Tableau.entries();
    for (i, (color, (name, _))) in colors.into_iter().zip(names).enumerate() {
        let pos = 4.5 - i as f64;
        // A sigmoid-like curve ending at about `pos`.
        ax.plot(
            &x,
            map(&x, |x| 0.85 * pos * (1.0 / (1.0 + (-x).exp()) + 0.2)),
        )?;
        ax.text(4.2, pos, format!("'C{i}': '{name}'"))
            .set_color(Color::from(color))
            .set_valign(VAlign::Center);
        ax.bar(
            [9.0],
            [1.0],
            BarOptions {
                thickness: vec![1.5],
                base: vec![pos - 0.5],
                ..BarOptions::default()
            },
        )?;
    }
    Ok(())
}

fn color_demo(fig: &Figure) -> CaseResult {
    let t = linspace(0.0, 2.0, 201);
    let s = map(&t, |t| (2.0 * std::f64::consts::PI * t).sin());
    fig.set_facecolor(Color::from((0.18, 0.31, 0.31)).resolve()?);
    let ax = fig.subplot();
    ax.set_facecolor("#eafff5");
    ax.set_title("Voltage vs. time chart")
        .set_color(Color::from("0.7"));
    ax.set_xlabel("Time [s]").set_ylabel("Voltage [mV]");
    ax.xaxis().set_label_color("c");
    ax.yaxis().set_label_color("peachpuff");
    ax.plot_fmt(&t, &s, "xkcd:crimson")?;
    ax.plot_with(
        &t,
        map(&s, |s| 0.7 * s),
        LineProps::default()
            .color("C4")
            .linestyle(LineStyle::Dashed),
    )?;
    ax.tick_params(&TickParams {
        labelcolor: Some("tab:orange".into()),
        ..TickParams::default()
    });
    Ok(())
}

fn color_sequences(fig: &Figure) -> CaseResult {
    let names = [
        "tab10",
        "tab20",
        "tab20b",
        "tab20c",
        "Pastel1",
        "Pastel2",
        "Paired",
        "Accent",
        "okabe_ito",
        "Dark2",
        "Set1",
        "Set2",
        "Set3",
        "petroff6",
        "petroff8",
        "petroff10",
    ];
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    for (n, name) in names.iter().enumerate() {
        let colors = color_sequence(name).ok_or("unknown colour sequence")?;
        let x: Vec<f64> = (0..colors.len()).map(|i| i as f64).collect();
        let y = vec![n as f64; colors.len()];
        let points = ax.scatter_with(
            x,
            y,
            ScatterOptions {
                colors: plt::axes::ScatterColors::PerPoint(
                    colors.into_iter().map(Color::from).collect(),
                ),
                edgecolors: Some(EdgeColors::Colors(vec![Color::from("dimgray")])),
                sizes: Some(vec![200.0]),
                ..ScatterOptions::default()
            },
        )?;
        points.set_zorder(2.0);
    }
    ax.set_yticks((0..names.len()).map(|i| i as f64).collect::<Vec<_>>())
        .set_yticklabels(names)?;
    ax.grid_with(
        true,
        &plt::axes::GridOptions {
            axis: Some(AxisName::Y),
            ..plt::axes::GridOptions::default()
        },
    );
    ax.yaxis().set_inverted(true);
    ax.xaxis().set_visible(false);
    for spine in ax.spines().iter() {
        spine.set_visible(false);
    }
    ax.tick_params(&TickParams {
        left: Some(false),
        ..TickParams::default()
    });
    ax.set_title("Built In Color Sequences");
    Ok(())
}

fn set_alpha(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let x: Vec<f64> = (0..20).map(f64::from).collect();
    let y = SpecRng::new(19_680_801).normal(20, 0.0, 1.0);
    let facecolors: Vec<&str> = y
        .iter()
        .map(|&y| if y > 0.0 { "green" } else { "red" })
        .collect();
    ax1.bar(
        &x,
        &y,
        BarOptions {
            color: facecolors.iter().map(|&c| Color::from(c)).collect(),
            edgecolor: facecolors.iter().map(|&c| Color::from(c)).collect(),
            props: PatchProps::default().alpha(0.5),
            ..BarOptions::default()
        },
    )?;
    ax1.set_title("Explicit 'alpha' keyword value\nshared by all bars and edges");
    let biggest = y.iter().fold(0.0_f64, |max, y| max.max(y.abs()));
    let face_alphas = map(&y, |y| y.abs() / biggest);
    // `(color, alpha)` pairs: the colour with its alpha replaced.
    let with_alphas = |alphas: &[f64]| -> Result<Vec<Color>, plt::colors::ColorError> {
        facecolors
            .iter()
            .zip(alphas)
            .map(|(&name, &alpha)| {
                let mut rgba = Color::from(name).resolve()?;
                rgba.a = alpha;
                Ok(Color::from(rgba))
            })
            .collect()
    };
    ax2.bar(
        &x,
        &y,
        BarOptions {
            color: with_alphas(&face_alphas)?,
            edgecolor: with_alphas(&map(&face_alphas, |alpha| 1.0 - alpha))?,
            ..BarOptions::default()
        },
    )?;
    ax2.set_title("Normalized alphas for\neach bar and each edge");
    Ok(())
}

/// One line from 0 to `i` per colour, in a constrained figure.
fn fan(fig: &Figure, colors: impl IntoIterator<Item = Color>) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    for (i, color) in colors.into_iter().enumerate() {
        ax.plot_with(
            [0.0, 1.0],
            [0.0, i as f64],
            LineProps::default().color(color),
        )?;
    }
    Ok(())
}

fn individual_colors_from_cmap_1(fig: &Figure) -> CaseResult {
    let plasma = colormaps::get("plasma")?;
    fan(
        fig,
        linspace(0.0, 1.0, 21)
            .into_iter()
            .map(|v| Color::from(plasma.sample(v))),
    )
}

fn individual_colors_from_cmap_2(fig: &Figure) -> CaseResult {
    let dark2: Colormap = colormaps::get("Dark2")?;
    fan(fig, dark2.lut().iter().map(|&c| Color::from(c)))
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/colorbar_basics", colorbar_basics).sized(13.0, 3.0, 100.0),
    FigureCase::new("gallery_ticks/colorbar_histogram", colorbar_histogram),
    FigureCase::new("gallery_ticks/color_by_yvalue", color_by_yvalue),
    FigureCase::new("gallery_ticks/color_cycle_default", color_cycle_default),
    FigureCase::new("gallery_ticks/color_demo", color_demo),
    FigureCase::new("gallery_ticks/color_sequences", color_sequences).sized(6.4, 9.6, 100.0),
    FigureCase::new("gallery_ticks/set_alpha", set_alpha).sized(8.0, 4.0, 100.0),
    FigureCase::new(
        "gallery_ticks/individual_colors_from_cmap_1",
        individual_colors_from_cmap_1,
    ),
    FigureCase::new(
        "gallery_ticks/individual_colors_from_cmap_2",
        individual_colors_from_cmap_2,
    ),
];
