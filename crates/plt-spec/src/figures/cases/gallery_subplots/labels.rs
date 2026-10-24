//! Titles and labels: `align_labels_demo`, `axis_labels_demo` and
//! `axes_props`.

use plt::axes::{AxisName, GridOptions, LabelLoc, TickParams};
use plt::colorbar::ColorbarOptions;
use plt::figure::Share;
use plt::fonts::{FontSize, RelativeSize};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::text::TextBoxProps;

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};

/// `align_labels_demo`'s pale yellow label boxes.
fn yellow_box() -> TextBoxProps {
    TextBoxProps {
        pad: Some(5.0),
        patch: PatchProps::default().facecolor("yellow").alpha(0.2),
        ..TextBoxProps::default()
    }
}

fn align_labels_demo(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    let grid = GridSpec::with_ratios(
        2,
        3,
        None,
        None,
        SubplotOverrides {
            wspace: Some(0.1),
            ..SubplotOverrides::default()
        },
    )?;
    let axs = fig.subplots_on(&grid, Share::None, Share::None)?;
    let rise = map(&linspace(0.0, 5.0, 100), |x| 1.0 - (-x).exp());
    for ax in axs.as_slice() {
        let spec = ax.subplot_spec().ok_or("subplots lie on a grid")?;
        let scale = if spec.is_first_row() { 2000.0 } else { 1.0 };
        ax.plot_y(map(&rise, |y| scale * y));
        if spec.is_last_row() {
            ax.set_xlabel("xlabel");
            ax.xaxis().set_label_bbox(Some(yellow_box()));
        }
        ax.set_ylabel("ylabel");
        ax.yaxis().set_label_bbox(Some(yellow_box()));
        ax.set_ylim(0.0, scale);
    }
    axs[(0, 0)].xaxis().tick_to(SpineSide::Top)?;
    axs[(1, 2)].tick_params(&TickParams {
        axis: Some(AxisName::X),
        labelrotation: Some(55.0),
        ..TickParams::default()
    });
    axs[(0, 0)].set_title("ylabels not aligned");
    fig.align_titles(None);
    fig.align_xlabels(None);
    fig.align_ylabels(Some(&[axs[(0, 1)].clone(), axs[(1, 1)].clone()]));
    axs[(0, 1)].set_title("fig.align_ylabels()");
    for row in 0..2 {
        axs[(row, 2)].yaxis().set_label_coords(-0.3, 0.5);
    }
    axs[(0, 2)].set_title("ylabels manually aligned");
    Ok(())
}

fn axis_labels_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ScatterOptions {
        colors: MappedColors::new([1.0, 2.0]).into(),
        ..ScatterOptions::default()
    };
    let sc = ax.scatter_with([1.0, 2.0], [1.0, 2.0], options)?;
    ax.set_ylabel("YLabel");
    ax.yaxis().set_label_loc(LabelLoc::End);
    ax.set_xlabel("XLabel");
    ax.xaxis().set_label_loc(LabelLoc::Start);
    fig.colorbar(&sc, ColorbarOptions::default())?
        .set_label("ZLabel", Some(LabelLoc::End));
    Ok(())
}

fn axes_props(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 2.0, 0.01);
    let ax = fig.subplot();
    ax.plot(&t, map(&t, |t| (2.0 * std::f64::consts::PI * t).sin()))?;
    ax.grid_with(
        true,
        GridOptions {
            linestyle: Some(LineStyle::DashDot),
            ..GridOptions::default()
        },
    );
    ax.tick_params(&TickParams {
        labelcolor: Some("r".into()),
        labelsize: Some(FontSize::Relative(RelativeSize::Medium)),
        width: Some(3.0),
        ..TickParams::default()
    });
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_subplots/align_labels_demo", align_labels_demo).sized(8.9, 5.5, 100.0),
    FigureCase::new("gallery_subplots/axis_labels_demo", axis_labels_demo),
    FigureCase::new("gallery_subplots/axes_props", axes_props),
];
