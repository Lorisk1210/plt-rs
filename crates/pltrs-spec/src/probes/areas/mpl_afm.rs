//! Probe operations for the `mpl_afm` area: Adobe Font Metrics ported from
//! matplotlib's `tests/test_afm.py` (see
//! `spec/reference/probes/test_mpl_afm.py`).

use pltrs_font::{AfmMetrics, CharMetrics};

use super::afm::{kern_dist_from_name, latin1};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("_afm.AFM.parse", parse),
    Op::new("_afm.AFM.get_kern_dist_from_name", kern_dist_from_name),
];

/// `[width, name, bbox]` of one character.
fn metrics_json(char: &CharMetrics) -> Json {
    Json::Array(vec![
        char.width.into(),
        char.name.as_str().into(),
        Json::Array(char.bbox.iter().map(|&v| Json::Integer(v as i64)).collect()),
    ])
}

/// The metrics `find` gives for every item of the `key` list (none
/// without one); an item it finds nothing for is matplotlib's `KeyError`.
fn char_list<'m>(
    args: Args<'_>,
    key: &str,
    find: impl Fn(&Json) -> Option<&'m CharMetrics>,
) -> Result<Json, ProbeError> {
    let Some(list) = args.get(key) else {
        return Ok(Json::Array(Vec::new()));
    };
    list.as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list")))?
        .iter()
        .map(|item| {
            find(item)
                .map(metrics_json)
                .ok_or_else(|| ProbeError::Rejected(format!("no character {item}")))
        })
        .collect::<Result<_, _>>()
        .map(Json::Array)
}

/// What `AFM(BytesIO(data.encode(encoding)))` reads: the header getters
/// and the metrics of the characters asked for.
fn parse(args: Args<'_>) -> Outcome {
    let text = args.str("data")?;
    let data = match args.opt_str("encoding")?.unwrap_or("latin-1") {
        "latin-1" => latin1(text)?,
        "utf-8" => text.as_bytes().to_vec(),
        other => return Err(ProbeError::BadArgs(format!("unknown encoding {other}"))),
    };
    let afm = AfmMetrics::parse(&data).map_err(ProbeError::rejected)?;
    let chars = char_list(args, "codes", |code| afm.char_by_code(code.as_i64()?))?;
    let by_name = char_list(args, "names", |name| afm.char_by_name(name.as_str()?))?;
    Ok(Json::Object(vec![
        ("fontname".into(), afm.font_name().ok().into()),
        ("fullname".into(), afm.full_name().ok().into()),
        ("familyname".into(), afm.family_name().ok().into()),
        ("weight".into(), afm.weight().ok().into()),
        ("angle".into(), afm.angle().ok().into()),
        (
            "underline_thickness".into(),
            afm.underline_thickness().ok().into(),
        ),
        ("chars".into(), chars),
        ("by_name".into(), by_name),
    ]))
}
