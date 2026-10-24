//! Bar charts: grouped, stacked, horizontal and broken bars, with value
//! labels.

use plt::axes::{BandAlign, BarLabelOptions, BarOptions, GroupedBarData, GroupedBarOptions};
use plt::legend::LegendOptions;
use plt::prelude::*;

use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "grouped_bars",
        "Grouped bars with value labels",
        grouped_bars,
    ),
    Entry::figure("stacked_bars", "Stacked bars", stacked_bars),
    Entry::figure(
        "horizontal_bars",
        "Horizontal bars, sorted",
        horizontal_bars,
    ),
    Entry::figure(
        "broken_barh",
        "A schedule drawn with broken_barh",
        broken_barh,
    ),
];

/// Three series side by side for each category, labelled with their
/// values.
fn grouped_bars() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let heights = GroupedBarData::Named(vec![
        ("2023".into(), vec![18.0, 24.0, 31.0, 12.0]),
        ("2024".into(), vec![21.0, 22.0, 35.0, 17.0]),
        ("2025".into(), vec![26.0, 25.0, 33.0, 23.0]),
    ]);
    let options = GroupedBarOptions {
        tick_labels: ["north", "south", "east", "west"]
            .map(String::from)
            .to_vec(),
        ..GroupedBarOptions::default()
    };
    for bars in ax.grouped_bar(heights, options)? {
        ax.bar_label(&bars, BarLabelOptions::default())?;
    }
    ax.set_title("Sales by region");
    ax.set_ylabel("units (thousands)");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// Two series stacked on each other.
fn stacked_bars() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri"];
    let morning = [5.0, 7.0, 6.0, 8.0, 4.0];
    let evening = [3.0, 2.0, 5.0, 4.0, 6.0];
    let labelled = |label: &str| BarOptions {
        label: Some(label.into()),
        ..BarOptions::default()
    };
    ax.bar(days.to_vec(), morning, labelled("morning"))?;
    ax.bar(
        days.to_vec(),
        evening,
        BarOptions {
            base: morning.to_vec().into(),
            ..labelled("evening")
        },
    )?;
    ax.set_title("Visits per day");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// Horizontal bars ordered by length.
fn horizontal_bars() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let mut languages: [(&str, f64); 5] = [
        ("Rust", 83.0),
        ("Python", 67.0),
        ("Go", 59.0),
        ("TypeScript", 72.0),
        ("Kotlin", 55.0),
    ];
    languages.sort_by(|a, b| a.1.total_cmp(&b.1));
    let names: Vec<&str> = languages.iter().map(|(name, _)| *name).collect();
    let scores: Vec<f64> = languages.iter().map(|(_, score)| *score).collect();
    let bars = ax.barh(names, scores, BarOptions::default())?;
    ax.bar_label(&bars, BarLabelOptions::default())?;
    ax.set_title("Admiration (%)");
    Ok(fig)
}

/// Busy intervals of four machines: each row is one `broken_barh` of
/// `(start, duration)` pairs, placed on a categorical y axis.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every gallery builder has the same signature"
)]
fn broken_barh() -> Built {
    let fig = Figure::with_size(6.4, 3.6, 100.0);
    let ax = fig.subplot();
    let schedule: [(&str, &[(f64, f64)]); 4] = [
        ("build", &[(0.0, 2.5), (6.0, 1.5)]),
        ("test", &[(2.5, 3.0), (7.5, 2.0)]),
        ("lint", &[(0.0, 1.0), (4.0, 0.5), (9.0, 0.5)]),
        ("deploy", &[(9.5, 1.5)]),
    ];
    for (row, (machine, busy)) in schedule.iter().enumerate() {
        let band = ax.broken_barh(busy, (*machine, 0.6), BandAlign::Center);
        band.set_facecolor(format!("C{row}"));
    }
    ax.yaxis().set_inverted(true);
    ax.set_xlabel("minutes since push");
    ax.set_title("A CI pipeline");
    Ok(fig)
}
