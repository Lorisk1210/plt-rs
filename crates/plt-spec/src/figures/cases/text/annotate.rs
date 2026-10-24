//! Annotation cases of `figures/cases_text.py`: coordinate systems, arrow
//! and connection styles, shrinking and the legacy arrow.

use std::f64::consts::PI;

use plt::coords::Coords;
use plt::gridspec::SubplotOverrides;
use plt::patches::{BoxStyle, ConnectionStyle};
use plt::prelude::*;
use plt::text::{
    AnnotateOptions, ArrowProps, FancyArrowProps, HAlign, LegacyArrowProps, TextBoxProps, VAlign,
};
use plt::transforms::Point;

use super::blank;
use crate::figures::{CaseResult, FigureCase};

/// `LINE_ARROWSTYLES`: the arrow styles drawn as lines.
const LINE_ARROWSTYLES: [&str; 10] = [
    "-", "->", "<-", "<->", "-[", "]-", "]-[", "|-|", "]->", "<-[",
];

/// `FILLED_ARROWSTYLES`: the arrow styles with filled parts.
const FILLED_ARROWSTYLES: [&str; 6] = ["-|>", "<|-", "<|-|>", "simple", "fancy", "wedge"];

/// `dict(arrowstyle=style)` as annotation options.
fn arrowprops(style: &str) -> Result<Option<ArrowProps>, Box<dyn std::error::Error>> {
    Ok(Some(ArrowProps::Fancy(style.parse::<FancyArrowProps>()?)))
}

/// A coordinate system by its matplotlib name.
fn coords(name: &str) -> Result<Coords, Box<dyn std::error::Error>> {
    Ok(name.parse()?)
}

/// `bbox=dict(boxstyle="round", fc="w")`.
fn round_white_box() -> Result<Option<TextBoxProps>, Box<dyn std::error::Error>> {
    Ok(Some(
        TextBoxProps::new("round".parse::<BoxStyle>()?)
            .with_patch(PatchProps::default().facecolor("w")),
    ))
}

/// One annotation of `annotate_coord_kinds`.
struct Note {
    text: &'static str,
    xy: (f64, f64),
    xytext: Option<(f64, f64)>,
    xycoords: &'static str,
    textcoords: Option<&'static str>,
    arrow: bool,
}

/// The annotations of `annotate_coord_kinds`.
const COORD_NOTES: [Note; 9] = [
    Note {
        text: "data",
        xy: (0.5, 0.5),
        xytext: Some((-1.5, 1.5)),
        xycoords: "data",
        textcoords: None,
        arrow: true,
    },
    Note {
        text: "axes fraction",
        xy: (0.1, 0.1),
        xytext: Some((0.3, 0.3)),
        xycoords: "axes fraction",
        textcoords: Some("axes fraction"),
        arrow: true,
    },
    Note {
        text: "figure pixels",
        xy: (5.0, 5.0),
        xytext: None,
        xycoords: "figure pixels",
        textcoords: None,
        arrow: false,
    },
    Note {
        text: "figure fraction",
        xy: (0.7, 0.92),
        xytext: None,
        xycoords: "figure fraction",
        textcoords: None,
        arrow: false,
    },
    Note {
        text: "offset points",
        xy: (1.0, -1.0),
        xytext: Some((-30.0, -25.0)),
        xycoords: "data",
        textcoords: Some("offset points"),
        arrow: true,
    },
    Note {
        text: "offset pixels",
        xy: (1.5, 1.0),
        xytext: Some((-60.0, 20.0)),
        xycoords: "data",
        textcoords: Some("offset pixels"),
        arrow: true,
    },
    Note {
        text: "polar",
        xy: (PI / 4.0, 1.5),
        xytext: Some((-1.8, -0.5)),
        xycoords: "polar",
        textcoords: Some("data"),
        arrow: true,
    },
    Note {
        text: "polar text",
        xy: (-1.0, -1.5),
        xytext: Some((3.0 * PI / 4.0, 1.8)),
        xycoords: "data",
        textcoords: Some("polar"),
        arrow: true,
    },
    Note {
        text: "offset fontsize",
        xy: (0.0, 0.0),
        xytext: Some((2.0, -4.0)),
        xycoords: "data",
        textcoords: Some("offset fontsize"),
        arrow: true,
    },
];

/// Adds the annotations of `annotate_coord_kinds`.
fn coord_kinds(ax: &Axes) -> CaseResult {
    for note in &COORD_NOTES {
        ax.annotate(
            note.text,
            note.xy,
            AnnotateOptions {
                xytext: note.xytext.map(Point::from),
                xycoords: coords(note.xycoords)?,
                textcoords: note.textcoords.map(coords).transpose()?,
                arrowprops: if note.arrow { arrowprops("->")? } else { None },
                ..AnnotateOptions::default()
            },
        )?;
    }
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("text/annotate_coord_kinds", |fig| {
        let ax = fig.subplot();
        ax.set_xlim(-2.0, 2.0).set_ylim(-2.0, 2.0);
        coord_kinds(&ax)?;
        ax.plot_fmt([0.5, 1.0, 1.5, 0.0], [0.5, -1.0, 1.0, 0.0], "k.")?;
        Ok(())
    })
    .sized(6.0, 4.5, 100.0),
    FigureCase::new("text/annotate_axes_coords_and_artist", |fig| {
        let ax = fig.subplot();
        ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.0])?;
        let peak = ax.annotate(
            "peak",
            (1.0, 1.0),
            AnnotateOptions {
                xytext: Some(Point::new(0.2, 0.5)),
                textcoords: Some(coords("axes fraction")?),
                arrowprops: arrowprops("->")?,
                ..AnnotateOptions::default()
            },
        )?;
        peak.set_bbox(round_white_box()?);
        ax.annotate(
            "from text",
            (1.0, 0.0),
            AnnotateOptions {
                xytext: Some(Point::new(0.0, -40.0)),
                xycoords: Coords::Artist(peak.id()),
                textcoords: Some(coords("offset points")?),
                arrowprops: arrowprops("<-")?,
                ..AnnotateOptions::default()
            },
        )?
        .set_halign(HAlign::Left);
        ax.annotate(
            "outside xlim",
            (2.4, 0.5),
            AnnotateOptions {
                xytext: Some(Point::new(0.6, 0.85)),
                textcoords: Some(coords("axes fraction")?),
                arrowprops: arrowprops("->")?,
                annotation_clip: Some(false),
                ..AnnotateOptions::default()
            },
        )?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("text/arrowstyles_line", |fig| {
        let ax = blank(fig);
        ax.set_axis_off();
        for (i, style) in LINE_ARROWSTYLES.into_iter().enumerate() {
            let (x, y) = (0.5 + 5.0 * (i / 5) as f64, 9.0 - 2.0 * (i % 5) as f64);
            let props = FancyArrowProps {
                mutation_scale: Some(15.0),
                ..style.parse::<FancyArrowProps>()?
            };
            ax.annotate(
                style,
                (x + 3.0, y),
                AnnotateOptions {
                    xytext: Some(Point::new(x, y)),
                    arrowprops: Some(ArrowProps::Fancy(props)),
                    ..AnnotateOptions::default()
                },
            )?
            .set_valign(VAlign::Center);
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/arrowstyles_filled", |fig| {
        let ax = blank(fig);
        ax.set_axis_off();
        for (i, style) in FILLED_ARROWSTYLES.into_iter().enumerate() {
            let (x, y) = (0.5 + 5.0 * (i / 3) as f64, 8.5 - 3.0 * (i % 3) as f64);
            let props = FancyArrowProps {
                mutation_scale: Some(20.0),
                patch: PatchProps::default().facecolor("tab:orange").edgecolor("k"),
                ..style.parse::<FancyArrowProps>()?
            };
            ax.annotate(
                style,
                (x + 3.5, y),
                AnnotateOptions {
                    xytext: Some(Point::new(x, y)),
                    arrowprops: Some(ArrowProps::Fancy(props)),
                    ..AnnotateOptions::default()
                },
            )?
            .set_valign(VAlign::Center);
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/arrowstyle_params", |fig| {
        let ax = blank(fig);
        ax.set_axis_off();
        let styles = [
            "->,head_length=0.8,head_width=0.4",
            "-[,widthB=1.5,lengthB=0.5",
            "|-|,widthA=0.3,widthB=1.0",
            "-|>,head_length=1.0,head_width=0.2",
            "fancy,head_length=0.6,head_width=0.6,tail_width=0.3",
            "simple,head_length=0.3,head_width=0.8,tail_width=0.6",
            "wedge,tail_width=0.7,shrink_factor=0.3",
            "<->,head_length=0.2,head_width=0.6",
        ];
        for (i, style) in styles.into_iter().enumerate() {
            let (x, y) = (0.2 + 5.0 * (i / 4) as f64, 9.0 - 2.5 * (i % 4) as f64);
            let props = FancyArrowProps {
                mutation_scale: Some(15.0),
                patch: PatchProps::default().facecolor("w"),
                ..style.parse::<FancyArrowProps>()?
            };
            let name = style.split(',').next().unwrap_or_default();
            ax.annotate(
                name,
                (x + 4.0, y),
                AnnotateOptions {
                    xytext: Some(Point::new(x, y)),
                    arrowprops: Some(ArrowProps::Fancy(props)),
                    ..AnnotateOptions::default()
                },
            )?
            .set_valign(VAlign::Center);
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/connectionstyles", |fig| {
        let axs = fig.subplots(2, 4)?;
        fig.subplots_adjust(&SubplotOverrides {
            hspace: Some(0.6),
            wspace: Some(0.3),
            ..SubplotOverrides::default()
        })?;
        let styles = [
            "arc3",
            "arc3,rad=0.3",
            "arc3,rad=-0.3",
            "angle3,angleA=90,angleB=0",
            "angle,angleA=-90,angleB=180,rad=0",
            "angle,angleA=-90,angleB=180,rad=5",
            "arc,angleA=-90,angleB=0,armA=30,armB=30,rad=0",
            "bar,fraction=0.1",
        ];
        for (ax, style) in axs.as_slice().iter().zip(styles) {
            ax.plot_with(
                [0.2, 0.8],
                [0.2, 0.8],
                LineProps::fmt("ko")?.markersize(3.0),
            )?;
            let props = FancyArrowProps {
                connectionstyle: style.parse::<ConnectionStyle>()?,
                shrink_a: 4.0,
                shrink_b: 4.0,
                .."->".parse::<FancyArrowProps>()?
            };
            ax.annotate(
                "",
                (0.2, 0.2),
                AnnotateOptions {
                    xytext: Some(Point::new(0.8, 0.8)),
                    arrowprops: Some(ArrowProps::Fancy(props)),
                    ..AnnotateOptions::default()
                },
            )?;
            ax.set_title(style.replace(',', "\n")).set_fontsize(7.0);
            ax.set_xticks(Vec::<f64>::new());
            ax.set_yticks(Vec::<f64>::new());
        }
        Ok(())
    })
    .sized(8.0, 5.0, 100.0),
    FigureCase::new("text/shrink_and_patches", |fig| {
        let ax = blank(fig);
        for (i, (sa, sb)) in [(0.0, 0.0), (5.0, 5.0), (15.0, 25.0)]
            .into_iter()
            .enumerate()
        {
            let y = 8.0 - 3.0 * i as f64;
            ax.plot_fmt([1.0, 8.0], [y, y], "r.")?;
            let props = FancyArrowProps {
                shrink_a: sa,
                shrink_b: sb,
                .."->".parse::<FancyArrowProps>()?
            };
            ax.annotate(
                format!("shrinkA={sa} shrinkB={sb}"),
                (8.0, y),
                AnnotateOptions {
                    xytext: Some(Point::new(1.0, y)),
                    arrowprops: Some(ArrowProps::Fancy(props)),
                    ..AnnotateOptions::default()
                },
            )?
            .set_valign(VAlign::Bottom);
        }
        let props = FancyArrowProps {
            connectionstyle: "arc3,rad=0.4".parse::<ConnectionStyle>()?,
            .."-|>".parse::<FancyArrowProps>()?
        };
        ax.annotate(
            "boxed",
            (9.0, 1.0),
            AnnotateOptions {
                xytext: Some(Point::new(6.0, 2.5)),
                arrowprops: Some(ArrowProps::Fancy(props)),
                ..AnnotateOptions::default()
            },
        )?
        .set_bbox(round_white_box()?);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/annotate_legacy_arrowprops", |fig| {
        let ax = blank(fig);
        ax.annotate(
            "default dict",
            (8.0, 8.0),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 7.0)),
                arrowprops: Some(ArrowProps::Legacy(LegacyArrowProps::default())),
                ..AnnotateOptions::default()
            },
        )?;
        let legacy = LegacyArrowProps {
            width: 4.0,
            headwidth: 12.0,
            headlength: 10.0,
            shrink: 0.05,
            patch: PatchProps::default().facecolor("tab:green"),
            ..LegacyArrowProps::default()
        };
        ax.annotate(
            "width/headwidth",
            (8.0, 4.0),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 3.0)),
                arrowprops: Some(ArrowProps::Legacy(legacy)),
                ..AnnotateOptions::default()
            },
        )?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
];
