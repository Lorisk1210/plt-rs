//! Widgets in a window: sliders drive a sine wave, a button resets them,
//! and dragging over the wave selects a span that the lower axes shows
//! enlarged (matplotlib's slider and span selector demos).
//!
//! ```text
//! cargo run -p pltrs --example widgets
//! ```

use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;

use pltrs::figure::Figure;
use pltrs::lines::Line2D;
use pltrs::numeric::linspace;
use pltrs::patches::PatchProps;
use pltrs::widgets::{
    Button, ButtonOptions, Orientation, Slider, SliderOptions, SpanSelector, SpanSelectorOptions,
};

/// The wave's settings and the lines showing it.
struct Wave {
    t: Vec<f64>,
    frequency: Cell<f64>,
    amplitude: Cell<f64>,
    lines: [Line2D; 2],
}

impl Wave {
    /// Redraws both lines with the current settings.
    fn update(&self) {
        let (f, a) = (self.frequency.get(), self.amplitude.get());
        let y: Vec<f64> = self
            .t
            .iter()
            .map(|t| a * (std::f64::consts::TAU * f * t).sin())
            .collect();
        for line in &self.lines {
            line.set_ydata(y.clone());
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let fig = Figure::with_size(8.0, 6.0, 100.0);
    let top = fig.add_axes([0.1, 0.58, 0.85, 0.36])?;
    let bottom = fig.add_axes([0.1, 0.27, 0.85, 0.22])?;
    let t = linspace(0.0, 1.0, 1000);
    let lines = [
        top.plot(t.clone(), vec![0.0; t.len()])?,
        bottom.plot(t.clone(), vec![0.0; t.len()])?,
    ];
    top.set_title("Drag over the wave to enlarge a span");
    top.set_ylim(-11.0, 11.0)?;
    bottom.set_ylim(-11.0, 11.0)?.set_xlim(0.0, 1.0)?;
    let wave = Rc::new(Wave {
        t,
        frequency: Cell::new(3.0),
        amplitude: Cell::new(5.0),
        lines,
    });
    wave.update();

    let frequency = Slider::new(
        &fig.add_axes([0.15, 0.12, 0.6, 0.03])?,
        "Frequency [Hz]",
        0.1,
        30.0,
        SliderOptions {
            valinit: wave.frequency.get(),
            ..SliderOptions::default()
        },
    )?;
    let amplitude = Slider::new(
        &fig.add_axes([0.15, 0.06, 0.6, 0.03])?,
        "Amplitude",
        0.0,
        10.0,
        SliderOptions {
            valinit: wave.amplitude.get(),
            ..SliderOptions::default()
        },
    )?;
    {
        let wave = Rc::clone(&wave);
        frequency.on_changed(move |value| {
            wave.frequency.set(value);
            wave.update();
        });
    }
    {
        let wave = Rc::clone(&wave);
        amplitude.on_changed(move |value| {
            wave.amplitude.set(value);
            wave.update();
        });
    }
    let sliders = Rc::new([frequency, amplitude]);
    let reset = Button::new(
        &fig.add_axes([0.82, 0.05, 0.12, 0.08])?,
        "Reset",
        ButtonOptions::default(),
    );
    {
        let sliders = Rc::clone(&sliders);
        reset.on_clicked(move || {
            for slider in sliders.iter() {
                slider.reset();
            }
        });
    }

    let span = SpanSelector::new(
        &top,
        Orientation::Horizontal,
        SpanSelectorOptions {
            interactive: true,
            drag_from_anywhere: true,
            props: PatchProps::default().facecolor("tab:blue").alpha(0.3),
            ..SpanSelectorOptions::default()
        },
    );
    span.on_select(move |xmin, xmax| {
        if xmax > xmin {
            bottom
                .set_xlim(xmin, xmax)
                .expect("a selection spans finite data coordinates");
        }
    });

    pltrs::gui::show(&[fig])?;
    Ok(())
}
