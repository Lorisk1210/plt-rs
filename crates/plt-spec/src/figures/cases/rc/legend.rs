//! The `legend.*` cases.

use plt::axes::{FillBetweenOptions, ScatterOptions};
use plt::collections::CollectionProps;
use plt::legend::LegendOptions;
use plt::prelude::*;

use super::{LABELLED, map, rc_cases, t};
use crate::figures::{CaseResult, FigureCase};

/// `_legend`: a titled legend of a marked line, a line, a scatter and a
/// band.
fn legend(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = t();
    let marked = LineProps::default()
        .marker(Marker::from(MarkerSymbol::Circle))
        .markersize(4.0)
        .label("sine");
    ax.plot_with(&t, map(&t, f64::sin), marked)?;
    ax.plot_with(
        &t,
        map(&t, |x| 0.5 * x.cos()),
        LineProps::default().label("cosine"),
    )?;
    let every: Vec<f64> = t.iter().step_by(4).copied().collect();
    ax.scatter_with(
        &every,
        map(&every, |x| 0.3 * x.sin() - 0.4),
        ScatterOptions::default(),
    )?
    .set_label("points");
    let band = FillBetweenOptions {
        props: CollectionProps::default().alpha(0.4),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&t, [-1.1], [-0.8], band)?.set_label("band");
    ax.legend(LegendOptions {
        title: Some("key".into()),
        ..LegendOptions::default()
    })?;
    Ok(())
}

/// `_legend_columns`: a two-column legend.
fn legend_columns(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = t();
    for i in 0..4 {
        let shift = f64::from(i);
        let props = LineProps::default().label(format!("c{i}"));
        ax.plot_with(&t, map(&t, |x| (x + shift).sin()), props)?;
    }
    ax.legend(LegendOptions {
        ncols: 2,
        ..LegendOptions::default()
    })?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    legend, LABELLED => {
        "legend_loc": [("legend.loc", "lower left")],
        "legend_frameon": [("legend.frameon", "False")],
        "legend_frame_style": [
            ("legend.framealpha", "1"),
            ("legend.facecolor", "lightyellow"),
            ("legend.edgecolor", "tab:red"),
            ("legend.linewidth", "2"),
        ],
        "legend_box_shape": [("legend.fancybox", "False"), ("legend.shadow", "True")],
        "legend_points": [("legend.numpoints", "3"), ("legend.scatterpoints", "4")],
        "legend_markerscale": [("legend.markerscale", "2")],
        "legend_fontsizes": [("legend.fontsize", "x-small"), ("legend.title_fontsize", "large")],
        "legend_labelcolor": [("legend.labelcolor", "linecolor")],
        "legend_paddings": [("legend.borderpad", "1.5"), ("legend.labelspacing", "1.2")],
        "legend_handle_spacing": [("legend.handletextpad", "2"), ("legend.borderaxespad", "2")],
        "legend_handle_size": [("legend.handlelength", "4"), ("legend.handleheight", "2")],
    }
    legend_columns, LABELLED => {
        "legend_columnspacing": [("legend.columnspacing", "5")],
    }
};
