//! `test_figure.py`, subfigures: side by side, on a grid cell, nested
//! twice, and with scatter markers that must keep their size.

use pltrs::axes::{Mesh, PcolorOptions};
use pltrs::collections::Collection;
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::figure::{Share, SubfigureOptions};
use pltrs::fonts::RelativeSize;
use pltrs::layout::LayoutEngine;
use pltrs::legend::{LegendHandle, LegendOptions};
use pltrs::prelude::*;
use pltrs::spines::SpineSide;
use pltrs_core::Rgba;

use super::{Fallible, TEAL, decorate, randn, remove_ticks_and_titles};
use crate::figures::cases::common::rgba;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;
use pltrs::colors::ColorizerOptions;
use pltrs::offsetbox::AnchorLoc;

/// The tests' seed for `np.random.seed`.
const SEED: u64 = 19_680_801;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_figure/subfigure", subfigure).with_rc(&[TEAL]),
    FigureCase::new("mpl_figure/subfigure_ss", subfigure_ss).with_rc(&[TEAL]),
    FigureCase::new("mpl_figure/subfigure_double", subfigure_double)
        .sized(10.0, 8.0, 100.0)
        .with_rc(&[TEAL]),
    FigureCase::new("mpl_figure/subfigure_scatter_size", subfigure_scatter_size),
];

/// `ax.pcolormesh(np.random.randn(30, 30), vmin=-limit, vmax=limit)`.
fn random_mesh(ax: &Axes, stream: &mut SpecRng, limit: f64) -> Fallible<Collection> {
    let options = PcolorOptions {
        colorizer: ColorizerOptions::default().limits(Some(-limit), Some(limit)),
        ..PcolorOptions::default()
    };
    Ok(ax.pcolormesh(Mesh::new(randn(stream, 30)?), options)?)
}

/// A random mesh in each of `axs` and one colorbar for them all at
/// `location`.
fn meshes_with_colorbar(
    sub: &Figure,
    axs: &[Axes],
    stream: &mut SpecRng,
    location: Option<SpineSide>,
) -> CaseResult {
    let mut pc = None;
    for ax in axs {
        pc = Some(random_mesh(ax, stream, 2.0)?);
    }
    sub.colorbar(
        &pc.ok_or("no mesh")?,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(axs.to_vec()),
            location,
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

fn subfigure(fig: &Figure) -> CaseResult {
    let mut stream = SpecRng::new(SEED);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let sub = fig.subfigures(1, 2, SubfigureOptions::default())?;
    let (left, right) = (&sub[(0, 0)], &sub[(0, 1)]);

    let axs = left.subplots(2, 2)?;
    meshes_with_colorbar(left, axs.as_slice(), &mut stream, None)?;
    left.suptitle("Left Side");
    left.set_facecolor(Rgba::WHITE);

    let axs = right.subplots(1, 3)?;
    meshes_with_colorbar(right, axs.as_slice(), &mut stream, Some(SpineSide::Bottom))?;
    right.suptitle("Right Side");
    right.set_facecolor(Rgba::WHITE);

    fig.suptitle("Figure suptitle")
        .set_fontsize(RelativeSize::XxLarge);

    // The draw order of subfigures.
    let handles =
        (0..5).map(|i| LegendHandle::line(LineProps::default().label(format!("Line{i}"))));
    let leg = fig.legend(
        LegendOptions::default()
            .handles(handles)
            .loc(AnchorLoc::Center),
    )?;
    left.set_zorder(leg.zorder() - 1.0);
    right.set_zorder(leg.zorder() + 1.0);
    Ok(())
}

fn subfigure_ss(fig: &Figure) -> CaseResult {
    let mut stream = SpecRng::new(SEED);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let gs = fig.add_gridspec(1, 2)?;

    let sub = fig.add_subfigure(&gs.cell(0, 0)?);
    sub.set_facecolor(rgba("pink")?);

    let axs = sub.subplots(2, 2)?;
    meshes_with_colorbar(&sub, axs.as_slice(), &mut stream, None)?;
    sub.suptitle("Left Side");

    let ax = fig.add_subplot_spec(&gs.cell(0, 1)?)?;
    ax.plot_y(arange(0.0, 20.0, 1.0));
    ax.set_title("Axes");

    fig.suptitle("Figure suptitle")
        .set_fontsize(RelativeSize::XxLarge);
    Ok(())
}

fn subfigure_double(fig: &Figure) -> CaseResult {
    let mut stream = SpecRng::new(SEED);
    fig.set_layout_engine(LayoutEngine::constrained())?;

    fig.suptitle("fig");

    let subfigs = fig.subfigures(
        1,
        2,
        SubfigureOptions {
            wspace: Some(0.07),
            ..SubfigureOptions::default()
        },
    )?;
    for (i, sub) in subfigs.as_slice().iter().enumerate() {
        sub.set_facecolor(rgba("coral")?);
        sub.suptitle(format!("subfigs[{i}]"));
    }

    let nested = subfigs[(0, 0)].subfigures(
        2,
        1,
        SubfigureOptions {
            height_ratios: Some(vec![1.0, 1.4]),
            ..SubfigureOptions::default()
        },
    )?;
    let (top, bottom) = (&nested[(0, 0)], &nested[(1, 0)]);
    top.suptitle("subfigsnest[0]");
    top.set_facecolor(rgba("r")?);
    let axsnest0 = top.subplots_shared(1, 2, Share::None, Share::All)?;
    let mut pc = None;
    for ax in axsnest0.as_slice() {
        pc = Some(random_mesh(ax, &mut stream, 2.5)?);
        decorate(ax, 12.0);
    }
    top.colorbar(
        &pc.ok_or("no mesh")?,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(axsnest0.as_slice().to_vec()),
            ..ColorbarOptions::default()
        },
    )?;

    bottom.suptitle("subfigsnest[1]");
    bottom.set_facecolor(rgba("g")?);
    let axsnest1 = bottom.subplots_shared(3, 1, Share::All, Share::None)?;
    for (nn, ax) in axsnest1.as_slice().iter().enumerate() {
        ax.set_ylabel(format!("ylabel{nn}"));
    }
    bottom.supxlabel("supxlabel");
    bottom.supylabel("supylabel");

    subfigs[(0, 1)].subplots(2, 2)?;
    Ok(())
}

fn subfigure_scatter_size(fig: &Figure) -> CaseResult {
    let gs = fig.add_gridspec(1, 2)?;
    let ax0 = fig.add_subplot_spec(&gs.cell(0, 1)?)?;
    scatter_squares(&ax0, None)?;

    let sfig = fig.add_subfigure(&gs.cell(0, 0)?);
    let axs = sfig.subplots(1, 2)?;
    for ax in [&ax0, &axs[(0, 0)]] {
        scatter_squares(ax, Some(("r", "g")))?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// Square markers of size 30 at `(1..3, 1..3)` and of sizes 20, 30 and
/// 40 at `(3..5, 1..3)`, in `colors` or the property cycle's.
fn scatter_squares(ax: &Axes, colors: Option<(&str, &str)>) -> CaseResult {
    let sets = [
        ([1.0, 2.0, 3.0], vec![30.0], colors.map(|c| c.0)),
        ([3.0, 4.0, 5.0], vec![20.0, 30.0, 40.0], colors.map(|c| c.1)),
    ];
    for (x, sizes, color) in sets {
        let options = ScatterOptions {
            marker: Some("s".parse()?),
            sizes: Some(sizes),
            colors: color.map_or(ScatterColors::Cycle, |c| ScatterColors::Single(c.into())),
            ..ScatterOptions::default()
        };
        ax.scatter_with(x.to_vec(), vec![1.0, 2.0, 3.0], options)?;
    }
    Ok(())
}
