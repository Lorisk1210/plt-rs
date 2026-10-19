//! Zooms, insets and the rest: `axes_zoom_effect`, `zoom_inset_axes`,
//! `broken_axis`, `geo_demo` and `axhspan_demo`.

use plt::axes::{ImshowOptions, TickParams};
use plt::axes_grid1::{BboxConnector, BboxConnectorPatch, BboxPatch, BboxSource, Corner};
use plt::figure::{Mosaic, MosaicCell, MosaicOptions, Share};
use plt::geo::GeoProjection;
use plt::gridspec::SubplotOverrides;
use plt::image::{Extent, Origin};
use plt::numeric::{Grid, meshgrid};
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::transforms::{Bbox, Point, Transform};

use super::{SEED, convolve};
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `_zoom_effect`: shades `bbox1` on `ax1` and `bbox2` on `ax2` and joins
/// them with lines and a band drawn on `ax2` (`zoom_effect01` and
/// `zoom_effect02` with `loc1a=3, loc2a=2, loc1b=4, loc2b=1`).
fn zoom_effect(ax1: &Axes, ax2: &Axes, bbox1: &BboxSource, bbox2: &BboxSource) {
    let shade = || PatchProps::default().edgecolor("none").alpha(0.2);
    let lines = [
        (Corner::LowerLeft, Corner::UpperLeft),
        (Corner::LowerRight, Corner::UpperRight),
    ]
    .map(|(loc1, loc2)| BboxConnector::new(bbox1.clone(), bbox2.clone(), loc1, Some(loc2)));
    ax1.add_patch(BboxPatch::new(bbox1.clone()))
        .update_props(shade());
    ax2.add_patch(BboxPatch::new(bbox2.clone()))
        .update_props(shade());
    for line in lines {
        ax2.add_patch(line).set_clip_on(false);
    }
    let band = BboxConnectorPatch {
        bbox1: bbox1.clone(),
        bbox2: bbox2.clone(),
        loc1a: Corner::LowerLeft,
        loc2a: Corner::UpperLeft,
        loc1b: Corner::LowerRight,
        loc2b: Corner::UpperRight,
    };
    ax2.add_patch(band).update_props(shade()).set_clip_on(false);
}

fn axes_zoom_effect(fig: &Figure) -> CaseResult {
    let cell = |name: &str| MosaicCell::Label(name.into());
    let mosaic = Mosaic::new(vec![
        vec![cell("zoom1"), cell("zoom2")],
        vec![cell("main"), cell("main")],
    ])?;
    let axs = fig.subplot_mosaic(&mosaic, &MosaicOptions::default())?;
    let axes = |name: &str| axs.get(name).ok_or("a mosaic cell");
    let (zoom1, zoom2, main) = (axes("zoom1")?, axes("zoom2")?, axes("main")?);
    main.set_xlim(0.0, 5.0);
    let span = Bbox::from_extents(0.2, 0.0, 0.8, 1.0);
    zoom_effect(
        zoom1,
        main,
        &BboxSource::transformed(span, zoom1.trans_xaxis()),
        &BboxSource::transformed(span, main.trans_xaxis()),
    );
    zoom2.set_xlim(2.0, 3.0);
    let limits_on_main = zoom2
        .trans_data()
        .then(zoom2.trans_axes().inverted())
        .then(main.trans_axes());
    let trans = Transform::blended(main.trans_data(), limits_on_main);
    zoom_effect(
        zoom2,
        main,
        &BboxSource::axes(zoom2),
        &BboxSource::view_limits(zoom2, trans),
    );
    Ok(())
}

/// `_bivariate_normal`: the 15 × 15 grid of
/// `axes_grid/bivariate_normal.npy`, the difference of two Gaussians over
/// [-3, 4] × [-4, 3], times ten.
fn bivariate_normal() -> Grid<f64> {
    let (x, y) = meshgrid(&arange(-3.0, 4.001, 0.5), &arange(-4.0, 3.001, 0.5));
    let tau = 2.0 * std::f64::consts::PI;
    Grid::from_fn(x.rows(), x.cols(), |r, c| {
        let (x, y) = (x[(r, c)], y[(r, c)]);
        let z1 = (-(x * x + y * y) / 2.0).exp() / tau;
        let (dx, dy) = ((x - 1.0) / 1.5, (y - 1.0) / 0.5);
        let z2 = (-(dx * dx + dy * dy) / 2.0).exp() / (tau * 1.5 * 0.5);
        (z1 - z2) * 10.0
    })
}

fn zoom_inset_axes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let z = bivariate_normal();
    let z2 = Grid::from_fn(150, 150, |r, c| {
        r.checked_sub(30)
            .zip(c.checked_sub(30))
            .and_then(|(r, c)| z.get(r, c).copied())
            .unwrap_or(0.0)
    });
    let image = || ImshowOptions {
        extent: Some(Extent::new(-3.0, 4.0, -4.0, 3.0)),
        origin: Some(Origin::Lower),
        ..ImshowOptions::default()
    };
    ax.imshow(z2.clone(), image())?;
    let axins = ax.inset_axes([0.5, 0.5, 0.47, 0.47]);
    axins.set_xlim(-1.5, -0.9).set_ylim(-2.5, -1.9);
    axins.set_xticklabels(Vec::<String>::new())?;
    axins.set_yticklabels(Vec::<String>::new())?;
    axins.imshow(z2, image())?;
    ax.indicate_inset_zoom(&axins, PatchProps::default().edgecolor("black"));
    Ok(())
}

fn broken_axis(fig: &Figure) -> CaseResult {
    let mut pts = map(&SpecRng::new(SEED).uniform(30, 0.0, 1.0), |v| v * 0.2);
    for i in [3, 14] {
        pts[i] += 0.8;
    }
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(0.05),
        ..SubplotOverrides::default()
    })?;
    ax1.plot_y(&pts);
    ax2.plot_y(&pts);
    ax1.set_ylim(0.78, 1.0);
    ax2.set_ylim(0.0, 0.22);
    ax1.spines().get(SpineSide::Bottom).set_visible(false);
    ax2.spines().get(SpineSide::Top).set_visible(false);
    ax1.xaxis().tick_to(SpineSide::Top)?;
    ax1.tick_params(&TickParams {
        labeltop: Some(false),
        ..TickParams::default()
    });
    ax2.xaxis().tick_to(SpineSide::Bottom)?;
    let d = 0.5;
    let slant = |ax: &Axes| {
        LineProps::default()
            .marker(Marker::Vertices(vec![
                Point::new(-1.0, -d),
                Point::new(1.0, d),
            ]))
            .markersize(12.0)
            .linestyle(LineStyle::None)
            .color("k")
            .markeredgecolor(Color::from("k"))
            .markeredgewidth(1.0)
            .clip_on(false)
            .transform(ax.trans_axes())
    };
    ax1.plot_with([0.0, 1.0], [0.0, 0.0], slant(ax1))?;
    ax2.plot_with([0.0, 1.0], [1.0, 1.0], slant(ax2))?;
    Ok(())
}

/// `_geo`: an empty map of `projection` with its grid, titled.
fn geo(fig: &Figure, projection: GeoProjection, title: &str) {
    let ax = fig.geo_subplot(projection);
    ax.set_title(title);
    ax.grid(true);
}

fn axhspan_demo(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let window = [1.0 / 30.0; 30];
    let smoothed = convolve(&SpecRng::new(SEED).normal(500, 0.0, 1.0), &window);
    let s = map(&smoothed[window.len() - 1..500], |v| 2.9 * v);
    ax1.plot_y(&s);
    ax1.axhspan(-1.0, 1.0).set_alpha(Some(0.1));
    ax1.set_ylim(-1.5, 1.5);
    ax1.set_title("axhspan");
    let (mu, sigma) = (8.0, 2.0);
    let x = linspace(0.0, 16.0, 401);
    let y = map(&x, |x| {
        (-((x - mu) * (x - mu)) / (2.0 * sigma * sigma)).exp()
    });
    for (low, high, gray) in [
        (mu - 2.0 * sigma, mu - sigma, "0.95"),
        (mu - sigma, mu + sigma, "0.9"),
        (mu + sigma, mu + 2.0 * sigma, "0.95"),
    ] {
        ax2.axvspan(low, high).set_color(gray);
    }
    ax2.axvline(
        mu,
        LineProps::default()
            .color("darkgrey")
            .linestyle(LineStyle::Dashed),
    );
    ax2.plot(&x, &y)?;
    ax2.set_title("axvspan");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_subplots/axes_zoom_effect", axes_zoom_effect),
    FigureCase::new("gallery_subplots/zoom_inset_axes", zoom_inset_axes),
    FigureCase::new("gallery_subplots/broken_axis", broken_axis),
    FigureCase::new("gallery_subplots/geo_demo_1", |fig| {
        geo(fig, GeoProjection::Aitoff, "Aitoff");
        Ok(())
    }),
    FigureCase::new("gallery_subplots/geo_demo_2", |fig| {
        geo(fig, GeoProjection::Hammer, "Hammer");
        Ok(())
    }),
    FigureCase::new("gallery_subplots/geo_demo_3", |fig| {
        geo(fig, GeoProjection::LAMBERT, "Lambert");
        Ok(())
    }),
    FigureCase::new("gallery_subplots/axhspan_demo", axhspan_demo).sized(7.0, 3.0, 100.0),
];
