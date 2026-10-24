//! Bar charts: grouped, stacked and horizontal bars, with value labels.

use plt::axes::{BarLabelOptions, BarOptions, GroupedBarData, GroupedBarOptions};
use plt::legend::LegendOptions;
use plt::prelude::*;

use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry {
        name: "grouped_bars",
        caption: "Grouped bars with value labels",
        build: grouped_bars,
    },
    Entry {
        name: "stacked_bars",
        caption: "Stacked bars",
        build: stacked_bars,
    },
    Entry {
        name: "horizontal_bars",
        caption: "Horizontal bars, sorted",
        build: horizontal_bars,
    },
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
            base: morning.to_vec(),
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
