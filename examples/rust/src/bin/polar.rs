//! Polar axes: a radar chart scoring three laptops on six criteria, each a
//! closed, shaded outline.

use std::f64::consts::{FRAC_PI_2, TAU};

use plt::layout::LayoutEngine;
use plt::offsetbox::AnchorLoc;
use plt::polar::ThetaDirection;
use plt::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let criteria = ["speed", "battery", "screen", "weight", "price", "keyboard"];
    let scores = [
        ("Aero 14", [4.5, 3.0, 4.0, 4.5, 2.0, 3.5]),
        ("Bolt 16", [5.0, 2.0, 4.5, 2.0, 2.5, 4.0]),
        ("Civic 13", [3.0, 4.5, 3.0, 4.0, 4.5, 3.0]),
    ];
    let step = TAU / criteria.len() as f64;
    let angles: Vec<f64> = (0..criteria.len()).map(|i| i as f64 * step).collect();

    let fig = Figure::with_size(6.4, 5.2, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.polar_subplot();
    ax.set_theta_offset(FRAC_PI_2)
        .set_theta_direction(ThetaDirection::Clockwise);
    for (name, values) in scores {
        // The outline closes back on the first criterion.
        let theta: Vec<f64> = angles.iter().chain(&angles[..1]).copied().collect();
        let r: Vec<f64> = values.iter().chain(&values[..1]).copied().collect();
        ax.plot_with(&theta, &r, LineProps::default().linewidth(2.0).label(name))?;
        ax.fill(&theta, &r, PatchProps::default().alpha(0.15))?;
    }
    ax.set_xticks(&angles).set_xticklabels(criteria)?;
    ax.set_ylim(0.0, 5.0)?;
    ax.set_yticks([1.0, 2.0, 3.0, 4.0, 5.0]);
    ax.set_rlabel_position(90.0);
    ax.set_title("Laptop review scores");
    let legend = LegendOptions::default()
        .loc(AnchorLoc::UpperRight)
        .bbox_to_anchor((1.25, 1.08));
    ax.legend(legend)?;

    fig.savefig("polar.png")?;
    Ok(())
}
