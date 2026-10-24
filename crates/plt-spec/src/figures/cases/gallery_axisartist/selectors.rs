//! Selectors before the first selection: `lasso_selector_demo_sgskip`,
//! `polygon_selector_demo`, `polygon_selector_simple`,
//! `rectangle_selector` and `span_selector`.

use std::f64::consts::PI;

use plt::axes::ScatterOptions;
use plt::layout::LayoutEngine;
use plt::navigation::PointerButton;
use plt::patches::PatchProps;
use plt::prelude::*;
use plt::widgets::{
    EllipseSelector, LassoSelector, Orientation, PolygonSelector, PolygonSelectorOptions,
    RectangleSelector, RectangleSelectorOptions, SelectorUnits, SpanSelector, SpanSelectorOptions,
};

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_axisartist/lasso_selector_demo_sgskip",
        lasso_selector_demo,
    ),
    FigureCase::new(
        "gallery_axisartist/polygon_selector_demo",
        polygon_selector_demo,
    ),
    FigureCase::new(
        "gallery_axisartist/polygon_selector_simple_1",
        polygon_selector_simple_1,
    ),
    FigureCase::new("gallery_axisartist/polygon_selector_simple_2", |fig| {
        PolygonSelector::new(&fig.subplot(), PolygonSelectorOptions::default());
        Ok(())
    }),
    FigureCase::new("gallery_axisartist/rectangle_selector", rectangle_selector),
    FigureCase::new("gallery_axisartist/span_selector", span_selector).sized(8.0, 6.0, 100.0),
];

fn lasso_selector_demo(fig: &Figure) -> CaseResult {
    let data = SpecRng::new(19_680_801).uniform(200, 0.0, 1.0);
    let (rows, _) = data.as_chunks::<2>();
    let (x, y): (Vec<f64>, Vec<f64>) = rows.iter().map(|&[x, y]| (x, y)).unzip();
    let ax = fig.subplot();
    ax.set_xlim(0.0, 1.0)
        .set_ylim(0.0, 1.0)
        .set_autoscale_on(false);
    let options = ScatterOptions {
        sizes: Some(vec![80.0]),
        ..ScatterOptions::default()
    };
    ax.scatter_with(x, y, options)?;
    let _lasso = LassoSelector::new(&ax, LineProps::default(), None);
    ax.set_title("Press enter to accept selected points.");
    Ok(())
}

fn polygon_selector_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let grid_size = 5;
    let (grid_x, grid_y): (Vec<f64>, Vec<f64>) = (0..grid_size * grid_size)
        .map(|i| (f64::from(i % grid_size), f64::from(i / grid_size)))
        .unzip();
    ax.scatter(grid_x, grid_y)?;
    let options = PolygonSelectorOptions {
        draw_bounding_box: true,
        ..PolygonSelectorOptions::default()
    };
    PolygonSelector::new(&ax, options);
    Ok(())
}

fn polygon_selector_simple_1(fig: &Figure) -> CaseResult {
    let selector = PolygonSelector::new(&fig.subplot(), PolygonSelectorOptions::default());
    selector.set_verts(&[(0.1, 0.4), (0.5, 0.9), (0.3, 0.2)])?;
    Ok(())
}

fn rectangle_selector(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 1)?;
    let x = linspace(0.0, 10.0, 100_000);
    let y = map(&x, |x| (2.0 * PI * x).sin());
    let options = RectangleSelectorOptions {
        buttons: Some(vec![PointerButton::Left, PointerButton::Right]),
        minspanx: 5.0,
        minspany: 5.0,
        spancoords: SelectorUnits::Pixels,
        interactive: true,
        ..RectangleSelectorOptions::default()
    };
    let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
    for ax in [top, bottom] {
        ax.plot(x.clone(), y.clone())?;
    }
    top.set_title(
        "Press 't' to toggle the selectors on and off.\n\
         Click and drag to draw a RectangleSelector.",
    );
    let _rectangle = RectangleSelector::new(top, options.clone());
    bottom.set_title("Click and drag to draw a EllipseSelector.");
    let _ellipse = EllipseSelector::new(bottom, options);
    Ok(())
}

fn span_selector(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    let x = arange(0.0, 5.0, 0.01);
    let noise = SpecRng::new(19_680_801).normal(x.len(), 0.0, 1.0);
    let y: Vec<f64> = x
        .iter()
        .zip(&noise)
        .map(|(x, n)| (2.0 * PI * x).sin() + 0.5 * n)
        .collect();
    ax1.plot(x, y)?;
    ax1.set_ylim(-2.0, 2.0);
    ax1.set_title("Press left mouse button and drag to select a region in the top graph");
    ax2.plot(Vec::<f64>::new(), Vec::<f64>::new())?;
    let options = SpanSelectorOptions {
        props: PatchProps::default().alpha(0.5).facecolor("tab:blue"),
        interactive: true,
        drag_from_anywhere: true,
        ..SpanSelectorOptions::default()
    };
    let _span = SpanSelector::new(ax1, Orientation::Horizontal, options);
    Ok(())
}
