//! Vector fields: `barb_demo`, `quiver_demo`, `quiver_simple_demo` and
//! `plot_streamplot`.

use std::f64::consts::PI;

use plt::axes::{Axes, ImshowOptions, MappedColors, ScatterColors, ScatterOptions};
use plt::collections::EdgeColors;
use plt::colorbar::ColorbarOptions;
use plt::colors::{Color, Colorize, ColorizerOptions, ColormapError, colormaps};
use plt::figure::{Figure, Share};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::image::{Extent, ImageAlpha, ScalarField};
use plt::layout::TightLayout;
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::patches::PatchProps;
use plt::quiver::{
    BarbIncrements, BarbPivot, BarbSizes, BarbsOptions, KeyCoordinates, LabelPosition, Pivot,
    Quiver, QuiverKeyOptions, QuiverOptions, QuiverUnits, VectorField,
};
use plt::rc::Aspect;
use plt::streamplot::{StreamColorMap, StreamColors, StreamField, StreamWidths, StreamplotOptions};
use plt::text::{HAlign, VAlign};
use plt::transforms::Point;

use super::data::mesh_field;
use crate::figures::cases::common::{mgrid_count, tickless};
use crate::figures::{CaseResult, FigureCase};

/// `x, y, u, v` of `barb_demo`'s six scattered barbs.
const BARB_POINTS: [[f64; 4]; 6] = [
    [-1.5, 0.5, -6.0, -6.0],
    [1.0, -1.0, -46.0, 46.0],
    [-3.0, -1.0, 11.0, -11.0],
    [1.0, 1.5, 80.0, 80.0],
    [0.5, 0.25, 25.0, 15.0],
    [-1.5, -0.5, -5.0, 40.0],
];

/// The scattered barbs, with the components of point `masked` (if any)
/// left out.
fn barb_points(masked: Option<usize>) -> VectorField {
    let column = |k: usize| -> Vec<f64> {
        BARB_POINTS
            .iter()
            .enumerate()
            .map(|(i, point)| {
                if k >= 2 && Some(i) == masked {
                    f64::NAN
                } else {
                    point[k]
                }
            })
            .collect()
    };
    VectorField::new(column(0), column(1), column(2), column(3))
}

/// The barbs `12 x, 12 y` on a 5 × 5 grid over `[-5, 5]²`, and their
/// speeds.
fn barb_grid() -> (VectorField, Vec<f64>) {
    let x = linspace(-5.0, 5.0, 5);
    let (xx, yy) = meshgrid(&x, &x);
    let u = xx.map(|x| 12.0 * x);
    let v = yy.map(|y| 12.0 * y);
    let speed = u
        .as_slice()
        .iter()
        .zip(v.as_slice())
        .map(|(u, v)| (u * u + v * v).sqrt())
        .collect();
    (VectorField::on_mesh(&xx, &yy, &u, &v), speed)
}

/// Long barbs centred on their positions.
fn long_middle() -> BarbsOptions {
    BarbsOptions {
        length: 8.0,
        pivot: BarbPivot::Middle,
        ..BarbsOptions::default()
    }
}

fn barb_demo_1(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let (grid, speed) = barb_grid();
    axs[(0, 0)].barbs(grid, BarbsOptions::default())?;
    axs[(0, 1)].barbs(barb_points(None), long_middle())?;
    let (grid, _) = barb_grid();
    let filled = BarbsOptions {
        fill_empty: true,
        rounding: false,
        sizes: BarbSizes {
            emptybarb: 0.25,
            spacing: 0.2,
            height: 0.3,
            ..BarbSizes::default()
        },
        ..BarbsOptions::default()
    };
    axs[(1, 0)].barbs(grid.with_colors(MappedColors::new(speed)), filled)?;
    let mut flipped = BarbsOptions {
        flagcolor: Some("r".into()),
        flip: vec![true],
        increments: BarbIncrements {
            half: 10.0,
            full: 20.0,
            flag: 100.0,
        },
        ..BarbsOptions::default()
    };
    // matplotlib hands a sequence of barb colours to the collection's
    // edge colours.
    flipped.props.edgecolors = Some(EdgeColors::Colors(vec!["b".into(), "g".into()]));
    axs[(1, 1)].barbs(barb_points(None), flipped)?;
    Ok(())
}

fn barb_demo_2(fig: &Figure) -> CaseResult {
    fig.subplot().barbs(barb_points(Some(4)), long_middle())?;
    Ok(())
}

/// `quiver_demo`'s field: `cos x, sin y` on a grid of steps 0.2 over
/// `[0, 2π)²`, every `step`-th point in both directions.
fn quiver_field(step: usize) -> (Grid<f64>, Grid<f64>, VectorField) {
    let every: Vec<f64> = arange(0.0, 2.0 * PI, 0.2)
        .into_iter()
        .step_by(step)
        .collect();
    let (xx, yy) = meshgrid(&every, &every);
    let u = xx.map(|x| x.cos());
    let v = yy.map(|y| y.sin());
    let field = VectorField::on_mesh(&xx, &yy, &u, &v);
    (xx, yy, field)
}

/// A key of length `length` at `(0.9, 0.9)` in figure fractions, labelled
/// on its right.
fn figure_key(ax: &Axes, q: &Quiver, length: f64, label: &str) {
    let options = QuiverKeyOptions {
        labelpos: LabelPosition::East,
        coordinates: KeyCoordinates::Figure,
        ..QuiverKeyOptions::default()
    };
    ax.quiverkey(q, (0.9, 0.9), length, label, options);
}

/// Scatter markers of one colour and size.
fn dots(color: &str, size: f64) -> ScatterOptions {
    ScatterOptions {
        colors: ScatterColors::Single(Color::from(color)),
        sizes: Some(vec![size]),
        ..ScatterOptions::default()
    }
}

fn quiver_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_title("Arrows scale with plot width, not view");
    let (_, _, field) = quiver_field(1);
    let q = ax.quiver(field, QuiverOptions::default())?;
    figure_key(&ax, &q, 2.0, r"$2 \frac{m}{s}$");
    Ok(())
}

fn quiver_demo_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_title("pivot='middle'; every third arrow; units='inches'");
    let (xx, yy, field) = quiver_field(3);
    let options = QuiverOptions {
        pivot: Pivot::Middle,
        units: QuiverUnits::Inches,
        ..QuiverOptions::default()
    };
    let q = ax.quiver(field, options)?;
    figure_key(&ax, &q, 1.0, r"$1 \frac{m}{s}$");
    ax.scatter_with(xx.as_slice(), yy.as_slice(), dots("r", 5.0))?;
    Ok(())
}

fn quiver_demo_3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_title("pivot='tip'; scales with x view");
    let (xx, yy, field) = quiver_field(1);
    let speed: Vec<f64> = field
        .u
        .iter()
        .zip(&field.v)
        .map(|(u, v)| u.hypot(*v))
        .collect();
    let options = QuiverOptions {
        units: QuiverUnits::X,
        pivot: Pivot::Tip,
        width: Some(0.022),
        scale: Some(1.0 / 0.15),
        ..QuiverOptions::default()
    };
    let q = ax.quiver(field.with_colors(MappedColors::new(speed)), options)?;
    figure_key(&ax, &q, 1.0, r"$1 \frac{m}{s}$");
    ax.scatter_with(xx.as_slice(), yy.as_slice(), dots("0.5", 1.0))?;
    Ok(())
}

fn quiver_simple_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(-10.0, 10.0, 1.0);
    let (u, v) = meshgrid(&x, &x);
    let q = ax.quiver(
        VectorField::on_axes(&x, &x, &u, &v),
        QuiverOptions::default(),
    )?;
    let options = QuiverKeyOptions {
        labelpos: LabelPosition::East,
        ..QuiverKeyOptions::default()
    };
    ax.quiverkey(&q, (0.3, 1.1), 10.0, "Quiver key, length = 10", options);
    Ok(())
}

/// `plot_streamplot`'s field `u = -1 - x² + y`, `v = 1 + x - y²` on
/// `np.mgrid[-3:3:100j, -3:3:100j]`.
fn stream_field() -> StreamField {
    let x = mgrid_count(-3.0, 3.0, 100);
    let (_, _, u) = mesh_field(&x, &x, |x, y| -1.0 - x * x + y);
    let (_, _, v) = mesh_field(&x, &x, |x, y| 1.0 + x - y * y);
    StreamField::new(x.clone(), x, u, v)
}

/// Streamlines coloured by `values` through `autumn`, two points wide.
fn autumn_lines(values: &Grid<f64>) -> Result<StreamplotOptions, ColormapError> {
    Ok(StreamplotOptions {
        color: Some(StreamColors::Mapped(Box::new(StreamColorMap {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("autumn")?),
                norm: None,
                ..ColorizerOptions::default()
            },
            values: values.clone(),
        }))),
        linewidth: Some(StreamWidths::Uniform(2.0)),
        ..StreamplotOptions::default()
    })
}

fn plot_streamplot_1(fig: &Figure) -> CaseResult {
    let grid = GridSpec::with_ratios(
        4,
        2,
        None,
        Some(vec![1.0, 1.0, 1.0, 2.0]),
        SubplotOverrides::default(),
    )?;
    let axs = fig.subplots_on(&grid, Share::None, Share::None)?;
    let axs = axs.as_slice();
    let field = stream_field();

    axs[0].streamplot(
        &field,
        StreamplotOptions {
            density: (0.5, 1.0),
            ..StreamplotOptions::default()
        },
    )?;
    axs[0].set_title("Varying Density");

    let strm = axs[1].streamplot(&field, autumn_lines(&field.u)?)?;
    fig.colorbar(&strm.lines, ColorbarOptions::default())?;
    axs[1].set_title("Varying Color");

    let speed = Grid::from_fn(100, 100, |i, j| field.u[(i, j)].hypot(field.v[(i, j)]));
    let fastest = speed
        .as_slice()
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let widths = StreamplotOptions {
        density: (0.6, 0.6),
        color: Some(StreamColors::Single("k".into())),
        linewidth: Some(StreamWidths::Varying(speed.map(|s| 5.0 * s / fastest))),
        num_arrows: 5,
        ..StreamplotOptions::default()
    };
    axs[2].streamplot(&field, widths)?;
    axs[2].set_title("Varying Line Width");

    let seeds = [
        (-2.0, -2.0),
        (-1.0, -1.0),
        (0.0, 0.0),
        (1.0, 1.0),
        (2.0, 2.0),
        (-1.0, 2.0),
    ];
    let seeded = StreamplotOptions {
        start_points: Some(seeds.iter().map(|&(x, y)| Point::new(x, y)).collect()),
        ..autumn_lines(&field.u)?
    };
    let strm = axs[3].streamplot(&field, seeded)?;
    fig.colorbar(&strm.lines, ColorbarOptions::default())?;
    axs[3].set_title("Controlling Starting Points");
    let (sx, sy): (Vec<f64>, Vec<f64>) = seeds.into_iter().unzip();
    axs[3].plot_fmt(sx, sy, "bo")?;
    axs[3].set_xlim(-3.0, 3.0)?;
    axs[3].set_ylim(-3.0, 3.0)?;

    axs[4].streamplot(
        &field,
        StreamplotOptions {
            num_arrows: 3,
            ..StreamplotOptions::default()
        },
    )?;
    axs[4].set_title("Multiple arrows");
    axs[5].set_axis_off();

    let masked = |i: usize, j: usize| (40..60).contains(&i) && (40..60).contains(&j);
    let holey = StreamField {
        u: Grid::from_fn(100, 100, |i, j| {
            if masked(i, j) || (i < 20 && j < 20) {
                f64::NAN
            } else {
                field.u[(i, j)]
            }
        }),
        ..field
    };
    let red = StreamplotOptions {
        color: Some(StreamColors::Single("r".into())),
        ..StreamplotOptions::default()
    };
    axs[6].streamplot(&holey, red)?;
    axs[6].set_title("Streamplot with Masking");
    let image = ImshowOptions {
        colorizer: ColorizerOptions::default().cmap(colormaps::get("gray")?),
        extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
        alpha: Some(ImageAlpha::Uniform(0.5)),
        aspect: Some(Aspect::Auto),
        ..ImshowOptions::default()
    };
    let visible = Grid::from_fn(100, 100, |i, j| !masked(i, j));
    axs[6].imshow(ScalarField::booleans(&visible), image)?;
    axs[6].set_aspect(Aspect::Equal)?;

    let unbroken = StreamplotOptions {
        broken_streamlines: false,
        ..StreamplotOptions::default()
    };
    axs[7].streamplot(&holey, unbroken)?;
    axs[7].set_title("Streamplot with unbroken streamlines");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The flow around a cylinder of `plot_streamplot`'s second figure.
fn cylinder_flow() -> StreamField {
    let x = linspace(-2.0, 2.0, 50);
    let y = linspace(-3.0, 3.0, 50);
    let (xx, yy) = meshgrid(&x, &y);
    let polar = |i: usize, j: usize| {
        let (x, y) = (xx[(i, j)], yy[(i, j)]);
        let th = y.atan2(x);
        let r = (x * x + y * y).sqrt();
        let vr = -th.cos() / (r * r);
        let vt = -th.sin() / (r * r) - 1.0 / r;
        (th, vr, vt)
    };
    let u = Grid::from_fn(50, 50, |i, j| {
        let (th, vr, vt) = polar(i, j);
        vr * th.cos() - vt * th.sin() + 1.0
    });
    let v = Grid::from_fn(50, 50, |i, j| {
        let (th, vr, vt) = polar(i, j);
        vr * th.sin() + vt * th.cos()
    });
    StreamField::new(x, y, u, v)
}

fn plot_streamplot_2(fig: &Figure) -> CaseResult {
    let field = cylinder_flow();
    let seeds: Vec<Point> = linspace(-2.0, 2.0, 50)
        .into_iter()
        .map(|y| Point::new(-1.75, y))
        .collect();
    let axs = fig.subplots(3, 1)?;
    let th_circ = linspace(0.0, 2.0 * PI, 100);
    let (cx, cy): (Vec<f64>, Vec<f64>) = th_circ.iter().map(|t| (t.cos(), t.sin())).unzip();
    // The example's scales are `[0.05, 1, 5]`, printed as Python does.
    for (ax, (max_val, printed)) in
        axs.as_slice()
            .iter()
            .zip([(0.05, "0.05"), (1.0, "1"), (5.0, "5")])
    {
        let ax_ins = ax.inset_axes([0.0, 0.7, 0.3, 0.35]);
        for (ax_curr, is_inset) in [(ax, false), (&ax_ins, true)] {
            let options = StreamplotOptions {
                start_points: Some(seeds.clone()),
                broken_streamlines: false,
                arrowsize: 1e-10,
                linewidth: Some(StreamWidths::Uniform(if is_inset { 2.0 } else { 0.6 })),
                color: Some(StreamColors::Single("k".into())),
                integration_max_step_scale: max_val,
                integration_max_error_scale: max_val,
                ..StreamplotOptions::default()
            };
            ax_curr.streamplot(&field, options)?;
            let disc = PatchProps::default()
                .color("w")
                .edgecolor("k")
                .linewidth(if is_inset { 6.0 } else { 2.0 });
            ax_curr.fill(&cx, &cy, disc)?;
            ax_curr.set_aspect(Aspect::Equal)?;
        }
        let mut text = format!(
            "integration_max_step_scale: {printed}\nintegration_max_error_scale: {printed}"
        );
        if printed == "1" {
            text.push_str("\n(default)");
        }
        ax.text(0.0, 0.0, text)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center);
        ax_ins.set_xlim(-1.2, -0.7)?;
        ax_ins.set_ylim(-0.8, -0.4)?;
        tickless(&ax_ins);
        ax.set_ylim(-1.5, 1.5)?;
        ax.set_axis_off();
        ax.indicate_inset_zoom(&ax_ins, PatchProps::default().edgecolor("k"));
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_images/barb_demo_1", barb_demo_1),
    FigureCase::new("gallery_images/barb_demo_2", barb_demo_2),
    FigureCase::new("gallery_images/quiver_demo_1", quiver_demo_1),
    FigureCase::new("gallery_images/quiver_demo_2", quiver_demo_2),
    FigureCase::new("gallery_images/quiver_demo_3", quiver_demo_3),
    FigureCase::new("gallery_images/quiver_simple_demo", quiver_simple_demo),
    FigureCase::new("gallery_images/plot_streamplot_1", plot_streamplot_1).sized(7.0, 12.0, 100.0),
    FigureCase::new("gallery_images/plot_streamplot_2", plot_streamplot_2).sized(6.0, 14.0, 100.0),
];
