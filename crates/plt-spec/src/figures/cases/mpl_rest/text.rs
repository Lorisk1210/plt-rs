//! The text cases: `test_ft2font.py` and `test_axis.py`.

use plt::axes::{TickLabelProps, TickParams};
use plt::colors::Color;
use plt::figure::Figure;
use plt::layout::LayoutEngine;
use plt::text::{HorizontalAlignment, VerticalAlignment};

use super::classic;
use crate::figures::{CaseResult, FigureCase};

/// `test_fallback_last_resort`: an emoji no bundled font has, drawn with
/// the Last Resort font's placeholder glyph.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn fallback_last_resort(fig: &Figure) -> CaseResult {
    fig.text(0.5, 0.5, "Hello 🙃 World!")
        .set_fontsize(24.0)
        .set_halign(HorizontalAlignment::Center)
        .set_valign(VerticalAlignment::Center);
    Ok(())
}

/// `test_tick_not_in_layout`: a constrained layout ignoring a long tick
/// label taken out of it (the labels are white).
fn tick_not_in_layout(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    ax.set_xticks([0.0, 1.0])
        .set_xticklabels(["short", "very long"])?;
    ax.tick_params(&TickParams {
        labelcolor: Some(Color::from("w")),
        ..TickParams::default()
    });
    fig.layout();
    let out_of_layout = TickLabelProps {
        in_layout: Some(false),
        ..TickLabelProps::default()
    };
    ax.xaxis().set_major_ticklabel_props(1, &out_of_layout);
    Ok(())
}

/// The group's text cases.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_rest/fallback_last_resort", fallback_last_resort).sized(3.0, 0.5, 100.0),
    classic("mpl_rest/tick_not_in_layout", tick_not_in_layout),
];
