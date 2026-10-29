//! The clipboard against the scripted compositor: copying text and
//! images, pasting another client's text, and losing the clipboard.

use std::io::Read;
use std::os::fd::OwnedFd;
use std::time::{Duration, Instant};

use super::super::fake::{Fake, Options};
use super::{expect, open, wait_requests};
use crate::backend::Backend;
use crate::event::WindowEvent;
use crate::wayland::wire::Request;

/// A deadline a little ahead, for polls reading what was sent already.
fn soon() -> Instant {
    Instant::now() + Duration::from_millis(20)
}

#[test]
fn copied_text_is_sent_to_whoever_asks() {
    let (mut backend, fake) = Fake::start(Options::default());
    let window = open(&mut backend);
    backend
        .set_clipboard_text(window, String::from("héllo"))
        .unwrap();
    wait_requests(&mut backend, &fake, "wl_data_device", 1, 1);
    let offered: Vec<String> = fake
        .state()
        .requests("wl_data_source", 0)
        .iter()
        .map(|r| {
            String::from_utf8_lossy(&r.body[4..])
                .trim_end_matches('\0')
                .to_owned()
        })
        .collect();
    assert_eq!(offered[0], "text/plain;charset=utf-8");
    assert_eq!(offered.len(), 4);
    let source = fake.state().requests("wl_data_device", 1)[0].word(0);
    let (mut reader, writer) = std::io::pipe().unwrap();
    fake.send(
        Request::new(source, 1)
            .string("UTF8_STRING")
            .fd(OwnedFd::from(writer)),
    );
    backend.poll(Some(soon()), &mut Vec::new()).unwrap();
    let mut text = String::new();
    reader.read_to_string(&mut text).unwrap();
    assert_eq!(text, "héllo");

    backend
        .set_clipboard_image(window, vec![0x89, b'P', b'N', b'G'])
        .unwrap();
    wait_requests(&mut backend, &fake, "wl_data_source", 1, 1);
    let destroyed = fake.state().requests("wl_data_source", 1)[0].object;
    assert_eq!(destroyed, source, "the text's source is replaced");
    let image = fake.state().requests("wl_data_device", 1)[1].word(0);
    fake.send(Request::new(image, 2));
    backend.poll(Some(soon()), &mut Vec::new()).unwrap();
    wait_requests(&mut backend, &fake, "wl_data_source", 1, 2);
}

#[test]
fn pasting_reads_another_clients_text() {
    let (mut backend, fake) = Fake::start(Options::default());
    let window = open(&mut backend);
    backend.request_clipboard_text(window).unwrap();
    expect(&mut backend, window, &WindowEvent::ClipboardText(None));
    fake.offer_clipboard(&["image/png", "text/plain", "UTF8_STRING"]);
    // The offer is on its way already; one poll reads it.
    backend.poll(Some(soon()), &mut Vec::new()).unwrap();
    backend.request_clipboard_text(window).unwrap();
    let text = WindowEvent::ClipboardText(Some(String::from("pasted as UTF8_STRING")));
    expect(&mut backend, window, &text);
    let offer = fake.offer_clipboard(&["image/png"]);
    backend.poll(Some(soon()), &mut Vec::new()).unwrap();
    wait_requests(&mut backend, &fake, "wl_data_offer", 2, 1);
    assert_ne!(
        fake.state().requests("wl_data_offer", 2)[0].object,
        offer,
        "the old offer goes"
    );
    backend.request_clipboard_text(window).unwrap();
    expect(&mut backend, window, &WindowEvent::ClipboardText(None));
}
