//! Live tests of the clipboard: another client pastes the image or text a
//! window put on it, and windows paste text from another client and from
//! themselves.

use std::time::{Duration, Instant};

use super::super::atoms::predefined;
use super::super::request;
use super::super::server_event::{SelectionRequest, ServerEvent};
use super::{Driver, is, live, open_window, pump_until, x11_loop, xid};
use crate::backend::Wait;
use crate::event::WindowEvent;
use crate::event_loop::EventLoop;
use crate::window::WindowOptions;

/// Asks for the clipboard as `target` through `driver`'s `requestor`
/// window and returns the answer, pumping `event_loop` so that the owner
/// answers; `None` when the owner refuses.
fn paste(
    driver: &mut Driver,
    event_loop: &mut EventLoop,
    requestor: u32,
    target: &str,
) -> Option<Vec<u8>> {
    let (clipboard, target, property) = (
        driver.atom("CLIPBOARD"),
        driver.atom(target),
        driver.atom("PLTRS_PASTE"),
    );
    driver.send(&request::convert_selection(
        requestor, clipboard, target, property, 0,
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline, "no answer to the paste");
        event_loop
            .pump_events(Some(Duration::ZERO), |_, _| {})
            .unwrap();
        let events = driver
            .conn
            .wait_for_events(Wait::Until(Instant::now() + Duration::from_millis(20)))
            .unwrap();
        // `SelectionNotify` (31), sent with `SendEvent`.
        if let Some(notify) = events.iter().find(|raw| raw[0] & 0x7f == 31) {
            let answered = u32::from_le_bytes([notify[20], notify[21], notify[22], notify[23]]);
            return (answered != 0).then(|| driver.property(requestor, property));
        }
    }
}

/// A hidden window of `driver`'s, to ask for and own selections.
fn driver_window(driver: &mut Driver) -> u32 {
    let id = driver.conn.generate_id().unwrap();
    let window = request::NewWindow {
        id,
        parent: driver.root,
        class: request::WindowClass::InputOnly,
        depth: 0,
        visual: 0,
        width: 1,
        height: 1,
    };
    driver.send(&request::create_window(&window, &[]));
    id
}

/// The 32-bit words of a property value.
fn words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|w| u32::from_le_bytes(*w))
        .collect()
}

#[test]
fn the_clipboard_hands_out_its_image() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    let requestor = driver_window(&mut driver);
    let mut event_loop = x11_loop();
    let owner = open_window(&mut event_loop, &WindowOptions::new("clipboard"));
    let png = b"\x89PNG\r\n\x1a\nnot really a PNG".to_vec();
    event_loop
        .context()
        .set_clipboard_image(owner, png.clone())
        .unwrap();

    let targets = paste(&mut driver, &mut event_loop, requestor, "TARGETS").expect("targets");
    assert_eq!(
        words(&targets),
        [driver.atom("TARGETS"), driver.atom("image/png")]
    );
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "image/png"),
        Some(png)
    );
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "UTF8_STRING"),
        None
    );
}

#[test]
fn the_clipboard_hands_out_its_text() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    let requestor = driver_window(&mut driver);
    let mut event_loop = x11_loop();
    let owner = open_window(&mut event_loop, &WindowOptions::new("clipboard"));
    event_loop
        .context()
        .set_clipboard_text(owner, "0.125 €".into())
        .unwrap();
    let targets = paste(&mut driver, &mut event_loop, requestor, "TARGETS").expect("targets");
    assert!(words(&targets).contains(&driver.atom("UTF8_STRING")));
    assert!(words(&targets).contains(&predefined::STRING));
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "UTF8_STRING"),
        Some("0.125 €".as_bytes().to_vec())
    );
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "STRING"),
        Some(b"0.125 ?".to_vec()),
        "Latin-1 has no euro sign"
    );
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "image/png"),
        None
    );
}

/// Pumps `event_loop` until `window` receives clipboard text.
fn pasted(event_loop: &mut EventLoop, window: crate::window::WindowId) -> Option<String> {
    let events = pump_until(event_loop, "the clipboard's text", |event| {
        matches!(
            event,
            crate::event::Event::Window {
                window: w,
                event: WindowEvent::ClipboardText(_),
            } if *w == window
        )
    });
    events.into_iter().find_map(|event| match event {
        crate::event::Event::Window {
            window: w,
            event: WindowEvent::ClipboardText(text),
        } if w == window => Some(text),
        _ => None,
    })?
}

#[test]
fn windows_paste_their_own_text_and_other_clients_text() {
    let Some(live) = live() else { return };
    let mut event_loop = x11_loop();
    let window = open_window(&mut event_loop, &WindowOptions::new("paste"));
    event_loop
        .context()
        .set_clipboard_text(window, "copied here".into())
        .unwrap();
    event_loop.context().request_clipboard_text(window).unwrap();
    assert_eq!(
        pasted(&mut event_loop, window).as_deref(),
        Some("copied here")
    );

    // Another client takes the clipboard, offering Latin-1 only.
    let mut driver = Driver::connect(&live.name);
    let owner = driver_window(&mut driver);
    let clipboard = driver.atom("CLIPBOARD");
    driver.send(&request::set_selection_owner(owner, clipboard, 0));
    event_loop.context().request_clipboard_text(window).unwrap();
    let mut answered = 0;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut text = None;
    while text.is_none() {
        assert!(Instant::now() < deadline, "no paste");
        let events = driver
            .conn
            .wait_for_events(Wait::Until(Instant::now() + Duration::from_millis(20)))
            .unwrap();
        for raw in &events {
            if let ServerEvent::SelectionRequest(asked) = ServerEvent::parse(raw) {
                // A desktop's clipboard bridge asks too: mutter mirrors
                // the X11 clipboard to Wayland clients.
                answered += usize::from(asked.requestor == xid(window));
                answer_in_latin1(&mut driver, asked);
            }
        }
        event_loop
            .pump_events(Some(Duration::ZERO), |_, event| {
                if is(
                    &event,
                    window,
                    &WindowEvent::ClipboardText(Some("café".into())),
                ) {
                    text = Some("café");
                }
            })
            .unwrap();
    }
    assert_eq!(answered, 2, "the window asks for UTF-8 first, then Latin-1");
}

/// Answers `asked` as an owner offering only `STRING`: "café".
fn answer_in_latin1(driver: &mut Driver, asked: SelectionRequest) {
    let property = if asked.target == predefined::STRING {
        let store =
            request::set_property_bytes(asked.requestor, asked.property, asked.target, b"caf\xe9");
        driver.send(&store);
        asked.property
    } else {
        0
    };
    let notify = request::selection_notify(
        asked.time,
        asked.requestor,
        asked.selection,
        asked.target,
        property,
    );
    driver.send(&request::send_event(asked.requestor, 0, &notify));
}
