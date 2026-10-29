//! Probe operations for the `mpl_determinism` area: the creation dates of
//! vector documents, after matplotlib's `tests/test_determinism.py` (see
//! `spec/reference/probes/test_mpl_determinism.py`). The documents are
//! dated explicitly with the probe's epoch, which is what pltrs does with
//! `SOURCE_DATE_EPOCH` (read in `crates/pltrs/tests/determinism`).

use pltrs::figure::{Figure, SaveFormat, SaveOptions};
use pltrs_render::{Metadata, Timestamp};

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("savefig.creation_date", creation_date)];

/// The text between `start` and the first `end` after it.
fn between<'a>(document: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let rest = &document[document.find(start)? + start.len()..];
    rest.find(end).map(|stop| &rest[..stop])
}

/// The creation date an empty figure saved as `format` records.
fn creation_date(args: Args<'_>) -> Outcome {
    let (format, start, end) = match args.str("format")? {
        "pdf" => (SaveFormat::Pdf, "/CreationDate (", ")"),
        "ps" => (SaveFormat::Ps, "%%CreationDate: ", "\n"),
        "svg" => (SaveFormat::Svg, "<dc:date>", "</dc:date>"),
        other => return Err(ProbeError::BadArgs(format!("unknown format {other}"))),
    };
    let options = SaveOptions {
        metadata: Some(Metadata {
            date: Some(Timestamp(args.i64("epoch")?)),
            ..Metadata::default()
        }),
        ..SaveOptions::default()
    };
    with_rc(args, || {
        let bytes = Figure::new()
            .to_format(format, options)
            .map_err(ProbeError::rejected)?;
        let document = String::from_utf8_lossy(&bytes);
        between(&document, start, end)
            .map(Json::from)
            .ok_or_else(|| ProbeError::rejected("the document records no creation date"))
    })
}
