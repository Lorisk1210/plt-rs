//! Tests of layout, painting and input routing.

use plt_core::Rgba;
use plt_platform::{Key, KeyEvent, Modifiers, MouseButton, PhysicalPosition, ScrollDelta};
use plt_raster::Rgba8;

use super::*;
use crate::icon::Icon;
use crate::widget::{EventCx, PaintCx};
use crate::widgets::{Button, Flex, Label, Separator, Surface, SurfaceEvent, ToolButton};

/// A widget reporting every event it receives as a message
/// `(name, event)`.
#[derive(Debug)]
struct Probe {
    name: &'static str,
    focusable: bool,
}

type Log = (&'static str, UiEvent);

impl Widget<Log> for Probe {
    fn measure(&mut self, _cx: &mut LayoutCx<'_, Log>, c: Constraints) -> Result<Size, UiError> {
        Ok(c.constrain(Size::INFINITE))
    }

    fn paint(&self, _cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, Log>, event: &UiEvent, _rect: Rect) {
        cx.emit((self.name, event.clone()));
    }

    fn accepts_focus(&self) -> bool {
        self.focusable
    }
}

/// Two probes side by side in a 200×100 UI: "left" (focusable) and
/// "right".
fn probes() -> Ui<Log> {
    let left = Probe {
        name: "left",
        focusable: true,
    };
    let right = Probe {
        name: "right",
        focusable: false,
    };
    let row = Flex::row()
        .with_flex_child(left, 1.0)
        .with_flex_child(right, 1.0);
    let mut ui = Ui::new(row, PhysicalSize::new(200, 100), 1.0).unwrap();
    ui.paint().unwrap();
    ui
}

fn at(x: f64, y: f64) -> PhysicalPosition {
    PhysicalPosition::new(x, y)
}

fn moved(x: f64, y: f64) -> WindowEvent {
    WindowEvent::CursorMoved {
        position: at(x, y),
        modifiers: Modifiers::NONE,
    }
}

fn button(state: ButtonState, x: f64, y: f64) -> WindowEvent {
    WindowEvent::MouseInput {
        button: MouseButton::Left,
        state,
        position: at(x, y),
        modifiers: Modifiers::NONE,
        click_count: 1,
    }
}

fn click(ui: &mut Ui<Log>, x: f64, y: f64) {
    ui.handle(&button(ButtonState::Pressed, x, y)).unwrap();
    ui.handle(&button(ButtonState::Released, x, y)).unwrap();
}

fn key() -> WindowEvent {
    WindowEvent::Key(KeyEvent {
        key: Key::Character('k'),
        state: ButtonState::Pressed,
        repeat: false,
        text: Some("k".into()),
        modifiers: Modifiers::NONE,
        scancode: 45,
    })
}

/// `(name, event kind)` pairs, for compact assertions.
fn kinds(log: &[Log]) -> Vec<(&'static str, &'static str)> {
    log.iter()
        .map(|(name, event)| {
            let kind = match event {
                UiEvent::PointerEnter(_) => "enter",
                UiEvent::PointerLeave => "leave",
                UiEvent::PointerMove { .. } => "move",
                UiEvent::PointerDown { .. } => "down",
                UiEvent::PointerUp { .. } => "up",
                UiEvent::Wheel { .. } => "wheel",
                UiEvent::Key(_) => "key",
                UiEvent::Paste(_) => "paste",
            };
            (*name, kind)
        })
        .collect()
}

#[test]
fn pointer_events_reach_the_widget_under_the_pointer() {
    let mut ui = probes();
    let log = ui.handle(&moved(50.0, 50.0)).unwrap();
    assert_eq!(kinds(&log), [("left", "enter"), ("left", "move")]);
    let log = ui
        .handle(&button(ButtonState::Pressed, 50.0, 50.0))
        .unwrap();
    assert_eq!(kinds(&log), [("left", "down")]);
    let log = ui
        .handle(&button(ButtonState::Released, 50.0, 50.0))
        .unwrap();
    assert_eq!(kinds(&log), [("left", "up")]);
    let log = ui.handle(&moved(150.0, 50.0)).unwrap();
    assert_eq!(
        kinds(&log),
        [("left", "leave"), ("right", "enter"), ("right", "move")]
    );
    let wheel = WindowEvent::MouseWheel {
        delta: ScrollDelta::Lines { x: 0.0, y: 1.0 },
        position: at(150.0, 50.0),
        modifiers: Modifiers::NONE,
    };
    assert_eq!(kinds(&ui.handle(&wheel).unwrap()), [("right", "wheel")]);
    let log = ui.handle(&WindowEvent::CursorLeft).unwrap();
    assert_eq!(kinds(&log), [("right", "leave")]);
}

#[test]
fn a_pressed_widget_captures_the_pointer() {
    let mut ui = probes();
    ui.handle(&moved(50.0, 50.0)).unwrap();
    ui.handle(&button(ButtonState::Pressed, 50.0, 50.0))
        .unwrap();
    let log = ui.handle(&moved(150.0, 50.0)).unwrap();
    assert_eq!(
        kinds(&log),
        [("left", "leave"), ("right", "enter"), ("left", "move")]
    );
    let log = ui
        .handle(&button(ButtonState::Released, 150.0, 50.0))
        .unwrap();
    assert_eq!(kinds(&log), [("left", "up")]);
    let log = ui.handle(&moved(151.0, 50.0)).unwrap();
    assert_eq!(kinds(&log), [("right", "move")]);
}

#[test]
fn keys_go_to_the_focused_widget_else_under_the_pointer() {
    let mut ui = probes();
    ui.handle(&moved(150.0, 50.0)).unwrap();
    let log = ui.handle(&key()).unwrap();
    assert_eq!(kinds(&log), [("right", "key")], "nothing focused yet");
    click(&mut ui, 50.0, 50.0);
    let left = ui.focus().expect("the left probe takes focus");
    ui.handle(&moved(150.0, 50.0)).unwrap();
    click(&mut ui, 150.0, 50.0);
    assert_eq!(
        ui.focus(),
        Some(left),
        "unfocusable widgets leave the focus alone"
    );
    assert_eq!(kinds(&ui.handle(&key()).unwrap()), [("left", "key")]);
}

#[test]
fn resizing_lays_out_again() {
    let mut ui = probes();
    let right = ui.root.widget().children()[1].id();
    assert_eq!(
        ui.widget_rect(right),
        Some(Rect::new(100.0, 0.0, 100.0, 100.0))
    );
    ui.handle(&WindowEvent::Resized(PhysicalSize::new(400, 50)))
        .unwrap();
    assert_eq!(
        ui.widget_rect(right),
        Some(Rect::new(200.0, 0.0, 200.0, 50.0))
    );
    assert!(ui.needs_paint());
    assert_eq!(ui.paint().unwrap().size(), PhysicalSize::new(400, 50));
    assert!(!ui.needs_paint());
}

#[test]
fn buttons_click_and_highlight() {
    let mut toolbar = Flex::row();
    let home = toolbar.push(Button::new("Home", "home"), 0.0);
    let mut ui = Ui::new(toolbar, PhysicalSize::new(200, 40), 1.0).unwrap();
    let sample = |ui: &mut Ui<&'static str>| {
        let rect = ui.widget_rect(home).unwrap();
        let frame = ui.paint().unwrap();
        let (x, y) = (rect.x as usize + 3, rect.center().y as usize);
        frame.pixels()[(y * frame.width() as usize + x) * 4]
    };
    let idle = sample(&mut ui);
    let center = ui.widget_rect(home).unwrap().center();
    ui.handle(&moved(center.x, center.y)).unwrap();
    let hot = sample(&mut ui);
    ui.handle(&button(ButtonState::Pressed, center.x, center.y))
        .unwrap();
    let pressed = sample(&mut ui);
    assert!(idle > hot && hot > pressed, "{idle} > {hot} > {pressed}");
    let released = ui.handle(&button(ButtonState::Released, center.x, center.y));
    assert_eq!(released.unwrap(), ["home"]);
    ui.handle(&button(ButtonState::Pressed, center.x, center.y))
        .unwrap();
    let outside = ui
        .handle(&button(ButtonState::Released, 199.0, 39.0))
        .unwrap();
    assert_eq!(
        outside,
        [] as [&str; 0],
        "releasing outside cancels the click"
    );
}

/// The red channel of the pixel at `(x, y)` of the next frame.
fn red_at(ui: &mut Ui<&'static str>, x: f64, y: f64) -> u8 {
    let frame = ui.paint().unwrap();
    frame.pixels()[(y as usize * frame.width() as usize + x as usize) * 4]
}

#[test]
fn tool_buttons_can_be_checked_and_disabled() {
    let square = Icon::new(24.0).fill(plt_core::Path::polygon([
        Point::new(4.0, 4.0),
        Point::new(20.0, 4.0),
        Point::new(20.0, 20.0),
        Point::new(4.0, 20.0),
    ]));
    let mut toolbar = Flex::row();
    let pan = toolbar.push(ToolButton::new(square.clone(), "pan"), 0.0);
    toolbar.push(Separator::vertical(), 0.0);
    let back = toolbar.push(ToolButton::new(square, "back"), 0.0);
    let mut ui = Ui::new(toolbar, PhysicalSize::new(200, 40), 1.0).unwrap();
    let corner = |ui: &Ui<&'static str>, id| {
        let rect = ui.widget_rect(id).unwrap();
        (rect.x + 3.0, rect.center().y)
    };
    let (x, y) = corner(&ui, pan);
    let flat = red_at(&mut ui, x, y);
    ui.widget_mut::<ToolButton<&'static str>>(pan)
        .unwrap()
        .set_checked(true);
    let checked = red_at(&mut ui, x, y);
    assert!(
        checked < flat,
        "a checked button shows a face: {checked} < {flat}"
    );
    ui.widget_mut::<ToolButton<&'static str>>(back)
        .unwrap()
        .set_enabled(false);
    let center = ui.widget_rect(back).unwrap().center();
    ui.handle(&button(ButtonState::Pressed, center.x, center.y))
        .unwrap();
    let released = ui.handle(&button(ButtonState::Released, center.x, center.y));
    assert_eq!(
        released.unwrap(),
        [] as [&str; 0],
        "disabled buttons ignore clicks"
    );
}

#[test]
fn surfaces_show_a_selection() {
    let mut ui = Ui::new(
        Surface::new(|_| None::<&'static str>),
        PhysicalSize::new(100, 100),
        1.0,
    )
    .unwrap();
    let plain = red_at(&mut ui, 50.0, 50.0);
    let root = ui.root_id();
    ui.widget_mut::<Surface<&'static str>>(root)
        .unwrap()
        .set_selection(Some(Rect::new(80.0, 80.0, -60.0, -60.0)));
    let tinted = red_at(&mut ui, 50.0, 50.0);
    assert!(
        tinted < plain,
        "the selection tints the surface: {tinted} < {plain}"
    );
    assert_eq!(red_at(&mut ui, 90.0, 90.0), plain, "outside the selection");
}

#[derive(Debug, Clone, PartialEq)]
enum Figure {
    Resized(PhysicalSize),
    Clicked(Point),
}

#[test]
fn surfaces_report_their_size_and_local_input() {
    let map = |event| match event {
        SurfaceEvent::Resized(size) => Some(Figure::Resized(size)),
        SurfaceEvent::Input(UiEvent::PointerDown { position, .. }) => {
            Some(Figure::Clicked(position))
        }
        SurfaceEvent::Input(_) => None,
    };
    let mut column = Flex::column();
    let status = column.push(Label::new("status"), 0.0);
    let surface = column.push(Surface::new(map), 1.0);
    let mut ui = Ui::new(column, PhysicalSize::new(300, 200), 1.0).unwrap();
    ui.paint().unwrap();
    let top = ui.widget_rect(surface).unwrap().y;
    let height = (200.0 - top).round() as u32;
    assert_eq!(
        ui.take_messages(),
        [Figure::Resized(PhysicalSize::new(300, height))]
    );
    let log = ui
        .handle(&button(ButtonState::Pressed, 10.0, top + 5.0))
        .unwrap();
    assert_eq!(log, [Figure::Clicked(Point::new(10.0, 5.0))]);
    assert_eq!(ui.focus(), Some(surface));

    let image = Pixmap::filled(4, 4, Rgba8::new(0, 0, 255, 255)).unwrap();
    ui.widget_mut::<Surface<Figure>>(surface)
        .unwrap()
        .set_image(image);
    ui.widget_mut::<Label>(status).unwrap().set_text("x=1 y=2");
    assert!(
        ui.widget_mut::<Label>(surface).is_none(),
        "wrong widget type"
    );
    let frame = ui.paint().unwrap();
    let at = ((top as usize + 1) * 300 + 1) * 4;
    assert_eq!(frame.pixels()[at..at + 4], [0, 0, 255, 255]);
}

#[test]
fn themes_set_the_background() {
    let theme = Theme {
        background: Rgba::new(0.0, 1.0, 0.0, 1.0),
        ..Theme::default()
    };
    let ui = Ui::new(Flex::<()>::row(), PhysicalSize::new(4, 4), 1.0).unwrap();
    let mut ui = ui.with_theme(theme);
    assert_eq!(ui.paint().unwrap().pixels()[..4], [0, 255, 0, 255]);
    ui.resize(PhysicalSize::new(2, 2), 2.0).unwrap();
    assert_eq!(
        (ui.size(), ui.scale_factor()),
        (PhysicalSize::new(2, 2), 2.0)
    );
}
