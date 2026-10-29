//! The filled areas of the section: bands between curves (with
//! transparency, masks, interpolation and transforms), along y as well,
//! Koch snowflakes and span regions.

use std::f64::consts::PI;

use pltrs::axes::{AxisMode, FillBetweenOptions, TickLevel};
use pltrs::collections::CollectionProps;
use pltrs::dates::{DateTime, Datetime64};
use pltrs::figure::Share;
use pltrs::layout::TightLayout;
use pltrs::legend::LegendOptions;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::text::HAlign;

use crate::figures::cases::common::{cumsum, map};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// A band filled with `facecolor` at `alpha`.
fn band(facecolor: &str, alpha: f64) -> FillBetweenOptions {
    FillBetweenOptions {
        props: CollectionProps::default().facecolor(facecolor).alpha(alpha),
        ..FillBetweenOptions::default()
    }
}

/// `band` restricted to the points where `mask` holds.
fn masked_band(mask: Vec<bool>, facecolor: &str, alpha: f64) -> FillBetweenOptions {
    FillBetweenOptions {
        mask: Some(mask),
        ..band(facecolor, alpha)
    }
}

/// A synthetic stand-in for `goog.npz`: `n` consecutive days from
/// 2004-08-19 and a random walk of closing prices.
fn price_series(
    rng: &mut SpecRng,
    n: usize,
) -> Result<(Vec<Datetime64>, Vec<f64>), Box<dyn std::error::Error>> {
    let start = DateTime::new(2004, 8, 19, 0, 0, 0, 0)?.timestamp_micros() / 1_000_000;
    let dates = (0..n as i64)
        .map(|day| Datetime64::new(start + day * 86_400, 0))
        .collect();
    let close = map(&cumsum(&rng.normal(n, 0.002, 0.02)), |c| 100.0 * c.exp());
    Ok((dates, close))
}

fn fill_between_alpha_1(fig: &Figure) -> CaseResult {
    let (dates, close) = price_series(&mut SpecRng::new(2004), 1047)?;
    let axs = fig.subplots_shared(1, 2, Share::All, Share::All)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax1.plot_with(
        dates.clone(),
        close.clone(),
        LineProps::default().linewidth(2.0),
    )?;
    let pricemin = close.iter().copied().fold(f64::INFINITY, f64::min);
    let days = ax2.xaxis().convert_units(dates);
    ax2.fill_between(days, [pricemin], close, band("C0", 0.7))?;
    for ax in [ax1, ax2] {
        ax.grid(true);
        ax.label_outer();
    }
    ax1.set_ylabel("price");
    fig.suptitle("Google (GOOG) daily closing price");
    fig.autofmt_xdate(0.2, 30.0, HAlign::Right, TickLevel::Major)?;
    Ok(())
}

/// The mean and population standard deviation of each row of
/// `rows × cols` row-major values.
fn row_stats(values: &[f64], cols: usize) -> (Vec<f64>, Vec<f64>) {
    values
        .chunks(cols)
        .map(|row| {
            let n = row.len() as f64;
            let mean = row.iter().sum::<f64>() / n;
            let variance = row.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
            (mean, variance.sqrt())
        })
        .unzip()
}

/// `S.cumsum(axis=0)` of `rows × cols` row-major values.
fn cumsum_rows(values: &[f64], cols: usize) -> Vec<f64> {
    let mut sums = values.to_vec();
    for i in cols..sums.len() {
        sums[i] += sums[i - cols];
    }
    sums
}

fn fill_between_alpha_2(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let (nsteps, nwalkers) = (100, 250);
    let t = arange(0.0, 100.0, 1.0);
    let x1 = cumsum_rows(&rng.normal(nsteps * nwalkers, 0.004, 0.02), nwalkers);
    let x2 = cumsum_rows(&rng.normal(nsteps * nwalkers, 0.002, 0.01), nwalkers);
    let (mu1, sigma1) = row_stats(&x1, nwalkers);
    let (mu2, sigma2) = row_stats(&x2, nwalkers);
    let ax = fig.subplot();
    let thick = LineProps::default().linewidth(2.0);
    ax.plot_with(&t, &mu1, thick.clone().label("mean population 1"))?;
    ax.plot_with(&t, &mu2, thick.label("mean population 2"))?;
    for (mu, sigma, color) in [(&mu1, &sigma1, "C0"), (&mu2, &sigma2, "C1")] {
        let upper: Vec<f64> = mu.iter().zip(sigma).map(|(m, s)| m + s).collect();
        let lower: Vec<f64> = mu.iter().zip(sigma).map(|(m, s)| m - s).collect();
        ax.fill_between(&t, upper, lower, band(color, 0.4))?;
    }
    ax.set_title(r"random walkers empirical $\mu$ and $\pm \sigma$ interval");
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperLeft))?;
    ax.set_xlabel("num steps");
    ax.set_ylabel("position");
    ax.grid(true);
    Ok(())
}

fn fill_between_alpha_3(fig: &Figure) -> CaseResult {
    let (mu, sigma) = (0.002, 0.01);
    let t = arange(0.0, 500.0, 1.0);
    let x = cumsum(&SpecRng::new(1).normal(500, mu, sigma));
    let lower = map(&t, |t| mu * t - sigma * t.sqrt());
    let upper = map(&t, |t| mu * t + sigma * t.sqrt());
    let ax = fig.subplot();
    ax.plot_with(
        &t,
        &x,
        LineProps::default().linewidth(2.0).label("walker position"),
    )?;
    let dashed = LineProps::default()
        .linewidth(1.0)
        .label("population mean")
        .color("C0")
        .linestyle(LineStyle::Dashed);
    ax.plot_with(&t, map(&t, |t| mu * t), dashed)?;
    let sigma_range = FillBetweenOptions {
        props: band("C0", 0.4).props.label("1 sigma range"),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&t, &lower, &upper, sigma_range)?;
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperLeft))?;
    let above = x.iter().zip(&upper).map(|(x, u)| x > u).collect();
    ax.fill_between(&t, &upper, &x, masked_band(above, "red", 0.4))?;
    let below = x.iter().zip(&lower).map(|(x, l)| x < l).collect();
    ax.fill_between(&t, &lower, &x, masked_band(below, "red", 0.4))?;
    ax.set_xlabel("num steps");
    ax.set_ylabel("position");
    ax.grid(true);
    Ok(())
}

fn fill_between_demo_1(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 2.0, 0.01);
    let y1 = map(&x, |x| (2.0 * PI * x).sin());
    let y2 = map(&x, |x| 0.8 * (4.0 * PI * x).sin());
    let axs = fig.subplots_shared(3, 1, Share::All, Share::None)?;
    let fills: [(&[f64], &str); 3] = [
        (&[0.0], "fill between y1 and 0"),
        (&[1.0], "fill between y1 and 1"),
        (&y2, "fill between y1 and y2"),
    ];
    for (ax, (other, title)) in axs.as_slice().iter().zip(fills) {
        ax.fill_between(&x, &y1, other, FillBetweenOptions::default())?;
        ax.set_title(title);
    }
    axs[(2, 0)].set_xlabel("x");
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

fn fill_between_demo_2(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 10.0, 11);
    let y = [
        3.9, 4.4, 10.8, 10.3, 11.2, 13.1, 14.1, 9.9, 13.9, 15.1, 12.5,
    ];
    let n = x.len() as f64;
    let x_mean = x.iter().sum::<f64>() / n;
    let y_mean = y.iter().sum::<f64>() / n;
    let sxx: f64 = x.iter().map(|x| (x - x_mean).powi(2)).sum();
    let sxy: f64 = x
        .iter()
        .zip(&y)
        .map(|(x, y)| (x - x_mean) * (y - y_mean))
        .sum();
    let (a, b) = (sxy / sxx, y_mean - sxy / sxx * x_mean);
    let y_est = map(&x, |x| a * x + b);
    let x_std = (sxx / n).sqrt();
    let y_err = map(&x, |x| {
        x_std * (1.0 / n + (x - x_mean).powi(2) / sxx).sqrt()
    });
    let ax = fig.subplot();
    ax.plot_fmt(&x, &y_est, "-")?;
    let lower: Vec<f64> = y_est.iter().zip(&y_err).map(|(y, e)| y - e).collect();
    let upper: Vec<f64> = y_est.iter().zip(&y_err).map(|(y, e)| y + e).collect();
    let translucent = FillBetweenOptions {
        props: CollectionProps::default().alpha(0.2),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&x, lower, upper, translucent)?;
    ax.plot_with(&x, y, LineProps::fmt("o")?.color("tab:brown"))?;
    Ok(())
}

fn fill_between_demo_3(fig: &Figure) -> CaseResult {
    let x = [0.0, 1.0, 2.0, 3.0];
    let y1 = [0.8, 0.8, 0.2, 0.2];
    let y2 = [0.0, 0.0, 1.0, 1.0];
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    for (ax, interpolate) in axs.as_slice().iter().zip([false, true]) {
        ax.set_title(format!(
            "interpolation={}",
            if interpolate { "True" } else { "False" }
        ));
        ax.plot_fmt(x, y1, "o--")?;
        ax.plot_fmt(x, y2, "o--")?;
        let above = y1.iter().zip(&y2).map(|(a, b)| a > b).collect();
        let below = y1
            .iter()
            .zip(&y2)
            .map(|(a, b)| if interpolate { a <= b } else { a < b })
            .collect();
        for (mask, color) in [(above, "C0"), (below, "C1")] {
            let options = FillBetweenOptions {
                mask: Some(mask),
                interpolate,
                props: CollectionProps::default().color(color).alpha(0.3),
                ..FillBetweenOptions::default()
            };
            ax.fill_between(x, y1, y2, options)?;
        }
    }
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

fn fill_between_demo_4(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(0.0, 4.0 * PI, 0.01);
    let y = map(&x, f64::sin);
    ax.plot_with(&x, &y, LineProps::default().color("black"))?;
    let threshold = 0.75;
    ax.axhline(
        threshold,
        LineProps::default()
            .color("green")
            .linewidth(2.0)
            .alpha(0.7),
    );
    let options = FillBetweenOptions {
        mask: Some(y.iter().map(|&y| y > threshold).collect()),
        props: CollectionProps::default()
            .color("green")
            .alpha(0.5)
            .transform(ax.trans_xaxis()),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&x, [0.0], [1.0], options)?;
    Ok(())
}

/// `y`, `x1` and `x2` of `fill_betweenx_demo`.
fn betweenx_curves() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let y = arange(0.0, 2.0, 0.01);
    let x1 = map(&y, |y| (2.0 * PI * y).sin());
    let x2 = map(&y, |y| 1.2 * (4.0 * PI * y).sin());
    (y, x1, x2)
}

fn fill_betweenx_demo_1(fig: &Figure) -> CaseResult {
    let (y, x1, x2) = betweenx_curves();
    let axs = fig.subplots_shared(1, 3, Share::None, Share::All)?;
    let fills: [(&[f64], &[f64], &str); 3] = [
        (&[0.0], &x1, "between (x1, 0)"),
        (&x1, &[1.0], "between (x1, 1)"),
        (&x1, &x2, "between (x1, x2)"),
    ];
    for (ax, (left, right, title)) in axs.as_slice().iter().zip(fills) {
        ax.fill_betweenx(&y, left, right, FillBetweenOptions::default())?;
        ax.set_title(title);
    }
    axs[(0, 1)].set_xlabel("x");
    Ok(())
}

/// The curves `x1` and `x2` against `y` in black, filled green where
/// `x2 >= x1` and red where `x2 <= x1`.
fn where_filled(ax: &Axes, y: &[f64], x1: &[f64], x2: &[f64]) -> CaseResult {
    let black = LineProps::default().color("black");
    ax.plot_with(x1, y, black.clone())?;
    ax.plot_with(x2, y, black)?;
    let greater = x2.iter().zip(x1).map(|(b, a)| b >= a).collect();
    let less = x2.iter().zip(x1).map(|(b, a)| b <= a).collect();
    for (mask, color) in [(greater, "green"), (less, "red")] {
        ax.fill_betweenx(y, x1, x2, masked_band(mask, color, 1.0))?;
    }
    Ok(())
}

fn fill_betweenx_demo_2(fig: &Figure) -> CaseResult {
    let (y, x1, x2) = betweenx_curves();
    let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    where_filled(&axs[(0, 0)], &y, &x1, &x2)?;
    axs[(0, 0)].set_title("fill_betweenx where");
    // `np.ma.masked_greater(x2, 1.0)`: masked values neither plot nor fill.
    let masked = map(&x2, |x| if x > 1.0 { f64::NAN } else { x });
    where_filled(&axs[(0, 1)], &y, &x1, &masked)?;
    axs[(0, 1)].set_title("regions with x2 > 1 are masked");
    Ok(())
}

/// The example's Koch snowflake of `order` (scale 10): its corners.
fn koch_snowflake(order: u32) -> (Vec<f64>, Vec<f64>) {
    let scale = 10.0;
    let mut points: Vec<(f64, f64)> = [0.0_f64, 120.0, 240.0]
        .iter()
        .map(|angle| {
            let phi = (angle + 90.0).to_radians();
            let r = scale / 3.0_f64.sqrt();
            (r * phi.cos(), r * phi.sin())
        })
        .collect();
    // `ZR = 0.5 - 0.5j * sqrt(3) / 3`.
    let (zr_re, zr_im) = (0.5, -0.5 * 3.0_f64.sqrt() / 3.0);
    for _ in 0..order {
        let n = points.len();
        points = (0..n)
            .flat_map(|i| {
                let (x1, y1) = points[i];
                let (x2, y2) = points[(i + 1) % n];
                let (dx, dy) = (x2 - x1, y2 - y1);
                [
                    (x1, y1),
                    (x1 + dx / 3.0, y1 + dy / 3.0),
                    (
                        x1 + (dx * zr_re - dy * zr_im),
                        y1 + (dx * zr_im + dy * zr_re),
                    ),
                    (x1 + dx / 3.0 * 2.0, y1 + dy / 3.0 * 2.0),
                ]
            })
            .collect();
    }
    points.into_iter().unzip()
}

fn fill_1(fig: &Figure) -> CaseResult {
    let (x, y) = koch_snowflake(5);
    let ax = fig.subplot();
    ax.axis_mode(AxisMode::Equal)?;
    ax.fill(x, y, PatchProps::default())?;
    Ok(())
}

fn fill_2(fig: &Figure) -> CaseResult {
    let (x, y) = koch_snowflake(2);
    let axs = fig.subplots(1, 3)?;
    let styles = [
        PatchProps::default(),
        PatchProps::default()
            .facecolor("lightsalmon")
            .edgecolor("orangered")
            .linewidth(3.0),
        PatchProps::default()
            .facecolor("none")
            .edgecolor("purple")
            .linewidth(3.0),
    ];
    for (ax, style) in axs.as_slice().iter().zip(styles) {
        ax.set_aspect(Aspect::Equal)?;
        ax.fill(&x, &y, style)?;
    }
    Ok(())
}

fn span_regions(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 2.0, 0.01);
    let s = map(&t, |t| (2.0 * PI * t).sin());
    let ax = fig.subplot();
    ax.plot_with(&t, &s, LineProps::default().color("black"))?;
    ax.axhline(0.0, LineProps::default().color("black"));
    let positive = s.iter().map(|&s| s > 0.0).collect();
    ax.fill_between(&t, [1.0], [0.0], masked_band(positive, "green", 0.5))?;
    let negative = s.iter().map(|&s| s < 0.0).collect();
    ax.fill_between(&t, [-1.0], [0.0], masked_band(negative, "red", 0.5))?;
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_lines/fill_between_alpha_1", fill_between_alpha_1),
    FigureCase::new("gallery_lines/fill_between_alpha_2", fill_between_alpha_2),
    FigureCase::new("gallery_lines/fill_between_alpha_3", fill_between_alpha_3),
    FigureCase::new("gallery_lines/fill_between_demo_1", fill_between_demo_1)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_lines/fill_between_demo_2", fill_between_demo_2),
    FigureCase::new("gallery_lines/fill_between_demo_3", fill_between_demo_3),
    FigureCase::new("gallery_lines/fill_between_demo_4", fill_between_demo_4),
    FigureCase::new("gallery_lines/fill_betweenx_demo_1", fill_betweenx_demo_1)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_lines/fill_betweenx_demo_2", fill_betweenx_demo_2)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_lines/fill_1", fill_1).sized(8.0, 8.0, 100.0),
    FigureCase::new("gallery_lines/fill_2", fill_2).sized(9.0, 3.0, 100.0),
    FigureCase::new("gallery_lines/span_regions", span_regions),
];
