//! Text box and background cases of `figures/cases_text.py`.

use pltrs::prelude::*;
use pltrs::text::{HAlign, LineSpacing, TextBoxProps, VAlign};

use super::{blank, styled_box};
use crate::figures::FigureCase;

/// `BOXSTYLES`: every box style.
const BOXSTYLES: [&str; 10] = [
    "square",
    "round",
    "round4",
    "sawtooth",
    "roundtooth",
    "circle",
    "ellipse",
    "darrow",
    "larrow",
    "rarrow",
];

/// `bbox=dict(**patch)` without a box style.
fn plain_box(patch: PatchProps) -> TextBoxProps {
    TextBoxProps {
        patch,
        ..TextBoxProps::default()
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("text/multialignment_linespacing", |fig| {
        let ax = blank(fig)?;
        let s = "short\na much longer line\nmid line";
        let unfilled = || Some(plain_box(PatchProps::default().facecolor("none")));
        ax.text(0.5, 5.0, s)
            .set_multialign(Some(HAlign::Left))
            .set_valign(VAlign::Center)
            .set_bbox(unfilled());
        ax.text(5.0, 5.0, s)
            .set_multialign(Some(HAlign::Center))
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center)
            .set_linespacing(LineSpacing::Factor(2.0))
            .set_bbox(unfilled());
        ax.text(9.5, 5.0, s)
            .set_multialign(Some(HAlign::Right))
            .set_halign(HAlign::Right)
            .set_valign(VAlign::Center)
            .set_linespacing(LineSpacing::Factor(0.9))
            .set_bbox(unfilled());
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("text/color_alpha_background", |fig| {
        let ax = blank(fig)?;
        ax.axhspan(4.0, 6.0).set_color("k");
        ax.text(0.5, 8.0, "red text")
            .set_color("red")
            .set_fontsize(14.0);
        ax.text(0.5, 4.6, "half transparent")
            .set_color("tab:blue")
            .set_alpha(Some(0.5))
            .set_fontsize(16.0);
        ax.text(5.5, 4.6, "opaque")
            .set_color("tab:blue")
            .set_fontsize(16.0);
        // matplotlib applies `backgroundcolor` before `fontsize`, so the
        // box padding follows the default size.
        ax.text(0.5, 1.5, "background colour")
            .set_backgroundcolor("yellow")
            .set_fontsize(12.0);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/bbox_boxstyles", |fig| {
        let ax = blank(fig)?;
        ax.set_axis_off();
        for (i, style) in BOXSTYLES.into_iter().enumerate() {
            let (x, y) = (2.0 + 6.0 * (i % 2) as f64, 9.0 - 2.0 * (i / 2) as f64);
            let patch = PatchProps::default()
                .facecolor("lightsteelblue")
                .edgecolor("navy");
            ax.text(x, y, style)
                .set_halign(HAlign::Center)
                .set_valign(VAlign::Center)
                .set_fontsize(11.0)
                .set_bbox(styled_box(style, patch)?);
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/bbox_pad_props", |fig| {
        let ax = blank(fig)?;
        let white = || PatchProps::default().facecolor("w");
        ax.text(1.0, 7.0, "pad 0.1")
            .set_bbox(styled_box("square,pad=0.1", white())?);
        ax.text(5.0, 7.0, "pad 1.0")
            .set_bbox(styled_box("square,pad=1.0", white())?);
        ax.text(1.0, 2.0, "round size").set_bbox(styled_box(
            "round,pad=0.6,rounding_size=0.8",
            PatchProps::default()
                .facecolor("gold")
                .edgecolor("k")
                .linewidth(2.0),
        )?);
        ax.text(5.0, 2.0, "alpha + dash").set_bbox(Some(plain_box(
            PatchProps::default()
                .facecolor("tab:red")
                .alpha(0.3)
                .linestyle(LineStyle::Dashed)
                .edgecolor("k"),
        )));
        ax.text(8.5, 5.0, "rotated")
            .set_rotation(30.0)
            .set_bbox(styled_box(
                "larrow",
                PatchProps::default().facecolor("0.9"),
            )?);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];
