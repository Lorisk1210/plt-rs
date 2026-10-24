//! The pyplot tutorial: lines from lists and format strings, categorical
//! plots, two subplots, a histogram with math text and the four common
//! scales. Its fifth and ninth figures are `gallery_misc/keyword_plotting`
//! and `gallery_text/annotation_basic`.

use std::f64::consts::PI;

use plt::axes::{AxisName, HistPlotOptions};
use plt::gridspec::SubplotOverrides;
use plt::prelude::*;
use plt::scale::ScaleOptions;
use plt::stats::{Bins, HistOptions};

use super::SEED;
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn pyplot_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_y([1.0, 2.0, 3.0, 4.0]);
    ax.set_ylabel("some numbers");
    Ok(())
}

fn pyplot_2(fig: &Figure) -> CaseResult {
    fig.subplot()
        .plot([1.0, 2.0, 3.0, 4.0], [1.0, 4.0, 9.0, 16.0])?;
    Ok(())
}

fn pyplot_3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_fmt([1.0, 2.0, 3.0, 4.0], [1.0, 4.0, 9.0, 16.0], "ro")?;
    ax.set_xlim(0.0, 6.0)?.set_ylim(0.0, 20.0)?;
    Ok(())
}

fn pyplot_4(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 5.0, 0.2);
    let ax = fig.subplot();
    ax.plot_fmt(&t, &t, "r--")?;
    ax.plot_fmt(&t, map(&t, |t| t * t), "bs")?;
    ax.plot_fmt(&t, map(&t, |t| t.powf(3.0)), "g^")?;
    Ok(())
}

fn pyplot_6(fig: &Figure) -> CaseResult {
    let names = vec!["group_a", "group_b", "group_c"];
    let values = [1.0, 10.0, 100.0];
    fig.add_subplot(1, 3, 1)?
        .bar(names.clone(), values, BarOptions::default())?;
    fig.add_subplot(1, 3, 2)?
        .scatter(names.clone(), values.to_vec())?;
    fig.add_subplot(1, 3, 3)?.plot(names, values)?;
    fig.suptitle("Categorical Plotting");
    Ok(())
}

fn pyplot_7(fig: &Figure) -> CaseResult {
    let f = |t: f64| (-t).exp() * (2.0 * PI * t).cos();
    let t1 = arange(0.0, 5.0, 0.1);
    let t2 = arange(0.0, 5.0, 0.02);
    let ax = fig.add_subplot(2, 1, 1)?;
    ax.plot_fmt(&t1, map(&t1, f), "bo")?;
    ax.plot_fmt(&t2, map(&t2, f), "k")?;
    fig.add_subplot(2, 1, 2)?
        .plot_fmt(&t2, map(&t2, |t| (2.0 * PI * t).cos()), "r--")?;
    Ok(())
}

fn pyplot_8(fig: &Figure) -> CaseResult {
    let x = SpecRng::new(SEED).normal(10_000, 100.0, 15.0);
    let ax = fig.subplot();
    let green = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Count(50)),
            density: true,
            ..HistOptions::default()
        },
        props: PatchProps::default().facecolor("g").alpha(0.75),
        ..HistPlotOptions::default()
    };
    ax.hist(&[x], green)?;
    ax.set_xlabel("Smarts").set_ylabel("Probability");
    ax.set_title("Histogram of IQ");
    ax.text(60.0, 0.025, r"$\mu=100,\ \sigma=15$");
    ax.set_xlim(40.0, 160.0)?.set_ylim(0.0, 0.03)?;
    ax.grid(true);
    Ok(())
}

fn pyplot_10(fig: &Figure) -> CaseResult {
    let mut y: Vec<f64> = SpecRng::new(SEED)
        .normal(1000, 0.5, 0.4)
        .into_iter()
        .filter(|&y| y > 0.0 && y < 1.0)
        .collect();
    y.sort_by(f64::total_cmp);
    let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
    let mean = y.iter().sum::<f64>() / y.len() as f64;
    let centred = map(&y, |y| y - mean);
    let symlog = ScaleOptions {
        linthresh: Some(0.01),
        ..ScaleOptions::default()
    };
    let panels = [
        ("linear", &y, ScaleOptions::default()),
        ("log", &y, ScaleOptions::default()),
        ("symlog", &centred, symlog),
        ("logit", &y, ScaleOptions::default()),
    ];
    for (index, (scale, values, options)) in (1..).zip(panels) {
        let ax = fig.add_subplot(2, 2, index)?;
        ax.plot(&x, values)?;
        ax.set_scale_with(AxisName::Y, scale, options)?;
        ax.set_title(scale);
        ax.grid(true);
    }
    fig.subplots_adjust(&SubplotOverrides {
        top: Some(0.92),
        bottom: Some(0.08),
        left: Some(0.10),
        right: Some(0.95),
        hspace: Some(0.25),
        wspace: Some(0.35),
    })?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_tutorials/pyplot_1", pyplot_1),
    FigureCase::new("gallery_tutorials/pyplot_2", pyplot_2),
    FigureCase::new("gallery_tutorials/pyplot_3", pyplot_3),
    FigureCase::new("gallery_tutorials/pyplot_4", pyplot_4),
    FigureCase::new("gallery_tutorials/pyplot_6", pyplot_6).sized(9.0, 3.0, 100.0),
    FigureCase::new("gallery_tutorials/pyplot_7", pyplot_7),
    FigureCase::new("gallery_tutorials/pyplot_8", pyplot_8),
    FigureCase::new("gallery_tutorials/pyplot_10", pyplot_10),
];
