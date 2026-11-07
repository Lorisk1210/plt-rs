//! Headless tests of hosted windows: opening, input, the toolbar, the
//! file chooser, timers and what the windows show.

use std::path::Path;
use std::time::{Duration, Instant};

use pltrs_core::Point;
use pltrs_platform::{
    ButtonState, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition, PhysicalSize,
    WindowEvent, WindowId,
};

use super::*;
use crate::gui::input::InputKind;
use crate::gui::window::toolbar_height;
use crate::image::RgbaImage;

/// A host showing a 400 × 300 figure in a window with a toolbar of
/// `buttons` in group `"navigation"`.
fn shown(buttons: &[&str]) -> (WindowHost, HeadlessHandle, Figure, HostedWindowId) {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    let (mut host, handle) = WindowHost::headless();
    let window = host.add_window(&figure, "Figure 1", (400.0, 300.0), true);
    for name in buttons {
        host.add_tool(window, item(name, "navigation"), None)
            .expect("the window has a toolbar");
    }
    host.show(window).expect("the window opens");
    (host, handle, figure, window)
}

fn item(name: &str, group: &str) -> ToolItem {
    ToolItem {
        name: name.to_owned(),
        group: group.to_owned(),
        tooltip: format!("{name} tooltip"),
        image: ToolImage::Named("home".to_owned()),
    }
}

/// The window system's window of hosted window `id`.
fn platform_window(host: &WindowHost, id: HostedWindowId) -> WindowId {
    host.window_system_id(id).expect("the window is shown")
}

/// Everything the host reports within a few rounds of pumping.
fn drain(host: &mut WindowHost) -> Vec<HostEvent> {
    let mut events = Vec::new();
    for _ in 0..3 {
        events.extend(host.pump(Some(Duration::ZERO)).expect("the loop runs"));
    }
    events
}

fn click(handle: &HeadlessHandle, id: WindowId, (x, y): (f64, f64), click_count: u32) {
    let position = PhysicalPosition::new(x, y);
    let moved = WindowEvent::CursorMoved {
        position,
        modifiers: Modifiers::NONE,
    };
    handle.inject(id, moved).expect("the window exists");
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            position,
            modifiers: Modifiers::NONE,
            click_count,
        };
        handle.inject(id, event).expect("the window exists");
    }
}

fn key(handle: &HeadlessHandle, id: WindowId, key: Key) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = KeyEvent {
            key,
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
fn windows_open_when_shown_at_their_size_above_the_toolbar() {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    let (mut host, handle) = WindowHost::headless();
    let window = host.add_window(&figure, "Figure 1", (400.0, 300.0), true);
    host.add_tool(window, item("Home", "navigation"), None)
        .expect("the window has a toolbar");
    assert!(!host.is_shown(window));
    assert!(handle.windows().is_empty(), "nothing opens before show");
    host.set_title(window, "My plot")
        .expect("the window exists");
    host.show(window).expect("the window opens");
    let id = platform_window(&host, window);
    let snapshot = handle.window(id).expect("open");
    assert_eq!(snapshot.title, "My plot");
    let height = (300.0 + toolbar_height()).round() as u32;
    assert_eq!(snapshot.size, PhysicalSize::new(400, height));
    let events = host.pump(Some(Duration::ZERO)).expect("the loop runs");
    assert_eq!(
        events.first(),
        Some(&HostEvent::Resized {
            window,
            width: 400,
            height: 300,
            scale_factor: 1.0
        }),
        "the canvas size comes first"
    );
    host.close(window).expect("the window closes");
    assert_eq!(handle.windows(), []);
    assert!(matches!(host.show(window), Err(GuiError::NotShown)));
    assert!(host.close(window).is_ok(), "closed already");
}

#[test]
fn input_over_the_canvas_is_reported_in_matplotlibs_terms() {
    let (mut host, handle, _, window) = shown(&[]);
    drain(&mut host);
    let id = platform_window(&host, window);
    click(&handle, id, (100.0, 50.0), 1);
    click(&handle, id, (100.0, 50.0), 2);
    key(&handle, id, Key::Character('q'));
    let inputs: Vec<CanvasInput> = drain(&mut host)
        .into_iter()
        .filter_map(|event| match event {
            HostEvent::Input { window: w, input } if w == window => Some(input),
            _ => None,
        })
        .collect();
    let kinds: Vec<&InputKind> = inputs.iter().map(|input| &input.kind).collect();
    assert_eq!(
        kinds,
        [
            &InputKind::Enter,
            &InputKind::Motion,
            &InputKind::Press {
                button: 1,
                double: false
            },
            &InputKind::Release { button: 1 },
            // The second click's motion to where the pointer is already
            // is no event, as in Qt.
            &InputKind::Press {
                button: 1,
                double: true
            },
            &InputKind::Release { button: 1 },
            &InputKind::KeyPress("q".into()),
            &InputKind::KeyRelease("q".into()),
        ]
    );
    assert_eq!(inputs[2].position, Some(Point::new(100.0, 50.0)));
    assert_eq!(inputs[2].buttons, [1]);
    assert_eq!(inputs[3].buttons, []);
    assert_eq!(
        inputs[6].position,
        Some(Point::new(100.0, 50.0)),
        "keys report where the pointer is"
    );
}

#[test]
fn closing_is_the_programs_decision() {
    let (mut host, handle, _, window) = shown(&[]);
    let id = platform_window(&host, window);
    handle
        .inject(id, WindowEvent::CloseRequested)
        .expect("the window exists");
    assert!(drain(&mut host).contains(&HostEvent::CloseRequested { window }));
    assert!(host.is_shown(window), "the window stays until closed");
}

#[test]
fn toolbar_buttons_report_clicks_and_follow_the_program() {
    let (mut host, handle, _, window) = shown(&["Home", "Back"]);
    host.add_tool(window, item("Pan", "zoompan"), None)
        .expect("a new group");
    host.add_tool(window, item("Start", "navigation"), Some(0))
        .expect("first of its group");
    assert_eq!(
        host.toolbar_layout(window).expect("open"),
        [
            (
                "navigation".to_owned(),
                vec!["Start".into(), "Home".into(), "Back".into()]
            ),
            ("zoompan".to_owned(), vec!["Pan".into()]),
        ]
    );
    host.set_tool_checked(window, "Pan", true).expect("open");
    assert_eq!(host.tool_checked(window, "Pan").expect("open"), Some(true));
    assert!(host.remove_tool(window, "Back").expect("open"));
    assert!(!host.remove_tool(window, "Back").expect("open"));
    host.set_message(window, "x=1 y=2").expect("open");
    assert_eq!(host.message(window).expect("open"), "x=1 y=2");
    drain(&mut host);
    // The first button sits at the toolbar's left end, below the canvas.
    let id = platform_window(&host, window);
    let below = 300.0 + toolbar_height() / 2.0;
    click(&handle, id, (20.0, below), 1);
    let clicked: Vec<String> = drain(&mut host)
        .into_iter()
        .filter_map(|event| match event {
            HostEvent::ToolClicked { name, .. } => Some(name),
            _ => None,
        })
        .collect();
    assert_eq!(clicked, ["Start"]);
}

#[test]
fn a_window_without_a_toolbar_refuses_buttons() {
    let figure = Figure::new();
    let (mut host, _handle) = WindowHost::headless();
    let window = host.add_window(&figure, "tool", (200.0, 100.0), false);
    assert!(matches!(
        host.add_tool(window, item("Home", "navigation"), None),
        Err(GuiError::NotShown)
    ));
    assert!(host.set_message(window, "ignored").is_ok());
    assert_eq!(host.message(window).expect("added"), "");
}

#[test]
fn bitmaps_and_unknown_names_make_buttons_too() {
    let (mut host, _handle, _, window) = shown(&[]);
    let bitmap = RgbaImage::transparent(26, 26);
    let custom = ToolItem {
        image: ToolImage::Bitmap(bitmap),
        ..item("test", "group")
    };
    host.add_tool(window, custom, None).expect("open");
    let unknown = ToolItem {
        image: ToolImage::Named("sparkles".into()),
        ..item("other", "group")
    };
    host.add_tool(window, unknown, None).expect("open");
    assert!(ToolImage::is_drawn("zoom_to_rect"));
    assert!(!ToolImage::is_drawn("sparkles"));
    assert_eq!(
        host.toolbar_layout(window).expect("open"),
        [("group".to_owned(), vec!["test".into(), "other".into()])]
    );
}

#[test]
fn the_file_chooser_holds_the_windows_input_until_it_ends() {
    let (mut host, handle, _, window) = shown(&[]);
    drain(&mut host);
    host.choose_file(window, None, "Figure_1.png")
        .expect("the chooser opens");
    let dialog = *handle.windows().last().expect("the chooser's window");
    let id = platform_window(&host, window);
    assert_ne!(dialog, id);
    click(&handle, id, (100.0, 50.0), 1);
    assert!(
        !drain(&mut host)
            .iter()
            .any(|event| matches!(event, HostEvent::Input { .. })),
        "the window is held"
    );
    key(&handle, dialog, Key::Escape);
    assert!(drain(&mut host).contains(&HostEvent::FileChosen { window, path: None }));
    assert_eq!(handle.windows(), [id], "the chooser closed");
}

/// Chooses a file in the chooser of a shown window opened in `directory`,
/// with `savefig.directory` set to `rc_directory`, by pressing Enter.
fn chosen_file(directory: Option<&Path>, rc_directory: &Path) -> Vec<HostEvent> {
    crate::rc::context(|| {
        let rc = rc_directory.display().to_string();
        crate::rc::set("savefig.directory", &rc).expect("a path");
        let (mut host, handle, _, window) = shown(&[]);
        drain(&mut host);
        host.choose_file(window, directory, "chart.svg")
            .expect("the chooser opens");
        let dialog = *handle.windows().last().expect("the chooser's window");
        key(&handle, dialog, Key::Enter);
        drain(&mut host)
    })
}

#[test]
fn the_file_chooser_hands_over_the_file_without_saving() {
    let dir = std::env::temp_dir().join(format!("pltrs-host-chooser-{}", std::process::id()));
    let other = dir.join("other");
    std::fs::create_dir_all(&other).expect("scratch directories");
    let window = HostedWindowId::from_raw(1);
    let path = |dir: &Path| HostEvent::FileChosen {
        window,
        path: Some(dir.join("chart.svg")),
    };
    assert!(
        chosen_file(None, &dir).contains(&path(&dir)),
        "in savefig.directory"
    );
    assert!(
        chosen_file(Some(&other), &dir).contains(&path(&other)),
        "in the directory asked for"
    );
    assert!(
        chosen_file(Some(&dir.join("missing")), &dir).contains(&path(&dir)),
        "a directory that is not one is ignored"
    );
    assert!(!dir.join("chart.svg").exists(), "the program saves");
    std::fs::remove_dir_all(&dir).expect("the scratch directories go");
}

#[test]
fn timers_fire_once_or_repeatedly_until_stopped() {
    let (mut host, _handle) = WindowHost::headless();
    let once = host.start_timer(Duration::from_millis(1), false);
    let every = host.start_timer(Duration::from_millis(1), true);
    let mut fired = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while fired.iter().filter(|&&id| id == every).count() < 3 && Instant::now() < deadline {
        for event in host
            .pump(Some(Duration::from_millis(5)))
            .expect("the loop runs")
        {
            if let HostEvent::Timer(id) = event {
                fired.push(id);
            }
        }
    }
    assert_eq!(fired.iter().filter(|&&id| id == once).count(), 1);
    assert!(!host.stop_timer(once), "a single shot stops itself");
    assert!(host.stop_timer(every));
    assert!(!host.stop_timer(every));
}

#[test]
fn windows_show_what_the_program_draws_on_the_canvas() {
    let (mut host, handle, figure, window) = shown(&[]);
    drain(&mut host);
    let id = platform_window(&host, window);
    let blank = handle.surface(id).expect("open").into_pixels();
    figure
        .subplot()
        .plot([0.0, 1.0], [0.0, 1.0])
        .expect("a line");
    figure.canvas().draw().expect("the figure draws");
    host.present(window).expect("the window updates");
    drain(&mut host);
    assert_ne!(handle.surface(id).expect("open").into_pixels(), blank);
}

#[test]
fn the_subplot_tool_reports_changes_for_the_program_to_carry_out() {
    use crate::gui::{SubplotChange, SubplotLayout};

    let (mut host, handle, figure, window) = shown(&[]);
    let layout = SubplotLayout::of(&figure);
    host.open_subplot_tool(window, layout, false)
        .expect("the tool opens");
    let main = platform_window(&host, window);
    let tools: Vec<WindowId> = handle
        .windows()
        .into_iter()
        .filter(|&w| w != main)
        .collect();
    let [tool] = tools[..] else {
        panic!("one tool window: {tools:?}");
    };
    // The top border's slider has the focus; Right steps it by 0.005.
    key(&handle, tool, Key::Right);
    let changes: Vec<SubplotChange> = drain(&mut host)
        .into_iter()
        .filter_map(|event| match event {
            HostEvent::SubplotChange { window: w, change } if w == window => Some(change),
            _ => None,
        })
        .collect();
    let [SubplotChange::Adjust(params)] = changes[..] else {
        panic!("one adjustment: {changes:?}");
    };
    assert_eq!(params.top, 0.885);
    assert_eq!(
        figure.subplot_params().top,
        0.88,
        "the program carries it out"
    );
    host.show_subplot_layout(window, SubplotLayout::Params(params), "")
        .expect("the tool shows it");
    // Opening it again raises it; closing the window closes it.
    host.open_subplot_tool(window, SubplotLayout::Params(params), false)
        .expect("the tool is raised");
    assert_eq!(handle.windows().len(), 2);
    host.close(window).expect("the window closes");
    assert_eq!(handle.windows(), Vec::<WindowId>::new());
}
