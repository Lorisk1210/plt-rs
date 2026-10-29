//! Stacked area chart: where a country's electricity came from, year by
//! year, the top edge giving the total.

use plt::axes::StackplotOptions;
use plt::collections::CollectionProps;
use plt::layout::LayoutEngine;
use plt::offsetbox::AnchorLoc;
use plt::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let years: Vec<f64> = (2000..=2024).map(f64::from).collect();
    let trend = |f: fn(f64) -> f64| years.iter().map(|year| f(year - 2000.0)).collect();
    let sources: [(&str, Vec<f64>); 6] = [
        ("coal", trend(|t| 210.0 - 7.5 * t + 0.08 * t * t)),
        ("gas", trend(|t| 90.0 + 3.2 * t - 0.06 * t * t)),
        ("nuclear", trend(|t| 160.0 - 1.5 * t)),
        ("hydro", trend(|t| 60.0 + 4.0 * (t / 2.2).sin())),
        ("wind", trend(|t| 0.35 * t * t)),
        ("solar", trend(|t| 0.012 * t * t * t)),
    ];

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let options = StackplotOptions {
        props: CollectionProps::default().alpha(0.9),
        ..StackplotOptions::default()
    };
    let layers = sources
        .iter()
        .map(|(_, layer)| layer.clone())
        .collect::<Vec<_>>();
    for (area, (label, _)) in ax.stackplot(&years, &layers, options)?.iter().zip(&sources) {
        area.set_label(*label);
    }
    ax.set_title("Electricity generation by source");
    ax.set_ylabel("TWh");
    ax.set_xlim(2000.0, 2024.0)?;
    let legend = LegendOptions::default()
        .loc(AnchorLoc::CenterLeft)
        .bbox_to_anchor((1.0, 0.5))
        .reverse(true);
    ax.legend(legend)?;

    fig.savefig("stacked_area.png")?;
    Ok(())
}
