//! Contour and filled contour plot: a relief map of two hills and a
//! valley, with labelled contour lines over coloured height bands.

use pltrs::contour::{ClabelFormat, ClabelOptions, ContourOptions, Levels, QuadCoords, QuadField};
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::ticker::LabelFormat;

/// The height in metres at `(x, y)` km.
fn height(x: f64, y: f64) -> f64 {
    let bump =
        |cx: f64, cy: f64, spread: f64| (-((x - cx).powi(2) + (y - cy).powi(2)) / spread).exp();
    900.0 * bump(3.0, 3.5, 3.0) + 650.0 * bump(7.2, 2.2, 2.0) - 250.0 * bump(5.5, 5.0, 1.5)
        + 120.0 * (x / 1.7).sin() * (y / 1.3).cos()
        + 300.0
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (xx, yy) = meshgrid(&linspace(0.0, 10.0, 200), &linspace(0.0, 6.0, 120));
    let z = Grid::from_fn(120, 200, |row, col| height(xx[(row, col)], yy[(row, col)]));
    let relief = QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy });

    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let filled = ContourOptions {
        levels: Levels::Count(14),
        ..ContourOptions::default()
    }
    .cmap(colormaps::get("terrain")?);
    let bands = ax.contourf(&relief, filled)?;
    let lines = ContourOptions {
        levels: Levels::Values(bands.levels().into_iter().step_by(2).collect()),
        colors: Some(vec!["0.2".into()]),
        linewidths: Some(vec![0.7]),
        ..ContourOptions::default()
    };
    let labels = ClabelOptions {
        fmt: ClabelFormat::Label(LabelFormat::from("%d m")),
        fontsize: Some(8.0.into()),
        ..ClabelOptions::default()
    };
    ax.contour(&relief, lines)?.clabel(labels)?;
    let colorbar = ColorbarOptions {
        label: "height (m)".into(),
        shrink: 0.8,
        ..ColorbarOptions::default()
    };
    fig.colorbar(&bands, colorbar)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("Relief map");
    ax.set_xlabel("east (km)").set_ylabel("north (km)");

    fig.savefig("contours.png")?;
    Ok(())
}
