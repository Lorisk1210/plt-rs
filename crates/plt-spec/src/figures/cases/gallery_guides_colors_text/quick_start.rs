//! The quick start guide: a first plot, data plotted against data, the
//! object-oriented interface, line, marker and scatter styles, labels and
//! annotations, legends, scales, ticks, dates and categories, twin and
//! secondary axes, colour-mapped plots with colorbars and a mosaic of axes
//! (`quick_start`).

use std::error::Error;
use std::f64::consts::PI;

use plt::axes::HistPlotOptions;
use plt::axes::{ImshowOptions, Mesh, PcolorOptions, SecondaryLocation};
use plt::collections::EdgeColors;
use plt::colorbar::ColorbarOptions;
use plt::colors::{ColorizerOptions, Extend, LogNorm, colormaps};
use plt::contour::{QuadCoords, QuadField};
use plt::dates::{ConciseDateFormatter, DateTime, TimeDelta};
use plt::figure::MosaicOptions;
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::scale::FuncTransform;
use plt::spines::SpineSide;
use plt::stats::{Bins, HistOptions};

use crate::figures::cases::common::{
    GALLERY_SEED, cumsum, func3, local_max, map, mesh_field, mosaic, values,
};
use crate::figures::cases::common::{constrained, indices};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `data1, data2, data3, data4 = np.random.randn(4, 100)`.
fn random_sets() -> [Vec<f64>; 4] {
    let values = SpecRng::new(GALLERY_SEED).normal(400, 0.0, 1.0);
    std::array::from_fn(|i| values[i * 100..(i + 1) * 100].to_vec())
}

fn quick_start_1(fig: &Figure) -> CaseResult {
    fig.subplot()
        .plot([1.0, 2.0, 3.0, 4.0], [1.0, 4.0, 2.0, 3.0])?;
    Ok(())
}

fn quick_start_2(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(GALLERY_SEED);
    let a = indices(50);
    let c: Vec<f64> = rng
        .integers(50, 0, 50)
        .into_iter()
        .map(|v| v as f64)
        .collect();
    let d = rng.normal(50, 0.0, 1.0);
    let noise = rng.normal(50, 0.0, 1.0);
    let b: Vec<f64> = a.iter().zip(&noise).map(|(a, n)| a + 10.0 * n).collect();
    let d = map(&d, |d| d.abs() * 100.0);
    let ax = constrained(fig)?;
    ax.scatter_with(
        a,
        b,
        ScatterOptions {
            sizes: Some(d),
            colors: MappedColors::new(c).into(),
            ..ScatterOptions::default()
        },
    )?;
    ax.set_xlabel("entry a");
    ax.set_ylabel("entry b");
    Ok(())
}

fn quick_start_3(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 2.0, 100);
    let ax = constrained(fig)?;
    for (power, label) in [(1, "linear"), (2, "quadratic"), (3, "cubic")] {
        ax.plot(&x, map(&x, |x| x.powi(power)))?.set_label(label);
    }
    ax.set_xlabel("x label");
    ax.set_ylabel("y label");
    ax.set_title("Simple Plot");
    ax.legend(LegendOptions::default())?;
    Ok(())
}

/// `my_plotter`: a helper function to make a graph.
fn my_plotter(
    ax: &Axes,
    data1: &[f64],
    data2: &[f64],
    props: LineProps,
) -> Result<Line2D, Box<dyn Error>> {
    Ok(ax.plot_with(data1, data2, props)?)
}

fn quick_start_5(fig: &Figure) -> CaseResult {
    let [data1, data2, data3, data4] = random_sets();
    let axs = fig.subplots(1, 2)?;
    my_plotter(
        &axs[(0, 0)],
        &data1,
        &data2,
        LineProps::default().marker("x".parse::<Marker>()?),
    )?;
    my_plotter(
        &axs[(0, 1)],
        &data3,
        &data4,
        LineProps::default().marker("o".parse::<Marker>()?),
    )?;
    Ok(())
}

fn quick_start_6(fig: &Figure) -> CaseResult {
    let [data1, data2, ..] = random_sets();
    let ax = fig.subplot();
    let x = indices(data1.len());
    ax.plot_with(
        &x,
        cumsum(&data1),
        LineProps::default()
            .color("blue")
            .linewidth(3.0)
            .linestyle(LineStyle::Dashed),
    )?;
    let l = ax.plot_with(
        &x,
        cumsum(&data2),
        LineProps::default().color("orange").linewidth(2.0),
    )?;
    l.set_linestyle(LineStyle::Dotted);
    Ok(())
}

fn quick_start_7(fig: &Figure) -> CaseResult {
    let [data1, data2, ..] = random_sets();
    fig.subplot().scatter_with(
        data1,
        data2,
        ScatterOptions {
            sizes: Some(vec![50.0]),
            colors: ScatterColors::Single(Color::from("C0")),
            edgecolors: Some(EdgeColors::Colors(vec![Color::from("k")])),
            ..ScatterOptions::default()
        },
    )?;
    Ok(())
}

fn quick_start_8(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    for (data, (marker, label)) in random_sets().iter().zip([
        ("o", "data1"),
        ("d", "data2"),
        ("v", "data3"),
        ("s", "data4"),
    ]) {
        ax.plot_with(
            indices(data.len()),
            data,
            LineProps::fmt(marker)?.label(label),
        )?;
    }
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn quick_start_9(fig: &Figure) -> CaseResult {
    let (mu, sigma) = (115.0, 15.0);
    let x = map(&SpecRng::new(GALLERY_SEED).normal(10_000, 0.0, 1.0), |v| {
        mu + sigma * v
    });
    let ax = constrained(fig)?;
    ax.hist(
        &[x],
        HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Count(50)),
                density: true,
                ..HistOptions::default()
            },
            props: PatchProps::default().facecolor("C0").alpha(0.75),
            ..HistPlotOptions::default()
        },
    )?;
    ax.set_xlabel("Length [cm]");
    ax.set_ylabel("Probability");
    ax.set_title("Aardvark lengths\n (not really)");
    ax.text(75.0, 0.025, r"$\mu=115,\ \sigma=15$");
    ax.set_xlim(55.0, 175.0)?.set_ylim(0.0, 0.03)?;
    ax.grid(true);
    Ok(())
}

fn quick_start_11(fig: &Figure) -> CaseResult {
    let [data1, data2, data3, _] = random_sets();
    let ax = fig.subplot();
    let x = indices(data1.len());
    ax.plot(&x, &data1)?.set_label("data1");
    ax.plot(&x, &data2)?.set_label("data2");
    ax.plot_with(&x, &data3, LineProps::fmt("d")?.label("data3"))?;
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn quick_start_12(fig: &Figure) -> CaseResult {
    let [data1, ..] = random_sets();
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?;
    let xdata = indices(data1.len());
    let data = map(&data1, |v| 10f64.powf(v));
    axs[(0, 0)].plot(&xdata, &data)?;
    axs[(0, 1)].set_yscale("log")?;
    axs[(0, 1)].plot(&xdata, &data)?;
    Ok(())
}

fn quick_start_13(fig: &Figure) -> CaseResult {
    let [data1, ..] = random_sets();
    let xdata = indices(data1.len());
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 1)?;
    let (automatic, manual) = (&axs[(0, 0)], &axs[(1, 0)]);
    automatic.plot(&xdata, &data1)?;
    automatic.set_title("Automatic ticks");
    manual.plot(&xdata, &data1)?;
    manual.set_xticks(arange(0.0, 100.0, 30.0));
    manual.set_xticklabels(["zero", "30", "sixty", "90"])?;
    manual.set_yticks(vec![-1.5, 0.0, 1.5]);
    manual.set_title("Manual ticks");
    Ok(())
}

fn quick_start_14(fig: &Figure) -> CaseResult {
    let start = DateTime::new(2021, 11, 15, 0, 0, 0, 0)?;
    let hours = 40 * 24;
    let dates = (0..hours)
        .map(|hour| start.checked_add(TimeDelta::new(0, 3600 * hour, 0)?))
        .collect::<Result<Vec<_>, _>>()?;
    let data = cumsum(&SpecRng::new(GALLERY_SEED).normal(dates.len(), 0.0, 1.0));
    let ax = constrained(fig)?;
    ax.plot(dates, data)?;
    ax.xaxis().set_major_formatter(ConciseDateFormatter::new());
    Ok(())
}

fn quick_start_15(fig: &Figure) -> CaseResult {
    let categories = vec!["turnips", "rutabaga", "cucumber", "pumpkins"];
    let heights = SpecRng::new(GALLERY_SEED).uniform(categories.len(), 0.0, 1.0);
    constrained(fig)?.bar(categories, heights, BarOptions::default())?;
    Ok(())
}

fn quick_start_16(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 5.0, 0.01);
    let s = map(&t, |t| (2.0 * PI * t).cos());
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax3) = (&axs[(0, 0)], &axs[(0, 1)]);
    let l1 = ax1.plot(&t, &s)?;
    let ax2 = ax1.twinx();
    let l2 = ax2.plot_with(&t, indices(t.len()), LineProps::fmt("C1")?)?;
    ax2.legend(
        LegendOptions::default()
            .handles([&l1, &l2])
            .labels(["Sine (left)", "Straight (right)"]),
    )?;
    ax3.plot(&t, &s)?;
    ax3.set_xlabel("Angle [rad]");
    let degrees = FuncTransform::new(|x| x * (180.0 / PI), |x| x * (PI / 180.0));
    ax3.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(degrees))?
        .set_xlabel("Angle [°]");
    Ok(())
}

/// A colorbar beside `ax`, extended at both ends.
fn extended(ax: &Axes) -> ColorbarOptions {
    ColorbarOptions {
        extend: Some(Extend::Both),
        ..ColorbarOptions::beside(ax)
    }
}

fn quick_start_17(fig: &Figure) -> CaseResult {
    let [data1, data2, data3, _] = random_sets();
    let grid = linspace(-3.0, 3.0, 128);
    let (x, y, z) = mesh_field(&grid, &grid, func3);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 2)?;
    let rdbu = || colormaps::get("RdBu_r");
    let pc = axs[(0, 0)].pcolormesh(
        Mesh::from_coords(x.clone(), y.clone(), z.clone()),
        PcolorOptions {
            colorizer: ColorizerOptions::default()
                .cmap(rdbu()?)
                .limits(Some(-1.0), Some(1.0)),
            ..PcolorOptions::default()
        },
    )?;
    fig.colorbar(&pc, ColorbarOptions::beside(&axs[(0, 0)]))?;
    axs[(0, 0)].set_title("pcolormesh()");
    let field = QuadField::new(z.clone()).with_coords(QuadCoords::Mesh { x, y });
    let co = axs[(0, 1)].contourf(&field, values(&linspace(-1.25, 1.25, 11)))?;
    fig.colorbar(&co, ColorbarOptions::beside(&axs[(0, 1)]))?;
    axs[(0, 1)].set_title("contourf()");
    let squared = Grid::from_fn(z.rows(), z.cols(), |r, c| z[(r, c)].powi(2) * 100.0);
    let image = axs[(1, 0)].imshow(
        squared,
        ImshowOptions::default().colorizer(
            ColorizerOptions::default()
                .cmap(colormaps::get("plasma")?)
                .norm(LogNorm::new(0.01, 100.0)),
        ),
    )?;
    fig.colorbar(&image, extended(&axs[(1, 0)]))?;
    axs[(1, 0)].set_title("imshow() with LogNorm()");
    let points = axs[(1, 1)].scatter_with(
        data1,
        data2,
        ScatterOptions {
            colors: MappedColors::new(data3).cmap(rdbu()?).into(),
            ..ScatterOptions::default()
        },
    )?;
    fig.colorbar(&points, extended(&axs[(1, 1)]))?;
    axs[(1, 1)].set_title("scatter()");
    Ok(())
}

fn quick_start_18(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let layout = mosaic(&[&["upleft", "right"], &["lowleft", "right"]])?;
    let axd = fig.subplot_mosaic(&layout, MosaicOptions::default())?;
    for label in ["upleft", "lowleft", "right"] {
        axd.get(label)
            .ok_or_else(|| format!("no axes labelled {label}"))?
            .set_title(label);
    }
    Ok(())
}

/// The guide's small figure size, in inches.
const SMALL: (f64, f64) = (5.0, 2.7);

/// The cases of this module; `quick_start_4` draws `quick_start_3`
/// through pyplot.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_colors_text/quick_start_1", quick_start_1),
    FigureCase::new("gallery_guides_colors_text/quick_start_2", quick_start_2)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_3", quick_start_3)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_5", quick_start_5)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_6", quick_start_6)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_7", quick_start_7)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_8", quick_start_8)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_9", quick_start_9)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_10", local_max)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_11", quick_start_11)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_12", quick_start_12)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_13", quick_start_13),
    FigureCase::new("gallery_guides_colors_text/quick_start_14", quick_start_14)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_15", quick_start_15)
        .sized(SMALL.0, SMALL.1, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_16", quick_start_16)
        .sized(7.0, 2.7, 100.0),
    FigureCase::new("gallery_guides_colors_text/quick_start_17", quick_start_17),
    FigureCase::new("gallery_guides_colors_text/quick_start_18", quick_start_18),
];
