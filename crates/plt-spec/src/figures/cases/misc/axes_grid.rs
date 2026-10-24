//! `axes_grid1` cases of `figures/cases_axes_misc.py`.

use plt::axes::{ImshowOptions, TickParams};
use plt::axes_grid1::{
    CbarMode, GridRect, ImageGrid, ImageGridOptions, LabelMode, SizeSpec, make_axes_locatable,
};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Colorize, ColorizerOptions, colormaps};
use plt::gridspec::GridSpec;
use plt::spines::SpineSide;

use super::field;
use crate::figures::{CaseResult, FigureCase};

/// `ImageGrid(fig, 111, ...)`: the grid divides the default subplot area.
fn subplot_111() -> Result<GridRect, plt::gridspec::GridSpecError> {
    Ok(GridRect::Subplot(GridSpec::new(1, 1)?.cell(0, 0)?))
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/axes_grid1_append_axes", append_axes).sized(5.0, 4.0, 100.0),
    FigureCase::new("misc/imagegrid_single_cbar", imagegrid_single_cbar).sized(5.0, 4.0, 100.0),
    FigureCase::new("misc/axesgrid_each_cbar_label_mode", axesgrid_each_cbar)
        .sized(6.0, 3.0, 100.0),
];

fn append_axes(fig: &plt::figure::Figure) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(field(20), ImshowOptions::default())?;
    let divider = make_axes_locatable(&ax);
    let cax = divider.append_axes(SpineSide::Right, SizeSpec::Percent(5.0), Some(0.05.into()));
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(cax),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&im, options)?;
    let top = divider.append_axes(SpineSide::Top, 0.6, Some(0.1.into()));
    top.share_x(&ax)?;
    top.plot_y(field(20).row(10));
    top.tick_params(&TickParams {
        labelbottom: Some(false),
        ..TickParams::default()
    });
    Ok(())
}

fn imagegrid_single_cbar(fig: &plt::figure::Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        cbar_mode: Some(CbarMode::Single),
        cbar_location: SpineSide::Right,
        cbar_pad: Some(0.1.into()),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot_111()?, (2, 2), options)?;
    let mut last = None;
    for (i, ax) in grid.axes().iter().enumerate() {
        let scale = (i as f64 + 1.0) / 4.0;
        let image = ImshowOptions {
            colorizer: ColorizerOptions::default().limits(Some(-1.0), Some(1.0)),
            ..ImshowOptions::default()
        };
        last = Some(ax.imshow(field(20).map(|v| v * scale), image)?);
    }
    let im = last.ok_or("no image")?;
    grid.cbar_axes()[0].colorbar(&im, ColorbarOptions::default())?;
    Ok(())
}

fn axesgrid_each_cbar(fig: &plt::figure::Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.5, 0.5),
        label_mode: LabelMode::L,
        cbar_mode: Some(CbarMode::Each),
        cbar_location: SpineSide::Top,
        cbar_pad: Some(0.05.into()),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot_111()?, (1, 3), options)?;
    let cmaps = ["viridis", "magma", "cividis"];
    for (i, (ax, cax)) in grid.axes().iter().zip(grid.cbar_axes()).enumerate() {
        let scale = i as f64 + 1.0;
        let image = ImshowOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get(cmaps[i])?),
            ..ImshowOptions::default()
        };
        let im = ax.imshow(field(10).map(|v| v * scale), image)?;
        cax.colorbar(&im, ColorbarOptions::default())?;
    }
    Ok(())
}
