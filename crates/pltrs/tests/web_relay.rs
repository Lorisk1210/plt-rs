//! The web relay end to end: a relay on a free local port, a browser
//! played by a minimal HTTP and WebSocket client, and the program's side
//! of the protocol played by the test: what the client sends arrives as
//! it came, what the program sends arrives as it was sent.

use std::thread;
use std::time::{Duration, Instant};

#[path = "support/browser.rs"]
#[allow(
    dead_code,
    reason = "the relay's tests need no help with matplotlib's messages"
)]
mod browser;

use browser::{Incoming, PATIENCE, Socket, get};
use pltrs::web::{RelayEvent, WebOptions, WebRelay};

/// The client script the relay serves.
const SCRIPT: &str = "/* the program's client script */";

/// A relay serving figure 1 on a free port.
fn relay() -> WebRelay {
    let relay =
        WebRelay::bind(&WebOptions::new("127.0.0.1", 0), SCRIPT.to_owned()).expect("a free port");
    relay.set_figures(&[(1, false)]);
    relay
}

/// Polls `relay` until `wanted` picks an event, failing after a while.
fn poll_until<T>(relay: &mut WebRelay, mut wanted: impl FnMut(RelayEvent) -> Option<T>) -> T {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline {
        for event in relay.poll(Some(Duration::from_millis(50))) {
            if let Some(found) = wanted(event) {
                return found;
            }
        }
    }
    panic!("the relay did not report what was expected");
}

#[test]
fn pages_and_the_script_follow_the_program() {
    let relay = relay();
    let addr = relay.local_addr();
    let page = get(addr, "/1");
    assert_eq!(page.status, 200);
    assert!(page.text().contains(&format!("ws://{addr}/")));
    assert_eq!(get(addr, "/2").status, 404, "a figure not served");
    assert_eq!(get(addr, "/js/mpl.js").text(), SCRIPT);
    assert_eq!(get(addr, "/_static/css/mpl.css").status, 200);
    relay.set_figures(&[(1, false), (2, true)]);
    assert_eq!(get(addr, "/2").status, 200);
    relay.set_figures(&[]);
    assert_eq!(get(addr, "/1").status, 404, "closed");
}

#[test]
fn websocket_messages_go_both_ways_as_they_come() {
    let mut relay = relay();
    let mut socket = Socket::open(relay.local_addr(), 1);
    let connection = poll_until(&mut relay, |event| match event {
        RelayEvent::Opened { connection, figure } => {
            assert_eq!(figure, 1);
            Some(connection)
        }
        _ => None,
    });
    let message = r#"{"type": "send_image_mode", "figure_id": "1"}"#;
    socket.send(message);
    let text = poll_until(&mut relay, |event| match event {
        RelayEvent::Message {
            connection: c,
            text,
        } if c == connection => Some(text),
        _ => None,
    });
    assert_eq!(text, message, "as it came, unparsed");
    socket.send("not even JSON");
    let raw = poll_until(&mut relay, |event| match event {
        RelayEvent::Message { text, .. } => Some(text),
        _ => None,
    });
    assert_eq!(raw, "not even JSON", "the program judges its messages");
    assert!(relay.send_text(connection, r#"{"type": "draw"}"#));
    assert_eq!(socket.next(), Incoming::Text(r#"{"type": "draw"}"#.into()));
    assert!(relay.send_binary(connection, b"\x89PNG frame"));
    assert_eq!(socket.next(), Incoming::Binary(b"\x89PNG frame".to_vec()));
    relay.close(connection);
    assert_eq!(socket.next(), Incoming::Close(Some(1001)));
    assert!(!relay.send_text(connection, "gone"));
}

#[test]
fn a_socket_the_client_closes_is_reported() {
    let mut relay = relay();
    let socket = Socket::open(relay.local_addr(), 1);
    let connection = poll_until(&mut relay, |event| match event {
        RelayEvent::Opened { connection, .. } => Some(connection),
        _ => None,
    });
    drop(socket);
    poll_until(&mut relay, |event| {
        (event == RelayEvent::Closed { connection }).then_some(())
    });
    assert!(!relay.send_text(connection, "gone"));
}

#[test]
fn downloads_wait_for_the_programs_file() {
    let mut relay = relay();
    let addr = relay.local_addr();
    let saved = thread::spawn(move || get(addr, "/1/download.svg"));
    let (request, format) = poll_until(&mut relay, |event| match event {
        RelayEvent::Download {
            request,
            figure: 1,
            format,
        } => Some((request, format)),
        _ => None,
    });
    assert_eq!(format, "svg");
    relay.answer_download(request, Some(b"<svg/>".to_vec()));
    let reply = saved.join().expect("the download returns");
    assert_eq!(reply.status, 200);
    assert_eq!(reply.header("content-type"), Some("image/svg+xml"));
    assert_eq!(reply.body, b"<svg/>");

    let refused = thread::spawn(move || get(addr, "/1/download.pdf"));
    let request = poll_until(&mut relay, |event| match event {
        RelayEvent::Download { request, .. } => Some(request),
        _ => None,
    });
    relay.answer_download(request, None);
    assert_eq!(refused.join().expect("the download returns").status, 404);
}
