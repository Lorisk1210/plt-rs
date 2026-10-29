//! Twin axes: a climate chart, monthly rainfall as bars on one y axis and
//! the mean temperature as a line on another, sharing the months.

use pltrs::axes::TickParams;
use pltrs::layout::LayoutEngine;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;
use pltrs::ticker::AxisName;

/// Colours the y axis label and tick labels of `ax`.
fn colour_y_axis(ax: &Axes, label: &str, color: &str) {
    ax.set_ylabel(label);
    ax.yaxis().set_label_color(color);
    ax.tick_params(TickParams {
        axis: Some(AxisName::Y),
        labelcolor: Some(color.into()),
        ..TickParams::default()
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let rain = [
        67.0, 64.0, 75.0, 91.0, 118.0, 132.0, 128.0, 131.0, 98.0, 84.0, 81.0, 76.0,
    ]; // mm
    let temperature = [
        0.4, 1.6, 5.4, 9.1, 13.5, 16.9, 18.9, 18.3, 14.6, 10.0, 4.9, 1.6,
    ]; // °C

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let options = BarOptions {
        color: vec!["C0".into()],
        props: PatchProps::default().alpha(0.75),
        label: Some("rainfall".into()),
        ..BarOptions::default()
    };
    let bars = ax.bar(months, rain, options)?;
    colour_y_axis(&ax, "rainfall (mm)", "C0");
    ax.set_ylim(0.0, 160.0)?;

    let right = ax.twinx()?;
    let warm = LineProps::default()
        .color("C1")
        .marker(MarkerSymbol::Circle)
        .linewidth(2.5)
        .label("temperature");
    let line = right.plot_with(months, temperature, warm)?;
    colour_y_axis(&right, "temperature (°C)", "C1");
    right.set_ylim(-4.0, 24.0)?;
    right.grid(false);

    ax.set_title("Climate of a Central European city");
    let legend = LegendOptions::default()
        .handle(&bars)
        .handle(&line)
        .loc(AnchorLoc::UpperLeft);
    right.legend(legend)?;

    fig.savefig("twin_axes.png")?;
    Ok(())
}
