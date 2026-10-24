//! `axes_scales`: linear and logarithmic axes, the other built-in scales,
//! scales with parameters and a function scale.

use std::f64::consts::PI;

use plt::figure::{MosaicAxes, MosaicOptions};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::scale::{FuncTransform, ScaleOptions};
use plt::ticker::{AxisName, FixedLocator, NullFormatter};

use crate::figures::cases::common::{inverse_mercator, map, mercator, mosaic};
use crate::figures::{CaseResult, FigureCase};

/// The page's curve: `2 sin x + 3` over `[0, 3π)` in steps of 0.1.
fn raised_sine() -> (Vec<f64>, Vec<f64>) {
    let x = arange(0.0, 3.0 * PI, 0.1);
    let y = map(&x, |x| 2.0 * x.sin() + 3.0);
    (x, y)
}

/// A constrained mosaic of `rows`.
fn constrained_mosaic(
    fig: &Figure,
    rows: &[&[&str]],
) -> Result<MosaicAxes, Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    Ok(fig.subplot_mosaic(&mosaic(rows)?, MosaicOptions::default())?)
}

/// The mosaic of the first figures, its axes named by their x and y
/// scales.
const SCALE_MOSAIC: &[&[&str]] = &[&["linear", "linear-log"], &["log-linear", "log-log"]];

fn axes_scales_1(fig: &Figure) -> CaseResult {
    let (x, y) = raised_sine();
    for (name, ax) in constrained_mosaic(fig, SCALE_MOSAIC)?.iter() {
        let (xscale, yscale) = name.split_once('-').unwrap_or((name, name));
        ax.plot(&x, &y)?;
        ax.set_xscale(xscale)?.set_yscale(yscale)?;
        ax.set_xlabel(xscale).set_ylabel(yscale);
    }
    Ok(())
}

fn axes_scales_2(fig: &Figure) -> CaseResult {
    let (x, y) = raised_sine();
    let axs = constrained_mosaic(fig, SCALE_MOSAIC)?;
    let ax = |name: &str| axs.get(name).ok_or("a mosaic axes");
    ax("linear")?.plot(&x, &y)?;
    ax("linear-log")?.semilogy(&x, &y)?;
    ax("log-linear")?.semilogx(&x, &y)?;
    ax("log-log")?.loglog(&x, &y)?;
    for (name, method, xlabel, ylabel) in [
        ("linear", "plot", "linear", "linear"),
        ("linear-log", "semilogy", "linear", "log"),
        ("log-linear", "semilogx", "log", "linear"),
        ("log-log", "loglog", "log", "log"),
    ] {
        let ax = ax(name)?;
        ax.set_xlabel(xlabel).set_ylabel(ylabel);
        ax.set_title(format!("{method}(x, y)"));
    }
    Ok(())
}

fn axes_scales_3(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 1000.0, 1.0);
    let mean = x.iter().sum::<f64>() / x.len() as f64;
    for (name, ax) in constrained_mosaic(fig, &[&["asinh", "symlog"], &["log", "logit"]])?.iter() {
        let yy = match name {
            "asinh" | "symlog" => map(&x, |x| x - mean),
            "logit" => map(&x, |x| x / 999.0),
            _ => x.clone(),
        };
        ax.plot(&yy, &yy)?;
        ax.set_yscale(name)?;
        ax.set_title(name);
    }
    Ok(())
}

fn axes_scales_4(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 1000.0, 1.0);
    let axs = constrained_mosaic(fig, &[&["log", "symlog"]])?;
    let log = axs.get("log").ok_or("a log axes")?;
    log.plot(&x, &x)?;
    let base_2 = ScaleOptions {
        base: Some(2.0),
        ..ScaleOptions::default()
    };
    log.set_scale_with(AxisName::Y, "log", base_2)?;
    log.set_title("log base=2");
    let symlog = axs.get("symlog").ok_or("a symlog axes")?;
    let centred = map(&x, |x| x - 499.5);
    symlog.plot(&centred, &centred)?;
    let linthresh = ScaleOptions {
        linthresh: Some(100.0),
        ..ScaleOptions::default()
    };
    symlog.set_scale_with(AxisName::Y, "symlog", linthresh)?;
    symlog.set_title("symlog linthresh=100");
    Ok(())
}

fn axes_scales_5(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 170.0, 0.1);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    ax.plot_with(
        &t,
        map(&t, |t| t / 2.0),
        LineProps::fmt("-")?.linewidth(2.0),
    )?;
    let mercator = ScaleOptions {
        functions: Some(FuncTransform::new(mercator, inverse_mercator)),
        ..ScaleOptions::default()
    };
    ax.set_scale_with(AxisName::Y, "function", mercator)?;
    ax.set_title("function: Mercator");
    ax.grid(true);
    ax.set_xlim(0.0, 180.0)?;
    ax.yaxis()
        .set_minor_formatter(NullFormatter)
        .set_major_locator(FixedLocator::new(arange(0.0, 90.0, 10.0)));
    Ok(())
}

fn axes_scales_6(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 1000.0, 1.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    fig.subplot().semilogy(&x, &x)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/axes_scales_1", axes_scales_1),
    FigureCase::new("gallery_guides_axes/axes_scales_2", axes_scales_2),
    FigureCase::new("gallery_guides_axes/axes_scales_3", axes_scales_3),
    FigureCase::new("gallery_guides_axes/axes_scales_4", axes_scales_4).sized(6.4, 3.0, 100.0),
    FigureCase::new("gallery_guides_axes/axes_scales_5", axes_scales_5),
    FigureCase::new("gallery_guides_axes/axes_scales_6", axes_scales_6).sized(3.2, 3.0, 100.0),
];
