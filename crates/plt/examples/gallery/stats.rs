//! Statistics: histograms, box and violin plots, pies.

use plt::axes::{BoxplotOptions, HistPlotOptions, PieOptions, ViolinplotOptions};
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions, HistType};

use super::data::Samples;
use super::{Built, Entry};
use plt::ticker::LabelFormat;

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry {
        name: "histograms",
        caption: "Overlapping histograms (step and filled)",
        build: histograms,
    },
    Entry {
        name: "box_and_violin",
        caption: "Box plots next to violin plots",
        build: box_and_violin,
    },
    Entry {
        name: "pie",
        caption: "Pie chart with percentages",
        build: pie,
    },
];

/// Two normal samples, as filled and step histograms.
fn histograms() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let mut samples = Samples::new(7);
    let a = samples.normals(2000, 0.0, 1.0);
    let b = samples.normals(2000, 1.5, 0.7);
    let layout = HistOptions {
        bins: Some(Bins::Count(40)),
        density: true,
        ..HistOptions::default()
    };
    ax.hist(
        &[a],
        HistPlotOptions {
            layout: layout.clone(),
            props: PatchProps::default().alpha(0.6).label("N(0, 1)"),
            ..HistPlotOptions::default()
        },
    )?;
    ax.hist(
        &[b],
        HistPlotOptions {
            layout: HistOptions {
                histtype: HistType::Step,
                ..layout
            },
            props: PatchProps::default().linewidth(2.0).label("N(1.5, 0.7)"),
            ..HistPlotOptions::default()
        },
    )?;
    ax.set_title("Two samples");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// The same four samples as box plots and as violins.
fn box_and_violin() -> Built {
    let fig = Figure::with_size(8.0, 4.0, 100.0);
    let grid = fig.subplots(1, 2)?;
    let mut samples = Samples::new(11);
    let data: Vec<Vec<f64>> = (0..4)
        .map(|i| samples.normals(200, f64::from(i), 0.5 + 0.3 * f64::from(i)))
        .collect();
    grid[(0, 0)].boxplot(&data, BoxplotOptions::default())?;
    grid[(0, 0)].set_title("boxplot");
    grid[(0, 1)].violinplot(&data, ViolinplotOptions::default())?;
    grid[(0, 1)].set_title("violinplot");
    Ok(fig)
}

/// Shares of a whole, labelled with their percentages.
fn pie() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.subplot();
    let options = PieOptions {
        labels: ["Frogs", "Hogs", "Dogs", "Logs"].map(String::from).to_vec(),
        autopct: Some(LabelFormat::Template("%1.1f%%".into())),
        explode: vec![0.0, 0.1, 0.0, 0.0],
        startangle: 90.0,
        ..PieOptions::default()
    };
    ax.pie(&[15.0, 30.0, 45.0, 10.0], options)?;
    ax.set_title("Pets");
    Ok(fig)
}
