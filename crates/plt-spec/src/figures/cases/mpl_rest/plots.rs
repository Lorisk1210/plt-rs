//! The plotting cases: `test_sankey.py`, `test_category.py`,
//! `test_animation.py`, `test_preprocess_data.py` and
//! `test_backend_cairo.py`.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use plt::animation::{AnimationOptions, Frames, FuncAnimation};
use plt::axes::{Axes, BarOptions, PlotError};
use plt::collections::{CollectionProps, CollectionSpec};
use plt::dates::{DateError, DateTime};
use plt::figure::Figure;
use plt::lines::LineStyle;
use plt::numeric::linspace;
use plt::patches::{PatchProps, PathShape};
use plt::sankey::{FlowOrientation, Sankey, SankeyOptions, Subdiagram};
use plt_core::Rgba;

use super::super::mpl_collections::cut_stars;
use super::classic;
use crate::figures::{CaseResult, FigureCase};

/// `test_sankey3`: a diagram whose flows go up, down and sideways, given
/// to the constructor (which adds them as its first subdiagram).
fn sankey3(fig: &Figure) -> CaseResult {
    use FlowOrientation::{Bottom, Side, Top};
    let flows = Subdiagram {
        flows: vec![0.25, -0.25, -0.25, 0.25, 0.5, -0.5],
        orientations: vec![Top, Bottom, Top, Bottom, Side, Side],
        ..Subdiagram::default()
    };
    Sankey::new(&fig.gca(), SankeyOptions::default())?
        .add(flows)?
        .finish()?;
    Ok(())
}

/// `datetime(2018, 3, day)`.
fn march_2018(day: u32) -> Result<DateTime, DateError> {
    DateTime::new(2018, 3, day, 0, 0, 0, 0)
}

/// `test_overriding_units_in_plot`: dates against categories, plotted
/// twice; the second plot keeps the units of the first.
fn overriding_units_in_plot(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    for (first, second) in [(1, 2), (3, 4)] {
        let dates = vec![march_2018(first)?, march_2018(second)?];
        ax.plot(dates, ["V1", "V2"])?;
    }
    Ok(())
}

/// `test_animation_frame`: a sine wave moved by a blitted animation
/// saved through its five frames, showing the last (saving draws the
/// animated line).
fn animation_frame(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 2.0 * PI)?.set_ylim(-1.0, 1.0)?;
    let x = linspace(0.0, 2.0 * PI, 100);
    let line = ax.plot(Vec::<f64>::new(), Vec::<f64>::new())?;
    let options = AnimationOptions {
        repeat: false,
        blit: true,
        ..AnimationOptions::default()
    };
    let cleared = line.clone();
    let init = move || {
        cleared.set_data(Vec::<f64>::new(), Vec::<f64>::new());
        cleared.clone()
    };
    let animate = move |i: usize| {
        let shift = i as f64 / 100.0;
        let y: Vec<f64> = x.iter().map(|x| (x + shift).sin()).collect();
        line.set_data(&x, &y);
        line.clone()
    };
    let animation = FuncAnimation::with_init(fig, Frames::Count(5), options, init, animate);
    animation.to_gif()?;
    Ok(())
}

/// The plotting methods `TestPlotTypes` runs through.
#[derive(Clone, Copy, Debug)]
enum Plotter {
    Scatter,
    Bar,
    Plot,
}

/// `TestPlotTypes.test_dict_unpack`: `plotter` given the keys and the
/// values of a dictionary.
fn dict_unpack(fig: &Figure, plotter: Plotter) -> CaseResult {
    let ddict = BTreeMap::from([(1, 4.0), (2, 5.0), (3, 6.0)]);
    let keys: Vec<f64> = ddict.keys().map(|&k| f64::from(k)).collect();
    let values: Vec<f64> = ddict.values().copied().collect();
    let ax = fig.subplot();
    match plotter {
        Plotter::Scatter => {
            ax.scatter(keys, values)?;
        }
        Plotter::Bar => {
            ax.bar(keys, values, BarOptions::default())?;
        }
        Plotter::Plot => {
            ax.plot(keys, values)?;
        }
    }
    Ok(())
}

/// Half-transparent red faces and three-quarter-opaque blue edges.
const FACE: Rgba = Rgba::new(1.0, 0.0, 0.0, 0.5);
const EDGE: Rgba = Rgba::new(0.0, 0.0, 1.0, 0.75);

/// The axes of both sides of `test_patch_alpha_coloring`.
fn alpha_axes(fig: &Figure) -> Result<Axes, PlotError> {
    let ax = fig.subplot();
    ax.set_xlim(-1.0, 2.0)?.set_ylim(-1.0, 2.0)?;
    Ok(ax)
}

/// `test_patch_alpha_coloring`: two cut-out stars as one path collection
/// with translucent faces and dash-dotted edges.
fn patch_alpha_coloring(fig: &Figure) -> CaseResult {
    let (star1, star2) = cut_stars()?;
    let props = CollectionProps::default()
        .linewidth(5.0)
        .linestyle(LineStyle::DashDot)
        .facecolor(FACE)
        .edgecolor(EDGE);
    alpha_axes(fig)?.add_collection(CollectionSpec::paths(vec![star1, star2]).with_props(props));
    Ok(())
}

/// The `fig_ref` side of `test_patch_alpha_coloring`: the same stars as
/// two path patches.
fn patch_alpha_coloring_ref(fig: &Figure) -> CaseResult {
    let (star1, star2) = cut_stars()?;
    let ax = alpha_axes(fig)?;
    let props = PatchProps::default()
        .linewidth(5.0)
        .linestyle(LineStyle::DashDot)
        .facecolor(FACE)
        .edgecolor(EDGE);
    for star in [star1, star2] {
        ax.add_patch_with(PathShape(star), props.clone());
    }
    Ok(())
}

/// The group's plotting cases.
pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_rest/sankey3", sankey3),
    FigureCase::new(
        "mpl_rest/overriding_units_in_plot",
        overriding_units_in_plot,
    ),
    classic("mpl_rest/animation_frame", animation_frame),
    classic("mpl_rest/dict_unpack_scatter", |fig| {
        dict_unpack(fig, Plotter::Scatter)
    }),
    classic("mpl_rest/dict_unpack_bar", |fig| {
        dict_unpack(fig, Plotter::Bar)
    }),
    classic("mpl_rest/dict_unpack_plot", |fig| {
        dict_unpack(fig, Plotter::Plot)
    }),
    classic("mpl_rest/patch_alpha_coloring", patch_alpha_coloring),
    classic(
        "mpl_rest/patch_alpha_coloring_ref",
        patch_alpha_coloring_ref,
    ),
];
