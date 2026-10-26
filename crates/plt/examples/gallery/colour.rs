//! Colour: colormap families, normalisations, colorbar extensions and
//! the ink of the dark look.

use plt::axes::{Mesh, PcolorOptions};
use plt::colorbar::ExtendFrac;
use plt::colors::{BoundaryNorm, ColorizerOptions, Extend, LogNorm, SymLogNorm, TwoSlopeNorm};
use plt::figure::SubfigureOptions;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::quiver::{QuiverOptions, VectorField};
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::style;
use plt::text::{HAlign, VAlign};

use super::vectors::title_row_key;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "colormaps",
        "Colormaps, family by family",
        colormaps_reference,
    ),
    Entry::figure("norms", "Norms: log, symlog, two-slope and boundary", norms),
    Entry::figure(
        "colorbar_extensions",
        "Colorbars with extensions, continuous and discrete",
        colorbar_extensions,
    ),
    Entry::figure(
        "ink_dark",
        "plt.ink: the marks matplotlib draws black, in the dark look",
        ink_dark,
    ),
];

/// A few colormaps of each family as gradient strips, named on the left.
fn colormaps_reference() -> Built {
    let families: [(&str, &[&str]); 4] = [
        (
            "Perceptually uniform",
            &["viridis", "plasma", "inferno", "magma", "cividis"],
        ),
        ("Sequential", &["Blues", "Greens", "YlOrBr", "PuRd", "GnBu"]),
        (
            "Diverging",
            &["RdBu", "coolwarm", "PiYG", "BrBG", "Spectral"],
        ),
        (
            "Cyclic and qualitative",
            &["twilight", "hsv", "tab10", "Set2", "Pastel1"],
        ),
    ];
    let fig = Figure::with_size(8.0, 4.4, 100.0);
    let panels = fig.subfigures(2, 2, SubfigureOptions::default())?;
    let gradient = Grid::from_fn(1, 256, |_, j| j as f64 / 255.0);
    for (panel, (family, names)) in panels.as_slice().iter().zip(families) {
        panel.suptitle(family);
        for (ax, name) in panel.subplots(names.len(), 1)?.as_slice().iter().zip(names) {
            let options = ImshowOptions {
                aspect: Some(Aspect::Auto),
                colorizer: ColorizerOptions::default().cmap(colormaps::get(name)?),
                ..ImshowOptions::default()
            };
            ax.imshow(gradient.clone(), options)?;
            ax.set_axis_off();
            let label = ax.text(-0.02, 0.5, *name);
            label.set_transform(ax.trans_axes());
            label
                .set_halign(HAlign::Right)
                .set_valign(VAlign::Center)
                .set_fontsize(9.0);
        }
    }
    Ok(fig)
}

/// Four fields, each with the norm that suits it: a norm maps data to
/// the `[0, 1]` range the colormap spans.
fn norms() -> Built {
    let fig = Figure::with_size(8.0, 6.0, 100.0);
    let axs = fig.subplots(2, 2)?;
    let x = linspace(-3.0, 3.0, 120);
    let (xx, yy) = meshgrid(&x, &linspace(-2.0, 2.0, 80));
    let field =
        |f: &dyn Fn(f64, f64) -> f64| Grid::from_fn(80, 120, |i, j| f(xx[(i, j)], yy[(i, j)]));
    let gauss = |x: f64, y: f64, s: f64| (-(x * x + y * y) / s).exp();
    let panels: [(&str, &str, Grid<f64>, ColorizerOptions); 4] = [
        (
            "LogNorm: values over five decades",
            "magma",
            field(&|x, y| 1e-3 + 100.0 * gauss(x, y, 0.3) + gauss(x - 1.5, y - 0.8, 0.5)),
            ColorizerOptions::default().norm(LogNorm::new(1e-3, 100.0)),
        ),
        (
            "SymLogNorm: spikes of both signs",
            "RdBu_r",
            field(&|x, y| 5.0 * gauss(x + 1.0, y, 0.1) - gauss(x - 1.0, y, 0.6) + 0.02 * x),
            ColorizerOptions::default().norm(
                SymLogNorm::new(0.1, 1.0, 10.0)?
                    .with_vmin(-5.0)
                    .with_vmax(5.0),
            ),
        ),
        (
            "TwoSlopeNorm: sea level in the middle",
            "terrain",
            field(&|x, y| 3000.0 * gauss(x - 0.5, y, 2.0) - 1000.0 + 400.0 * (2.0 * x).sin()),
            ColorizerOptions::default().norm(TwoSlopeNorm::new(0.0, Some(-1000.0), Some(3000.0))?),
        ),
        (
            "BoundaryNorm: five classes",
            "viridis",
            field(&|x, y| x * x + 2.0 * y * y),
            ColorizerOptions::default().norm(BoundaryNorm::new(
                [0.0, 1.0, 2.0, 4.0, 8.0, 12.0],
                256,
                Extend::Neither,
            )?),
        ),
    ];
    for (ax, (title, cmap, z, colorizer)) in axs.as_slice().iter().zip(panels) {
        let options = PcolorOptions {
            colorizer: colorizer.cmap(colormaps::get(cmap)?),
            ..PcolorOptions::default()
        };
        let mesh = ax.pcolormesh(
            Mesh::from_coords(x.clone(), linspace(-2.0, 2.0, 80), z),
            options,
        )?;
        fig.colorbar(&mesh, ColorbarOptions::beside(ax))?;
        ax.set_title(title);
    }
    Ok(fig)
}

/// Data beyond the colour limits drawn in the colormap's "under" and
/// "over" colours, which the colorbar shows as pointed or square ends.
fn colorbar_extensions() -> Built {
    let fig = Figure::with_size(8.0, 3.8, 100.0);
    let axs = fig.subplots(1, 2)?;
    let x = linspace(-3.0, 3.0, 200);
    let z = Grid::from_fn(150, 200, |i, j| x[j].sin() * 1.6 + (i as f64 / 30.0).cos());
    let cmap = colormaps::get("RdBu_r")?
        .with_under("midnightblue")?
        .with_over("darkred")?;
    let continuous = ImshowOptions {
        colorizer: ColorizerOptions::default()
            .cmap(cmap.clone())
            .limits(Some(-1.5), Some(1.5)),
        ..ImshowOptions::default()
    };
    let image = axs[(0, 0)].imshow(z.clone(), continuous)?;
    let below = |ax: &Axes| ColorbarOptions {
        location: Some(SpineSide::Bottom),
        extend: Some(Extend::Both),
        ..ColorbarOptions::beside(ax)
    };
    fig.colorbar(
        &image,
        ColorbarOptions {
            label: "extend = both".into(),
            ..below(&axs[(0, 0)])
        },
    )?;
    let classes = BoundaryNorm::new([-1.5, -0.5, 0.0, 0.5, 1.5], cmap.n(), Extend::Both)?;
    let discrete = ImshowOptions {
        colorizer: ColorizerOptions::default().cmap(cmap).norm(classes),
        ..ImshowOptions::default()
    };
    let image = axs[(0, 1)].imshow(z, discrete)?;
    let options = ColorbarOptions {
        label: "discrete, rectangular ends".into(),
        extendrect: true,
        extendfrac: ExtendFrac::Auto,
        drawedges: true,
        orientation: Some(Orientation::Horizontal),
        ..below(&axs[(0, 1)])
    };
    fig.colorbar(&image, options)?;
    for ax in axs.as_slice() {
        ax.set_axis_off();
    }
    Ok(fig)
}

/// The dark look's ink: error bars of bar charts, quiver arrows and their
/// key, which matplotlib draws in black, follow `plt.ink` and stay
/// visible on the charcoal background.
fn ink_dark() -> Built {
    style::context("plt-modern-dark", || {
        let fig = Figure::with_size(8.0, 3.8, 100.0);
        let axs = fig.subplots(1, 2)?;
        let bars = BarOptions {
            errors: ErrorbarOptions {
                yerr: Some(vec![0.6, 0.9, 0.5, 1.1].into()),
                capsize: Some(4.0),
                ..ErrorbarOptions::default()
            },
            ..BarOptions::default()
        };
        axs[(0, 0)].bar(
            ["north", "south", "east", "west"],
            [4.0, 6.5, 3.2, 5.1],
            bars,
        )?;
        axs[(0, 0)].set_title("Bars with error bars");
        let g = linspace(-2.0, 2.0, 11);
        let (xx, yy) = meshgrid(&g, &g);
        let u = yy.map(|y| -y);
        let arrows = axs[(0, 1)].quiver(
            VectorField::on_mesh(&xx, &yy, &u, &xx),
            QuiverOptions::default(),
        )?;
        axs[(0, 1)].quiverkey(&arrows, (0.88, 0.94), 1.0, "1 m/s", title_row_key());
        axs[(0, 1)].set_aspect(Aspect::Equal)?;
        axs[(0, 1)].set_title("quiver");
        Ok(fig)
    })
}
