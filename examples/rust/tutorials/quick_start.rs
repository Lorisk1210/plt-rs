//! matplotlib's "Quick start guide" in pltrs (Rust): the same figure
//! made in the object-oriented style and in the pyplot style, then
//! styling, labels, scales, extra axes and colour-mapped data.
//!
//! ```text
//! cargo run -p pltrs-examples --example quick_start   # writes quick_start_*.png here
//! ```
//!
//! Each section names the matplotlib calls it stands for.

use pltrs::axes::{Mesh, PcolorOptions, SecondaryLocation};
use pltrs::colorbar::ColorbarAxes;
use pltrs::colors::ColorizerOptions;
use pltrs::contour::{ContourOptions, Levels, QuadCoords, QuadField};
use pltrs::figure::Share;
use pltrs::image::Extent;
use pltrs::layout::LayoutEngine;
use pltrs::numeric::pow;
use pltrs::prelude::*;
use pltrs::pyplot as plt;
use pltrs::scale::FuncTransform;
use pltrs::spines::SpineSide;
use pltrs::stats::Bins;
use pltrs::text::{AnnotateOptions, ArrowProps, FancyArrowProps};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    object_style()?;
    pyplot_style()?;
    styling_and_labels()?;
    scales_and_extra_axes()?;
    color_mapped_data()?;
    Ok(())
}

/// `fig, ax = plt.subplots(); ax.plot(x, x, label="linear"); ...`: the
/// explicit interface, where every call goes to the figure or axes it
/// changes.
fn object_style() -> Result<(), Box<dyn std::error::Error>> {
    let x = linspace(0.0, 2.0, 100);
    let fig = Figure::with_size(5.0, 2.7, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    ax.plot_with(&x, &x, LineProps::default().label("linear"))?;
    ax.plot_with(
        &x,
        x.iter().map(|v| v * v),
        LineProps::default().label("quadratic"),
    )?;
    ax.plot_with(
        &x,
        x.iter().map(|v| v.powi(3)),
        LineProps::default().label("cubic"),
    )?;
    ax.set_xlabel("x label").set_ylabel("y label");
    ax.set_title("Simple Plot");
    ax.legend(LegendOptions::default())?;
    fig.savefig("quick_start_object.png")?;
    Ok(())
}

/// `plt.figure(figsize=...); plt.plot(...); plt.xlabel(...)`: the
/// implicit interface, where pyplot keeps the current figure and axes.
fn pyplot_style() -> Result<(), Box<dyn std::error::Error>> {
    let x = linspace(0.0, 2.0, 100);
    plt::figure_with_size(5.0, 2.7, 100.0);
    plt::plot_with(&x, &x, LineProps::default().label("linear"))?;
    plt::plot_with(
        &x,
        x.iter().map(|v| v * v),
        LineProps::default().label("quadratic"),
    )?;
    plt::plot_with(
        &x,
        x.iter().map(|v| v.powi(3)),
        LineProps::default().label("cubic"),
    )?;
    plt::xlabel("x label");
    plt::ylabel("y label");
    plt::title("Simple Plot");
    plt::legend(LegendOptions::default())?;
    plt::savefig("quick_start_pyplot.png")?;
    plt::close();
    Ok(())
}

/// Colours, line widths, line styles and markers (`color=`,
/// `linewidth=`, `linestyle=`, `marker=`), axis labels, text with math
/// (`ax.text(..., r"$\mu=115$")`), an annotation with an arrow and a
/// legend.
fn styling_and_labels() -> Result<(), Box<dyn std::error::Error>> {
    let mut samples = pltrs_examples::Samples::new(19_680_801);
    let iq = samples.normal(10_000, 100.0, 15.0);
    let t = arange(0.0, 5.0, 0.01);
    let wave: Vec<f64> = t
        .iter()
        .map(|t| (std::f64::consts::TAU * t).cos())
        .collect();

    let fig = Figure::with_size(9.0, 3.2, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let [hist_ax, wave_ax] = [fig.add_subplot(1, 2, 1)?, fig.add_subplot(1, 2, 2)?];

    let options = HistPlotOptions {
        bins: Some(Bins::Count(50)),
        density: true,
        props: PatchProps::default().facecolor("C0").alpha(0.75),
        ..HistPlotOptions::default()
    };
    hist_ax.hist(&iq, options)?;
    hist_ax.set_xlabel("Length [cm]").set_ylabel("Probability");
    hist_ax.set_title("Aardvark lengths\n(not really)");
    hist_ax.text(75.0, 0.025, r"$\mu=115,\ \sigma=15$");
    hist_ax.set_xlim(40.0, 160.0)?.set_ylim(0.0, 0.03)?;
    hist_ax.grid(true);

    wave_ax.plot_with(
        &t,
        &wave,
        LineProps::default()
            .color("orange")
            .linewidth(2.0)
            .linestyle(LineStyle::Dashed)
            .label("cos(2πt)"),
    )?;
    wave_ax.plot_fmt(&t[..50], &wave[..50], "o")?;
    let arrow = ArrowProps::Fancy(FancyArrowProps::new("->".parse()?));
    let options = AnnotateOptions {
        xytext: Some((3.0, 1.5).into()),
        arrowprops: Some(arrow),
        ..AnnotateOptions::default()
    };
    wave_ax.annotate("local max", (2.0, 1.0), options)?;
    wave_ax.set_ylim(-2.0, 2.0)?;
    wave_ax.legend(LegendOptions::default())?;
    fig.savefig("quick_start_styling.png")?;
    Ok(())
}

/// `ax.set_yscale("log")`, tick locations and labels
/// (`ax.set_xticks(...)`), `ax.twinx()` and `ax.secondary_xaxis(...)`.
fn scales_and_extra_axes() -> Result<(), Box<dyn std::error::Error>> {
    let x = arange(1.0, 100.0, 1.0);
    let fig = Figure::with_size(9.0, 3.2, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let [log_ax, twin_ax] = [fig.add_subplot(1, 2, 1)?, fig.add_subplot(1, 2, 2)?];

    log_ax.plot(&x, x.iter().map(|&v| pow(10.0, v / 20.0)))?;
    log_ax.set_yscale("log")?;
    log_ax.set_xticks([0.0, 25.0, 50.0, 75.0, 100.0]);
    log_ax.set_xticklabels(["zero", "25", "fifty", "75", "100"])?;
    log_ax.set_title("A log scale with named ticks");

    let t = linspace(0.0, 2.0 * std::f64::consts::PI, 100);
    twin_ax.plot(&t, t.iter().map(|t| t.sin()))?;
    let twin = twin_ax.twinx()?;
    twin.plot_with(
        &t,
        t.iter().map(|t| 20.0 * t.cos()),
        LineProps::default().color("C1"),
    )?;
    twin.set_ylabel("cos, scaled");
    let degrees = FuncTransform::new(f64::to_degrees, f64::to_radians);
    let top = twin_ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(degrees))?;
    top.set_xlabel("angle [°]");
    twin_ax.set_xlabel("angle [rad]");
    fig.savefig("quick_start_axes.png")?;
    Ok(())
}

/// `pcolormesh`, `contourf`, `imshow` and `scatter` with colorbars
/// (`fig.colorbar(pc, ax=ax)`), on a grid of axes sharing x and y
/// (`plt.subplots(2, 2, sharex=True, sharey=True)`).
fn color_mapped_data() -> Result<(), Box<dyn std::error::Error>> {
    let x = linspace(-3.0, 3.0, 128);
    let (xx, yy) = meshgrid(&x, &x);
    let z = Grid::from_fn(128, 128, |row, col| {
        let (x, y) = (xx[(row, col)], yy[(row, col)]);
        (1.0 - x / 2.0 + x.powi(5) + y.powi(3)) * (-x * x - y * y).exp()
    });

    let fig = Figure::with_size(7.0, 5.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots_shared(2, 2, Share::All, Share::All)?;

    let mesh = axs[(0, 0)].pcolormesh(
        Mesh::from_coords(x.clone(), x.clone(), z.clone()),
        PcolorOptions::default(),
    )?;
    fig.colorbar(&mesh, beside(&axs[(0, 0)]))?;
    axs[(0, 0)].set_title("pcolormesh()");

    let field = QuadField::new(z.clone()).with_coords(QuadCoords::Mesh { x: xx, y: yy });
    let bands = ContourOptions {
        levels: Levels::Count(10),
        ..ContourOptions::default()
    };
    let contours = axs[(0, 1)].contourf(&field, bands)?;
    fig.colorbar(&contours, beside(&axs[(0, 1)]))?;
    axs[(0, 1)].set_title("contourf()");

    let options = ImshowOptions {
        colorizer: ColorizerOptions::default().cmap(colormaps::get("plasma")?),
        extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
        ..ImshowOptions::default()
    };
    let image = axs[(1, 0)].imshow(z.clone(), options)?;
    fig.colorbar(&image, beside(&axs[(1, 0)]))?;
    axs[(1, 0)].set_title("imshow()");

    let mut samples = pltrs_examples::Samples::new(3);
    let (px, py) = (
        samples.uniform(100, -3.0, 3.0),
        samples.uniform(100, -3.0, 3.0),
    );
    let values: Vec<f64> = px.iter().zip(&py).map(|(x, y)| x * y).collect();
    let options = ScatterOptions {
        colors: MappedColors::new(&values)
            .cmap(colormaps::get("RdBu_r")?)
            .into(),
        ..ScatterOptions::default()
    };
    let points = axs[(1, 1)].scatter_with(&px, &py, options)?;
    fig.colorbar(&points, beside(&axs[(1, 1)]))?;
    axs[(1, 1)].set_title("scatter()");
    fig.savefig("quick_start_colormapped.png")?;
    Ok(())
}

/// A colorbar taking its room from `ax` (`fig.colorbar(mappable, ax=ax)`).
fn beside(ax: &Axes) -> ColorbarOptions {
    ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.clone()),
        ..ColorbarOptions::default()
    }
}
