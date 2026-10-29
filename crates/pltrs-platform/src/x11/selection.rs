//! Owning the clipboard: offering an image or text and handing it to the
//! clients that ask (ICCCM section 2.2), within one request so that no
//! `INCR` transfer is needed.

use super::X11Backend;
use super::atoms::{Atoms, predefined};
use super::request::{self, Request};
use super::server_event::SelectionRequest;
use crate::error::Error;
use crate::window::WindowId;

/// The bytes a `ChangeProperty` request adds to its data: the header and
/// the padding to a four-byte boundary.
const CHANGE_PROPERTY_OVERHEAD: usize = 28;

/// What this connection offers on the clipboard while it owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Clipboard {
    /// A PNG image, offered as `image/png`.
    Image(Vec<u8>),
    /// Text, offered as UTF-8 (`UTF8_STRING`, `TEXT` and
    /// `text/plain;charset=utf-8`) and as Latin-1 (`STRING`).
    Text(String),
}

impl Clipboard {
    /// The forms the content can be converted to, `TARGETS` first.
    fn targets(&self, atoms: &Atoms) -> Vec<u32> {
        match self {
            Self::Image(_) => vec![atoms.targets, atoms.image_png],
            Self::Text(_) => vec![
                atoms.targets,
                atoms.utf8_string,
                atoms.text_plain_utf8,
                atoms.text,
                predefined::STRING,
            ],
        }
    }

    /// The content converted to `target`, with the type to store it
    /// under, or `None` when it has no such form.
    fn convert(&self, target: u32, atoms: &Atoms) -> Option<(u32, Vec<u8>)> {
        match self {
            Self::Image(png) if target == atoms.image_png => Some((target, png.clone())),
            Self::Text(text) if target == predefined::STRING => {
                Some((predefined::STRING, latin1(text)))
            }
            Self::Text(text)
                if [atoms.utf8_string, atoms.text_plain_utf8, atoms.text].contains(&target) =>
            {
                // `TEXT` lets the owner pick an encoding: UTF-8 here.
                let kind = if target == atoms.text {
                    atoms.utf8_string
                } else {
                    target
                };
                Some((kind, text.as_bytes().to_vec()))
            }
            _ => None,
        }
    }
}

/// `text` in Latin-1, the encoding of `STRING`; characters it lacks become
/// `?`.
fn latin1(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect()
}

impl X11Backend {
    /// Makes `window` the owner of the clipboard, holding `content`.
    pub(super) fn offer_clipboard(
        &mut self,
        window: WindowId,
        content: Clipboard,
    ) -> Result<(), Error> {
        let (xid, _) = self.window(window)?;
        self.clipboard = Some(content);
        let owner = request::set_selection_owner(xid, self.atoms.clipboard, self.last_input_time);
        self.conn.send(&owner)?;
        self.conn.flush()
    }

    /// Answers a request for the clipboard: its targets, or its content in
    /// the form asked for when it fits in one request; anything else is
    /// refused.
    pub(super) fn answer_selection(&mut self, asked: SelectionRequest) -> Result<(), Error> {
        // Obsolete clients name no property: the target is used instead.
        let property = if asked.property == 0 {
            asked.target
        } else {
            asked.property
        };
        let stored = match &self.clipboard {
            Some(content) if asked.selection == self.atoms.clipboard => {
                self.store(content, asked.requestor, property, asked.target)
            }
            _ => None,
        };
        let answered = match stored {
            Some(change) => {
                self.conn.send(&change)?;
                property
            }
            None => 0,
        };
        let notify = request::selection_notify(
            asked.time,
            asked.requestor,
            asked.selection,
            asked.target,
            answered,
        );
        self.conn
            .send(&request::send_event(asked.requestor, 0, &notify))?;
        self.conn.flush()
    }

    /// The request storing `content` converted to `target` in `property`
    /// of `requestor`, if it has that form and fits in one request.
    fn store(
        &self,
        content: &Clipboard,
        requestor: u32,
        property: u32,
        target: u32,
    ) -> Option<Request> {
        if target == self.atoms.targets {
            let targets = content.targets(&self.atoms);
            return Some(request::set_property_u32(
                requestor,
                property,
                predefined::ATOM,
                &targets,
            ));
        }
        let (kind, data) = content.convert(target, &self.atoms)?;
        (data.len() + CHANGE_PROPERTY_OVERHEAD <= self.conn.max_request_bytes())
            .then(|| request::set_property_bytes(requestor, property, kind, &data))
    }

    /// Forgets the clipboard once another client owns it.
    pub(super) fn clipboard_taken(&mut self, selection: u32) {
        if selection == self.atoms.clipboard {
            self.clipboard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_offered_in_utf8_and_latin1() {
        assert_eq!(latin1("café €"), b"caf\xe9 ?");
        let atoms = Atoms::numbered();
        let text = Clipboard::Text("é".into());
        assert_eq!(
            text.targets(&atoms),
            [
                atoms.targets,
                atoms.utf8_string,
                atoms.text_plain_utf8,
                atoms.text,
                predefined::STRING
            ]
        );
        let utf8 = Some((atoms.utf8_string, "é".into()));
        assert_eq!(text.convert(atoms.text, &atoms), utf8);
        assert_eq!(text.convert(atoms.utf8_string, &atoms), utf8);
        assert_eq!(
            text.convert(predefined::STRING, &atoms),
            Some((predefined::STRING, vec![0xe9]))
        );
        assert_eq!(text.convert(atoms.image_png, &atoms), None);
        let image = Clipboard::Image(vec![1]);
        assert_eq!(
            image.convert(atoms.image_png, &atoms),
            Some((atoms.image_png, vec![1]))
        );
        assert_eq!(image.convert(atoms.utf8_string, &atoms), None);
    }
}
