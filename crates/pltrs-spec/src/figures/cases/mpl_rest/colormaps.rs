//! The cases from `test_multivariate_colormaps.py`: colours of value
//! pairs shown as RGBA images.

use std::error::Error;

use pltrs::axes::ImshowOptions;
use pltrs::colors::multivar::{
    BivarShape, CombinationMode, MultivarColormap, MultivarError, bivar_colormap,
};
use pltrs::colors::{Colormap, Normalized, colormaps};
use pltrs::figure::Figure;
use pltrs::image::Interpolation;
use pltrs::numeric::{Grid, linspace};
use pltrs_core::Rgba;

use super::{classic, remove_text};
use crate::figures::{CaseResult, FigureCase};

/// The `size × size` RGBA image whose pixel `(row, col)` has the colour
/// `color(row, col)`.
fn image(
    size: usize,
    color: impl Fn(usize, usize) -> Result<Rgba, MultivarError>,
) -> Result<Grid<[f64; 4]>, Box<dyn Error>> {
    let pixels = (0..size * size)
        .map(|i| color(i / size, i % size).map(Rgba::to_array))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Grid::from_vec(size, size, pixels)?)
}

/// `imshow` options of both cases: nearest-neighbour interpolation.
fn nearest() -> ImshowOptions {
    ImshowOptions {
        interpolation: Some(Interpolation::Nearest),
        ..ImshowOptions::default()
    }
}

/// `test_bivariate_cmap_shapes`: the square `BiPeak` and circular
/// `BiCone` colormaps over a grid reaching beyond the unit square, as
/// they are and ignoring what lies outside their shape.
fn bivariate_cmap_shapes(fig: &Figure) -> CaseResult {
    // `np.linspace(-0.1, 1.1, 10, dtype="float32")`.
    let values: Vec<f64> = linspace(-0.1, 1.1, 10)
        .iter()
        .map(|&v| f64::from(v as f32))
        .collect();
    let shapes = [
        ("BiPeak", BivarShape::Square),
        ("BiCone", BivarShape::Circle),
        ("BiPeak", BivarShape::Ignore),
        ("BiCone", BivarShape::CircleIgnore),
    ];
    let axes = fig.subplots(1, 4)?;
    for (ax, (name, shape)) in axes.as_slice().iter().zip(shapes) {
        let cmap = bivar_colormap(name)?.with_extremes(None, None, Some(shape), None)?;
        let pixels = image(values.len(), |row, col| {
            let fraction = |i: usize| Normalized::Fraction(values[i]);
            cmap.lookup(fraction(col), fraction(row), None)
        })?;
        ax.imshow(pixels, nearest())?;
    }
    remove_text(fig);
    Ok(())
}

/// `test_multivar_alpha_mixing`: `rainbow` added to a colormap fading
/// from opaque to transparent black.
fn multivar_alpha_mixing(fig: &Figure) -> CaseResult {
    let fading: Vec<Rgba> = linspace(1.0, 0.0, 256)
        .into_iter()
        .map(|alpha| Rgba::new(0.0, 0.0, 0.0, alpha))
        .collect();
    let alpha_cmap = Colormap::from_list("from_list", fading, 256, 1.0)?;
    let cmap = MultivarColormap::new(
        "multivariate colormap",
        vec![colormaps::get("rainbow")?, alpha_cmap],
        CombinationMode::Add,
    )?;
    let pixels = image(10, |row, col| {
        let pair = [row, col].map(|i| Normalized::Fraction(i as f64 / 9.0));
        cmap.lookup(&pair, None)
    })?;
    fig.subplot().imshow(pixels, nearest())?;
    remove_text(fig);
    Ok(())
}

/// The group's colormap cases.
pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_rest/bivariate_cmap_shapes", bivariate_cmap_shapes).sized(10.0, 2.0, 100.0),
    classic("mpl_rest/multivar_alpha_mixing", multivar_alpha_mixing),
];
