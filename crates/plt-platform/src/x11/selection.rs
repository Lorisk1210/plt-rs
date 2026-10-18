//! Owning the clipboard: offering an image and handing it to the clients
//! that ask (ICCCM section 2.2), within one request so that no `INCR`
//! transfer is needed.

use super::X11Backend;
use super::atoms::predefined;
use super::request;
use super::server_event::SelectionRequest;
use crate::error::Error;
use crate::window::WindowId;

/// The bytes a `ChangeProperty` request adds to its data: the header and
/// the padding to a four-byte boundary.
const CHANGE_PROPERTY_OVERHEAD: usize = 28;

impl X11Backend {
    /// Makes `window` the owner of the clipboard, holding `png`.
    pub(super) fn offer_clipboard(&mut self, window: WindowId, png: Vec<u8>) -> Result<(), Error> {
        let (xid, _) = self.window(window)?;
        self.clipboard = Some(png);
        let owner = request::set_selection_owner(xid, self.atoms.clipboard, self.last_input_time);
        self.conn.send(&owner)?;
        self.conn.flush()
    }

    /// Answers a request for the clipboard: its targets, or the image as
    /// `image/png` when it fits in one request; anything else is refused.
    pub(super) fn answer_selection(&mut self, asked: SelectionRequest) -> Result<(), Error> {
        // Obsolete clients name no property: the target is used instead.
        let property = if asked.property == 0 {
            asked.target
        } else {
            asked.property
        };
        let stored = match &self.clipboard {
            Some(png) if asked.selection == self.atoms.clipboard => {
                if asked.target == self.atoms.targets {
                    let targets = [self.atoms.targets, self.atoms.image_png];
                    Some(request::set_property_u32(
                        asked.requestor,
                        property,
                        predefined::ATOM,
                        &targets,
                    ))
                } else if asked.target == self.atoms.image_png
                    && png.len() + CHANGE_PROPERTY_OVERHEAD <= self.conn.max_request_bytes()
                {
                    Some(request::set_property_bytes(
                        asked.requestor,
                        property,
                        self.atoms.image_png,
                        png,
                    ))
                } else {
                    None
                }
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

    /// Forgets the clipboard once another client owns it.
    pub(super) fn clipboard_taken(&mut self, selection: u32) {
        if selection == self.atoms.clipboard {
            self.clipboard = None;
        }
    }
}
