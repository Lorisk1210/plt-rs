//! Live test of the clipboard: another client pastes the image a window
//! put on it.

use std::time::{Duration, Instant};

use super::super::request::{self, testing};
use super::super::transport::Wait;
use super::{Driver, live, open_window, x11_loop};
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
        driver.atom("PLT_PASTE"),
    );
    driver.send(&testing::convert_selection(
        requestor, clipboard, target, property,
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

#[test]
fn the_clipboard_hands_out_its_image() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    let requestor = driver.conn.generate_id().unwrap();
    let root = driver.root;
    let window = request::NewWindow {
        id: requestor,
        parent: root,
        class: request::WindowClass::InputOnly,
        depth: 0,
        visual: 0,
        width: 1,
        height: 1,
    };
    driver.send(&request::create_window(&window, &[]));
    let mut event_loop = x11_loop();
    let owner = open_window(&mut event_loop, &WindowOptions::new("clipboard"));
    let png = b"\x89PNG\r\n\x1a\nnot really a PNG".to_vec();
    event_loop
        .context()
        .set_clipboard_image(owner, png.clone())
        .unwrap();

    let targets = paste(&mut driver, &mut event_loop, requestor, "TARGETS").expect("targets");
    let words: Vec<u32> = targets
        .as_chunks::<4>()
        .0
        .iter()
        .map(|w| u32::from_le_bytes(*w))
        .collect();
    assert_eq!(words, [driver.atom("TARGETS"), driver.atom("image/png")]);
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "image/png"),
        Some(png)
    );
    assert_eq!(
        paste(&mut driver, &mut event_loop, requestor, "UTF8_STRING"),
        None
    );
}
