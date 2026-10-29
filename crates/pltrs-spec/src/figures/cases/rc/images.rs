//! The `image.*` and `contour.*` cases.

use pltrs::axes::ImshowOptions;
use pltrs::colors::Color;
use pltrs::contour::{ContourOptions, Levels, QuadCoords, QuadField};
use pltrs::figure::Figure;
use pltrs::numeric::{Grid, linspace, meshgrid};

use super::{SMALL, rc_cases};
use crate::figures::{CaseResult, FigureCase};

/// The outer product `rows[r] * cols[c]` (numpy's
/// `cols[None, :] * rows[:, None]`).
fn outer(rows: &[f64], cols: &[f64]) -> Grid<f64> {
    Grid::from_fn(rows.len(), cols.len(), |r, c| rows[r] * cols[c])
}

/// `_image`: a small array, upsampled.
fn image(fig: &Figure) -> CaseResult {
    let cols: Vec<f64> = linspace(0.0, 3.0, 8).into_iter().map(f64::sin).collect();
    let rows: Vec<f64> = linspace(0.0, 2.0, 5).into_iter().map(f64::cos).collect();
    fig.subplot()
        .imshow(outer(&rows, &cols), ImshowOptions::default())?;
    Ok(())
}

/// `_fine_image`: a fine pattern, downsampled.
fn fine_image(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 6.0, 400);
    let cols: Vec<f64> = x.iter().map(|v| (40.0 * v).sin()).collect();
    let rows: Vec<f64> = x.iter().map(|v| (28.0 * v).cos()).collect();
    fig.subplot()
        .imshow(outer(&rows, &cols), ImshowOptions::default())?;
    Ok(())
}

/// `np.sin(x) * np.cos(y)` on an `n` × `n` grid over [-2, 2]², with its
/// coordinates.
fn wave_field(n: usize) -> QuadField {
    let (x, y) = meshgrid(&linspace(-2.0, 2.0, n), &linspace(-2.0, 2.0, n));
    let z = Grid::from_fn(n, n, |r, c| x[(r, c)].sin() * y[(r, c)].cos());
    QuadField::new(z).with_coords(QuadCoords::Mesh { x, y })
}

/// `_contours`: black contour lines; the negative levels are dashed.
fn contours(fig: &Figure) -> CaseResult {
    let options = ContourOptions {
        levels: Levels::Count(7),
        colors: Some(vec![Color::from("k")]),
        ..ContourOptions::default()
    };
    fig.subplot().contour(&wave_field(30), options)?;
    Ok(())
}

/// `_masked_contours`: filled contours of masked data; the corners next to
/// masked points show.
fn masked_contours(fig: &Figure) -> CaseResult {
    let masked = |r: usize, c: usize| {
        ((3..6).contains(&r) && (4..7).contains(&c)) || (r, c) == (8, 2) || (r, c) == (9, 9)
    };
    let field = wave_field(12).with_mask(Grid::from_fn(12, 12, masked));
    let options = ContourOptions {
        levels: Levels::Count(6),
        ..ContourOptions::default()
    };
    fig.subplot().contourf(&field, options)?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    image, SMALL => {
        "image_aspect": [("image.aspect", "auto")],
        "image_interpolation": [("image.interpolation", "bilinear")],
        "image_interpolation_stage": [
            ("image.interpolation", "bilinear"),
            ("image.interpolation_stage", "rgba"),
        ],
        "image_cmap": [("image.cmap", "plasma")],
        "image_origin": [("image.origin", "lower")],
    }
    fine_image, SMALL => {
        "image_resample": [("image.resample", "False")],
    }
    contours, SMALL => {
        "contour_negative_linestyle": [("contour.negative_linestyle", "dashdot")],
        "contour_linewidth": [("contour.linewidth", "2.5")],
    }
    masked_contours, SMALL => {
        "contour_corner_mask": [("contour.corner_mask", "False")],
        "contour_algorithm": [("contour.algorithm", "mpl2005")],
    }
};
