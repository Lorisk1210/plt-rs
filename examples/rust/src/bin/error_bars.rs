//! Error bars: the activity of an enzyme and of a heat-stable variant
//! across temperatures, each point the mean of repeated measurements with
//! its standard deviation.

use pltrs::layout::LayoutEngine;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;

/// Bell-shaped activity with a maximum of `peak` at `optimum` °C.
fn activity(temperature: &[f64], peak: f64, optimum: f64, width: f64) -> Vec<f64> {
    temperature
        .iter()
        .map(|t| peak * (-((t - optimum) / width).powi(2)).exp())
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temperature = arange(20.0, 85.0, 5.0);
    let wild = activity(&temperature, 100.0, 42.0, 14.0);
    let stable = activity(&temperature, 92.0, 58.0, 17.0);

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    for (values, fmt, error, label) in [
        (&wild, "o-", (4.0, 0.06), "wild type"),
        (&stable, "s--", (3.0, 0.08), "heat-stable variant"),
    ] {
        let options = ErrorbarOptions {
            yerr: Some(
                values
                    .iter()
                    .map(|v| error.0 + error.1 * v)
                    .collect::<Vec<_>>()
                    .into(),
            ),
            fmt: fmt.parse()?,
            capsize: Some(3.0),
            line: LineProps::default().label(label),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&temperature, values, options)?;
    }
    ax.set_title("Enzyme activity against temperature");
    ax.set_xlabel("temperature (°C)")
        .set_ylabel("activity (% of maximum)");
    ax.set_ylim(0.0, 115.0)?;
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperRight))?;

    fig.savefig("error_bars.png")?;
    Ok(())
}
