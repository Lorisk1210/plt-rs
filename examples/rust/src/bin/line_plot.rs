//! Line plot with a legend: the step response of a spring-mass system for
//! three damping ratios.

use plt::layout::LayoutEngine;
use plt::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let t = linspace(0.0, 12.0, 400);
    let omega = 1.5; // natural frequency (rad/s)

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    for zeta in [0.15_f64, 0.4, 0.7] {
        let damped = omega * (1.0 - zeta * zeta).sqrt();
        let response = t.iter().map(|&t| {
            1.0 - (-zeta * omega * t).exp()
                * ((damped * t).cos() + zeta / (1.0 - zeta * zeta).sqrt() * (damped * t).sin())
        });
        ax.plot_with(
            &t,
            response,
            LineProps::default().label(format!("ζ = {zeta}")),
        )?;
    }
    ax.axhline(
        1.0,
        LineProps::default()
            .color("0.5")
            .linestyle(LineStyle::Dashed)
            .linewidth(1.0),
    );
    ax.set_title("Step response of a damped oscillator");
    ax.set_xlabel("time (s)").set_ylabel("displacement");
    ax.set_xlim(0.0, 12.0)?;
    ax.legend(LegendOptions::default().title("damping ratio"))?;

    fig.savefig("line_plot.png")?;
    Ok(())
}
