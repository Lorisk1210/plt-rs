//! Hatching cases of `figures/cases_patches.py`.

use plt::patches::{Circle, Rectangle};
use plt::prelude::*;
use plt::text::HorizontalAlignment;

use super::canvas;
use crate::figures::FigureCase;

/// `HATCHES`: every hatch character.
const HATCHES: [&str; 10] = ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*"];

/// `HATCH_COMBOS`: repeated and combined hatch characters.
const HATCH_COMBOS: [&str; 18] = [
    "//", "///", "\\\\\\", "||", "--", "++", "xx", "oo", "OO", "..", "**", "/o", "\\|", "|*",
    "-\\", "x*", "o-", "O.",
];

/// Python's `repr` of a hatch string: quoted, backslashes doubled.
fn repr(hatch: &str) -> String {
    format!("'{}'", hatch.replace('\\', "\\\\"))
}

/// A grid of hatched squares labelled with their pattern.
fn hatch_grid(
    ax: &Axes,
    hatches: &[&str],
    columns: usize,
    (top, height, label_dy, fontsize): (f64, f64, f64, f64),
    props: &PatchProps,
) {
    for (i, hatch) in hatches.iter().enumerate() {
        let (x, y) = (
            0.2 + 2.0 * (i % columns) as f64,
            top - 2.0 * (i / columns) as f64,
        );
        ax.add_patch(Rectangle::new((x, y), 1.6, height))
            .update_props(props.clone().hatch(*hatch));
        ax.text(x + 0.8, y + label_dy, repr(hatch))
            .set_halign(HorizontalAlignment::Center)
            .set_fontsize(fontsize);
    }
}

/// The three bars of `hatch_edge_and_hatchcolor`.
fn edge_and_hatch_colors(ax: &Axes) {
    ax.add_patch(Rectangle::new((0.5, 1.0), 2.5, 8.0))
        .update_props(
            PatchProps::default()
                .hatch("//")
                .facecolor("w")
                .edgecolor("tab:blue"),
        );
    ax.add_patch(Rectangle::new((3.75, 1.0), 2.5, 8.0))
        .update_props(
            PatchProps::default()
                .hatch("\\\\")
                .facecolor("0.9")
                .edgecolor("k")
                .hatchcolor("tab:green"),
        );
    ax.add_patch(Rectangle::new((7.0, 1.0), 2.5, 8.0))
        .update_props(
            PatchProps::default()
                .hatch("+")
                .facecolor("w")
                .edgecolor("k")
                .linewidth(1.0),
        )
        .set_hatch_linewidth(3.0);
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("patches/hatches_basic", |fig| {
        let ax = canvas(fig, (0.0, 10.0), (0.0, 4.0), false);
        let unfilled = PatchProps::default().fill(false);
        hatch_grid(&ax, &HATCHES, 5, (2.2, 1.6, 1.65, 8.0), &unfilled);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("patches/hatches_density_combos", |fig| {
        let ax = canvas(fig, (0.0, 12.0), (0.0, 6.0), false);
        let props = PatchProps::default()
            .facecolor("lightyellow")
            .edgecolor("k");
        hatch_grid(&ax, &HATCH_COMBOS, 6, (4.1, 1.5, 1.55, 7.0), &props);
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("patches/hatch_linewidth_color_rc", |fig| {
        let ax = canvas(fig, (0.0, 10.0), (0.0, 10.0), false);
        ax.add_patch(Rectangle::new((0.5, 1.0), 4.0, 8.0))
            .update_props(
                PatchProps::default()
                    .hatch("/")
                    .facecolor("w")
                    .edgecolor("k"),
            );
        ax.add_patch(Circle::new((7.5, 5.0), 2.5))
            .update_props(PatchProps::default().hatch("x").fill(false));
        Ok(())
    })
    .sized(5.0, 3.0, 100.0)
    .with_rc(&[("hatch.linewidth", "2.5"), ("hatch.color", "tab:red")]),
    FigureCase::new("patches/hatch_edge_and_hatchcolor", |fig| {
        let ax = canvas(fig, (0.0, 10.0), (0.0, 10.0), false);
        edge_and_hatch_colors(&ax);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("patches/hatched_bars_alpha", |fig| {
        let options = BarOptions {
            hatch: ["/", "o", "x"].map(String::from).to_vec(),
            color: vec!["tab:blue".into()],
            edgecolor: vec!["k".into()],
            props: PatchProps::default().alpha(0.5),
            ..BarOptions::default()
        };
        fig.subplot()
            .bar([1.0, 2.0, 3.0], [3.0, 1.0, 2.0], options)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
];
