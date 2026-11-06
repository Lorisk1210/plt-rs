//! matplotlib's colormap tutorials ("Choosing Colormaps", "Creating
//! Colormaps", "Colormap normalization") in pltrs (Rust): the bundled
//! colormaps, colormaps of your own, and the norms mapping data onto
//! them.
//!
//! ```text
//! cargo run -p pltrs-examples --example colormaps_and_norms   # writes colormaps_*.png here
//! ```
#![expect(clippy::print_stdout, reason = "the tutorial reports what it made")]

use pltrs::colorbar::ColorbarAxes;
use pltrs::colors::{
    BoundaryNorm, CenteredNorm, ColorizerOptions, Colormap, Extend, LogNorm, Norm, PowerNorm,
    SymLogNorm, TwoSlopeNorm,
};
use pltrs::gridspec::SubplotOverrides;
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// One panel of the norms figure: its title, its data, the norm and the
/// colormap's name.
type Panel<'a> = (&'a str, &'a Grid<f64>, Box<dyn Norm>, &'a str);

fn main() -> Outcome {
    choosing()?;
    creating()?;
    normalizing()?;
    Ok(())
}

/// "Choosing Colormaps": a strip of each family's colormaps
/// (`mpl.colormaps["viridis"]`, `plt.imshow(gradient, cmap=name)`).
fn choosing() -> Outcome {
    let families = [
        "viridis", "cividis", "Greys", "YlGnBu", "RdBu", "coolwarm", "twilight", "tab10", "terrain",
    ];
    let gradient = Grid::from_fn(1, 256, |_, col| col as f64 / 255.0);
    let fig = Figure::with_size(6.4, 3.6, 100.0);
    let axs = fig.subplots(families.len(), 1)?;
    fig.subplots_adjust(SubplotOverrides {
        left: Some(0.25),
        bottom: Some(0.02),
        right: Some(0.98),
        top: Some(0.98),
        ..SubplotOverrides::default()
    })?;
    for (ax, name) in axs.as_slice().iter().zip(families) {
        let options = ImshowOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get(name)?),
            aspect: Some(pltrs::rc::Aspect::Auto),
            ..ImshowOptions::default()
        };
        ax.imshow(gradient.clone(), options)?;
        ax.set_axis_off();
        let label = fig.text(0.23, 0.0, name);
        let (_, bottom, _, height) = ax.position().bounds();
        label.set_y(bottom + height / 2.0);
        label
            .set_halign(pltrs::text::HAlign::Right)
            .set_valign(pltrs::text::VAlign::Center);
    }
    println!("{} colormaps are registered", colormaps::names().len());
    fig.savefig("colormaps_choosing.png")?;
    Ok(())
}

/// "Creating Colormaps": `ListedColormap`,
/// `LinearSegmentedColormap.from_list`, `cmap.reversed()`,
/// `cmap.resampled(n)`, `cmap.with_extremes(over=...)` and
/// `mpl.colormaps.register(cmap)`.
fn creating() -> Outcome {
    let listed = Colormap::listed("traffic", vec!["green", "gold", "red"])?;
    let ramp = Colormap::from_list("sea", vec!["navy", "turquoise", "ivory"], 256, 1.0)?;
    colormaps::register(ramp.clone())?;
    let made = [
        listed,
        colormaps::get("sea")?,
        ramp.reversed(),
        colormaps::get("viridis")?.resampled(8)?,
        colormaps::get("viridis")?
            .with_over("red")?
            .with_under("black")?,
    ];
    let data = Grid::from_fn(30, 30, |row, col| {
        let (x, y) = (col as f64 / 29.0 - 0.5, row as f64 / 29.0 - 0.5);
        1.2 * (-(x * x + y * y) * 6.0).exp() - 0.1
    });
    let fig = Figure::with_size(10.0, 2.4, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, made.len())?;
    for (ax, cmap) in axs.as_slice().iter().zip(made) {
        let name = cmap.name().to_owned();
        let options = ImshowOptions {
            colorizer: ColorizerOptions::default()
                .cmap(cmap)
                .limits(Some(0.0), Some(1.0)),
            ..ImshowOptions::default()
        };
        let image = ax.imshow(data.clone(), options)?;
        let colorbar = ColorbarOptions {
            axes: ColorbarAxes::Parent(ax.clone()),
            extend: Some(Extend::Both),
            ..ColorbarOptions::default()
        };
        fig.colorbar(&image, colorbar)?;
        ax.set_title(name);
        ax.set_axis_off();
    }
    fig.savefig("colormaps_creating.png")?;
    Ok(())
}

/// "Colormap normalization": `colors.LogNorm`, `CenteredNorm`,
/// `TwoSlopeNorm`, `SymLogNorm`, `PowerNorm` and `BoundaryNorm`, each
/// given to `pcolormesh`/`imshow` as `norm=`.
fn normalizing() -> Outcome {
    let n = 100;
    let x = linspace(-3.0, 3.0, n);
    let (xx, yy) = meshgrid(&x, &x);
    // A narrow peak on a broad bump, which a linear norm hides.
    let peaked = Grid::from_fn(n, n, |r, c| {
        let (x, y) = (xx[(r, c)], yy[(r, c)]);
        (-(x * x) - y * y).exp() + 50.0 * (-((x * 10.0).powi(2)) - (y * 10.0).powi(2)).exp()
    });
    // Positive and negative bumps of different sizes.
    let diverging = Grid::from_fn(n, n, |r, c| {
        let (x, y) = (xx[(r, c)], yy[(r, c)]);
        5.0 * (-(x - 1.0).powi(2) - y * y).exp() - (-(x + 1.0).powi(2) - y * y).exp()
    });
    let norms: [Panel<'_>; 6] = [
        (
            "LogNorm",
            &peaked,
            Box::new(LogNorm::new(1e-3, 51.0)),
            "viridis",
        ),
        (
            "PowerNorm(γ = 0.3)",
            &peaked,
            Box::new(PowerNorm::new(0.3)),
            "viridis",
        ),
        (
            "CenteredNorm",
            &diverging,
            Box::new(CenteredNorm::new(0.0, None)),
            "RdBu_r",
        ),
        (
            "TwoSlopeNorm",
            &diverging,
            Box::new(TwoSlopeNorm::new(0.0, Some(-1.0), Some(5.0))?),
            "RdBu_r",
        ),
        (
            "SymLogNorm",
            &diverging,
            Box::new(SymLogNorm::new(0.1, 1.0, 10.0)?),
            "RdBu_r",
        ),
        (
            "BoundaryNorm",
            &diverging,
            Box::new(BoundaryNorm::new(
                [-1.0, -0.5, 0.0, 1.0, 2.5, 5.0],
                256,
                Extend::Neither,
            )?),
            "RdBu_r",
        ),
    ];
    let fig = Figure::with_size(10.0, 6.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 3)?;
    for (ax, (title, data, norm, cmap)) in axs.as_slice().iter().zip(norms) {
        let options = ImshowOptions {
            colorizer: ColorizerOptions::default()
                .cmap(colormaps::get(cmap)?)
                .norm(norm),
            ..ImshowOptions::default()
        };
        let image = ax.imshow(data.clone(), options)?;
        let colorbar = ColorbarOptions {
            axes: ColorbarAxes::Parent(ax.clone()),
            ..ColorbarOptions::default()
        };
        fig.colorbar(&image, colorbar)?;
        ax.set_title(title);
        ax.set_axis_off();
    }
    fig.savefig("colormaps_norms.png")?;
    Ok(())
}
