//! Titles, labels and loose texts: title positions, subplot labels,
//! the many kinds of text of `text_commands`, shared font settings,
//! accented and minus characters, texts strung together, watermarks,
//! text boxes and a line labelled at its end (`titles_demo`,
//! `label_subplots`, `text_commands`, `text_fontdict`, `accented_text`,
//! `unicode_minus`, `rainbow_text`, `watermark_text`,
//! `placing_text_boxes` and `line_with_text`).

use plt::axes::{Axis, HistPlotOptions, TitleLocation};
use plt::coords::Coords;
use plt::figure::{Mosaic, MosaicAxes, MosaicCell, MosaicOptions};
use plt::fonts::{FontSize, FontStyle, FontWeight, RelativeSize};
use plt::gridspec::SubplotOverrides;
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::stats::Bins;
use plt::text::{AnnotateOptions, HAlign, Text, TextBoxProps, VAlign};
use plt::transforms::{Point, Transform};
use plt_core::Affine2D;

use crate::figures::cases::common::{black_arrow, indices, styled_box, tickless};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn titles_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_y(indices(10));
    ax.set_title("Center Title");
    ax.set_title_at(TitleLocation::Left, "Left Title");
    ax.set_title_at(TitleLocation::Right, "Right Title");
    Ok(())
}

/// A constrained figure with two axes plotting `range(10)`.
fn constrained_pair(fig: &Figure) -> Result<Vec<Axes>, Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?;
    for ax in axs.as_slice() {
        ax.plot_y(indices(10));
    }
    Ok(axs.as_slice().to_vec())
}

fn titles_demo_2(fig: &Figure) -> CaseResult {
    let axs = constrained_pair(fig)?;
    for ax in &axs {
        ax.xaxis().set_label_position(SpineSide::Top)?;
        ax.set_xlabel("X-label");
        ax.set_title("Center Title");
    }
    axs[1].xaxis().tick_to(SpineSide::Top)?;
    Ok(())
}

fn titles_demo_3(fig: &Figure) -> CaseResult {
    let axs = constrained_pair(fig)?;
    axs[0].xaxis().set_label_position(SpineSide::Top)?;
    axs[0].set_xlabel("X-label");
    axs[0].set_title("Manual y");
    axs[0]
        .set_title_y(TitleLocation::Center, 1.0)
        .set_title_pad(-14.0);
    plt::rc::set("axes.titley", "1.0")?;
    plt::rc::set("axes.titlepad", "-14")?;
    axs[1].set_xlabel("X-label");
    axs[1].set_title("rcParam y");
    Ok(())
}

/// The mosaic of `label_subplots`, `[['a)', 'c)'], ['b)', 'c)'], ['d)',
/// 'd)']]`, in a constrained figure.
fn label_mosaic(fig: &Figure) -> Result<MosaicAxes, Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let label = |name: &str| MosaicCell::Label(name.into());
    let mosaic = Mosaic::new(vec![
        vec![label("a)"), label("c)")],
        vec![label("b)"), label("c)")],
        vec![label("d)"), label("d)")],
    ])?;
    Ok(fig.subplot_mosaic(&mosaic, MosaicOptions::default())?)
}

fn label_subplots_1(fig: &Figure) -> CaseResult {
    for (label, ax) in label_mosaic(fig)?.iter() {
        let note = ax.annotate(
            label,
            (0.0, 1.0),
            AnnotateOptions {
                xycoords: "axes fraction".parse()?,
                xytext: Some(Point::new(0.5, -0.5).into()),
                textcoords: Some("offset fontsize".parse()?),
                ..AnnotateOptions::default()
            },
        )?;
        note.set_fontsize(RelativeSize::Medium)
            .set_valign(VAlign::Top)
            .set_family(["serif"])
            .set_bbox(Some(TextBoxProps {
                pad: Some(3.0),
                patch: PatchProps::default().facecolor("0.7").edgecolor("none"),
                ..TextBoxProps::default()
            }));
    }
    Ok(())
}

fn label_subplots_2(fig: &Figure) -> CaseResult {
    for (label, ax) in label_mosaic(fig)?.iter() {
        ax.text(0.0, 1.0, label)
            .set_transform(ax.trans_axes().offset_inches(-20.0 / 72.0, 7.0 / 72.0))
            .set_fontsize(RelativeSize::Medium)
            .set_valign(VAlign::Bottom)
            .set_family(["serif"]);
    }
    Ok(())
}

fn label_subplots_3(fig: &Figure) -> CaseResult {
    for (label, ax) in label_mosaic(fig)?.iter() {
        ax.set_title("Normal Title").set_style(FontStyle::Italic);
        ax.set_title_at(TitleLocation::Left, label)
            .set_family(["serif"])
            .set_fontsize(RelativeSize::Medium);
    }
    Ok(())
}

fn text_commands(fig: &Figure) -> CaseResult {
    fig.suptitle("bold figure suptitle")
        .set_fontsize(14.0)
        .set_weight(FontWeight::BOLD);
    let ax = fig.subplot();
    fig.subplots_adjust(SubplotOverrides {
        top: Some(0.85),
        ..SubplotOverrides::default()
    })?;
    ax.set_title("axes title");
    ax.set_xlabel("xlabel");
    ax.set_ylabel("ylabel");
    ax.text(3.0, 8.0, "boxed italics text in data coords")
        .set_style(FontStyle::Italic)
        .set_bbox(Some(TextBoxProps {
            pad: Some(10.0),
            patch: PatchProps::default().facecolor("red").alpha(0.5),
            ..TextBoxProps::default()
        }));
    ax.text(2.0, 6.0, "an equation: $E=mc^2$")
        .set_fontsize(15.0);
    ax.text(3.0, 2.0, "Unicode: Institut für Festkörperphysik");
    ax.text(0.95, 0.01, "colored text in axes coords")
        .set_valign(VAlign::Bottom)
        .set_halign(HAlign::Right)
        .set_transform(ax.trans_axes())
        .set_color("green")
        .set_fontsize(15.0);
    ax.plot_fmt([2.0], [1.0], "o")?;
    ax.annotate(
        "annotate",
        (2.0, 1.0),
        AnnotateOptions {
            xytext: Some(Point::new(3.0, 4.0).into()),
            arrowprops: Some(black_arrow()),
            ..AnnotateOptions::default()
        },
    )?;
    ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
    Ok(())
}

/// The `font` dictionary of `text_fontdict` on a text.
fn dark_red_serif(text: &Text) {
    text.set_family(["serif"])
        .set_color("darkred")
        .set_weight(FontWeight::NORMAL)
        .set_fontsize(16.0);
}

/// The `font` dictionary of `text_fontdict` on an axis label.
fn dark_red_serif_label(axis: &Axis) {
    axis.set_label_family(["serif"])
        .set_label_color("darkred")
        .set_label_weight(FontWeight::NORMAL)
        .set_label_fontsize(FontSize::Points(16.0));
}

fn text_fontdict(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 5.0, 100);
    let y: Vec<f64> = x
        .iter()
        .map(|x| (2.0 * std::f64::consts::PI * x).cos() * (-x).exp())
        .collect();
    let ax = fig.subplot();
    ax.plot_fmt(&x, y, "k")?;
    dark_red_serif(&ax.set_title("Damped exponential decay"));
    dark_red_serif(&ax.text(2.0, 0.65, r"$\cos(2 \pi t) \exp(-t)$"));
    ax.set_xlabel("time (s)");
    dark_red_serif_label(&ax.xaxis());
    ax.set_ylabel("voltage (mV)");
    dark_red_serif_label(&ax.yaxis());
    fig.subplots_adjust(SubplotOverrides {
        left: Some(0.15),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn accented_text_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_title("GISCARD CHAHUTÉ À L'ASSEMBLÉE");
    ax.set_xlabel("LE COUP DE DÉ DE DE GAULLE");
    ax.set_ylabel("André was here!");
    ax.text(0.2, 0.8, "Institut für Festkörperphysik")
        .set_rotation(45.0);
    ax.text(0.4, 0.2, "AVA (check kerning)");
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn unicode_minus(fig: &Figure) -> CaseResult {
    for (y, label, value) in [
        (0.6, "Unicode minus:", "\u{2212}1"),
        (0.3, "ASCII hyphen:", "-1"),
    ] {
        fig.text(0.15, y, label).set_fontsize(20.0);
        fig.text(0.85, y, value)
            .set_halign(HAlign::Right)
            .set_fontsize(20.0);
    }
    Ok(())
}

/// A change of style for one word of `rainbow_text`.
type Restyle = fn(&Text);

fn rainbow_text(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    tickless(&ax);
    let mut text: Text = ax.text(0.1, 0.5, "Matplotlib");
    text.set_color("red");
    let words: [(&str, &str, Restyle); 3] = [
        (" says,", "gold", |t| {
            t.set_weight(FontWeight::BOLD);
        }),
        (" hello", "green", |t| {
            t.set_style(FontStyle::Italic);
        }),
        (" world!", "blue", |t| {
            t.set_family(["serif"]);
        }),
    ];
    for (word, color, style) in words {
        let next = ax.annotate(
            word,
            (1.0, 0.0),
            AnnotateOptions {
                xycoords: Coords::Artist(text.id()),
                ..AnnotateOptions::default()
            },
        )?;
        next.set_valign(VAlign::Bottom).set_color(color);
        style(&next);
        text = Text::clone(&next);
    }
    Ok(())
}

fn watermark_text(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let y = SpecRng::new(19_680_801).uniform(20, 0.0, 1.0);
    let x: Vec<f64> = (0..20).map(f64::from).collect();
    ax.plot_with(
        x,
        y,
        LineProps::fmt("-o")?
            .markersize(20.0)
            .linewidth(2.0)
            .alpha(0.7)
            .markerfacecolor("orange"),
    )?;
    ax.grid(true);
    ax.text(0.5, 0.5, "created with matplotlib")
        .set_transform(ax.trans_axes())
        .set_fontsize(40.0)
        .set_color("gray")
        .set_alpha(Some(0.5))
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_rotation(30.0);
    Ok(())
}

/// The mean, median and (population) standard deviation of `x`.
fn summary(x: &[f64]) -> (f64, f64, f64) {
    let n = x.len() as f64;
    let mean = x.iter().sum::<f64>() / n;
    let mut sorted = x.to_vec();
    sorted.sort_by(f64::total_cmp);
    let half = sorted.len() / 2;
    let median = if sorted.len().is_multiple_of(2) {
        f64::midpoint(sorted[half - 1], sorted[half])
    } else {
        sorted[half]
    };
    let variance = x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (mean, median, variance.sqrt())
}

fn placing_text_boxes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x: Vec<f64> = SpecRng::new(19_680_801)
        .normal(10_000, 0.0, 1.0)
        .iter()
        .map(|v| 30.0 * v)
        .collect();
    let (mu, median, sigma) = summary(&x);
    let textstr = format!("$\\mu={mu:.2}$\n$\\mathrm{{median}}={median:.2}$\n$\\sigma={sigma:.2}$");
    ax.hist(
        &[x],
        HistPlotOptions {
            bins: Some(Bins::Count(50)),
            ..HistPlotOptions::default()
        },
    )?;
    ax.text(0.05, 0.95, textstr)
        .set_transform(ax.trans_axes())
        .set_fontsize(14.0)
        .set_valign(VAlign::Top)
        .set_bbox(styled_box(
            "round",
            PatchProps::default().facecolor("wheat").alpha(0.5),
        )?);
    Ok(())
}

/// `line_with_text`: matplotlib subclasses `Line2D` to draw its label 2
/// pixels beyond its last point right after the line; here the label is
/// a separate text at the line's z-order, added after it.
fn line_with_text(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut rng = SpecRng::new(19_680_801);
    let (x, y) = (rng.uniform(20, 0.0, 1.0), rng.uniform(20, 0.0, 1.0));
    let line = ax.add_line(
        &x,
        &y,
        LineProps::default()
            .markerfacecolor("red")
            .markersize(12.0)
            .label("line label"),
    )?;
    let last = x.len() - 1;
    ax.text(x[last], y[last], line.label())
        .set_transform(
            ax.trans_data()
                .then(Transform::Affine(Affine2D::translation(2.0, 2.0))),
        )
        .set_zorder(line.zorder())
        .set_color("red")
        .set_fontsize(16.0);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_text/titles_demo_1", titles_demo_1),
    FigureCase::new("gallery_text/titles_demo_2", titles_demo_2),
    FigureCase::new("gallery_text/titles_demo_3", titles_demo_3),
    FigureCase::new("gallery_text/label_subplots_1", label_subplots_1),
    FigureCase::new("gallery_text/label_subplots_2", label_subplots_2),
    FigureCase::new("gallery_text/label_subplots_3", label_subplots_3),
    FigureCase::new("gallery_text/text_commands", text_commands),
    FigureCase::new("gallery_text/text_fontdict", text_fontdict),
    FigureCase::new("gallery_text/accented_text_2", accented_text_2),
    FigureCase::new("gallery_text/unicode_minus", unicode_minus).sized(4.0, 2.0, 100.0),
    FigureCase::new("gallery_text/rainbow_text", rainbow_text).with_rc(&[("font.size", "20")]),
    FigureCase::new("gallery_text/watermark_text", watermark_text),
    FigureCase::new("gallery_text/placing_text_boxes", placing_text_boxes),
    FigureCase::new("gallery_text/line_with_text", line_with_text),
];
