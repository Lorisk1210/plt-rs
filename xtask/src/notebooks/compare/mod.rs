//! Two records of one notebook compared: matplotlib's and pltrs'.
//!
//! Each comparison is a check with an id of the outcome files,
//! `notebooks/<name>.ipynb::<what>`:
//!
//! * `run`: the notebook ran to its end;
//! * `<cell>::outputs`: the cell's outputs are of the same kinds, in the
//!   same order (`display_data` with the same mime types, streams of the
//!   same names, ...), and `<cell>::output<k>` is its `k`-th output: every
//!   `display_data` and `execute_result` with its mime bundle (PNG and
//!   JPEG pixels within matplotlib's RMS tolerance, SVG and PDF documents
//!   item by item, `text/plain` reprs and HTML with their ids, addresses
//!   and embedded images normalized) and metadata (`width` and `height`
//!   for retina, `needs_background`), every stream's text, every error's
//!   class and message;
//! * `<cell>::messages` and `<cell>::message<k>`, where either run sent
//!   messages besides outputs: the comm messages of the widget and nbAgg
//!   backends (their types, keys and values, the images their buffers and
//!   states carry), and what the kernel sent while it handled the
//!   frontend's messages.
//!
//! Under matplotlib every check passes but `run` when the reference run
//! could not finish; under pltrs a check passes when pltrs sent what
//! matplotlib sent.

mod media;
mod text;
mod values;

use pltrs_core::json::Json;
use pltrs_spec::figures::vector::VectorFormat;

use super::record::{Cell, Message, Record, text_of};
use media::{ImageFormat, compare_documents, compare_images, decoded_base64};
use values::{Context, Mismatch, compare_json, compare_strings};

/// One comparison and how each run ends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Check {
    /// Its id in the outcome files.
    pub(super) id: String,
    /// Why the reference run fails it, if it does.
    pub(super) reference: Option<String>,
    /// Why pltrs fails it, if it does.
    pub(super) pltrs: Option<String>,
}

/// The checks of the notebook `file` (`notebooks/<name>.ipynb`), whose
/// records are `reference` (matplotlib's) and `pltrs`.
pub(super) fn compare(file: &str, reference: &Record, pltrs: &Record) -> Vec<Check> {
    let mut checks = vec![Check {
        id: format!("{file}::run"),
        reference: reference.failure.clone(),
        pltrs: pltrs.failure.clone(),
    }];
    let mut cx = Context::default();
    for (index, expected) in reference.cells.iter().enumerate() {
        let actual = pltrs.cells.get(index).filter(|cell| cell.id == expected.id);
        checks.extend(compare_cell(
            &format!("{file}::{}", expected.id),
            expected,
            actual,
            &mut cx,
        ));
    }
    checks
}

/// The checks of the cell `expected` (`prefix`: its id) against `actual`,
/// pltrs' run of it (None if pltrs did not run it).
fn compare_cell(
    prefix: &str,
    expected: &Cell,
    actual: Option<&Cell>,
    cx: &mut Context,
) -> Vec<Check> {
    let mut checks = Vec::new();
    let mut check = |what: &str, outcome: Result<(), Mismatch>| {
        checks.push(Check {
            id: format!("{prefix}::{what}"),
            reference: None,
            pltrs: outcome.err().map(|mismatch| mismatch.to_string()),
        });
    };
    let Some(actual) = actual else {
        check("outputs", Err(Mismatch::new("the cell did not run")));
        return checks;
    };
    check(
        "outputs",
        same_kinds(&expected.outputs, &actual.outputs, output_kind),
    );
    for (index, output) in expected.outputs.iter().enumerate() {
        let outcome = actual
            .outputs
            .get(index)
            .ok_or_else(|| Mismatch::new("missing"))
            .and_then(|other| compare_output(output, other, cx));
        check(&format!("output{index}"), outcome);
    }
    if expected.messages.is_empty() && actual.messages.is_empty() {
        return checks;
    }
    check(
        "messages",
        same_kinds(&expected.messages, &actual.messages, message_kind),
    );
    for (index, message) in expected.messages.iter().enumerate() {
        let outcome = actual
            .messages
            .get(index)
            .ok_or_else(|| Mismatch::new("missing"))
            .and_then(|other| compare_message(message, other, cx));
        check(&format!("message{index}"), outcome);
    }
    checks
}

/// Whether `expected` and `actual` hold items of the same kinds, by
/// `kind`, in the same order.
fn same_kinds<T>(expected: &[T], actual: &[T], kind: fn(&T) -> String) -> Result<(), Mismatch> {
    let kinds = |items: &[T]| items.iter().map(kind).collect::<Vec<_>>();
    let (a, b) = (kinds(expected), kinds(actual));
    if a == b {
        Ok(())
    } else {
        Err(Mismatch::new(format!(
            "[{}], not [{}]",
            b.join(", "),
            a.join(", ")
        )))
    }
}

/// The kind of the output `output`: its type and its mime types, stream
/// name or error class.
fn output_kind(output: &Json) -> String {
    let kind = output
        .get("output_type")
        .and_then(Json::as_str)
        .unwrap_or("?");
    let detail = match kind {
        "stream" => output.get("name").and_then(Json::as_str).map(str::to_owned),
        "error" => output
            .get("ename")
            .and_then(Json::as_str)
            .map(str::to_owned),
        _ => output.get("data").and_then(Json::as_object).map(|bundle| {
            let mut mimes: Vec<&str> = bundle.iter().map(|(mime, _)| mime.as_str()).collect();
            mimes.sort_unstable();
            mimes.join(" ")
        }),
    };
    detail.map_or_else(|| kind.to_owned(), |detail| format!("{kind}({detail})"))
}

/// The kind of the message `message`: its type, and for comm messages
/// the comm's target, the widget method or the type of matplotlib's
/// message it carries.
fn message_kind(message: &Message) -> String {
    if let Some(output) = message.as_output() {
        return output_kind(&output);
    }
    let content = &message.content;
    let data = content.get("data");
    let field = |value: Option<&Json>, key: &str| {
        value
            .and_then(|v| v.get(key))
            .and_then(Json::as_str)
            .map(str::to_owned)
    };
    let protocol = |text: Option<String>| {
        text.and_then(|text| Json::parse(&text).ok())
            .and_then(|message| {
                message
                    .get("type")
                    .and_then(Json::as_str)
                    .map(str::to_owned)
            })
    };
    let detail = field(Some(content), "target_name")
        .or_else(|| field(data, "blob").map(|_| "blob".to_owned()))
        .or_else(|| {
            protocol(field(data.and_then(|d| d.get("content")), "data"))
                .map(|kind| format!("custom {kind}"))
        })
        .or_else(|| protocol(field(data, "data")))
        .or_else(|| field(data, "method"));
    detail.map_or_else(
        || message.kind.clone(),
        |detail| format!("{}({detail})", message.kind),
    )
}

/// Whether the outputs `expected` and `actual` are the same (see the
/// module's description).
fn compare_output(expected: &Json, actual: &Json, cx: &mut Context) -> Result<(), Mismatch> {
    let (kind, other) = (output_kind(expected), output_kind(actual));
    if kind != other {
        return Err(Mismatch::new(format!("{other}, not {kind}")));
    }
    let field = |output: &Json, key: &str| output.get(key).map(text_of).unwrap_or_default();
    match expected.get("output_type").and_then(Json::as_str) {
        Some("stream") => compare_strings(&field(expected, "text"), &field(actual, "text"), cx)
            .map_err(|m| m.within("text")),
        Some("error") => compare_strings(&field(expected, "evalue"), &field(actual, "evalue"), cx)
            .map_err(|m| m.within("evalue")),
        Some("display_data" | "execute_result") => {
            for key in ["data", "metadata", "execution_count"] {
                let (a, b) = (expected.get(key), actual.get(key));
                let outcome = match (key, a, b) {
                    ("data", Some(Json::Object(a)), Some(Json::Object(b))) => {
                        compare_bundle(a, b, cx)
                    }
                    (_, Some(a), Some(b)) => compare_json(a, b, cx),
                    _ if a == b => Ok(()),
                    _ => Err(Mismatch::new("missing")),
                };
                outcome.map_err(|m| m.within(key))?;
            }
            Ok(())
        }
        _ => compare_json(expected, actual, cx),
    }
}

/// Whether the mime bundles `expected` and `actual`, which hold the same
/// mime types, hold the same data.
fn compare_bundle(
    expected: &[(String, Json)],
    actual: &[(String, Json)],
    cx: &mut Context,
) -> Result<(), Mismatch> {
    for (mime, value) in expected {
        let other = actual
            .iter()
            .find(|(key, _)| key == mime)
            .map_or(&Json::Null, |(_, value)| value);
        compare_mime(mime, value, other, cx).map_err(|m| m.within(mime))?;
    }
    Ok(())
}

/// Whether the data `expected` and `actual` of the mime type `mime` are
/// the same.
fn compare_mime(
    mime: &str,
    expected: &Json,
    actual: &Json,
    cx: &mut Context,
) -> Result<(), Mismatch> {
    let (a, b) = (text_of(expected), text_of(actual));
    if let Some(format) = ImageFormat::of_mime(mime) {
        return Ok(compare_images(
            format,
            &decoded_base64(&a)?,
            &decoded_base64(&b)?,
        )?);
    }
    match mime {
        "image/svg+xml" => Ok(compare_documents(
            VectorFormat::Svg,
            a.as_bytes(),
            b.as_bytes(),
        )?),
        "application/pdf" => Ok(compare_documents(
            VectorFormat::Pdf,
            &decoded_base64(&a)?,
            &decoded_base64(&b)?,
        )?),
        _ if matches!(
            (expected, actual),
            (
                Json::String(_) | Json::Array(_),
                Json::String(_) | Json::Array(_)
            )
        ) =>
        {
            compare_strings(&a, &b, cx)
        }
        _ => compare_json(expected, actual, cx),
    }
}

/// Whether the messages `expected` and `actual` are the same: as outputs
/// for the messages that are outputs, else their content, and their
/// buffers (images by their pixels).
fn compare_message(expected: &Message, actual: &Message, cx: &mut Context) -> Result<(), Mismatch> {
    let (kind, other) = (message_kind(expected), message_kind(actual));
    if kind != other {
        return Err(Mismatch::new(format!("{other}, not {kind}")));
    }
    if let (Some(a), Some(b)) = (expected.as_output(), actual.as_output()) {
        return compare_output(&a, &b, cx);
    }
    compare_json(&expected.content, &actual.content, cx)?;
    if expected.buffers.len() != actual.buffers.len() {
        return Err(Mismatch::new(format!(
            "{} buffers, not {}",
            actual.buffers.len(),
            expected.buffers.len()
        )));
    }
    for (index, (a, b)) in expected.buffers.iter().zip(&actual.buffers).enumerate() {
        let outcome = match (ImageFormat::of_bytes(a), ImageFormat::of_bytes(b)) {
            (Some(format), Some(other)) if format == other => compare_images(format, a, b),
            _ if a == b => Ok(()),
            _ => Err(format!(
                "{} bytes, not the {} matplotlib sent",
                b.len(),
                a.len()
            )),
        };
        outcome.map_err(|err| Mismatch::new(err).within(format!("buffer {index}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: &str, outputs: &str, messages: Vec<Message>) -> Cell {
        let Json::Array(outputs) = Json::parse(outputs).unwrap() else {
            unreachable!("outputs are an array")
        };
        Cell {
            id: id.to_owned(),
            outputs,
            messages,
        }
    }

    fn message(kind: &str, content: &str) -> Message {
        Message {
            kind: kind.to_owned(),
            content: Json::parse(content).unwrap(),
            buffers: Vec::new(),
        }
    }

    fn record(cells: Vec<Cell>) -> Record {
        Record {
            cells,
            failure: None,
        }
    }

    /// The ids of the checks pltrs fails, with why.
    fn failing(checks: &[Check]) -> Vec<(String, String)> {
        checks
            .iter()
            .filter_map(|check| Some((check.id.clone(), check.pltrs.clone()?)))
            .collect()
    }

    #[test]
    fn equal_records_pass_every_check() {
        let outputs = r#"[{"output_type": "execute_result", "execution_count": 1,
            "data": {"text/plain": "[<matplotlib.lines.Line2D at 0x7f00>]"}, "metadata": {}},
            {"output_type": "stream", "name": "stdout", "text": "x\n"}]"#;
        let draw = r#"{"comm_id": "0123456789abcdef0123456789abcdef",
            "data": {"method": "custom", "content": {"data": "{\"type\": \"draw\"}"}}}"#;
        let a = record(vec![cell("plot", outputs, vec![message("comm_msg", draw)])]);
        let pltrs_outputs = outputs.replace("7f00", "55aa");
        let b = record(vec![cell(
            "plot",
            &pltrs_outputs,
            vec![message("comm_msg", draw)],
        )]);
        let checks = compare("notebooks/n.ipynb", &a, &b);
        let ids: Vec<&str> = checks.iter().map(|check| check.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "notebooks/n.ipynb::run",
                "notebooks/n.ipynb::plot::outputs",
                "notebooks/n.ipynb::plot::output0",
                "notebooks/n.ipynb::plot::output1",
                "notebooks/n.ipynb::plot::messages",
                "notebooks/n.ipynb::plot::message0",
            ]
        );
        assert_eq!(failing(&checks), []);
        assert_eq!(
            message_kind(&a.cells[0].messages[0]),
            "comm_msg(custom draw)"
        );
    }

    #[test]
    fn differences_fail_their_checks() {
        let a = record(vec![cell(
            "c",
            r#"[{"output_type": "error", "ename": "ValueError", "evalue": "bad", "traceback": []}]"#,
            Vec::new(),
        )]);
        let b = record(vec![cell(
            "c",
            r#"[{"output_type": "error", "ename": "ValueError", "evalue": "worse", "traceback": ["x"]},
                {"output_type": "stream", "name": "stderr", "text": "w"}]"#,
            vec![message("comm_close", r#"{"comm_id": "a", "data": {}}"#)],
        )]);
        assert_eq!(
            failing(&compare("f", &a, &b)),
            [
                (
                    "f::c::outputs".to_owned(),
                    "[error(ValueError), stream(stderr)], not [error(ValueError)]".to_owned()
                ),
                (
                    "f::c::output0".to_owned(),
                    r#"evalue: "worse", not "bad""#.to_owned()
                ),
                (
                    "f::c::messages".to_owned(),
                    "[comm_close], not []".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn cells_pltrs_did_not_run_fail() {
        let a = record(vec![cell("c", "[]", Vec::new())]);
        let mut b = record(Vec::new());
        b.failure = Some("cell c: DeadKernelError".to_owned());
        assert_eq!(
            failing(&compare("f", &a, &b)),
            [
                ("f::run".to_owned(), "cell c: DeadKernelError".to_owned()),
                (
                    "f::c::outputs".to_owned(),
                    "the cell did not run".to_owned()
                ),
            ]
        );
    }
}
