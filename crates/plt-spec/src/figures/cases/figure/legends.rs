//! Figure and subfigure legends, and the tight bounding box of saved
//! figures.

use plt::figure::SubfigureOptions;
use plt::fonts::{FontSize, RelativeSize};
use plt::gridspec::SubplotOverrides;
use plt::layout::LayoutEngine;
use plt::legend::{BboxToAnchor, LegendHandle, LegendLoc, LegendMode, LegendOptions};
use plt::offsetbox::AnchorLoc;
use plt::prelude::*;
use plt::text::{HAlign, VAlign};
use plt::transforms::Transform;

use super::{map, t};
use crate::figures::CaseResult;

pub(super) fn legends_outside_constrained(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 2)?;
    let t = t();
    for (i, ax) in axs.as_slice().iter().enumerate() {
        let shift = i as f64;
        ax.plot(&t, map(&t, |t| (t + shift).sin()))?
            .set_label(format!("sin {i}"));
        ax.plot_fmt(&t, map(&t, |t| (t + shift).cos()), "--")?
            .set_label(format!("cos {i}"));
    }
    let (handles, labels) = axs[(0, 0)].legend_handles_labels();
    fig.legend(LegendOptions {
        handles: Some(handles.clone()),
        labels: Some(labels.clone()),
        loc: Some("outside right upper".parse::<LegendLoc>()?),
        title: Some("right upper".into()),
        ..LegendOptions::default()
    })?;
    fig.legend(LegendOptions {
        handles: Some(handles[..1].to_vec()),
        labels: Some(labels[..1].to_vec()),
        loc: Some("outside lower left".parse::<LegendLoc>()?),
        title: Some("lower left".into()),
        ncols: 1,
        ..LegendOptions::default()
    })?;
    fig.legend(LegendOptions {
        handles: Some(handles),
        labels: Some(labels),
        loc: Some("outside left center".parse::<LegendLoc>()?),
        frameon: Some(false),
        ..LegendOptions::default()
    })?;
    Ok(())
}

pub(super) fn legend_bbox_figure_coords(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let t = t();
    let mut lines = Vec::new();
    for (i, ax) in axs.as_slice().iter().enumerate() {
        let line = ax.plot(&t, map(&t, |t| (t + i as f64).sin()))?;
        line.set_label(format!("series {i}"));
        lines.push(LegendHandle::from(&line));
    }
    fig.subplots_adjust(&SubplotOverrides {
        bottom: Some(0.25),
        ..SubplotOverrides::default()
    })?;
    fig.legend(LegendOptions {
        handles: Some(lines.clone()),
        loc: Some(LegendLoc::Anchored(AnchorLoc::LowerCenter)),
        bbox_to_anchor: Some(BboxToAnchor::Point(0.5, 0.02)),
        bbox_transform: Some(Transform::Figure(fig.id())),
        ncols: 2,
        mode: LegendMode::Expand,
        borderaxespad: Some(0.5),
        ..LegendOptions::default()
    })?;
    fig.legend(LegendOptions {
        handles: Some(lines[..1].to_vec()),
        loc: Some(LegendLoc::Anchored(AnchorLoc::UpperRight)),
        bbox_to_anchor: Some(BboxToAnchor::Point(1.0, 1.0)),
        fontsize: Some(FontSize::Relative(RelativeSize::Small)),
        title: Some("anchored".into()),
        title_fontsize: Some(FontSize::Relative(RelativeSize::Small)),
        ..LegendOptions::default()
    })?;
    Ok(())
}

pub(super) fn subfigure_legends_suplabels(fig: &Figure) -> CaseResult {
    let subfigs = fig.subfigures(
        1,
        2,
        SubfigureOptions {
            wspace: Some(0.05),
            ..SubfigureOptions::default()
        },
    )?;
    let (left, right) = (&subfigs[(0, 0)], &subfigs[(0, 1)]);
    let t = t();
    for (sub, offset) in [(left, 0.0), (right, 1.0)] {
        let ax = sub.subplot();
        ax.plot(&t, map(&t, |t| (t + offset).sin()))?
            .set_label("sin");
        ax.plot(&t, map(&t, |t| (t + offset).cos()))?
            .set_label("cos");
        sub.supxlabel(format!("supxlabel {offset}"));
    }
    left.legend(LegendOptions {
        loc: Some(LegendLoc::Anchored(AnchorLoc::UpperLeft)),
        fontsize: Some(FontSize::Relative(RelativeSize::Small)),
        ..LegendOptions::default()
    })?;
    right.legend(LegendOptions {
        loc: Some(LegendLoc::Anchored(AnchorLoc::LowerCenter)),
        ncols: 2,
        frameon: Some(false),
        ..LegendOptions::default()
    })?;
    right.set_facecolor(Color::from("0.95").resolve()?);
    left.suptitle("left").set_x(0.1).set_halign(HAlign::Left);
    Ok(())
}

pub(super) fn savefig_tight_pad_extra_text(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = t();
    ax.plot(&t, map(&t, f64::sin))?;
    ax.set_title("cropped to its content");
    fig.text(1.02, 0.5, "beyond the right edge")
        .set_rotation(90.0)
        .set_valign(VAlign::Center);
    fig.text(-0.3, 0.5, "excluded from the box")
        .set_valign(VAlign::Center)
        .set_in_layout(false);
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.3),
        right: Some(0.6),
        top: Some(0.7),
        bottom: Some(0.3),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}
