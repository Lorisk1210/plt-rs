//! Scatter plot with a colormap and a colorbar: the weight, fuel use and
//! power of 150 cars, the marker size giving the engine size.

use pltrs::collections::EdgeColors;
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;
use pltrs_examples::Samples;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = Samples::new(7);
    let weight = rng.normal(150, 1.45, 0.3); // tonnes
    let power: Vec<f64> = weight
        .iter()
        .zip(rng.normal(150, 0.0, 12.0))
        .map(|(w, noise)| 40.0 + 85.0 * w + noise)
        .collect(); // kW
    let consumption: Vec<f64> = weight
        .iter()
        .zip(&power)
        .zip(rng.normal(150, 0.0, 0.45))
        .map(|((w, p), noise)| 2.2 + 3.1 * w + 0.012 * p + noise)
        .collect(); // l/100 km
    let engine: Vec<f64> = weight
        .iter()
        .zip(rng.normal(150, 0.0, 0.15))
        .map(|(w, noise)| 0.4 + 0.9 * w + noise)
        .collect(); // litres

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let options = ScatterOptions {
        sizes: Some(engine.iter().map(|e| 60.0 * e * e).collect()),
        colors: MappedColors::new(&power)
            .cmap(colormaps::get("viridis")?)
            .into(),
        alpha: 0.85.into(),
        edgecolors: Some(EdgeColors::Colors(vec!["white".into()])),
        linewidths: Some(vec![0.6]),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with(&weight, &consumption, options)?;
    let colorbar = ColorbarOptions {
        label: "power (kW)".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&points, colorbar)?;
    ax.set_title("Heavier cars use more fuel");
    ax.set_xlabel("weight (t)")
        .set_ylabel("fuel use (l/100 km)");

    fig.savefig("scatter_colorbar.png")?;
    Ok(())
}
