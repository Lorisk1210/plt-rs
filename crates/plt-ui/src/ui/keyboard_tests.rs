//! Tests of keyboard operation: focus traversal and rings, the default
//! button, tab shortcuts, focus on hidden pages, tool buttons and
//! surfaces.

use plt_core::Path;
use plt_platform::{
    ButtonState, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition, PhysicalSize, WindowEvent,
};

use super::Ui;
use crate::icon::Icon;
use crate::widgets::{
    Button, Checkbox, Flex, Slider, Stack, Surface, SurfaceEvent, Tabs, TextInput, ToolButton,
};
use crate::{UiEvent, WidgetId};

type Message = &'static str;

fn press(key: Key, modifiers: Modifiers) -> WindowEvent {
    let text = match key {
        Key::Character(c) if !modifiers.control() => Some(c.to_string()),
        _ => None,
    };
    WindowEvent::Key(KeyEvent {
        key,
        state: ButtonState::Pressed,
        repeat: false,
        text,
        modifiers,
        scancode: 0,
    })
}

fn tab(ui: &mut Ui<Message>) {
    ui.handle(&press(Key::Tab, Modifiers::NONE)).unwrap();
}

fn click(ui: &mut Ui<Message>, id: WidgetId) -> Vec<Message> {
    let center = ui.widget_rect(id).unwrap().center();
    let mut messages = Vec::new();
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            position: PhysicalPosition::new(center.x, center.y),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        messages.extend(ui.handle(&event).unwrap());
    }
    messages
}

fn square() -> Icon {
    Icon::new(24.0).fill(Path::circle(plt_core::Point::new(12.0, 12.0), 6.0))
}

/// The RGBA pixel at `(x, y)` of the next frame.
fn pixel(ui: &mut Ui<Message>, x: f64, y: f64) -> [u8; 4] {
    let frame = ui.paint().unwrap();
    let at = (y as usize * frame.width() as usize + x as usize) * 4;
    frame.pixels()[at..at + 4].try_into().unwrap()
}

/// A dialog-like column: a field, a check box, a slider and two buttons,
/// the last one primary.
fn dialog() -> (Ui<Message>, [WidgetId; 5]) {
    let mut column = Flex::column()
        .with_padding(crate::Spacing::Lg)
        .with_spacing(crate::Spacing::Md);
    let field = column.push(TextInput::new("0.5", |_| "edited"), 0.0);
    let check = column.push(Checkbox::new("Legend", |_| "toggled"), 0.0);
    let slider = column.push(Slider::new(0.0, 1.0, 0.5, |_| "slid"), 0.0);
    let mut footer = Flex::row().with_spacing(crate::Spacing::Sm);
    let cancel = footer.push(Button::new("Cancel", "cancel"), 0.0);
    let ok = footer.push(Button::new("OK", "ok").primary(), 0.0);
    column.push(footer, 0.0);
    let ui = Ui::new(column, PhysicalSize::new(300, 260), 1.0).unwrap();
    (ui, [field, check, slider, cancel, ok])
}

#[test]
fn tab_reaches_every_control_in_reading_order() {
    let (mut ui, ids) = dialog();
    let mut order = Vec::new();
    for _ in 0..ids.len() {
        tab(&mut ui);
        order.push(ui.focus().unwrap());
    }
    assert_eq!(order, ids);
    ui.handle(&press(Key::Tab, Modifiers::SHIFT)).unwrap();
    assert_eq!(ui.focus(), Some(ids[3]), "Shift+Tab goes back");
}

#[test]
fn focus_rings_show_after_tab_and_hide_after_a_click() {
    let (mut ui, [_, check, ..]) = dialog();
    let rect = ui.widget_rect(check).unwrap();
    // Just left of the box, where the ring is drawn.
    let (x, y) = (rect.x - 1.0, rect.center().y);
    let plain = pixel(&mut ui, x, y);
    ui.set_focus(Some(check));
    assert_eq!(
        pixel(&mut ui, x, y),
        plain,
        "no ring before the keyboard is used"
    );
    ui.handle(&press(Key::Tab, Modifiers::SHIFT)).unwrap();
    tab(&mut ui);
    assert_eq!(ui.focus(), Some(check));
    let ring = pixel(&mut ui, x, y);
    assert_ne!(ring, plain, "the ring shows");
    assert!(ring[2] > ring[0], "in the accent blue: {ring:?}");
    click(&mut ui, check);
    assert_eq!(pixel(&mut ui, x, y), plain, "a click hides the ring");
}

#[test]
fn enter_presses_the_default_button_unless_the_focus_uses_it() {
    let (mut ui, [field, check, slider, cancel, _]) = dialog();
    for id in [field, check, slider] {
        ui.set_focus(Some(id));
        let messages = ui.handle(&press(Key::Enter, Modifiers::NONE)).unwrap();
        assert_eq!(messages, ["ok"]);
    }
    ui.set_focus(Some(cancel));
    let messages = ui.handle(&press(Key::Enter, Modifiers::NONE)).unwrap();
    assert_eq!(messages, ["cancel"], "a focused button takes Enter");
    let messages = ui
        .handle(&press(Key::Character(' '), Modifiers::NONE))
        .unwrap();
    assert_eq!(messages, ["cancel"], "and Space");
    ui.set_focus(Some(check));
    let messages = ui
        .handle(&press(Key::Character(' '), Modifiers::NONE))
        .unwrap();
    assert_eq!(messages, ["toggled"]);
    let messages = ui.handle(&press(Key::Escape, Modifiers::NONE)).unwrap();
    assert!(messages.is_empty(), "Escape is the dialog's");
}

#[test]
fn the_first_control_takes_the_focus() {
    let (mut ui, [field, ..]) = dialog();
    ui.focus_first();
    assert_eq!(ui.focus(), Some(field));
}

/// Two pages under a tab bar, each with a field, and a button below.
fn tabbed() -> (Ui<usize>, Vec<WidgetId>, WidgetId, WidgetId) {
    let mut column = Flex::column();
    let tabs = column.push(Tabs::new(vec!["One".into(), "Two".into()], |i| i), 0.0);
    let mut stack = Stack::new();
    let mut fields = Vec::new();
    for page in 0..2 {
        let mut row = Flex::row();
        fields.push(row.push(TextInput::new(format!("page {page}"), |_| 99), 1.0));
        stack.push(row);
    }
    let stack = column.push(stack, 1.0);
    column.push(Button::new("OK", 100), 0.0);
    let ui = Ui::new(column, PhysicalSize::new(300, 200), 1.0).unwrap();
    (ui, fields, tabs, stack)
}

/// Shows `page` of `stack`, as the application does when told.
fn show(ui: &mut Ui<usize>, stack: WidgetId, page: usize) {
    ui.widget_mut::<Stack<usize>>(stack).unwrap().show(page);
}

#[test]
fn ctrl_tab_switches_tabs_and_the_focus_follows_into_the_page() {
    let (mut ui, fields, tabs, stack) = tabbed();
    ui.set_focus(Some(fields[0]));
    let next = WindowEvent::Key(KeyEvent {
        key: Key::Tab,
        state: ButtonState::Pressed,
        repeat: false,
        text: None,
        modifiers: Modifiers::CONTROL,
        scancode: 0,
    });
    assert_eq!(ui.handle(&next).unwrap(), [1]);
    show(&mut ui, stack, 1);
    ui.paint().unwrap();
    assert_eq!(ui.focus(), Some(fields[1]), "the field of the page shown");
    assert_eq!(
        ui.handle(&press(Key::PageUp, Modifiers::CONTROL)).unwrap(),
        [0]
    );
    show(&mut ui, stack, 0);
    // The next key fixes the focus before it is delivered.
    let typed = ui
        .handle(&press(Key::Character('x'), Modifiers::NONE))
        .unwrap();
    assert_eq!(typed, [99]);
    assert_eq!(ui.focus(), Some(fields[0]));
    let bar = ui.widget_rect(tabs).unwrap();
    let first_tab = WindowEvent::MouseInput {
        button: MouseButton::Left,
        state: ButtonState::Pressed,
        position: PhysicalPosition::new(bar.x + 5.0, bar.center().y),
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    ui.handle(&first_tab).unwrap();
    assert_eq!(
        ui.focus(),
        Some(fields[0]),
        "clicking a tab leaves the focus"
    );
}

#[test]
fn arrows_select_tabs_once_tab_focuses_the_bar() {
    let (mut ui, _, tabs, _) = tabbed();
    ui.handle(&press(Key::Tab, Modifiers::NONE)).unwrap();
    assert_eq!(ui.focus(), Some(tabs));
    assert_eq!(ui.handle(&press(Key::Right, Modifiers::NONE)).unwrap(), [1]);
}

/// A canvas above a toolbar of two tool buttons.
fn figure_window() -> (Ui<Message>, WidgetId, [WidgetId; 2]) {
    let canvas = Surface::new(|event| match event {
        SurfaceEvent::Input(UiEvent::Key(key)) if key.state == ButtonState::Pressed => {
            Some("canvas key")
        }
        SurfaceEvent::Input(UiEvent::Key(_)) => Some("canvas release"),
        _ => None,
    })
    .with_unused_keys();
    let mut root = Flex::column();
    let canvas = root.push(canvas, 1.0);
    let mut toolbar = Flex::row();
    let home = toolbar.push(
        ToolButton::new(square(), "home").with_tooltip("Home (h)"),
        0.0,
    );
    let pan = toolbar.push(ToolButton::new(square(), "pan"), 0.0);
    root.push(toolbar, 0.0);
    let mut ui = Ui::new(root, PhysicalSize::new(200, 150), 1.0).unwrap();
    ui.set_focus(Some(canvas));
    (ui, canvas, [home, pan])
}

#[test]
fn tool_buttons_take_the_focus_from_the_keyboard_only() {
    let (mut ui, canvas, [home, pan]) = figure_window();
    assert_eq!(click(&mut ui, pan), ["pan"]);
    assert_eq!(
        ui.focus(),
        Some(canvas),
        "the click leaves the canvas focused"
    );
    assert_eq!(
        ui.handle(&press(Key::Tab, Modifiers::NONE)).unwrap(),
        ["canvas key"],
        "the canvas takes Tab"
    );
    ui.handle(&press(Key::Tab, Modifiers::CONTROL)).unwrap();
    assert_eq!(ui.focus(), Some(home), "Ctrl+Tab moves on from the canvas");
    assert_eq!(
        ui.pending_tooltip(),
        Some(home),
        "with the button's tooltip"
    );
    assert_eq!(
        ui.handle(&press(Key::Character(' '), Modifiers::NONE))
            .unwrap(),
        ["home"]
    );
    assert_eq!(ui.pending_tooltip(), None, "a used key dismisses it");
    assert_eq!(
        ui.handle(&press(Key::Character('h'), Modifiers::NONE))
            .unwrap(),
        ["canvas key"],
        "keys the button leaves go to the canvas"
    );
    let release = WindowEvent::Key(KeyEvent {
        key: Key::Character('h'),
        state: ButtonState::Released,
        repeat: false,
        text: None,
        modifiers: Modifiers::NONE,
        scancode: 0,
    });
    assert_eq!(
        ui.handle(&release).unwrap(),
        ["canvas release"],
        "and so do their releases"
    );
    tab(&mut ui);
    assert_eq!(ui.focus(), Some(pan));
    ui.widget_mut::<ToolButton<Message>>(pan)
        .unwrap()
        .set_enabled(false);
    ui.paint().unwrap();
    assert_eq!(
        ui.focus(),
        Some(canvas),
        "a disabled button gives the focus on"
    );
}
