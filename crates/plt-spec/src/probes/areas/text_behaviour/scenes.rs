//! The `text.scene` op: one scene per upstream `test_text.py` test, each
//! returning the values that test asserts on.

use plt::artist::ArtistHandle;
use plt::axes::Axes;
use plt::figure::Figure;
use plt::layout::TightLayout;
use plt::patches::ArrowStyle;
use plt::rc;
use plt::text::{
    AnnotateOptions, Annotation, ArrowProps, HorizontalAlignment, LegacyArrowProps, Text,
    TextRotationMode, VerticalAlignment,
};
use plt_core::Point;

use super::extent;
use crate::json::Json;
use crate::probes::encode::{bbox, object};
use crate::probes::{Args, Outcome, ProbeError};

/// The data points `annotation_contains` presses at.
const CONTAINS_POINTS: [(f64, f64); 7] = [
    (0.5, 0.6),
    (0.5, 0.5),
    (0.45, 0.45),
    (0.61, 0.605),
    (0.3, 0.3),
    (0.62, 0.62),
    (0.7, 0.4),
];

const WRONG_MATH: &str = r"$ \wrong{math} $";

pub(super) fn scene(args: Args<'_>) -> Outcome {
    match args.str("scene")? {
        "annotation_contains" => annotation_contains(),
        "set_position" => set_position(),
        "non_default_dpi" => non_default_dpi(),
        "null_rotation" => Ok(null_rotation()),
        "annotation_update" => annotation_update(),
        "suptitle_wrapped" => Ok(suptitle_wrapped()),
        "offset_fontsize" => offset_fontsize(),
        "antialiased" => antialiased(),
        "annotation_text_only" => annotation_text_only(),
        "annotation_with_arrow" => annotation_with_arrow(),
        "arrow_annotation" => arrow_annotation(),
        "empty_annotation" => empty_annotation(),
        "tightbbox_log_domain" => tightbbox_log_domain(),
        "nonfinite_positions" => nonfinite_positions(),
        "parse_math" => parse_math(),
        other => Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
}

fn annotate(
    ax: &Axes,
    text: &str,
    xy: (f64, f64),
    options: AnnotateOptions,
) -> Result<Annotation, ProbeError> {
    ax.annotate(text, xy, options).map_err(ProbeError::rejected)
}

/// Options placing an annotation: the text at `xytext` in `textcoords`
/// (the coordinates of the point when `None`), the point in `xycoords`.
fn placed(
    xytext: Option<(f64, f64)>,
    xycoords: &str,
    textcoords: Option<&str>,
) -> Result<AnnotateOptions, ProbeError> {
    let parse = |name: &str| name.parse().map_err(ProbeError::rejected);
    Ok(AnnotateOptions {
        xytext: xytext.map(Point::from),
        xycoords: parse(xycoords)?,
        textcoords: textcoords.map(parse).transpose()?,
        ..AnnotateOptions::default()
    })
}

/// matplotlib's legacy `arrowprops={"facecolor": "black", "width": ...,
/// "headwidth": ..., "shrink": 0}`.
fn legacy_arrow(width: f64, headwidth: f64) -> ArrowProps {
    let mut props = LegacyArrowProps {
        width,
        headwidth,
        ..LegacyArrowProps::default()
    };
    props.patch.facecolor = Some("black".into());
    ArrowProps::Legacy(props)
}

fn annotation_contains() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let arrow: ArrowStyle = "->".parse().map_err(ProbeError::rejected)?;
    let options = AnnotateOptions {
        arrowprops: Some(arrow.into()),
        ..placed(Some((0.6, 0.6)), "data", None)?
    };
    let ann = annotate(&ax, "hello", (0.4, 0.4), options)?;
    fig.layout();
    let data = fig
        .resolve_transform(&ax.trans_data())
        .map_err(ProbeError::rejected)?;
    let hits = CONTAINS_POINTS
        .iter()
        .map(|&p| Json::Bool(ann.contains(data.transform_point(p.into())).is_some()))
        .collect();
    Ok(Json::Array(hits))
}

fn set_position() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let ann = annotate(
        &ax,
        "test",
        (0.0, 0.0),
        placed(Some((0.0, 0.0)), "data", Some("figure pixels"))?,
    )?;
    let before = extent(ann.window_extent());
    ann.set_position(Point::new(15.0, 15.0));
    Ok(object([
        ("before", before),
        ("after", extent(ann.window_extent())),
    ]))
}

fn non_default_dpi() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let texts: Vec<Text> = ["", "O"]
        .into_iter()
        .map(|s| {
            let text = ax.text(0.5, 0.5, s);
            text.set_halign(HorizontalAlignment::Left)
                .set_valign(VerticalAlignment::Bottom);
            text
        })
        .collect();
    let dpi = fig.dpi();
    let pairs = texts
        .iter()
        .map(|text| {
            let at_dpi = extent(text.window_extent());
            fig.set_dpi(dpi * 10.0);
            let scaled = extent(text.window_extent());
            fig.set_dpi(dpi);
            Json::Array(vec![at_dpi, scaled])
        })
        .collect();
    Ok(Json::Array(pairs))
}

fn null_rotation() -> Json {
    let fig = Figure::new();
    let ax = fig.subplot();
    let mut rows = Vec::new();
    for ha in ["center", "right", "left"] {
        for va in ["center", "top", "bottom", "baseline", "center_baseline"] {
            let extents = [TextRotationMode::Anchor, TextRotationMode::Default].map(|mode| {
                let text = ax.text(0.5, 0.5, "test");
                text.set_rotation(0.0)
                    .set_valign(VerticalAlignment::parse(va).expect("a valid alignment"))
                    .set_halign(HorizontalAlignment::parse(ha).expect("a valid alignment"))
                    .set_rotation_mode(mode);
                extent(text.window_extent())
            });
            let [anchor, default] = extents;
            rows.push(Json::Array(vec![ha.into(), va.into(), anchor, default]));
        }
    }
    Json::Array(rows)
}

fn annotation_update() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let ann = annotate(&ax, "annotation", (0.5, 0.5), AnnotateOptions::default())?;
    let before = extent(ann.window_extent());
    fig.tight_layout(&TightLayout::default())
        .map_err(ProbeError::rejected)?;
    Ok(object([
        ("before", before),
        ("after", extent(ann.window_extent())),
    ]))
}

fn suptitle_wrapped() -> Json {
    let wrapped = Figure::with_size(3.0, 3.0, 100.0);
    let title = wrapped.suptitle("suptitle that is clearly too long in this case");
    title.set_wrap(true);
    let explicit = Figure::with_size(3.0, 3.0, 100.0);
    let reference = explicit.suptitle("suptitle that is clearly\ntoo long in this case");
    object([
        ("wrapped", extent(title.window_extent())),
        ("explicit", extent(reference.window_extent())),
    ])
}

fn offset_fontsize() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let extents = [
        ((10.0, 10.0), "offset points"),
        ((1.0, 1.0), "offset fontsize"),
    ]
    .into_iter()
    .map(|(xytext, coords)| {
        let ann = annotate(
            &ax,
            "test",
            (0.5, 0.5),
            placed(Some(xytext), "data", Some(coords))?,
        )?;
        ann.set_fontsize(10.0);
        Ok(extent(ann.window_extent()))
    })
    .collect::<Result<Vec<_>, ProbeError>>()?;
    Ok(Json::Array(extents))
}

fn antialiased() -> Outcome {
    let fig = Figure::new();
    let text = fig.text(0.5, 0.5, "foo\nbar");
    let default = text.antialiased();
    let set_false = text.set_antialiased(false).antialiased();
    let set_true = text.set_antialiased(true).antialiased();
    let ax = fig.subplot();
    let ann = annotate(&ax, "foo\nbar", (0.5, 0.5), AnnotateOptions::default())?;
    let annotation = ann.antialiased();
    let ann = annotate(&ax, "foo\nbar", (0.5, 0.5), AnnotateOptions::default())?;
    ann.set_antialiased(false);
    Ok(object([
        ("text", default.into()),
        ("set_false", set_false.into()),
        ("set_true", set_true.into()),
        ("annotation", annotation.into()),
        ("annotation_false", ann.antialiased().into()),
    ]))
}

fn annotation_text_only() -> Outcome {
    let fig = Figure::new();
    let ann = annotate(
        &fig.subplot(),
        "test",
        (0.0, 0.0),
        placed(None, "figure pixels", None)?,
    )?;
    let text = fig.text(0.0, 0.0, "test");
    Ok(object([
        ("annotation", extent(ann.window_extent())),
        ("text", extent(text.window_extent())),
    ]))
}

fn annotation_with_arrow() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let text = ax.text(0.0, 0.0, "test");
    let options = AnnotateOptions {
        arrowprops: Some(legacy_arrow(2.0, 21.0)),
        ..placed(Some((50.0, 50.0)), "figure pixels", None)?
    };
    let ann = annotate(&ax, "test", (0.0, 50.0), options)?;
    // `fig.canvas.draw()`: drawn arrows measure at the canvas dpi.
    fig.layout();
    Ok(object([
        ("text", extent(text.window_extent())),
        ("annotation", extent(ann.window_extent())),
    ]))
}

fn arrow_annotation() -> Outcome {
    let fig = Figure::with_size(2.0, 2.0, 100.0);
    let options = AnnotateOptions {
        arrowprops: Some(legacy_arrow(8.0, 10.0)),
        ..placed(Some((50.0, 50.0)), "figure pixels", None)?
    };
    let ann = annotate(&fig.subplot(), "", (0.0, 50.0), options)?;
    fig.layout();
    Ok(extent(ann.window_extent()))
}

fn empty_annotation() -> Outcome {
    let fig = Figure::with_size(2.0, 2.0, 100.0);
    let ann = annotate(
        &fig.subplot(),
        "",
        (0.0, 50.0),
        placed(Some((0.0, 50.0)), "figure pixels", None)?,
    )?;
    Ok(extent(ann.window_extent()))
}

fn tightbbox_log_domain() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.set_yscale("log").map_err(ProbeError::rejected)?;
    ax.set_ylim(1.0, 100.0);
    for (x, y, s) in [
        (0.0, -5.0, "invalid"),
        (f64::NAN, 10.0, "nan"),
        (0.5, 10.0, "valid"),
    ] {
        ax.text(x, y, s).set_clip_on(false);
    }
    Ok(bbox(&fig.tight_bbox()))
}

fn nonfinite_positions() -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.text(0.0, f64::NAN, "nan");
    ax.text(f64::INFINITY, 0.0, "inf");
    fig.render_raster().map_err(ProbeError::rejected)?;
    Ok("drawn".into())
}

/// Whether a figure holding the malformed math text draws, after
/// `configure` set the text up.
fn draw_outcome(configure: impl FnOnce(&Text)) -> Json {
    let fig = Figure::new();
    configure(&fig.subplot().text(0.0, 0.0, WRONG_MATH));
    let outcome = if fig.render_raster().is_ok() {
        "drawn"
    } else {
        "error"
    };
    outcome.into()
}

fn parse_math() -> Outcome {
    let rc_false = rc::context(|| {
        rc::set("text.parse_math", "False").map_err(ProbeError::rejected)?;
        Ok::<_, ProbeError>(draw_outcome(|_| {}))
    })?;
    Ok(object([
        (
            "parse_math_false",
            draw_outcome(|text| {
                text.set_parse_math(false);
            }),
        ),
        (
            "parse_math_true",
            draw_outcome(|text| {
                text.set_parse_math(true);
            }),
        ),
        ("rc_default", draw_outcome(|_| {})),
        ("rc_false", rc_false),
    ]))
}
