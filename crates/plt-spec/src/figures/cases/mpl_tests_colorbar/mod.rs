//! `figures/cases_mpl_tests_colorbar.py`: matplotlib's non-image tests
//! from `test_colorbar.py` that only draw, in the test suite's default
//! style: a colorbar of a colormap alone, the placement keywords of a
//! horizontal colorbar, a colorbar of a bare mappable whose colormap
//! changes, a mesh with one alpha per cell, a style with grids, five
//! colorbars of one image and a colorbar asked of another figure.

use std::error::Error;

use plt::axes::{Axes, ImshowOptions, Mesh, PcolorOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions, ParentAnchor};
use plt::colors::{Normalize, ScalarMappable, colormaps};
use plt::image::AxesImage;
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt_core::Anchor;

use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic("mpl_tests_colorbar/colorbarbase", |fig| {
            let options = ColorbarOptions {
                axes: ColorbarAxes::Cax(fig.gca()),
                ..ColorbarOptions::default()
            };
            fig.colorbar(&ScalarMappable::new(colormaps::get("bone")?), options)?;
            Ok(())
        }),
        classic("mpl_tests_colorbar/colorbar_axes_kw", |fig| {
            let image = image(&fig.gca(), vec![vec![1.0, 2.0], vec![3.0, 4.0]])?;
            let options = ColorbarOptions {
                orientation: Some(Orientation::Horizontal),
                fraction: 0.2,
                pad: Some(0.2),
                shrink: 0.5,
                aspect: 10.0,
                anchor: Some(Anchor::Fraction(0.0, 0.0)),
                panchor: ParentAnchor::At(Anchor::Fraction(0.0, 1.0)),
                ..ColorbarOptions::default()
            };
            fig.colorbar(&image, options)?;
            Ok(())
        }),
        classic("mpl_tests_colorbar/mappable_no_alpha", |fig| {
            let ax = fig.subplot();
            let mut mappable = ScalarMappable::with_norm(
                colormaps::get("viridis")?,
                Box::new(Normalize::default()),
            );
            let colorbar = fig.colorbar(&mappable, ColorbarOptions::beside(&ax))?;
            mappable.set_cmap(colormaps::get("plasma")?);
            colorbar.update_normal(&mappable)?;
            Ok(())
        }),
        classic("mpl_tests_colorbar/mappable_2d_alpha", |fig| {
            let ax = fig.subplot();
            let x = Grid::from_fn(2, 2, |r, c| (r * 2 + c + 1) as f64 / 4.0);
            let alphas = x.as_slice().to_vec();
            let mesh = ax.pcolormesh(Mesh::new(x), PcolorOptions::default())?;
            mesh.set_alphas(alphas);
            fig.colorbar(&mesh, ColorbarOptions::beside(&ax))?;
            Ok(())
        }),
        classic(
            "mpl_tests_colorbar/colorbar_no_warning_rcparams_grid_true",
            grid_style,
        )
        .with_rc_setup(|rc| Ok(rc.set("axes.grid", "True")?)),
        classic("mpl_tests_colorbar/colorbar_axes_parameters", |fig| {
            let axs = fig.subplots(2, 1)?;
            let axs = axs.as_slice();
            let image = image(&axs[0], vec![vec![0.0, 1.0], vec![2.0, 3.0]])?;
            let both = || ColorbarOptions {
                axes: ColorbarAxes::Parents(axs.to_vec()),
                ..ColorbarOptions::default()
            };
            fig.colorbar(&image, both())?;
            fig.colorbar(&image, ColorbarOptions::beside(&axs[0]))?;
            for _ in 0..3 {
                fig.colorbar(&image, both())?;
            }
            Ok(())
        }),
        classic("mpl_tests_colorbar/colorbar_wrong_figure", |fig| {
            fig.set_layout_engine(LayoutEngine::constrained());
            let image = image(&fig.subplot(), vec![vec![0.0, 1.0]])?;
            let tight = Figure::new();
            tight.set_layout_engine(LayoutEngine::tight());
            tight.colorbar(&image, ColorbarOptions::default())?;
            Ok(())
        }),
    ]
}

/// `test_colorbar_no_warning_rcparams_grid_true`: under `axes.grid`, a
/// mesh in an axes without a grid and its colorbar.
fn grid_style(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.grid(false);
    let mesh = ax.pcolormesh(
        Mesh::from_coords(
            vec![0.0, 1.0],
            vec![0.0, 1.0],
            Grid::from_fn(1, 1, |_, _| 1.0),
        ),
        PcolorOptions::default(),
    )?;
    fig.colorbar(&mesh, ColorbarOptions::default())?;
    Ok(())
}

/// An image of `rows` in `ax`, in the default colours.
fn image(ax: &Axes, rows: Vec<Vec<f64>>) -> Result<AxesImage, Box<dyn Error>> {
    Ok(ax.imshow(Grid::from_rows(rows)?, ImshowOptions::default())?)
}
