//! Scatter legends of `figures/cases_scatter.py`: entries made by
//! `legend_elements`, several legends on one axes and hatched markers.

use pltrs::axes::{MappedColors, ScatterColors, ScatterOptions};
use pltrs::collections::{
    EdgeColors, ElementCount, LegendElementsOptions, LegendProperty, LegendValueFn,
};
use pltrs::colors::{Color, Colorize, colormaps};
use pltrs::figure::Figure;
use pltrs::legend::LegendOptions;
use pltrs::numeric::{arange, linspace};
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::ArtistHandle;
use pltrs::ticker::StrMethodFormatter;

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("scatter/legend_elements_colors_sizes", colors_sizes).sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/legend_elements_num_fmt_func", num_fmt_func).sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/hatched_markers_legend", hatched_markers).sized(4.0, 3.0, 100.0),
];

fn colors_sizes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut r = SpecRng::new(11);
    let (x, y) = (r.uniform(40, 0.0, 1.0), r.uniform(40, 0.0, 1.0));
    let classes: Vec<f64> = r.integers(40, 1, 5).into_iter().map(|c| c as f64).collect();
    let sizes = r.uniform(40, 20.0, 300.0);
    let options = ScatterOptions {
        colors: MappedColors::new(classes).into(),
        sizes: Some(sizes),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with(x, y, options)?;
    let (handles, labels) = points.legend_elements(LegendElementsOptions::default())?;
    let classes = LegendOptions::default()
        .handles(handles)
        .labels(labels)
        .loc(AnchorLoc::LowerLeft)
        .title("Classes");
    ax.keep_legend(&ax.legend(classes)?);
    let sizes = LegendElementsOptions {
        prop: LegendProperty::Sizes,
        alpha: Some(0.6),
        ..LegendElementsOptions::default()
    };
    let (handles, labels) = points.legend_elements(sizes)?;
    let sizes = LegendOptions::default()
        .handles(handles)
        .labels(labels)
        .loc(AnchorLoc::UpperRight)
        .title("Sizes");
    ax.legend(sizes)?;
    Ok(())
}

fn num_fmt_func(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 1.0, 30);
    let price: Vec<f64> = x.iter().map(|x| 2.0 + 18.0 * x * x).collect();
    let options = ScatterOptions {
        sizes: Some(price.iter().map(|p| p * 20.0).collect()),
        colors: MappedColors::new(linspace(-1.0, 3.0, 30))
            .cmap(colormaps::get("coolwarm")?)
            .into(),
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("k")])),
        linewidths: Some(vec![0.5]),
        ..ScatterOptions::default()
    };
    let y: Vec<f64> = x.iter().map(|x| (4.0 * x).sin()).collect();
    let points = ax.scatter_with(x, y, options)?;
    let sizes = LegendElementsOptions {
        prop: LegendProperty::Sizes,
        num: ElementCount::Target(4),
        color: Some(Color::from("0.4")),
        fmt: Some(Box::new(StrMethodFormatter::new("$ {x:.2f}")?)),
        func: LegendValueFn::new(|s| s / 20.0),
        ..LegendElementsOptions::default()
    };
    let (handles, labels) = points.legend_elements(sizes)?;
    let price = LegendOptions::default()
        .handles(handles)
        .labels(labels)
        .loc(AnchorLoc::LowerLeft)
        .title("Price");
    ax.keep_legend(&ax.legend(price)?);
    let values = LegendElementsOptions {
        num: ElementCount::Values(vec![0.0, 1.5, 3.0]),
        ..LegendElementsOptions::default()
    };
    let (handles, labels) = points.legend_elements(values)?;
    let value = LegendOptions::default()
        .handles(handles)
        .labels(labels)
        .loc(AnchorLoc::UpperRight)
        .title("Value");
    ax.legend(value)?;
    Ok(())
}

fn hatched_markers(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(0.0, 6.0, 1.0);
    let rows: Vec<f64> = x.iter().map(|v| v % 3.0).collect();
    let hollow = ScatterOptions {
        sizes: Some(vec![600.0]),
        marker: Some("s".parse()?),
        colors: ScatterColors::Single(Color::none()),
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("tab:blue")])),
        ..ScatterOptions::default()
    };
    let squares = ax.scatter_with(&x, &rows, hollow)?;
    squares.set_hatch("///").set_label("hatched");
    let dotted = ScatterOptions {
        sizes: Some(vec![300.0]),
        colors: ScatterColors::Single(Color::from("tab:orange")),
        ..ScatterOptions::default()
    };
    let shifted: Vec<f64> = rows.iter().map(|v| v + 0.5).collect();
    let circles = ax.scatter_with(&x, shifted, dotted)?;
    circles.set_hatch("..").set_label("dotted");
    circles.set_zorder(1.0);
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperLeft))?;
    Ok(())
}
