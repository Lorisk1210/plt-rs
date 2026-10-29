//! The clipboard through `wl_data_device`: offering text or a PNG image,
//! and reading the text another client (or this one) offers.
//!
//! Transfers go through pipes. Each one runs on a thread of its own, so
//! that a slow or absent peer — or this very client, pasting what it
//! copied — never blocks the event loop; a finished paste wakes the loop,
//! which then reports it.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::sync::{Arc, Mutex, PoisonError};

use super::WaylandBackend;
use super::protocol::data;
use super::protocol::{Event, Interface};
use crate::backend::Waker;
use crate::error::Error;
use crate::event::{Event as LoopEvent, WindowEvent};
use crate::window::WindowId;

/// The text types offered, and asked for in this order of preference.
const TEXT_TYPES: [&str; 4] = [
    "text/plain;charset=utf-8",
    "UTF8_STRING",
    "text/plain",
    "TEXT",
];
/// The type of PNG images.
const PNG_TYPE: &str = "image/png";
/// The most text read from the clipboard (4 MiB).
const MAX_TEXT_LEN: u64 = 4 << 20;

/// What this client offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Content {
    /// A PNG image.
    Image(Arc<[u8]>),
    /// UTF-8 text.
    Text(Arc<str>),
}

impl Content {
    /// The MIME types it is offered as.
    fn types(&self) -> &'static [&'static str] {
        match self {
            Self::Image(_) => &[PNG_TYPE],
            Self::Text(_) => &TEXT_TYPES,
        }
    }

    /// The bytes to send for `mime`, if it has that form.
    fn bytes(&self, mime: &str) -> Option<Arc<[u8]>> {
        match self {
            Self::Image(png) if mime == PNG_TYPE => Some(Arc::clone(png)),
            Self::Text(text) if TEXT_TYPES.contains(&mime) => Some(Arc::from(text.as_bytes())),
            _ => None,
        }
    }
}

/// Pastes finished by reader threads, waiting to be reported.
type Pastes = Arc<Mutex<Vec<(WindowId, Option<String>)>>>;

/// The clipboard state.
#[derive(Debug, Default)]
pub(crate) struct Clipboard {
    /// The seat's `wl_data_device`.
    pub(crate) device: Option<u32>,
    /// The source this client put on the clipboard, and its content.
    source: Option<(u32, Content)>,
    /// Offers announced, with their MIME types so far.
    offers: HashMap<u32, Vec<String>>,
    /// The offer on the clipboard.
    selection: Option<u32>,
    /// Finished pastes.
    pastes: Pastes,
}

impl Clipboard {
    /// The text type to ask the clipboard's offer for, if it has text.
    fn text_type(&self) -> Option<(u32, &'static str)> {
        let offer = self.selection?;
        let types = self.offers.get(&offer)?;
        TEXT_TYPES
            .into_iter()
            .find(|wanted| types.iter().any(|offered| offered == wanted))
            .map(|mime| (offer, mime))
    }
}

/// Writes `bytes` to `fd` and closes it, ignoring a reader that leaves
/// early.
fn spawn_writer(fd: OwnedFd, bytes: Arc<[u8]>) {
    std::thread::spawn(move || {
        // A peer that closes its end early just does not get the rest.
        let _ = File::from(fd).write_all(&bytes);
    });
}

/// Reads text from `reader` to its end (at most [`MAX_TEXT_LEN`]), then
/// records it for `window` and wakes the loop.
fn spawn_reader(
    reader: std::io::PipeReader,
    window: WindowId,
    pastes: Pastes,
    waker: Arc<dyn Waker>,
) {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let text = reader
            .take(MAX_TEXT_LEN)
            .read_to_end(&mut bytes)
            .ok()
            .map(|_| String::from_utf8_lossy(&bytes).into_owned());
        pastes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((window, text));
        // The loop may be gone already; then nobody waits for the paste.
        let _ = waker.wake();
    });
}

impl WaylandBackend {
    /// Puts `content` on the clipboard for `window`'s client; without a
    /// seat or a data device manager there is no clipboard to put it on.
    pub(super) fn offer_clipboard(
        &mut self,
        window: WindowId,
        content: Content,
    ) -> Result<(), Error> {
        self.window(window)?;
        let (Some(device), Some(manager)) = (
            self.clipboard.device,
            self.globals.get(Interface::DataDeviceManager),
        ) else {
            return Ok(());
        };
        let serial = self.seat.as_ref().map_or(0, |seat| seat.serial);
        let source = self.conn.new_id(Interface::DataSource)?;
        self.conn.send(data::create_data_source(manager, source))?;
        for mime in content.types() {
            self.conn.send(data::offer(source, mime))?;
        }
        self.conn
            .send(data::set_selection(device, source, serial))?;
        if let Some((old, _)) = self.clipboard.source.replace((source, content)) {
            self.conn.send(data::destroy_source(old))?;
        }
        self.conn.flush()
    }

    /// Asks for the clipboard's text for `window`; it arrives later as
    /// [`WindowEvent::ClipboardText`].
    pub(super) fn request_clipboard(&mut self, window: WindowId) -> Result<(), Error> {
        self.window(window)?;
        let Some((offer, mime)) = self.clipboard.text_type() else {
            let event = WindowEvent::ClipboardText(None);
            self.pending.push(LoopEvent::Window { window, event });
            return Ok(());
        };
        let (reader, writer) =
            std::io::pipe().map_err(|error| Error::io("creating a pipe", &error))?;
        self.conn
            .send(data::receive(offer, mime, OwnedFd::from(writer)))?;
        self.conn.flush()?;
        let waker = Arc::clone(&self.waker) as Arc<dyn Waker>;
        spawn_reader(reader, window, Arc::clone(&self.clipboard.pastes), waker);
        Ok(())
    }

    /// Handles an event of the data device, an offer or the source.
    pub(super) fn clipboard_event(&mut self, object: u32, event: Event) -> Result<(), Error> {
        match event {
            Event::DataOffer(offer) => {
                self.clipboard.offers.insert(offer, Vec::new());
            }
            Event::Offer(mime) => {
                if let Some(types) = self.clipboard.offers.get_mut(&object) {
                    types.push(mime);
                }
            }
            Event::Selection(offer) => {
                let offer = (offer != 0).then_some(offer);
                if let Some(old) = self.clipboard.selection.take()
                    && Some(old) != offer
                {
                    self.drop_offer(old)?;
                }
                self.clipboard.selection = offer;
            }
            Event::DragEnter(offer) if offer != 0 => self.drop_offer(offer)?,
            Event::Send { mime, fd } => {
                let bytes = match &self.clipboard.source {
                    Some((source, content)) if *source == object => content.bytes(&mime),
                    _ => None,
                };
                if let Some(bytes) = bytes {
                    spawn_writer(fd, bytes);
                }
            }
            // Sources replaced earlier were destroyed then.
            Event::Cancelled
                if self
                    .clipboard
                    .source
                    .as_ref()
                    .is_some_and(|(source, _)| *source == object) =>
            {
                self.clipboard.source = None;
                self.conn.send(data::destroy_source(object))?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Destroys an offer this client does not use.
    fn drop_offer(&mut self, offer: u32) -> Result<(), Error> {
        self.clipboard.offers.remove(&offer);
        if self.conn.interface(offer) == Some(Interface::DataOffer) {
            self.conn
                .destroy_server_object(data::destroy_offer(offer), offer)?;
        }
        Ok(())
    }

    /// Reports the pastes that finished.
    pub(super) fn finished_pastes(&mut self, out: &mut Vec<LoopEvent>) {
        let pastes = std::mem::take(
            &mut *self
                .clipboard
                .pastes
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        for (window, text) in pastes {
            if self.window(window).is_ok() {
                let event = WindowEvent::ClipboardText(text);
                out.push(LoopEvent::Window { window, event });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn content_is_offered_by_type() {
        let text = Content::Text(Arc::from("é"));
        assert_eq!(text.types(), TEXT_TYPES);
        assert_eq!(text.bytes("UTF8_STRING").as_deref(), Some("é".as_bytes()));
        assert_eq!(text.bytes(PNG_TYPE), None);
        let image = Content::Image(Arc::from(&[1_u8, 2][..]));
        assert_eq!(image.types(), [PNG_TYPE]);
        assert_eq!(image.bytes(PNG_TYPE).as_deref(), Some(&[1_u8, 2][..]));
        assert_eq!(image.bytes("text/plain"), None);
    }

    #[test]
    fn the_preferred_text_type_is_asked_for() {
        let mut clipboard = Clipboard::default();
        assert_eq!(clipboard.text_type(), None);
        clipboard.offers.insert(
            0xff00_0001,
            vec![String::from("text/plain"), String::from("UTF8_STRING")],
        );
        clipboard.selection = Some(0xff00_0001);
        assert_eq!(clipboard.text_type(), Some((0xff00_0001, "UTF8_STRING")));
        clipboard
            .offers
            .insert(0xff00_0002, vec![String::from(PNG_TYPE)]);
        clipboard.selection = Some(0xff00_0002);
        assert_eq!(clipboard.text_type(), None);
    }

    /// A waker counting its wake-ups.
    #[derive(Debug, Default)]
    struct Count(AtomicUsize);

    impl Waker for Count {
        fn wake(&self) -> Result<(), Error> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn transfers_run_on_threads() {
        let (reader, writer) = std::io::pipe().unwrap();
        spawn_writer(OwnedFd::from(writer), Arc::from(&b"pasted"[..]));
        let pastes = Pastes::default();
        let waker = Arc::new(Count::default());
        let window = WindowId::from_raw(4);
        let wake = Arc::clone(&waker) as Arc<dyn Waker>;
        spawn_reader(reader, window, Arc::clone(&pastes), wake);
        let deadline = Instant::now() + Duration::from_secs(5);
        while waker.0.load(Ordering::SeqCst) == 0 {
            assert!(Instant::now() < deadline, "the paste finishes");
            std::thread::yield_now();
        }
        assert_eq!(
            *pastes.lock().unwrap(),
            [(window, Some(String::from("pasted")))]
        );
    }
}
