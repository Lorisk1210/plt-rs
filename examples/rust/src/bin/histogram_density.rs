//! Histogram with a density curve: 1500 commute times, normalised to a
//! density, against the log-normal distribution they were drawn from.

use std::f64::consts::PI;

use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::stats::Bins;
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps};
use plt_examples::Samples;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mu, sigma) = (3.3, 0.35); // of the logarithm of the time in minutes
    let minutes: Vec<f64> = Samples::new(3)
        .normal(1500, mu, sigma)
        .into_iter()
        .map(f64::exp)
        .collect();

    let x = linspace(5.0, 80.0, 300);
    let density = x.iter().map(|x| {
        (-(x.ln() - mu).powi(2) / (2.0 * sigma * sigma)).exp() / (x * sigma * (2.0 * PI).sqrt())
    });
    let median = mu.exp();

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let options = HistPlotOptions {
        bins: Some(Bins::Count(40)),
        range: Some((5.0, 80.0)),
        density: true,
        label: "survey".into(),
        props: PatchProps::default().alpha(0.6).edgecolor("white"),
        ..HistPlotOptions::default()
    };
    ax.hist(&minutes, options)?;
    let model = LineProps::default()
        .linewidth(2.5)
        .label("log-normal model");
    ax.plot_with(&x, density, model)?;
    let marker = LineProps::default()
        .color("0.3")
        .linestyle(LineStyle::Dashed)
        .linewidth(1.0);
    ax.axvline(median, marker);
    let arrow = FancyArrowProps {
        patch: PatchProps::default().color("0.3"),
        ..FancyArrowProps::new("->".parse()?)
    };
    let options = AnnotateOptions {
        xytext: Some((42.0, 0.036).into()),
        arrowprops: Some(ArrowProps::Fancy(arrow)),
        ..AnnotateOptions::default()
    };
    let note = ax.annotate(format!("median {median:.0} min"), (median, 0.03), options)?;
    note.set_color("0.3");
    ax.set_title("How long the commute takes");
    ax.set_xlabel("minutes").set_ylabel("density");
    ax.set_xlim(5.0, 80.0)?;
    ax.legend(LegendOptions::default())?;

    fig.savefig("histogram_density.png")?;
    Ok(())
}
