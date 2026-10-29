//! Log axes: how the running time of three sorting algorithms grows with
//! the input, power laws becoming straight lines on log-log axes.

use pltrs::axes::{GridOptions, TickLevel};
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let n = logspace(1.0, 7.0, 13, 10.0);
    let seconds = |f: fn(f64) -> f64| n.iter().map(|&n| f(n)).collect::<Vec<_>>();
    let algorithms = [
        (
            "insertion sort, O(n²)",
            seconds(|n| 2.5e-9 * n * n),
            MarkerSymbol::Circle,
        ),
        (
            "merge sort, O(n log n)",
            seconds(|n| 4e-8 * n * n.log2()),
            MarkerSymbol::Square,
        ),
        (
            "radix sort, O(n)",
            seconds(|n| 1.2e-7 * n),
            MarkerSymbol::TriangleUp,
        ),
    ];

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    for (label, time, marker) in algorithms {
        ax.loglog(&n, time)?.set_marker(marker).set_label(label);
    }
    let dotted = LineProps::default()
        .color("0.5")
        .linestyle(LineStyle::Dotted)
        .linewidth(1.0);
    ax.axhline(1.0, dotted);
    ax.text(12.0, 1.6, "one second").set_color("0.4");
    let minor = GridOptions {
        which: TickLevel::Minor,
        linewidth: Some(0.4),
        alpha: Some(0.5),
        ..GridOptions::default()
    };
    ax.grid_with(true, minor);
    ax.set_title("Running time of sorting algorithms");
    ax.set_xlabel("number of items").set_ylabel("time (s)");
    ax.set_ylim(1e-7, 1e5)?;
    ax.legend(LegendOptions::default())?;

    fig.savefig("log_axes.png")?;
    Ok(())
}
