//! Making colormaps: listed colormaps from names, edited, cut and joined
//! colour tables, segmented colormaps from anchor rows and from lists of
//! colours, reversed colormaps, and colormaps registered under a name
//! (`colormap-manipulation`).

use pltrs::axes::{ImshowOptions, Mesh, PcolorOptions};
use pltrs::colorbar::ColorbarOptions;
use pltrs::colors::{ColorizerOptions, Colormap, Segment, SegmentData, colormaps};
use pltrs::layout::LayoutEngine;
use pltrs::numeric::Grid;
use pltrs::prelude::*;
use pltrs_core::Rgba;

use crate::figures::cases::common::GALLERY_SEED;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The colours of the tutorial's first colormaps.
const ORANGE_TO_TEAL: [&str; 4] = ["darkorange", "gold", "lawngreen", "lightseagreen"];

/// `plot_examples`: the same random field under each colormap, each with a
/// colorbar (the cases give the figure its `2 * n + 2` by 3 in size).
fn plot_examples(fig: &Figure, cmaps: Vec<Colormap>) -> CaseResult {
    let data = Grid::from_vec(30, 30, SpecRng::new(GALLERY_SEED).normal(900, 0.0, 1.0))?;
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, cmaps.len())?;
    for (ax, cmap) in axs.as_slice().iter().zip(cmaps) {
        let options = PcolorOptions {
            colorizer: ColorizerOptions::default()
                .cmap(cmap)
                .limits(Some(-4.0), Some(4.0)),
            ..PcolorOptions::default()
        };
        let psm = ax.pcolormesh(Mesh::new(data.clone()), options)?;
        psm.set_rasterized(true);
        fig.colorbar(&psm, ColorbarOptions::beside(ax))?;
    }
    Ok(())
}

/// A listed colormap of `colors` (`ListedColormap(colors)`).
fn listed(colors: Vec<Rgba>) -> Result<Colormap, Box<dyn std::error::Error>> {
    Ok(Colormap::listed("from_list", colors)?)
}

/// `cmap(np.linspace(start, stop, n))`.
fn samples(cmap: &Colormap, start: f64, stop: f64, n: usize) -> Vec<Rgba> {
    linspace(start, stop, n)
        .into_iter()
        .map(|x| cmap.sample(x))
        .collect()
}

/// viridis with 256 entries (`colormaps['viridis'].resampled(256)`).
fn viridis() -> Result<Colormap, Box<dyn std::error::Error>> {
    Ok(colormaps::get("viridis")?.resampled(256)?)
}

fn colormap_manipulation_1(fig: &Figure) -> CaseResult {
    plot_examples(
        fig,
        vec![Colormap::listed("from_list", ORANGE_TO_TEAL.to_vec())?],
    )
}

fn colormap_manipulation_2(fig: &Figure) -> CaseResult {
    let viridis = viridis()?;
    let mut newcolors = samples(&viridis, 0.0, 1.0, 256);
    let pink = Rgba::new(248.0 / 256.0, 24.0 / 256.0, 148.0 / 256.0, 1.0);
    newcolors[..25].fill(pink);
    plot_examples(fig, vec![viridis, listed(newcolors)?])
}

fn colormap_manipulation_3(fig: &Figure) -> CaseResult {
    let viridis_big = colormaps::get("viridis")?;
    let newcmp = listed(samples(&viridis_big, 0.25, 0.75, 128))?;
    plot_examples(fig, vec![viridis()?, newcmp])
}

fn colormap_manipulation_4(fig: &Figure) -> CaseResult {
    let top = colormaps::get("Oranges_r")?.resampled(128)?;
    let bottom = colormaps::get("Blues")?.resampled(128)?;
    let mut newcolors = samples(&top, 0.0, 1.0, 128);
    newcolors.extend(samples(&bottom, 0.0, 1.0, 128));
    let newcmp = Colormap::listed("OrangeBlue", newcolors)?;
    plot_examples(fig, vec![viridis()?, newcmp])
}

fn colormap_manipulation_5(fig: &Figure) -> CaseResult {
    let n = 256;
    let channel = |start: f64| linspace(start / 256.0, 1.0, n);
    let (red, green, blue) = (channel(90.0), channel(40.0), channel(40.0));
    let vals = (0..n)
        .map(|i| Rgba::new(red[i], green[i], blue[i], 1.0))
        .collect();
    plot_examples(fig, vec![viridis()?, listed(vals)?])
}

/// The tutorial's segment data: red rising to 1 at the middle, green from
/// a quarter to three quarters, blue over the upper half.
fn cdict(red: Vec<[f64; 3]>) -> SegmentData {
    SegmentData::new(
        Segment::Anchors(red),
        Segment::Anchors(vec![
            [0.0, 0.0, 0.0],
            [0.25, 0.0, 0.0],
            [0.75, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ]),
        Segment::Anchors(vec![[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [1.0, 1.0, 1.0]]),
    )
}

/// `plot_linearmap`: the red, green and blue channels of the segmented
/// colormap `data` describes.
fn plot_linearmap(fig: &Figure, data: SegmentData) -> CaseResult {
    let newcmp = Colormap::segmented("testCmap", data, 256, 1.0)?;
    let rgba = samples(&newcmp, 0.0, 1.0, 256);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    for xx in [0.25, 0.5, 0.75] {
        ax.axvline(
            xx,
            LineProps::default()
                .color("0.7")
                .linestyle(LineStyle::Dashed),
        );
    }
    let index: Vec<f64> = (0..256).map(|i| f64::from(i) / 256.0).collect();
    for (channel, color) in ["r", "g", "b"].into_iter().enumerate() {
        let values: Vec<f64> = rgba.iter().map(|c| c.to_array()[channel]).collect();
        ax.plot_with(&index, values, LineProps::default().color(color))?;
    }
    ax.set_xlabel("index");
    ax.set_ylabel("RGB");
    Ok(())
}

fn colormap_manipulation_6(fig: &Figure) -> CaseResult {
    plot_linearmap(
        fig,
        cdict(vec![[0.0, 0.0, 0.0], [0.5, 1.0, 1.0], [1.0, 1.0, 1.0]]),
    )
}

fn colormap_manipulation_7(fig: &Figure) -> CaseResult {
    plot_linearmap(
        fig,
        cdict(vec![[0.0, 0.0, 0.3], [0.5, 1.0, 0.9], [1.0, 1.0, 1.0]]),
    )
}

fn colormap_manipulation_8(fig: &Figure) -> CaseResult {
    let cmap1 = Colormap::from_list("mycmap", ORANGE_TO_TEAL.to_vec(), 256, 1.0)?;
    let nodes = [0.0, 0.4, 0.8, 1.0];
    let cmap2 = Colormap::from_stops("mycmap", nodes.into_iter().zip(ORANGE_TO_TEAL), 256, 1.0)?;
    plot_examples(fig, vec![cmap1, cmap2])
}

/// The tutorial's colormap to reverse and register.
fn my_cmap() -> Result<Colormap, Box<dyn std::error::Error>> {
    let colors = vec!["#ffffcc", "#a1dab4", "#41b6c4", "#2c7fb8", "#253494"];
    Ok(Colormap::listed("my_cmap", colors)?)
}

fn colormap_manipulation_9(fig: &Figure) -> CaseResult {
    let my_cmap = my_cmap()?;
    let my_cmap_r = my_cmap.reversed();
    plot_examples(fig, vec![my_cmap, my_cmap_r])
}

/// The two images of the registered colormaps, by name.
fn registered_images(fig: &Figure) -> CaseResult {
    let data = Grid::from_vec(1, 5, vec![1.0, 2.0, 3.0, 4.0, 5.0])?;
    let axs = fig.subplots(2, 1)?;
    for (ax, name) in axs.as_slice().iter().zip(["my_cmap", "my_cmap_r"]) {
        ax.imshow(
            data.clone(),
            ImshowOptions::default().cmap(colormaps::get(name)?),
        )?;
    }
    Ok(())
}

fn colormap_manipulation_10(fig: &Figure) -> CaseResult {
    let my_cmap = my_cmap()?;
    colormaps::register(my_cmap.reversed())?;
    colormaps::register(my_cmap)?;
    let drawn = registered_images(fig);
    colormaps::unregister("my_cmap")?;
    colormaps::unregister("my_cmap_r")?;
    drawn
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_1",
        colormap_manipulation_1,
    )
    .sized(4.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_2",
        colormap_manipulation_2,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_3",
        colormap_manipulation_3,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_4",
        colormap_manipulation_4,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_5",
        colormap_manipulation_5,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_6",
        colormap_manipulation_6,
    )
    .sized(4.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_7",
        colormap_manipulation_7,
    )
    .sized(4.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_8",
        colormap_manipulation_8,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_9",
        colormap_manipulation_9,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_guides_colors_text/colormap_manipulation_10",
        colormap_manipulation_10,
    ),
];
