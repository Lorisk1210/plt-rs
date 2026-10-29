//! Time series with dates: a year of daily visits to a website, a weekly
//! rhythm and a seasonal trend, with its 7-day mean and a campaign marked.

use std::f64::consts::PI;

use plt::dates::{AutoDateLocator, ConciseDateFormatter, DateRange, DateTime, TimeDelta};
use plt::layout::LayoutEngine;
use plt::offsetbox::AnchorLoc;
use plt::prelude::*;
use plt_examples::Samples;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let first = DateTime::new(2024, 1, 1, 0, 0, 0, 0)?;
    let days: Vec<DateTime> = DateRange::new(
        first,
        DateTime::new(2025, 1, 1, 0, 0, 0, 0)?,
        TimeDelta::from_days(1)?,
    )?
    .collect();
    let noise = Samples::new(5).normal(days.len(), 0.0, 260.0);
    let visits: Vec<f64> = (0..days.len())
        .zip(noise)
        .map(|(day, noise)| {
            let t = day as f64;
            // 2024 began on a Monday: weekends are quiet.
            let weekly = if day % 7 >= 5 { -900.0 } else { 250.0 };
            let seasonal = 4200.0 + 900.0 * (2.0 * PI * (t - 20.0) / 366.0).cos() + 2.5 * t;
            let campaign = if (244..274).contains(&day) {
                1400.0 * (PI * (t - 244.0) / 30.0).sin()
            } else {
                0.0
            };
            seasonal + weekly + campaign + noise
        })
        .collect();
    let mean: Vec<f64> = visits
        .windows(7)
        .map(|week| week.iter().sum::<f64>() / 7.0)
        .collect();

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let daily = LineProps::default()
        .color("0.75")
        .linewidth(0.8)
        .label("daily");
    ax.plot_with(&days[..], &visits, daily)?;
    let weekly = LineProps::default().linewidth(2.0).label("7-day mean");
    ax.plot_with(&days[3..days.len() - 3], &mean, weekly)?;
    let campaign = ax.axvspan(
        DateTime::new(2024, 9, 1, 0, 0, 0, 0)?,
        DateTime::new(2024, 10, 1, 0, 0, 0, 0)?,
    );
    campaign
        .set_color("C1")
        .set_alpha(Some(0.12))
        .set_label("campaign");
    ax.xaxis()
        .set_major_locator(AutoDateLocator::new())
        .set_major_formatter(ConciseDateFormatter::new());
    ax.set_title("Daily visits to the website");
    ax.set_ylabel("visits");
    ax.legend(
        LegendOptions::default()
            .loc(AnchorLoc::UpperCenter)
            .ncols(3),
    )?;

    fig.savefig("time_series.png")?;
    Ok(())
}
