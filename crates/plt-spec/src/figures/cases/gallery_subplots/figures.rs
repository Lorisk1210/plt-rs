//! Whole figures: `multiple_figs_demo`, `figure_size_units`,
//! `custom_figure_class` and `figure_title`.

use std::f64::consts::PI;

use plt::axes::TitleLocation;
use plt::dates::{DateError, DateTime};
use plt::figure::Share;
use plt::fonts::{FontSize, RelativeSize};
use plt::layout::LayoutEngine;
use plt::numeric::nanmax;
use plt::patches::BoxStyle;
use plt::prelude::*;
use plt::text::{HAlign, TextBoxProps, VAlign};

use super::SEED;
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `multiple_figs_demo`'s time and its sines of one and two periods per
/// second.
fn time_and_sines() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let t = arange(0.0, 2.0, 0.01);
    let s1 = map(&t, |t| (2.0 * PI * t).sin());
    let s2 = map(&t, |t| (4.0 * PI * t).sin());
    (t, s1, s2)
}

fn multiple_figs_demo_1(fig: &Figure) -> CaseResult {
    let (t, s1, s2) = time_and_sines();
    let top = fig.add_subplot(2, 1, 1)?;
    top.plot(&t, &s1)?;
    fig.add_subplot(2, 1, 2)?.plot(&t, map(&s1, |s| 2.0 * s))?;
    top.plot_fmt(&t, &s2, "s")?;
    top.set_xticklabels(Vec::<String>::new())?;
    Ok(())
}

fn multiple_figs_demo_2(fig: &Figure) -> CaseResult {
    let (t, _, s2) = time_and_sines();
    fig.subplot().plot(&t, &s2)?;
    Ok(())
}

/// `figure_size_units`: `text` in the middle of an empty axes.
fn size_label(fig: &Figure, text: &str) {
    fig.subplot()
        .text(0.5, 0.5, text)
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_fontsize(28.0)
        .set_color("C1");
}

/// Centimetres in inches.
const CM: f64 = 1.0 / 2.54;

/// Pixels in inches at the default 100 dpi.
const PX: f64 = 1.0 / 100.0;

fn custom_figure_class(fig: &Figure) -> CaseResult {
    // What `WatermarkFigure(watermark='draft')` adds when it is created.
    let watermark = TextBoxProps {
        boxstyle: Some("square".parse::<BoxStyle>()?),
        patch: PatchProps::default()
            .linewidth(3.0)
            .edgecolor("gray")
            .facecolor((0.9, 0.9, 0.9, 0.5))
            .alpha(0.5),
        ..TextBoxProps::default()
    };
    fig.text(0.5, 0.5, "draft")
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_rotation(30.0)
        .set_fontsize(40.0)
        .set_color("gray")
        .set_bbox(Some(watermark))
        .set_alpha(Some(0.5));
    let x = linspace(-3.0, 3.0, 201);
    fig.subplot()
        .plot(&x, map(&x, |x| x.tanh() + 0.1 * (5.0 * x).cos()))?;
    Ok(())
}

fn figure_title_1(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 5.0, 501);
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax1.plot(&x, map(&x, |x| (6.0 * x).cos() * (-x).exp()))?;
    ax1.set_title("damped");
    ax1.set_xlabel("time (s)").set_ylabel("amplitude");
    ax2.plot(&x, map(&x, |x| (6.0 * x).cos()))?;
    ax2.set_xlabel("time (s)");
    ax2.set_title("undamped");
    fig.suptitle("Different types of oscillations")
        .set_fontsize(16.0);
    Ok(())
}

/// `_STOCKS`: each column's name and the months (from January 1990,
/// end exclusive) without a price.
const STOCKS: [(&str, Option<(usize, usize)>); 8] = [
    ("IBM", None),
    ("AAPL", None),
    ("MSFT", None),
    ("XRX", None),
    ("AMZN", Some((0, 88))),
    ("DELL", Some((285, 347))),
    ("GOOGL", Some((0, 175))),
    ("ADBE", None),
];

/// `_stocks`: the first of every month from January 1990 to June 2022,
/// and per column a geometric random walk, missing where `STOCKS` says.
fn stocks() -> Result<(Vec<DateTime>, Vec<Vec<f64>>), DateError> {
    let dates = (0..(2022 - 1990) * 12 + 6)
        .map(|month| DateTime::new(1990 + month / 12, (month % 12) as u32 + 1, 1, 0, 0, 0, 0))
        .collect::<Result<Vec<_>, _>>()?;
    let mut rng = SpecRng::new(SEED);
    let columns = STOCKS
        .iter()
        .map(|&(_, missing)| {
            let mut total = 0.0;
            let mut price: Vec<f64> = rng
                .normal(dates.len(), 0.01, 0.08)
                .into_iter()
                .map(|step| {
                    total += step;
                    f64::exp(total)
                })
                .collect();
            if let Some((start, end)) = missing {
                price[start..end].fill(f64::NAN);
            }
            price
        })
        .collect();
    Ok((dates, columns))
}

fn figure_title_2(fig: &Figure) -> CaseResult {
    let (dates, columns) = stocks()?;
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    let axs = fig.subplots_shared(4, 2, Share::All, Share::All)?;
    for ((ax, (name, _)), y) in axs.as_slice().iter().zip(STOCKS).zip(columns) {
        let peak = nanmax(y.iter().copied()).ok_or("a price")?;
        ax.plot_with(
            dates.clone(),
            map(&y, |v| v / peak),
            LineProps::default().linewidth(2.5),
        )?;
        ax.set_title_at(TitleLocation::Left, name)
            .set_fontsize(FontSize::Relative(RelativeSize::Small));
    }
    fig.supxlabel("Year");
    fig.supylabel("Stock price relative to max");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_subplots/multiple_figs_demo_1",
        multiple_figs_demo_1,
    ),
    FigureCase::new(
        "gallery_subplots/multiple_figs_demo_2",
        multiple_figs_demo_2,
    ),
    FigureCase::new("gallery_subplots/figure_size_units_1", |fig| {
        size_label(fig, "6 inches x 2 inches");
        Ok(())
    })
    .sized(6.0, 2.0, 100.0),
    FigureCase::new("gallery_subplots/figure_size_units_2", |fig| {
        size_label(fig, "15cm x 5cm");
        Ok(())
    })
    .sized(15.0 * CM, 5.0 * CM, 100.0),
    FigureCase::new("gallery_subplots/figure_size_units_3", |fig| {
        size_label(fig, "600px x 200px");
        Ok(())
    })
    .sized(600.0 * PX, 200.0 * PX, 100.0),
    FigureCase::new("gallery_subplots/custom_figure_class", custom_figure_class),
    FigureCase::new("gallery_subplots/figure_title_1", figure_title_1),
    FigureCase::new("gallery_subplots/figure_title_2", figure_title_2).sized(9.0, 5.0, 100.0),
];
