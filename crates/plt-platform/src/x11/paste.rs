//! Reading the clipboard: asking its owner for text and taking the answer
//! from a property of the window that asked (ICCCM section 2.4).

use super::X11Backend;
use super::atoms::predefined;
use super::reply;
use super::request;
use super::server_event::SelectionNotify;
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::window::WindowId;

/// The most text read from the clipboard, in 32-bit units (4 MiB).
const MAX_TEXT_WORDS: u32 = 1 << 20;

impl X11Backend {
    /// Asks the owner of the clipboard to store its text, as UTF-8, in a
    /// property of `window`; the answer arrives as a `SelectionNotify`.
    pub(super) fn request_text(&mut self, window: WindowId) -> Result<(), Error> {
        let (xid, _) = self.window(window)?;
        self.ask_for_text(xid, self.atoms.utf8_string)
    }

    fn ask_for_text(&mut self, xid: u32, target: u32) -> Result<(), Error> {
        self.pastes.insert(xid, target);
        let convert = request::convert_selection(
            xid,
            self.atoms.clipboard,
            target,
            self.atoms.paste,
            self.last_input_time,
        );
        self.conn.send(&convert)?;
        self.conn.flush()
    }

    /// Handles the answer to a request for the clipboard's text: reads it,
    /// or asks again for Latin-1 when the owner has no UTF-8, and reports
    /// the outcome to the window that asked.
    pub(super) fn receive_text(
        &mut self,
        out: &mut Vec<Event>,
        answer: SelectionNotify,
    ) -> Result<(), Error> {
        if answer.selection != self.atoms.clipboard {
            return Ok(());
        }
        let Some(target) = self.pastes.remove(&answer.requestor) else {
            return Ok(());
        };
        let text = if answer.property == 0 {
            if target == self.atoms.utf8_string {
                return self.ask_for_text(answer.requestor, predefined::STRING);
            }
            None
        } else {
            let take = request::take_property(answer.requestor, answer.property, MAX_TEXT_WORDS);
            let reply = self.conn.round_trip(&take)?;
            let property = reply::property(&reply)?;
            decode_text(&property, self.atoms.incr)
        };
        if self.windows.contains_key(&answer.requestor) {
            let window = WindowId::from_raw(u64::from(answer.requestor));
            let event = WindowEvent::ClipboardText(text);
            out.push(Event::Window { window, event });
        }
        Ok(())
    }
}

/// The text in `property`: Latin-1 for `STRING`, else UTF-8 (invalid
/// sequences replaced). Incremental transfers (type `incr`) of very large
/// text are not supported.
fn decode_text(property: &reply::Property<'_>, incr: u32) -> Option<String> {
    if property.format != 8 || property.kind == incr {
        None
    } else if property.kind == predefined::STRING {
        Some(property.value.iter().copied().map(char::from).collect())
    } else {
        Some(String::from_utf8_lossy(property.value).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_decoded_by_type() {
        let property = |kind, format, value: &'static [u8]| reply::Property {
            kind,
            format,
            value,
        };
        let (utf8, incr) = (300, 301);
        let decoded = |p| decode_text(&p, incr);
        assert_eq!(decoded(property(utf8, 8, "é".as_bytes())), Some("é".into()));
        assert_eq!(
            decoded(property(predefined::STRING, 8, b"caf\xe9")),
            Some("café".into())
        );
        assert_eq!(
            decoded(property(utf8, 8, b"a\xffb")),
            Some("a\u{fffd}b".into())
        );
        assert_eq!(decoded(property(incr, 8, b"")), None, "too large");
        assert_eq!(decoded(property(predefined::ATOM, 32, &[0; 4])), None);
    }
}
