//! 3-D scatter cases.

use plt::artist::ArtistHandle;
use plt::axes::{MappedColors, ScatterColors, ScatterOptions};
use plt::collections::{CollectionProps, EdgeColors};
use plt::colors::colormaps;
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::mplot3d::{Axes3DOptions, Axis3, FaceColors, Scatter3DOptions, SurfaceOptions};

use super::surface;
use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// Marker areas of `s=area`.
fn area(area: f64) -> ScatterOptions {
    ScatterOptions {
        sizes: Some(vec![area]),
        ..ScatterOptions::default()
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d/plot_line", |fig| {
        let ax = fig.subplot_3d();
        let [x, y, z] = super::helix();
        let helix = LineProps::default().label("helix");
        ax.plot_with(&x, &y, &z, Axis3::Z, helix)?;
        ax.plot_fmt(&[-1.0, 1.0], &[-1.0, 1.0], &[0.0, 1.0], "r--o")?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/scatter_depthshade", |fig| {
        let ax = fig.subplot_3d();
        let mut g = SpecRng::new(41);
        let (x, y, z) = (
            g.uniform(80, 0.0, 1.0),
            g.uniform(80, 0.0, 1.0),
            g.uniform(80, 0.0, 1.0),
        );
        ax.scatter_with(
            &x,
            &y,
            &z,
            Scatter3DOptions {
                scatter: area(40.0),
                ..Scatter3DOptions::default()
            },
        )?;
        let (x, y, z) = (
            g.uniform(30, 0.0, 1.0),
            g.uniform(30, 0.0, 1.0),
            g.uniform(30, 0.0, 1.0),
        );
        let scatter = ScatterOptions {
            marker: Some("^".parse()?),
            ..area(60.0)
        };
        ax.scatter_with(
            &x,
            &y,
            &z,
            Scatter3DOptions {
                scatter,
                ..Scatter3DOptions::default()
            },
        )?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/scatter_no_depthshade_cmap", |fig| {
        let ax = fig.subplot_3d();
        let mut g = SpecRng::new(42);
        let (x, y, z) = (
            g.uniform(100, 0.0, 1.0),
            g.uniform(100, 0.0, 1.0),
            g.uniform(100, 0.0, 1.0),
        );
        let scatter = ScatterOptions {
            colors: MappedColors::new(z.clone())
                .with_cmap(colormaps::get("plasma")?)
                .into(),
            edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
            ..area(30.0)
        };
        let options = Scatter3DOptions {
            depthshade: Some(false),
            scatter,
            ..Scatter3DOptions::default()
        };
        ax.scatter_with(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/computed_zorder_false", |fig| {
        let ax = fig.subplot_3d_with(Axes3DOptions {
            computed_zorder: false,
            ..Axes3DOptions::default()
        })?;
        let (x, y, z) = surface(25);
        let options = SurfaceOptions {
            colors: FaceColors::Single("0.7".into()),
            props: CollectionProps::default().zorder(1.0),
            ..SurfaceOptions::default()
        };
        ax.plot_surface(&x, &y, &z, options)?;
        let scatter = ScatterOptions {
            colors: ScatterColors::Single("r".into()),
            ..area(80.0)
        };
        let points = ax.scatter_with(
            &[0.0, 1.0, -1.0],
            &[0.0, -1.0, 1.0],
            &[-0.5],
            Scatter3DOptions {
                scatter,
                ..Scatter3DOptions::default()
            },
        )?;
        points.set_zorder(2.0);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
