//! matplotlib's "Annotations" tutorial in pltrs (Rust): annotating a
//! point with an arrow, the coordinate systems of the point and of the
//! text, arrow and connection styles, text in boxes, and anchored
//! artists.
//!
//! ```text
//! cargo run -p pltrs-examples --example annotation_guide   # writes annotation_*.png here
//! ```

use pltrs::layout::LayoutEngine;
use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
use pltrs::patches::BoxStyle;
use pltrs::prelude::*;
use pltrs::text::{
    AnnotateOptions, ArrowProps, FancyArrowProps, HAlign, TextBoxProps, TextProps, VAlign,
};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn main() -> Outcome {
    basic()?;
    coordinate_systems()?;
    arrow_styles()?;
    Ok(())
}

/// `ax.annotate("local max", xy=(2, 1), xytext=(3, 1.5),
/// arrowprops=dict(facecolor="black", shrink=0.05))`.
fn basic() -> Outcome {
    let fig = Figure::with_size(5.0, 3.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let t = arange(0.0, 5.0, 0.01);
    ax.plot_with(
        &t,
        t.iter().map(|t| (std::f64::consts::TAU * t).cos()),
        LineProps::default().linewidth(2.0),
    )?;
    let arrow = FancyArrowProps {
        patch: PatchProps::default().facecolor("black"),
        shrink_a: 0.0,
        shrink_b: 5.0,
        ..FancyArrowProps::new("-|>".parse()?)
    };
    let options = AnnotateOptions {
        xytext: Some((3.0, 1.5).into()),
        arrowprops: Some(ArrowProps::Fancy(arrow)),
        ..AnnotateOptions::default()
    };
    ax.annotate("local max", (2.0, 1.0), options)?;
    ax.set_ylim(-2.0, 2.0)?;
    fig.savefig("annotation_basic.png")?;
    Ok(())
}

/// `xycoords` and `textcoords`: "data", "axes fraction", "figure
/// fraction" and "offset points".
fn coordinate_systems() -> Outcome {
    let fig = Figure::with_size(6.0, 4.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    ax.plot_fmt([0.2, 0.5, 0.8], [0.3, 0.7, 0.4], "o-")?;
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    let arrow = || {
        Some(ArrowProps::Fancy(FancyArrowProps::new(
            "->".parse().expect("an arrow style"),
        )))
    };
    let point = AnnotateOptions {
        xytext: Some((-60.0, 30.0).into()),
        textcoords: Some("offset points".parse()?),
        arrowprops: arrow(),
        ..AnnotateOptions::default()
    };
    ax.annotate("data point, text 60 pt left", (0.5, 0.7), point)?;
    let corner = AnnotateOptions {
        xycoords: "axes fraction".parse()?,
        ..AnnotateOptions::default()
    };
    ax.annotate("axes fraction (0.02, 0.95)", (0.02, 0.95), corner)?
        .set_valign(VAlign::Top);
    let figure_corner = AnnotateOptions {
        xycoords: "figure fraction".parse()?,
        ..AnnotateOptions::default()
    };
    ax.annotate("figure fraction (0.99, 0.01)", (0.99, 0.01), figure_corner)?
        .set_halign(HAlign::Right)
        .set_valign(VAlign::Bottom);
    let mixed = AnnotateOptions {
        xycoords: "data".parse()?,
        xytext: Some((0.6, 0.15).into()),
        textcoords: Some("axes fraction".parse()?),
        arrowprops: arrow(),
        ..AnnotateOptions::default()
    };
    ax.annotate("text in axes fraction", (0.8, 0.4), mixed)?;
    fig.savefig("annotation_coordinates.png")?;
    Ok(())
}

/// `arrowprops=dict(arrowstyle=..., connectionstyle=...)`, text in a
/// box (`bbox=dict(boxstyle="round")`) and `AnchoredText`.
fn arrow_styles() -> Outcome {
    let styles = ["->", "-|>", "<->", "fancy", "simple", "wedge", "]-[", "|-|"];
    let connections = [
        "arc3,rad=0.3",
        "angle3,angleA=90,angleB=0",
        "angle,angleA=-90,angleB=180,rad=5",
        "bar,fraction=0.3",
    ];
    let fig = Figure::with_size(8.0, 4.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let [left, right] = [fig.add_subplot(1, 2, 1)?, fig.add_subplot(1, 2, 2)?];
    for ax in [&left, &right] {
        ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
        ax.set_xticks(Vec::<f64>::new());
        ax.set_yticks(Vec::<f64>::new());
    }
    for (i, style) in styles.iter().enumerate() {
        let y = 0.92 - i as f64 * 0.12;
        let mut arrow = FancyArrowProps::new(style.parse()?);
        arrow.patch = PatchProps::default().facecolor("C0").edgecolor("C0");
        let options = AnnotateOptions {
            xytext: Some((0.1, y).into()),
            arrowprops: Some(ArrowProps::Fancy(arrow)),
            ..AnnotateOptions::default()
        };
        ax_text(&left, style, (0.75, y), options)?;
    }
    left.set_title("arrowstyle");
    for (i, connection) in connections.iter().enumerate() {
        let y = 0.85 - i as f64 * 0.22;
        let mut arrow = FancyArrowProps::new("->".parse()?);
        arrow.connectionstyle = connection.parse()?;
        let options = AnnotateOptions {
            xytext: Some((0.1, y).into()),
            arrowprops: Some(ArrowProps::Fancy(arrow)),
            ..AnnotateOptions::default()
        };
        // A bar connects points side by side; the others reach down.
        let drop = if connection.starts_with("bar") {
            0.0
        } else {
            0.12
        };
        let label = right.annotate(*connection, (0.8, y - drop), options)?;
        label.set_fontsize(8.0);
        label.set_bbox(Some(
            TextBoxProps::new("round,pad=0.3".parse::<BoxStyle>()?)
                .with_patch(PatchProps::default().facecolor("0.95")),
        ));
    }
    right.set_title("connectionstyle");
    // `ax.add_artist(AnchoredText("an AnchoredText", loc="lower right"))`
    let note = AnchoredBox::text(
        "an AnchoredText",
        TextProps::default(),
        AnchoredOptions::new(AnchorLoc::LowerRight),
    )?;
    right.add_anchored_box(note);
    fig.savefig("annotation_styles.png")?;
    Ok(())
}

/// An annotation of `ax` at `xy` labelled `label`, its text small.
fn ax_text(ax: &Axes, label: &str, xy: (f64, f64), options: AnnotateOptions) -> Outcome {
    ax.annotate(label, xy, options)?.set_fontsize(9.0);
    Ok(())
}
