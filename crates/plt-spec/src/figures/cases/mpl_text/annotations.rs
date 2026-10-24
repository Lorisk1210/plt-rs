//! The annotation, clipping, opacity, tick label and anti-aliasing cases
//! of `test_text.py`.

use plt::axes::TickLabelProps;
use plt::coords::{CoordUnit, Coords};
use plt::dates::DateTime;
use plt::prelude::*;
use plt::text::{AnnotateOptions, VAlign};
use plt::ticker::NullLocator;
use plt::transforms::Point;

use super::classic;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// An annotation name with the point of its positive and of its negative
/// copy.
type SignedPlacement = (&'static str, (f64, f64), (f64, f64));

/// The annotations of the negative coordinate tests: for each placement an
/// annotation at the positive point and a top-aligned one at the negative
/// point, both in `<unit> <name>` coordinates.
fn signed_annotations(fig: &Figure, unit: &str, placements: [SignedPlacement; 3]) -> CaseResult {
    let ax = fig.subplot();
    for (name, positive, negative) in placements {
        let coords: Coords = format!("{unit} {name}").parse()?;
        for (sign, xy) in [('+', positive), ('-', negative)] {
            let options = AnnotateOptions {
                xytext: Some(Point::from(xy)),
                xycoords: coords.clone(),
                textcoords: Some(coords.clone()),
                ..AnnotateOptions::default()
            };
            let note = ax.annotate(format!("{sign} {name}"), xy, options)?;
            note.set_fontsize(32.0);
            if sign == '-' {
                note.set_valign(VAlign::Top);
            }
        }
    }
    Ok(())
}

/// `test_annotation_negative_ax_coords`.
fn negative_ax_coords(fig: &Figure) -> CaseResult {
    signed_annotations(
        fig,
        "axes",
        [
            ("points", (30.0, 20.0), (30.0, -20.0)),
            ("fraction", (0.75, 0.05), (0.75, -0.05)),
            ("pixels", (160.0, 25.0), (160.0, -25.0)),
        ],
    )
}

/// `test_annotation_negative_fig_coords`.
fn negative_fig_coords(fig: &Figure) -> CaseResult {
    signed_annotations(
        fig,
        "figure",
        [
            ("points", (10.0, 250.0), (-10.0, 310.0)),
            ("fraction", (0.05, 0.5), (-0.05, 0.45)),
            ("pixels", (50.0, 50.0), (-50.0, 150.0)),
        ],
    )
}

/// `test_agg_text_clip`: the same random texts, clipped to the axes above
/// and unclipped below.
fn agg_text_clip(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    let draws = SpecRng::new(1).uniform(20, 0.0, 1.0);
    for &[x, y] in draws.as_chunks::<2>().0 {
        axs[(0, 0)].text(x, y, "foo").set_clip_on(true);
        axs[(1, 0)].text(x, y, "foo");
    }
    Ok(())
}

/// `test_font_scaling`: every even font size from 4 to 42 pt.
fn font_scaling(fig: &Figure) {
    let ax = fig.subplot();
    ax.xaxis().set_major_locator(NullLocator);
    ax.yaxis().set_major_locator(NullLocator);
    ax.set_ylim(-10.0, 600.0);
    for (i, size) in (4..43).step_by(2).enumerate() {
        ax.text(0.1, (i * 30) as f64, format!("{size} pt font size"))
            .set_fontsize(f64::from(size));
    }
}

/// A text of the opacity tests: height, text, colour and alpha.
type OpacityText = (f64, &'static str, Option<(f64, f64, f64, f64)>, Option<f64>);

/// The opacity tests: texts at `x = 0.25` on an axes without axis lines.
fn opacity(fig: &Figure, texts: [OpacityText; 3]) {
    let ax = fig.subplot();
    ax.set_axis_off();
    for (y, text, color, alpha) in texts {
        let text = ax.text(0.25, y, text);
        if let Some(color) = color {
            text.set_color(color);
        }
        text.set_alpha(alpha);
    }
}

/// `test_text_as_path_opacity`: half-transparent single letters.
const PATH_OPACITY: [OpacityText; 3] = [
    (0.25, "c", Some((0.0, 0.0, 0.0, 0.5)), None),
    (0.5, "a", None, Some(0.5)),
    (0.75, "x", Some((0.0, 0.0, 0.0, 1.0)), Some(0.5)),
];

/// `test_text_as_text_opacity`: half-transparent sentences.
const TEXT_OPACITY: [OpacityText; 3] = [
    (0.25, "50% using `color`", Some((0.0, 0.0, 0.0, 0.5)), None),
    (0.5, "50% using `alpha`", None, Some(0.5)),
    (
        0.75,
        "50% using `alpha` and 100% `color`",
        Some((0.0, 0.0, 0.0, 1.0)),
        Some(0.5),
    ),
];

/// `test_annotation_units`: an annotation at a date and an axes fraction,
/// offset by nothing; a fixed date stands for `datetime.now()`.
fn annotation_units(fig: &Figure) -> CaseResult {
    let when = DateTime::new(2026, 10, 19, 12, 30, 0, 0)?;
    let ax = fig.subplot();
    ax.plot_fmt(vec![when], [1.0], "o")?;
    let options = AnnotateOptions {
        xytext: Some(Point::new(0.0, 0.0)),
        xycoords: Coords::blended(Coords::Data, "axes fraction".parse()?),
        textcoords: Some("offset points".parse()?),
        ..AnnotateOptions::default()
    };
    let x = plt::dates::get_epoch().date2num(when);
    ax.annotate("x", (x, 0.5), options)?;
    Ok(())
}

/// `test_buffer_size`: a large non-ASCII tick label next to a white one.
fn buffer_size(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_yticks([0.0, 1.0]);
    ax.set_yticklabels(["\u{20ac}", "a"])?;
    let white = TickLabelProps {
        color: Some("w".into()),
        ..TickLabelProps::default()
    };
    ax.yaxis().set_major_ticklabel_props(1, white);
    Ok(())
}

/// `test_annotate_and_offsetfrom_copy_input`: a text 10 points right of
/// the middle of a line, by `OffsetFrom` (`fig_test`) and by the line as
/// the annotated artist (`fig_ref`).
fn offsetfrom(fig: &Figure, from_line: bool) -> CaseResult {
    let ax = fig.subplot();
    let line = ax.plot([0.0, 2.0], [0.0, 2.0])?;
    let (xy, options) = if from_line {
        let options = AnnotateOptions {
            xytext: Some(Point::new(10.0, 0.0)),
            textcoords: Some(Coords::OffsetFrom {
                artist: line.id(),
                reference: Point::new(0.5, 0.5),
                unit: CoordUnit::Points,
            }),
            ..AnnotateOptions::default()
        };
        ((0.0, 0.0), options)
    } else {
        let options = AnnotateOptions {
            xytext: Some(Point::new(10.0, 0.0)),
            xycoords: Coords::Artist(line.id()),
            textcoords: Some("offset points".parse()?),
            ..AnnotateOptions::default()
        };
        ((0.5, 0.5), options)
    };
    ax.annotate("foo", xy, options)?;
    Ok(())
}

/// The anti-aliasing tests: a figure text, anti-aliased as given or as
/// `text.antialiased` says.
fn antialiased_text(fig: &Figure, antialiased: Option<bool>) {
    let text = fig.text(0.5, 0.5, "6 inches x 2 inches");
    if let Some(antialiased) = antialiased {
        text.set_antialiased(antialiased);
    }
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_text/annotation_negative_ax_coords", negative_ax_coords),
    FigureCase::new(
        "mpl_text/annotation_negative_fig_coords",
        negative_fig_coords,
    ),
    FigureCase::new("mpl_text/agg_text_clip", agg_text_clip),
    FigureCase::new("mpl_text/font_scaling", |fig| {
        font_scaling(fig);
        Ok(())
    })
    .sized(6.4, 12.4, 100.0)
    .with_rc(&[("pdf.fonttype", "42")]),
    classic("mpl_text/text_as_path_opacity", |fig| {
        opacity(fig, PATH_OPACITY);
        Ok(())
    }),
    classic("mpl_text/text_as_text_opacity", |fig| {
        opacity(fig, TEXT_OPACITY);
        Ok(())
    })
    .with_rc_setup(|rc| {
        rc.set("svg.fonttype", "none")?;
        Ok(())
    }),
    classic("mpl_text/annotation_units", annotation_units),
    classic("mpl_text/buffer_size", buffer_size),
    classic("mpl_text/annotate_and_offsetfrom_copy_input", |fig| {
        offsetfrom(fig, true)
    }),
    classic("mpl_text/annotate_and_offsetfrom_copy_input_ref", |fig| {
        offsetfrom(fig, false)
    }),
    classic("mpl_text/text_antialiased_off_default_vs_manual", |fig| {
        antialiased_text(fig, Some(false));
        Ok(())
    }),
    classic(
        "mpl_text/text_antialiased_off_default_vs_manual_ref",
        |fig| {
            antialiased_text(fig, None);
            Ok(())
        },
    )
    .with_rc_setup(|rc| {
        rc.set("text.antialiased", "False")?;
        Ok(())
    }),
    classic("mpl_text/text_antialiased_on_default_vs_manual", |fig| {
        antialiased_text(fig, Some(true));
        Ok(())
    }),
];
