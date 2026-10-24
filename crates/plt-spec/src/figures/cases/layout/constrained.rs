//! The constrained-layout section of `figures/cases_layout.py`, plus the
//! colorbars that steal space without a layout engine.

use plt::axes::{Mesh, PcolorOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{ColorizerOptions, Extend, colormaps};
use plt::figure::{Share, SubfigureOptions};
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::legend::{LegendLoc, LegendOptions};
use plt::offsetbox::AnchorLoc;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::spines::SpineSide;

use super::{fill, grid};
use crate::figures::{CaseResult, FigureCase};

/// [`grid`] scaled by `gain`, as a mesh.
fn scaled_grid(gain: f64) -> Mesh {
    Mesh::new(grid().map(|v| v * gain))
}

/// `pcolormesh` options with a named colormap.
fn colored(cmap: &str) -> Result<PcolorOptions, plt::colors::ColormapError> {
    Ok(PcolorOptions {
        colorizer: ColorizerOptions::default().cmap(colormaps::get(cmap)?),
        ..PcolorOptions::default()
    })
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("layout/constrained_basic", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            fill(ax, i)?;
            ax.set_title(format!("title {i}"));
            ax.set_xlabel("x label").set_ylabel("y label");
        }
        Ok(())
    }),
    FigureCase::new("layout/constrained_colorbars", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            let mesh = ax.pcolormesh(scaled_grid(i as f64 + 1.0), colored("viridis")?)?;
            fig.colorbar(&mesh, ColorbarOptions::beside(ax))?;
        }
        Ok(())
    }),
    FigureCase::new(
        "layout/constrained_shared_colorbar",
        constrained_shared_colorbar,
    ),
    FigureCase::new("layout/constrained_suptitle_suplabels", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let axs = fig.subplots_shared(1, 3, Share::None, Share::All)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            fill(ax, i)?;
        }
        fig.suptitle("Constrained suptitle").set_fontsize(16.0);
        fig.supxlabel("time");
        fig.supylabel("value");
        Ok(())
    }),
    FigureCase::new("layout/constrained_legend_outside", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let t = linspace(0.0, 10.0, 40);
        let axs = fig.subplots(1, 2)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            let phase = i as f64;
            let sin: Vec<f64> = t.iter().map(|&t| (t + phase).sin()).collect();
            let cos: Vec<f64> = t.iter().map(|&t| (t + phase).cos()).collect();
            for (name, y) in [("sin", sin), ("cos", cos)] {
                ax.plot_with(
                    t.clone(),
                    y,
                    LineProps::default().label(format!("{name} {i}")),
                )?;
            }
        }
        fig.legend(
            LegendOptions::default()
                .loc("outside upper center".parse::<LegendLoc>()?)
                .ncols(4),
        )?;
        axs[(0, 1)].legend(
            LegendOptions::default()
                .loc(AnchorLoc::CenterLeft)
                .bbox_to_anchor((1.0, 0.5)),
        )?;
        Ok(())
    }),
    FigureCase::new("layout/constrained_pads", |fig| {
        fig.set_layout_engine(Some(LayoutEngine::Constrained(ConstrainedLayout {
            w_pad: 0.2,
            h_pad: 0.2,
            wspace: 0.1,
            hspace: 0.1,
            ..ConstrainedLayout::default()
        })))?;
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            fill(ax, i)?;
        }
        Ok(())
    }),
    FigureCase::new(
        "layout/subfigures_nested_constrained",
        subfigures_nested_constrained,
    ),
    FigureCase::new("layout/colorbar_steals_space", |fig| {
        let axs = fig.subplots(1, 2)?;
        let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
        let mesh = left.pcolormesh(scaled_grid(1.0), PcolorOptions::default())?;
        fig.colorbar(&mesh, ColorbarOptions::beside(left))?;
        let mesh = right.pcolormesh(scaled_grid(1.0), colored("magma")?)?;
        let options = ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            extend: Some(Extend::Both),
            ..ColorbarOptions::beside(right)
        };
        fig.colorbar(&mesh, options)?;
        Ok(())
    }),
];

fn constrained_shared_colorbar(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 2)?;
    let options = || -> Result<PcolorOptions, plt::colors::ColormapError> {
        Ok(PcolorOptions {
            colorizer: ColorizerOptions::default()
                .cmap(colormaps::get("RdBu_r")?)
                .limits(Some(-2.0), Some(2.0)),
            ..PcolorOptions::default()
        })
    };
    let mut last = None;
    for ax in axs.as_slice() {
        last = Some(ax.pcolormesh(scaled_grid(1.0), options()?)?);
    }
    let mesh = last.ok_or("no mesh")?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
        shrink: 0.6,
        location: Some(SpineSide::Bottom),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&mesh, options)?;
    Ok(())
}

fn subfigures_nested_constrained(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let rows = fig.subfigures(2, 1, SubfigureOptions::default())?;
    let (top, bottom) = (&rows[(0, 0)], &rows[(1, 0)]);
    top.suptitle("top");
    for (i, ax) in top.subplots(1, 3)?.as_slice().iter().enumerate() {
        fill(ax, i)?;
    }
    let cols = bottom.subfigures(1, 2, SubfigureOptions::default())?;
    let (left, right) = (&cols[(0, 0)], &cols[(0, 1)]);
    left.set_facecolor(Color::from("lightyellow").resolve()?);
    left.supxlabel("left x");
    fill(&left.subplot(), 0)?;
    right.set_facecolor(Color::from("lightcyan").resolve()?);
    let mesh = right
        .subplot()
        .pcolormesh(scaled_grid(1.0), PcolorOptions::default())?;
    right.colorbar(&mesh, ColorbarOptions::default())?;
    Ok(())
}
