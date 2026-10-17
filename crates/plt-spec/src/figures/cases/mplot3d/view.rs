//! View, projection, limits and decoration cases.

use plt::colors::colormaps;
use plt::mplot3d::{
    Axes3D, Axes3DError, Axes3DOptions, Axis3, FaceColormap, FaceColors, ProjType, Sampling,
    SurfaceOptions, WireframeOptions,
};

use super::{plot_helix, surface};
use crate::figures::FigureCase;

/// `ax.plot_wireframe(*_surface(20), rstride=2, cstride=2)`.
fn wireframe(ax: &Axes3D) -> Result<(), Axes3DError> {
    let (x, y, z) = surface(20);
    let options = WireframeOptions {
        sampling: Sampling::Strides { rows: 2, cols: 2 },
        ..WireframeOptions::default()
    };
    ax.plot_wireframe(&x, &y, &z, options).map(drop)
}

/// `ax.plot_surface(*_surface(25), cmap="viridis")`.
fn viridis_surface(ax: &Axes3D) -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, z) = surface(25);
    let options = SurfaceOptions {
        colors: FaceColors::Mapped(Box::new(FaceColormap::new(colormaps::get("viridis")?))),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d/empty_default", |fig| {
        fig.subplot_3d();
        Ok(())
    }),
    FigureCase::new("mplot3d/view_top", |fig| {
        let ax = fig.subplot_3d();
        plot_helix(&ax)?;
        ax.view_init(90.0, -90.0, 0.0);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/box_aspect", |fig| {
        let ax = fig.subplot_3d();
        plot_helix(&ax)?;
        ax.set_box_aspect(Some([3.0, 1.0, 0.5]), 1.0)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("mplot3d/labels_limits", |fig| {
        let ax = fig.subplot_3d();
        plot_helix(&ax)?;
        ax.set_xlabel("X axis")
            .set_ylabel("Y axis")
            .set_zlabel("Z axis");
        ax.set_title("limits");
        ax.set_xlim(-2.0, 2.0)?.set_zlim(0.0, 0.5)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/inverted_axes", |fig| {
        let ax = fig.subplot_3d();
        plot_helix(&ax)?;
        ax.invert(Axis3::Z).invert(Axis3::X);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/pane_colors_no_grid", |fig| {
        let ax = fig.subplot_3d();
        plot_helix(&ax)?;
        ax.set_pane_color(Axis3::X, (1.0, 0.0, 0.0, 0.2))
            .set_pane_color(Axis3::Y, (0.0, 1.0, 0.0, 0.2))
            .set_pane_color(Axis3::Z, (0.0, 0.0, 1.0, 0.2))
            .grid(false);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/view_init_roll", |fig| {
        let ax = fig.subplot_3d();
        wireframe(&ax)?;
        ax.view_init(20.0, 30.0, 15.0);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/view_negative_elev", |fig| {
        let ax = fig.subplot_3d();
        viridis_surface(&ax)?;
        ax.view_init(-25.0, 120.0, 0.0);
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/proj_ortho", |fig| {
        let ax = fig.subplot_3d_with(Axes3DOptions {
            proj_type: ProjType::Ortho,
            ..Axes3DOptions::default()
        })?;
        wireframe(&ax)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/focal_length", |fig| {
        let ax = fig.subplot_3d();
        wireframe(&ax)?;
        ax.set_proj_type(ProjType::Persp { focal_length: 0.2 })?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axis_off", |fig| {
        let ax = fig.subplot_3d();
        viridis_surface(&ax)?;
        ax.set_axis_off();
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
