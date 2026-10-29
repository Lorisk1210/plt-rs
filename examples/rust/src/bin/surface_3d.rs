//! 3-D surface: the "peaks" function, three bumps and two wells, coloured
//! by height with a colorbar.

use pltrs::collections::CollectionProps;
use pltrs::colors::ColorizerOptions;
use pltrs::layout::LayoutEngine;
use pltrs::mplot3d::{FaceColors, Sampling, SurfaceOptions};
use pltrs::prelude::*;

/// The peaks function at `(x, y)`.
fn peaks(x: f64, y: f64) -> f64 {
    3.0 * (1.0 - x).powi(2) * (-x.powi(2) - (y + 1.0).powi(2)).exp()
        - 10.0 * (x / 5.0 - x.powi(3) - y.powi(5)) * (-x.powi(2) - y.powi(2)).exp()
        - (-(x + 1.0).powi(2) - y.powi(2)).exp() / 3.0
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x = linspace(-3.0, 3.0, 80);
    let (xx, yy) = meshgrid(&x, &x);
    let z = Grid::from_fn(80, 80, |row, col| peaks(xx[(row, col)], yy[(row, col)]));

    let fig = Figure::with_size(6.4, 5.2, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot_3d();
    let options = SurfaceOptions {
        sampling: Sampling::Strides { rows: 1, cols: 1 },
        colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get("viridis")?)),
        props: CollectionProps {
            antialiased: Some(vec![false]),
            ..CollectionProps::default().linewidth(0.0)
        },
        ..SurfaceOptions::default()
    };
    let surface = ax.plot_surface(&xx, &yy, &z, options)?;
    let colorbar = ColorbarOptions {
        shrink: 0.6,
        label: "height".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&surface, colorbar)?;
    ax.view_init(30.0, -55.0, 0.0);
    ax.set_xlabel("x").set_ylabel("y").set_zlabel("z");
    ax.set_title("The peaks function");

    fig.savefig("surface_3d.png")?;
    Ok(())
}
