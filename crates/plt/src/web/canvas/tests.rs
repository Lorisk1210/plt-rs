use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::artist::ArtistHandle;
use crate::events::EventKind;
use crate::navigation::PointerButton;

/// A canvas for a 400 × 300 figure with one axes over [0, 10]².
fn served() -> (WebCanvas, Figure) {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    let ax = figure
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
    (WebCanvas::new(&figure, "Figure 1"), figure)
}

fn events(canvas: &mut WebCanvas) -> Vec<ServerEvent> {
    canvas
        .take_outputs()
        .into_iter()
        .filter_map(|output| match output {
            Output::Event(event) => Some(event),
            Output::Frame(_) => None,
        })
        .collect()
}

fn mouse(kind: MouseKind, x: f64, y: f64) -> ClientMessage {
    ClientMessage::Mouse(MouseInput {
        kind,
        x,
        y,
        button: Some(PointerButton::Left),
        step: 0.0,
        wheel: 0.0,
        ctrl: false,
        shift: false,
    })
}

fn key(key: &str) -> ClientMessage {
    ClientMessage::Key {
        pressed: true,
        key: key.into(),
    }
}

fn xlim(figure: &Figure) -> (f64, f64) {
    figure.axes()[0].xlim()
}

#[test]
fn a_client_joins_and_gets_everything() {
    let (mut canvas, _) = served();
    canvas.connected();
    assert_eq!(
        events(&mut canvas),
        [ServerEvent::Resize {
            width: 400.0,
            height: 300.0,
            forward: true
        }]
    );
    assert_eq!(canvas.handle(ClientMessage::Refresh), Next::Continue);
    assert_eq!(
        events(&mut canvas),
        [
            ServerEvent::FigureLabel("Figure 1".into()),
            ServerEvent::HistoryButtons {
                back: false,
                forward: false
            },
            ServerEvent::CaptureScroll(true),
            ServerEvent::NavigateMode(NavMode::None),
            ServerEvent::Draw,
        ]
    );
    canvas.handle(ClientMessage::SendImageMode);
    assert_eq!(
        events(&mut canvas),
        [ServerEvent::ImageMode(ImageMode::Full)]
    );
}

#[test]
fn draws_send_full_then_difference_frames() {
    let (mut canvas, figure) = served();
    canvas.handle(ClientMessage::Draw);
    let outputs = canvas.take_outputs();
    let [
        Output::Event(ServerEvent::Resize { forward: true, .. }),
        Output::Frame(png),
    ] = outputs.as_slice()
    else {
        panic!("the size, then a frame: {outputs:?}");
    };
    let image = plt_codec::png::decode(png).expect("a PNG frame");
    assert_eq!((image.width(), image.height()), (400, 300));
    assert!(!figure.is_stale(), "drawn");
    canvas.handle(ClientMessage::Draw);
    let outputs = canvas.take_outputs();
    assert_eq!(
        outputs[0],
        Output::Event(ServerEvent::ImageMode(ImageMode::Diff))
    );
    assert!(matches!(outputs[1], Output::Frame(_)));
}

#[test]
fn blits_send_the_pixels_that_changed() {
    let (mut canvas, figure) = served();
    let ax = figure.axes()[0].clone();
    let line = ax.plot(vec![0.0, 10.0], vec![5.0, 5.0]).expect("plotted");
    line.set_animated(true);
    canvas.handle(ClientMessage::Draw);
    canvas.take_outputs();
    canvas.idle();
    assert_eq!(canvas.take_outputs(), [], "nothing new");
    ax.draw_artist(&line).expect("drawn");
    figure.canvas().blit(Some(&ax.bbox())).expect("blitted");
    canvas.idle();
    let outputs = canvas.take_outputs();
    let [
        Output::Event(ServerEvent::ImageMode(ImageMode::Diff)),
        Output::Frame(png),
    ] = outputs.as_slice()
    else {
        panic!("a difference frame: {outputs:?}");
    };
    let image = plt_codec::png::decode(png).expect("a PNG frame");
    let pixels = image.to_rgba8();
    let at = |x: usize, y: usize| &pixels[(y * 400 + x) * 4..(y * 400 + x + 1) * 4];
    assert_ne!(at(200, 150), [0, 0, 0, 0], "the line changed");
    assert_eq!(at(200, 50), [0, 0, 0, 0], "the rest did not");
    assert!(!figure.is_stale());
}

#[test]
fn stale_figures_ask_for_one_draw_at_a_time() {
    let (mut canvas, figure) = served();
    canvas.handle(ClientMessage::Draw);
    canvas.take_outputs();
    figure.axes()[0].set_xlim(0.0, 5.0).expect("finite limits");
    canvas.idle();
    canvas.idle();
    assert_eq!(events(&mut canvas), [ServerEvent::Draw], "asked once");
    // The client was busy and dropped the request: its next ack lets the
    // server ask again.
    canvas.handle(ClientMessage::Ack);
    canvas.idle();
    assert_eq!(events(&mut canvas), [ServerEvent::Draw]);
}

#[test]
fn resizing_follows_the_device_pixel_ratio() {
    let (mut canvas, figure) = served();
    canvas.handle(ClientMessage::SetDevicePixelRatio(2.0));
    assert_eq!(events(&mut canvas), [ServerEvent::Draw]);
    assert_eq!(figure.dpi(), 200.0);
    let resized = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&resized);
    figure.connect(EventKind::Resize, move |event| {
        log.borrow_mut().push(event.clone());
    });
    canvas.handle(ClientMessage::Resize {
        width: 500.7,
        height: 250.0,
    });
    assert_eq!(figure.size_pixels(), (1000.0, 500.0));
    assert_eq!(
        events(&mut canvas),
        [
            ServerEvent::Resize {
                width: 500.0,
                height: 250.0,
                forward: false
            },
            ServerEvent::Draw
        ]
    );
    assert_eq!(
        resized.borrow().as_slice(),
        [Event::Resize {
            width: 1000.0,
            height: 500.0
        }]
    );
    // Downloads keep the figure's own resolution.
    let png = canvas.download(SaveFormat::Png).expect("saved");
    let image = plt_codec::png::decode(&png).expect("a PNG");
    assert_eq!((image.width(), image.height()), (500, 250));
}

#[test]
fn hostile_sizes_are_bounded() {
    let (mut canvas, figure) = served();
    canvas.handle(ClientMessage::SetDevicePixelRatio(1e9));
    let (width, height) = figure.size_pixels();
    assert_eq!(figure.dpi(), 800.0, "at most 8 pixels per CSS pixel");
    assert!(width <= MAX_SIDE && height <= MAX_SIDE && width * height <= MAX_AREA * 1.001);
    canvas.handle(ClientMessage::Resize {
        width: 1e12,
        height: 1e12,
    });
    let (width, height) = figure.size_pixels();
    assert!(width <= MAX_SIDE && height <= MAX_SIDE && width * height <= MAX_AREA * 1.001);
    canvas.handle(ClientMessage::SetDevicePixelRatio(1.0));
    canvas.handle(ClientMessage::Resize {
        width: -5.0,
        height: 0.0,
    });
    assert_eq!(figure.size_pixels(), (1.0, 1.0));
}

#[test]
fn dragging_in_pan_mode_moves_the_view() {
    let (mut canvas, figure) = served();
    canvas.handle(ClientMessage::Toolbar(Some(ToolbarButton::Pan)));
    assert_eq!(
        events(&mut canvas),
        [
            ServerEvent::NavigateMode(NavMode::Pan),
            ServerEvent::Cursor(CursorShape::Pointer)
        ]
    );
    canvas.handle(mouse(MouseKind::Press, 200.0, 150.0));
    canvas.handle(mouse(MouseKind::Motion, 240.0, 150.0));
    let moved = events(&mut canvas);
    assert!(
        moved.contains(&ServerEvent::Cursor(CursorShape::Move)),
        "{moved:?}"
    );
    assert!(
        moved.iter().any(
            |event| matches!(event, ServerEvent::Message(text) if text.starts_with("(x, y) = "))
        ),
        "{moved:?}"
    );
    canvas.handle(mouse(MouseKind::Release, 240.0, 150.0));
    assert_eq!(
        events(&mut canvas),
        [ServerEvent::HistoryButtons {
            back: true,
            forward: false
        }]
    );
    let (low, high) = xlim(&figure);
    assert!(
        (low + 1.0).abs() < 1e-9 && (high - 9.0).abs() < 1e-9,
        "{low}..{high}"
    );
    // `h` goes home, `p` leaves the pan mode.
    canvas.handle(key("h"));
    assert_eq!(xlim(&figure), (0.0, 10.0));
    canvas.handle(key("p"));
    assert!(events(&mut canvas).contains(&ServerEvent::NavigateMode(NavMode::None)));
}

#[test]
fn side_buttons_step_through_the_view_history() {
    let (mut canvas, figure) = served();
    canvas.handle(ClientMessage::Toolbar(Some(ToolbarButton::Pan)));
    canvas.handle(mouse(MouseKind::Press, 200.0, 150.0));
    canvas.handle(mouse(MouseKind::Motion, 240.0, 150.0));
    canvas.handle(mouse(MouseKind::Release, 240.0, 150.0));
    let panned = xlim(&figure);
    let side = |button| {
        ClientMessage::Mouse(MouseInput {
            kind: MouseKind::Press,
            x: 100.0,
            y: 100.0,
            button: Some(button),
            step: 0.0,
            wheel: 0.0,
            ctrl: false,
            shift: false,
        })
    };
    canvas.handle(side(PointerButton::Back));
    assert_eq!(xlim(&figure), (0.0, 10.0), "back");
    assert!(events(&mut canvas).contains(&ServerEvent::HistoryButtons {
        back: false,
        forward: true
    }));
    canvas.handle(side(PointerButton::Forward));
    assert_eq!(xlim(&figure), panned, "forward");
}

#[test]
fn zooming_shows_and_removes_the_rubberband() {
    let (mut canvas, figure) = served();
    canvas.handle(ClientMessage::Toolbar(Some(ToolbarButton::Zoom)));
    canvas.handle(mouse(MouseKind::Press, 100.0, 75.0));
    canvas.handle(mouse(MouseKind::Motion, 300.0, 225.0));
    let band = events(&mut canvas)
        .into_iter()
        .find_map(|event| match event {
            ServerEvent::Rubberband(band) => Some(band),
            _ => None,
        });
    let band = band.flatten().expect("a rubberband while dragging");
    assert_eq!(
        (band.x0(), band.y0(), band.x1(), band.y1()),
        (100.0, 225.0, 300.0, 75.0)
    );
    canvas.handle(mouse(MouseKind::Release, 300.0, 225.0));
    assert!(events(&mut canvas).contains(&ServerEvent::Rubberband(None)));
    assert_eq!(xlim(&figure), (2.5, 7.5));
}

#[test]
fn a_tool_of_the_navigation_group_takes_turns_with_pan() {
    use crate::gui::{NAVIGATION_GROUP, ToolKind, ToolSpec};

    let (mut canvas, figure) = served();
    let spec = ToolSpec {
        name: "lasso".into(),
        keys: vec!["m".into()],
        kind: ToolKind::Toggle,
        radio_group: Some(NAVIGATION_GROUP.into()),
        cursor: Some(CursorShape::Hand),
        ..ToolSpec::default()
    };
    figure.add_tool(spec, |_| {}).expect("the tool is new");
    canvas.handle(ClientMessage::Toolbar(Some(ToolbarButton::Pan)));
    canvas.handle(key("m"));
    assert!(events(&mut canvas).contains(&ServerEvent::NavigateMode(NavMode::None)));
    canvas.handle(mouse(MouseKind::Motion, 200.0, 150.0));
    assert!(events(&mut canvas).contains(&ServerEvent::Cursor(CursorShape::Hand)));
    canvas.handle(ClientMessage::Toolbar(Some(ToolbarButton::Pan)));
    assert_eq!(figure.tool_toggled("lasso"), Some(false), "pan took over");
}

#[test]
fn input_reaches_figure_callbacks() {
    let (mut canvas, figure) = served();
    let log = Rc::new(RefCell::new(Vec::new()));
    for kind in [
        EventKind::KeyPress,
        EventKind::ButtonPress,
        EventKind::Scroll,
    ] {
        let log = Rc::clone(&log);
        figure.connect(kind, move |event| log.borrow_mut().push(event.clone()));
    }
    canvas.handle(mouse(MouseKind::Motion, 200.0, 150.0));
    canvas.handle(key("a"));
    canvas.handle(mouse(MouseKind::DoubleClick, 200.0, 150.0));
    canvas.handle(ClientMessage::Mouse(MouseInput {
        step: 1.0,
        wheel: 1.0,
        ..match mouse(MouseKind::Scroll, 200.0, 150.0) {
            ClientMessage::Mouse(input) => input,
            _ => unreachable!(),
        }
    }));
    let log = log.borrow();
    let Event::KeyPress(pressed) = &log[0] else {
        panic!("a key press: {log:?}");
    };
    assert_eq!(pressed.key, "a");
    assert_eq!(pressed.location.xdata, Some(5.0));
    let Event::ButtonPress(double) = &log[1] else {
        panic!("a double click: {log:?}");
    };
    assert!(double.dblclick);
    assert!(matches!(&log[2], Event::Scroll(scroll) if scroll.step > 0.0));
    let (low, high) = xlim(&figure);
    assert!(high - low < 10.0, "the wheel zooms in");
}

#[test]
fn shortcuts_close_and_save() {
    let (mut canvas, _) = served();
    assert_eq!(canvas.handle(key("q")), Next::Close);
    assert_eq!(canvas.handle(key("ctrl+s")), Next::Continue);
    assert!(events(&mut canvas).contains(&ServerEvent::Save));
    let (mut canvas, figure) = served();
    figure.set_capture_keys(true);
    assert_eq!(
        canvas.handle(key("q")),
        Next::Continue,
        "a widget has the keys"
    );
}
