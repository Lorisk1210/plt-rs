//! Legends: entries made from proxy artists, figure legends outside the
//! axes, shadows and titles, entries of bars, error bars and stems, and
//! several handles in one entry (`custom_legends`, `figlegend_demo`,
//! `legend` and `legend_demo`).

use std::f64::consts::{FRAC_PI_2, PI};

use pltrs::axes::StemOptions;
use pltrs::colors::{Colormap, colormaps};
use pltrs::fonts::{FontSize, RelativeSize};
use pltrs::layout::LayoutEngine;
use pltrs::legend::{HandlerTuple, LegendHandle, LegendLoc, LegendOptions};
use pltrs::numeric::geomspace;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;
use pltrs::rc::Cycler;

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `y = f(x)` element-wise.
fn map(x: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    x.iter().copied().map(f).collect()
}

/// `custom_legends`' ten noisy geometric ramps, coloured along `coolwarm`
/// by the property cycle; returns the axes and the colormap.
fn coolwarm_lines(fig: &Figure) -> Result<(Axes, Colormap), Box<dyn std::error::Error>> {
    const LINES: usize = 10;
    const POINTS: usize = 100;
    let ramp = geomspace(1.0, 10.0, POINTS);
    let noise = SpecRng::new(19_680_801).normal(LINES * POINTS, 0.0, 1.0);
    let cmap = colormaps::get("coolwarm")?;
    let colors = linspace(0.0, 1.0, LINES)
        .into_iter()
        .map(|v| Color::from(cmap.sample(v)));
    pltrs::rc::update(|rc| rc.axes_prop_cycle = Cycler::colors_of(colors));
    let ax = fig.subplot();
    for line in noise.chunks(POINTS) {
        ax.plot_y(
            ramp.iter()
                .zip(line)
                .map(|(r, n)| r + n)
                .collect::<Vec<_>>(),
        );
    }
    Ok((ax, cmap))
}

fn custom_legends_1(fig: &Figure) -> CaseResult {
    coolwarm_lines(fig)?;
    Ok(())
}

fn custom_legends_2(fig: &Figure) -> CaseResult {
    let (ax, cmap) = coolwarm_lines(fig)?;
    let custom_lines = [0.0, 0.5, 1.0].map(|v| {
        LegendHandle::line(
            LineProps::default()
                .color(Color::from(cmap.sample(v)))
                .linewidth(4.0),
        )
    });
    ax.legend(
        LegendOptions::default()
            .handles(custom_lines)
            .labels(["Cold", "Medium", "Hot"]),
    )?;
    Ok(())
}

fn custom_legends_3(fig: &Figure) -> CaseResult {
    let legend_elements = [
        LegendHandle::line(LineProps::default().color("b").linewidth(4.0).label("Line")),
        LegendHandle::line(
            LineProps::default()
                .marker(Marker::from(MarkerSymbol::Circle))
                .color("w")
                .label("Scatter")
                .markerfacecolor("g")
                .markersize(15.0),
        ),
        LegendHandle::patch(
            PatchProps::default()
                .facecolor("orange")
                .edgecolor("r")
                .label("Color Patch"),
        ),
    ];
    let ax = fig.subplot();
    ax.legend(
        LegendOptions::default()
            .handles(legend_elements)
            .loc(AnchorLoc::Center),
    )?;
    Ok(())
}

fn figlegend_demo(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?;
    let x = arange(0.0, 4.0 * PI, 0.2);
    let labelled = |label: &str| LineProps::default().label(label);
    let (ax0, ax1) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax0.plot_with(&x, map(&x, f64::sin), labelled("Line 1"))?;
    ax0.plot_with(
        &x,
        map(&x, |x| (-x / 2.0).exp()),
        labelled("Line 2").marker(Marker::from(MarkerSymbol::Circle)),
    )?;
    ax1.plot_with(&x, map(&x, f64::sin), labelled("Line 3").color("tab:green"))?;
    ax1.plot_with(
        &x,
        map(&x, |x| (-x / 4.0).exp()),
        labelled("Line 4")
            .color("tab:red")
            .marker(Marker::from(MarkerSymbol::TriangleUp)),
    )?;
    fig.legend(LegendOptions::default().loc("outside right upper".parse::<LegendLoc>()?))?;
    Ok(())
}

fn legend(fig: &Figure) -> CaseResult {
    let a = arange(0.0, 3.0, 0.02);
    let c = map(&a, f64::exp);
    let d: Vec<f64> = c.iter().rev().copied().collect();
    let total: Vec<f64> = c.iter().zip(&d).map(|(c, d)| c + d).collect();
    let ax = fig.subplot();
    ax.plot_with(&a, &c, LineProps::fmt("k--")?.label("Model length"))?;
    ax.plot_with(&a, &d, LineProps::fmt("k:")?.label("Data length"))?;
    ax.plot_with(
        &a,
        total,
        LineProps::fmt("k")?.label("Total message length"),
    )?;
    let legend = ax.legend(
        LegendOptions::default()
            .loc(AnchorLoc::UpperCenter)
            .shadow(true)
            .fontsize(FontSize::Relative(RelativeSize::XLarge)),
    )?;
    legend.frame().set_facecolor("C0");
    Ok(())
}

fn legend_demo_1(fig: &Figure) -> CaseResult {
    let t1 = arange(0.0, 2.0, 0.1);
    let t2 = arange(0.0, 2.0, 0.01);
    let ax = fig.subplot();
    let decay = map(&t2, |t| (-t).exp());
    ax.plot(&t2, &decay)?;
    let l2 = ax.plot_fmt(&t2, map(&t2, |t| (2.0 * PI * t).sin()), "--o")?;
    ax.plot_fmt(&t1, map(&t1, |t| (1.0 + t).ln()), ".")?;
    let damped: Vec<f64> = t2
        .iter()
        .zip(&decay)
        .map(|(t, e)| e * (2.0 * PI * t).sin())
        .collect();
    let l4 = ax.plot_fmt(&t2, damped, "s-.")?;
    ax.legend(
        LegendOptions::default()
            .handles([&l2, &l4])
            .labels(["oscillatory", "damped"])
            .loc(AnchorLoc::UpperRight)
            .shadow(true),
    )?;
    ax.set_xlabel("time");
    ax.set_ylabel("volts");
    ax.set_title("Damped oscillation");
    Ok(())
}

fn legend_demo_2(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 1.0, 50);
    let axs = fig.subplots(2, 1)?;
    let (ax0, ax1) = (&axs[(0, 0)], &axs[(1, 0)]);
    for n in 1..5 {
        ax0.plot_with(
            &x,
            map(&x, |x| x.powf(f64::from(n))),
            LineProps::default().label(format!("n={n}")),
        )?;
    }
    let leg = ax0.legend(
        LegendOptions::default()
            .loc(AnchorLoc::UpperLeft)
            .bbox_to_anchor((0.0, 1.0))
            .ncols(2)
            .shadow(true)
            .title("Legend")
            .fancybox(true),
    )?;
    leg.title().set_color("red");
    ax1.plot_with(
        &x,
        map(&x, |x| x * x),
        LineProps::default().label("multi\nline"),
    )?;
    let half_pi = linspace(0.0, FRAC_PI_2, 50);
    ax1.plot_with(
        map(&half_pi, f64::sin),
        map(&half_pi, f64::cos),
        LineProps::default().label(r"$\frac{1}{2}\pi$"),
    )?;
    ax1.plot_with(
        &x,
        map(&x, |x| 2f64.powf(x * x)),
        LineProps::default().label("$2^{x^2}$"),
    )?;
    ax1.legend(LegendOptions::default().shadow(true).fancybox(true))?;
    Ok(())
}

fn legend_demo_3(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(3, 1)?;
    let (top_ax, middle_ax, bottom_ax) = (&axs[(0, 0)], &axs[(1, 0)], &axs[(2, 0)]);
    let bars = |label: &str| BarOptions {
        thickness: 0.4.into(),
        align: BarAlign::Center,
        label: Some(label.into()),
        ..BarOptions::default()
    };
    top_ax.bar([0.0, 1.0, 2.0], [0.2, 0.3, 0.1], bars("Bar 1"))?;
    top_ax.bar(
        [0.5, 1.5, 2.5],
        [0.3, 0.2, 0.2],
        BarOptions {
            color: vec![Color::from("red")],
            ..bars("Bar 2")
        },
    )?;
    top_ax.legend(LegendOptions::default())?;
    let x = [0.0, 1.0, 2.0];
    let errors = [
        ([2.0, 3.0, 1.0], Some(0.4), None, "s", "test 1"),
        ([3.0, 2.0, 4.0], None, Some(0.3), "o", "test 2"),
        ([1.0, 1.0, 3.0], Some(0.4), Some(0.3), "^", "test 3"),
    ];
    for (y, xerr, yerr, fmt, label) in errors {
        middle_ax.errorbar(
            x,
            y,
            ErrorbarOptions {
                xerr: xerr.map(Into::into),
                yerr: yerr.map(Into::into),
                fmt: fmt.parse()?,
                line: LineProps::default().label(label),
                ..ErrorbarOptions::default()
            },
        )?;
    }
    middle_ax.legend(LegendOptions::default())?;
    bottom_ax.stem(
        [0.3, 1.5, 2.7],
        [1.0, 3.6, 2.7],
        StemOptions {
            label: Some("stem test".into()),
            ..StemOptions::default()
        },
    )?;
    bottom_ax.legend(LegendOptions::default())?;
    Ok(())
}

fn legend_demo_4(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 1)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    let square = |x: f64, y: f64, color: &str, marker: &str| {
        Ok::<_, Box<dyn std::error::Error>>(ax1.scatter_with(
            vec![x],
            vec![y],
            ScatterOptions {
                colors: ScatterColors::Single(Color::from(color)),
                marker: Some(marker.parse()?),
                sizes: Some(vec![100.0]),
                ..ScatterOptions::default()
            },
        )?)
    };
    let p1 = square(1.0, 5.0, "r", "s")?;
    let p2 = square(3.0, 2.0, "b", "o")?;
    let p3 = ax1.plot_fmt([1.0, 5.0], [4.0, 4.0], "m-d")?;
    let divided = |pad| HandlerTuple { ndivide: None, pad };
    ax1.legend(
        LegendOptions::default()
            .handles([
                LegendHandle::tuple(
                    [LegendHandle::from(&p1), LegendHandle::from(&p3)],
                    divided(None),
                ),
                LegendHandle::from(&p2),
            ])
            .labels(["two keys", "one key"])
            .scatterpoints(1_usize)
            .numpoints(1_usize),
    )?;
    let x_left = [1.0, 2.0, 3.0];
    let rneg = ax2.bar(
        x_left,
        [2.0, 1.0, 4.0],
        BarOptions {
            thickness: 0.5.into(),
            color: vec![Color::from("w")],
            hatch: vec!["///".into()],
            label: Some("-1".into()),
            ..BarOptions::default()
        },
    )?;
    let rpos = ax2.bar(
        x_left,
        [1.0, 3.0, 2.0],
        BarOptions {
            thickness: 0.5.into(),
            color: vec![Color::from("k")],
            label: Some("+1".into()),
            ..BarOptions::default()
        },
    )?;
    let (neg, pos) = (LegendHandle::from(&rneg), LegendHandle::from(&rpos));
    ax2.legend(
        LegendOptions::default()
            .handles([
                LegendHandle::tuple([pos.clone(), neg.clone()], divided(None)),
                LegendHandle::tuple([neg, pos], divided(Some(0.0))),
            ])
            .labels(["pad!=0", "pad=0"]),
    )?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_text/custom_legends_1", custom_legends_1),
    FigureCase::new("gallery_text/custom_legends_2", custom_legends_2),
    FigureCase::new("gallery_text/custom_legends_3", custom_legends_3),
    FigureCase::new("gallery_text/figlegend_demo", figlegend_demo),
    FigureCase::new("gallery_text/legend", legend),
    FigureCase::new("gallery_text/legend_demo_1", legend_demo_1),
    FigureCase::new("gallery_text/legend_demo_2", legend_demo_2),
    FigureCase::new("gallery_text/legend_demo_3", legend_demo_3),
    FigureCase::new("gallery_text/legend_demo_4", legend_demo_4),
];
