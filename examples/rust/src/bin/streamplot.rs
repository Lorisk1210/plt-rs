//! Streamplot: air flowing past a cylinder (ideal flow), the streamlines
//! coloured by speed.

use pltrs::colors::ColorizerOptions;
use pltrs::layout::LayoutEngine;
use pltrs::patches::Circle;
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::streamplot::{
    StreamColorMap, StreamColors, StreamField, StreamWidths, StreamplotOptions,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let radius: f64 = 1.0;
    let x = linspace(-4.0, 4.0, 160);
    let y = linspace(-2.5, 2.5, 100);
    let (rows, cols) = (y.len(), x.len());
    // The velocity at each grid point; no flow inside the cylinder.
    let velocity = |row: usize, col: usize| {
        let (x, y) = (x[col], y[row]);
        let r2 = x * x + y * y;
        if r2 < radius.powi(2) {
            (0.0, 0.0)
        } else {
            let u = 1.0 - radius.powi(2) * (x * x - y * y) / (r2 * r2);
            let v = -2.0 * radius.powi(2) * x * y / (r2 * r2);
            (u, v)
        }
    };
    let u = Grid::from_fn(rows, cols, |row, col| velocity(row, col).0);
    let v = Grid::from_fn(rows, cols, |row, col| velocity(row, col).1);
    let speed = Grid::from_fn(rows, cols, |row, col| u[(row, col)].hypot(v[(row, col)]));

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let options = StreamplotOptions {
        density: (1.6, 1.6),
        linewidth: Some(StreamWidths::Uniform(1.0)),
        arrowsize: 0.8,
        color: Some(StreamColors::Mapped(Box::new(StreamColorMap {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("plasma")?),
            values: speed,
        }))),
        ..StreamplotOptions::default()
    };
    let stream = ax.streamplot(&StreamField::new(x, y, u, v), options)?;
    ax.add_patch_with(
        Circle::new((0.0, 0.0), radius),
        PatchProps::default().color("0.85").zorder(3.0),
    );
    let colorbar = ColorbarOptions {
        label: "speed (relative to the free stream)".into(),
        shrink: 0.8,
        ..ColorbarOptions::default()
    };
    fig.colorbar(&stream.lines, colorbar)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_xlim(-4.0, 4.0)?.set_ylim(-2.5, 2.5)?;
    ax.set_title("Flow past a cylinder");

    fig.savefig("streamplot.png")?;
    Ok(())
}
