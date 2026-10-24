//! Colormaps and colour names: every built-in colormap as a gradient,
//! colormaps made from a list of colours and from segment data, and the
//! tables of named colours (`colormap_reference`, `custom_cmap` and
//! `named_colors`).

use std::cmp::Ordering;
use std::f64::consts::PI;

use plt::axes::ImshowOptions;
use plt::colorbar::ColorbarOptions;
use plt::colors::{ColorTable, Colormap, Segment, SegmentData, colormaps, rgb_to_hsv};
use plt::gridspec::SubplotOverrides;
use plt::image::Origin;
use plt::numeric::Grid;
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{HAlign, VAlign};

use crate::figures::{CaseResult, FigureCase};

/// The colormaps of each figure of `colormap_reference`, by category.
const COLORMAPS: [(&str, &[&str]); 8] = [
    (
        "Perceptually Uniform Sequential",
        &["viridis", "plasma", "inferno", "magma", "cividis"],
    ),
    (
        "Sequential",
        &[
            "Greys", "Purples", "Blues", "Greens", "Oranges", "Reds", "YlOrBr", "YlOrRd", "OrRd",
            "PuRd", "RdPu", "BuPu", "GnBu", "PuBu", "YlGnBu", "PuBuGn", "BuGn", "YlGn",
        ],
    ),
    (
        "Sequential (2)",
        &[
            "gray",
            "bone",
            "pink",
            "spring",
            "summer",
            "autumn",
            "winter",
            "cool",
            "Wistia",
            "hot",
            "afmhot",
            "gist_heat",
            "copper",
        ],
    ),
    (
        "Diverging",
        &[
            "PiYG", "PRGn", "BrBG", "PuOr", "RdGy", "RdBu", "RdYlBu", "RdYlGn", "Spectral",
            "coolwarm", "bwr", "seismic", "berlin", "managua", "vanimo",
        ],
    ),
    ("Cyclic", &["twilight", "twilight_shifted", "hsv"]),
    (
        "Qualitative",
        &[
            "Pastel1",
            "Pastel2",
            "Paired",
            "Accent",
            "okabe_ito",
            "Dark2",
            "Set1",
            "Set2",
            "Set3",
            "tab10",
            "tab20",
            "tab20b",
            "tab20c",
        ],
    ),
    (
        "Miscellaneous",
        &[
            "flag",
            "prism",
            "ocean",
            "gist_earth",
            "terrain",
            "gist_stern",
            "gnuplot",
            "gnuplot2",
            "CMRmap",
            "cubehelix",
            "brg",
            "gist_rainbow",
            "rainbow",
            "jet",
            "turbo",
            "nipy_spectral",
            "gist_ncar",
        ],
    ),
    ("Original and reversed ", &["viridis", "viridis_r"]),
];

/// The height in inches of the figure of category `index`: room for the
/// title and one strip per colormap.
const fn gradients_height(index: usize) -> f64 {
    let nrows = COLORMAPS[index].1.len() as f64;
    0.35 + 0.15 + (nrows + (nrows - 1.0) * 0.1) * 0.22
}

/// `plot_color_gradients`: one horizontal gradient strip per colormap of
/// category `index`, named on its left.
fn color_gradients(fig: &Figure, index: usize) -> CaseResult {
    let (category, names) = COLORMAPS[index];
    let figh = gradients_height(index);
    let ramp = linspace(0.0, 1.0, 256);
    let gradient = Grid::from_fn(2, ramp.len(), |_, col| ramp[col]);
    let axs = fig.subplots(names.len(), 1)?;
    fig.subplots_adjust(&SubplotOverrides {
        top: Some(1.0 - 0.35 / figh),
        bottom: Some(0.15 / figh),
        left: Some(0.2),
        right: Some(0.99),
        ..SubplotOverrides::default()
    })?;
    axs[(0, 0)]
        .set_title(format!("{category} colormaps"))
        .set_fontsize(14.0);
    for (ax, &name) in axs.as_slice().iter().zip(names) {
        ax.imshow(
            gradient.clone(),
            ImshowOptions {
                aspect: Some(Aspect::Auto),
                ..ImshowOptions::default().cmap(colormaps::get(name)?)
            },
        )?;
        ax.text(-0.01, 0.5, name)
            .set_valign(VAlign::Center)
            .set_halign(HAlign::Right)
            .set_fontsize(10.0)
            .set_transform(ax.trans_axes());
    }
    for ax in axs.as_slice() {
        ax.set_axis_off();
    }
    Ok(())
}

/// `cos(x)·sin(y)·10` on the grid of `custom_cmap`.
fn waves() -> Grid<f64> {
    let x = arange(0.0, PI, 0.1);
    let y = arange(0.0, 2.0 * PI, 0.1);
    Grid::from_fn(y.len(), x.len(), |row, col| {
        x[col].cos() * y[row].sin() * 10.0
    })
}

/// The 2 × 2 axes of `custom_cmap`, spread over the figure.
fn custom_cmap_grid(fig: &Figure) -> Result<Vec<Axes>, Box<dyn std::error::Error>> {
    let axs = fig.subplots(2, 2)?;
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.02),
        bottom: Some(0.06),
        right: Some(0.95),
        top: Some(0.94),
        wspace: Some(0.05),
        ..SubplotOverrides::default()
    })?;
    Ok(axs.as_slice().to_vec())
}

fn custom_cmap_1(fig: &Figure) -> CaseResult {
    let z = waves();
    let colors = vec![(1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)];
    for (ax, n_bin) in custom_cmap_grid(fig)?.iter().zip([3, 6, 10, 100]) {
        let cmap = Colormap::from_list("my_list", colors.clone(), n_bin, 1.0)?;
        let image = ax.imshow(
            z.clone(),
            ImshowOptions {
                origin: Some(Origin::Lower),
                ..ImshowOptions::default().cmap(cmap)
            },
        )?;
        ax.set_title(format!("N bins: {n_bin}"));
        fig.colorbar(&image, ColorbarOptions::beside(ax))?;
    }
    Ok(())
}

/// A channel through anchor rows `(x, y_below, y_above)`.
fn anchors(rows: &[[f64; 3]]) -> Segment {
    Segment::Anchors(rows.to_vec())
}

/// `cdict1`: blue to dark to red.
fn blue_red_1() -> SegmentData {
    SegmentData::new(
        anchors(&[[0.0, 0.0, 0.0], [0.5, 0.0, 0.1], [1.0, 1.0, 1.0]]),
        anchors(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]),
        anchors(&[[0.0, 0.0, 1.0], [0.5, 0.1, 0.0], [1.0, 0.0, 0.0]]),
    )
}

/// `cdict2`: blue to red, jumping to full colour at the middle.
fn blue_red_2() -> SegmentData {
    SegmentData::new(
        anchors(&[[0.0, 0.0, 0.0], [0.5, 0.0, 1.0], [1.0, 0.1, 1.0]]),
        anchors(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]),
        anchors(&[[0.0, 0.0, 0.1], [0.5, 1.0, 0.0], [1.0, 0.0, 0.0]]),
    )
}

/// `cdict3`: dark blue through white to dark red.
fn blue_red_3() -> SegmentData {
    SegmentData::new(
        anchors(&[
            [0.0, 0.0, 0.0],
            [0.25, 0.0, 0.0],
            [0.5, 0.8, 1.0],
            [0.75, 1.0, 1.0],
            [1.0, 0.4, 1.0],
        ]),
        anchors(&[
            [0.0, 0.0, 0.0],
            [0.25, 0.0, 0.0],
            [0.5, 0.9, 0.9],
            [0.75, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ]),
        anchors(&[
            [0.0, 0.0, 0.4],
            [0.25, 1.0, 1.0],
            [0.5, 1.0, 0.8],
            [0.75, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ]),
    )
}

/// A colormap of 256 entries from segment data.
fn segmented(name: &str, data: SegmentData) -> Result<Colormap, plt::colors::ColormapError> {
    Colormap::segmented(name, data, 256, 1.0)
}

fn custom_cmap_2(fig: &Figure) -> CaseResult {
    let z = waves();
    let blue_red3 = segmented("BlueRed3", blue_red_3())?;
    // `cdict4`: `cdict3` turning translucent towards the middle.
    let blue_red_alpha = segmented(
        "BlueRedAlpha",
        blue_red_3().with_alpha(anchors(&[
            [0.0, 1.0, 1.0],
            [0.5, 0.3, 0.3],
            [1.0, 1.0, 1.0],
        ])),
    )?;
    let axs = custom_cmap_grid(fig)?;
    let shown = |ax: &Axes, cmap: Colormap| -> Result<_, Box<dyn std::error::Error>> {
        let image = ax.imshow(z.clone(), ImshowOptions::default().cmap(cmap))?;
        fig.colorbar(&image, ColorbarOptions::beside(ax))?;
        Ok(image)
    };
    shown(&axs[0], segmented("BlueRed1", blue_red_1())?)?;
    shown(&axs[2], segmented("BlueRed2", blue_red_2())?)?;
    // matplotlib draws these two with `image.cmap` set to `BlueRed3`.
    shown(&axs[1], blue_red3.clone())?;
    axs[1].set_title("Alpha = 1");
    axs[3].plot_with(
        [0.0, 10.0 * PI],
        [0.0, 20.0 * PI],
        LineProps::default().color("c").linewidth(20.0).zorder(-1.0),
    )?;
    shown(&axs[3], blue_red3)?.set_cmap(blue_red_alpha);
    axs[3].set_title("Varying alpha");
    fig.suptitle("Custom Blue-Red colormaps").set_fontsize(16.0);
    fig.subplots_adjust(&SubplotOverrides {
        top: Some(0.9),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// `plot_colortable`'s layout in pixels at 72 dpi: cells, swatches and
/// the figure margin.
const CELL_WIDTH: f64 = 212.0;
const CELL_HEIGHT: f64 = 22.0;
const SWATCH_WIDTH: f64 = 48.0;
const MARGIN: f64 = 12.0;

/// Rows of a colour table of `n` colours in `ncols` columns.
const fn colortable_rows(n: usize, ncols: usize) -> usize {
    n.div_ceil(ncols)
}

/// The size in pixels of a colour table of `n` colours in `ncols`
/// columns.
const fn colortable_size(n: usize, ncols: usize) -> (f64, f64) {
    let rows = colortable_rows(n, ncols) as f64;
    (
        CELL_WIDTH * ncols as f64 + 2.0 * MARGIN,
        CELL_HEIGHT * rows + 2.0 * MARGIN,
    )
}

/// The colour tables of `named_colors`: the table, its number of colours,
/// the columns it is shown in and whether it is sorted by hue.
const TABLES: [(ColorTable, usize, usize, bool); 3] = [
    (ColorTable::Base, 8, 3, false),
    (ColorTable::Tableau, 10, 2, false),
    (ColorTable::Css4, 148, 4, true),
];

/// `plot_colortable` of table `index` of [`TABLES`]: a swatch and the
/// name of each colour (as matplotlib stores them), in table order or by
/// hue, saturation and value.
fn colortable(fig: &Figure, index: usize) -> CaseResult {
    let (table, _, ncols, sort) = TABLES[index];
    let mut colors: Vec<(&str, Color)> = match table {
        // The base colours are RGB triples, the others hex codes.
        ColorTable::Base => table
            .entries()
            .into_iter()
            .map(|(name, rgba)| (name, Color::from(rgba)))
            .collect(),
        _ => table
            .hex_entries()
            .iter()
            .map(|&(name, hex)| (name, Color::from(hex)))
            .collect(),
    };
    if sort {
        let mut keyed = colors
            .into_iter()
            .map(|(name, color)| {
                let rgba = color.resolve()?;
                Ok((rgb_to_hsv([rgba.r, rgba.g, rgba.b])?, name, color))
            })
            .collect::<Result<Vec<_>, plt::colors::ColorError>>()?;
        // A stable sort: colours of equal hue, saturation and value keep
        // their order.
        keyed.sort_by(|(a, ..), (b, ..)| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        colors = keyed
            .into_iter()
            .map(|(_, name, color)| (name, color))
            .collect();
    }
    let nrows = colortable_rows(colors.len(), ncols);
    let (width, height) = colortable_size(colors.len(), ncols);
    let ax = fig.subplot();
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(MARGIN / width),
        bottom: Some(MARGIN / height),
        right: Some((width - MARGIN) / width),
        top: Some((height - MARGIN) / height),
        ..SubplotOverrides::default()
    })?;
    ax.set_xlim(0.0, CELL_WIDTH * ncols as f64)?;
    ax.set_ylim(CELL_HEIGHT * (nrows as f64 - 0.5), -CELL_HEIGHT / 2.0)?;
    ax.yaxis().set_visible(false);
    ax.xaxis().set_visible(false);
    ax.set_axis_off();
    for (i, (name, color)) in colors.into_iter().enumerate() {
        let (row, col) = ((i % nrows) as f64, (i / nrows) as f64);
        let y = row * CELL_HEIGHT;
        let swatch_start_x = CELL_WIDTH * col;
        let text_pos_x = CELL_WIDTH * col + SWATCH_WIDTH + 7.0;
        ax.text(text_pos_x, y, name)
            .set_fontsize(14.0)
            .set_halign(HAlign::Left)
            .set_valign(VAlign::Center);
        ax.add_patch_with(
            Rectangle::new((swatch_start_x, y - 9.0), SWATCH_WIDTH, 18.0),
            PatchProps::default().facecolor(color).edgecolor("0.7"),
        );
    }
    Ok(())
}

/// The case showing table `index` of [`TABLES`] (built by `build`), on a
/// canvas of the table's size at 72 dpi.
const fn colortable_case(
    name: &'static str,
    build: fn(&Figure) -> CaseResult,
    index: usize,
) -> FigureCase {
    let (_, n, ncols, _) = TABLES[index];
    let (width, height) = colortable_size(n, ncols);
    FigureCase::new(name, build).sized(width / 72.0, height / 72.0, 72.0)
}

/// The case showing category `index` of [`COLORMAPS`] (built by `build`),
/// on a canvas as tall as its strips need.
const fn gradients_case(
    name: &'static str,
    build: fn(&Figure) -> CaseResult,
    index: usize,
) -> FigureCase {
    FigureCase::new(name, build).sized(6.4, gradients_height(index), 100.0)
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    gradients_case(
        "gallery_ticks/colormap_reference_1",
        |f| color_gradients(f, 0),
        0,
    ),
    gradients_case(
        "gallery_ticks/colormap_reference_2",
        |f| color_gradients(f, 1),
        1,
    ),
    gradients_case(
        "gallery_ticks/colormap_reference_3",
        |f| color_gradients(f, 2),
        2,
    ),
    gradients_case(
        "gallery_ticks/colormap_reference_4",
        |f| color_gradients(f, 3),
        3,
    ),
    gradients_case(
        "gallery_ticks/colormap_reference_5",
        |f| color_gradients(f, 4),
        4,
    ),
    gradients_case(
        "gallery_ticks/colormap_reference_6",
        |f| color_gradients(f, 5),
        5,
    ),
    gradients_case(
        "gallery_ticks/colormap_reference_7",
        |f| color_gradients(f, 6),
        6,
    ),
    gradients_case(
        "gallery_ticks/colormap_reference_8",
        |f| color_gradients(f, 7),
        7,
    ),
    FigureCase::new("gallery_ticks/custom_cmap_1", custom_cmap_1).sized(6.0, 9.0, 100.0),
    FigureCase::new("gallery_ticks/custom_cmap_2", custom_cmap_2).sized(6.0, 9.0, 100.0),
    colortable_case("gallery_ticks/named_colors_1", |fig| colortable(fig, 0), 0),
    colortable_case("gallery_ticks/named_colors_2", |fig| colortable(fig, 1), 1),
    colortable_case("gallery_ticks/named_colors_3", |fig| colortable(fig, 2), 2),
];
