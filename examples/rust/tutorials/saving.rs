//! Saving figures in every format matplotlib writes (`savefig` and its
//! keywords, `PdfPages`, rendering into memory) in pltrs (Rust).
//!
//! ```text
//! cargo run -p pltrs-examples --example saving   # writes saving.* here
//! ```
//!
//! The format follows the file name, as in matplotlib. PGF, the LaTeX
//! picture format, is written as a `.pgf` file; turning it into a PDF
//! (`savefig("fig.pdf", backend="pgf")`) needs LaTeX, and so do figures
//! with `text.usetex`, which this program leaves out. PostScript has no
//! transparency: saving the translucent band as PS and EPS draws it
//! opaque and warns so on stderr, once per file, as matplotlib does.
#![expect(clippy::print_stdout, reason = "the tutorial reports what it made")]

use pltrs::collections::CollectionProps;
use pltrs::figure::{BboxInches, Metadata, PadInches, PdfPages, SaveFormat, SaveOptions};
use pltrs::image::imread;
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// The file extensions of the formats saved here, raster then vector.
const FORMATS: [&str; 13] = [
    "png", "jpg", "tiff", "gif", "webp", "avif", "rgba", "svg", "svgz", "pdf", "ps", "eps", "pgf",
];

fn main() -> Outcome {
    let fig = figure()?;

    // `fig.savefig("saving.png")`, ... for every format.
    for extension in FORMATS {
        let path = format!("saving.{extension}");
        fig.savefig(&path)?;
        let size = std::fs::metadata(&path)?.len();
        println!("{path:<14} {size:>8} bytes");
    }

    // `savefig(path, dpi=200, bbox_inches="tight", pad_inches=0.05,
    // transparent=True, metadata={"Title": ...})`
    let options = SaveOptions {
        dpi: Some(200.0),
        bbox_inches: Some(BboxInches::Tight),
        pad_inches: Some(PadInches::Inches(0.05)),
        transparent: Some(true),
        metadata: Some(Metadata {
            title: Some("Damped oscillations".into()),
            author: Some("pltrs".into()),
            ..Metadata::default()
        }),
        ..SaveOptions::default()
    };
    fig.savefig_with("saving_tight.png", options)?;
    let tight = imread("saving_tight.png")?;
    println!(
        "saving_tight.png is {} × {} pixels",
        tight.shape().1,
        tight.shape().0
    );

    // `savefig(io.BytesIO(), format="svg")`: into memory.
    let svg = fig.to_svg()?;
    let jpeg = fig.to_format(
        "jpeg".parse::<SaveFormat>()?,
        SaveOptions {
            jpeg_quality: Some(60),
            ..SaveOptions::default()
        },
    )?;
    println!(
        "SVG in memory: {} characters; JPEG at quality 60: {} bytes",
        svg.len(),
        jpeg.len()
    );

    // `with PdfPages("saving_pages.pdf") as pdf: pdf.savefig(fig)`
    let mut pages = PdfPages::new()?;
    for damping in [0.1, 0.3, 0.9] {
        let page = Figure::with_size(4.0, 3.0, 100.0);
        let ax = page.subplot();
        let t = linspace(0.0, 10.0, 200);
        ax.plot(&t, t.iter().map(|t| (-damping * t).exp() * (3.0 * t).cos()))?;
        ax.set_title(format!("damping {damping}"));
        pages.savefig(&page)?;
    }
    pages.metadata_mut().title = Some("Three damping ratios".into());
    let count = pages.page_count();
    pages.save("saving_pages.pdf")?;
    println!("saving_pages.pdf has {count} pages");
    Ok(())
}

/// The figure every format is saved from: lines, a filled area, text
/// with math, and a legend.
fn figure() -> Result<Figure, Box<dyn std::error::Error>> {
    let fig = Figure::with_size(5.0, 3.2, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let t = linspace(0.0, 10.0, 300);
    for (damping, color) in [(0.2, "C0"), (0.5, "C1")] {
        let y: Vec<f64> = t
            .iter()
            .map(|t| (-damping * t).exp() * (3.0 * t).cos())
            .collect();
        ax.plot_with(
            &t,
            &y,
            LineProps::default()
                .color(color)
                .label(format!("ζ = {damping}")),
        )?;
    }
    let envelope: Vec<f64> = t.iter().map(|t| (-0.2 * t).exp()).collect();
    let below: Vec<f64> = envelope.iter().map(|e| -e).collect();
    let band = FillBetweenOptions {
        props: CollectionProps::default().alpha(0.15),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&t, &below, &envelope, band)?;
    ax.text(5.0, 0.6, r"$e^{-\zeta t}\cos(3t)$");
    ax.set_xlabel("time (s)");
    ax.set_title("Damped oscillations");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}
