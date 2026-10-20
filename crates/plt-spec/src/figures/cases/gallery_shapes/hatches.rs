//! Hatching: hatch colours of patches, bars and scatter markers, hatched
//! bars, fills and polygons, and swatches of every pattern
//! (`hatchcolor_demo`, `hatch_demo` and `hatch_style_reference`).

use plt::axes::{AxisMode, FillBetweenOptions};
use plt::collections::{CollectionProps, EdgeColors, HatchColors};
use plt::colors::{Alpha, colormaps};
use plt::figure::{Mosaic, MosaicCell, MosaicOptions};
use plt::layout::LayoutEngine;
use plt::patches::{Ellipse, Polygon, Rectangle};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::HorizontalAlignment as HAlign;
use plt::transforms::Point;

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};

fn hatchcolor_demo_1(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let dotted = PatchProps::default()
        .hatch(".")
        .hatchcolor("red")
        .edgecolor("black")
        .linewidth(2.0);
    ax1.add_patch_with(Rectangle::new((0.1, 0.5), 0.8, 0.3), dotted);
    let crossed = PatchProps::default()
        .hatch("x")
        .edgecolor("orange")
        .linewidth(2.0);
    ax1.add_patch_with(Rectangle::new((0.1, 0.1), 0.8, 0.3), crossed);
    let x = arange(1.0, 5.0, 1.0);
    let bars = BarOptions {
        props: PatchProps::default()
            .facecolor("none")
            .edgecolor("red")
            .hatch("//")
            .hatchcolor("blue"),
        ..BarOptions::default()
    };
    ax2.bar(x.clone(), x, bars)?;
    ax2.set_xlim(0.0, 5.0).set_ylim(0.0, 5.0);
    Ok(())
}

fn hatchcolor_demo_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 1.0, 10);
    let y = linspace(0.0, 1.0, 9);
    let stagger = (x[1] - x[0]) / 2.0;
    let (xs, ys): (Vec<f64>, Vec<f64>) = y
        .iter()
        .enumerate()
        .flat_map(|(row, &y)| {
            let shift = if row % 2 == 1 { stagger } else { 0.0 };
            x.iter().map(move |&x| (x + shift, y))
        })
        .unzip();
    let rainbow = colormaps::get("rainbow")?;
    let colors = x.iter().map(|&v| rainbow.sample(v).into()).collect();
    let options = ScatterOptions {
        sizes: Some(vec![1700.0]),
        colors: ScatterColors::Single("none".into()),
        edgecolors: Some(EdgeColors::Colors(vec!["gray".into()])),
        linewidths: Some(vec![2.0]),
        marker: Some("h".parse()?),
        alpha: Alpha::Keep,
        ..ScatterOptions::default()
    };
    ax.scatter_with(xs, ys, options)?
        .set_hatch("xxx")
        .set_hatchcolors(HatchColors::Colors(colors));
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    Ok(())
}

/// Bars over `x` hatched with `patterns` (one each, cycled), black edged,
/// standing on `bottom`.
fn hatched_bars(
    ax: &Axes,
    x: &[f64],
    heights: &[f64],
    bottom: &[f64],
    patterns: &[&str],
) -> CaseResult {
    let bars = BarOptions {
        base: bottom.to_vec(),
        edgecolor: vec!["black".into()],
        hatch: patterns.iter().map(|&p| p.to_owned()).collect(),
        ..BarOptions::default()
    };
    ax.bar(x.to_vec(), heights, bars)?;
    Ok(())
}

fn hatch_demo(fig: &Figure) -> CaseResult {
    let label = |name: &str| MosaicCell::Label(name.into());
    let mosaic = Mosaic::new(vec![
        vec![label("bar1"), label("patches")],
        vec![label("bar2"), label("patches")],
    ])?;
    let axd = fig.subplot_mosaic(&mosaic, &MosaicOptions::default())?;
    let get = |name: &str| axd.get(name).ok_or_else(|| format!("no axes {name}"));
    let (bar1, bar2, patches) = (get("bar1")?, get("bar2")?, get("patches")?);
    let x = arange(1.0, 5.0, 1.0);
    let y1 = x.clone();
    let y2 = vec![4.0; 4];
    hatched_bars(bar1, &x, &y1, &[0.0], &["/"])?;
    hatched_bars(bar1, &x, &y2, &y1, &["//"])?;
    hatched_bars(bar2, &x, &y1, &[0.0], &["--", "+", "x", "\\"])?;
    hatched_bars(bar2, &x, &y2, &y1, &["*", "o", "O", "."])?;
    let x = arange(0.0, 40.0, 0.2);
    let fill = FillBetweenOptions {
        props: CollectionProps::default()
            .hatch("///")
            .zorder(2.0)
            .facecolor("c"),
        ..FillBetweenOptions::default()
    };
    patches.fill_between(x.clone(), map(&x, |x| x.sin() * 4.0 + 30.0), [0.0], fill)?;
    let starred = PatchProps::default().fill(true).hatch("*").facecolor("y");
    patches.add_patch_with(Ellipse::new((4.0, 50.0), 10.0, 10.0), starred);
    let triangle = [(10.0, 20.0), (30.0, 50.0), (50.0, 10.0)].map(Point::from);
    let mixed = PatchProps::default().hatch(r"\/...").facecolor("g");
    patches.add_patch_with(Polygon::new(triangle, true), mixed);
    patches.set_xlim(0.0, 40.0).set_ylim(10.0, 60.0);
    patches.set_aspect(Aspect::Ratio(1.0))?;
    Ok(())
}

/// `hatch_style_reference`'s figures: a labelled square per hatch.
fn hatch_swatches(fig: &Figure, hatches: [&str; 10]) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    let axs = fig.subplots(2, 5)?;
    for (ax, hatch) in axs.as_slice().iter().zip(hatches) {
        let swatch = PatchProps::default().fill(false).hatch(hatch);
        ax.add_patch_with(Rectangle::new((0.0, 0.0), 2.0, 2.0), swatch);
        ax.text(1.0, -0.5, format!("' {hatch} '"))
            .set_fontsize(15.0)
            .set_halign(HAlign::Center);
        ax.axis_mode(AxisMode::Equal);
        ax.axis_mode(AxisMode::Off);
    }
    Ok(())
}

fn hatch_style_reference_1(fig: &Figure) -> CaseResult {
    hatch_swatches(fig, ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*"])
}

fn hatch_style_reference_2(fig: &Figure) -> CaseResult {
    hatch_swatches(
        fig,
        ["//", r"\\", "||", "--", "++", "xx", "oo", "OO", "..", "**"],
    )
}

fn hatch_style_reference_3(fig: &Figure) -> CaseResult {
    hatch_swatches(
        fig,
        ["/o", r"\|", "|*", r"\-", "+o", "x*", "o-", "O|", "O.", "*-"],
    )
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_shapes/hatchcolor_demo_1", hatchcolor_demo_1),
    FigureCase::new("gallery_shapes/hatchcolor_demo_2", hatchcolor_demo_2),
    FigureCase::new("gallery_shapes/hatch_demo", hatch_demo),
    FigureCase::new(
        "gallery_shapes/hatch_style_reference_1",
        hatch_style_reference_1,
    )
    .sized(6.4, 3.2, 100.0),
    FigureCase::new(
        "gallery_shapes/hatch_style_reference_2",
        hatch_style_reference_2,
    )
    .sized(6.4, 3.2, 100.0),
    FigureCase::new(
        "gallery_shapes/hatch_style_reference_3",
        hatch_style_reference_3,
    )
    .sized(6.4, 3.2, 100.0),
];
