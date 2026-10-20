//! Image grids and dividers of `figures/cases_toolkits.py`.

use plt::axes::{AxisName, HistPlotOptions, ImshowOptions, ScatterOptions, TickParams};
use plt::axes_grid1::{
    CbarMode, GridDirection, GridRect, ImageGrid, ImageGridOptions, LabelMode, SizeSpec,
    make_axes_area_auto_adjustable, make_axes_locatable,
};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Color, colormaps};
use plt::figure::Figure;
use plt::gridspec::{GridSpec, GridSpecError};
use plt::image::ColorizerOptions;
use plt::lines::LineProps;
use plt::numeric::linspace;
use plt::orientation::Orientation;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::stats::{Bins, HistOptions, HistOrientation};

use super::field;
use crate::figures::CaseResult;
use crate::rng::SpecRng;

/// The cell `index` of a one-row grid of `ncols` subplots (`12x` codes).
fn subplot(ncols: usize, index: usize) -> Result<GridRect, GridSpecError> {
    Ok(GridRect::Subplot(GridSpec::new(1, ncols)?.cell(0, index)?))
}

/// `imshow(data, cmap=cmap)` options with optional limits.
fn image(
    cmap: &str,
    limits: Option<(f64, f64)>,
) -> Result<ImshowOptions, Box<dyn std::error::Error>> {
    let mut colors = ColorizerOptions::default().cmap(colormaps::get(cmap)?);
    if let Some((vmin, vmax)) = limits {
        colors = colors.limits(Some(vmin), Some(vmax));
    }
    Ok(ImshowOptions {
        colors,
        ..ImshowOptions::default()
    })
}

pub(super) fn cbar_edge(fig: &Figure) -> CaseResult {
    let right = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        cbar_mode: Some(CbarMode::Edge),
        cbar_location: SpineSide::Right,
        cbar_pad: Some(0.05.into()),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(2, 0)?, (2, 2), &right)?;
    for (i, ax) in grid.axes().iter().enumerate() {
        let cmap = ["viridis", "plasma"][i / 2];
        let im = ax.imshow(field(20, i as f64), image(cmap, Some((-1.0, 1.0)))?)?;
        if i % 2 == 1 {
            grid.cbar_axes()[i / 2].colorbar(&im, ColorbarOptions::default())?;
        }
    }
    let bottom = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        cbar_mode: Some(CbarMode::Edge),
        cbar_location: SpineSide::Bottom,
        cbar_pad: Some(0.25.into()),
        cbar_size: SizeSpec::Percent(10.0),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(2, 1)?, (2, 2), &bottom)?;
    for (i, ax) in grid.axes().iter().enumerate() {
        let cmap = ["cividis", "magma"][i % 2];
        let scale = 1.0 + (i % 2) as f64;
        let data = field(20, -(i as f64)).map(|v| v * scale);
        let im = ax.imshow(data, image(cmap, None)?)?;
        if i >= 2 {
            grid.cbar_axes()[i % 2].colorbar(&im, ColorbarOptions::default())?;
        }
    }
    Ok(())
}

pub(super) fn label_modes(fig: &Figure) -> CaseResult {
    let lines = ImageGridOptions {
        n_axes: Some(5),
        direction: GridDirection::Column,
        axes_pad: (0.3, 0.2),
        share_all: true,
        aspect: false,
        label_mode: LabelMode::One,
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(2, 0)?, (2, 3), &lines)?;
    let x = linspace(0.0, 2.0 * std::f64::consts::PI, 50);
    for (i, ax) in grid.axes().iter().enumerate() {
        let shifted: Vec<f64> = x.iter().map(|x| (x + i as f64).sin()).collect();
        ax.plot_with(
            &x,
            shifted,
            LineProps::default().color(format!("C{i}").as_str()),
        )?;
        ax.set_title(i.to_string()).set_fontsize(8.0);
    }
    let images = ImageGridOptions {
        axes_pad: (0.4, 0.4),
        label_mode: LabelMode::All,
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(2, 1)?, (2, 2), &images)?;
    for (i, ax) in grid.axes().iter().enumerate() {
        ax.imshow(field(8, i as f64), image("gray", None)?)?;
    }
    Ok(())
}

pub(super) fn each_cbar_left(fig: &Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.7, 0.7),
        cbar_mode: Some(CbarMode::Each),
        cbar_location: SpineSide::Left,
        cbar_size: SizeSpec::Percent(10.0),
        cbar_pad: Some(0.25.into()),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(1, 0)?, (1, 3), &options)?;
    for (i, (ax, cax)) in grid.axes().iter().zip(grid.cbar_axes()).enumerate() {
        let scale = 10f64.powi(i32::try_from(i)?);
        let im = ax.imshow(
            field(12, i as f64).map(|v| v * scale),
            image("RdBu_r", None)?,
        )?;
        let label = ColorbarOptions {
            label: format!("scale 1e{i}"),
            ..ColorbarOptions::default()
        };
        cax.colorbar(&im, label)?;
    }
    Ok(())
}

pub(super) fn divider_sides(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = field(30, 0.0);
    let im = ax.imshow(data.clone(), image("viridis", None)?)?;
    let divider = make_axes_locatable(&ax);
    let right = divider.append_axes(SpineSide::Right, SizeSpec::Percent(5.0), Some(0.05.into()));
    fig.colorbar(
        &im,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(right),
            ..ColorbarOptions::default()
        },
    )?;
    let bottom = divider.append_axes(SpineSide::Bottom, 0.15, Some(0.5.into()));
    fig.colorbar(
        &im,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(bottom),
            orientation: Some(Orientation::Horizontal),
            ..ColorbarOptions::default()
        },
    )?;
    let left = divider.append_axes(SpineSide::Left, SizeSpec::Percent(20.0), Some(0.4.into()));
    left.share_y(&ax)?;
    let means: Vec<f64> = (0..30)
        .map(|r| data.row(r).iter().map(|v| v.abs()).sum::<f64>() / 30.0)
        .collect();
    let rows: Vec<f64> = (0..30).map(f64::from).collect();
    left.plot(means, rows)?;
    left.xaxis().set_inverted(true);
    Ok(())
}

pub(super) fn marginal_histograms(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut r = SpecRng::new(41);
    let x = r.normal(400, 0.0, 1.0);
    let noise = r.normal(400, 0.0, 0.7);
    let y: Vec<f64> = x.iter().zip(&noise).map(|(x, n)| 0.6 * x + n).collect();
    let points = ScatterOptions {
        sizes: Some(vec![6.0]),
        alpha: 0.6.into(),
        ..ScatterOptions::default()
    };
    ax.scatter_with(&x, &y, points)?;
    ax.set_aspect(Aspect::Ratio(1.0))?;
    let divider = make_axes_locatable(&ax);
    let top = divider.append_axes(SpineSide::Top, 1.0, Some(0.1.into()));
    top.share_x(&ax)?;
    let right = divider.append_axes(SpineSide::Right, 1.0, Some(0.1.into()));
    right.share_y(&ax)?;
    top.tick_params(&TickParams {
        axis: Some(AxisName::X),
        labelbottom: Some(false),
        ..TickParams::default()
    });
    right.tick_params(&TickParams {
        axis: Some(AxisName::Y),
        labelleft: Some(false),
        ..TickParams::default()
    });
    let edges = linspace(-4.0, 4.0, 33);
    let upper = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Edges(edges.clone())),
            ..HistOptions::default()
        },
        ..HistPlotOptions::default()
    };
    top.hist(&[x], &upper)?;
    let side = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Edges(edges)),
            orientation: HistOrientation::Horizontal,
            ..HistOptions::default()
        },
        color: vec![Color::from("tab:orange")],
        ..HistPlotOptions::default()
    };
    right.hist(&[y], &side)?;
    Ok(())
}

/// `toolkits/auto_adjustable_area`: an axes filling the figure, shrunk
/// until a long tick label fits.
pub(super) fn auto_adjustable(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.set_yticks(vec![0.5])
        .set_yticklabels(["very long label"])?;
    make_axes_area_auto_adjustable(&ax, &[&ax], 0.1, &SpineSide::ALL);
    Ok(())
}

/// `toolkits/auto_adjustable_area_shared`: two stacked axes, the first
/// making room for the decorations of both.
pub(super) fn auto_adjustable_shared(fig: &Figure) -> CaseResult {
    let ax1 = fig.add_axes([0.0, 0.0, 1.0, 0.5]);
    let ax2 = fig.add_axes([0.0, 0.5, 1.0, 0.5]);
    ax1.set_yticks(vec![0.5])
        .set_yticklabels(["very long label"])?;
    ax1.set_ylabel("Y label");
    ax2.set_title("Title");
    make_axes_area_auto_adjustable(&ax1, &[&ax1, &ax2], 0.1, &SpineSide::ALL);
    make_axes_area_auto_adjustable(&ax2, &[&ax2], 0.1, &SpineSide::ALL);
    Ok(())
}
