//! The first half of `annotations`: basic annotations, text placed
//! relative to an arrow patch, offsets from scattered points, boxes around
//! text, and arrows with connection styles and anchors (`annotations_1` to
//! `annotations_12`).

use plt::artist::ArtistHandle;
use plt::axes::ScatterOptions;
use plt::coords::Coords;
use plt::patches::{ArrowStyle, BoxShape, BoxStyle, CustomBox, FancyArrowPatch};
use plt::prelude::*;
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, HAlign, TextBoxProps, VAlign};
use plt::transforms::Point;
use plt_core::Path;

use crate::figures::cases::common::{
    arrow, cosine, local_max, pointing, styled_box, test_annotation,
};
use crate::figures::{CaseResult, FigureCase};

fn annotations_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    cosine(&ax, LineProps::default().linewidth(2.0))?;
    ax.annotate(
        "local max",
        (2.0, 1.0),
        pointing((0.01, 0.99), "axes fraction")?,
    )?
    .set_valign(VAlign::Top)
    .set_halign(HAlign::Left);
    ax.set_ylim(-2.0, 2.0)?;
    Ok(())
}

fn annotations_3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let arr = ax.add_patch(
        FancyArrowPatch::between((1.25, 1.5), (1.75, 1.5))
            .with_arrowstyle("->,head_width=.15".parse::<ArrowStyle>()?)
            .with_mutation_scale(20.0),
    );
    ax.annotate(
        "label",
        (0.5, 0.5),
        AnnotateOptions {
            xycoords: Coords::Artist(arr.id()),
            ..AnnotateOptions::default()
        },
    )?
    .set_halign(HAlign::Center)
    .set_valign(VAlign::Bottom);
    ax.set_xlim(1.0, 2.0)?.set_ylim(1.0, 2.0)?;
    Ok(())
}

fn annotations_5(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [1.0, 3.0, 5.0, 7.0, 9.0];
    let y = [2.0, 4.0, 6.0, 8.0, 10.0];
    ax.scatter_with(
        x,
        y,
        ScatterOptions {
            sizes: Some(vec![20.0]),
            ..ScatterOptions::default()
        },
    )?;
    for ((xi, yi), text) in x.into_iter().zip(y).zip(["A", "B", "C", "D", "E"]) {
        ax.annotate(
            text,
            (xi, yi),
            AnnotateOptions {
                xytext: Some(Point::new(1.5, 1.5).into()),
                textcoords: Some("offset points".parse()?),
                ..AnnotateOptions::default()
            },
        )?;
    }
    Ok(())
}

fn annotations_6(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.text(0.5, 0.5, "Direction")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_rotation(45.0)
        .set_fontsize(15.0)
        .set_bbox(styled_box(
            "rarrow,pad=0.3",
            PatchProps::default()
                .facecolor("lightblue")
                .edgecolor("steelblue")
                .linewidth(2.0),
        )?);
    Ok(())
}

/// The guide's custom box outline: the text box padded by `pad`, with a
/// point on its left side (`_angled_box`).
fn angled(x0: f64, y0: f64, width: f64, height: f64, pad: f64) -> Path {
    let (x0, y0) = (x0 - pad, y0 - pad);
    let (x1, y1) = (x0 + width + 2.0 * pad, y0 + height + 2.0 * pad);
    Path::closed(
        [
            (x0, y0),
            (x1, y0),
            (x1, y1),
            (x0, y1),
            (x0 - pad, f64::midpoint(y0, y1)),
            (x0, y0),
            (x0, y0),
        ]
        .map(Point::from)
        .to_vec(),
    )
}

/// The guide's custom box shape: `custom_box_style` with the default
/// padding of 0.3, `MyStyle` with its own.
const ANGLED: BoxShape = BoxShape::Custom(CustomBox::new("angled", angled));

/// A big turned `Test` in a translucent box of `style`.
fn test_box(fig: &Figure, style: BoxStyle) {
    fig.subplot()
        .text(0.5, 0.5, "Test")
        .set_fontsize(30.0)
        .set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_rotation(30.0)
        .set_bbox(Some(
            TextBoxProps::new(style).with_patch(PatchProps::default().alpha(0.2)),
        ));
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn annotations_7(fig: &Figure) -> CaseResult {
    test_box(fig, BoxStyle::new(ANGLED));
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn annotations_8(fig: &Figure) -> CaseResult {
    test_box(fig, BoxStyle::new(ANGLED).with_pad(0.5));
    Ok(())
}

fn annotations_9(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.annotate(
        "",
        (0.2, 0.2),
        AnnotateOptions {
            xytext: Some(Point::new(0.8, 0.8).into()),
            textcoords: Some(Coords::Data),
            arrowprops: Some(ArrowProps::Fancy(arrow("->", "arc3")?)),
            ..AnnotateOptions::default()
        },
    )?;
    Ok(())
}

fn annotations_10(fig: &Figure) -> CaseResult {
    test_annotation(&fig.subplot(), arrow("simple", "arc3,rad=-0.2")?, None)
}

/// A white `-|>` arrow along `connection`, starting at `relpos` of the
/// text box, under a white round4 box (`_round4_test`).
fn round4_test(ax: &Axes, connection: &str, relpos: (f64, f64)) -> CaseResult {
    let white = || PatchProps::default().facecolor("w");
    test_annotation(
        ax,
        FancyArrowProps {
            relpos,
            patch: white(),
            ..arrow("-|>", connection)?
        },
        styled_box("round4", white())?,
    )
}

/// The default `relpos` of annotation arrows: the centre of the text.
const CENTRE: (f64, f64) = (0.5, 0.5);

fn annotations_11(fig: &Figure) -> CaseResult {
    round4_test(&fig.subplot(), "arc3,rad=-0.2", CENTRE)
}

fn annotations_12(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    round4_test(&ax, "arc3,rad=0.2", (0.0, 0.0))?;
    round4_test(&ax, "arc3,rad=-0.2", (1.0, 0.0))
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_colors_text/annotations_1", local_max).sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_2", annotations_2)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_3", annotations_3)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_5", annotations_5)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_6", annotations_6)
        .sized(5.0, 5.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_7", annotations_7)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_8", annotations_8)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_9", annotations_9)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_10", annotations_10)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_11", annotations_11)
        .sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/annotations_12", annotations_12)
        .sized(3.0, 3.0, 100.0),
];
