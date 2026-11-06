//! Interaction: random sequences of presses, drags, releases, double
//! clicks, wheel turns and keys reach figures holding widgets (buttons,
//! sliders, check and radio buttons, text boxes, cursors and every
//! selector, set up with random options) and pickable artists whose
//! callbacks change and remove things; navigation pans, zooms and walks
//! its history; and the canvas copies, restores and blits regions.

use pltrs::artist::{ArtistHandle, Picker};
use pltrs::axes::Axes;
use pltrs::events::{Event, EventKind, EventSource};
use pltrs::figure::Figure;
use pltrs::navigation::{DragKey, NavMode, Navigation, PointerButton};
use pltrs::transforms::{Bbox, Point};
use pltrs::widgets::{
    Button, ButtonOptions, CheckButtons, Cursor, CursorOptions, EllipseSelector, LassoSelector,
    LassoSelectorOptions, Orientation, PolygonSelector, PolygonSelectorOptions, RadioButtons,
    RangeSlider, RectangleSelector, RectangleSelectorOptions, Slider, SliderOptions, SpanSelector,
    SpanSelectorOptions, TextBox, TextBoxOptions, ValStep,
};

use crate::harness::{Gen, fuzz};

/// The keys typed: matplotlib's bindings, editing keys and odd text.
const KEYS: [&str; 20] = [
    "enter",
    "escape",
    "backspace",
    "delete",
    "left",
    "right",
    "home",
    "end",
    "shift",
    "control",
    "ctrl+z",
    "a",
    "x",
    "y",
    "1",
    "-",
    " ",
    "\u{1F600}",
    "e\u{301}",
    "\t",
];

/// A widget and how it is set up.
#[derive(Debug)]
enum WidgetSpec {
    Button,
    Slider {
        range: (f64, f64),
        init: f64,
        step: Option<f64>,
        vertical: bool,
    },
    RangeSlider {
        range: (f64, f64),
    },
    Check {
        labels: usize,
    },
    Radio {
        labels: usize,
        active: usize,
    },
    TextBox {
        initial: String,
    },
    Cursor,
    Span {
        minspan: f64,
        interactive: bool,
        vertical: bool,
    },
    Rectangle {
        minspan: (f64, f64),
        interactive: bool,
        ellipse: bool,
    },
    Lasso,
    Polygon {
        grab: f64,
    },
}

/// One input of a sequence, in display pixels.
#[derive(Debug, Clone)]
enum Action {
    Press(f64, f64, usize),
    Release(f64, f64, usize),
    DoubleClick(f64, f64),
    Motion(f64, f64),
    Scroll(f64, f64, f64),
    Key(String, f64, f64),
    Navigate(usize, f64, f64),
    Blit([f64; 4]),
}

/// A figure, its widgets and an input sequence.
#[derive(Debug)]
struct Session {
    widgets: Vec<WidgetSpec>,
    actions: Vec<Action>,
}

/// The size of the figures, in pixels.
const SIZE: (f64, f64) = (300.0, 240.0);

impl WidgetSpec {
    fn generate(g: &mut Gen) -> Self {
        match g.below(11) {
            0 => Self::Button,
            1 => Self::Slider {
                range: (g.number_in(-10.0, 0.0), g.number_in(0.0, 10.0)),
                init: g.number_in(-10.0, 10.0),
                step: g.chance(0.4).then(|| g.number_in(0.0, 2.0)),
                vertical: g.chance(0.3),
            },
            2 => Self::RangeSlider {
                range: (g.number_in(-10.0, 0.0), g.number_in(0.0, 10.0)),
            },
            3 => Self::Check { labels: g.below(6) },
            4 => Self::Radio {
                labels: 1 + g.below(5),
                active: g.below(7),
            },
            5 => Self::TextBox {
                initial: g.soup(&KEYS, 4),
            },
            6 => Self::Cursor,
            7 => Self::Span {
                minspan: g.number_in(0.0, 2.0),
                interactive: g.chance(0.5),
                vertical: g.chance(0.5),
            },
            8 => Self::Rectangle {
                minspan: (g.number_in(0.0, 1.0), g.number_in(0.0, 1.0)),
                interactive: g.chance(0.5),
                ellipse: g.chance(0.3),
            },
            9 => Self::Lasso,
            _ => Self::Polygon {
                grab: g.number_in(0.0, 20.0),
            },
        }
    }
}

impl Action {
    fn generate(g: &mut Gen) -> Self {
        let mut at = || {
            (
                g.between(-10.0, SIZE.0 + 10.0),
                g.between(-10.0, SIZE.1 + 10.0),
            )
        };
        let (x, y) = at();
        match g.below(20) {
            0..5 => Self::Press(x, y, g.below(PointerButton::ALL.len())),
            5..10 => Self::Release(x, y, g.below(PointerButton::ALL.len())),
            10 => Self::DoubleClick(x, y),
            11..15 => Self::Motion(x, y),
            15 => Self::Scroll(x, y, g.pick(&[-3.0, -1.0, 1.0, 2.0, 1e9, f64::NAN])),
            16 | 17 => Self::Key(g.pick(&KEYS).to_owned(), x, y),
            18 => Self::Navigate(g.below(9), x, y),
            _ => Self::Blit([
                g.number_in(-0.2, 1.2),
                g.number_in(-0.2, 1.2),
                g.number_in(-0.2, 1.2),
                g.number_in(-0.2, 1.2),
            ]),
        }
    }
}

/// A pointer button by number, the side buttons included.
fn button(index: usize) -> PointerButton {
    PointerButton::ALL[index % PointerButton::ALL.len()]
}

/// The widgets kept alive while the session runs (they disconnect when
/// dropped).
type Kept = Vec<Box<dyn std::any::Any>>;

/// Makes the widget of `spec` in `ax`, with callbacks that change the
/// plot in `target`, and keeps it in `kept`.
fn make_widget(spec: &WidgetSpec, ax: &Axes, target: &Axes, kept: &mut Kept) {
    let target = target.clone();
    match spec {
        WidgetSpec::Button => {
            let button = Button::new(ax, "Go", ButtonOptions::default());
            button.on_clicked(move || {
                target.cla();
            });
            kept.push(Box::new(button));
        }
        WidgetSpec::Slider { .. } | WidgetSpec::RangeSlider { .. } => {
            make_slider(spec, ax, target, kept);
        }
        WidgetSpec::Check { labels } => {
            let names = labels_of("c", *labels);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let check = CheckButtons::new(ax, &names, &vec![true; names.len()]);
            check.on_clicked(move |_| {
                target.set_visible(!target.visible());
            });
            kept.push(Box::new(check));
        }
        WidgetSpec::Radio { labels, active } => {
            let names = labels_of("r", *labels);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            let radio = RadioButtons::new(ax, &names, *active);
            radio.on_clicked(move |label| {
                let _ = target.set_xscale(if label == "r0" { "log" } else { "linear" });
            });
            kept.push(Box::new(radio));
        }
        WidgetSpec::TextBox { initial } => {
            let text = TextBox::new(ax, "t", initial, TextBoxOptions::default());
            text.on_submit(move |value| {
                target.set_title(value);
            });
            kept.push(Box::new(text));
        }
        _ => make_selector(spec, ax, target, kept),
    }
}

/// `count` labels starting with `prefix`.
fn labels_of(prefix: &str, count: usize) -> Vec<String> {
    (0..count).map(|i| format!("{prefix}{i}")).collect()
}

/// Makes a slider or range slider that sets the limits of `target`.
fn make_slider(spec: &WidgetSpec, ax: &Axes, target: Axes, kept: &mut Kept) {
    match spec {
        WidgetSpec::Slider {
            range,
            init,
            step,
            vertical,
        } => {
            let options = SliderOptions {
                valinit: *init,
                valstep: step.map(ValStep::Step),
                orientation: if *vertical {
                    Orientation::Vertical
                } else {
                    Orientation::Horizontal
                },
                ..SliderOptions::default()
            };
            if let Ok(slider) = Slider::new(ax, "s", range.0, range.1, options) {
                slider.on_changed(move |value| {
                    let _ = target.set_xlim(value, value * 2.0 + 1.0);
                });
                kept.push(Box::new(slider));
            }
        }
        WidgetSpec::RangeSlider { range } => {
            if let Ok(slider) =
                RangeSlider::new(ax, "r", range.0, range.1, None, SliderOptions::default())
            {
                slider.on_changed(move |(low, high)| {
                    let _ = target.set_ylim(low, high);
                });
                kept.push(Box::new(slider));
            }
        }
        _ => {}
    }
}

/// Makes a cursor or a selector, some of them changing `target`.
fn make_selector(spec: &WidgetSpec, ax: &Axes, target: Axes, kept: &mut Kept) {
    match spec {
        WidgetSpec::Cursor => kept.push(Box::new(Cursor::new(ax, CursorOptions::default()))),
        WidgetSpec::Span {
            minspan,
            interactive,
            vertical,
        } => {
            let options = SpanSelectorOptions {
                minspan: *minspan,
                interactive: *interactive,
                ..SpanSelectorOptions::default()
            };
            let direction = if *vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            };
            let span = SpanSelector::new(ax, direction, options);
            span.on_select(move |low, high| {
                let _ = target.set_xlim(low, high);
            });
            kept.push(Box::new(span));
        }
        WidgetSpec::Rectangle {
            minspan,
            interactive,
            ellipse,
        } => {
            let options = RectangleSelectorOptions {
                minspanx: minspan.0,
                minspany: minspan.1,
                interactive: *interactive,
                ..RectangleSelectorOptions::default()
            };
            if *ellipse {
                kept.push(Box::new(EllipseSelector::new(ax, options)));
            } else {
                kept.push(Box::new(RectangleSelector::new(ax, options)));
            }
        }
        WidgetSpec::Lasso => {
            let lasso = LassoSelector::new(ax, LassoSelectorOptions::default());
            lasso.on_select(move |points| {
                let (x, y): (Vec<f64>, Vec<f64>) = points.iter().copied().unzip();
                let _ = target.plot(x, y);
            });
            kept.push(Box::new(lasso));
        }
        WidgetSpec::Polygon { grab } => {
            let options = PolygonSelectorOptions {
                grab_range: *grab,
                ..PolygonSelectorOptions::default()
            };
            kept.push(Box::new(PolygonSelector::new(ax, options)));
        }
        _ => {}
    }
}

/// The figure: a plot with pickable artists (whose pick callback removes
/// them) and one axes per widget.
fn build(session: &Session, kept: &mut Kept) -> Figure {
    let fig = Figure::with_size(SIZE.0 / 60.0, SIZE.1 / 60.0, 60.0);
    let Ok(plot) = fig.add_axes([0.1, 0.5, 0.8, 0.45]) else {
        return fig;
    };
    if let Ok(line) = plot.plot(vec![0.0, 1.0, 2.0], vec![1.0, 3.0, 2.0]) {
        line.set_picker(Some(Picker::Radius(5.0)));
    }
    if let Ok(points) = plot.scatter(vec![0.5, 1.5], vec![2.0, 1.0]) {
        points.set_picker(Some(Picker::Default));
    }
    plot.text(1.0, 2.0, "pick me")
        .set_picker(Some(Picker::Default));
    fig.connect(EventKind::Pick, |event| {
        if let Event::Pick(pick) = event {
            pick.artist.remove();
        }
    });
    let count = session.widgets.len().max(1) as f64;
    for (i, spec) in session.widgets.iter().enumerate() {
        let left = 0.05 + 0.9 * i as f64 / count;
        if let Ok(ax) = fig.add_axes([left, 0.05, 0.85 / count, 0.35]) {
            make_widget(spec, &ax, &plot, kept);
        }
    }
    fig
}

/// Runs the session's actions.
fn run(session: &Session) {
    let mut kept = Kept::new();
    let fig = build(session, &mut kept);
    let mut source = EventSource::new();
    let mut navigation = Navigation::new();
    let canvas = fig.canvas();
    let _ = canvas.draw();
    for action in &session.actions {
        match action {
            Action::Press(x, y, b) => source.button_press(&fig, *x, *y, button(*b), false),
            Action::Release(x, y, b) => source.button_release(&fig, *x, *y, button(*b)),
            Action::DoubleClick(x, y) => source.double_click(&fig, *x, *y, PointerButton::Left),
            Action::Motion(x, y) => source.motion(&fig, *x, *y),
            Action::Scroll(x, y, step) => source.scroll(&fig, *x, *y, *step),
            Action::Key(key, x, y) => {
                source.key_press(&fig, key, *x, *y);
                source.key_release(&fig, key, *x, *y);
            }
            Action::Navigate(what, x, y) => {
                navigate(&mut navigation, &fig, *what, Point::new(*x, *y));
            }
            Action::Blit([x0, y0, x1, y1]) => {
                let bbox = Bbox::from_extents(x0 * SIZE.0, y0 * SIZE.1, x1 * SIZE.0, y1 * SIZE.1);
                if let Ok(region) = canvas.copy_from_bbox(&bbox) {
                    let _ = canvas.restore_region(&region);
                }
                let _ = canvas.blit(Some(&bbox));
                let _ = canvas.draw_idle();
            }
        }
    }
    let _ = fig.to_png();
}

/// One navigation step: a mode switch, a press, drag or release, a wheel
/// turn or a history move.
fn navigate(navigation: &mut Navigation, fig: &Figure, what: usize, at: Point) {
    match what {
        0 => drop(navigation.toggle(fig, NavMode::Pan)),
        1 => drop(navigation.toggle(fig, NavMode::Zoom)),
        2 => drop(navigation.press(fig, at, PointerButton::Left)),
        3 => drop(navigation.drag(at, DragKey::None)),
        4 => navigation.release(fig, at, DragKey::Control),
        5 => navigation.wheel(fig, at, 1.0),
        6 => navigation.home(fig),
        7 => navigation.back(fig),
        _ => drop(navigation.readout(fig, at)),
    }
}

#[test]
fn random_input_sequences() {
    fuzz(
        "interaction",
        200,
        |g| Session {
            widgets: (0..=g.below(4)).map(|_| WidgetSpec::generate(g)).collect(),
            actions: (0..g.len(60)).map(|_| Action::generate(g)).collect(),
        },
        run,
    );
}
