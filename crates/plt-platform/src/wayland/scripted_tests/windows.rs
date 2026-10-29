//! Windows against the scripted compositor: configures, decoration modes
//! and scales.

use std::time::{Duration, Instant};

use super::super::fake::{Fake, Options};
use super::{expect, open, poll_until, present, surface, wait_requests};
use crate::backend::Backend;
use crate::event::{Event, WindowEvent};
use crate::geometry::PhysicalSize;
use crate::wayland::decoration::layout::TITLE_BAR_HEIGHT;
use crate::wayland::wire::Request;

/// `xdg_toplevel.state.maximized` and `activated`, as an array.
fn states(states: &[u32]) -> Vec<u8> {
    states
        .iter()
        .flat_map(|state| state.to_ne_bytes())
        .collect()
}

#[test]
fn configures_resize_and_restore_the_window() {
    let (mut backend, fake) = Fake::start(Options::default());
    let window = open(&mut backend);
    assert_eq!(backend.inner_size(window), Ok(PhysicalSize::new(320, 200)));
    let (xdg_surface, toplevel) = fake.state().toplevel(surface(window));
    let acks = fake.state().requests("xdg_surface", 4).len();
    fake.send(
        Request::new(toplevel, 0)
            .int(800)
            .int(600)
            .array(&states(&[1, 4])),
    );
    fake.send(Request::new(xdg_surface, 0).uint(77));
    let height = 600 - TITLE_BAR_HEIGHT;
    expect(
        &mut backend,
        window,
        &WindowEvent::Resized(PhysicalSize::new(800, height)),
    );
    wait_requests(&mut backend, &fake, "xdg_surface", 4, acks + 1);
    assert_eq!(
        fake.state()
            .requests("xdg_surface", 4)
            .last()
            .unwrap()
            .word(0),
        77
    );
    fake.send(Request::new(toplevel, 0).int(0).int(0).array(&[]));
    fake.send(Request::new(xdg_surface, 0).uint(78));
    expect(
        &mut backend,
        window,
        &WindowEvent::Resized(PhysicalSize::new(320, 200)),
    );
    fake.send(Request::new(toplevel, 1));
    expect(&mut backend, window, &WindowEvent::CloseRequested);
    fake.send(Request::new(toplevel, 0).int(i32::MAX).int(300).array(&[]));
    fake.send(Request::new(xdg_surface, 0).uint(79));
    let cut = PhysicalSize::new(8192, 300 - TITLE_BAR_HEIGHT);
    expect(&mut backend, window, &WindowEvent::Resized(cut));
}

#[test]
fn the_application_resizes_floating_windows_only() {
    let (mut backend, fake) = Fake::start(Options::default());
    let window = open(&mut backend);
    backend
        .set_inner_size(window, PhysicalSize::new(300, 150).into())
        .unwrap();
    expect(
        &mut backend,
        window,
        &WindowEvent::Resized(PhysicalSize::new(300, 150)),
    );
    let (xdg_surface, toplevel) = fake.state().toplevel(surface(window));
    fake.send(
        Request::new(toplevel, 0)
            .int(900)
            .int(700)
            .array(&states(&[1])),
    );
    fake.send(Request::new(xdg_surface, 0).uint(5));
    let maximized = PhysicalSize::new(900, 700 - TITLE_BAR_HEIGHT);
    expect(&mut backend, window, &WindowEvent::Resized(maximized));
    backend
        .set_inner_size(window, PhysicalSize::new(100, 100).into())
        .unwrap();
    assert_eq!(
        backend.inner_size(window),
        Ok(maximized),
        "the compositor decides"
    );
}

#[test]
fn server_side_decorations_replace_the_title_bar() {
    let options = Options {
        globals: vec![("zxdg_decoration_manager_v1", 1)],
        decoration_mode: 2,
        ..Options::default()
    };
    let (mut backend, fake) = Fake::start(options);
    let window = open(&mut backend);
    assert!(backend.windows[&surface(window)].decorations.is_none());
    let mode = fake.state().requests("zxdg_toplevel_decoration_v1", 1)[0].word(0);
    assert_eq!(mode, 2, "server-side asked for");

    let options = Options {
        globals: vec![("zxdg_decoration_manager_v1", 1)],
        decoration_mode: 1,
        ..Options::default()
    };
    let (mut backend, _fake) = Fake::start(options);
    let window = open(&mut backend);
    assert!(
        backend.windows[&surface(window)].decorations.is_some(),
        "client-side imposed"
    );
}

#[test]
fn integer_scales_multiply_the_buffer() {
    let options = Options {
        output_scale: 2,
        ..Options::default()
    };
    let (mut backend, fake) = Fake::start(options);
    let window = open(&mut backend);
    assert_eq!(backend.scale_factor(window), Ok(2.0));
    assert_eq!(backend.inner_size(window), Ok(PhysicalSize::new(640, 400)));
    wait_requests(&mut backend, &fake, "wl_surface", 8, 2);
    let content = fake
        .state()
        .requests("wl_surface", 8)
        .into_iter()
        .find(|r| r.object == surface(window))
        .map(|r| r.word(0));
    assert_eq!(content, Some(2));
    let output = fake.state().id_of("wl_output").unwrap();
    fake.send(Request::new(surface(window), 0).uint(output));
    fake.send(Request::new(surface(window), 2).int(3));
    let events = expect(&mut backend, window, &WindowEvent::ScaleFactorChanged(3.0));
    assert!(events.contains(&WindowEvent::Resized(PhysicalSize::new(960, 600))));
}

#[test]
fn fractional_scales_go_through_a_viewport() {
    let options = Options {
        globals: vec![("wp_viewporter", 1), ("wp_fractional_scale_manager_v1", 1)],
        fractional: 180,
        ..Options::default()
    };
    let (mut backend, fake) = Fake::start(options);
    let window = open(&mut backend);
    assert_eq!(backend.scale_factor(window), Ok(1.5));
    assert_eq!(backend.inner_size(window), Ok(PhysicalSize::new(480, 300)));
    wait_requests(&mut backend, &fake, "wp_viewport", 2, 2);
    let destinations: Vec<(u32, u32)> = fake
        .state()
        .requests("wp_viewport", 2)
        .iter()
        .map(|r| (r.word(0), r.word(1)))
        .collect();
    assert!(destinations.contains(&(320, 200)), "{destinations:?}");
    let scale = fake.state().id_of("wp_fractional_scale_v1").unwrap();
    fake.send(Request::new(scale, 0).uint(240));
    let events = expect(&mut backend, window, &WindowEvent::ScaleFactorChanged(2.0));
    assert!(events.contains(&WindowEvent::Resized(PhysicalSize::new(640, 400))));
}

#[test]
fn wake_ups_arrive_once_and_not_at_start() {
    let (mut backend, _fake) = Fake::start(Options::default());
    let mut events = Vec::new();
    let soon = Instant::now() + Duration::from_millis(50);
    backend.poll(Some(soon), &mut events).unwrap();
    assert!(!events.contains(&Event::WakeUp), "{events:?}");
    let waker = backend.waker();
    std::thread::spawn(move || {
        waker.wake().unwrap();
        waker.wake().unwrap();
    })
    .join()
    .unwrap();
    let events = poll_until(&mut backend, "the wake-up", |events| {
        events.contains(&Event::WakeUp)
    });
    assert_eq!(events.iter().filter(|e| **e == Event::WakeUp).count(), 1);
}

#[test]
fn raising_asks_for_activation() {
    let options = Options {
        globals: vec![("wl_seat", 8), ("xdg_activation_v1", 1)],
        ..Options::default()
    };
    let (mut backend, fake) = Fake::start(options);
    let window = open(&mut backend);
    backend.raise_window(window).unwrap();
    poll_until(&mut backend, "the activation", |_| {
        !fake.state().requests("xdg_activation_v1", 2).is_empty()
    });
    let activate = fake.state().requests("xdg_activation_v1", 2)[0].clone();
    assert_eq!(&activate.body[4..11], b"token-1");
    present(&mut backend, window);
    backend.destroy_window(window).unwrap();
}
