//! Tests of text fields in a UI: keys, clicks and the clipboard.

use plt_platform::{
    ButtonState, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition, PhysicalSize, WindowEvent,
};

use super::*;
use crate::ui::Ui;
use crate::widget::ClipboardRequest;

/// What the field under test reports.
#[derive(Debug, Clone, PartialEq)]
enum Report {
    Changed(String),
    Submitted(String),
}

/// A focused field showing `text` filling a 300 × 28 UI.
fn field(text: &str, submits: bool) -> Ui<Report> {
    let mut input = TextInput::new(text, Report::Changed);
    if submits {
        input = input.on_submit(Report::Submitted);
    }
    let mut ui = Ui::new(input, PhysicalSize::new(300, 28), 1.0).unwrap();
    ui.set_focus(Some(ui.root_id()));
    ui
}

fn key(key: Key, text: Option<&str>, modifiers: Modifiers) -> WindowEvent {
    WindowEvent::Key(KeyEvent {
        key,
        state: ButtonState::Pressed,
        repeat: false,
        text: text.map(str::to_owned),
        modifiers,
        scancode: 0,
    })
}

fn ctrl(c: char) -> WindowEvent {
    key(Key::Character(c), None, Modifiers::CONTROL)
}

fn typed(ui: &mut Ui<Report>, text: &str) -> Vec<Report> {
    text.chars()
        .flat_map(|c| {
            let text = c.to_string();
            ui.handle(&key(Key::Character(c), Some(&text), Modifiers::NONE))
                .unwrap()
        })
        .collect()
}

fn input(ui: &mut Ui<Report>) -> &mut TextInput<Report> {
    let root = ui.root_id();
    ui.widget_mut::<TextInput<Report>>(root).unwrap()
}

#[test]
fn typing_reports_every_change() {
    let mut ui = field("0.1", false);
    assert_eq!(
        typed(&mut ui, "25"),
        [
            Report::Changed("0.12".into()),
            Report::Changed("0.125".into())
        ]
    );
    let messages = ui
        .handle(&key(Key::Backspace, None, Modifiers::CONTROL))
        .unwrap();
    assert_eq!(messages, [Report::Changed("0.".into())]);
}

#[test]
fn enter_submits_only_fields_that_submit() {
    let mut plain = field("a", false);
    let enter = plain
        .handle(&key(Key::Enter, None, Modifiers::NONE))
        .unwrap();
    assert_eq!(enter, []);
    let mut submitting = field("a", true);
    assert_eq!(
        submitting
            .handle(&key(Key::Enter, None, Modifiers::NONE))
            .unwrap(),
        [Report::Submitted("a".into())]
    );
}

#[test]
fn copying_cutting_and_pasting_go_through_the_clipboard() {
    let mut ui = field("figure.png", false);
    input(&mut ui).select(0, 6);
    ui.handle(&ctrl('c')).unwrap();
    assert_eq!(
        ui.take_clipboard_requests(),
        [ClipboardRequest::Copy("figure".into())]
    );
    assert_eq!(
        ui.handle(&ctrl('x')).unwrap(),
        [Report::Changed(".png".into())]
    );
    assert_eq!(
        ui.take_clipboard_requests(),
        [ClipboardRequest::Copy("figure".into())]
    );
    ui.handle(&key(Key::End, None, Modifiers::NONE)).unwrap();
    ui.handle(&key(Key::Insert, None, Modifiers::SHIFT))
        .unwrap();
    assert_eq!(ui.take_clipboard_requests(), [ClipboardRequest::Paste]);
    let pasted = WindowEvent::ClipboardText(Some("plot 2\n".into()));
    assert_eq!(
        ui.handle(&pasted).unwrap(),
        [Report::Changed(".pngplot 2".into())]
    );
    assert!(
        ui.handle(&pasted).unwrap().is_empty(),
        "only what was asked for is pasted"
    );
    ui.handle(&ctrl('v')).unwrap();
    assert_eq!(ui.take_clipboard_requests(), [ClipboardRequest::Paste]);
    let lines = WindowEvent::ClipboardText(Some("a\tb\nc".into()));
    ui.handle(&lines).unwrap();
    assert_eq!(input(&mut ui).text(), ".pngplot 2a b c");
    ui.handle(&ctrl('c')).unwrap();
    assert!(
        ui.take_clipboard_requests().is_empty(),
        "nothing selected, nothing copied"
    );
}

#[test]
fn undo_and_redo() {
    let mut ui = field("0.5", false);
    ui.handle(&ctrl('a')).unwrap();
    typed(&mut ui, "0.25");
    assert_eq!(
        ui.handle(&ctrl('z')).unwrap(),
        [Report::Changed("0.5".into())]
    );
    assert_eq!(
        ui.handle(&key(
            Key::Character('Z'),
            None,
            Modifiers::CONTROL | Modifiers::SHIFT
        ))
        .unwrap(),
        [Report::Changed("0.25".into())]
    );
    ui.handle(&ctrl('z')).unwrap();
    assert_eq!(
        ui.handle(&ctrl('y')).unwrap(),
        [Report::Changed("0.25".into())]
    );
    assert!(ui.handle(&ctrl('y')).unwrap().is_empty(), "nothing to redo");
}

fn click(ui: &mut Ui<Report>, x: f64, click_count: u32, modifiers: Modifiers) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            position: PhysicalPosition::new(x, 14.0),
            modifiers,
            click_count,
        };
        ui.handle(&event).unwrap();
    }
}

#[test]
fn clicks_place_the_caret_and_select_words_and_lines() {
    let mut ui = field("plot data.png", false);
    ui.paint().unwrap();
    let after = |ui: &mut Ui<Report>, index: usize| {
        let input = input(ui);
        input.x_of(index) + input.padding + 1.0
    };
    let x = after(&mut ui, 2);
    click(&mut ui, x, 2, Modifiers::NONE);
    assert_eq!(input(&mut ui).selection(), Some((0, 4)), "a word");
    click(&mut ui, x, 3, Modifiers::NONE);
    assert_eq!(input(&mut ui).selection(), Some((0, 13)), "everything");
    click(&mut ui, x, 1, Modifiers::NONE);
    assert_eq!(input(&mut ui).selection(), None);
    let end = after(&mut ui, 9);
    click(&mut ui, end, 1, Modifiers::SHIFT);
    assert_eq!(input(&mut ui).selection(), Some((2, 9)), "Shift extends");
}

#[test]
fn clicks_find_the_nearest_boundary() {
    // Kerned pairs, a combining mark, a zero-width joiner and wide signs.
    let mut ui = field("AVAToé\u{301}x\u{200D}y 😀W.", false);
    ui.paint().unwrap();
    let input = input(&mut ui);
    let text = input.text().to_owned();
    let boundaries: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    let nearest = |x: f64| {
        boundaries
            .iter()
            .copied()
            .map(|i| (i, (input.x_of(i) - x).abs()))
            .reduce(|best, next| if next.1 < best.1 { next } else { best })
            .map(|(i, _)| i)
    };
    let width = input.x_of(text.len());
    for step in -10..=220 {
        let x = f64::from(step) * (width + 20.0) / 200.0 - 10.0;
        assert_eq!(Some(input.index_at(x)), nearest(x), "at {x}");
    }
}

#[test]
fn clicks_in_long_text_take_few_layouts() {
    // Found by fuzzing the figure options: a click into a long pasted
    // text laid out the text before every boundary, for minutes.
    let mut ui = field(&"9".repeat(50_000), false);
    ui.paint().unwrap();
    click(&mut ui, 150.0, 1, Modifiers::NONE);
    let caret = input(&mut ui).editor.caret();
    assert!(caret > 0 && caret < 50_000, "caret at {caret}");
}

#[test]
fn text_typed_before_the_field_is_laid_out_shows_once_it_is() {
    // A field on a page not yet shown is typed into at no width at all.
    let blank = TextInput::new("", Report::Changed);
    let mut ui = Ui::new(blank, PhysicalSize::new(1, 28), 1.0).unwrap();
    ui.set_focus(Some(ui.root_id()));
    typed(&mut ui, "thick");
    assert!(input(&mut ui).scroll > 0.0, "the caret kept in no width");
    ui.resize(PhysicalSize::new(300, 28), 1.0).unwrap();
    assert_eq!(input(&mut ui).scroll, 0.0, "the whole text shows");
}

#[test]
fn a_refused_text_is_marked_until_edited() {
    let mut ui = field("ten", false);
    input(&mut ui).set_invalid(true);
    let pixels = |ui: &mut Ui<Report>| ui.paint().unwrap().pixels().to_vec();
    let marked = pixels(&mut ui);
    // The outline on the left edge turns from the accent's blue to the
    // danger's red.
    let edge = &marked[14 * 300 * 4..14 * 300 * 4 + 4];
    assert!(edge[0] > edge[2], "red: {edge:?}");
    ui.handle(&key(Key::Left, None, Modifiers::NONE)).unwrap();
    assert!(input(&mut ui).invalid(), "moving the caret keeps the mark");
    typed(&mut ui, "s");
    assert!(!input(&mut ui).invalid(), "editing clears it");
    assert_ne!(pixels(&mut ui), marked);
}
