//! Arranging axes: mosaics, twin axes and insets.

use plt::figure::{Mosaic, MosaicOptions};
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::prelude::*;

use super::data::Samples;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry {
        name: "mosaic",
        caption: "subplot_mosaic with constrained layout",
        build: mosaic,
    },
    Entry {
        name: "twin_axes",
        caption: "Two y axes sharing an x axis (twinx)",
        build: twin_axes,
    },
    Entry {
        name: "inset",
        caption: "An inset zooming into a detail",
        build: inset,
    },
];

/// Four panels of different sizes, laid out by name.
fn mosaic() -> Built {
    let fig = Figure::with_size(7.0, 5.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axes = fig.subplot_mosaic(&"AAB;CDB".parse::<Mosaic>()?, MosaicOptions::default())?;
    let mut samples = Samples::new(3);
    for (name, ax) in axes.iter() {
        let n = 20 + name.len() * 10;
        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let mut level = 0.0;
        let walk: Vec<f64> = (0..n)
            .map(|_| {
                level += samples.normal(0.0, 1.0);
                level
            })
            .collect();
        ax.plot(x, walk)?;
        ax.set_title(format!("panel {name}"));
    }
    Ok(fig)
}

/// Temperature and rainfall over a year on their own y axes.
fn twin_axes() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let months: Vec<f64> = (1..=12).map(f64::from).collect();
    let temperature: Vec<f64> = months
        .iter()
        .map(|m| 11.0 - 10.0 * (2.0 * std::f64::consts::PI * (m - 1.0) / 12.0).cos())
        .collect();
    let rain: Vec<f64> = months
        .iter()
        .map(|m| 60.0 + 25.0 * (2.0 * std::f64::consts::PI * m / 12.0).sin())
        .collect();
    ax.plot_with(
        &months,
        temperature,
        LineProps::fmt("C0o-")?.label("temperature"),
    )?;
    ax.set_xlabel("month").set_ylabel("temperature (°C)");
    let right = ax.twinx()?;
    right.plot_with(&months, rain, LineProps::fmt("C1s--")?.label("rain"))?;
    right.set_ylabel("rain (mm)");
    ax.set_title("A year of weather");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// A signal with a fine structure, magnified in an inset.
fn inset() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 2000);
    let y: Vec<f64> = x
        .iter()
        .map(|x| x.sin() + 0.05 * (40.0 * x).sin())
        .collect();
    ax.plot(x.clone(), y.clone())?;
    ax.set_ylim(-1.5, 2.5)?;
    let zoom = ax.inset_axes([0.55, 0.6, 0.4, 0.35]);
    zoom.plot(x, y)?;
    zoom.set_xlim(1.2, 1.9)?.set_ylim(0.85, 1.08)?;
    ax.set_title("Inset axes");
    Ok(fig)
}
