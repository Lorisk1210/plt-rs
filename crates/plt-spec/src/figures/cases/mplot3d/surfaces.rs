//! Surface, wireframe and triangulated-surface cases.

use plt::collections::CollectionProps;
use plt::colors::{Color, colormaps};
use plt::mplot3d::{
    FaceColormap, FaceColors, Sampling, SurfaceOptions, TrisurfOptions, WireframeOptions,
};
use plt::numeric::Grid;
use plt::tri::Triangulation;

use super::surface;
use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// `cmap=name`.
fn mapped(name: &str) -> Result<FaceColors, plt::colors::ColormapError> {
    Ok(FaceColors::Mapped(Box::new(FaceColormap::new(
        colormaps::get(name)?,
    ))))
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d/plot_surface_cmap", |fig| {
        let (x, y, z) = surface(40);
        let options = SurfaceOptions {
            colors: mapped("viridis")?,
            ..SurfaceOptions::default()
        };
        fig.subplot_3d().plot_surface(&x, &y, &z, options)?;
        Ok(())
    }),
    FigureCase::new("mplot3d/plot_surface_strides_edges", |fig| {
        let (x, y, z) = surface(40);
        let options = SurfaceOptions {
            sampling: Sampling::Strides { rows: 4, cols: 4 },
            colors: FaceColors::Single("tab:orange".into()),
            props: CollectionProps::default().edgecolor("k").linewidth(0.4),
            ..SurfaceOptions::default()
        };
        fig.subplot_3d().plot_surface(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/plot_surface_no_shade_alpha", |fig| {
        let (x, y, z) = surface(25);
        let options = SurfaceOptions {
            colors: FaceColors::Single("tab:blue".into()),
            shade: Some(false),
            props: CollectionProps::default().alpha(0.5),
            ..SurfaceOptions::default()
        };
        fig.subplot_3d().plot_surface(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/plot_surface_facecolors", |fig| {
        let (x, y, z) = surface(16);
        let coolwarm = colormaps::get("coolwarm")?;
        let checker = Grid::from_fn(16, 16, |i, j| {
            let parity = ((i + j) % 2) as f64;
            Color::from(coolwarm.sample(parity))
        });
        let options = SurfaceOptions {
            cell_colors: Some(checker),
            shade: Some(false),
            ..SurfaceOptions::default()
        };
        fig.subplot_3d().plot_surface(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/plot_wireframe", |fig| {
        let (x, y, z) = surface(40);
        let options = WireframeOptions {
            sampling: Sampling::Strides { rows: 4, cols: 6 },
            props: CollectionProps::default().color("k").linewidth(0.6),
        };
        fig.subplot_3d().plot_wireframe(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/plot_trisurf", |fig| {
        let mut rng = SpecRng::new(43);
        let x = rng.uniform(150, -3.0, 3.0);
        let y = rng.uniform(150, -3.0, 3.0);
        let z: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x.cos() * y.sin()).collect();
        let tri = Triangulation::delaunay(&x, &y)?;
        let options = TrisurfOptions {
            colors: mapped("Spectral")?,
            props: CollectionProps::default().linewidth(0.2).edgecolor("grey"),
            ..TrisurfOptions::default()
        };
        fig.subplot_3d().plot_trisurf(&tri, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
