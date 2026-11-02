//! The pltrs (Rust) side of a workload: its figure built and saved in this
//! process, under the configuration the scripts get in matplotlib and
//! pltrs (Python).

use std::error::Error;
use std::time::Instant;

use pltrs::animation::HtmlOptions;
use pltrs::figure::{BboxInches, Figure, SaveOptions};
use pltrs::look::{self, Appearance};
use pltrs_render::Metadata;

use super::workloads::{Built, DPI, Output, Workload};

/// Puts this thread in the configuration a matplotlib user starts from,
/// which pltrs (Python) also runs in here (the benchmark gives it
/// `PLTRS_LOOK=matplotlib`): matplotlib's look and default rcParams.
pub(super) fn user_configuration() {
    look::set_appearance(Appearance::MATPLOTLIB);
    pltrs::rc::restore_startup();
}

/// How documents are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Documents {
    /// As a program writes them by default, and as matplotlib's scripts
    /// do: with the default metadata (creator, producer and date in PDF,
    /// PostScript and SVG, the software in PNG) and random SVG ids.
    AsUsers,
    /// Without metadata and with SVG ids of a fixed salt, so that the
    /// documents of two builds of pltrs can be compared byte for byte.
    Reproducible,
}

/// A run of a workload: how long building and saving the figure took, and
/// the document saved.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Timed {
    /// The seconds from making the figure to the end of the save.
    pub(super) seconds: f64,
    /// What was saved (empty for a figure only drawn without output).
    pub(super) document: Vec<u8>,
}

/// Builds the figure of `workload` from `built` and finishes it as
/// `output` asks. The clock runs from making the figure until the
/// document is in memory, as it does around a script's `draw()` and
/// `savefig`; setting the workload's rcParams comes before it, dropping
/// the figure after it.
pub(super) fn render(
    workload: &Workload,
    built: &Built,
    output: Output,
    documents: Documents,
) -> Result<Timed, Box<dyn Error>> {
    pltrs::rc::context(|| {
        if documents == Documents::Reproducible {
            pltrs::rc::set("svg.hashsalt", "plt-rs")?;
        }
        for (key, value) in workload.rc {
            pltrs::rc::set(key, value)?;
        }
        let (width, height) = workload.figsize;
        let start = Instant::now();
        let fig = Figure::with_size(width, height, DPI);
        let mut animation = None;
        let document = match built {
            Built::Figure(build) => {
                build(&fig)?;
                finish(&fig, output, documents)?
            }
            Built::Animation(build) => animation
                .insert(build(&fig)?)
                .to_jshtml(HtmlOptions::default())?
                .into_bytes(),
        };
        let seconds = start.elapsed().as_secs_f64();
        drop((animation, fig));
        Ok(Timed { seconds, document })
    })
}

/// Saves `fig` to memory as `output` asks, or draws it without output and
/// returns no document.
fn finish(fig: &Figure, output: Output, documents: Documents) -> Result<Vec<u8>, Box<dyn Error>> {
    match output {
        Output::Save { format, tight } => {
            let options = SaveOptions {
                dpi: Some(DPI),
                bbox_inches: tight.then_some(BboxInches::Tight),
                metadata: (documents == Documents::Reproducible).then(Metadata::none),
                ..SaveOptions::default()
            };
            Ok(fig.to_format(format, options)?)
        }
        Output::LayoutOnly => {
            fig.draw_without_rendering()?;
            Ok(Vec::new())
        }
        Output::Frames => Err("only an animation renders frames".into()),
    }
}

/// A timed run of `workload` as the benchmark makes it: the text pltrs
/// laid out in earlier runs forgotten first, as matplotlib keeps none from
/// one figure to the next.
pub(super) fn timed_run(workload: &Workload, built: &Built) -> Result<Timed, Box<dyn Error>> {
    pltrs::text::clear_text_caches();
    render(workload, built, workload.output, Documents::AsUsers)
}
