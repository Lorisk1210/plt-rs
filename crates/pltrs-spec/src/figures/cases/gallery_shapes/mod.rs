//! `figures/cases_gallery_shapes.py`: matplotlib's gallery sections
//! "Shapes and collections", "Style sheets" and "`axes_grid1`", one case per
//! figure of each example.

mod anchored;
mod boxes;
mod collections;
mod dividers;
mod grids;
mod hatches;
mod insets;
mod paths;
mod shapes;
mod sheet_reference;
mod sheets;

use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// The examples' `np.random.seed(19680801)`.
const SEED: u64 = 19_680_801;

/// The extent the examples show the sample data in.
const EXTENT: [f64; 4] = [-3.0, 4.0, -4.0, 3.0];

/// `np.random.beta(a, b, size=n)` for integer `a` and `b`: the ratio
/// `x / (x + y)` of gamma draws of shapes `a` and `b`.
fn beta(rng: &mut SpecRng, a: u32, b: u32, n: usize) -> Vec<f64> {
    let x = rng.gamma(n, a);
    let y = rng.gamma(n, b);
    x.iter().zip(&y).map(|(x, y)| x / (x + y)).collect()
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        anchored::CASES,
        boxes::CASES,
        collections::CASES,
        dividers::CASES,
        grids::CASES,
        hatches::CASES,
        insets::CASES,
        paths::CASES,
        shapes::CASES,
        sheet_reference::CASES,
        sheets::CASES,
    ]
    .concat()
}
