//! The constrained-layout section of `figures/cases_layout.py`, plus the
//! colorbars that steal space without a layout engine.

use plt::axes::{Mesh, PcolorOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Extend, colormaps};
use plt::figure::{Share, SubfigureOptions};
use plt::image::ColorizerOptions;
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::spines::SpineSide;

use super::fill;
use crate::figures::{CaseResult, FigureCase};

/// `GRID = np.add.outer(np.sin(np.linspace(0, 3, 12)),
/// np.cos(np.linspace(0, 4, 16)))`, scaled by `gain`.
fn grid(gain: f64) -> Mesh {
    let rows = linspace(0.0, 3.0, 12);
    let cols = linspace(0.0, 4.0, 16);
    Mesh::new(Grid::from_fn(12, 16, |r, c| {
        (rows[r].sin() + cols[c].cos()) * gain
    }))
}

/// `pcolormesh` options with a named colormap.
fn colored(cmap: &str) -> Result<PcolorOptions, plt::colors::ColormapError> {
    Ok(PcolorOptions {
        colors: ColorizerOptions::default().cmap(colormaps::get(cmap)?),
        ..PcolorOptions::default()
    })
}

/// `fig.set_layout_engine("constrained")`.
fn constrained(fig: &Figure) {
    fig.set_layout_engine(Some(
        LayoutEngine::Constrained(ConstrainedLayout::default()),
    ));
}

/// Colorbar options stealing space from `ax`.
fn beside(ax: &Axes) -> ColorbarOptions {
    ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.clone()),
        ..ColorbarOptions::default()
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("layout/constrained_basic", |fig| {
        constrained(fig);
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            fill(ax, i)?;
            ax.set_title(format!("title {i}"));
            ax.set_xlabel("x label").set_ylabel("y label");
        }
        Ok(())
    }),
    FigureCase::new("layout/constrained_colorbars", |fig| {
        constrained(fig);
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            let mesh = ax.pcolormesh(grid(i as f64 + 1.0), colored("viridis")?)?;
            fig.colorbar(&mesh, beside(ax))?;
        }
        Ok(())
    }),
    FigureCase::new(
        "layout/constrained_shared_colorbar",
        constrained_shared_colorbar,
    ),
    FigureCase::new("layout/constrained_suptitle_suplabels", |fig| {
        constrained(fig);
        let axs = fig.subplots_shared(1, 3, Share::None, Share::All)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            fill(ax, i)?;
        }
        fig.suptitle("Constrained suptitle").set_fontsize(16.0);
        fig.supxlabel("time");
        fig.supylabel("value");
        Ok(())
    }),
    FigureCase::new("layout/constrained_pads", |fig| {
        fig.set_layout_engine(Some(LayoutEngine::Constrained(ConstrainedLayout {
            w_pad: 0.2,
            h_pad: 0.2,
            wspace: 0.1,
            hspace: 0.1,
            ..ConstrainedLayout::default()
        })));
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
        let mesh = left.pcolormesh(grid(1.0), PcolorOptions::default())?;
        fig.colorbar(&mesh, beside(left))?;
        let mesh = right.pcolormesh(grid(1.0), colored("magma")?)?;
        let options = ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            extend: Some(Extend::Both),
            ..beside(right)
        };
        fig.colorbar(&mesh, options)?;
        Ok(())
    }),
];

fn constrained_shared_colorbar(fig: &Figure) -> CaseResult {
    constrained(fig);
    let axs = fig.subplots(2, 2)?;
    let options = || -> Result<PcolorOptions, plt::colors::ColormapError> {
        Ok(PcolorOptions {
            colors: ColorizerOptions::default()
                .cmap(colormaps::get("RdBu_r")?)
                .limits(Some(-2.0), Some(2.0)),
            ..PcolorOptions::default()
        })
    };
    let mut last = None;
    for ax in axs.as_slice() {
        last = Some(ax.pcolormesh(grid(1.0), options()?)?);
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
    constrained(fig);
    let rows = fig.subfigures(2, 1, &SubfigureOptions::default())?;
    let (top, bottom) = (&rows[(0, 0)], &rows[(1, 0)]);
    top.suptitle("top");
    for (i, ax) in top.subplots(1, 3)?.as_slice().iter().enumerate() {
        fill(ax, i)?;
    }
    let cols = bottom.subfigures(1, 2, &SubfigureOptions::default())?;
    let (left, right) = (&cols[(0, 0)], &cols[(0, 1)]);
    left.set_facecolor(Color::from("lightyellow").resolve()?);
    left.supxlabel("left x");
    fill(&left.subplot(), 0)?;
    right.set_facecolor(Color::from("lightcyan").resolve()?);
    let mesh = right
        .subplot()
        .pcolormesh(grid(1.0), PcolorOptions::default())?;
    right.colorbar(&mesh, ColorbarOptions::default())?;
    Ok(())
}
