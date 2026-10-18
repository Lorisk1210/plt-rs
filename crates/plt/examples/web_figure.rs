//! An interactive figure in a web browser (the `WebAgg` backend): a wave a
//! timer moves along, with matplotlib's toolbar to pan, zoom and download.
//!
//! ```text
//! cargo run -p plt --example web_figure                  # opens a browser
//! cargo run -p plt --example web_figure -- --no-browser  # prints the address
//! ```
//!
//! The server listens on port 8988 of 127.0.0.1 (or a free port nearby,
//! as the `webagg.*` rcParams say) until `q` closes the figure.

use std::error::Error;
use std::time::Duration;

use plt::figure::Figure;
use plt::numeric::linspace;
use plt::web::{WebOptions, serve};

/// A damped wave shifted by `phase`.
fn wave(x: &[f64], phase: f64) -> Vec<f64> {
    x.iter()
        .map(|x| (x - phase).sin() * (-x / 10.0).exp())
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut options = WebOptions::from_rc()?;
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("--no-browser") => options.open_in_browser = false,
        Some(_) => return Err("usage: web_figure [--no-browser]".into()),
    }
    let figure = Figure::with_size(6.4, 4.0, 100.0);
    let ax = figure.subplot();
    let x = linspace(0.0, 20.0, 400);
    let line = ax.plot(x.clone(), wave(&x, 0.0))?;
    ax.set_title("A travelling wave");
    ax.set_xlabel("x").set_ylabel("amplitude");
    ax.grid(true);
    let mut phase = 0.0;
    figure.add_timer(Duration::from_millis(100), move || {
        phase += 0.1;
        line.set_data(&x, &wave(&x, phase));
        true
    });
    Ok(serve(&[figure], &options)?)
}
