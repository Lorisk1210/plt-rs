//! The platform-independent event loop, driven through the headless
//! backend: dispatch order, redraw coalescing, timers, wake-ups, exit,
//! presenting and errors.

use std::thread;
use std::time::{Duration, Instant};

use pltrs_platform::{
    ButtonState, Error, Event, EventLoop, Frame, Key, KeyEvent, LogicalSize, Modifiers, OwnedFrame,
    PhysicalRect, PhysicalSize, PumpStatus, WindowEvent, WindowId, WindowOptions,
};

fn options(width: u32, height: u32) -> WindowOptions {
    WindowOptions::new("test").with_inner_size(PhysicalSize::new(width, height))
}

/// Runs one non-blocking iteration and returns the dispatched events.
fn pump(event_loop: &mut EventLoop) -> Vec<Event> {
    let mut events = Vec::new();
    let status = event_loop.pump_events(Some(Duration::ZERO), |_, event| events.push(event));
    assert_eq!(status, Ok(PumpStatus::Continue));
    events
}

fn window_event(window: WindowId, event: WindowEvent) -> Event {
    Event::Window { window, event }
}

fn key_press(key: Key) -> WindowEvent {
    WindowEvent::Key(KeyEvent {
        key,
        state: ButtonState::Pressed,
        repeat: false,
        text: None,
        modifiers: Modifiers::NONE,
        scancode: 0,
    })
}

#[test]
fn started_comes_first_and_new_windows_get_a_redraw() {
    let (mut event_loop, _handle) = EventLoop::headless();
    let mut window = None;
    let mut events = Vec::new();
    event_loop
        .pump_events(Some(Duration::ZERO), |cx, event| {
            if event == Event::Started {
                window = Some(cx.create_window(&options(4, 3)).unwrap());
            }
            events.push(event);
        })
        .unwrap();
    let window = window.unwrap();
    assert_eq!(
        events,
        [
            Event::Started,
            window_event(window, WindowEvent::RedrawRequested),
            Event::AboutToWait
        ]
    );
    assert_eq!(
        pump(&mut event_loop),
        [Event::AboutToWait],
        "Started is sent once"
    );
}

#[test]
fn redraws_are_coalesced_and_follow_input() {
    let (mut event_loop, handle) = EventLoop::headless();
    let window = event_loop.context().create_window(&options(4, 3)).unwrap();
    pump(&mut event_loop);
    for _ in 0..3 {
        event_loop.context().request_redraw(window).unwrap();
    }
    handle.inject(window, WindowEvent::RedrawRequested).unwrap();
    handle
        .inject(window, WindowEvent::Resized(PhysicalSize::new(8, 6)))
        .unwrap();
    handle.inject(window, key_press(Key::Enter)).unwrap();
    assert_eq!(
        pump(&mut event_loop),
        [
            window_event(window, WindowEvent::Resized(PhysicalSize::new(8, 6))),
            window_event(window, key_press(Key::Enter)),
            window_event(window, WindowEvent::RedrawRequested),
            Event::AboutToWait,
        ]
    );
    assert_eq!(
        event_loop.context().inner_size(window),
        Ok(PhysicalSize::new(8, 6))
    );
}

#[test]
fn closing_drops_pending_events_and_ends_run() {
    let (mut event_loop, handle) = EventLoop::headless();
    let window = event_loop.context().create_window(&options(2, 2)).unwrap();
    handle.inject(window, WindowEvent::CloseRequested).unwrap();
    handle.inject(window, key_press(Key::Escape)).unwrap();
    let mut log = Vec::new();
    event_loop
        .run(|cx, event| {
            if let Event::Window { window, event } = event {
                if event == WindowEvent::CloseRequested {
                    cx.close_window(window).unwrap();
                }
                log.push(event);
            }
        })
        .unwrap();
    assert_eq!(log, [WindowEvent::CloseRequested, WindowEvent::Destroyed]);
    assert_eq!(handle.windows(), []);
}

#[test]
fn timers_fire_in_deadline_order_and_can_be_cancelled() {
    let (mut event_loop, _handle) = EventLoop::headless();
    let mut cx = event_loop.context();
    let late = cx.set_timeout(Duration::from_millis(100));
    let early = cx.set_timeout(Duration::from_millis(10));
    let cancelled = cx.set_timeout(Duration::from_millis(20));
    let interval = cx.set_interval(Duration::from_millis(40));
    assert!(cx.cancel_timer(cancelled));
    assert!(!cx.cancel_timer(cancelled));
    let mut fired = Vec::new();
    let deadline = Instant::now() + Duration::from_millis(150);
    let status = event_loop.run_until(deadline, |cx, event| {
        if let Event::Timer(id) = event {
            fired.push(id);
            if fired.iter().filter(|&&t| t == interval).count() == 3 {
                cx.cancel_timer(interval);
            }
        }
    });
    assert_eq!(status, Ok(PumpStatus::Continue));
    assert!(Instant::now() >= deadline);
    assert_eq!(fired, [early, interval, interval, late, interval]);
}

#[test]
fn pump_events_waits_at_most_the_timeout() {
    let (mut event_loop, _handle) = EventLoop::headless();
    let started = Instant::now();
    let status = event_loop.pump_events(Some(Duration::from_millis(30)), |_, _| {});
    assert_eq!(status, Ok(PumpStatus::Continue));
    let waited = started.elapsed();
    assert!(waited >= Duration::from_millis(25) && waited < Duration::from_secs(2));
}

#[test]
fn input_from_other_threads_wakes_a_blocking_run() {
    let (mut event_loop, handle) = EventLoop::headless();
    let window = event_loop.context().create_window(&options(2, 2)).unwrap();
    let proxy = event_loop.proxy();
    let injector = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        handle
            .inject(window, key_press(Key::Character('q')))
            .unwrap();
        thread::sleep(Duration::from_millis(20));
        proxy.wake_up().unwrap();
        proxy
    });
    let mut log = Vec::new();
    event_loop
        .run(|cx, event| match event {
            Event::Window {
                event: WindowEvent::Key(key),
                ..
            } => log.push(format!("key {}", key.key)),
            Event::WakeUp => {
                log.push(String::from("wake-up"));
                cx.close_window(window).unwrap();
            }
            _ => {}
        })
        .unwrap();
    assert_eq!(log, ["key q", "wake-up"]);
    let proxy = injector.join().unwrap();
    drop(event_loop);
    assert_eq!(proxy.wake_up(), Err(Error::EventLoopClosed));
}

#[test]
fn exit_stops_run_and_is_reported_by_pump() {
    let (mut event_loop, _handle) = EventLoop::headless();
    let window = event_loop.context().create_window(&options(2, 2)).unwrap();
    event_loop
        .run(|cx, event| {
            if let Event::Window {
                event: WindowEvent::RedrawRequested,
                ..
            } = event
            {
                cx.exit();
            }
        })
        .unwrap();
    assert_eq!(
        event_loop.context().windows(),
        [window],
        "exit leaves windows open"
    );
    let status = event_loop.pump_events(Some(Duration::ZERO), |cx, _| cx.exit());
    assert_eq!(status, Ok(PumpStatus::Exit));
    let status = event_loop.pump_events(Some(Duration::ZERO), |_, _| {});
    assert_eq!(
        status,
        Ok(PumpStatus::Continue),
        "the exit request was consumed"
    );
}

#[test]
fn presenting_composites_into_the_window() {
    let (mut event_loop, handle) = EventLoop::headless();
    let window = event_loop.context().create_window(&options(4, 4)).unwrap();
    let mut cx = event_loop.context();
    let red = OwnedFrame::filled(PhysicalSize::new(6, 6), [255, 0, 0, 255]).unwrap();
    cx.present(window, &red.as_frame()).unwrap();
    let green = OwnedFrame::filled(PhysicalSize::new(4, 4), [0, 255, 0, 128]).unwrap();
    cx.present_region(window, &green.as_frame(), PhysicalRect::new(1, 1, 2, 10))
        .unwrap();
    let frames = handle.take_frames();
    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0].region,
        PhysicalRect::new(0, 0, 4, 4),
        "clipped to the window"
    );
    assert_eq!(frames[1].region, PhysicalRect::new(1, 1, 2, 3));
    let surface = cx.read_pixels(window).unwrap();
    assert_eq!(surface, handle.surface(window).unwrap());
    assert_eq!(surface.pixel(0, 0), Some([255, 0, 0, 255]));
    assert_eq!(
        surface.pixel(2, 3),
        Some([0, 255, 0, 255]),
        "windows are opaque"
    );
    assert_eq!(surface.pixel(3, 3), Some([255, 0, 0, 255]));
    cx.present(window, &Frame::new(0, 0, &[]).unwrap()).unwrap();
    assert_eq!(handle.take_frames(), [], "empty frames present nothing");
}

#[test]
fn scale_factor_changes_resize_logical_windows() {
    let (mut event_loop, handle) = EventLoop::headless();
    handle.set_scale_factor(2.0).unwrap();
    let options = WindowOptions::new("hidpi").with_inner_size(LogicalSize::new(30.0, 20.0));
    let window = event_loop.context().create_window(&options).unwrap();
    assert_eq!(
        event_loop.context().inner_size(window),
        Ok(PhysicalSize::new(60, 40))
    );
    pump(&mut event_loop);
    handle.set_scale_factor(1.5).unwrap();
    assert_eq!(
        pump(&mut event_loop),
        [
            window_event(window, WindowEvent::ScaleFactorChanged(1.5)),
            window_event(window, WindowEvent::RedrawRequested),
            Event::AboutToWait,
        ]
    );
    assert_eq!(event_loop.context().scale_factor(window), Ok(1.5));
    assert!(handle.set_scale_factor(0.0).is_err());
    assert!(handle.set_scale_factor(f64::NAN).is_err());
}

#[test]
fn window_state_is_observable() {
    let (mut event_loop, handle) = EventLoop::headless();
    let options = options(10, 10)
        .with_app_id("demo")
        .with_min_inner_size(PhysicalSize::new(5, 5));
    let window = event_loop.context().create_window(&options).unwrap();
    let mut cx = event_loop.context();
    cx.set_title(window, "renamed").unwrap();
    cx.set_cursor(window, pltrs_platform::CursorIcon::Crosshair)
        .unwrap();
    let snapshot = handle.window(window).unwrap();
    assert_eq!(snapshot.title, "renamed");
    assert_eq!(snapshot.app_id, "demo");
    assert_eq!(snapshot.min_size, Some(PhysicalSize::new(5, 5)));
    assert_eq!(snapshot.cursor, pltrs_platform::CursorIcon::Crosshair);
}

#[test]
fn operations_on_closed_windows_fail() {
    let (mut event_loop, _handle) = EventLoop::headless();
    let mut cx = event_loop.context();
    let window = cx.create_window(&options(2, 2)).unwrap();
    assert!(matches!(
        cx.set_inner_size(window, PhysicalSize::new(0, 5)),
        Err(Error::InvalidArgument(_))
    ));
    cx.close_window(window).unwrap();
    let unknown = Err(Error::UnknownWindow(window));
    assert_eq!(cx.close_window(window), unknown);
    assert_eq!(cx.request_redraw(window), unknown);
    assert_eq!(cx.set_title(window, "x"), unknown);
    assert_eq!(cx.inner_size(window), Err(Error::UnknownWindow(window)));
    let empty = Frame::new(0, 0, &[]).unwrap();
    assert_eq!(cx.present(window, &empty), unknown);
    assert!(matches!(
        cx.create_window(&options(0, 2)),
        Err(Error::InvalidArgument(_))
    ));
}
