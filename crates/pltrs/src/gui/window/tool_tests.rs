//! Headless tests of the tools programs add to figure windows: radio
//! groups shared with the pan and zoom tools, and cursors.

use std::cell::RefCell;
use std::rc::Rc;

use pltrs_platform::{EventLoop, HeadlessHandle, Modifiers, PhysicalPosition};

use super::tests::{key, pump};
use super::*;
use crate::gui::{NAVIGATION_GROUP, ToolKind, ToolSpec};

/// Moves the pointer of window `id` to `(x, y)` from its top-left corner.
fn move_to(handle: &HeadlessHandle, id: WindowId, (x, y): (f64, f64)) {
    let moved = WindowEvent::CursorMoved {
        position: PhysicalPosition::new(x, y),
        modifiers: Modifiers::NONE,
    };
    handle.inject(id, moved).expect("the window exists");
}

#[test]
fn a_tool_of_the_navigation_group_takes_turns_with_pan_and_zoom() {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    figure.subplot();
    let uses = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&uses);
    let spec = ToolSpec {
        name: "lasso".into(),
        keys: vec!["m".into()],
        kind: ToolKind::Toggle,
        radio_group: Some(NAVIGATION_GROUP.into()),
        cursor: Some(CursorShape::Hand),
        ..ToolSpec::default()
    };
    figure
        .add_tool(spec, move |event| log.borrow_mut().push(event.toggled))
        .expect("the tool is new");
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    pump(&mut event_loop, &mut window);
    let id = window.id();

    key(&handle, id, 'p');
    key(&handle, id, 'm');
    pump(&mut event_loop, &mut window);
    assert_eq!(window.navigation.mode(), NavMode::None, "pan gave way");
    assert_eq!(figure.tool_toggled("lasso"), Some(true));
    move_to(&handle, id, (200.0, 150.0));
    pump(&mut event_loop, &mut window);
    let cursor = |handle: &HeadlessHandle| handle.window(id).expect("open").cursor;
    assert_eq!(
        cursor(&handle),
        CursorIcon::Pointer,
        "the hand over the axes"
    );
    move_to(&handle, id, (2.0, 2.0));
    pump(&mut event_loop, &mut window);
    assert_eq!(cursor(&handle), CursorIcon::Default, "the arrow elsewhere");

    key(&handle, id, 'o');
    pump(&mut event_loop, &mut window);
    assert_eq!(window.navigation.mode(), NavMode::Zoom);
    assert_eq!(figure.tool_toggled("lasso"), Some(false), "zoom took over");
    assert_eq!(*uses.borrow(), [true, false]);

    // Switched on by the program, the tool takes over once the window
    // refreshes.
    figure.trigger_tool("lasso");
    window
        .refresh(&mut event_loop.context())
        .expect("the window refreshes");
    assert_eq!(window.navigation.mode(), NavMode::None);
    assert!(
        figure.widgets_available(crate::events::LockOwner::new()),
        "the zoom tool released the widget lock"
    );
}

#[test]
fn tools_starting_on_are_on_in_new_windows() {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    figure.subplot();
    let spec = ToolSpec {
        name: "mark".into(),
        kind: ToolKind::Toggle,
        default_toggled: true,
        ..ToolSpec::default()
    };
    figure.add_tool(spec, |_| {}).expect("the tool is new");
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    pump(&mut event_loop, &mut window);
    assert_eq!(figure.tool_toggled("mark"), Some(true));
    key(&handle, window.id(), 'p');
    pump(&mut event_loop, &mut window);
    assert_eq!(window.navigation.mode(), NavMode::Pan, "in no radio group");
    assert_eq!(figure.tool_toggled("mark"), Some(true));
}

#[test]
fn tool_messages_show_in_the_status_line() {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    figure.subplot();
    let spec = ToolSpec {
        name: "note".into(),
        keys: vec!["m".into()],
        ..ToolSpec::default()
    };
    figure
        .add_tool(spec, |event| event.figure.tool_message("noted"))
        .expect("the tool is new");
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    pump(&mut event_loop, &mut window);
    key(&handle, window.id(), 'm');
    pump(&mut event_loop, &mut window);
    let status = window
        .toolbar
        .status(&mut window.window, &mut event_loop.context());
    assert_eq!(status, "noted");
}
