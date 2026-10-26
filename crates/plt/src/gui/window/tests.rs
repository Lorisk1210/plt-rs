//! Headless tests of figure windows.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use plt_platform::{
    ButtonState, Event, EventLoop, HeadlessHandle, Key, KeyEvent, Modifiers, MouseButton,
    PhysicalPosition,
};

use super::*;
use crate::animation::{AnimationOptions, Frames, FuncAnimation};
use crate::artist::ArtistHandle;
use crate::events::EventKind;
use crate::figure::SaveOptions;
use crate::gui::subplot_dialog::{Param, SubplotMessage};
use crate::lines::Line2D;
use crate::widgets::{Cursor, CursorOptions};

/// An opaque white pixel.
const WHITE: [u8; 4] = [255; 4];

/// Runs one loop iteration, feeding the events of `window` and its
/// tool windows to it.
pub(super) fn pump(event_loop: &mut EventLoop, window: &mut FigureWindow) -> bool {
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

pub(super) fn mouse(handle: &HeadlessHandle, id: WindowId, state: ButtonState, (x, y): (f64, f64)) {
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
    chord(handle, id, c, Modifiers::NONE);
}

/// Presses and releases `c` with `modifiers` held.
fn chord(handle: &HeadlessHandle, id: WindowId, c: char, modifiers: Modifiers) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = KeyEvent {
            key: Key::Character(c),
            state,
            repeat: false,
            text: None,
            modifiers,
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
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
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
fn dragging_rotates_3d_axes() {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    let ax = figure.subplot_3d();
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    assert!(pump(&mut event_loop, &mut window));
    let id = window.id();
    mouse(&handle, id, ButtonState::Pressed, (200.0, 150.0));
    mouse(&handle, id, ButtonState::Released, (240.0, 140.0));
    pump(&mut event_loop, &mut window);
    let turned = ax.view();
    assert_ne!(turned.azim, -60.0, "the drag turned the view");
    key(&handle, id, 'h');
    pump(&mut event_loop, &mut window);
    assert_eq!(ax.view().azim, -60.0, "home restores the view");
    key(&handle, id, 'q');
    assert!(!pump(&mut event_loop, &mut window));
}

#[test]
fn user_tools_and_help_are_on_the_toolbar() {
    use crate::gui::{Icon, ToolKind, ToolSpec};

    let figure = Figure::with_size(4.0, 3.0, 100.0);
    figure.subplot();
    let uses = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = std::rc::Rc::clone(&uses);
    let spec = ToolSpec {
        name: "mark".into(),
        description: "Mark the peak".into(),
        icon: Icon::new(24.0),
        keys: vec!["m".into()],
        kind: ToolKind::Toggle,
    };
    figure
        .add_tool(spec, move |_| count.set(count.get() + 1))
        .expect("the tool is new");
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    assert!(pump(&mut event_loop, &mut window));
    let id = window.id();
    key(&handle, id, 'm');
    pump(&mut event_loop, &mut window);
    assert_eq!((uses.get(), figure.tool_toggled("mark")), (1, Some(true)));
    window
        .apply(&mut event_loop.context(), Message::Tool(Tool::User(0)))
        .expect("the button works");
    assert_eq!((uses.get(), figure.tool_toggled("mark")), (2, Some(false)));
    assert_eq!(window.toolbar.hint(Tool::User(0)), "Mark the peak (m)");
    window
        .apply(&mut event_loop.context(), Message::Tool(Tool::Help))
        .expect("help opens");
    assert!(window.help_dialog.is_some(), "the help window is open");
    key(&handle, id, 'q');
    assert!(!pump(&mut event_loop, &mut window));
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
    let tool_id = tool.id();
    let snapshot = handle.window(tool_id).expect("the tool window exists");
    assert_eq!(snapshot.title, "Subplot configuration tool");
    assert!(window.owns_window(tool_id));
    pump(&mut event_loop, &mut window);
    handle.take_frames();

    // Click 30% along the "left" slider: its thumb's centre travels
    // between the ends of the track, inset by the thumb's radius.
    let tool = window.subplot_tool.as_ref().expect("the tool is open");
    let left = tool
        .controls
        .iter()
        .find(|control| control.param == Param::Left)
        .copied()
        .expect("the tool has a left slider");
    let track = tool
        .window()
        .ui()
        .widget_rect(left.slider)
        .expect("the slider is laid out");
    let radius = tool.window().ui().theme().checkbox_size / 2.0;
    let at = (
        track.x + radius + 0.3 * (track.width - 2.0 * radius),
        track.center().y,
    );
    mouse(&handle, tool_id, ButtonState::Pressed, at);
    mouse(&handle, tool_id, ButtonState::Released, at);
    pump(&mut event_loop, &mut window);
    let value = figure.subplot_params().left;
    assert!((value - 0.3).abs() < 0.01, "left is {value}");
    let redrawn: Vec<_> = handle.take_frames().iter().map(|f| f.window).collect();
    assert!(redrawn.contains(&main), "the figure is redrawn");
    assert!(redrawn.contains(&tool_id), "the slider is redrawn");
    let tool = window.subplot_tool.as_mut().expect("the tool is open");
    let shown = tool
        .window_mut()
        .ui_mut()
        .widget_mut::<plt_ui::TextInput<SubplotMessage>>(left.field)
        .map(|field| field.text().to_owned());
    assert_eq!(
        shown.as_deref(),
        Some("0.300"),
        "the field follows the slider"
    );

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

#[test]
fn high_density_displays_scale_the_figure_once_and_saving_ignores_them() {
    let figure = Figure::with_size(2.0, 1.5, 100.0);
    figure.subplot();
    let (mut event_loop, handle) = EventLoop::headless();
    handle.set_scale_factor(2.0).expect("a valid scale factor");
    let windows: Vec<FigureWindow> = (0..2)
        .map(|_| {
            let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
                .expect("the window opens");
            pump(&mut event_loop, &mut window);
            assert_eq!(figure.canvas().device_pixel_ratio(), 2.0);
            assert_eq!(figure.dpi(), 200.0, "scaled once, however often shown");
            assert_eq!(figure.width_height(), (400, 300));
            window
        })
        .collect();
    let saved = figure
        .render_raster_with(SaveOptions::default())
        .expect("saved")
        .to_image();
    assert_eq!((saved.width(), saved.height()), (200, 150));
    drop(windows);
    assert_eq!(figure.canvas().device_pixel_ratio(), 1.0);
    assert_eq!(figure.dpi(), 100.0, "unscaled once no window shows it");
}

#[test]
fn ctrl_c_copies_the_figure_to_the_clipboard() {
    let figure = Figure::with_size(2.0, 1.5, 100.0);
    figure.subplot();
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    pump(&mut event_loop, &mut window);
    assert_eq!(handle.clipboard_image(), None);

    chord(&handle, window.id(), 'c', Modifiers::CONTROL);
    pump(&mut event_loop, &mut window);
    let png = handle.clipboard_image().expect("the figure is copied");
    let image = plt_codec::png::decode(&png).expect("the clipboard holds a PNG");
    assert_eq!((image.width(), image.height()), (200, 150));
}

/// A 4 × 3 in figure at 100 dpi with an axes and an animated line, shown
/// in a headless window whose first frame was presented.
fn shown_with_animated_line() -> (EventLoop, HeadlessHandle, FigureWindow, Line2D) {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    let ax = figure
        .add_axes([0.25, 0.25, 0.5, 0.5])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(0.0, 1.0))
        .expect("finite limits");
    let line = ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).expect("plotted");
    line.set_animated(true);
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    pump(&mut event_loop, &mut window);
    handle.take_frames();
    (event_loop, handle, window, line)
}

/// Where the axes of [`shown_with_animated_line`] lie in the window.
const AXES_AREA: PhysicalRect = PhysicalRect::new(100, 75, 200, 150);

#[test]
fn blits_update_only_their_part_of_the_window() {
    let (mut event_loop, handle, mut window, line) = shown_with_animated_line();
    let figure = window.figure().clone();
    let ax = figure.axes()[0].clone();
    let canvas = figure.canvas();
    let background = canvas.copy_from_bbox(&figure.bbox()).expect("saved");
    ax.draw_artist(&line).expect("drawn");
    canvas.blit(Some(&ax.bbox())).expect("blitted");
    window.refresh(&mut event_loop.context()).expect("shown");
    let frames = handle.take_frames();
    let [frame] = frames.as_slice() else {
        panic!("one partial update: {frames:?}");
    };
    assert_eq!(frame.region, AXES_AREA);
    assert_ne!(frame.image.pixel(200, 150), Some(WHITE), "the line");
    canvas.restore_region(&background).expect("restored");
    canvas.blit(None).expect("blitted");
    window.refresh(&mut event_loop.context()).expect("shown");
    let frames = handle.take_frames();
    assert_eq!(frames[0].region, PhysicalRect::new(0, 0, 400, 300));
    assert_eq!(frames[0].image.pixel(200, 150), Some(WHITE));
    assert!(!figure.is_stale(), "no full draw");
}

#[test]
fn resizes_reach_callbacks_before_the_figure_is_drawn_again() {
    let (mut event_loop, handle, mut window, _) = shown_with_animated_line();
    let seen = Rc::new(RefCell::new(Vec::new()));
    for kind in [EventKind::Resize, EventKind::Draw] {
        let log = Rc::clone(&seen);
        window
            .figure()
            .connect(kind, move |event| log.borrow_mut().push(event.kind()));
    }
    let resized = WindowEvent::Resized(PhysicalSize::new(500, 340));
    handle
        .inject(window.id(), resized)
        .expect("the window exists");
    pump(&mut event_loop, &mut window);
    assert_eq!(seen.borrow().first(), Some(&EventKind::Resize));
    assert!(seen.borrow().contains(&EventKind::Draw));
    let image = window.figure().canvas().buffer_rgba().expect("drawn");
    assert_eq!(image.width(), 500);
}

#[test]
fn blitted_animations_play_through_partial_updates() {
    let (mut event_loop, handle, mut window, line) = shown_with_animated_line();
    let moved = line.clone();
    let options = AnimationOptions {
        blit: true,
        ..AnimationOptions::default()
    };
    let animation = FuncAnimation::new(window.figure(), Frames::Count(4), options, move |i| {
        let y = [0.0, 0.25, 0.5, 0.75][i % 4];
        moved.set_data([0.0, 1.0], [y, y]);
        moved.clone()
    });
    // The window draws the figure, which starts the animation.
    window
        .refresh(&mut event_loop.context())
        .expect("refreshed");
    pump(&mut event_loop, &mut window);
    let (timer, _) = window.timers[0];
    handle.take_frames();
    let mut cx = event_loop.context();
    window.tick(&mut cx, timer).expect("ticked");
    window.tick(&mut cx, timer).expect("ticked");
    let frames = handle.take_frames();
    assert_eq!(frames.len(), 2, "one update per frame");
    assert!(
        frames.iter().all(|frame| frame.region == AXES_AREA),
        "only the axes: {frames:?}"
    );
    drop(animation);
}

#[test]
fn blitting_cursors_draw_without_redrawing_the_figure() {
    let (mut event_loop, handle, mut window, _) = shown_with_animated_line();
    let figure = window.figure().clone();
    let options = CursorOptions {
        useblit: true,
        ..CursorOptions::default()
    };
    let cursor = Cursor::new(&figure.axes()[0], options);
    // The window draws the figure, which saves the cursor's background.
    figure.canvas().draw_idle().expect("drawn");
    window.refresh(&mut event_loop.context()).expect("shown");
    let draws = Rc::new(RefCell::new(0));
    let counted = Rc::clone(&draws);
    figure.connect(EventKind::Draw, move |_| *counted.borrow_mut() += 1);
    let moved = WindowEvent::CursorMoved {
        position: PhysicalPosition::new(200.0, 150.0),
        modifiers: Modifiers::NONE,
    };
    handle
        .inject(window.id(), moved)
        .expect("the window exists");
    pump(&mut event_loop, &mut window);
    window.refresh(&mut event_loop.context()).expect("shown");
    let frames = handle.take_frames();
    let frame = frames.last().expect("a frame is presented");
    assert_ne!(
        frame.image.pixel(200, 100),
        Some(WHITE),
        "the vertical line"
    );
    assert_eq!(frame.image.pixel(150, 100), Some(WHITE));
    assert_eq!(*draws.borrow(), 0, "the figure is not drawn again");
    assert!(cursor.vertical_line().visible() && !figure.is_stale());
}
