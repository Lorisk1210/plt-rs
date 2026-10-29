//! The pltrs (Rust) side of a workload: its figure built and saved in this
//! process, under the configuration the scripts get in matplotlib and
//! pltrs (Python).

use std::error::Error;
use std::time::Instant;

use pltrs::figure::{BboxInches, Figure, SaveOptions};
use pltrs::look::{self, Appearance};
use pltrs_render::Metadata;

use super::stats::{Samples, Sampling};
use super::workloads::{Build, DPI, Output, Workload};

/// Puts this thread in the configuration a matplotlib user starts from,
/// which pltrs (Python) also selects when it is imported: matplotlib's look and
/// default rcParams.
pub(super) fn user_configuration() {
    look::set_appearance(Appearance::MATPLOTLIB);
    pltrs::rc::restore_startup();
}

/// Builds the figure of `workload` with `build` and finishes it as
/// `output` asks: the document saved, or nothing for a figure only laid
/// out. Documents carry no metadata and SVG ids a fixed salt, so that two
/// builds of pltrs can be compared byte for byte.
pub(super) fn render(
    workload: &Workload,
    build: &Build,
    output: Output,
) -> Result<Vec<u8>, Box<dyn Error>> {
    pltrs::rc::context(|| {
        pltrs::rc::set("svg.hashsalt", "plt-rs")?;
        for (key, value) in workload.rc {
            pltrs::rc::set(key, value)?;
        }
        let (width, height) = workload.figsize;
        let fig = Figure::with_size(width, height, DPI);
        build(&fig)?;
        finish(&fig, output)
    })
}

/// Saves `fig` to memory as `output` asks, or only lays it out and
/// returns no document.
fn finish(fig: &Figure, output: Output) -> Result<Vec<u8>, Box<dyn Error>> {
    match output {
        Output::Save { format, tight } => {
            let options = SaveOptions {
                dpi: Some(DPI),
                bbox_inches: tight.then_some(BboxInches::Tight),
                metadata: Some(Metadata::none()),
                ..SaveOptions::default()
            };
            Ok(fig.to_format(format, options)?)
        }
        Output::LayoutOnly => {
            fig.layout();
            Ok(Vec::new())
        }
    }
}

/// Times `workload` in this process, its data made by `build`: one run to
/// warm up, then as many runs as `sampling` asks. Returns the times and
/// the last document saved.
pub(super) fn time_warm(
    workload: &Workload,
    build: &Build,
    sampling: Sampling,
) -> Result<(Samples, Vec<u8>), Box<dyn Error>> {
    let mut document = render(workload, build, workload.output)?;
    let mut times = Vec::new();
    while sampling.wants_more(times.len(), times.iter().sum()) {
        let start = Instant::now();
        document = render(workload, build, workload.output)?;
        times.push(start.elapsed().as_secs_f64());
    }
    let samples = Samples::new(times).ok_or("the sampling asked for no runs")?;
    Ok((samples, document))
}
