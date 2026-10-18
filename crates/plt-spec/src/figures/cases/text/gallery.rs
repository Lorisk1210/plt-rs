//! `figures/cases_text_gallery.py`: alignment against a box, text rotated
//! with its transform, arrow-shaped boxes, the anatomy of an annotation
//! arrow, simple annotations with `relpos`, rotated multi-line boxes and
//! annotations of other annotations.

use plt::artist::ArtistHandle;
use plt::axes::{ScatterColors, ScatterOptions};
use plt::coords::{CoordUnit, Coords};
use plt::patches::{ArrowStyle, BoxStyle, ConnectionStyle, Ellipse, Rectangle};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{
    AnnotateOptions, ArrowProps, FancyArrowProps, HorizontalAlignment as HAlign, LineSpacing,
    TextBoxProps, TextRotationMode as RotationMode, VerticalAlignment as VAlign,
};
use plt::transforms::Point;

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `bbox=dict(boxstyle=style, **patch)`.
fn styled_box(
    style: &str,
    patch: PatchProps,
) -> Result<Option<TextBoxProps>, Box<dyn std::error::Error>> {
    Ok(Some(
        TextBoxProps::new(style.parse::<BoxStyle>()?).with_patch(patch),
    ))
}

/// `arrowprops=dict(arrowstyle=style, connectionstyle=connection)`.
fn arrow(style: &str, connection: &str) -> Result<FancyArrowProps, Box<dyn std::error::Error>> {
    Ok(FancyArrowProps {
        connectionstyle: connection.parse::<ConnectionStyle>()?,
        ..FancyArrowProps::new(style.parse::<ArrowStyle>()?)
    })
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn alignment_against_box(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    let (left, width, bottom, height) = (0.25, 0.5, 0.25, 0.5);
    let (right, top) = (left + width, bottom + height);
    ax.add_patch(Rectangle::new((left, bottom), width, height))
        .set_fill(false)
        .set_transform(ax.trans_axes())
        .set_clip_on(false);
    let middle = f64::midpoint(bottom, top);
    let texts = [
        (left, bottom, "left top", HAlign::Left, VAlign::Top, 0.0),
        (
            left,
            bottom,
            "left bottom",
            HAlign::Left,
            VAlign::Bottom,
            0.0,
        ),
        (
            right,
            top,
            "right bottom",
            HAlign::Right,
            VAlign::Bottom,
            0.0,
        ),
        (right, top, "right top", HAlign::Right, VAlign::Top, 0.0),
        (
            right,
            bottom,
            "center top",
            HAlign::Center,
            VAlign::Top,
            0.0,
        ),
        (
            left,
            middle,
            "right center",
            HAlign::Right,
            VAlign::Center,
            90.0,
        ),
        (
            left,
            middle,
            "left center",
            HAlign::Left,
            VAlign::Center,
            90.0,
        ),
    ];
    for (x, y, label, halign, valign, rotation) in texts {
        ax.text(x, y, label)
            .set_halign(halign)
            .set_valign(valign)
            .set_rotation(rotation)
            .set_transform(ax.trans_axes());
    }
    ax.text(f64::midpoint(left, right), middle, "middle")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_fontsize(20.0)
        .set_color(Color::from("red"))
        .set_transform(ax.trans_axes());
    ax.text(right, middle, "centered")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_rotation(90.0)
        .set_transform(ax.trans_axes());
    ax.text(left, top, "rotated\nwith newlines")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_rotation(45.0)
        .set_transform(ax.trans_axes());
    ax.set_axis_off();
    Ok(())
}

fn rotation_relative_to_line(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ramp: Vec<f64> = (0..8).map(f64::from).collect();
    ax.plot(&ramp, &ramp)?;
    ax.set_xlim(-10.0, 10.0);
    ax.text(-8.0, 0.0, "text 45° in screen coordinates")
        .set_fontsize(14.0)
        .set_rotation(45.0)
        .set_rotation_mode(RotationMode::Anchor);
    ax.text(0.0, 0.0, "text 45° in data coordinates")
        .set_fontsize(14.0)
        .set_rotation(45.0)
        .set_rotation_mode(RotationMode::Anchor)
        .set_transform_rotates_text(true);
    ax.text(-9.0, 6.0, "-30° in data")
        .set_fontsize(10.0)
        .set_rotation(-30.0)
        .set_transform_rotates_text(true)
        .set_bbox(Some(TextBoxProps {
            patch: PatchProps::default().facecolor("w").edgecolor("tab:blue"),
            ..TextBoxProps::default()
        }));
    Ok(())
}

fn arrow_boxes_rotated(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Ratio(1.0))?;
    let mut rng = SpecRng::new(2);
    let mut cloud = |shift: f64| {
        let x: Vec<f64> = rng
            .normal(100, 0.0, 1.0)
            .iter()
            .map(|v| shift + v)
            .collect();
        let y: Vec<f64> = rng
            .normal(100, 0.0, 1.0)
            .iter()
            .map(|v| shift + v)
            .collect();
        (x, y)
    };
    for (shift, color) in [(-1.0, "r"), (1.0, "g")] {
        let (x, y) = cloud(shift);
        ax.scatter_with(
            x,
            y,
            ScatterOptions {
                colors: ScatterColors::Single(Color::from(color)),
                ..ScatterOptions::default()
            },
        )?;
    }
    let sample_box = || {
        let mut patch = PatchProps::default().facecolor("w").edgecolor("0.5");
        patch.base.alpha = Some(0.9);
        styled_box("round", patch)
    };
    for (xy, label) in [(-2.0, "Sample A"), (2.0, "Sample B")] {
        ax.text(xy, xy, label)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center)
            .set_fontsize(20.0)
            .set_bbox(sample_box()?);
    }
    let direction = ax.text(0.0, 0.0, "Direction");
    direction
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_rotation(45.0)
        .set_fontsize(15.0);
    let arrow_patch = PatchProps::default()
        .facecolor((0.8, 0.9, 0.9))
        .edgecolor("b")
        .linewidth(2.0);
    direction.set_bbox(styled_box("rarrow", arrow_patch.clone())?);
    // `t.get_bbox_patch().set_boxstyle("rarrow", pad=0.6)`.
    direction.set_bbox(styled_box("rarrow,pad=0.6", arrow_patch)?);
    ax.text(2.5, -2.5, "Back")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_rotation(-30.0)
        .set_fontsize(12.0)
        .set_bbox(styled_box(
            "larrow,pad=0.4",
            PatchProps::default()
                .facecolor("lightyellow")
                .edgecolor("k"),
        )?);
    ax.text(-2.5, 2.5, "Both")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_rotation(90.0)
        .set_fontsize(12.0)
        .set_bbox(styled_box(
            "darrow,pad=0.3",
            PatchProps::default()
                .facecolor("mistyrose")
                .edgecolor("tab:red")
                .linewidth(1.5),
        )?);
    ax.set_xlim(-4.0, 4.0).set_ylim(-4.0, 4.0);
    Ok(())
}

fn annotation_arrow_anatomy(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let (x1, y1, x2, y2) = (0.3, 0.3, 0.7, 0.7);
    let panels = [
        ("connect", "-", false, 0.0),
        ("clip", "-", true, 0.0),
        ("shrink", "-", true, 5.0),
        ("mutate", "fancy", true, 5.0),
    ];
    for (ax, (label, style, clip, shrink)) in axs.as_slice().iter().zip(panels) {
        ax.plot_fmt([x1, x2], [y1, y2], ".")?;
        let ellipse = ax.add_artist(Ellipse {
            angle: 30.0,
            ..Ellipse::new((x1, y1), 0.3, 0.4)
        });
        ellipse.set_alpha(Some(0.2));
        let props = FancyArrowProps {
            shrink_b: shrink,
            patch_b: clip.then(|| ellipse.id()),
            patch: PatchProps::default().color("0.5"),
            ..arrow(style, "arc3,rad=0.3")?
        };
        ax.annotate(
            "",
            (x1, y1),
            AnnotateOptions {
                xytext: Some(Point::new(x2, y2)),
                textcoords: Some(Coords::Data),
                arrowprops: Some(ArrowProps::Fancy(props)),
                ..AnnotateOptions::default()
            },
        )?;
        ax.text(0.05, 0.95, label)
            .set_transform(ax.trans_axes())
            .set_halign(HAlign::Left)
            .set_valign(VAlign::Top);
        ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
        ax.set_xticks(Vec::<f64>::new())
            .set_yticks(Vec::<f64>::new());
        ax.set_aspect(Aspect::Ratio(1.0))?;
    }
    Ok(())
}

/// One `Test` annotation of `annotate_simple_relpos` from `(0.8, 0.8)` to
/// `(0.2, 0.2)`, size 20 and centred.
fn simple_test(ax: &Axes, props: FancyArrowProps, bbox: Option<TextBoxProps>) -> CaseResult {
    let note = ax.annotate(
        "Test",
        (0.2, 0.2),
        AnnotateOptions {
            xytext: Some(Point::new(0.8, 0.8)),
            textcoords: Some(Coords::Data),
            arrowprops: Some(ArrowProps::Fancy(props)),
            ..AnnotateOptions::default()
        },
    )?;
    note.set_fontsize(20.0)
        .set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_bbox(bbox);
    Ok(())
}

fn annotate_simple_relpos(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    for ax in axs.as_slice() {
        ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
        ax.set_xticks(Vec::<f64>::new())
            .set_yticks(Vec::<f64>::new());
    }
    simple_test(&axs[(0, 0)], arrow("simple", "arc3,rad=-0.2")?, None)?;
    let white = || PatchProps::default().facecolor("w");
    simple_test(
        &axs[(0, 1)],
        FancyArrowProps {
            patch: white(),
            ..arrow("-|>", "arc3,rad=-0.2")?
        },
        styled_box("round4", white())?,
    )?;
    for (relpos, connection) in [((0.0, 0.0), "arc3,rad=0.2"), ((1.0, 0.0), "arc3,rad=-0.2")] {
        simple_test(
            &axs[(0, 2)],
            FancyArrowProps {
                relpos,
                patch: white(),
                ..arrow("-|>", connection)?
            },
            styled_box("round", white())?,
        )?;
    }
    Ok(())
}

fn rotated_multiline_boxes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 6.0);
    let text = "first line\nsecond, longer line\n3";
    let variants = [
        (1.5, 20.0, RotationMode::Default, HAlign::Left),
        (5.0, -35.0, RotationMode::Anchor, HAlign::Center),
        (8.5, 90.0, RotationMode::Default, HAlign::Right),
    ];
    for (x, rotation, mode, align) in variants {
        ax.plot_with([x], [3.0], LineProps::fmt("k+")?.markersize(12.0))?;
        ax.text(x, 3.0, text)
            .set_rotation(rotation)
            .set_rotation_mode(mode)
            .set_multialign(Some(align))
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center)
            .set_linespacing(LineSpacing::Factor(1.5))
            .set_bbox(styled_box(
                "round,pad=0.4",
                PatchProps::default()
                    .facecolor("lavender")
                    .edgecolor("slateblue"),
            )?);
    }
    Ok(())
}

fn annotate_annotation_chain(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 6.0);
    let round = || styled_box("round", PatchProps::default().facecolor("w"));
    let an1 = ax.annotate("Test 1", (0.5, 0.5), AnnotateOptions::default())?;
    an1.set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_bbox(round()?);
    let an2 = ax.annotate(
        "Test 2",
        (1.0, 0.5),
        AnnotateOptions {
            xycoords: Coords::Artist(an1.id()),
            xytext: Some(Point::new(30.0, 0.0)),
            textcoords: Some("offset points".parse()?),
            arrowprops: Some(ArrowProps::Fancy(FancyArrowProps::new("->".parse()?))),
            ..AnnotateOptions::default()
        },
    )?;
    an2.set_valign(VAlign::Center)
        .set_halign(HAlign::Left)
        .set_bbox(round()?);
    let an3 = ax.annotate(
        "Test 3",
        (0.5, 1.0),
        AnnotateOptions {
            xycoords: Coords::Artist(an2.id()),
            xytext: Some(Point::new(0.5, 1.1)),
            textcoords: Some(Coords::blended(
                Coords::Artist(an1.id()),
                "axes fraction".parse()?,
            )),
            arrowprops: Some(ArrowProps::Fancy(FancyArrowProps {
                patch_b: Some(an2.id()),
                ..arrow("->", "arc3,rad=0.2")?
            })),
            ..AnnotateOptions::default()
        },
    )?;
    an3.set_valign(VAlign::Bottom)
        .set_halign(HAlign::Center)
        .set_bbox(styled_box(
            "sawtooth",
            PatchProps::default().facecolor("w"),
        )?);
    let an4 = ax.annotate(
        "Test 4",
        (6.0, 2.0),
        AnnotateOptions {
            xytext: Some(Point::new(8.0, 5.0)),
            arrowprops: Some(ArrowProps::Fancy(FancyArrowProps {
                patch: PatchProps::default()
                    .facecolor("tab:orange")
                    .edgecolor("none"),
                ..arrow("wedge,tail_width=0.7", "arc3,rad=-0.3")?
            })),
            ..AnnotateOptions::default()
        },
    )?;
    an4.set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_bbox(styled_box(
            "circle,pad=0.5",
            PatchProps::default().facecolor("0.9"),
        )?);
    ax.annotate(
        "from 4",
        (0.0, 0.5),
        AnnotateOptions {
            xycoords: Coords::Artist(an4.id()),
            xytext: Some(Point::new(-60.0, -40.0)),
            textcoords: Some("offset points".parse()?),
            arrowprops: Some(ArrowProps::Fancy(FancyArrowProps {
                shrink_a: 0.0,
                shrink_b: 0.0,
                ..FancyArrowProps::new("<|-".parse()?)
            })),
            ..AnnotateOptions::default()
        },
    )?
    .set_halign(HAlign::Right);
    Ok(())
}

/// After the gallery's `annotate_simple_coord03` (`text.OffsetFrom`).
fn annotate_offset_from(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let round = || styled_box("round", PatchProps::default().facecolor("w"));
    let an1 = ax.annotate("Test 1", (0.5, 0.5), AnnotateOptions::default())?;
    an1.set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_bbox(round()?);
    let offset_from = |(x, y): (f64, f64), unit: CoordUnit| Coords::OffsetFrom {
        artist: an1.id(),
        reference: Point::new(x, y),
        unit,
    };
    let an2 = ax.annotate(
        "Test 2",
        (0.1, 0.1),
        AnnotateOptions {
            xytext: Some(Point::new(0.0, -10.0)),
            textcoords: Some(offset_from((0.5, 0.0), CoordUnit::Points)),
            arrowprops: Some(ArrowProps::Fancy(FancyArrowProps::new("->".parse()?))),
            ..AnnotateOptions::default()
        },
    )?;
    an2.set_valign(VAlign::Top)
        .set_halign(HAlign::Center)
        .set_bbox(round()?);
    let an3 = ax.annotate(
        "Test 3",
        (0.9, 0.8),
        AnnotateOptions {
            xytext: Some(Point::new(5.0, 5.0)),
            textcoords: Some(offset_from((1.0, 1.0), CoordUnit::Pixels)),
            arrowprops: Some(ArrowProps::Fancy(FancyArrowProps::new("->".parse()?))),
            ..AnnotateOptions::default()
        },
    )?;
    an3.set_valign(VAlign::Bottom).set_halign(HAlign::Left);
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("text/alignment_against_box", alignment_against_box).sized(6.0, 4.5, 100.0),
    FigureCase::new("text/rotation_relative_to_line", rotation_relative_to_line)
        .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/arrow_boxes_rotated", arrow_boxes_rotated).sized(5.0, 5.0, 100.0),
    FigureCase::new("text/annotation_arrow_anatomy", annotation_arrow_anatomy)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("text/annotate_simple_relpos", annotate_simple_relpos).sized(6.0, 3.0, 100.0),
    FigureCase::new("text/rotated_multiline_boxes", rotated_multiline_boxes).sized(6.0, 4.0, 100.0),
    FigureCase::new("text/annotate_annotation_chain", annotate_annotation_chain)
        .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/annotate_offset_from", annotate_offset_from).sized(4.0, 3.0, 100.0),
];
