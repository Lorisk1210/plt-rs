//! The scatter plots of the section: sizes and colours from a price
//! series, marginal histograms, masked sizes, star and polygon markers and
//! legends made of colours, sizes and `legend_elements`.

use std::f64::consts::PI;

use plt::axes::{Axes, HistPlotOptions, TickParams};
use plt::collections::{
    Collection, EdgeColors, ElementCount, LegendElementsOptions, LegendProperty,
};
use plt::colors::{Alpha, colormaps};
use plt::figure::{Mosaic, MosaicOptions, Share};
use plt::fonts::FontSize;
use plt::layout::{LayoutEngine, TightLayout};
use plt::legend::{LegendLoc, LegendOptions};
use plt::markers::{MarkerStyle, PolygonStyle};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::stats::{Bins, HistOrientation};
use plt::ticker::{AxisName, StrMethodFormatter};
use plt::transforms::Point;

use crate::figures::cases::common::{cumsum, map};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Scatter options colouring the points by `values` through the default
/// colormap.
fn mapped(values: Vec<f64>) -> ScatterOptions {
    ScatterOptions {
        colors: MappedColors::new(values).into(),
        ..ScatterOptions::default()
    }
}

fn scatter_demo2(fig: &Figure) -> CaseResult {
    // A synthetic stand-in for the last 250 trading days of `goog.npz`.
    let mut rng = SpecRng::new(250);
    let adj_close = map(&cumsum(&rng.normal(250, 0.0, 0.03)), |c| 400.0 * c.exp());
    let volume = rng.uniform(250, 2e6, 8e6);
    let open: Vec<f64> = adj_close
        .iter()
        .zip(rng.normal(250, 0.0, 0.01))
        .map(|(close, change)| close * (1.0 + change))
        .collect();
    let delta1: Vec<f64> = adj_close.windows(2).map(|w| (w[1] - w[0]) / w[0]).collect();
    let sizes = volume[..248]
        .iter()
        .map(|v| (15.0 * v / volume[0]).powi(2))
        .collect();
    let close = adj_close[..248]
        .iter()
        .zip(&open)
        .map(|(adj, open)| 0.003 * adj / 0.003 * open)
        .collect();
    let ax = fig.subplot();
    let options = ScatterOptions {
        sizes: Some(sizes),
        alpha: Alpha::Uniform(0.5),
        ..mapped(close)
    };
    ax.scatter_with(&delta1[..248], &delta1[1..], options)?;
    ax.set_xlabel(r"$\Delta_i$");
    ax.xaxis().set_label_fontsize(FontSize::Points(15.0));
    ax.set_ylabel(r"$\Delta_{i+1}$");
    ax.yaxis().set_label_fontsize(FontSize::Points(15.0));
    ax.set_title("Volume and percent change");
    ax.grid(true);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The example's `scatter_hist`: a scatter plot with the histograms of its
/// coordinates above and beside it, in bins 0.25 wide.
fn scatter_hist(x: &[f64], y: &[f64], ax: &Axes, ax_histx: &Axes, ax_histy: &Axes) -> CaseResult {
    ax_histx.tick_params(&TickParams {
        axis: Some(AxisName::X),
        labelbottom: Some(false),
        ..TickParams::default()
    });
    ax_histy.tick_params(&TickParams {
        axis: Some(AxisName::Y),
        labelleft: Some(false),
        ..TickParams::default()
    });
    ax.scatter(x, y)?;
    let binwidth = 0.25;
    let xymax = x.iter().chain(y).fold(0.0_f64, |max, v| max.max(v.abs()));
    let lim = ((xymax / binwidth).trunc() + 1.0) * binwidth;
    let bins = arange(-lim, lim + binwidth, binwidth);
    let histogram = |orientation| HistPlotOptions {
        bins: Some(Bins::Edges(bins.clone())),
        orientation,
        ..HistPlotOptions::default()
    };
    ax_histx.hist(&[x], histogram(HistOrientation::Vertical))?;
    ax_histy.hist(&[y], histogram(HistOrientation::Horizontal))?;
    Ok(())
}

/// The 1000 normal `x` and `y` of `scatter_hist`.
fn scatter_hist_data() -> (Vec<f64>, Vec<f64>) {
    let mut rng = SpecRng::new(19_680_801);
    (rng.normal(1000, 0.0, 1.0), rng.normal(1000, 0.0, 1.0))
}

fn scatter_hist_1(fig: &Figure) -> CaseResult {
    let (x, y) = scatter_hist_data();
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let options = MosaicOptions {
        width_ratios: Some(vec![4.0, 1.0]),
        height_ratios: Some(vec![1.0, 4.0]),
        ..MosaicOptions::default()
    };
    let axs = fig.subplot_mosaic(&"x.;sy".parse::<Mosaic>()?, options)?;
    let get = |label| axs.get(label).ok_or("a mosaic cell");
    scatter_hist(&x, &y, get("s")?, get("x")?, get("y")?)
}

fn scatter_hist_2(fig: &Figure) -> CaseResult {
    let (x, y) = scatter_hist_data();
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    let ax_histx = ax.inset_axes([0.0, 1.05, 1.0, 0.25]);
    ax_histx.share_x(&ax)?;
    let ax_histy = ax.inset_axes([1.05, 0.0, 0.25, 1.0]);
    ax_histy.share_y(&ax)?;
    scatter_hist(&x, &y, &ax, &ax_histx, &ax_histy)
}

fn scatter_masked(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let r0 = 0.6;
    let x = map(&rng.uniform(100, 0.0, 1.0), |u| 0.9 * u);
    let y = map(&rng.uniform(100, 0.0, 1.0), |u| 0.9 * u);
    let area = map(&rng.uniform(100, 0.0, 1.0), |u| (20.0 * u).powi(2));
    let ax = fig.subplot();
    // Points whose size is masked are left out, colours included.
    for (marker, inside) in [("^", false), ("o", true)] {
        let kept: Vec<usize> = (0..100)
            .filter(|&i| (x[i].hypot(y[i]) < r0) == inside)
            .collect();
        let pick = |values: &[f64]| kept.iter().map(|&i| values[i]).collect::<Vec<_>>();
        let options = ScatterOptions {
            sizes: Some(pick(&area)),
            marker: Some(marker.parse()?),
            ..mapped(pick(&area).iter().map(|a| a.sqrt()).collect())
        };
        ax.scatter_with(pick(&x), pick(&y), options)?;
    }
    let theta = arange(0.0, PI / 2.0, 0.01);
    ax.plot(map(&theta, |t| r0 * t.cos()), map(&theta, |t| r0 * t.sin()))?;
    Ok(())
}

fn scatter_star_poly(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let x = rng.uniform(10, 0.0, 1.0);
    let y = rng.uniform(10, 0.0, 1.0);
    let z: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x.hypot(*y)).collect();
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots_shared(2, 3, Share::All, Share::All)?;
    let verts = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, -1.0)]
        .map(|(x, y)| Point::new(x, y))
        .to_vec();
    let polygon = |sides, style| Marker::Polygon {
        sides,
        style,
        angle: 0.0,
    };
    let markers: [(Marker, &str); 6] = [
        (">".parse()?, "marker='>'"),
        (r"$\clubsuit$".parse()?, r"marker=r'\$\clubsuit\$'"),
        (Marker::Vertices(verts), "marker=verts"),
        (polygon(5, PolygonStyle::Regular), "marker=(5, 0)"),
        (polygon(5, PolygonStyle::Star), "marker=(5, 1)"),
        (polygon(5, PolygonStyle::Asterisk), "marker=(5, 2)"),
    ];
    for (ax, (marker, title)) in axs.as_slice().iter().zip(markers) {
        let fillstyle = plt::rc::params().markers_fillstyle.parse()?;
        let options = ScatterOptions {
            sizes: Some(vec![80.0]),
            marker: Some(MarkerStyle::new(marker, fillstyle)?),
            ..mapped(z.clone())
        };
        ax.scatter_with(&x, &y, options)?;
        ax.set_title(title);
    }
    Ok(())
}

fn scatter_with_legend_1(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let ax = fig.subplot();
    for color in ["tab:blue", "tab:orange", "tab:green"] {
        let (x, y) = (rng.uniform(750, 0.0, 1.0), rng.uniform(750, 0.0, 1.0));
        let scale = map(&rng.uniform(750, 0.0, 1.0), |u| 200.0 * u);
        let options = ScatterOptions {
            colors: ScatterColors::Single(Color::from(color)),
            sizes: Some(scale),
            alpha: Alpha::Uniform(0.3),
            edgecolors: Some(EdgeColors::Colors(vec![Color::none()])),
            ..ScatterOptions::default()
        };
        ax.scatter_with(x, y, options)?.set_label(color);
    }
    ax.legend(LegendOptions::default())?;
    ax.grid(true);
    Ok(())
}

/// A legend of `points` built by `legend_elements` with `options`.
fn elements_legend(
    points: &Collection,
    options: LegendElementsOptions,
    loc: &str,
    title: &str,
) -> Result<LegendOptions, Box<dyn std::error::Error>> {
    let (handles, labels) = points.legend_elements(options)?;
    Ok(LegendOptions::default()
        .handles(handles)
        .labels(labels)
        .loc(loc.parse::<LegendLoc>()?)
        .title(title))
}

/// `values` as floats.
fn floats(values: Vec<i64>) -> Vec<f64> {
    values.into_iter().map(|v| v as f64).collect()
}

fn scatter_with_legend_2(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let (x, y) = (rng.uniform(45, 0.0, 1.0), rng.uniform(45, 0.0, 1.0));
    let c = floats(rng.integers(45, 1, 5));
    let s = floats(rng.integers(45, 10, 220));
    let ax = fig.subplot();
    let options = ScatterOptions {
        sizes: Some(s),
        ..mapped(c)
    };
    let points = ax.scatter_with(x, y, options)?;
    let classes = elements_legend(
        &points,
        LegendElementsOptions::default(),
        "lower left",
        "Classes",
    )?;
    ax.keep_legend(&ax.legend(classes)?);
    let sizes = LegendElementsOptions {
        prop: LegendProperty::Sizes,
        alpha: Some(0.6),
        ..LegendElementsOptions::default()
    };
    ax.legend(elements_legend(&points, sizes, "upper right", "Sizes")?)?;
    Ok(())
}

/// `np.random.rayleigh(scale, size=n)` by inversion.
fn rayleigh(rng: &mut SpecRng, scale: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|_| scale * (-2.0 * (1.0 - rng.next_f64()).ln()).sqrt())
        .collect()
}

/// `np.random.poisson(lam, size=n)` by Knuth's multiplication method.
fn poisson(rng: &mut SpecRng, lam: f64, n: usize) -> Vec<f64> {
    let limit = (-lam).exp();
    (0..n)
        .map(|_| {
            let (mut k, mut p) = (0.0, rng.next_f64());
            while p > limit {
                k += 1.0;
                p *= rng.next_f64();
            }
            k
        })
        .collect()
}

fn scatter_with_legend_3(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let volume = rayleigh(&mut rng, 27.0, 40);
    let amount = poisson(&mut rng, 10.0, 40);
    let ranking = rng.normal(40, 0.0, 1.0);
    let price = rng.uniform(40, 1.0, 10.0);
    let cmap = colormaps::get("Spectral")?;
    let ax = fig.subplot();
    let options = ScatterOptions {
        sizes: Some(map(&price, |p| 0.3 * (p * 3.0).powi(2))),
        colors: MappedColors::new(ranking)
            .limits(Some(-3.0), Some(3.0))
            .cmap(cmap.clone())
            .into(),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with(volume, amount, options)?;
    let five = LegendElementsOptions {
        num: ElementCount::Target(5),
        ..LegendElementsOptions::default()
    };
    ax.keep_legend(&ax.legend(elements_legend(&points, five, "upper left", "Ranking")?)?);
    let prices = LegendElementsOptions {
        prop: LegendProperty::Sizes,
        num: ElementCount::Target(5),
        color: Some(Color::from(cmap.sample(0.7))),
        fmt: Some(Box::new(StrMethodFormatter::new("$ {x:.2f}")?)),
        func: Some(Box::new(|s| (s / 0.3).sqrt() / 3.0)),
        ..LegendElementsOptions::default()
    };
    ax.legend(elements_legend(&points, prices, "lower right", "Price")?)?;
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_lines/scatter_demo2", scatter_demo2),
    FigureCase::new("gallery_lines/scatter_hist_1", scatter_hist_1).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_lines/scatter_hist_2", scatter_hist_2),
    FigureCase::new("gallery_lines/scatter_masked", scatter_masked),
    FigureCase::new("gallery_lines/scatter_star_poly", scatter_star_poly),
    FigureCase::new("gallery_lines/scatter_with_legend_1", scatter_with_legend_1),
    FigureCase::new("gallery_lines/scatter_with_legend_2", scatter_with_legend_2),
    FigureCase::new("gallery_lines/scatter_with_legend_3", scatter_with_legend_3),
];
