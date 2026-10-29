//! Live tests of window properties, window-manager messages and the scale
//! factor.

use std::time::{Duration, Instant};

use super::super::atoms::predefined;
use super::super::request::{self, WindowAttribute, event_mask, testing};
use super::super::server_event::ServerEvent;
use super::{Driver, ResetResources, is, live, of, open_window, pump_until, x11_loop, xid};
use crate::backend::Wait;
use crate::error::Error;
use crate::event::WindowEvent;
use crate::geometry::{LogicalSize, PhysicalSize};
use crate::window::{CursorIcon, Icon, WindowOptions};

#[test]
fn window_properties_follow_the_options() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    let mut event_loop = x11_loop();
    let icon = Icon::from_rgba(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 128]).unwrap();
    let options = WindowOptions::new("Figure 1 – α")
        .with_app_id("plt-live")
        .with_inner_size(PhysicalSize::new(320, 200))
        .with_min_inner_size(PhysicalSize::new(100, 80))
        .with_icon(icon);
    let window = open_window(&mut event_loop, &options);
    let id = xid(window);
    let net_wm_name = driver.atom("_NET_WM_NAME");
    assert_eq!(driver.property(id, net_wm_name), "Figure 1 – α".as_bytes());
    assert_eq!(driver.property(id, predefined::WM_NAME), b"Figure 1 ? ?");
    assert_eq!(
        driver.property(id, predefined::WM_CLASS),
        b"plt-live\0plt-live\0"
    );
    let protocols = driver.atom("WM_PROTOCOLS");
    let expected = [driver.atom("WM_DELETE_WINDOW"), driver.atom("_NET_WM_PING")];
    assert_eq!(driver.property_u32(id, protocols), expected);
    let hints = driver.property_u32(id, predefined::WM_NORMAL_HINTS);
    assert_eq!((hints[5], hints[6]), (100, 80));
    let net_wm_icon = driver.atom("_NET_WM_ICON");
    assert_eq!(
        driver.property_u32(id, net_wm_icon),
        [2, 1, 0xffff_0000, 0x8000_00ff]
    );
    let net_wm_pid = driver.atom("_NET_WM_PID");
    assert_eq!(driver.property_u32(id, net_wm_pid), [std::process::id()]);

    let mut cx = event_loop.context();
    cx.set_title(window, "Renamed").unwrap();
    for cursor in [
        CursorIcon::Crosshair,
        CursorIcon::Pointer,
        CursorIcon::Text,
        CursorIcon::Default,
    ] {
        cx.set_cursor(window, cursor).unwrap();
    }
    assert_eq!(driver.property(id, net_wm_name), b"Renamed");
    cx.close_window(window).unwrap();
    assert_eq!(
        cx.set_title(window, "gone"),
        Err(Error::UnknownWindow(window))
    );
    let events = pump_until(&mut event_loop, "destruction", |e| {
        is(e, window, &WindowEvent::Destroyed)
    });
    assert_eq!(of(&events, window).last(), Some(&WindowEvent::Destroyed));
}

#[test]
fn window_manager_messages_are_handled() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    let mut event_loop = x11_loop();
    let window = open_window(&mut event_loop, &WindowOptions::new("wm"));
    let (protocols, delete, ping) = (
        driver.atom("WM_PROTOCOLS"),
        driver.atom("WM_DELETE_WINDOW"),
        driver.atom("_NET_WM_PING"),
    );
    driver.client_message(xid(window), protocols, [delete, 0, 0, 0, 0]);
    pump_until(&mut event_loop, "a close request", |e| {
        is(e, window, &WindowEvent::CloseRequested)
    });

    let root = driver.root;
    let listen = WindowAttribute::EventMask(event_mask::SUBSTRUCTURE_NOTIFY);
    driver.send(&request::change_window_attributes(root, &[listen]));
    driver.client_message(xid(window), protocols, [ping, 4321, xid(window), 0, 0]);
    let deadline = Instant::now() + Duration::from_secs(5);
    let pong = loop {
        assert!(Instant::now() < deadline, "no answer to the ping");
        event_loop
            .pump_events(Some(Duration::ZERO), |_, _| {})
            .unwrap();
        let events = driver
            .conn
            .wait_for_events(Wait::Until(Instant::now() + Duration::from_millis(20)));
        let pong = events.unwrap().iter().map(ServerEvent::parse).find(|event| {
            matches!(event, ServerEvent::ClientMessage { window, data, .. } if *window == root && data[0] == ping)
        });
        if let Some(pong) = pong {
            break pong;
        }
    };
    assert_eq!(
        pong,
        ServerEvent::ClientMessage {
            window: root,
            message_type: protocols,
            data: [ping, 4321, xid(window), 0, 0],
        }
    );
}

#[test]
fn scale_factor_follows_xft_dpi() {
    let Some(live) = live() else { return };
    let mut driver = Driver::connect(&live.name);
    if !driver.is_bare_server() {
        return;
    }
    let _reset = ResetResources(&live.name);
    driver.set_resources("Xft.antialias:\t1\nXft.dpi:\t192\n");
    let mut event_loop = x11_loop();
    let options = WindowOptions::new("hidpi").with_inner_size(LogicalSize::new(200.0, 100.0));
    let window = open_window(&mut event_loop, &options);
    assert_eq!(event_loop.context().scale_factor(window), Ok(2.0));
    assert_eq!(
        event_loop.context().inner_size(window),
        Ok(PhysicalSize::new(400, 200))
    );
    driver.set_resources("Xft.dpi:\t144\n");
    pump_until(&mut event_loop, "the new scale", |e| {
        is(e, window, &WindowEvent::ScaleFactorChanged(1.5))
    });
    driver.send(&testing::delete_property(
        driver.root,
        predefined::RESOURCE_MANAGER,
    ));
    pump_until(&mut event_loop, "the default scale", |e| {
        is(e, window, &WindowEvent::ScaleFactorChanged(1.0))
    });
}
