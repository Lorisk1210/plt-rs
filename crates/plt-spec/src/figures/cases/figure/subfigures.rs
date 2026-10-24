//! Subfigures, super labels, the compressed layout and nested grid
//! specifications.

use plt::axes::{AxisName, TickParams};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::figure::{Share, SubfigureOptions};
use plt::fonts::{FontSize, RelativeSize};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::{LayoutEngine, TightLayout};
use plt::prelude::*;
use plt::spines::SpineSide;

use super::{field, field_mesh, map, t};
use crate::figures::CaseResult;
use plt::colors::ColorizerOptions;

/// `_example_plot(ax, seed, hide_labels)`: the field and, unless hidden,
/// labels and a title in 12 points.
fn example_plot(
    ax: &Axes,
    seed: u64,
    hide_labels: bool,
) -> Result<plt::collections::Collection, Box<dyn std::error::Error>> {
    let mesh = field_mesh(ax, seed, None)?;
    if !hide_labels {
        ax.set_xlabel("x-label").set_ylabel("y-label");
        for axis in [ax.xaxis(), ax.yaxis()] {
            axis.set_label_fontsize(FontSize::Points(12.0));
        }
        ax.set_title("Title").set_fontsize(12.0);
    }
    Ok(mesh)
}

/// A colour by name, resolved.
fn rgba(name: &str) -> Result<plt_core::Rgba, plt::colors::ColorError> {
    Color::from(name).resolve()
}

pub(super) fn gallery_subfigures(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let subfigs = fig.subfigures(
        1,
        2,
        SubfigureOptions {
            wspace: Some(0.07),
            ..SubfigureOptions::default()
        },
    )?;
    let (left, right) = (&subfigs[(0, 0)], &subfigs[(0, 1)]);
    let axs_left = left.subplots_shared(1, 2, Share::None, Share::All)?;
    left.set_facecolor(rgba("0.75")?);
    let mut last = None;
    for (i, ax) in axs_left.as_slice().iter().enumerate() {
        last = Some(example_plot(ax, i as u64, false)?);
    }
    left.suptitle("Left plots")
        .set_fontsize(RelativeSize::XLarge);
    left.colorbar(
        &last.ok_or("no mesh")?,
        ColorbarOptions {
            shrink: 0.6,
            axes: ColorbarAxes::Parents(axs_left.as_slice().to_vec()),
            location: Some(SpineSide::Bottom),
            ..ColorbarOptions::default()
        },
    )?;
    let axs_right = right.subplots_shared(3, 1, Share::All, Share::None)?;
    let mut last = None;
    for (nn, ax) in axs_right.as_slice().iter().enumerate() {
        last = Some(example_plot(ax, 10 + nn as u64, true)?);
        match nn {
            2 => {
                ax.set_xlabel("xlabel");
            }
            1 => {
                ax.set_ylabel("ylabel");
            }
            _ => {}
        }
    }
    right.set_facecolor(rgba("0.85")?);
    right.colorbar(
        &last.ok_or("no mesh")?,
        ColorbarOptions {
            shrink: 0.6,
            axes: ColorbarAxes::Parents(axs_right.as_slice().to_vec()),
            ..ColorbarOptions::default()
        },
    )?;
    right
        .suptitle("Right plots")
        .set_fontsize(RelativeSize::XLarge);
    fig.suptitle("Figure suptitle")
        .set_fontsize(RelativeSize::XxLarge);
    Ok(())
}

pub(super) fn subfigures_nested_ratios(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    fig.suptitle("fig");
    let subfigs = fig.subfigures(
        1,
        2,
        SubfigureOptions {
            wspace: Some(0.07),
            width_ratios: Some(vec![1.5, 1.0]),
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
    let ax0 = top.subplot();
    let mesh = example_plot(&ax0, 20, true)?;
    top.colorbar(
        &mesh,
        ColorbarOptions {
            axes: ColorbarAxes::Parent(ax0),
            ..ColorbarOptions::default()
        },
    )?;
    bottom.suptitle("subfigsnest[1]");
    bottom.set_facecolor(rgba("g")?);
    let axs = bottom.subplots_shared(3, 1, Share::All, Share::None)?;
    for (nn, ax) in axs.as_slice().iter().enumerate() {
        ax.set_ylabel(format!("ylabel{nn}"));
    }
    bottom.supxlabel("supxlabel");
    bottom.supylabel("supylabel");
    let t = t();
    let axs_right = subfigs[(0, 1)].subplots(2, 2)?;
    for (i, ax) in axs_right.as_slice().iter().enumerate() {
        ax.plot(&t, map(&t, |t| (t + i as f64).cos()))?;
    }
    Ok(())
}

pub(super) fn tight_suptitle_suplabels(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let t = t();
    for (i, ax) in axs.as_slice().iter().enumerate() {
        ax.plot(map(&t, |t| t * 1000.0), map(&t, |t| (t + i as f64).sin()))?;
        ax.set_title(format!("panel {i}"));
        ax.tick_params(&TickParams {
            axis: Some(AxisName::X),
            labelrotation: Some(30.0),
            ..TickParams::default()
        });
    }
    fig.suptitle("A tight suptitle").set_fontsize(16.0);
    fig.supxlabel("time [ms]");
    fig.supylabel("signal");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) fn compressed_layout_images(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let axs = fig.subplots(2, 2)?;
    let mut last = None;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        let options = plt::axes::ImshowOptions {
            colorizer: ColorizerOptions::default().limits(Some(-2.5), Some(2.5)),
            ..plt::axes::ImshowOptions::default()
        };
        last = Some(ax.imshow(field(30 + i as u64), options)?);
        ax.set_title(format!("image {i}"));
    }
    fig.colorbar(
        &last.ok_or("no image")?,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
            shrink: 0.6,
            ..ColorbarOptions::default()
        },
    )?;
    fig.suptitle("compressed");
    Ok(())
}

pub(super) fn constrained_subgridspec_colorbar(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let gs = GridSpec::with_ratios(
        1,
        2,
        Some(vec![1.0, 1.5]),
        None,
        SubplotOverrides::default(),
    )?;
    let left = GridSpec::nested(&gs.cell(0, 0)?, 2, 1, None, None, (None, None))?;
    let right = GridSpec::nested(&gs.cell(0, 1)?, 2, 2, None, None, (Some(0.05), None))?;
    let ax_top = fig.add_subplot_spec(&left.cell(0, 0)?)?;
    let ax_bottom = fig.add_subplot_spec(&left.cell(1, 0)?)?;
    let t = t();
    ax_top.plot(&t, map(&t, f64::sin))?;
    ax_bottom.plot(&t, map(&t, f64::cos))?;
    ax_bottom.set_xlabel("time");
    let mut images = Vec::new();
    for r in 0..2 {
        for c in 0..2 {
            images.push(fig.add_subplot_spec(&right.cell(r, c)?)?);
        }
    }
    let mut last = None;
    for (i, ax) in images.iter().enumerate() {
        last = Some(field_mesh(ax, 40 + i as u64, Some("magma"))?);
    }
    let mesh = last.ok_or("no mesh")?;
    fig.colorbar(
        &mesh,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(images.clone()),
            label: "field".into(),
            ..ColorbarOptions::default()
        },
    )?;
    fig.colorbar(
        &mesh,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(images[2..].to_vec()),
            location: Some(SpineSide::Bottom),
            shrink: 0.7,
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}
