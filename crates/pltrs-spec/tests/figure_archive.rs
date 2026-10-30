//! Every translated figure case, archived before it is drawn and read
//! back, draws the same pixels as the figure it was archived from (what
//! pickling a figure promises). Cases holding what cannot be archived
//! (Rust closures given without an origin, the program's own artist
//! types) are listed with the reason, as long as the reason is that.

use std::error::Error;

use pltrs::figure::{Figure, SaveOptions};
use pltrs_core::archive::{ArchiveError, NoProgram};
use pltrs_render::RgbaImage;
use pltrs_spec::figures::{self, Capture, FigureCase};

/// What became of one case.
enum Outcome {
    /// The figure read back draws the same pixels.
    Same,
    /// The figure holds something that cannot be archived.
    Unsupported(String),
    /// The figure read back draws other pixels (the number differing).
    Differs(usize),
}

/// The pixels `fig` draws for `case`.
fn pixels(case: &FigureCase, fig: &Figure) -> Result<RgbaImage, Box<dyn Error>> {
    let options = SaveOptions {
        dpi: Some(case.dpi),
        ..SaveOptions::default()
    };
    Ok(fig.render_raster_partial(options)?.0.to_image())
}

/// Archives the case's figure before drawing it, reads it back and
/// compares what both draw.
fn round_trip(case: &FigureCase) -> Result<Outcome, Box<dyn Error>> {
    case.with_figure(|fig| {
        let archived = fig.to_archive(&NoProgram);
        let original = pixels(case, fig)?;
        let (bytes, externals) = match archived {
            Ok(archive) => archive,
            Err(ArchiveError::Unsupported(what)) => return Ok(Outcome::Unsupported(what)),
            Err(err) => return Err(err.into()),
        };
        let back = Figure::from_archive(&bytes, &externals, &NoProgram)?;
        let restored = pixels(case, &back)?;
        let differing = original
            .data()
            .chunks(4)
            .zip(restored.data().chunks(4))
            .filter(|(a, b)| a != b)
            .count();
        let same_size = original.data().len() == restored.data().len();
        Ok(if same_size && differing == 0 {
            Outcome::Same
        } else {
            Outcome::Differs(differing.max(1))
        })
    })
}

#[test]
fn archived_figure_cases_draw_the_same() {
    let mut failures = Vec::new();
    let (mut same, mut unsupported) = (0, 0);
    for case in figures::translated_cases() {
        if case.skip_reason().is_some() || case.capture == Capture::Canvas {
            continue;
        }
        match round_trip(&case) {
            Ok(Outcome::Same) => same += 1,
            Ok(Outcome::Unsupported(what)) => {
                unsupported += 1;
                println!("{:<44} not archived: {what}", case.name);
            }
            Ok(Outcome::Differs(count)) => {
                failures.push(format!("{}: {count} pixels differ", case.name));
            }
            Err(err) => failures.push(format!("{}: {err}", case.name)),
        }
    }
    println!("{same} cases drew the same once read back, {unsupported} not archived");
    assert!(
        failures.is_empty(),
        "archive round trips failed:\n{}",
        failures.join("\n")
    );
}
