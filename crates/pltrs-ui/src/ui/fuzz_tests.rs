//! Randomised robustness tests of the widgets: a dialog of tabs over a
//! form (a text field, a choice, a drop-down, a slider, a check box), a
//! list, and a second field atop rows of shortcuts in a scrolled view,
//! with Cancel and OK below, gets random keys
//! (text editing, selection, undo and redo, Tab and Ctrl+Tab, type-ahead
//! in the list and the drop-down), typing, clicks, drags, wheel turns,
//! resizes, scale changes and clipboard answers, while the application
//! changes the widgets between events as dialogs do: new text, any
//! selection (byte offsets inside characters and past the end), new
//! rows, out-of-range choices, pages and values. Nothing may panic or
//! take long.

use pltrs_platform::{
    ButtonState, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition, PhysicalSize,
    ScrollDelta, WindowEvent,
};

use super::Ui;
use crate::fuzz::{Gen, fuzz};
use crate::widget::ClipboardRequest;
use crate::widgets::{
    Button, Checkbox, Choice, Dropdown, Flex, Form, Keycaps, List, ListItem, Scroll, Slider, Stack,
    Tabs, TextInput,
};
use crate::{Spacing, WidgetId};

/// What the widgets report.
#[derive(Clone, Debug, PartialEq)]
enum Message {
    Text(String),
    Submitted(String),
    Chosen(usize),
    Picked(usize),
    Slid(f64),
    Checked(bool),
    Tab(usize),
    Row(usize),
    Opened(usize),
    Button(&'static str),
}

/// Labels for options and rows: ASCII, letters whose case changes their
/// length (`İ`, `ß`, `ﬃ`), combining marks, emoji and an empty one.
const LABELS: [&str; 10] = [
    "solid",
    "Dashed",
    "İstanbul",
    "ßtraße",
    "ﬃx",
    "e\u{301}cole",
    "😀 smile",
    "",
    "solid",
    "Σίσυφος",
];

/// The widgets the application updates.
#[derive(Clone, Copy, Debug)]
struct Ids {
    tabs: WidgetId,
    stack: WidgetId,
    field: WidgetId,
    choice: WidgetId,
    dropdown: WidgetId,
    slider: WidgetId,
    check: WidgetId,
    list: WidgetId,
}

fn labels(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| LABELS[i % LABELS.len()].to_owned())
        .collect()
}

fn rows(count: usize) -> Vec<ListItem> {
    (0..count)
        .map(|i| ListItem::new(format!("{} {i}", LABELS[i % LABELS.len()])))
        .collect()
}

/// The dialog under test.
fn dialog() -> (Ui<Message>, Ids) {
    let mut column = Flex::column()
        .with_padding(Spacing::Sm)
        .with_spacing(Spacing::Sm);
    let tabs = column.push(Tabs::new(labels(3), Message::Tab), 0.0);
    let mut stack = Stack::new();
    let mut form = Form::new();
    let field = form.row(
        "Name",
        TextInput::new("Figure 1.png", Message::Text).on_submit(Message::Submitted),
    );
    let choice = form.row("Format", Choice::new(labels(4), Message::Chosen));
    let dropdown = form.row("Style", Dropdown::new(labels(10), Message::Picked));
    let slider = form.row("Size", Slider::new(-1.0, 1.0, 0.5, Message::Slid));
    let check = form.row("Grid", Checkbox::new("Grid", Message::Checked));
    stack.push(form);
    let list = stack.push(
        List::new(rows(30), Message::Row)
            .on_activate(Message::Opened)
            .with_visible_rows(5),
    );
    let mut third = Form::new();
    third.row("Find", TextInput::new("", Message::Text));
    for label in labels(12) {
        third.row(
            label.clone(),
            Keycaps::new(vec![label, "Ctrl+K".to_owned()]),
        );
    }
    stack.push(Scroll::new(third));
    let stack = column.push(stack, 1.0);
    let mut footer = Flex::row().with_spacing(Spacing::Sm);
    footer.push(Button::new("Cancel", Message::Button("cancel")), 0.0);
    footer.push(Button::new("OK", Message::Button("ok")).primary(), 0.0);
    column.push(footer, 0.0);
    let ui = Ui::new(column, PhysicalSize::new(360, 320), 1.0).expect("the dialog lays out");
    let ids = Ids {
        tabs,
        stack,
        field,
        choice,
        dropdown,
        slider,
        check,
        list,
    };
    (ui, ids)
}

/// Keys of fields, lists, drop-downs and focus traversal.
const KEYS: [Key; 16] = [
    Key::Tab,
    Key::Enter,
    Key::Escape,
    Key::Backspace,
    Key::Delete,
    Key::Left,
    Key::Right,
    Key::Up,
    Key::Down,
    Key::Home,
    Key::End,
    Key::PageUp,
    Key::PageDown,
    Key::Insert,
    Key::Shift,
    Key::Control,
];

/// Characters typed: letters for type-ahead and shortcuts (select all,
/// copy, cut, paste, undo, redo), and characters whose case or width is
/// unusual.
const CHARS: [char; 24] = [
    'a', 'c', 'x', 'v', 'z', 'y', 's', 'd', 'i', 'e', '1', '.', ' ', 'İ', 'ı', 'ß', 'ẞ', 'ﬃ', 'σ',
    'Σ', '😀', '\u{301}', '\u{200D}', '\u{0}',
];

/// A widget an input aims at, or anywhere.
#[derive(Clone, Copy, Debug)]
enum Aim {
    Widget(usize),
    Anywhere(f64, f64),
}

/// One input or change by the application.
#[derive(Clone, Debug)]
enum Step {
    Key(Key, Modifiers),
    Type(String),
    Click(Aim, (f64, f64), MouseButton, u32),
    Drag(Aim, (f64, f64), (f64, f64)),
    Wheel(Aim, f64),
    Hover(Aim),
    Resize(u32, u32),
    Scale(f64),
    SetText(String),
    Select(usize, usize),
    SetRows(usize, Option<usize>),
    Choose(usize),
    Show(usize),
    Slide(f64),
    Paint,
}

impl Aim {
    fn generate(g: &mut Gen) -> Self {
        if g.chance(0.8) {
            Self::Widget(g.below(8))
        } else {
            Self::Anywhere(g.between(-0.1, 1.1), g.between(-0.1, 1.1))
        }
    }
}

impl Step {
    fn generate(g: &mut Gen) -> Self {
        let near = |g: &mut Gen| (g.between(-0.6, 0.6), g.between(-0.6, 0.6));
        match g.below(40) {
            0..12 => {
                let modifiers = g.pick(&[
                    Modifiers::NONE,
                    Modifiers::NONE,
                    Modifiers::SHIFT,
                    Modifiers::CONTROL,
                    Modifiers::CONTROL | Modifiers::SHIFT,
                ]);
                let key = if g.chance(0.6) {
                    g.pick(&KEYS)
                } else {
                    Key::Character(g.pick(&CHARS))
                };
                Self::Key(key, modifiers)
            }
            12..18 => Self::Type((0..=g.len(10)).map(|_| g.pick(&CHARS)).collect()),
            18..24 => Self::Click(
                Aim::generate(g),
                near(g),
                g.pick(&[MouseButton::Left, MouseButton::Left, MouseButton::Right]),
                g.pick(&[1, 1, 2, 3]),
            ),
            24..26 => Self::Drag(Aim::generate(g), near(g), near(g)),
            26 => Self::Wheel(Aim::generate(g), g.pick(&[-3.0, -1.0, 1.0, 40.0])),
            27 => Self::Hover(Aim::generate(g)),
            28 => Self::Resize(g.pick(&[0, 1, 50, 360, 900]), g.pick(&[0, 1, 40, 320])),
            29 => Self::Scale(g.pick(&[0.5, 1.0, 2.0, 1e-9, 1e-300, 1e9])),
            30 | 31 => Self::SetText(g.soup(&LABELS, 4)),
            32 | 33 => Self::Select(g.below(40), g.below(40)),
            34 => Self::SetRows(g.len(40), g.chance(0.7).then(|| g.below(45))),
            35 => Self::Choose(g.below(12)),
            36 => Self::Show(g.below(4)),
            37 => Self::Slide(g.number()),
            _ => Self::Paint,
        }
    }
}

/// A key press, typing its text unless Control is held.
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

/// The point `aim` names, `offset` (in widget sizes) from a widget's
/// centre.
fn point(ui: &Ui<Message>, ids: &Ids, aim: Aim, (dx, dy): (f64, f64)) -> PhysicalPosition {
    let size = ui.size();
    let (width, height) = (f64::from(size.width), f64::from(size.height));
    match aim {
        Aim::Anywhere(fx, fy) => PhysicalPosition::new(fx * width, fy * height),
        Aim::Widget(index) => {
            let id = [
                ids.tabs,
                ids.stack,
                ids.field,
                ids.choice,
                ids.dropdown,
                ids.slider,
                ids.check,
                ids.list,
            ][index];
            let rect = ui.widget_rect(id).unwrap_or_default();
            let center = rect.center();
            PhysicalPosition::new(center.x + dx * rect.width, center.y + dy * rect.height)
        }
    }
}

fn mouse(
    position: PhysicalPosition,
    button: MouseButton,
    state: ButtonState,
    clicks: u32,
) -> WindowEvent {
    WindowEvent::MouseInput {
        button,
        state,
        position,
        modifiers: Modifiers::NONE,
        click_count: clicks,
    }
}

fn moved(position: PhysicalPosition) -> WindowEvent {
    WindowEvent::CursorMoved {
        position,
        modifiers: Modifiers::NONE,
    }
}

/// The events of an input step; changes by the application are made at
/// once.
fn events(step: &Step, ui: &mut Ui<Message>, ids: &Ids) -> Vec<WindowEvent> {
    match step {
        Step::Key(key, modifiers) => vec![press(*key, *modifiers)],
        Step::Type(text) => text
            .chars()
            .map(|c| press(Key::Character(c), Modifiers::NONE))
            .collect(),
        Step::Click(aim, offset, button, clicks) => {
            let at = point(ui, ids, *aim, *offset);
            vec![
                moved(at),
                mouse(at, *button, ButtonState::Pressed, *clicks),
                mouse(at, *button, ButtonState::Released, *clicks),
            ]
        }
        Step::Drag(aim, from, to) => {
            let (from, to) = (point(ui, ids, *aim, *from), point(ui, ids, *aim, *to));
            vec![
                moved(from),
                mouse(from, MouseButton::Left, ButtonState::Pressed, 1),
                moved(to),
                mouse(to, MouseButton::Left, ButtonState::Released, 1),
            ]
        }
        Step::Wheel(aim, notches) => vec![WindowEvent::MouseWheel {
            delta: ScrollDelta::Lines {
                x: 0.0,
                y: *notches,
            },
            position: point(ui, ids, *aim, (0.0, 0.0)),
            modifiers: Modifiers::NONE,
        }],
        Step::Hover(aim) => vec![moved(point(ui, ids, *aim, (0.0, 0.0)))],
        Step::Resize(width, height) => {
            vec![WindowEvent::Resized(PhysicalSize::new(*width, *height))]
        }
        Step::Scale(scale) => vec![WindowEvent::ScaleFactorChanged(*scale)],
        _ => {
            change(step, ui, ids);
            Vec::new()
        }
    }
}

/// Makes a change of the application.
fn change(step: &Step, ui: &mut Ui<Message>, ids: &Ids) {
    match step {
        Step::SetText(text) => {
            if let Some(field) = ui.widget_mut::<TextInput<Message>>(ids.field) {
                field.set_text(text.clone());
            }
        }
        Step::Select(start, end) => {
            if let Some(field) = ui.widget_mut::<TextInput<Message>>(ids.field) {
                field.select(*start, *end);
            }
        }
        Step::SetRows(count, selected) => {
            if let Some(list) = ui.widget_mut::<List<Message>>(ids.list) {
                list.set_items(rows(*count));
                list.set_selected(*selected);
            }
        }
        Step::Choose(index) => {
            if let Some(choice) = ui.widget_mut::<Choice<Message>>(ids.choice) {
                choice.set_chosen(*index);
            }
            if let Some(dropdown) = ui.widget_mut::<Dropdown<Message>>(ids.dropdown) {
                dropdown.set_chosen(*index);
            }
            if let Some(tabs) = ui.widget_mut::<Tabs<Message>>(ids.tabs) {
                tabs.set_selected(*index);
            }
        }
        Step::Show(page) => show(ui, ids, *page),
        Step::Slide(value) => {
            if let Some(slider) = ui.widget_mut::<Slider<Message>>(ids.slider) {
                slider.set_value(*value);
            }
            if let Some(check) = ui.widget_mut::<Checkbox<Message>>(ids.check) {
                check.set_checked(value.is_sign_positive());
            }
        }
        _ => {}
    }
}

fn show(ui: &mut Ui<Message>, ids: &Ids, page: usize) {
    if let Some(stack) = ui.widget_mut::<Stack<Message>>(ids.stack) {
        stack.show(page);
    }
}

/// A session: inputs, and the text the clipboard answers pastes with.
#[derive(Debug)]
struct Session {
    steps: Vec<Step>,
    clipboard: Option<String>,
}

/// Runs a session; what widgets ask of the clipboard is answered as an
/// application does, and tab selections show their page.
fn run(session: &Session) {
    let (mut ui, ids) = dialog();
    ui.focus_first();
    for step in &session.steps {
        for event in events(step, &mut ui, &ids) {
            let Ok(messages) = ui.handle(&event) else {
                continue;
            };
            for message in messages {
                if let Message::Tab(page) = message {
                    show(&mut ui, &ids, page);
                }
            }
            for request in ui.take_clipboard_requests() {
                if request == ClipboardRequest::Paste {
                    let answer = WindowEvent::ClipboardText(session.clipboard.clone());
                    let _ = ui.handle(&answer);
                }
            }
        }
        if matches!(step, Step::Paint) {
            let _ = ui.paint();
        }
    }
    let _ = ui.paint();
}

#[test]
fn widgets_survive_random_input() {
    fuzz(
        "ui widgets",
        600,
        |g| Session {
            steps: (0..g.len(60)).map(|_| Step::generate(g)).collect(),
            clipboard: g.chance(0.8).then(|| {
                if g.chance(0.05) {
                    "x".repeat(20_000)
                } else {
                    g.soup(&LABELS, 5)
                }
            }),
        },
        run,
    );
}
