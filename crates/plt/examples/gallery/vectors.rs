//! Vector fields: streamlines, arrows with a key, and wind barbs.

use plt::colorbar::ColorbarOptions;
use plt::colors::{ColorizerOptions, colormaps};
use plt::numeric::{Grid, linspace, meshgrid};
use plt::prelude::*;
use plt::quiver::{
    BarbsOptions, KeyCoordinates, LabelPosition, Pivot, QuiverKeyOptions, QuiverOptions,
    VectorField,
};
use plt::rc::Aspect;
use plt::streamplot::{StreamColorMap, StreamColors, StreamField, StreamWidths, StreamplotOptions};

use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("streamlines", "Streamlines coloured by speed", streamlines),
    Entry::figure(
        "streamlines_width",
        "Streamlines whose width and colour follow the speed",
        streamlines_width,
    ),
    Entry::figure("quiver", "A vector field of arrows", quiver),
    Entry::figure(
        "quiver_key",
        "Arrows coloured by length, with a key",
        quiver_key,
    ),
    Entry::figure("barbs", "Wind barbs", barbs),
];

/// Quiver key options for a key placed in figure coordinates with its
/// label on its right, as in the free right end of a title row, where
/// layouts leave room for it (keys outside the axes are not laid out).
pub(crate) fn title_row_key() -> QuiverKeyOptions {
    QuiverKeyOptions {
        coordinates: KeyCoordinates::Figure,
        labelpos: LabelPosition::East,
        ..QuiverKeyOptions::default()
    }
}

/// The velocity `u = -1 - x² + y`, `v = 1 + x - y²` on an `n × n` grid
/// over `[-3, 3]²`, with its speed.
fn saddle_flow(n: usize) -> (Vec<f64>, [Grid<f64>; 3]) {
    let x = linspace(-3.0, 3.0, n);
    let u = Grid::from_fn(n, n, |i, j| -1.0 - x[j] * x[j] + x[i]);
    let v = Grid::from_fn(n, n, |i, j| 1.0 + x[j] - x[i] * x[i]);
    let speed = Grid::from_fn(n, n, |i, j| u[(i, j)].hypot(v[(i, j)]));
    (x, [u, v, speed])
}

/// Streamlines of the saddle flow, coloured by speed.
fn streamlines() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.subplot();
    let (x, [u, v, speed]) = saddle_flow(60);
    let field = StreamField::new(x.clone(), x, u, v);
    let options = StreamplotOptions {
        density: (1.2, 1.2),
        color: Some(StreamColors::Mapped(Box::new(StreamColorMap {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("plasma")?),
            values: speed,
        }))),
        ..StreamplotOptions::default()
    };
    ax.streamplot(&field, options)?;
    ax.set_title("streamplot");
    Ok(fig)
}

/// The same flow with the line width proportional to the speed, so that
/// fast currents stand out, and a colorbar for the colours.
fn streamlines_width() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot();
    let (x, [u, v, speed]) = saddle_flow(60);
    let top = speed.as_slice().iter().copied().fold(0.0, f64::max);
    let options = StreamplotOptions {
        density: (0.8, 0.8),
        linewidth: Some(StreamWidths::Varying(speed.map(|s| 0.4 + 4.0 * s / top))),
        color: Some(StreamColors::Mapped(Box::new(StreamColorMap {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
            values: speed,
        }))),
        ..StreamplotOptions::default()
    };
    let streams = ax.streamplot(&StreamField::new(x.clone(), x, u, v), options)?;
    let colorbar = ColorbarOptions {
        label: "speed".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&streams.lines, colorbar)?;
    ax.set_title("Width and colour by speed");
    Ok(fig)
}

/// A swirl with a sink, as arrows on a regular mesh.
fn quiver() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(-2.0, 2.0, 15);
    let (xx, yy) = meshgrid(&x, &x);
    let u = Grid::from_fn(15, 15, |i, j| -yy[(i, j)] - 0.3 * xx[(i, j)]);
    let v = Grid::from_fn(15, 15, |i, j| xx[(i, j)] - 0.3 * yy[(i, j)]);
    ax.quiver(
        VectorField::on_mesh(&xx, &yy, &u, &v),
        QuiverOptions::default(),
    )?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("quiver");
    Ok(fig)
}

/// `u = cos x`, `v = sin y` as arrows coloured by their length. The key
/// in the free corner of the title row gives the speed of an arrow of its
/// length.
fn quiver_key() -> Built {
    let fig = Figure::with_size(5.5, 5.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 2.0 * std::f64::consts::PI, 13);
    let (xx, yy) = meshgrid(&x, &x);
    let u = xx.map(|x| x.cos());
    let v = yy.map(|y| y.sin());
    let speed: Vec<f64> = u
        .as_slice()
        .iter()
        .zip(v.as_slice())
        .map(|(u, v)| u.hypot(*v))
        .collect();
    let colors = MappedColors::new(speed).cmap(colormaps::get("viridis")?);
    let options = QuiverOptions {
        pivot: Pivot::Middle,
        ..QuiverOptions::default()
    };
    let arrows = ax.quiver(
        VectorField::on_mesh(&xx, &yy, &u, &v).with_colors(colors),
        options,
    )?;
    ax.quiverkey(&arrows, (0.8, 0.955), 1.0, "1 m/s", title_row_key());
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("cos x, sin y");
    Ok(fig)
}

/// Wind barbs on a coarse grid: each barb points into the wind, and its
/// flags, full and half lines add up the speed in knots.
fn barbs() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(-5.0, 5.0, 9);
    let (xx, yy) = meshgrid(&x, &x);
    let u = Grid::from_fn(9, 9, |i, j| 20.0 + 6.0 * yy[(i, j)] + 2.0 * xx[(i, j)]);
    let v = Grid::from_fn(9, 9, |i, j| {
        8.0 * (xx[(i, j)] / 3.0).sin() - 2.0 * yy[(i, j)]
    });
    let options = BarbsOptions {
        length: 6.5,
        ..BarbsOptions::default()
    };
    ax.barbs(VectorField::on_mesh(&xx, &yy, &u, &v), options)?;
    ax.set_xlim(-6.0, 6.0)?.set_ylim(-6.0, 6.0)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("Wind barbs (knots)");
    Ok(fig)
}
