//! Styling several case groups share: fonts and languages, bar labels,
//! contour options, boxes, arrows and annotations, Sankey labels, and path
//! effects.

use std::f64::consts::PI;

use plt::artist::ArtistHandle;
use plt::axes::{Axes, BarLabelOptions};
use plt::collections::CollectionProps;
use plt::colors::Color;
use plt::contour::{ContourOptions, ContourSet, Levels};
use plt::figure::Figure;
use plt::fonts::{FontProperties, FontSize};
use plt::lines::LineProps;
use plt::numeric::arange;
use plt::patches::{ArrowStyle, BoxStyle, ConnectionStyle, PatchProps};
use plt::patheffects::{GcUpdate, PatchEffect, PathEffect, Stroke, TickedStroke};
use plt::sankey::FlowOrientation;
use plt::text::{
    ArrowProps, FancyArrowProps, Language, LanguageRange, LegacyArrowProps, Text, TextBoxProps,
    VAlign,
};

/// Character ranges of a language.
pub(crate) fn ranges(ranges: &[(&str, usize, usize)]) -> Language {
    ranges
        .iter()
        .map(|&(tag, start, end)| LanguageRange::new(tag, start, end))
        .collect()
}

/// The default font at `points`.
pub(crate) fn sized(points: f64) -> FontProperties {
    let rc = plt::rc::params();
    FontProperties::from_rc(&rc).with_size(FontSize::Points(points), rc.font_size)
}

/// `bar_label` options with only `padding` set.
pub(crate) fn padded(padding: f64) -> BarLabelOptions {
    BarLabelOptions {
        padding: vec![padding],
        ..BarLabelOptions::default()
    }
}

/// About `n` automatic contour levels (`levels=n`).
pub(crate) fn count(n: i64) -> ContourOptions {
    ContourOptions {
        levels: Levels::Count(n),
        ..ContourOptions::default()
    }
}

/// Explicit contour levels (`levels=[...]`).
pub(crate) fn values(levels: &[f64]) -> ContourOptions {
    ContourOptions {
        levels: Levels::Values(levels.to_vec()),
        ..ContourOptions::default()
    }
}

/// Colours by name (`colors=[...]`).
pub(crate) fn colors(names: &[&str]) -> Vec<Color> {
    names.iter().map(|&name| Color::from(name)).collect()
}

/// Hatch patterns, `None` leaving a band plain (`hatches=[...]`).
pub(crate) fn hatches(patterns: &[Option<&str>]) -> Vec<Option<String>> {
    patterns.iter().map(|p| p.map(str::to_owned)).collect()
}

/// `bbox=dict(boxstyle=style, **patch)`.
pub(crate) fn styled_box(
    style: &str,
    patch: PatchProps,
) -> Result<Option<TextBoxProps>, Box<dyn std::error::Error>> {
    Ok(Some(
        TextBoxProps::new(style.parse::<BoxStyle>()?).with_patch(patch),
    ))
}

/// The legacy annotation arrow of matplotlib's examples and tests,
/// `arrowprops=dict(facecolor='black', shrink=0.05)`: black, shortened by
/// 5%.
pub(crate) fn black_arrow() -> ArrowProps {
    ArrowProps::Legacy(LegacyArrowProps {
        shrink: 0.05,
        patch: PatchProps::default().facecolor("black"),
        ..LegacyArrowProps::default()
    })
}

/// `arrowprops=dict(arrowstyle=style, connectionstyle=connection)`.
pub(crate) fn arrow(
    style: &str,
    connection: &str,
) -> Result<FancyArrowProps, Box<dyn std::error::Error>> {
    Ok(FancyArrowProps {
        connectionstyle: connection.parse::<ConnectionStyle>()?,
        ..FancyArrowProps::new(style.parse::<ArrowStyle>()?)
    })
}

/// Sankey flow orientations from matplotlib's codes `-1`, `0` and `1`.
pub(crate) fn flow_orientations(codes: &[i8]) -> Vec<FlowOrientation> {
    codes
        .iter()
        .map(|code| match code {
            1 => FlowOrientation::Top,
            -1 => FlowOrientation::Bottom,
            _ => FlowOrientation::Side,
        })
        .collect()
}

/// Sankey flow labels, `None` for Python's `None`.
pub(crate) fn flow_labels(texts: &[Option<&str>]) -> Vec<Option<String>> {
    texts.iter().map(|t| t.map(str::to_owned)).collect()
}

/// Sankey flow labels that all show text.
pub(crate) fn shown_labels(texts: &[&str]) -> Vec<Option<String>> {
    texts.iter().map(|&t| Some(t.to_owned())).collect()
}

/// A stroke `width` points wide in `color` (`Stroke(linewidth=width,
/// foreground=color)`).
pub(crate) fn stroke(width: f64, color: &str) -> Stroke {
    Stroke::new(GcUpdate::default().linewidth(width).foreground(color))
}

/// `[withStroke(linewidth=width, foreground=color)]`.
pub(crate) fn with_stroke(width: f64, color: &str) -> Vec<PathEffect> {
    vec![PathEffect::with_stroke(stroke(width, color))]
}

/// Sets `effects` on every collection of `contours` (matplotlib's
/// `ContourSet.set(path_effects=...)`).
pub(crate) fn contour_effects(contours: &ContourSet, effects: &[PathEffect]) {
    for collection in contours.collections() {
        collection.set_path_effects(effects.to_vec());
    }
}

/// `[withTickedStroke(angle=angle, spacing=spacing, length=length)]`.
pub(crate) fn ticked_stroke(angle: f64, spacing: f64, length: f64) -> Vec<PathEffect> {
    vec![PathEffect::with_ticked_stroke(TickedStroke {
        angle,
        spacing,
        length,
        ..TickedStroke::default()
    })]
}

/// `lw=linewidth, antialiased=False`.
pub(crate) fn aliased(linewidth: f64) -> CollectionProps {
    CollectionProps {
        antialiased: Some(vec![false]),
        ..CollectionProps::default().linewidth(linewidth)
    }
}

/// Plots `cos(2πt)` for `t = np.arange(0, 5, 0.01)` with `line`.
pub(crate) fn cosine(ax: &Axes, line: LineProps) -> Result<(), Box<dyn std::error::Error>> {
    let t = arange(0.0, 5.0, 0.01);
    let s: Vec<f64> = t.iter().map(|t| (2.0 * PI * t).cos()).collect();
    ax.plot_with(t, s, line)?;
    Ok(())
}

/// The black arrow from text at `xytext` in `textcoords` to `xy`.
pub(crate) fn pointing(
    xytext: (f64, f64),
    textcoords: &str,
) -> Result<plt::text::AnnotateOptions, Box<dyn std::error::Error>> {
    Ok(plt::text::AnnotateOptions {
        xytext: Some(xytext.into()),
        textcoords: Some(textcoords.parse()?),
        arrowprops: Some(black_arrow()),
        ..plt::text::AnnotateOptions::default()
    })
}

/// The first annotation of matplotlib's guides (`annotation_basic`): a
/// black arrow from "local max" to the maximum of a thick cosine.
pub(crate) fn local_max(fig: &Figure) -> Result<(), Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    cosine(&ax, LineProps::default().linewidth(2.0))?;
    ax.annotate("local max", (2.0, 1.0), pointing((3.0, 1.5), "data")?)?;
    ax.set_ylim(-2.0, 2.0);
    Ok(())
}

/// A centred, 20-point `Test` annotation from `(0.8, 0.8)` to `(0.2,
/// 0.2)` with the arrow `props` and the box `bbox` (the `Test`
/// annotations of matplotlib's annotation guide).
pub(crate) fn test_annotation(
    ax: &Axes,
    props: FancyArrowProps,
    bbox: Option<TextBoxProps>,
) -> Result<(), Box<dyn std::error::Error>> {
    use plt::text::{HAlign, VAlign};
    let note = ax.annotate(
        "Test",
        (0.2, 0.2),
        plt::text::AnnotateOptions {
            xytext: Some((0.8, 0.8).into()),
            textcoords: Some(plt::coords::Coords::Data),
            arrowprops: Some(ArrowProps::Fancy(props)),
            ..plt::text::AnnotateOptions::default()
        },
    )?;
    note.set_fontsize(20.0)
        .set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_bbox(bbox);
    Ok(())
}

/// matplotlib's "Hatch shadow" text (in its path effects test and guide):
/// 75 points, weight 1000, centred vertically, drawn as a black patch
/// outlined in white over a grey shadow hatched `xxxx`, 4 points right
/// and down (two `PathPatchEffect`s).
pub(crate) fn hatch_shadow_text(text: &Text) -> Result<(), Box<dyn std::error::Error>> {
    let hatched = PatchEffect::new(
        (4.0, -4.0),
        PatchProps::default().hatch("xxxx").facecolor("gray"),
    )?;
    let outlined = PatchEffect::new(
        (0.0, 0.0),
        PatchProps::default()
            .edgecolor("white")
            .facecolor("black")
            .linewidth(1.1),
    )?;
    text.set_fontsize(75.0)
        .set_weight("1000".parse()?)
        .set_valign(VAlign::Center)
        .set_path_effects(vec![
            PathEffect::path_patch(hatched),
            PathEffect::path_patch(outlined),
        ]);
    Ok(())
}
