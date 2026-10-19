//! Annotations: every coordinate system an annotated point or its text
//! can use, polar annotations, the connection styles and arrow styles of
//! annotation arrows, arrows clipped by patches and by other annotations
//! (`annotation_basic`, `annotation_polar` and `annotation_demo`).

use std::f64::consts::{FRAC_PI_2, FRAC_PI_3, PI};

use plt::artist::ArtistHandle;
use plt::coords::{CoordUnit, Coords};
use plt::patches::Ellipse;
use plt::polar::PolarAxes;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{
    AnnotateOptions, Annotation, ArrowProps, ArrowStart, FancyArrowProps,
    HorizontalAlignment as HAlign, TextBoxProps, VerticalAlignment as VAlign,
};
use plt::transforms::Point;

use crate::figures::cases::common::{arrow, black_arrow, styled_box};
use crate::figures::{CaseResult, FigureCase};

/// Plots `cos(2πt)` for `t = np.arange(0, 5, 0.01)` with `line`.
fn cosine(ax: &Axes, line: LineProps) -> CaseResult {
    let t = arange(0.0, 5.0, 0.01);
    let s: Vec<f64> = t.iter().map(|t| (2.0 * PI * t).cos()).collect();
    ax.plot_with(t, s, line)?;
    Ok(())
}

/// The black arrow from text at `xytext` in `textcoords` to `xy`.
fn pointing(
    xytext: (f64, f64),
    textcoords: &str,
) -> Result<AnnotateOptions, Box<dyn std::error::Error>> {
    Ok(AnnotateOptions {
        xytext: Some(xytext.into()),
        textcoords: Some(textcoords.parse()?),
        arrowprops: Some(black_arrow()),
        ..AnnotateOptions::default()
    })
}

fn annotation_basic(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    cosine(&ax, LineProps::default().linewidth(2.0))?;
    ax.annotate("local max", (2.0, 1.0), pointing((3.0, 1.5), "data")?)?;
    ax.set_ylim(-2.0, 2.0);
    Ok(())
}

/// The spiral `r = np.arange(0, 1, 0.001)`, `θ = 4πr` plotted with `line`
/// on polar axes, its point 800 marked and annotated from the lower left
/// corner of the figure.
fn polar_annotation(ax: &PolarAxes, line: LineProps) -> CaseResult {
    let r = arange(0.0, 1.0, 0.001);
    let theta: Vec<f64> = r.iter().map(|r| 2.0 * 2.0 * PI * r).collect();
    ax.plot_with(&theta, &r, line)?;
    let (thisr, thistheta) = (r[800], theta[800]);
    ax.plot_fmt([thistheta], [thisr], "o")?;
    ax.annotate(
        "a polar annotation",
        (thistheta, thisr),
        pointing((0.05, 0.05), "figure fraction")?,
    )?
    .set_halign(HAlign::Left)
    .set_valign(VAlign::Bottom);
    Ok(())
}

fn annotation_polar(fig: &Figure) -> CaseResult {
    polar_annotation(
        &fig.polar_subplot(),
        LineProps::default().color("#ee8d18").linewidth(3.0),
    )
}

fn annotation_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    cosine(&ax, LineProps::default())?;
    let at = |coords: &str| -> Result<AnnotateOptions, Box<dyn std::error::Error>> {
        Ok(AnnotateOptions {
            xycoords: coords.parse()?,
            ..AnnotateOptions::default()
        })
    };
    ax.annotate("figure pixels", (10.0, 10.0), at("figure pixels")?)?;
    ax.annotate("figure points", (107.0, 110.0), at("figure points")?)?
        .set_fontsize(12.0);
    ax.annotate("figure fraction", (0.025, 0.975), at("figure fraction")?)?
        .set_halign(HAlign::Left)
        .set_valign(VAlign::Top)
        .set_fontsize(20.0);
    ax.annotate(
        "point offset from data",
        (3.0, 1.0),
        pointing((-10.0, 90.0), "offset points")?,
    )?
    .set_halign(HAlign::Center)
    .set_valign(VAlign::Bottom);
    ax.annotate(
        "axes fraction",
        (2.0, 1.0),
        pointing((0.36, 0.68), "axes fraction")?,
    )?
    .set_halign(HAlign::Right)
    .set_valign(VAlign::Top);
    ax.annotate(
        "pixel offset from axes fraction",
        (1.0, 0.0),
        AnnotateOptions {
            xytext: Some(Point::new(-20.0, 20.0)),
            textcoords: Some("offset pixels".parse()?),
            ..at("axes fraction")?
        },
    )?
    .set_halign(HAlign::Right)
    .set_valign(VAlign::Bottom);
    ax.set_xlim(-1.0, 5.0).set_ylim(-3.0, 5.0);
    Ok(())
}

fn annotation_demo_2(fig: &Figure) -> CaseResult {
    polar_annotation(&fig.polar_subplot(), LineProps::default())
}

fn annotation_demo_3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    ax.add_artist(Ellipse::new((0.0, 0.0), 10.0, 20.0))
        .update_props(PatchProps::default().facecolor("r").alpha(0.5));
    ax.annotate(
        "the top",
        (FRAC_PI_2, 10.0),
        AnnotateOptions {
            xytext: Some(Point::new(FRAC_PI_3, 20.0)),
            xycoords: Coords::Polar,
            textcoords: Some(Coords::Polar),
            arrowprops: Some(black_arrow()),
            clip_on: true,
            ..AnnotateOptions::default()
        },
    )?
    .set_halign(HAlign::Left)
    .set_valign(VAlign::Bottom);
    ax.set_xlim(-20.0, 20.0).set_ylim(-20.0, 20.0);
    Ok(())
}

/// One annotation of `annotation_demo`'s connection styles.
struct Connection {
    /// The text.
    text: &'static str,
    /// The annotated point.
    xy: (f64, f64),
    /// The text's offset from the point, in points.
    offset: (f64, f64),
    /// The arrow's connection style.
    style: &'static str,
    /// The style of the text's grey box, if any.
    boxstyle: Option<&'static str>,
    /// The text's size in points, unless the default.
    size: Option<f64>,
    /// The arrow's `shrinkA` and `shrinkB`.
    shrink: (f64, f64),
}

impl Connection {
    /// A plain text with the default shrinking.
    const fn new(
        text: &'static str,
        xy: (f64, f64),
        offset: (f64, f64),
        style: &'static str,
    ) -> Self {
        Self {
            text,
            xy,
            offset,
            style,
            boxstyle: None,
            size: None,
            shrink: (2.0, 2.0),
        }
    }

    /// The text in a grey box of `style`.
    const fn boxed(self, style: &'static str) -> Self {
        Self {
            boxstyle: Some(style),
            ..self
        }
    }
}

/// The connection styles of `annotation_demo`.
const CONNECTIONS: [Connection; 9] = [
    Connection::new("straight", (0.0, 1.0), (-50.0, 30.0), "arc3"),
    Connection::new("arc3,\nrad 0.2", (0.5, -1.0), (-80.0, -60.0), "arc3,rad=.2"),
    Connection::new(
        "arc,\nangle 50",
        (1.0, 1.0),
        (-90.0, 50.0),
        "arc,angleA=0,armA=50,rad=10",
    ),
    Connection::new(
        "arc,\narms",
        (1.5, -1.0),
        (-80.0, -60.0),
        "arc,angleA=0,armA=40,angleB=-90,armB=30,rad=7",
    ),
    Connection::new(
        "angle,\nangle 90",
        (2.0, 1.0),
        (-70.0, 30.0),
        "angle,angleA=0,angleB=90,rad=10",
    ),
    Connection::new(
        "angle3,\nangle -90",
        (2.5, -1.0),
        (-80.0, -60.0),
        "angle3,angleA=0,angleB=-90",
    ),
    Connection::new(
        "angle,\nround",
        (3.0, 1.0),
        (-60.0, 30.0),
        "angle,angleA=0,angleB=90,rad=10",
    )
    .boxed("round"),
    Connection {
        size: Some(20.0),
        ..Connection::new(
            "angle,\nround4",
            (3.5, -1.0),
            (-70.0, -80.0),
            "angle,angleA=0,angleB=-90,rad=10",
        )
        .boxed("round4,pad=.5")
    },
    Connection {
        shrink: (0.0, 10.0),
        ..Connection::new(
            "angle,\nshrink",
            (4.0, 1.0),
            (-60.0, 30.0),
            "angle,angleA=0,angleB=90,rad=10",
        )
        .boxed("round")
    },
];

/// `bbox=dict(boxstyle=style, fc="0.8")`.
fn grey_box(style: &str) -> Result<Option<TextBoxProps>, Box<dyn std::error::Error>> {
    styled_box(style, PatchProps::default().facecolor("0.8"))
}

fn annotation_demo_4(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    cosine(&ax, LineProps::default().linewidth(3.0))?;
    for connection in &CONNECTIONS {
        let (shrink_a, shrink_b) = connection.shrink;
        let note = ax.annotate(
            connection.text,
            connection.xy,
            AnnotateOptions {
                xytext: Some(connection.offset.into()),
                textcoords: Some("offset points".parse()?),
                arrowprops: Some(ArrowProps::Fancy(FancyArrowProps {
                    shrink_a,
                    shrink_b,
                    ..arrow("->", connection.style)?
                })),
                ..AnnotateOptions::default()
            },
        )?;
        if let Some(style) = connection.boxstyle {
            note.set_bbox(grey_box(style)?);
        }
        if let Some(size) = connection.size {
            note.set_fontsize(size);
        }
    }
    ax.annotate(
        "",
        (4.0, 1.0),
        AnnotateOptions {
            xytext: Some(Point::new(4.5, -1.0)),
            textcoords: Some(Coords::Data),
            arrowprops: Some(ArrowProps::Fancy(FancyArrowProps {
                shrink_a: 5.0,
                shrink_b: 5.0,
                patch: PatchProps::default().edgecolor("k"),
                ..arrow("<->", "bar")?
            })),
            ..AnnotateOptions::default()
        },
    )?;
    ax.set_xlim(-1.0, 5.0).set_ylim(-4.0, 3.0);
    Ok(())
}

/// The pale red of `annotation_demo`'s bubbles.
const BUBBLE: (f64, f64, f64) = (1.0, 0.7, 0.7);
/// The darker red of their contours.
const CONTOUR: (f64, f64, f64) = (1.0, 0.5, 0.5);

fn annotation_demo_5(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let el = ax.add_patch(Ellipse::new((2.0, -1.0), 0.5, 0.5)).id();
    let towards_el = |text: &str,
                      offset: (f64, f64),
                      props: FancyArrowProps|
     -> Result<Annotation, Box<dyn std::error::Error>> {
        let note = ax.annotate(
            text,
            (2.0, -1.0),
            AnnotateOptions {
                xytext: Some(offset.into()),
                textcoords: Some("offset points".parse()?),
                arrowprops: Some(ArrowProps::Fancy(FancyArrowProps {
                    patch_b: Some(el),
                    ..props
                })),
                ..AnnotateOptions::default()
            },
        )?;
        Ok(note)
    };
    let grey = || PatchProps::default().facecolor("0.6").edgecolor("none");
    towards_el(
        "$->$",
        (-150.0, -140.0),
        arrow("->", "angle,angleA=90,angleB=0,rad=10")?,
    )?
    .set_bbox(grey_box("round")?);
    let filled = [
        (
            "arrow\nfancy",
            (-100.0, 60.0),
            "fancy",
            "angle3,angleA=0,angleB=-90",
        ),
        ("arrow\nsimple", (100.0, 60.0), "simple", "arc3,rad=0.3"),
        (
            "wedge",
            (-100.0, -100.0),
            "wedge,tail_width=0.7",
            "arc3,rad=-0.3",
        ),
    ];
    for (text, offset, style, connection) in filled {
        let props = FancyArrowProps {
            patch: grey(),
            ..arrow(style, connection)?
        };
        towards_el(text, offset, props)?.set_fontsize(20.0);
    }
    let bubble_arrow = |relpos: (f64, f64), connection: &str, edge: PatchProps| {
        Ok::<_, Box<dyn std::error::Error>>(FancyArrowProps {
            patch_a: ArrowStart::Unclipped,
            relpos,
            patch: edge.facecolor(BUBBLE),
            ..arrow("wedge,tail_width=1.", connection)?
        })
    };
    let contoured = || PatchProps::default().edgecolor(CONTOUR);
    towards_el(
        "bubble,\ncontours",
        (0.0, -70.0),
        bubble_arrow((0.2, 0.8), "arc3,rad=-0.1", contoured())?,
    )?
    .set_fontsize(20.0)
    .set_bbox(styled_box("round", contoured().facecolor(BUBBLE))?);
    let borderless = || PatchProps::default().edgecolor("none");
    towards_el(
        "bubble",
        (55.0, 0.0),
        bubble_arrow((0.2, 0.5), "arc3", borderless())?,
    )?
    .set_fontsize(20.0)
    .set_valign(VAlign::Center)
    .set_bbox(styled_box("round", borderless().facecolor(BUBBLE))?);
    ax.set_xlim(-1.0, 5.0).set_ylim(-5.0, 3.0);
    Ok(())
}

/// `annotation_demo`'s annotations in grey round boxes with `->` arrows.
struct BoxedNotes<'a> {
    ax: &'a Axes,
}

impl BoxedNotes<'_> {
    /// Annotates `xy` in `xycoords` with `text` at `xytext` in
    /// `textcoords`, aligned by `halign` and `valign`, with `arrow` (or no
    /// arrow).
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the keywords of one annotate call"
    )]
    fn note(
        &self,
        text: &str,
        xy: (f64, f64),
        xycoords: Coords,
        xytext: (f64, f64),
        textcoords: Coords,
        (halign, valign): (HAlign, VAlign),
        arrow: Option<FancyArrowProps>,
    ) -> Result<Annotation, Box<dyn std::error::Error>> {
        let note = self.ax.annotate(
            text,
            xy,
            AnnotateOptions {
                xycoords,
                xytext: Some(xytext.into()),
                textcoords: Some(textcoords),
                arrowprops: arrow.map(ArrowProps::Fancy),
                ..AnnotateOptions::default()
            },
        )?;
        note.set_halign(halign)
            .set_valign(valign)
            .set_bbox(grey_box("round")?);
        Ok(note)
    }
}

fn annotation_demo_6(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let plain = || arrow("->", "arc3");
    let curved = || arrow("->", "arc3,rad=0.2");
    let offset_points: Coords = "offset points".parse()?;
    let notes = BoxedNotes { ax: ax1 };
    for coords in ["figure fraction", "axes fraction"] {
        for (corner, offset, align) in [
            (0.0, 20.0, (HAlign::Left, VAlign::Bottom)),
            (1.0, -20.0, (HAlign::Right, VAlign::Top)),
        ] {
            notes.note(
                &format!("{coords} : {corner}, {corner}"),
                (corner, corner),
                coords.parse()?,
                (offset, offset),
                offset_points.clone(),
                align,
                Some(plain()?),
            )?;
        }
    }
    let centred = (HAlign::Center, VAlign::Center);
    let an1 = notes.note(
        "Drag me 1",
        (0.5, 0.7),
        Coords::Data,
        (0.5, 0.7),
        Coords::Data,
        centred,
        None,
    )?;
    let an2 = notes.note(
        "Drag me 2",
        (0.5, 0.5),
        Coords::Artist(an1.id()),
        (0.5, 0.3),
        "axes fraction".parse()?,
        centred,
        Some(FancyArrowProps {
            patch_b: Some(an1.id()),
            ..curved()?
        }),
    )?;
    notes.note(
        "",
        (0.5, 0.5),
        Coords::Artist(an2.id()),
        (0.5, 0.5),
        Coords::Artist(an1.id()),
        centred,
        Some(FancyArrowProps {
            patch_a: ArrowStart::Artist(an1.id()),
            patch_b: Some(an2.id()),
            ..curved()?
        }),
    )?;
    let notes = BoxedNotes { ax: ax2 };
    let below = (HAlign::Center, VAlign::Top);
    let text = notes.note(
        "xy=(0, 1)\nxycoords=(\"data\", \"axes fraction\")",
        (0.0, 1.0),
        Coords::blended(Coords::Data, "axes fraction".parse()?),
        (0.0, -20.0),
        offset_points.clone(),
        below,
        Some(plain()?),
    )?;
    notes.note(
        "xy=(0.5, 0)\nxycoords=artist",
        (0.5, 0.0),
        Coords::Artist(text.id()),
        (0.0, -20.0),
        offset_points,
        below,
        Some(plain()?),
    )?;
    notes.note(
        "xy=(0.8, 0.5)\nxycoords=ax1.transData",
        (0.8, 0.5),
        Coords::Transform(ax1.trans_data()),
        (10.0, 10.0),
        Coords::OffsetFrom {
            artist: ax2.id(),
            reference: Point::new(0.0, 0.0),
            unit: CoordUnit::Points,
        },
        (HAlign::Left, VAlign::Bottom),
        Some(plain()?),
    )?;
    ax2.set_xlim(-2.0, 2.0).set_ylim(-2.0, 2.0);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_text/annotation_basic", annotation_basic),
    FigureCase::new("gallery_text/annotation_polar", annotation_polar),
    FigureCase::new("gallery_text/annotation_demo_1", annotation_demo_1).sized(4.0, 4.0, 100.0),
    FigureCase::new("gallery_text/annotation_demo_2", annotation_demo_2).sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_text/annotation_demo_3", annotation_demo_3),
    FigureCase::new("gallery_text/annotation_demo_4", annotation_demo_4).sized(8.0, 5.0, 100.0),
    FigureCase::new("gallery_text/annotation_demo_5", annotation_demo_5),
    FigureCase::new("gallery_text/annotation_demo_6", annotation_demo_6),
];
