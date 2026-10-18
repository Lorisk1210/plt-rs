//! Bbox connectors and host and parasite axes of
//! `figures/cases_toolkits.py`.

use std::f64::consts::{PI, TAU};

use plt::axes::Axes;
use plt::axes_grid1::{
    BboxConnector, BboxConnectorPatch, BboxPatch, BboxSource, Corner, ParasiteLink, host_axes,
    host_subplot,
};
use plt::axisartist::Toggle;
use plt::figure::Figure;
use plt::layout::TightLayout;
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::numeric::linspace;
use plt::patches::PatchProps;
use plt::prelude::ArtistHandle;
use plt::spines::SpineSide;
use plt_core::{Affine2D, Bbox};

use crate::figures::CaseResult;

/// `x` and `f(x)` for `x` in `linspace(start, stop, n)`.
fn curve(start: f64, stop: f64, n: usize, f: impl Fn(f64) -> f64) -> (Vec<f64>, Vec<f64>) {
    let x = linspace(start, stop, n);
    let y = x.iter().map(|&v| f(v)).collect();
    (x, y)
}

/// A translucent face without an edge (`fc=color, ec="none", alpha=alpha`).
fn shade(color: &str, alpha: f64) -> PatchProps {
    PatchProps::default()
        .facecolor(color)
        .edgecolor("none")
        .alpha(alpha)
}

/// The quadrilateral joining the lower edge of `bbox1` to the upper edge
/// of `bbox2` (`loc1a=3, loc2a=2, loc1b=4, loc2b=1`).
fn zoom_band(bbox1: BboxSource, bbox2: BboxSource) -> BboxConnectorPatch {
    BboxConnectorPatch {
        bbox1,
        bbox2,
        loc1a: Corner::LowerLeft,
        loc2a: Corner::UpperLeft,
        loc1b: Corner::LowerRight,
        loc2b: Corner::UpperRight,
    }
}

pub(super) fn bbox_connector_patch(fig: &Figure) -> CaseResult {
    let ax1 = fig.add_subplot(2, 2, 1)?;
    let ax2 = fig.add_subplot(2, 2, 2)?;
    let main = fig.add_subplot(2, 1, 2)?;
    let (x, y) = curve(0.0, 5.0, 100, |v| (2.0 * v).sin());
    for ax in [&ax1, &ax2, &main] {
        ax.plot(x.clone(), y.clone())?;
    }
    main.set_xlim(0.0, 5.0);
    ax1.set_xlim(1.0, 2.0);
    ax2.set_xlim(3.0, 4.0);
    let span = Bbox::from_extents(1.0, 0.0, 2.0, 1.0);
    let upper = BboxSource::transformed(span, ax1.trans_xaxis());
    let lower = BboxSource::transformed(span, main.trans_xaxis());
    ax1.add_patch(BboxPatch::new(upper.clone()))
        .update_props(shade("tab:blue", 0.2));
    main.add_patch(BboxPatch::new(lower.clone()))
        .update_props(shade("tab:blue", 0.2));
    for (loc1, loc2) in [
        (Corner::LowerLeft, Corner::UpperLeft),
        (Corner::LowerRight, Corner::UpperRight),
    ] {
        main.add_patch(BboxConnector::new(
            upper.clone(),
            lower.clone(),
            loc1,
            Some(loc2),
        ))
        .update_props(PatchProps::default().edgecolor("tab:blue"))
        .set_clip_on(false);
    }
    main.add_patch(zoom_band(upper, lower))
        .update_props(shade("tab:blue", 0.2).fill(true))
        .set_clip_on(false);
    let second =
        BboxSource::transformed(Bbox::from_extents(3.0, 0.0, 4.0, 1.0), main.trans_xaxis());
    main.add_patch(BboxPatch::new(second.clone()))
        .update_props(shade("tab:green", 0.2));
    let band = PatchProps::default()
        .facecolor("tab:green")
        .edgecolor("tab:green")
        .alpha(0.3)
        .fill(true);
    main.add_patch(zoom_band(BboxSource::axes(&ax2), second))
        .update_props(band)
        .set_clip_on(false);
    Ok(())
}

/// Plots `(x, y)` labelled `label`, returning the line's colour.
fn labelled(
    ax: &Axes,
    x: &[f64],
    y: &[f64],
    label: &str,
) -> Result<plt_core::Rgba, Box<dyn std::error::Error>> {
    let line = ax.plot(x.to_vec(), y.to_vec())?;
    line.set_label(label);
    Ok(line.rgba()?)
}

pub(super) fn twinx_twiny_legend(fig: &Figure) -> CaseResult {
    let host = host_subplot(fig, 1, 1, 1)?;
    let par1 = host.twinx();
    let par2 = host.twiny();
    host.set_xlabel("Distance");
    host.set_ylabel("Density");
    par1.set_ylabel("Temperature");
    par2.set_xlabel("Time");
    let c1 = labelled(&host, &[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0], "Density")?;
    let c2 = labelled(&par1, &[0.0, 1.0, 2.0], &[0.0, 3.0, 2.0], "Temperature")?;
    let c3 = labelled(&par2, &[0.0, 50.0, 100.0], &[0.5, 1.5, 1.0], "Velocity")?;
    host.legend(
        LegendOptions::default()
            .labelcolor("linecolor")
            .loc("upper left"),
    )?;
    host.yaxis().set_label_color(c1);
    par1.yaxis().set_label_color(c2);
    par2.xaxis().set_label_color(c3);
    Ok(())
}

pub(super) fn twin_aux_tight(fig: &Figure) -> CaseResult {
    let left = host_subplot(fig, 1, 2, 1)?;
    let (xx, sin) = curve(0.0, TAU, 100, f64::sin);
    left.plot(xx.clone(), sin)?;
    let twin = left.twin(None)?;
    left.set_xlim(0.0, TAU).set_ylim(-1.1, 1.1);
    twin.set_xticks(vec![0.0, 0.5 * PI, PI, 1.5 * PI, TAU]);
    twin.set_xticklabels(["$0$", r"$\pi/2$", r"$\pi$", r"$3\pi/2$", r"$2\pi$"])?;
    twin.axis(SpineSide::Right).set_ticklabels_visible(false);
    twin.axis(SpineSide::Top).set_ticklabels_visible(true);
    let right = host_subplot(fig, 1, 2, 2)?;
    let degrees = right.twin(Some(Affine2D::scaling(PI / 180.0, 2.0)))?;
    let cos: Vec<f64> = xx.iter().map(|v| v.cos()).collect();
    right.plot_with(xx, cos, LineProps::default().color("tab:red"))?;
    right.set_xlim(0.0, TAU).set_ylim(-1.2, 1.2);
    right.set_xlabel("radians");
    degrees.set_xlabel("degrees");
    degrees.axis(SpineSide::Top).set_label_visible(true);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) fn aux_axes_best_legend(fig: &Figure) -> CaseResult {
    let host = host_axes(fig, [0.12, 0.14, 0.72, 0.8]);
    let par = host.get_aux_axes(ParasiteLink::Independent)?;
    par.share_x(&host)?;
    host.axis(SpineSide::Right).set_visible(false);
    let right = par.axis(SpineSide::Right);
    right.set_visible(true);
    right.set_ticklabels_visible(true);
    right.set_label_visible(true);
    par.axis(SpineSide::Left).toggle(&Toggle {
        all: Some(false),
        ..Toggle::default()
    });
    let (x, sin) = curve(0.0, 10.0, 50, f64::sin);
    let decay: Vec<f64> = x.iter().map(|v| 100.0 * (-v / 3.0).exp()).collect();
    labelled(&host, &x, &sin, "sin")?;
    labelled(&par, &x, &decay, "decay")?;
    par.set_ylabel("decay");
    host.set_xlabel("time");
    host.legend(LegendOptions::default())?;
    Ok(())
}
