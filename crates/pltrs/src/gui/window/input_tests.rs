//! Headless tests of the figure events a window gives its figure: those
//! matplotlib's Qt backend gives for the same input (see
//! [`crate::gui::input`]).

use std::cell::RefCell;
use std::rc::Rc;

use pltrs_core::Point;
use pltrs_platform::{
    ButtonState, EventLoop, HeadlessHandle, Key, KeyEvent, Modifiers, MouseButton,
    PhysicalPosition, ScrollDelta,
};

use super::tests::pump;
use super::*;
use crate::events::EventKind;

/// A shown 4 × 3 in figure with one axes, whose figure events are
/// recorded.
fn shown() -> (
    EventLoop,
    HeadlessHandle,
    FigureWindow,
    Rc<RefCell<Vec<Event>>>,
) {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    figure.subplot();
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    pump(&mut event_loop, &mut window);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let kinds = [
        EventKind::Motion,
        EventKind::ButtonPress,
        EventKind::ButtonRelease,
        EventKind::Scroll,
        EventKind::KeyPress,
        EventKind::KeyRelease,
        EventKind::AxesEnter,
        EventKind::AxesLeave,
        EventKind::FigureEnter,
        EventKind::FigureLeave,
    ];
    for kind in kinds {
        let log = Rc::clone(&seen);
        figure.connect(kind, move |event| log.borrow_mut().push(event.clone()));
    }
    (event_loop, handle, window, seen)
}

fn at(x: f64, y: f64) -> PhysicalPosition {
    PhysicalPosition::new(x, y)
}

fn moved(position: PhysicalPosition) -> WindowEvent {
    WindowEvent::CursorMoved {
        position,
        modifiers: Modifiers::NONE,
    }
}

fn button(state: ButtonState, position: PhysicalPosition) -> WindowEvent {
    WindowEvent::MouseInput {
        button: MouseButton::Left,
        state,
        position,
        modifiers: Modifiers::NONE,
        click_count: 1,
    }
}

fn key(key: Key, state: ButtonState, modifiers: Modifiers) -> WindowEvent {
    WindowEvent::Key(KeyEvent {
        key,
        state,
        repeat: false,
        text: None,
        modifiers,
        scancode: 0,
    })
}

/// Hands `events` to the window and returns the figure events they made.
fn feed(
    event_loop: &mut EventLoop,
    handle: &HeadlessHandle,
    window: &mut FigureWindow,
    seen: &RefCell<Vec<Event>>,
    events: Vec<WindowEvent>,
) -> Vec<Event> {
    for event in events {
        handle
            .inject(window.id(), event)
            .expect("the window exists");
    }
    pump(event_loop, window);
    seen.borrow_mut().drain(..).collect()
}

fn kinds(events: &[Event]) -> Vec<EventKind> {
    events.iter().map(Event::kind).collect()
}

#[test]
fn leaving_with_a_button_held_leaves_at_the_release_where_the_pointer_went() {
    let (mut event_loop, handle, mut window, seen) = shown();
    let inside = at(200.0, 150.0);
    let events = feed(
        &mut event_loop,
        &handle,
        &mut window,
        &seen,
        vec![
            WindowEvent::CursorEntered { position: inside },
            moved(inside),
            moved(inside),
            button(ButtonState::Pressed, inside),
            WindowEvent::CursorLeft {
                position: Some(at(450.0, 150.0)),
            },
            moved(at(460.0, 150.0)),
            button(ButtonState::Released, at(460.0, 150.0)),
        ],
    );
    assert_eq!(
        kinds(&events),
        [
            EventKind::FigureEnter,
            EventKind::AxesEnter,
            EventKind::Motion,
            EventKind::ButtonPress,
            EventKind::AxesLeave,
            EventKind::Motion,
            EventKind::ButtonRelease,
            EventKind::FigureLeave,
        ],
        "one motion where the pointer stayed; the leave after the release"
    );
    let left = events[7].location().expect("a leave has a location");
    assert_eq!((left.x, left.y), (460.0, 150.0));
    assert!(left.inaxes.is_none());
    // Keys typed meanwhile report where the pointer is, outside.
    let events = feed(
        &mut event_loop,
        &handle,
        &mut window,
        &seen,
        vec![key(
            Key::Character('a'),
            ButtonState::Pressed,
            Modifiers::NONE,
        )],
    );
    let location = events[0].location().expect("keys have a location");
    assert_eq!(Point::new(location.x, location.y), Point::new(460.0, 150.0));
}

#[test]
fn keys_wheels_and_leaves_are_named_and_counted_as_in_qt() {
    let (mut event_loop, handle, mut window, seen) = shown();
    let inside = at(200.0, 150.0);
    let wheel = |delta, modifiers| WindowEvent::MouseWheel {
        delta,
        position: inside,
        modifiers,
    };
    let events = feed(
        &mut event_loop,
        &handle,
        &mut window,
        &seen,
        vec![
            WindowEvent::CursorEntered { position: inside },
            key(Key::Shift, ButtonState::Pressed, Modifiers::NONE),
            key(Key::Shift, ButtonState::Released, Modifiers::SHIFT),
            key(Key::Character('d'), ButtonState::Pressed, Modifiers::SUPER),
            wheel(ScrollDelta::Lines { x: 1.0, y: 0.0 }, Modifiers::NONE),
            wheel(ScrollDelta::Lines { x: 0.0, y: -1.0 }, Modifiers::NONE),
            WindowEvent::CursorLeft { position: None },
        ],
    );
    let names: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            Event::KeyPress(key) | Event::KeyRelease(key) => Some(key.key.as_str()),
            _ => None,
        })
        .collect();
    let super_d = if cfg!(target_os = "macos") {
        "cmd+d"
    } else {
        "meta+d"
    };
    assert_eq!(names, ["shift", "shift", super_d]);
    let steps: Vec<f64> = events
        .iter()
        .filter_map(|event| match event {
            Event::Scroll(mouse) => Some(mouse.step),
            _ => None,
        })
        .collect();
    assert_eq!(steps, [-1.0], "a sideways wheel is no event");
    let Some(Event::FigureLeave(left)) = events.last() else {
        panic!("the pointer left: {events:?}");
    };
    assert_eq!((left.x, left.y), (200.0, 150.0), "where it was last seen");
    assert!(
        !kinds(&events).contains(&EventKind::AxesLeave),
        "the axes are left at the next motion over the figure"
    );
}
