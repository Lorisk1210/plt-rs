//! Headless tests of figure windows and their dialogs operated from the
//! keyboard alone.

use plt_platform::{ButtonState, EventLoop, HeadlessHandle, Key, KeyEvent, Modifiers};

use super::tests::{mouse, pump};
use super::*;
use crate::artist::ArtistHandle;
use crate::lines::LineProps;

/// Presses and releases `key` with `modifiers` in window `id`; characters
/// typed without Control type their text.
fn press(handle: &HeadlessHandle, id: WindowId, key: Key, modifiers: Modifiers) {
    let text = match key {
        Key::Character(c) if !modifiers.control() && !modifiers.alt() => Some(c.to_string()),
        _ => None,
    };
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = KeyEvent {
            key,
            state,
            repeat: false,
            text: text.clone().filter(|_| state == ButtonState::Pressed),
            modifiers,
            scancode: 0,
        };
        handle
            .inject(id, WindowEvent::Key(event))
            .expect("the window exists");
    }
}

/// Types `text` into window `id`.
fn type_text(handle: &HeadlessHandle, id: WindowId, text: &str) {
    for c in text.chars() {
        press(handle, id, Key::Character(c), Modifiers::NONE);
    }
}

/// A shown 4 × 3 in figure with one labelled curve.
fn shown() -> (Figure, EventLoop, HeadlessHandle, FigureWindow) {
    let figure = Figure::with_size(4.0, 3.0, 100.0);
    let ax = figure.subplot();
    ax.plot_with(
        vec![0.0, 1.0, 2.0],
        vec![1.0, 3.0, 2.0],
        LineProps::default().label("signal"),
    )
    .expect("the curve is drawn");
    let (mut event_loop, handle) = EventLoop::headless();
    let mut window = FigureWindow::open(&mut event_loop.context(), &figure, "Figure 1")
        .expect("the window opens");
    pump(&mut event_loop, &mut window);
    (figure, event_loop, handle, window)
}

#[test]
fn the_toolbar_takes_the_focus_from_the_keyboard_and_leaves_the_canvas_its_keys() {
    let (_figure, mut event_loop, handle, mut window) = shown();
    let id = window.id();
    let focus = |window: &FigureWindow| window.window.ui().focus();
    assert_eq!(focus(&window), Some(window.canvas));
    press(&handle, id, Key::Tab, Modifiers::CONTROL);
    pump(&mut event_loop, &mut window);
    assert_ne!(
        focus(&window),
        Some(window.canvas),
        "Ctrl+Tab leaves the canvas"
    );
    press(&handle, id, Key::Character('p'), Modifiers::NONE);
    pump(&mut event_loop, &mut window);
    assert_eq!(
        window.navigation.mode(),
        NavMode::Pan,
        "the canvas still gets 'p'"
    );
    // Home, then Back and Forward (disabled: skipped), Pan, Zoom,
    // Subplots and Figure options.
    for _ in 0..4 {
        press(&handle, id, Key::Tab, Modifiers::NONE);
    }
    pump(&mut event_loop, &mut window);
    let options = focus(&window).expect("a tool button has the focus");
    press(&handle, id, Key::Character(' '), Modifiers::NONE);
    pump(&mut event_loop, &mut window);
    let dialog = window
        .options_dialog
        .as_ref()
        .expect("Space opened it")
        .id();
    press(&handle, dialog, Key::Escape, Modifiers::NONE);
    pump(&mut event_loop, &mut window);
    assert!(window.options_dialog.is_none(), "Escape closed it");

    // A click on the canvas takes the focus back; clicking the toolbar
    // does not take it, as in Qt.
    let options_button = window
        .window
        .ui()
        .widget_rect(options)
        .expect("the button is laid out")
        .center();
    mouse(&handle, id, ButtonState::Pressed, (200.0, 150.0));
    mouse(&handle, id, ButtonState::Released, (200.0, 150.0));
    pump(&mut event_loop, &mut window);
    assert_eq!(focus(&window), Some(window.canvas));
    let at = (options_button.x, options_button.y);
    mouse(&handle, id, ButtonState::Pressed, at);
    mouse(&handle, id, ButtonState::Released, at);
    pump(&mut event_loop, &mut window);
    assert!(window.options_dialog.is_some(), "the click opened it");
    assert_eq!(focus(&window), Some(window.canvas), "and left the focus");
}

#[test]
fn the_save_dialog_works_from_the_keyboard_and_the_clipboard() {
    let dir = std::env::temp_dir().join(format!("plt-save-keys-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the directory is made");
    crate::rc::context(|| {
        crate::rc::update(|rc| rc.savefig_directory = dir.display().to_string());
        let (_figure, mut event_loop, handle, mut window) = shown();
        press(&handle, window.id(), Key::Character('s'), Modifiers::NONE);
        pump(&mut event_loop, &mut window);
        let dialog = window.save_dialog.as_ref().expect("'s' opens it").id();
        // The file name has the focus, its stem selected.
        press(&handle, dialog, Key::Character('c'), Modifiers::CONTROL);
        pump(&mut event_loop, &mut window);
        assert_eq!(handle.clipboard_text().as_deref(), Some("Figure_1"));
        handle.set_clipboard_text("pasted");
        press(&handle, dialog, Key::Character('v'), Modifiers::CONTROL);
        pump(&mut event_loop, &mut window);
        pump(&mut event_loop, &mut window);
        // Tab to the formats, then the last one, PGF, and back to SVG.
        press(&handle, dialog, Key::Tab, Modifiers::NONE);
        press(&handle, dialog, Key::End, Modifiers::NONE);
        for _ in 0..4 {
            press(&handle, dialog, Key::Left, Modifiers::NONE);
        }
        press(&handle, dialog, Key::Enter, Modifiers::NONE);
        pump(&mut event_loop, &mut window);
        assert!(dir.join("pasted.svg").is_file(), "Enter saved the figure");
        assert!(window.save_dialog.is_none());
    });
    std::fs::remove_dir_all(&dir).expect("the directory is removed");
}

#[test]
fn the_subplot_tool_starts_on_its_first_slider() {
    let (figure, mut event_loop, handle, mut window) = shown();
    let top = figure.subplot_params().top;
    window
        .use_tool(&mut event_loop.context(), Tool::Subplots)
        .expect("the tool opens");
    let tool = window.subplot_tool.as_ref().expect("the tool is open").id();
    pump(&mut event_loop, &mut window);
    press(&handle, tool, Key::Right, Modifiers::NONE);
    press(&handle, tool, Key::Right, Modifiers::NONE);
    pump(&mut event_loop, &mut window);
    let moved = figure.subplot_params().top;
    assert!(
        (moved - (top + 0.02)).abs() < 1e-9,
        "two steps of 0.01 from {top}: {moved}"
    );
    press(&handle, tool, Key::Enter, Modifiers::NONE);
    pump(&mut event_loop, &mut window);
    assert!(window.subplot_tool.is_none(), "Enter pressed Close");
}

#[test]
fn the_options_dialog_switches_tabs_and_applies_from_the_keyboard() {
    let (figure, mut event_loop, handle, mut window) = shown();
    window
        .use_tool(&mut event_loop.context(), Tool::Options)
        .expect("the dialog opens");
    let dialog = window.options_dialog.as_ref().expect("it is open").id();
    pump(&mut event_loop, &mut window);
    // The title has the focus.
    type_text(&handle, dialog, "Keyed");
    // Ctrl+Tab shows the curves, and the focus follows to their list;
    // Tab then reaches the label of the curve, selected to be typed over.
    press(&handle, dialog, Key::Tab, Modifiers::CONTROL);
    pump(&mut event_loop, &mut window);
    press(&handle, dialog, Key::Tab, Modifiers::NONE);
    type_text(&handle, dialog, "renamed");
    press(&handle, dialog, Key::Enter, Modifiers::NONE);
    pump(&mut event_loop, &mut window);
    assert!(window.options_dialog.is_none(), "Enter pressed OK");
    let ax = &figure.axes()[0];
    assert_eq!(ax.shown_title().1, "Keyed");
    assert_eq!(ax.lines()[0].label(), "renamed");
}

#[test]
fn help_opens_with_f1_and_closes_with_enter() {
    let (_figure, mut event_loop, handle, mut window) = shown();
    press(&handle, window.id(), Key::F(1), Modifiers::NONE);
    pump(&mut event_loop, &mut window);
    let help = window.help_dialog.as_ref().expect("F1 opens it").id();
    press(&handle, help, Key::Enter, Modifiers::NONE);
    pump(&mut event_loop, &mut window);
    assert!(window.help_dialog.is_none());
}
