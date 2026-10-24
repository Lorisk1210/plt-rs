//! The `figure.*` and `savefig.*` cases.

use plt::colors::Color;
use plt::figure::{Figure, SubfigureOptions};

use super::{LABELLED, PANELS, SMALL, curve, map, rc_cases, t};
use crate::figures::{CaseResult, FigureCase};

/// `np.sin(T + shift)`.
fn shifted_sine(shift: f64) -> Vec<f64> {
    map(&t(), |x| (x + shift).sin())
}

/// `_supertitles`: two panels under a figure title and figure axis labels.
fn supertitles(fig: &Figure) -> CaseResult {
    for (i, ax) in fig.subplots(1, 2)?.as_slice().iter().enumerate() {
        ax.plot(t(), shifted_sine(i as f64))?;
    }
    fig.suptitle("Suptitle");
    fig.supxlabel("figure x");
    fig.supylabel("figure y");
    Ok(())
}

/// `_panels`: four titled and labelled panels.
fn panels(fig: &Figure) -> CaseResult {
    for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
        ax.plot(t(), shifted_sine(i as f64))?;
        ax.set_title(format!("panel {i}"));
        ax.set_xlabel("x");
    }
    Ok(())
}

/// `_subfigures`: two coloured subfigures, each with a curve.
fn subfigures(fig: &Figure) -> CaseResult {
    let subfigs = fig.subfigures(1, 2, SubfigureOptions::default())?;
    for (i, (sub, color)) in subfigs
        .as_slice()
        .iter()
        .zip(["lightsteelblue", "wheat"])
        .enumerate()
    {
        sub.set_facecolor(Color::from(color).resolve()?);
        sub.subplot().plot(t(), shifted_sine(i as f64))?;
    }
    Ok(())
}

/// `_framed`: a curve in a figure with a thick border.
fn framed(fig: &Figure) -> CaseResult {
    fig.set_linewidth(8.0);
    curve(fig)
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    supertitles, LABELLED => {
        "figure_title_label_style": [
            ("figure.titlesize", "small"),
            ("figure.titleweight", "bold"),
            ("figure.labelsize", "x-small"),
            ("figure.labelweight", "bold"),
        ],
    }
    curve, SMALL => {
        "figure_facecolor": [("figure.facecolor", "lightsteelblue")],
        "savefig_facecolor": [("savefig.facecolor", "lightgreen"), ("savefig.dpi", "50")],
        "savefig_transparent": [("savefig.transparent", "True")],
        "savefig_bbox": [("savefig.bbox", "tight")],
        "savefig_pad_inches": [("savefig.bbox", "tight"), ("savefig.pad_inches", "0.4")],
    }
    subfigures, SMALL => {
        "figure_frameon": [("figure.frameon", "False")],
    }
    framed, SMALL => {
        "figure_edgecolor": [("figure.edgecolor", "tab:red")],
        "savefig_edgecolor": [("savefig.edgecolor", "tab:blue")],
    }
    panels, PANELS => {
        "figure_autolayout": [("figure.autolayout", "True")],
        "figure_subplot": [
            ("figure.subplot.left", "0.2"),
            ("figure.subplot.right", "0.95"),
            ("figure.subplot.bottom", "0.15"),
            ("figure.subplot.top", "0.9"),
            ("figure.subplot.wspace", "0.5"),
            ("figure.subplot.hspace", "0.8"),
        ],
        "figure_constrained_layout": [("figure.constrained_layout.use", "True")],
        "figure_constrained_layout_pads": [
            ("figure.constrained_layout.use", "True"),
            ("figure.constrained_layout.h_pad", "0.15"),
            ("figure.constrained_layout.w_pad", "0.15"),
        ],
        "figure_constrained_layout_spaces": [
            ("figure.constrained_layout.use", "True"),
            ("figure.constrained_layout.hspace", "0.3"),
            ("figure.constrained_layout.wspace", "0.3"),
        ],
    }
};
