//! Headless tests of figure windows.

use std::time::Duration;

use plt_platform::{
    ButtonState, Event, EventLoop, HeadlessHandle, Key, KeyEvent, Modifiers, MouseButton,
    PhysicalPosition,
};

use super::*;

/// Runs one loop iteration, feeding the events of `window` and its
/// tool windows to it.
fn pump(event_loop: &mut EventLoop, window: &mut FigureWindow) -> bool {
    let mut open = true;
    event_loop
        .pump_events(Some(Duration::ZERO), |cx, event| {
            if let Event::Window { window: id, event } = event
                && window.owns_window(id)
            {
                open &= window
                    .handle(cx, id, &event)
                    .expect("the window handles events");
            }
        })
        .expect("the headless loop runs");
    open
}

fn mouse(handle: &HeadlessHandle, id: WindowId, state: ButtonState, (x, y): (f64, f64)) {
    let position = PhysicalPosition::new(x, y);
    handle
        .inject(
            id,
            WindowEvent::CursorMoved {
                position,
                modifiers: Modifiers::NONE,
            },
        )
        .expect("the window exists");
    let click = WindowEvent::MouseInput {
        button: MouseButton::Left,
        state,
        position,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    handle.inject(id, click).expect("the window exists");
}

fn key(handle: &HeadlessHandle, id: WindowId, c: char) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = KeyEvent {
            key: Key::Character(c),
            state,
            repeat: false,
            text: None,
            modifiers: Modifiers::NONE,
            scancode: 0,
        };
        handle
            .inject(id, WindowEvent::Key(event))
            .expect("the window exists");
    }
}

#[test]
fn shows_the_figure_and_pans_it() {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    let ax = figure.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    let presses = std::rc::Rc::new(std::cell::Cell::new(0));
    let counter = std::rc::Rc::clone(&presses);
    figure.connect(crate::events::EventKind::ButtonPress, move |event| {
        if event.location().is_some_and(|at| at.inaxes.is_some()) {
            counter.set(counter.get() + 1);
        }
    });
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    assert!(pump(&mut event_loop, &mut window));
    let frames = handle.take_frames();
    let frame = frames.last().expect("a frame is presented");
    // The canvas shows the white figure above the toolbar.
    assert_eq!(frame.image.pixel(5, 5), Some([255, 255, 255, 255]));
    assert_eq!(window.canvas_size, PhysicalSize::new(400, 300));

    let id = window.id();
    mouse(&handle, id, ButtonState::Pressed, (200.0, 150.0));
    mouse(&handle, id, ButtonState::Released, (200.0, 150.0));
    key(&handle, id, 'p');
    pump(&mut event_loop, &mut window);
    assert_eq!(window.navigation.mode(), NavMode::Pan);
    assert_eq!(presses.get(), 1, "callbacks see clicks in the axes");
    mouse(&handle, id, ButtonState::Pressed, (200.0, 150.0));
    mouse(&handle, id, ButtonState::Released, (231.0, 150.0));
    pump(&mut event_loop, &mut window);
    let (x0, x1) = ax.xlim();
    assert!(
        x0 < -0.5 && (x1 - x0 - 10.0).abs() < 1e-9,
        "panned to ({x0}, {x1})"
    );
    key(&handle, id, 'h');
    pump(&mut event_loop, &mut window);
    assert_eq!(ax.xlim(), (0.0, 10.0), "home restores the view");
    key(&handle, id, 'q');
    assert!(!pump(&mut event_loop, &mut window), "q closes the window");
}

#[test]
fn the_subplot_tool_adjusts_the_figure() {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    figure.subplots(2, 2).expect("a 2×2 grid has cells");
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    let main = window.id();
    window
        .use_tool(&mut event_loop.context(), Tool::Subplots)
        .expect("the tool opens");
    let tool = window.subplot_tool.as_ref().expect("the tool is open");
    let tool_id = tool.window.id();
    let snapshot = handle.window(tool_id).expect("the tool window exists");
    assert_eq!(snapshot.title, "Subplot configuration tool");
    assert_eq!(snapshot.size, PhysicalSize::new(600, 300), "no toolbar");
    assert!(window.owns_window(tool_id));
    pump(&mut event_loop, &mut window);
    handle.take_frames();

    // Click 30% along the "left" slider.
    let tool = window.subplot_tool.as_ref().expect("the tool is open");
    let slider = tool.window.figure().axes()[0].position();
    let at = (
        600.0 * (slider.x0() + 0.3 * slider.width()),
        300.0 * (1.0 - (slider.y0() + 0.5 * slider.height())),
    );
    mouse(&handle, tool_id, ButtonState::Pressed, at);
    mouse(&handle, tool_id, ButtonState::Released, at);
    pump(&mut event_loop, &mut window);
    let left = figure.subplot_params().left;
    assert!((left - 0.3).abs() < 0.01, "left is {left}");
    let redrawn: Vec<_> = handle.take_frames().iter().map(|f| f.window).collect();
    assert!(redrawn.contains(&main), "the figure is redrawn");
    assert!(redrawn.contains(&tool_id), "the slider is redrawn");

    // Clicking the tool again raises its window.
    event_loop
        .context()
        .raise_window(main)
        .expect("the window exists");
    window
        .use_tool(&mut event_loop.context(), Tool::Subplots)
        .expect("the tool is raised");
    assert_eq!(handle.stacking_order(), [main, tool_id]);

    handle
        .inject(tool_id, WindowEvent::CloseRequested)
        .expect("the window exists");
    assert!(pump(&mut event_loop, &mut window));
    assert!(window.subplot_tool.is_none());
    assert_eq!(handle.windows(), [main]);
    window
        .close(&mut event_loop.context())
        .expect("the window closes");
    assert_eq!(handle.windows(), Vec::<WindowId>::new());
}

#[test]
fn the_save_dialog_writes_the_figure() {
    let dir = std::env::temp_dir().join(format!("plt-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the directory is made");
    crate::rc::context(|| {
        crate::rc::update(|rc| rc.savefig_directory = dir.display().to_string());
        let figure = Figure::with_size(2.0, 1.5, 100.0);
        figure.subplot();
        let (mut event_loop, handle) = EventLoop::headless();
        let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
            .expect("the window opens");
        window
            .use_tool(&mut event_loop.context(), Tool::Save)
            .expect("the dialog opens");
        let dialog = handle.stacking_order()[1];
        assert_eq!(
            handle.window(dialog).expect("it exists").title,
            "Save the figure"
        );
        pump(&mut event_loop, &mut window);
        // The name's stem is selected: typing replaces it.
        for c in ['p', 'l', 'o', 't'] {
            let event = KeyEvent {
                key: Key::Character(c),
                state: ButtonState::Pressed,
                repeat: false,
                text: Some(c.to_string()),
                modifiers: Modifiers::NONE,
                scancode: 0,
            };
            handle
                .inject(dialog, WindowEvent::Key(event))
                .expect("the dialog exists");
        }
        let enter = KeyEvent {
            key: Key::Enter,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers: Modifiers::NONE,
            scancode: 0,
        };
        handle
            .inject(dialog, WindowEvent::Key(enter))
            .expect("the dialog exists");
        pump(&mut event_loop, &mut window);
        assert!(dir.join("plot.png").is_file(), "the figure is saved");
        assert_eq!(handle.windows(), [window.id()], "the dialog closed");
    });
    std::fs::remove_dir_all(&dir).expect("the directory is removed");
}
