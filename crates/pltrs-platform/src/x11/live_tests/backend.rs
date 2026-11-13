//! Live tests of the X11 backend used directly: frame upload in tiles,
//! X errors and argument checks.

use std::time::{Duration, Instant};

use super::super::X11Backend;
use super::super::connection::Connection;
use super::super::cursor::INHERIT;
use super::super::request;
use super::live;
use crate::backend::Backend;
use crate::blocking::Blocking;
use crate::error::{Error, X11Error};
use crate::event::{Event, WindowEvent};
use crate::frame::OwnedFrame;
use crate::geometry::{PhysicalRect, PhysicalSize};
use crate::window::{CursorIcon, WindowOptions};

#[test]
fn large_frames_are_split_into_requests_the_server_accepts() {
    let Some(live) = live() else { return };
    let mut backend = X11Backend::open(&live.name).unwrap();
    backend.conn.limit_request_bytes(16 * 1024);
    let window = backend
        .create_window(&WindowOptions::new("tiles").with_inner_size(PhysicalSize::new(203, 157)))
        .unwrap();
    let mut events = Vec::new();
    while !events.contains(&Event::Window {
        window,
        event: WindowEvent::RedrawRequested,
    }) {
        backend
            .poll(
                Some(Instant::now() + Duration::from_secs(5)),
                &mut events,
                Blocking::DIRECT,
            )
            .unwrap();
    }
    // The size shown, which a window manager may have rounded: mutter
    // makes it even when it scales Xwayland's windows by 2.
    let size = backend.inner_size(window).unwrap();
    let pixels: Vec<u8> = (0..size.area() as u32)
        .flat_map(|i| [(i % 251) as u8, (i % 241) as u8, (i % 239) as u8, 255])
        .collect();
    let frame = OwnedFrame::new(size.width, size.height, pixels).unwrap();
    backend
        .present(window, &frame.as_frame(), PhysicalRect::from_size(size))
        .unwrap();
    assert_eq!(backend.read_pixels(window).unwrap(), frame);
}

#[test]
fn cursors_are_drawn_at_the_cursor_size() {
    let Some(live) = live() else { return };
    let mut backend = X11Backend::open(&live.name).unwrap();
    let (root, conn) = (backend.root, &mut backend.conn);
    let cursors = &mut backend.cursors;
    let cross = cursors.get(conn, root, CursorIcon::Crosshair, 48).unwrap();
    assert_ne!(cross, INHERIT);
    assert_eq!(
        cursors.get(conn, root, CursorIcon::Crosshair, 48).unwrap(),
        cross,
        "made once"
    );
    let larger = cursors.get(conn, root, CursorIcon::Crosshair, 72).unwrap();
    assert_ne!(larger, cross);
    for icon in [CursorIcon::Pointer, CursorIcon::Wait, CursorIcon::Move] {
        assert_ne!(cursors.get(conn, root, icon, 300).unwrap(), INHERIT);
    }
    assert_eq!(
        cursors.get(conn, root, CursorIcon::Default, 48).unwrap(),
        INHERIT
    );
    assert!(cursors.draws(), "X servers have RENDER");
    let mut events = Vec::new();
    backend
        .poll(
            Some(Instant::now() + Duration::from_millis(50)),
            &mut events,
            Blocking::DIRECT,
        )
        .unwrap();
}

#[test]
fn x_errors_become_typed_errors() {
    let Some(live) = live() else { return };
    let mut conn = Connection::connect(&live.name).unwrap();
    let unused = conn.generate_id().unwrap();
    let checked = conn.send_checked(&request::destroy_window(unused));
    assert!(
        matches!(checked, Err(Error::X11(X11Error { code: 3, major_opcode: 4, bad_value, .. })) if bad_value == unused),
        "{checked:?}"
    );

    let mut backend = X11Backend::open(&live.name).unwrap();
    let unused = backend.conn.generate_id().unwrap();
    backend.conn.send(&request::map_window(unused)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let polled = backend.poll(Some(deadline), &mut Vec::new(), Blocking::DIRECT);
    assert!(
        matches!(polled, Err(Error::X11(X11Error { code: 3, major_opcode: 8, bad_value, .. })) if bad_value == unused),
        "{polled:?}"
    );
}

#[test]
fn invalid_arguments_and_unknown_windows_are_rejected() {
    let Some(live) = live() else { return };
    let mut backend = X11Backend::open(&live.name).unwrap();
    let too_big = WindowOptions::new("huge").with_inner_size(PhysicalSize::new(70_000, 10));
    assert!(matches!(
        backend.create_window(&too_big),
        Err(Error::InvalidArgument(_))
    ));
    let window = backend.create_window(&WindowOptions::new("ok")).unwrap();
    assert_eq!(backend.window_ids(), [window]);
    backend.destroy_window(window).unwrap();
    assert!(!backend.has_window(window));
    assert_eq!(
        backend.destroy_window(window),
        Err(Error::UnknownWindow(window))
    );
}
