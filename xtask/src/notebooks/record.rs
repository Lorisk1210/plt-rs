//! The record of one notebook's execution, as `spec/notebooks/execute.py`
//! writes it: per cell its outputs and the messages the kernel sent
//! besides, and why the execution stopped early, if it did.

use std::path::Path;

use pltrs_codec::base64;
use pltrs_core::json::Json;

/// What one kernel sent while it executed a notebook.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Record {
    /// The cells executed, in order.
    pub(super) cells: Vec<Cell>,
    /// Why the execution stopped before the last cell (the kernel died, a
    /// cell timed out), if it did.
    pub(super) failure: Option<String>,
}

/// One cell's share of a record.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Cell {
    /// The cell's id in the notebook.
    pub(super) id: String,
    /// Its outputs as nbformat keeps them (`output_type` and the fields
    /// of that type), consecutive streams of one name joined.
    pub(super) outputs: Vec<Json>,
    /// The comm messages the kernel sent while the cell ran, then every
    /// message it sent while it handled what the frontend sent after it.
    pub(super) messages: Vec<Message>,
}

/// A message of the kernel's, other than its outputs.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Message {
    /// Its type (`comm_open`, `comm_msg`, `stream`, ...).
    pub(super) kind: String,
    /// Its content.
    pub(super) content: Json,
    /// Its binary buffers.
    pub(super) buffers: Vec<Vec<u8>>,
}

impl Message {
    /// The output the message stands for, as nbformat keeps an output of
    /// a cell (`output_type` added to its content), for the messages that
    /// are outputs (`stream`, `display_data`, `execute_result`, `error`).
    pub(super) fn as_output(&self) -> Option<Json> {
        let Json::Object(members) = &self.content else {
            return None;
        };
        matches!(
            self.kind.as_str(),
            "stream" | "display_data" | "execute_result" | "error"
        )
        .then(|| {
            let mut output = vec![("output_type".to_owned(), Json::String(self.kind.clone()))];
            output.extend(
                members
                    .iter()
                    .filter(|(key, _)| key != "transient")
                    .cloned(),
            );
            Json::Object(output)
        })
    }
}

impl Record {
    /// Reads the record at `path`.
    pub(super) fn read(path: &Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
        let document = Json::parse(&text).map_err(|err| format!("{}: {err}", path.display()))?;
        Self::from_json(&document).map_err(|err| format!("{}: {err}", path.display()))
    }

    /// The record of the JSON `document`.
    fn from_json(document: &Json) -> Result<Self, String> {
        let cells = document
            .get("cells")
            .and_then(Json::as_array)
            .ok_or("no cells")?
            .iter()
            .map(Cell::from_json)
            .collect::<Result<_, _>>()?;
        let failure = document
            .get("failure")
            .and_then(Json::as_str)
            .map(str::to_owned);
        Ok(Self { cells, failure })
    }
}

impl Cell {
    /// The cell of the JSON `cell`.
    fn from_json(cell: &Json) -> Result<Self, String> {
        let id = cell
            .get("id")
            .and_then(Json::as_str)
            .ok_or("a cell without id")?
            .to_owned();
        let outputs = cell
            .get("outputs")
            .and_then(Json::as_array)
            .ok_or_else(|| format!("cell {id}: no outputs"))?;
        let messages = cell
            .get("messages")
            .and_then(Json::as_array)
            .ok_or_else(|| format!("cell {id}: no messages"))?
            .iter()
            .map(Message::from_json)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("cell {id}: {err}"))?;
        Ok(Self {
            outputs: joined_streams(outputs.to_vec()),
            messages: joined_stream_messages(messages),
            id,
        })
    }
}

impl Message {
    /// The message of the JSON `message`.
    fn from_json(message: &Json) -> Result<Self, String> {
        let kind = message
            .get("type")
            .and_then(Json::as_str)
            .ok_or("a message without type")?
            .to_owned();
        let content = message.get("content").cloned().unwrap_or(Json::Null);
        let buffers = message
            .get("buffers")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .map(|buffer| {
                let text = buffer.as_str().ok_or("a buffer that is no string")?;
                base64::decode(text).map_err(|err| format!("a buffer: {err}"))
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            kind,
            content,
            buffers,
        })
    }
}

/// The stream `output`'s name and text, if it is a stream.
fn stream(output: &Json) -> Option<(&str, String)> {
    if output.get("output_type").and_then(Json::as_str) != Some("stream") {
        return None;
    }
    let name = output.get("name").and_then(Json::as_str)?;
    Some((name, text_of(output.get("text")?)))
}

/// The text of an nbformat multi-line string: a string, or the list of
/// its lines.
pub(super) fn text_of(value: &Json) -> String {
    match value {
        Json::String(text) => text.clone(),
        Json::Array(lines) => lines.iter().filter_map(Json::as_str).collect(),
        _ => String::new(),
    }
}

/// The stream output named `name` with `text`.
fn stream_output(name: &str, text: String) -> Json {
    Json::Object(vec![
        ("output_type".to_owned(), Json::String("stream".to_owned())),
        ("name".to_owned(), Json::String(name.to_owned())),
        ("text".to_owned(), Json::String(text)),
    ])
}

/// `outputs` with consecutive streams of one name joined into one, as
/// nbconvert's `coalesce_streams` joins them: the kernel flushes a stream
/// whenever its interval elapses, so where one output ends and the next
/// begins depends on timing.
fn joined_streams(outputs: Vec<Json>) -> Vec<Json> {
    let mut joined: Vec<Json> = Vec::with_capacity(outputs.len());
    for output in outputs {
        if let (Some((name, text)), Some(last)) = (stream(&output), joined.last_mut())
            && let Some((last_name, last_text)) = stream(last)
            && last_name == name
        {
            *last = stream_output(name, last_text + &text);
            continue;
        }
        joined.push(output);
    }
    joined
}

/// `messages` with consecutive stream messages of one name joined (see
/// [`joined_streams`]).
fn joined_stream_messages(messages: Vec<Message>) -> Vec<Message> {
    let mut joined: Vec<Message> = Vec::with_capacity(messages.len());
    for message in messages {
        let output = message.as_output();
        if let (Some((name, text)), Some(last)) =
            (output.as_ref().and_then(stream), joined.last_mut())
            && let Some(last_output) = last.as_output()
            && let Some((last_name, last_text)) = stream(&last_output)
            && last_name == name
        {
            last.content = Json::Object(vec![
                ("name".to_owned(), Json::String(name.to_owned())),
                ("text".to_owned(), Json::String(last_text + &text)),
            ]);
            continue;
        }
        joined.push(message);
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_are_read_with_their_buffers() {
        let document = Json::parse(
            r#"{"cells": [{"id": "a", "outputs": [], "messages": [
                {"type": "comm_msg", "content": {"comm_id": "x"}, "buffers": ["AAEC"]}]}],
               "failure": null}"#,
        )
        .unwrap();
        let record = Record::from_json(&document).unwrap();
        assert_eq!(record.failure, None);
        assert_eq!(record.cells[0].id, "a");
        assert_eq!(record.cells[0].messages[0].buffers, [vec![0, 1, 2]]);
        assert_eq!(record.cells[0].messages[0].as_output(), None);
    }

    #[test]
    fn consecutive_streams_of_one_name_are_joined() {
        let document = Json::parse(
            r#"{"cells": [{"id": "a", "messages": [], "outputs": [
                {"output_type": "stream", "name": "stdout", "text": "a\n"},
                {"output_type": "stream", "name": "stdout", "text": ["b\n", "c"]},
                {"output_type": "stream", "name": "stderr", "text": "d"},
                {"output_type": "stream", "name": "stdout", "text": "e"}]}],
               "failure": "cell a: DeadKernelError"}"#,
        )
        .unwrap();
        let record = Record::from_json(&document).unwrap();
        let texts: Vec<String> = record.cells[0]
            .outputs
            .iter()
            .map(|output| text_of(output.get("text").unwrap()))
            .collect();
        assert_eq!(texts, ["a\nb\nc", "d", "e"]);
        assert_eq!(record.failure.as_deref(), Some("cell a: DeadKernelError"));
    }

    #[test]
    fn output_messages_read_as_outputs() {
        let message = Message {
            kind: "stream".to_owned(),
            content: Json::parse(r#"{"name": "stdout", "text": "x"}"#).unwrap(),
            buffers: Vec::new(),
        };
        let output = message.as_output().unwrap();
        assert_eq!(
            output.get("output_type").and_then(Json::as_str),
            Some("stream")
        );
        let joined = joined_stream_messages(vec![message.clone(), message]);
        assert_eq!(joined.len(), 1);
        assert_eq!(
            joined[0].content.get("text").and_then(Json::as_str),
            Some("xx")
        );
    }
}
