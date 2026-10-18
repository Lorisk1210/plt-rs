//! 3-D contour cases.

use plt::collections::CollectionProps;
use plt::colors::colormaps;
use plt::contour::{ContourOptions, Levels};
use plt::mplot3d::{Axis3, Contour3DOptions, FaceColors, SurfaceOptions};

use super::surface;
use crate::figures::FigureCase;

/// `levels=n, cmap=name`.
pub(super) fn levels(n: i64, cmap: &str) -> Result<ContourOptions, plt::colors::ColormapError> {
    Ok(ContourOptions {
        levels: Levels::Count(n),
        cmap: Some(colormaps::get(cmap)?),
        ..ContourOptions::default()
    })
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d/contour_projections", |fig| {
        let ax = fig.subplot_3d();
        let (x, y, z) = surface(40);
        let options = SurfaceOptions {
            colors: FaceColors::Single("0.8".into()),
            props: CollectionProps::default().alpha(0.4),
            ..SurfaceOptions::default()
        };
        ax.plot_surface(&x, &y, &z, options)?;
        for (zdir, offset) in [(Axis3::Z, -0.6), (Axis3::X, -4.0), (Axis3::Y, 4.0)] {
            let contour = ContourOptions {
                cmap: Some(colormaps::get("coolwarm")?),
                ..ContourOptions::default()
            };
            let options = Contour3DOptions {
                zdir,
                offset: Some(offset),
                contour,
                ..Contour3DOptions::default()
            };
            ax.contour(&x, &y, &z, options)?;
        }
        ax.set_xlim(-4.0, 3.0)?
            .set_ylim(-3.0, 4.0)?
            .set_zlim(-0.6, 0.8)?;
        Ok(())
    })
    .sized(5.0, 5.0, 100.0),
    FigureCase::new("mplot3d/contour3d_lines", |fig| {
        let (x, y, z) = surface(40);
        let options = Contour3DOptions {
            contour: levels(12, "viridis")?,
            ..Contour3DOptions::default()
        };
        fig.subplot_3d().contour(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/contourf3d", |fig| {
        let (x, y, z) = surface(40);
        let options = Contour3DOptions {
            contour: levels(8, "magma")?,
            ..Contour3DOptions::default()
        };
        fig.subplot_3d().contourf(&x, &y, &z, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
