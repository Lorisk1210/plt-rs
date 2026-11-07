//! Archives: figures of the plotting target archived, as pickling a
//! figure does, then read back corrupted (bits flipped, bytes
//! overwritten, spans repeated or deleted, the end cut off). Reading
//! back may fail, but never panic or exhaust memory, and a figure read
//! back must draw, or fail to, without panicking, saved whole or in its
//! tight box (which measures every artist).

use pltrs::figure::{BboxInches, Figure, SaveOptions};
use pltrs_core::archive::{External, NoProgram};

use crate::harness::{Corruption, Gen, fuzz};
use crate::plotting::{self, Plan};
use crate::support::format;

/// How many plans the corpus is made of (those whose figure cannot be
/// made or archived are left out).
const PLANS: u64 = 24;

/// The most bytes a corruption confined to a window of an archive spans.
const WINDOW: usize = 4096;

/// A figure's archive: its bytes and the externals they refer to.
type Archived = (Vec<u8>, Vec<External>);

/// The seed corpus: the archived figures of the plotting target's plans
/// of the first seeds, every other one drawn before it is archived (a
/// drawn figure holds its layout and ticks).
fn corpus() -> Vec<Archived> {
    (0..PLANS)
        .filter_map(|seed| {
            let fig = plotting::build(&Plan::generate(&mut Gen::new(seed)))?;
            if seed % 2 == 0 {
                let _ = fig.to_png();
            }
            fig.to_archive(&NoProgram).ok()
        })
        .collect()
}

/// A corruption of `bytes`: of the whole (its header the likeliest part
/// hit) now and then, otherwise of a window anywhere in it, so that the
/// edits reach every part of a figure.
fn corruption(g: &mut Gen, bytes: &[u8]) -> Corruption {
    if g.chance(0.25) {
        return g.corrupt(bytes);
    }
    let offset = g.below(bytes.len());
    let end = (offset + WINDOW).min(bytes.len());
    let mut corruption = g.corrupt(&bytes[offset..end]);
    corruption.shift(offset);
    corruption
}

#[test]
fn corrupted_figure_archives() {
    let corpus = corpus();
    assert!(!corpus.is_empty(), "no figure of the corpus archives");
    fuzz(
        "archives",
        2_000,
        |g| {
            let archive = g.below(corpus.len());
            let tight = g.chance(0.3);
            (archive, corruption(g, &corpus[archive].0), format(g), tight)
        },
        |(archive, corruption, format, tight)| {
            let (bytes, externals) = &corpus[*archive];
            if let Ok(fig) = Figure::from_archive(&corruption.apply(bytes), externals, &NoProgram) {
                let options = SaveOptions {
                    bbox_inches: tight.then_some(BboxInches::Tight),
                    ..SaveOptions::default()
                };
                let _ = fig.to_format(*format, options);
            }
        },
    );
}
