//! Sliders and text boxes: `range_slider`, `slider_demo`,
//! `slider_snap_demo` and `textbox`.

use std::f64::consts::PI;

use plt::axes::{HistPlotOptions, ImshowOptions};
use plt::gridspec::SubplotOverrides;
use plt::numeric::Grid;
use plt::patches::PatchProps;
use plt::prelude::*;
use plt::stats::{BinEstimator, Bins, HistOptions};
use plt::text::HorizontalAlignment;
use plt::widgets::{
    Button, ButtonOptions, Orientation, RangeSlider, Slider, SliderOptions, TextBox,
    TextBoxOptions, ValStep,
};

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_axisartist/range_slider", range_slider).sized(10.0, 5.0, 100.0),
    FigureCase::new("gallery_axisartist/slider_demo", slider_demo),
    FigureCase::new("gallery_axisartist/slider_snap_demo", slider_snap_demo),
    FigureCase::new("gallery_axisartist/textbox", textbox),
];

/// Only the bottom margin of the subplots changed.
fn bottom(margin: f64) -> SubplotOverrides {
    SubplotOverrides {
        bottom: Some(margin),
        ..SubplotOverrides::default()
    }
}

/// `amplitude * np.sin(2 * np.pi * frequency * t)`.
fn wave(t: &[f64], amplitude: f64, frequency: f64) -> Vec<f64> {
    map(t, |t| amplitude * (2.0 * PI * frequency * t).sin())
}

/// The slider examples' reset button.
fn reset_button(fig: &Figure) -> Button {
    let options = ButtonOptions {
        hovercolor: "0.975".into(),
        ..ButtonOptions::default()
    };
    Button::new(&fig.add_axes([0.8, 0.025, 0.1, 0.04]), "Reset", options)
}

fn range_slider(fig: &Figure) -> CaseResult {
    let n = 128;
    let pixels = SpecRng::new(19_680_801).normal(n * n, 0.0, 1.0);
    let img = Grid::from_vec(n, n, pixels.clone())?;
    let axs = fig.subplots(1, 2)?;
    fig.subplots_adjust(&bottom(0.25))?;
    axs[(0, 0)].imshow(img, ImshowOptions::default())?;
    let auto = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Estimator(BinEstimator::Auto)),
            ..HistOptions::default()
        },
        ..HistPlotOptions::default()
    };
    axs[(0, 1)].hist(&[&pixels], &auto)?;
    axs[(0, 1)].set_title("Histogram of pixel intensities");
    let (low, high) = pixels
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let slider_ax = fig.add_axes([0.20, 0.1, 0.60, 0.03]);
    let slider = RangeSlider::new(
        &slider_ax,
        "Threshold",
        low,
        high,
        None,
        &SliderOptions::default(),
    );
    let (lower, upper) = slider.val();
    for x in [lower, upper] {
        axs[(0, 1)].axvline(x, LineProps::default().color("k"));
    }
    Ok(())
}

fn slider_demo(fig: &Figure) -> CaseResult {
    let t = linspace(0.0, 1.0, 1000);
    let ax = fig.subplot();
    ax.plot_with(
        t.clone(),
        wave(&t, 5.0, 3.0),
        LineProps::default().linewidth(2.0),
    )?;
    ax.set_xlabel("Time [s]");
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.25),
        ..bottom(0.25)
    })?;
    let frequency = SliderOptions {
        valinit: 3.0,
        ..SliderOptions::default()
    };
    let _freq = Slider::new(
        &fig.add_axes([0.25, 0.1, 0.65, 0.03]),
        "Frequency [Hz]",
        0.1,
        30.0,
        &frequency,
    );
    let amplitude = SliderOptions {
        valinit: 5.0,
        orientation: Orientation::Vertical,
        ..SliderOptions::default()
    };
    let _amp = Slider::new(
        &fig.add_axes([0.1, 0.25, 0.0225, 0.63]),
        "Amplitude",
        0.0,
        10.0,
        &amplitude,
    );
    reset_button(fig);
    Ok(())
}

fn slider_snap_demo(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 1.0, 0.001);
    let ax = fig.subplot();
    fig.subplots_adjust(&bottom(0.25))?;
    ax.plot_with(
        t.clone(),
        wave(&t, 5.0, 3.0),
        LineProps::default().linewidth(2.0),
    )?;
    let ax_freq = fig.add_axes([0.25, 0.1, 0.65, 0.03]);
    let ax_amp = fig.add_axes([0.25, 0.15, 0.65, 0.03]);
    let mut allowed_amplitudes = linspace(0.1, 5.0, 100);
    allowed_amplitudes.extend([6.0, 7.0, 8.0, 9.0]);
    let amplitude = SliderOptions {
        valinit: 5.0,
        valstep: Some(ValStep::Values(allowed_amplitudes)),
        poly: PatchProps::default().color("green"),
        ..SliderOptions::default()
    };
    let _amp = Slider::new(&ax_amp, "Amp", 0.1, 9.0, &amplitude);
    let frequency = SliderOptions {
        valinit: 2.0 * PI,
        valstep: Some(ValStep::Step(PI)),
        initcolor: "none".into(),
        ..SliderOptions::default()
    };
    let _freq = Slider::new(&ax_freq, "Freq", 0.0, 10.0 * PI, &frequency);
    reset_button(fig);
    Ok(())
}

/// What `textbox`'s `submit` evaluates, as far as this example needs:
/// powers `t ** n` of the time `t`.
fn evaluate(expression: &str, t: &[f64]) -> Option<Vec<f64>> {
    let exponent: i32 = expression
        .strip_prefix('t')?
        .trim_start()
        .strip_prefix("**")?
        .trim()
        .parse()
        .ok()?;
    Some(map(t, |t| t.powi(exponent)))
}

fn textbox(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    fig.subplots_adjust(&bottom(0.2))?;
    let t = arange(-2.0, 2.0, 0.001);
    let line = ax.plot_with(
        t.clone(),
        vec![0.0; t.len()],
        LineProps::default().linewidth(2.0),
    )?;
    let options = TextBoxOptions {
        textalignment: HorizontalAlignment::Center,
        ..TextBoxOptions::default()
    };
    let text_box = TextBox::new(
        &fig.add_axes([0.1, 0.05, 0.8, 0.075]),
        "Evaluate",
        "",
        options,
    );
    let plot = ax.clone();
    text_box.on_submit(move |expression| {
        if let Some(y) = evaluate(expression, &t) {
            line.set_ydata(y);
            plot.relim(false);
            plot.autoscale_view(None);
        }
    });
    text_box.set_val("t ** 2");
    Ok(())
}
