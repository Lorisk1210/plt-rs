//! matplotlib's layout tutorials ("Arranging multiple Axes in a Figure",
//! "Constrained layout guide", "Tight layout guide") in pltrs (Rust):
//! grids of subplots, grid specs with ratios and spans, mosaics,
//! subfigures, insets, twins, and the layout engines that make room for
//! the labels.
//!
//! ```text
//! cargo run -p pltrs-examples --example layouts   # writes layouts_*.png here
//! ```

use pltrs::figure::{MosaicOptions, Share, SubfigureOptions};
use pltrs::gridspec::{GridSpec, SubplotOverrides};
use pltrs::layout::{LayoutEngine, TightLayout};
use pltrs::prelude::*;

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn main() -> Outcome {
    subplots_and_engines()?;
    grid_specs()?;
    mosaics_and_subfigures()?;
    insets_and_twins()?;
    Ok(())
}

/// A sine for the axes `ax`, labelled so that layouts have text to fit.
fn example_plot(ax: &Axes, title: &str) -> Outcome {
    let x = linspace(0.0, 10.0, 50);
    ax.plot(&x, x.iter().map(|x| x.sin()))?;
    ax.set_xlabel("x-label").set_ylabel("y-label");
    ax.set_title(title);
    Ok(())
}

/// `plt.subplots(2, 2, sharex=True)`, then the same grid without a
/// layout engine, with `subplots_adjust`, with `tight_layout()` and with
/// `layout="constrained"`.
fn subplots_and_engines() -> Outcome {
    let engines: [(&str, Option<LayoutEngine>); 3] = [
        ("none", None),
        ("tight", Some(LayoutEngine::Tight(TightLayout::default()))),
        ("constrained", Some(LayoutEngine::constrained())),
    ];
    for (name, engine) in engines {
        let fig = Figure::with_size(6.0, 4.5, 100.0);
        let axs = fig.subplots_shared(2, 2, Share::All, Share::None)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            example_plot(ax, &format!("Axes {i}"))?;
        }
        match engine {
            Some(engine) => fig.set_layout_engine(engine)?,
            None => fig.subplots_adjust(SubplotOverrides {
                hspace: Some(0.4),
                ..SubplotOverrides::default()
            })?,
        };
        fig.suptitle(format!("layout: {name}"));
        fig.savefig(format!("layouts_engine_{name}.png"))?;
    }
    Ok(())
}

/// `GridSpec(2, 3, width_ratios=[2, 1, 1])`, spans (`gs[0:2, 0]`) and a
/// grid nested in a cell (`gs[1, 1:].subgridspec(1, 2)`).
fn grid_specs() -> Outcome {
    let fig = Figure::with_size(7.0, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let gs = GridSpec::with_ratios(
        2,
        3,
        Some(vec![2.0, 1.0, 1.0]),
        None,
        SubplotOverrides::default(),
    )?;
    let tall = fig.add_subplot_spec(&gs.span(0..2, 0..1)?)?;
    example_plot(&tall, "gs[:, 0]")?;
    let wide = fig.add_subplot_spec(&gs.span(0..1, 1..3)?)?;
    example_plot(&wide, "gs[0, 1:]")?;
    let inner = GridSpec::nested(&gs.span(1..2, 1..3)?, 1, 2, None, None, (None, None))?;
    for col in 0..2 {
        let ax = fig.add_subplot_spec(&inner.cell(0, col)?)?;
        ax.plot([0.0, 1.0], [col as f64, 1.0 - col as f64])?;
        ax.set_title(format!("inner[0, {col}]"));
    }
    fig.savefig("layouts_gridspec.png")?;
    Ok(())
}

/// `fig.subplot_mosaic("AAB;C.B")` and `fig.subfigures(1, 2)`, each
/// subfigure with its own grid, title and background.
fn mosaics_and_subfigures() -> Outcome {
    let fig = Figure::with_size(7.0, 4.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axd = fig.subplot_mosaic("AAB;C.B", MosaicOptions::default())?;
    for (label, ax) in axd.iter() {
        example_plot(ax, &format!("ax[\"{label}\"]"))?;
    }
    fig.savefig("layouts_mosaic.png")?;

    let fig = Figure::with_size(7.0, 4.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let options = SubfigureOptions {
        width_ratios: Some(vec![2.0, 1.0]),
        ..SubfigureOptions::default()
    };
    let subfigs = fig.subfigures(1, 2, options)?;
    let (left, right) = (&subfigs[(0, 0)], &subfigs[(0, 1)]);
    for ax in left.subplots(2, 2)?.as_slice() {
        ax.plot([0.0, 1.0], [0.0, 1.0])?;
    }
    left.suptitle("Left: a 2 × 2 grid");
    left.set_facecolor("0.92");
    example_plot(&right.subplot(), "one axes")?;
    right.suptitle("Right");
    fig.suptitle("Subfigures");
    fig.savefig("layouts_subfigures.png")?;
    Ok(())
}

/// `ax.inset_axes([0.6, 0.6, 0.35, 0.35])` showing a detail, marked with
/// `ax.indicate_inset_zoom(inset)`, and `ax.twinx()`.
fn insets_and_twins() -> Outcome {
    let fig = Figure::with_size(7.0, 3.2, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let [main, twin_host] = [fig.add_subplot(1, 2, 1)?, fig.add_subplot(1, 2, 2)?];

    let x = linspace(0.0, 10.0, 400);
    let y: Vec<f64> = x.iter().map(|x| x.sin() + 0.1 * (20.0 * x).sin()).collect();
    main.plot(&x, &y)?;
    let inset = main.inset_axes([0.6, 0.6, 0.35, 0.35]);
    inset.plot(&x, &y)?;
    inset.set_xlim(1.0, 2.0)?.set_ylim(0.8, 1.15)?;
    main.indicate_inset_zoom(&inset, PatchProps::default());
    main.set_title("An inset zoom");

    twin_host.plot_with(
        &x,
        x.iter().map(|x| x.exp()),
        LineProps::default().color("C0"),
    )?;
    twin_host.set_ylabel("exp(x)");
    let twin = twin_host.twinx()?;
    twin.plot_with(
        &x,
        x.iter().map(|x| x.sqrt()),
        LineProps::default().color("C1"),
    )?;
    twin.set_ylabel("sqrt(x)");
    twin_host.set_title("Two y axes");
    fig.savefig("layouts_insets.png")?;
    Ok(())
}
