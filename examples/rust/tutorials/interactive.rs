//! matplotlib's "Interactive figures" and "Event handling and picking"
//! tutorials in pltrs (Rust): callbacks on mouse and key events
//! (`mpl_connect`), picking, widgets, timers, and showing the figure in a
//! window with its navigation toolbar.
//!
//! ```text
//! cargo run -p pltrs-examples --example interactive -- --show   # opens a window
//! cargo run -p pltrs-examples --example interactive             # plays the clicks itself
//! ```
//!
//! Without `--show` the program plays the user's part with an
//! [`EventSource`], the synthetic input matplotlib's tests use, and saves
//! the result as `interactive.png`, so that it also runs where no window
//! can open.
#![expect(clippy::print_stdout, reason = "the tutorial reports what it made")]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use pltrs::artist::Picker;
use pltrs::events::{Event, EventKind, EventSource, TimerKey};
use pltrs::navigation::PointerButton;
use pltrs::prelude::*;
use pltrs::widgets::{Button, ButtonOptions, CheckButtons, Slider, SliderOptions, Widget};

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// The figure, its plot, and what the callbacks record.
struct Demo {
    fig: Figure,
    ax: Axes,
    slider_ax: Axes,
    freq: Rc<Slider>,
    picked: Rc<RefCell<Vec<usize>>>,
    ticks: Rc<RefCell<u32>>,
    timer: TimerKey,
    /// Kept alive: a widget reacts for as long as it lives.
    _widgets: (CheckButtons, Button),
}

fn main() -> Outcome {
    let demo = build()?;
    if std::env::args().any(|arg| arg == "--show") {
        // `plt.show()`: the window has matplotlib's toolbar (pan, zoom,
        // home, back, forward, save) and key bindings.
        pltrs::gui::show(std::slice::from_ref(&demo.fig))?;
    } else {
        play(&demo)?;
        demo.fig.savefig("interactive.png")?;
    }
    Ok(())
}

/// A plot with callbacks on clicks, picks and keys, three widgets and a
/// timer.
fn build() -> Result<Demo, Box<dyn std::error::Error>> {
    let fig = Figure::with_size(7.0, 5.0, 100.0);
    let ax = fig.add_axes([0.1, 0.35, 0.6, 0.55])?;
    let x = linspace(0.0, 1.0, 200);
    let wave = ax.plot(
        &x,
        x.iter().map(|x| (2.0 * std::f64::consts::TAU * x).sin()),
    )?;
    let markers = ax.plot_fmt([0.25, 0.5, 0.75], [0.0, 0.0, 0.0], "o")?;
    ax.set_ylim(-1.5, 1.5)?;
    ax.set_title("click the axes, pick a marker, press 'g'");

    // `fig.canvas.mpl_connect("button_press_event", onclick)`
    let status = fig.text(0.1, 0.25, "");
    let (clicked, plot_ax) = (status.clone(), ax.clone());
    fig.connect(EventKind::ButtonPress, move |event| {
        if let Event::ButtonPress(mouse) = event
            && mouse.location.inaxes.as_ref() == Some(&plot_ax)
            && let (Some(x), Some(y)) = (mouse.location.xdata, mouse.location.ydata)
        {
            clicked.set_text(format!("clicked at x = {x:.2}, y = {y:.2}"));
        }
    });

    // `line.set_picker(5)` and a `pick_event` callback: `event.ind`
    // holds the indices of the points under the pointer.
    markers.set_picker(Some(Picker::Radius(5.0)));
    let picked = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&picked);
    fig.connect(EventKind::Pick, move |event| {
        if let Event::Pick(pick) = event {
            seen.borrow_mut().extend(pick.ind.iter().copied());
        }
    });

    // `key_press_event`: "g" toggles the grid.
    let grid_ax = ax.clone();
    let grid_on = Rc::new(RefCell::new(false));
    fig.connect(EventKind::KeyPress, move |event| {
        if let Event::KeyPress(key) = event
            && key.key == "g"
        {
            let mut on = grid_on.borrow_mut();
            *on = !*on;
            grid_ax.grid(*on);
        }
    });

    // `Slider(ax, "Freq", 0.1, 10, valinit=2)` with `on_changed`.
    let options = SliderOptions {
        valinit: 2.0,
        ..SliderOptions::default()
    };
    let slider_ax = fig.add_axes([0.1, 0.12, 0.6, 0.04])?;
    let freq = Rc::new(Slider::new(&slider_ax, "Freq", 0.1, 10.0, options)?);
    let line = wave.clone();
    freq.on_changed(move |value| {
        line.set_ydata(
            line.xdata()
                .iter()
                .map(|x| (value * std::f64::consts::TAU * x).sin()),
        );
    });

    // `CheckButtons(ax, ["wave", "markers"], [True, True])` showing and
    // hiding the lines, and a `Button` resetting the slider.
    let checks = CheckButtons::new(
        &fig.add_axes([0.75, 0.6, 0.2, 0.2])?,
        &["wave", "markers"],
        &[true, true],
    );
    checks.on_clicked(move |label| {
        let line = match label {
            Some("wave") => &wave,
            Some("markers") => &markers,
            _ => return,
        };
        line.set_visible(!line.visible());
    });
    let reset = Button::new(
        &fig.add_axes([0.75, 0.1, 0.15, 0.06])?,
        "Reset",
        ButtonOptions::default(),
    );
    let to_reset = Rc::clone(&freq);
    reset.on_clicked(move || to_reset.reset());

    // `fig.canvas.new_timer(interval=500)`: a window fires it every 500 ms.
    let ticks = Rc::new(RefCell::new(0));
    let counter = Rc::clone(&ticks);
    let timer = fig.add_timer(Duration::from_millis(500), move || {
        *counter.borrow_mut() += 1;
        true
    });
    Ok(Demo {
        fig,
        ax,
        slider_ax,
        freq,
        picked,
        ticks,
        timer,
        _widgets: (checks, reset),
    })
}

/// The user's part, played: a click, a pick, a key, the slider set, a
/// timer tick, then a click on the slider once it is turned off.
fn play(demo: &Demo) -> Outcome {
    let Demo { fig, ax, .. } = demo;
    let mut user = EventSource::new();
    let click = ax.data_to_display((0.1, 0.8))?;
    user.button_press(fig, click.x, click.y, PointerButton::Left, false);
    user.button_release(fig, click.x, click.y, PointerButton::Left);
    let marker = ax.data_to_display((0.5, 0.0))?;
    user.button_press(fig, marker.x, marker.y, PointerButton::Left, false);
    user.button_release(fig, marker.x, marker.y, PointerButton::Left);
    user.key_press(fig, "g", click.x, click.y);
    demo.freq.set_val(5.0);
    fig.fire_timer(demo.timer);
    assert_eq!(*demo.picked.borrow(), [1], "the middle marker");
    assert_eq!(*demo.ticks.borrow(), 1);

    // `widget.set_active(False)`: a widget turned off ignores the user.
    Widget::set_active(&*demo.freq, false);
    let track = demo.slider_ax.data_to_display((9.0, 0.5))?;
    user.button_press(fig, track.x, track.y, PointerButton::Left, false);
    user.button_release(fig, track.x, track.y, PointerButton::Left);
    assert_eq!(
        demo.freq.val(),
        5.0,
        "the turned-off slider ignores the click"
    );
    println!(
        "frequency {}, picked {:?}",
        demo.freq.val(),
        demo.picked.borrow()
    );
    Ok(())
}
