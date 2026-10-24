//! Diagrams, widgets and animation: a Sankey flow diagram, interactive
//! controls drawn in a figure, and an animation saved with its frames.

use std::f64::consts::PI;
use std::time::Duration;

use plt::animation::{AnimationOptions, Frames, FuncAnimation};
use plt::figure::{Mosaic, MosaicOptions};
use plt::fonts::FontWeight;
use plt::prelude::*;
use plt::sankey::{FlowOrientation, PathLengths, Sankey, SankeyOptions, Subdiagram};
use plt::ticker::LabelFormat;
use plt::widgets::{Button, ButtonOptions, CheckButtons, RadioButtons, Slider, SliderOptions};

use super::lines::map;
use super::{Animated, Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("sankey", "A Sankey diagram of an energy budget", sankey),
    Entry::figure(
        "widgets",
        "Widgets: sliders, buttons, radio and check buttons",
        widgets,
    ),
    Entry::animation(
        "animation",
        "An animation saved as an animated PNG",
        animation,
    ),
];

/// Where the energy of a household goes, in percent: inputs enter from
/// the left, losses leave through the top and bottom, and the width of
/// every arrow is proportional to its flow.
fn sankey() -> Built {
    let fig = Figure::with_size(6.4, 4.8, 100.0);
    let ax = fig.subplot();
    ax.set_axis_off();
    let options = SankeyOptions {
        scale: 0.01,
        offset: 0.25,
        format: LabelFormat::Template("%.0f".into()),
        unit: Some("%".into()),
        ..SankeyOptions::default()
    };
    let (side, top, bottom) = (
        FlowOrientation::Side,
        FlowOrientation::Top,
        FlowOrientation::Bottom,
    );
    let budget = Subdiagram {
        flows: vec![60.0, 25.0, 15.0, -30.0, -18.0, -12.0, -40.0],
        labels: [
            "grid",
            "gas",
            "solar",
            "heating",
            "hot water",
            "losses",
            "appliances",
        ]
        .map(|label| Some(label.to_owned()))
        .to_vec(),
        orientations: vec![side, top, bottom, top, top, bottom, side],
        pathlengths: PathLengths::PerFlow(vec![0.3, 0.3, 0.3, 0.3, 0.6, 0.3, 0.3]),
        patchlabel: "home".into(),
        props: PatchProps::default()
            .facecolor("C0")
            .alpha(0.35)
            .edgecolor("C0"),
        ..Subdiagram::default()
    };
    let mut diagram = Sankey::new(&ax, options)?;
    diagram.add(budget)?;
    let drawn = diagram.finish()?;
    if let Some(home) = drawn.first() {
        home.text.set_weight(FontWeight::BOLD).set_fontsize(13.0);
    }
    ax.set_title("Household energy (%)");
    Ok(fig)
}

/// The controls of an interactive figure, as they are drawn: in a window
/// they move the sliders and change the curve; here they show their
/// initial state. Each widget lives in an axes of the mosaic.
fn widgets() -> Built {
    let fig = Figure::with_size(7.5, 5.0, 100.0);
    let layout: Mosaic = "PR;PC;F.;AB".parse()?;
    let options = MosaicOptions {
        width_ratios: Some(vec![4.0, 1.2]),
        height_ratios: Some(vec![2.0, 2.0, 0.35, 0.35]),
        ..MosaicOptions::default()
    };
    let axes = fig.subplot_mosaic(&layout, options)?;
    let cell = |name: &str| axes.get(name).cloned().ok_or("a cell of the mosaic");
    let plot = cell("P")?;
    let t = linspace(0.0, 1.0, 500);
    plot.plot(&t, map(&t, |t| 2.0 * (2.0 * PI * 3.0 * t).sin()))?;
    plot.set_title("A wave and its controls");
    let frequency = SliderOptions {
        valinit: 3.0,
        ..SliderOptions::default()
    };
    let _frequency = Slider::new(&cell("F")?, "frequency", 0.1, 10.0, frequency)?;
    let amplitude = SliderOptions {
        valinit: 2.0,
        ..SliderOptions::default()
    };
    let _amplitude = Slider::new(&cell("A")?, "amplitude", 0.0, 5.0, amplitude)?;
    let _reset = Button::new(&cell("B")?, "Reset", ButtonOptions::default());
    let radio = cell("R")?;
    radio.set_title("colour");
    let _colour = RadioButtons::new(&radio, &["blue", "orange", "green"], 0);
    let checks = cell("C")?;
    checks.set_title("show");
    let _shown = CheckButtons::new(
        &checks,
        &["grid", "legend", "markers"],
        &[true, false, false],
    );
    Ok(fig)
}

/// A wave packet centred on `centre`, sampled at `x`.
fn packet(x: &[f64], centre: f64) -> Vec<f64> {
    map(x, |x| {
        (-(x - centre).powi(2) / 1.5).exp() * (6.0 * (x - centre)).cos()
    })
}

/// A wave packet travelling to the right: the frame function moves the
/// packet and returns nothing, so every frame is drawn in full. The
/// animation is saved with all its frames; the gallery writes it as an
/// animated PNG (`save` picks APNG, GIF, WebP or a video from the name).
fn animation() -> Animated {
    let fig = Figure::with_size(5.0, 3.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 400);
    let wave = ax.plot(&x, packet(&x, 2.0))?;
    ax.set_ylim(-1.2, 1.2)?;
    ax.set_title("A travelling wave packet");
    let options = AnimationOptions {
        interval: Duration::from_millis(60),
        ..AnimationOptions::default()
    };
    let frames = 40;
    Ok(FuncAnimation::new(
        &fig,
        Frames::Count(frames),
        options,
        move |i| {
            let centre = 2.0 + 6.0 * i as f64 / (frames - 1) as f64;
            wave.set_ydata(packet(&x, centre));
        },
    ))
}
