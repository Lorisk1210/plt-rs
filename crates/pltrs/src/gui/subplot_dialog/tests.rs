//! Headless tests of the subplot tool: the values it shows, what typing,
//! the keys and the buttons ask for, and the figure's layout after a
//! figure window carried that out, in matplotlib's look and under
//! constrained layout.

use std::time::Duration;

use pltrs_platform::{
    ButtonState, Event, EventLoop, HeadlessHandle, Key, KeyEvent, Modifiers, WindowEvent,
};
use pltrs_ui::TextInput;

use super::*;
use crate::layout::PlaceHolderLayout;

/// A subplot tool open on a headless display, its changes carried out on
/// its figure as a figure window carries them out.
struct Shown {
    event_loop: EventLoop,
    handle: HeadlessHandle,
    figure: Figure,
    dialog: SubplotDialog,
    saved: FigureLayout,
    /// Whether the tool asked to close.
    closing: bool,
}

impl Shown {
    /// The tool of `figure`, as a figure window opens it.
    fn open(figure: Figure) -> Self {
        let (mut event_loop, handle) = EventLoop::headless();
        let layout = SubplotLayout::of(&figure);
        let offer = offers_constrained(&figure);
        let dialog = SubplotDialog::open(&mut event_loop.context(), &figure, layout, offer)
            .expect("the tool opens");
        let saved = FigureLayout::of(&figure);
        let mut shown = Self {
            event_loop,
            handle,
            figure,
            dialog,
            saved,
            closing: false,
        };
        shown.run();
        shown
    }

    /// Handles the input injected, carrying out the changes asked for.
    fn run(&mut self) {
        let Self {
            event_loop,
            figure,
            dialog,
            saved,
            closing,
            ..
        } = self;
        event_loop
            .pump_events(Some(Duration::ZERO), |cx, event| {
                let Event::Window { event, .. } = event else {
                    return;
                };
                for request in dialog.handle(cx, &event).expect("the tool handles input") {
                    match request {
                        Request::Change(change) => {
                            let problem = saved.apply(figure, change).err();
                            let layout = SubplotLayout::of(figure);
                            let problem = problem.as_deref().unwrap_or_default();
                            dialog.show(cx, layout, problem).expect("the tool shows it");
                        }
                        Request::Close => *closing = true,
                    }
                }
            })
            .expect("the headless loop runs");
    }

    /// Presses and releases `key` with `modifiers`, typing `text`.
    fn press(&mut self, key: Key, text: Option<&str>, modifiers: Modifiers) {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let event = KeyEvent {
                key,
                state,
                repeat: false,
                text: text
                    .filter(|_| state == ButtonState::Pressed)
                    .map(str::to_owned),
                modifiers,
                scancode: 0,
            };
            self.handle
                .inject(self.dialog.id(), WindowEvent::Key(event))
                .expect("the tool is open");
        }
        self.run();
    }

    /// Types `text` into the focused widget.
    fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.press(Key::Character(c), Some(&c.to_string()), Modifiers::NONE);
        }
    }

    /// Gives the focus to the field (or the slider) of `param`.
    fn focus(&mut self, param: Param, field: bool) {
        let control = self.dialog.control(param).expect("the tool shows it");
        let id = if field { control.field } else { control.slider };
        self.dialog.window_mut().ui_mut().set_focus(Some(id));
    }

    /// The text of the field of `param`.
    fn field(&mut self, param: Param) -> String {
        let control = self.dialog.control(param).expect("the tool shows it");
        self.dialog
            .window_mut()
            .ui_mut()
            .widget_mut::<TextInput<SubplotMessage>>(control.field)
            .map(|field| field.text().to_owned())
            .expect("the field exists")
    }

    /// The text of the note.
    fn note(&mut self) -> String {
        let note = self.dialog.note;
        self.dialog
            .window_mut()
            .ui_mut()
            .widget_mut::<Label>(note)
            .map(|label| label.text().to_owned())
            .expect("the note exists")
    }

    /// Sends what a widget of the tool would send.
    fn send(&mut self, message: SubplotMessage) {
        let mut cx = self.event_loop.context();
        let request = self
            .dialog
            .message(&mut cx, message)
            .expect("the tool acts");
        if let Some(request) = request {
            match request {
                Request::Change(change) => {
                    let problem = self.saved.apply(&self.figure, change).err();
                    let layout = SubplotLayout::of(&self.figure);
                    self.dialog
                        .show(&mut cx, layout, problem.as_deref().unwrap_or_default())
                        .expect("the tool shows it");
                }
                Request::Close => self.closing = true,
            }
        }
    }
}

const LEFT: Param = Param::Subplot(SubplotParam::Left);
const TOP: Param = Param::Subplot(SubplotParam::Top);
const W_PAD: Param = Param::Constrained(LayoutParam::WPad);

/// A figure with a 2×2 grid of titled axes.
fn grid_figure() -> Figure {
    let figure = Figure::with_size(6.4, 4.8, 100.0);
    let grid = figure.subplots(2, 2).expect("a 2×2 grid has cells");
    for ax in grid.as_slice() {
        ax.set_title("title");
        ax.set_xlabel("x");
    }
    figure
}

#[test]
fn matplotlib_look_shows_the_qt_tools_values() {
    let mut shown = Shown::open(grid_figure());
    assert!(shown.dialog.switch.is_none(), "no engine, no switch");
    let params: Vec<&str> = shown
        .dialog
        .controls
        .iter()
        .map(|c| c.param.name())
        .collect();
    assert_eq!(
        params,
        ["top", "bottom", "left", "right", "hspace", "wspace"]
    );
    assert_eq!(shown.field(TOP), "0.880");
    assert_eq!(shown.field(LEFT), "0.125");
    let title = shown
        .handle
        .window(shown.dialog.id())
        .expect("the tool is open")
        .title;
    assert_eq!(title, "Subplot configuration tool");
}

#[test]
fn fields_apply_on_enter_or_when_they_lose_the_focus() {
    let mut shown = Shown::open(grid_figure());
    shown.focus(LEFT, true);
    shown.press(Key::Character('a'), None, Modifiers::CONTROL);
    shown.type_text("0.");
    assert_eq!(
        shown.figure.subplot_params().left,
        0.125,
        "not while typing"
    );
    shown.type_text("3");
    shown.press(Key::Enter, None, Modifiers::NONE);
    assert_eq!(shown.figure.subplot_params().left, 0.3);
    assert_eq!(shown.field(LEFT), "0.300");
    assert!(!shown.closing, "Enter applies the field, not Close");

    // Text that is no number gives way to the value.
    shown.press(Key::Character('a'), None, Modifiers::CONTROL);
    shown.type_text("wide");
    shown.press(Key::Enter, None, Modifiers::NONE);
    assert_eq!(shown.field(LEFT), "0.300");
    assert_eq!(shown.figure.subplot_params().left, 0.3);

    // Past the right border: a thousandth short of it, as in Qt's tool,
    // applied as the focus moves on.
    shown.press(Key::Character('a'), None, Modifiers::CONTROL);
    shown.type_text("0.95");
    shown.press(Key::Tab, None, Modifiers::NONE);
    assert_eq!(shown.figure.subplot_params().left, 0.899);
    assert_eq!(shown.field(LEFT), "0.899");
}

#[test]
fn a_field_applied_as_a_slider_takes_the_focus_keeps_its_value() {
    let mut shown = Shown::open(grid_figure());
    shown.focus(LEFT, true);
    shown.press(Key::Character('a'), None, Modifiers::CONTROL);
    shown.type_text("0.3");
    // The next key reaches the top border's slider: the field applies,
    // then the slider moves, both in one event.
    shown.focus(TOP, false);
    shown.press(Key::Right, None, Modifiers::NONE);
    let params = shown.figure.subplot_params();
    assert_eq!((params.left, params.top), (0.3, 0.885));
    assert_eq!(shown.field(LEFT), "0.300");
}

#[test]
fn sliders_step_as_the_qt_tools_spin_boxes() {
    let mut shown = Shown::open(grid_figure());
    shown.focus(TOP, false);
    shown.press(Key::Right, None, Modifiers::NONE);
    assert_eq!(shown.figure.subplot_params().top, 0.885);
    assert_eq!(shown.field(TOP), "0.885");
    shown.press(Key::PageDown, None, Modifiers::NONE);
    assert_eq!(shown.figure.subplot_params().top, 0.85);
    shown.press(Key::Home, None, Modifiers::NONE);
    let params = shown.figure.subplot_params();
    assert_eq!(params.top, 0.111, "a thousandth above the bottom");
    assert!(params.top > params.bottom);
}

#[test]
fn tight_layout_reset_and_export() {
    let mut shown = Shown::open(grid_figure());
    let initial = shown.figure.subplot_params();
    shown.send(SubplotMessage::TightLayout);
    let tight = shown.figure.subplot_params();
    assert!(
        tight.left < initial.left && tight.top > initial.top,
        "{tight:?}"
    );
    assert_eq!(shown.field(LEFT), format!("{:.3}", tight.left));
    assert_eq!(shown.note(), "", "nothing gave way");

    shown.send(SubplotMessage::Export);
    let copied = shown
        .handle
        .clipboard_text()
        .expect("the values were copied");
    assert_eq!(copied, param::export_text(&SubplotLayout::Params(tight)));
    assert!(copied.starts_with("top=0.9") && copied.contains(",\nwspace="));
    assert_eq!(shown.note(), EXPORTED);

    shown.send(SubplotMessage::Reset);
    assert_eq!(shown.figure.subplot_params(), initial);
    assert_eq!(shown.field(LEFT), "0.125");

    shown.send(SubplotMessage::Close);
    assert!(shown.closing);
}

#[test]
fn constrained_layout_offers_its_spacing_and_gives_way_on_request() {
    let figure = grid_figure();
    figure
        .set_layout_engine(LayoutEngine::constrained())
        .expect("no colorbars");
    let mut shown = Shown::open(figure);
    let switch = shown
        .dialog
        .switch
        .expect("constrained layout has a switch");
    let chosen = |shown: &mut Shown| {
        shown
            .dialog
            .window_mut()
            .ui_mut()
            .widget_mut::<Choice<SubplotMessage>>(switch.choice)
            .map(|choice| choice.chosen())
    };
    assert_eq!(chosen(&mut shown), Some(0));
    assert_eq!(shown.field(W_PAD), "0.042");
    let spacing = |figure: &Figure| match figure.layout_engine() {
        Some(LayoutEngine::Constrained(layout)) => Some(ConstrainedSpacing::of(&layout)),
        _ => None,
    };

    // The values adjust constrained layout, which stays on.
    shown.focus(W_PAD, false);
    shown.press(Key::Right, None, Modifiers::NONE);
    let pads = spacing(&shown.figure).expect("constrained layout stays on");
    assert_eq!(pads.w_pad, 0.045);
    let before = shown.figure.axes()[0].position();
    shown.press(Key::PageUp, None, Modifiers::NONE);
    shown.figure.layout();
    assert_ne!(shown.figure.axes()[0].position(), before, "the axes moved");

    // The subplot parameters: constrained layout gives way to a tight
    // layout, which the sliders then adjust.
    shown.send(SubplotMessage::Mode(PARAMS_PAGE));
    assert_eq!(
        shown.figure.layout_engine(),
        Some(LayoutEngine::PlaceHolder(PlaceHolderLayout {
            adjust_compatible: true,
            colorbar_gridspec: true,
        }))
    );
    assert_eq!(chosen(&mut shown), Some(PARAMS_PAGE));
    assert_eq!(shown.note(), CONSTRAINED_OFF);
    // The tight layout of the subplots where their parameters place them,
    // as if constrained layout had never been on (matplotlib's own
    // sequence, from where constrained layout put them, leaves wider
    // margins).
    let plain = grid_figure();
    plain
        .tight_layout(crate::layout::TightLayout::default())
        .expect("the decorations fit");
    let tight = shown.figure.subplot_params();
    assert_eq!(tight, plain.subplot_params());
    shown.focus(LEFT, false);
    shown.press(Key::Right, None, Modifiers::NONE);
    // To the next multiple of the step, as the sliders' keys go.
    assert_eq!(shown.figure.subplot_params().left, 0.075);

    // Back to constrained layout, with the spacing set before.
    shown.send(SubplotMessage::Mode(0));
    assert_eq!(spacing(&shown.figure).map(|s| s.w_pad), Some(0.05));
    assert_eq!(chosen(&mut shown), Some(0));

    // Reset puts back the layout the tool opened with.
    shown.send(SubplotMessage::Mode(PARAMS_PAGE));
    shown.send(SubplotMessage::Reset);
    assert_eq!(
        shown.figure.layout_engine(),
        Some(LayoutEngine::constrained())
    );
    assert_eq!(shown.field(W_PAD), "0.042");
}

#[test]
fn colorbars_of_constrained_layout_keep_their_way_of_placing() {
    let figure = Figure::with_size(6.4, 4.8, 100.0);
    figure
        .set_layout_engine(LayoutEngine::constrained())
        .expect("no colorbars yet");
    let ax = figure.subplot();
    let values = crate::numeric::Grid::from_fn(4, 4, |r, c| if r > c { 1.0 } else { 0.0 });
    let image = ax
        .imshow(values, crate::axes::ImshowOptions::default())
        .expect("an image");
    figure
        .colorbar(&image, crate::colorbar::ColorbarOptions::default())
        .expect("a colorbar");
    let mut shown = Shown::open(figure);
    shown.send(SubplotMessage::TightLayout);
    assert!(matches!(
        shown.figure.layout_engine(),
        Some(LayoutEngine::PlaceHolder(PlaceHolderLayout {
            adjust_compatible: true,
            colorbar_gridspec: false,
        }))
    ));
    shown.send(SubplotMessage::Reset);
    assert_eq!(
        shown.figure.layout_engine(),
        Some(LayoutEngine::constrained())
    );
}

#[test]
fn figures_without_axes_take_the_parameters_too() {
    let mut shown = Shown::open(Figure::with_size(4.0, 3.0, 100.0));
    shown.focus(TOP, true);
    shown.press(Key::Character('a'), None, Modifiers::CONTROL);
    shown.type_text("0.5");
    shown.press(Key::Enter, None, Modifiers::NONE);
    assert_eq!(shown.figure.subplot_params().top, 0.5);
    shown.send(SubplotMessage::TightLayout);
    assert_eq!(shown.figure.subplot_params().top, 0.5, "nothing to lay out");
}
