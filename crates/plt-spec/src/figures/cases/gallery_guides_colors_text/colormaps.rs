//! The colormap reference of `colormaps`: for each category of colormaps,
//! a strip of every colormap, the lightness of its colours (see
//! [`super::lightness`]) and the colormap beside the grey of its lightness.

use plt::axes::ImshowOptions;
use plt::colors::colormaps;
use plt::gridspec::SubplotOverrides;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{HAlign, VAlign};

use super::lightness::{grayscale, lightness};
use crate::figures::{CaseResult, FigureCase};

/// A category of the guide's colormaps, with how many of them one
/// lightness subplot shows and how far apart (`_DSUBS`, `_DC`).
pub(super) struct Category {
    /// The category's name.
    pub(super) name: &'static str,
    /// Its colormaps.
    pub(super) cmaps: &'static [&'static str],
    /// Colormaps per lightness subplot.
    pub(super) per_subplot: usize,
    /// Horizontal spacing of the colormaps in a lightness subplot.
    pub(super) spacing: f64,
}

impl Category {
    /// The number of lightness subplots.
    pub(super) const fn subplots(&self) -> usize {
        self.cmaps.len().div_ceil(self.per_subplot)
    }
}

/// The guide's categories of colormaps, in its order (`cmaps`).
pub(super) const CATEGORIES: [Category; 7] = [
    Category {
        name: "Perceptually Uniform Sequential",
        cmaps: &["viridis", "plasma", "inferno", "magma", "cividis"],
        per_subplot: 5,
        spacing: 1.4,
    },
    Category {
        name: "Sequential",
        cmaps: &[
            "Greys", "Purples", "Blues", "Greens", "Oranges", "Reds", "YlOrBr", "YlOrRd", "OrRd",
            "PuRd", "RdPu", "BuPu", "GnBu", "PuBu", "YlGnBu", "PuBuGn", "BuGn", "YlGn",
        ],
        per_subplot: 6,
        spacing: 0.7,
    },
    Category {
        name: "Sequential (2)",
        cmaps: &[
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
        per_subplot: 6,
        spacing: 1.4,
    },
    Category {
        name: "Diverging",
        cmaps: &[
            "PiYG", "PRGn", "BrBG", "PuOr", "RdGy", "RdBu", "RdYlBu", "RdYlGn", "Spectral",
            "coolwarm", "bwr", "seismic", "berlin", "managua", "vanimo",
        ],
        per_subplot: 6,
        spacing: 1.4,
    },
    Category {
        name: "Cyclic",
        cmaps: &["twilight", "twilight_shifted", "hsv"],
        per_subplot: 3,
        spacing: 1.4,
    },
    Category {
        name: "Qualitative",
        cmaps: &[
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
        per_subplot: 4,
        spacing: 1.4,
    },
    Category {
        name: "Miscellaneous",
        cmaps: &[
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
        per_subplot: 6,
        spacing: 1.4,
    },
];

/// The two rows of 256 values from 0 to 1 every strip shows (`gradient`).
pub(super) fn gradient() -> Grid<f64> {
    let row = linspace(0.0, 1.0, 256);
    Grid::from_fn(2, 256, |_, c| row[c])
}

/// `imshow(gradient, aspect='auto', cmap=name)`.
pub(super) fn strip(ax: &Axes, name: &str) -> CaseResult {
    let options = ImshowOptions::default().cmap(colormaps::get(name)?);
    ax.imshow(
        gradient(),
        ImshowOptions {
            aspect: Some(Aspect::Auto),
            ..options
        },
    )?;
    Ok(())
}

/// The height of the gradients figure of `nrows` colormaps.
const fn gradients_height(nrows: usize) -> f64 {
    let n = nrows as f64;
    0.35 + 0.15 + (n + (n - 1.0) * 0.1) * 0.22
}

/// The first `plot_color_gradients`: a strip of each colormap of the
/// category, named on its left.
fn plot_color_gradients(fig: &Figure, category: &Category) -> CaseResult {
    let nrows = category.cmaps.len();
    let figh = gradients_height(nrows);
    let axs = fig.subplots(nrows + 1, 1)?;
    fig.subplots_adjust(&SubplotOverrides {
        top: Some(1.0 - 0.35 / figh),
        bottom: Some(0.15 / figh),
        left: Some(0.2),
        right: Some(0.99),
        ..SubplotOverrides::default()
    })?;
    axs[(0, 0)]
        .set_title(format!("{} colormaps", category.name))
        .set_fontsize(14.0);
    for (ax, name) in axs.as_slice().iter().zip(category.cmaps) {
        strip(ax, name)?;
        ax.text(-0.01, 0.5, *name)
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

fn gradients<const C: usize>(fig: &Figure) -> CaseResult {
    plot_color_gradients(fig, &CATEGORIES[C])
}

fn lightness_case<const C: usize>(fig: &Figure) -> CaseResult {
    lightness(fig, &CATEGORIES[C])
}

fn grayscale_case<const C: usize>(fig: &Figure) -> CaseResult {
    grayscale(fig, &CATEGORIES[C])
}

/// The three cases of the category `$c`: its gradients, lightness and
/// grayscale figures, `colormaps_$g`, `colormaps_$l` and `colormaps_$s`.
macro_rules! category_cases {
    ($c:literal: $g:literal, $l:literal, $s:literal) => {
        [
            FigureCase::new(
                concat!("gallery_guides_colors_text/colormaps_", $g),
                gradients::<$c>,
            )
            .sized(6.4, gradients_height(CATEGORIES[$c].cmaps.len()), 100.0),
            FigureCase::new(
                concat!("gallery_guides_colors_text/colormaps_", $l),
                lightness_case::<$c>,
            )
            .sized(7.0, 2.6 * CATEGORIES[$c].subplots() as f64, 100.0)
            .with_rc(&[("font.size", "12")]),
            FigureCase::new(
                concat!("gallery_guides_colors_text/colormaps_", $s),
                grayscale_case::<$c>,
            )
            .with_rc(&[("font.size", "14")]),
        ]
    };
}

/// The cases of this module.
pub(super) fn cases() -> Vec<FigureCase> {
    [
        category_cases!(0: 1, 8, 15),
        category_cases!(1: 2, 9, 16),
        category_cases!(2: 3, 10, 17),
        category_cases!(3: 4, 11, 18),
        category_cases!(4: 5, 12, 19),
        category_cases!(5: 6, 13, 20),
        category_cases!(6: 7, 14, 21),
    ]
    .concat()
}
