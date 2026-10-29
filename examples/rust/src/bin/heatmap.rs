//! Annotated heatmap: how strongly six daily weather measurements move
//! together, each cell labelled with its correlation.

use pltrs::axes::{TickLabelProps, TickLevel};
use pltrs::colors::ColorizerOptions;
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;
use pltrs::text::{HAlign, RotationMode, VAlign};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let names = [
        "temperature",
        "humidity",
        "wind",
        "pressure",
        "rain",
        "sunshine",
    ];
    let correlation = Grid::from_rows(vec![
        vec![1.00, -0.42, -0.18, 0.21, -0.25, 0.68],
        vec![-0.42, 1.00, 0.05, -0.35, 0.57, -0.71],
        vec![-0.18, 0.05, 1.00, -0.48, 0.31, -0.12],
        vec![0.21, -0.35, -0.48, 1.00, -0.62, 0.44],
        vec![-0.25, 0.57, 0.31, -0.62, 1.00, -0.53],
        vec![0.68, -0.71, -0.12, 0.44, -0.53, 1.00],
    ])?;

    let fig = Figure::with_size(6.4, 5.2, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let options = ImshowOptions {
        colorizer: ColorizerOptions::default()
            .cmap(colormaps::get("RdBu_r")?)
            .limits(Some(-1.0), Some(1.0)),
        ..ImshowOptions::default()
    };
    let image = ax.imshow(correlation.clone(), options)?;
    let colorbar = ColorbarOptions {
        label: "correlation".into(),
        shrink: 0.8,
        ..ColorbarOptions::default()
    };
    fig.colorbar(&image, colorbar)?;
    let ticks = arange(0.0, names.len() as f64, 1.0);
    ax.set_xticks(&ticks).set_xticklabels(names)?;
    ax.set_yticks(&ticks).set_yticklabels(names)?;
    let slanted = TickLabelProps {
        rotation: Some(40.0),
        halign: Some(HAlign::Right),
        rotation_mode: Some(RotationMode::Anchor),
        ..TickLabelProps::default()
    };
    ax.xaxis().set_ticklabel_props(TickLevel::Major, slanted);
    ax.grid(false);
    ax.spines().set_visible(false);
    for row in 0..names.len() {
        for col in 0..names.len() {
            let value = correlation[(row, col)];
            let label = ax.text(col as f64, row as f64, format!("{value:.2}"));
            label
                .set_halign(HAlign::Center)
                .set_valign(VAlign::Center)
                .set_fontsize(9.0)
                .set_color(if value.abs() > 0.5 { "white" } else { "black" });
        }
    }
    ax.set_title("Correlation between weather measurements");

    fig.savefig("heatmap.png")?;
    Ok(())
}
