//! Animation saved as a GIF: a wave packet travelling along a string, its
//! envelope drawn behind it.

use std::time::Duration;

use pltrs::animation::{AnimationOptions, Frames, FuncAnimation};
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;

/// The envelope of the packet centred at `centre`, over `x`.
fn envelope(x: &[f64], centre: f64) -> Vec<f64> {
    x.iter()
        .map(|x| (-(x - centre).powi(2) / 1.5).exp())
        .collect()
}

/// The displacement of the packet centred at `centre`, over `x`.
fn packet(x: &[f64], centre: f64) -> Vec<f64> {
    let carrier = x.iter().map(|x| (4.0 * (x - centre)).cos());
    envelope(x, centre)
        .iter()
        .zip(carrier)
        .map(|(e, c)| e * c)
        .collect()
}

/// `values` with their signs flipped.
fn negated(values: Vec<f64>) -> Vec<f64> {
    values.into_iter().map(|v| -v).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x = linspace(0.0, 10.0, 400);
    let frames = 30;

    let fig = Figure::with_size(6.0, 3.2, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let guide = LineProps::default()
        .color("0.75")
        .linewidth(1.0)
        .linestyle(LineStyle::Dashed);
    let upper = ax.plot_with(&x, envelope(&x, 2.0), guide.clone())?;
    let lower = ax.plot_with(&x, negated(envelope(&x, 2.0)), guide)?;
    let wave = ax.plot_with(&x, packet(&x, 2.0), LineProps::default().linewidth(2.0))?;
    ax.set_xlim(0.0, 10.0)?.set_ylim(-1.2, 1.2)?;
    ax.set_title("A travelling wave packet");
    ax.set_xlabel("position");

    let options = AnimationOptions {
        interval: Duration::from_millis(80),
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(&fig, Frames::Count(frames), options, move |frame| {
        let centre = 2.0 + 6.0 * frame as f64 / (frames - 1) as f64;
        upper.set_ydata(envelope(&x, centre));
        lower.set_ydata(negated(envelope(&x, centre)));
        wave.set_ydata(packet(&x, centre));
    });
    animation.save_gif("animation.gif")?;
    Ok(())
}
