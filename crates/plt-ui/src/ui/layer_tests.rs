//! Tests of keyboard focus traversal, popups, tooltips and natural sizes.

use plt_core::{Path, Point, Rgba};
use plt_platform::{
    ButtonState, Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition, PhysicalSize, WindowEvent,
};

use super::Ui;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};
use crate::widgets::{Button, Flex, Label, TextInput, ToolButton};

const RED: Rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);

/// A focusable widget of a fixed size reporting the keys it receives.
#[derive(Debug)]
struct Field {
    name: &'static str,
    wants_tab: bool,
}

impl Widget<&'static str> for Field {
    fn measure(
        &mut self,
        _cx: &mut LayoutCx<'_, &'static str>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        Ok(constraints.constrain(Size::new(40.0, 20.0)))
    }

    fn paint(&self, _cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, &'static str>, event: &UiEvent, _rect: Rect) {
        if matches!(event, UiEvent::Key(_)) {
            cx.emit(self.name);
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn wants_tab(&self) -> bool {
        self.wants_tab
    }
}

/// A widget whose press opens a popup painting a red square at the
/// window's (150, 150); the next press anywhere reports "outside" and
/// closes it.
#[derive(Debug, Default)]
struct Popper {
    open: bool,
}

impl Widget<&'static str> for Popper {
    fn measure(
        &mut self,
        _cx: &mut LayoutCx<'_, &'static str>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        Ok(constraints.constrain(Size::new(50.0, 20.0)))
    }

    fn paint(&self, _cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        Ok(())
    }

    fn paint_popup(&self, cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        cx.canvas()
            .fill_rect(Rect::new(150.0, 150.0, 10.0, 10.0), RED)
    }

    fn event(&mut self, cx: &mut EventCx<'_, &'static str>, event: &UiEvent, _rect: Rect) {
        if let UiEvent::PointerDown { .. } = event {
            if self.open {
                cx.emit("outside");
                cx.close_popup();
            } else {
                cx.open_popup();
            }
            self.open = !self.open;
        }
    }
}

fn tab(modifiers: Modifiers) -> WindowEvent {
    WindowEvent::Key(KeyEvent {
        key: Key::Tab,
        state: ButtonState::Pressed,
        repeat: false,
        text: Some("\t".into()),
        modifiers,
        scancode: 23,
    })
}

fn click(ui: &mut Ui<&'static str>, x: f64, y: f64) -> Vec<&'static str> {
    let mut messages = Vec::new();
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let event = WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            position: PhysicalPosition::new(x, y),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        messages.extend(ui.handle(&event).unwrap());
    }
    messages
}

fn move_to(ui: &mut Ui<&'static str>, x: f64, y: f64) {
    let event = WindowEvent::CursorMoved {
        position: PhysicalPosition::new(x, y),
        modifiers: Modifiers::NONE,
    };
    ui.handle(&event).unwrap();
}

/// The red channel of the pixel at `(x, y)` of the next frame.
fn red_at(ui: &mut Ui<&'static str>, x: u32, y: u32) -> u8 {
    let frame = ui.paint().unwrap();
    frame.pixels()[((y * frame.width() + x) * 4) as usize]
}

#[test]
fn tab_moves_the_focus_in_tree_order() {
    let mut row = Flex::row();
    let first = row.push(TextInput::new("figure.png", |_| "edited"), 0.0);
    row.push(Label::new("Not focusable"), 0.0);
    let second = row.push(
        Field {
            name: "second",
            wants_tab: false,
        },
        0.0,
    );
    let mut ui = Ui::new(row, PhysicalSize::new(600, 40), 1.0).unwrap();
    assert_eq!(ui.handle(&tab(Modifiers::NONE)).unwrap(), [] as [&str; 0]);
    assert_eq!(ui.focus(), Some(first));
    let selection = ui
        .widget_mut::<TextInput<&'static str>>(first)
        .and_then(|input| input.selection());
    assert_eq!(
        selection,
        Some((0, 10)),
        "a tabbed-to field selects its text"
    );
    ui.handle(&tab(Modifiers::NONE)).unwrap();
    assert_eq!(ui.focus(), Some(second));
    ui.handle(&tab(Modifiers::NONE)).unwrap();
    assert_eq!(ui.focus(), Some(first), "the focus wraps around");
    ui.handle(&tab(Modifiers::SHIFT)).unwrap();
    assert_eq!(ui.focus(), Some(second), "Shift+Tab goes back");
}

#[test]
fn widgets_wanting_tab_receive_it() {
    let field = Field {
        name: "canvas",
        wants_tab: true,
    };
    let mut row = Flex::row();
    let canvas = row.push(field, 0.0);
    let mut ui = Ui::new(row, PhysicalSize::new(100, 40), 1.0).unwrap();
    ui.set_focus(Some(canvas));
    assert_eq!(ui.handle(&tab(Modifiers::NONE)).unwrap(), ["canvas"]);
    assert_eq!(ui.focus(), Some(canvas));
}

#[test]
fn an_open_popup_takes_every_press_and_paints_on_top() {
    let row = Flex::row()
        .with_child(Popper::default())
        .with_flex_child(Button::new("OK", "ok"), 1.0);
    let root = Flex::column().with_child(row);
    let mut ui = Ui::new(root, PhysicalSize::new(200, 200), 1.0).unwrap();
    let background = red_at(&mut ui, 155, 155);
    assert_eq!(click(&mut ui, 10.0, 10.0), [] as [&str; 0]);
    assert!(ui.has_popup());
    assert_eq!(red_at(&mut ui, 155, 155), 255, "the popup is painted");
    assert_eq!(
        click(&mut ui, 150.0, 10.0),
        ["outside"],
        "the press goes to the popup, not the button"
    );
    assert!(!ui.has_popup());
    assert_eq!(red_at(&mut ui, 155, 155), background);
    assert_eq!(click(&mut ui, 150.0, 10.0), ["ok"]);
}

#[test]
fn tooltips_wait_show_and_hide() {
    let icon = Icon::new(24.0).fill(Path::circle(Point::new(12.0, 12.0), 6.0));
    let mut row = Flex::row();
    let tool = row.push(ToolButton::new(icon, "go").with_tooltip("Go there"), 0.0);
    let root = Flex::column().with_child(row);
    let mut ui = Ui::new(root, PhysicalSize::new(200, 100), 1.0).unwrap();
    let bubble = (8, 48);
    let background = red_at(&mut ui, bubble.0, bubble.1);
    move_to(&mut ui, 16.0, 16.0);
    assert_eq!(ui.pending_tooltip(), Some(tool));
    assert_eq!(red_at(&mut ui, bubble.0, bubble.1), background, "not yet");
    ui.show_tooltip();
    assert_eq!(ui.shown_tooltip(), Some(tool));
    assert!(
        red_at(&mut ui, bubble.0, bubble.1) < 100,
        "a dark bubble below"
    );
    move_to(&mut ui, 150.0, 16.0);
    assert_eq!((ui.pending_tooltip(), ui.shown_tooltip()), (None, None));
    assert_eq!(red_at(&mut ui, bubble.0, bubble.1), background);
    move_to(&mut ui, 16.0, 16.0);
    ui.show_tooltip();
    click(&mut ui, 16.0, 16.0);
    assert_eq!(
        (ui.pending_tooltip(), ui.shown_tooltip()),
        (None, None),
        "a click dismisses the tooltip"
    );
}

#[test]
fn natural_sizes_ignore_the_window() {
    let row = Flex::row()
        .with_child(Popper::default())
        .with_child(Popper::default());
    let mut ui = Ui::new(row, PhysicalSize::new(400, 300), 1.0).unwrap();
    assert_eq!(ui.natural_size().unwrap(), Size::new(100.0, 20.0));
    assert!(ui.needs_paint(), "the next frame lays out again");
}
