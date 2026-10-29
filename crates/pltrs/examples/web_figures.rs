//! Two figures in a web browser (the `WebAgg` backend): the server's root
//! page shows both, and `/1` and `/2` show one each.
//!
//! ```text
//! cargo run -p pltrs --example web_figures                  # opens a browser
//! cargo run -p pltrs --example web_figures -- --no-browser  # prints the address
//! ```
//!
//! The figures and the page take the look the `PLTRS_LOOK`,
//! `PLTRS_FIGURE_LOOK` and `PLTRS_CHROME_LOOK` environment variables choose
//! (see `pltrs::look`): `PLTRS_FIGURE_LOOK=modern-dark` shows dark figures on
//! a dark page, `PLTRS_LOOK=matplotlib` matplotlib's own page. The server
//! listens where the `webagg.*` rcParams say (port 8988 of 127.0.0.1, or
//! a free port nearby) until `q` has closed both figures.

use std::error::Error;

use pltrs::prelude::*;
use pltrs::web::{WebOptions, serve};

/// Damped oscillations of three phases, labelled and with a legend.
fn oscillations() -> Result<Figure, Box<dyn Error>> {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let t = linspace(0.0, 10.0, 400);
    for (phase, label) in [(0.0, "phase 0"), (1.0, "phase 1"), (2.0, "phase 2")] {
        let y: Vec<f64> = t
            .iter()
            .map(|t| (t + phase).sin() * (-t / 6.0).exp())
            .collect();
        ax.plot_with(&t, y, LineProps::default().label(label))?;
    }
    ax.set_title("Damped oscillations");
    ax.set_xlabel("time (s)").set_ylabel("amplitude");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// A wave interference pattern with a colorbar.
fn interference() -> Result<Figure, Box<dyn Error>> {
    let fig = Figure::with_size(5.0, 4.0, 100.0);
    let ax = fig.subplot();
    let z = Grid::from_fn(120, 120, |i, j| {
        let (x, y) = (j as f64 / 12.0 - 5.0, i as f64 / 12.0 - 5.0);
        (x.hypot(y) * 2.0).sin() + (x * 1.5).cos() * (y * 0.7).sin()
    });
    let image = ax.imshow(z, ImshowOptions::default())?;
    fig.colorbar(&image, ColorbarOptions::default())?;
    ax.set_title("Interference");
    Ok(fig)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut options = WebOptions::from_rc()?;
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("--no-browser") => options.open_in_browser = false,
        Some(_) => return Err("usage: web_figures [--no-browser]".into()),
    }
    Ok(serve(&[oscillations()?, interference()?], options)?)
}
