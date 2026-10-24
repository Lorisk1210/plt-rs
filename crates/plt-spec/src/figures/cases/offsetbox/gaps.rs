//! The small API gaps of `cases_offsetbox.py`: `Axes.arrow`,
//! `Axes.ecdf`, `pyplot.figtext` and `pyplot.xkcd`.

use plt::axes::EcdfOptions;
use plt::gridspec::SubplotOverrides;
use plt::legend::LegendOptions;
use plt::offsetbox::AnchorLoc;
use plt::orientation::Orientation;
use plt::patches::{ArrowShape, BoxStyle, FancyArrow};
use plt::prelude::*;
use plt::rc::RcParams;
use plt::spines::SpineSide;
use plt::style::{XkcdSketch, apply_xkcd};
use plt::text::{AnnotateOptions, ArrowProps, HAlign, TextBoxProps};
use plt::text::{FancyArrowProps, VAlign};

use super::{of_t, t};
use crate::figures::CaseResult;
use crate::rng::SpecRng;

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn axes_arrow(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.arrow(FancyArrow {
        head_width: Some(0.05),
        head_length: Some(0.1),
        ..FancyArrow::new(0.0, 0.0, 0.5, 0.5)
    })
    .update_props(PatchProps::default().facecolor("k").edgecolor("k"));
    ax.arrow(FancyArrow {
        width: 0.02,
        length_includes_head: true,
        ..FancyArrow::new(0.1, 0.8, 0.6, 0.0)
    })
    .set_color("tab:red");
    ax.arrow(FancyArrow {
        width: 0.03,
        shape: ArrowShape::Left,
        overhang: 0.3,
        ..FancyArrow::new(0.9, 0.1, 0.0, 0.5)
    })
    .set_color("tab:green");
    ax.arrow(FancyArrow {
        width: 0.01,
        shape: ArrowShape::Right,
        head_starts_at_zero: true,
        head_width: Some(0.08),
        ..FancyArrow::new(0.5, 0.1, 0.3, 0.2)
    })
    .set_alpha(Some(0.6));
    ax.arrow(FancyArrow {
        width: 0.015,
        head_length: Some(0.05),
        overhang: -0.4,
        ..FancyArrow::new(0.2, 0.6, -0.15, -0.3)
    })
    .update_props(
        PatchProps::default()
            .fill(false)
            .linewidth(2.0)
            .edgecolor("tab:purple"),
    );
    Ok(())
}

pub(super) fn ecdf(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 3)?;
    let (ax1, ax2, ax3) = (&axes[(0, 0)], &axes[(0, 1)], &axes[(0, 2)]);
    let data = SpecRng::new(301).normal(150, 0.0, 1.0);
    let labelled = |label: &str| EcdfOptions {
        line: LineProps::default().label(label),
        ..EcdfOptions::default()
    };
    ax1.ecdf(&data, labelled("CDF"))?;
    ax1.ecdf(
        &data,
        EcdfOptions {
            complementary: true,
            ..labelled("CCDF")
        },
    )?;
    ax1.legend(LegendOptions::default())?;
    let weights = SpecRng::new(302).uniform(150, 0.2, 2.0);
    ax2.ecdf(
        &data,
        EcdfOptions {
            weights: Some(weights),
            orientation: Orientation::Horizontal,
            line: LineProps::default().color("tab:green").linewidth(2.0),
            ..EcdfOptions::default()
        },
    )?;
    ax2.ecdf(
        &data,
        EcdfOptions {
            orientation: Orientation::Horizontal,
            complementary: true,
            line: LineProps::default().linestyle(LineStyle::Dashed),
            ..EcdfOptions::default()
        },
    )?;
    let counts = [1.0, 2.0, 2.0, 3.0, 5.0, 5.0, 5.0, 8.0, 8.0, 13.0];
    ax3.ecdf(
        &counts,
        EcdfOptions {
            compress: true,
            line: LineProps::default().marker(Marker::Symbol(MarkerSymbol::Circle)),
            ..EcdfOptions::default()
        },
    )?;
    ax3.ecdf(
        &counts,
        EcdfOptions {
            weights: Some(counts.to_vec()),
            compress: true,
            complementary: true,
            ..EcdfOptions::default()
        },
    )?;
    Ok(())
}

pub(super) fn figtext(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot(t(), of_t(|x| x.sin().powi(2)))?;
    fig.subplots_adjust(&SubplotOverrides {
        bottom: Some(0.2),
        ..SubplotOverrides::default()
    })?;
    fig.text(0.5, 0.03, "a caption centred below the axes")
        .set_halign(HAlign::Center)
        .set_fontsize(12.0);
    fig.text(0.02, 0.98, "top left")
        .set_valign(VAlign::Top)
        .set_color("tab:blue");
    fig.text(0.98, 0.5, "rotated on the right")
        .set_rotation(90.0)
        .set_halign(HAlign::Right)
        .set_valign(VAlign::Center)
        .set_bbox(Some(
            TextBoxProps::new("round".parse::<BoxStyle>()?)
                .with_patch(PatchProps::default().facecolor("0.9")),
        ));
    Ok(())
}

/// `rc=_xkcd_rc(scale=2, length=50, randomness=4)`.
pub(super) fn xkcd_strong(rc: &mut RcParams) -> CaseResult {
    let sketch = XkcdSketch {
        scale: 2.0,
        length: 50.0,
        randomness: 4.0,
    };
    Ok(apply_xkcd(rc, sketch)?)
}

pub(super) fn xkcd_bacon(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.1, 0.2, 0.8, 0.7])?;
    let spines = ax.spines();
    for side in [SpineSide::Top, SpineSide::Right] {
        spines.get(side).set_visible(false);
    }
    ax.set_xticks(Vec::<f64>::new());
    ax.set_yticks(Vec::<f64>::new());
    ax.set_ylim(-30.0, 10.0)?;
    let data: Vec<f64> = (0..100)
        .map(|i| if i < 70 { 1.0 } else { 1.0 - f64::from(i - 70) })
        .collect();
    ax.annotate(
        "THE DAY I REALIZED\nI COULD COOK BACON\nWHENEVER I WANTED",
        (70.0, 1.0),
        AnnotateOptions {
            xytext: Some((15.0, -10.0).into()),
            arrowprops: Some(ArrowProps::Fancy(FancyArrowProps::new("->".parse()?))),
            ..AnnotateOptions::default()
        },
    )?;
    let x: Vec<f64> = (0..100).map(f64::from).collect();
    ax.plot(x, data)?;
    ax.set_xlabel("time").set_ylabel("my overall health");
    fig.text(0.5, 0.05, "\"Stove Ownership\" from xkcd by Randall Munroe")
        .set_halign(HAlign::Center);
    Ok(())
}

pub(super) fn xkcd_ticks(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_with(t(), of_t(f64::sin), LineProps::default().label("sine"))?;
    ax.plot_with(
        t(),
        of_t(|x| 0.5 * x.cos()),
        LineProps::default().label("half cosine"),
    )?;
    ax.bar(
        [2.0, 5.0, 8.0],
        [0.4, -0.3, 0.6],
        BarOptions {
            thickness: 1.2.into(),
            color: vec!["tab:orange".into()],
            label: Some("bars".into()),
            ..BarOptions::default()
        },
    )?;
    ax.set_title("sketchy");
    ax.set_xlabel("x");
    ax.legend(LegendOptions::default().loc(AnchorLoc::LowerLeft))?;
    Ok(())
}
