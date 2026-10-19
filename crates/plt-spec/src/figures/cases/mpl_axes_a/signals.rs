//! Autocorrelation of a random walk and of integer data, sparsity patterns
//! (`spy`) as an image and as markers, and `matshow`.

use plt::axes::{ImshowOptions, SpyOptions};
use plt::image::Origin;
use plt::mlab::XcorrOptions;
use plt::numeric::Grid;
use plt::prelude::*;

use crate::figures::CaseResult;
use crate::figures::cases::common::cumsum;
use crate::rng::SpecRng;

/// The seed the tests give `np.random.seed`.
const SEED: u64 = 19_680_801;

/// `ax.acorr(x, maxlags=len(x) - 1)` on a new subplot.
fn acorr_all_lags(fig: &Figure, x: &[f64]) -> CaseResult {
    let options = XcorrOptions {
        maxlags: Some(x.len() - 1),
        ..XcorrOptions::default()
    };
    fig.subplot()
        .acorr(x, &options, true, LineProps::default())?;
    Ok(())
}

pub(super) fn acorr(fig: &Figure) -> CaseResult {
    acorr_all_lags(fig, &cumsum(&SpecRng::new(SEED).normal(512, 0.0, 1.0)))
}

pub(super) fn acorr_integers(fig: &Figure) -> CaseResult {
    let steps = SpecRng::new(SEED).uniform(51, 0.0, 10.0);
    let x: Vec<f64> = cumsum(&steps).into_iter().map(f64::ceil).collect();
    acorr_all_lags(fig, &x)
}

pub(super) fn spy(fig: &Figure) -> CaseResult {
    let cells = SpecRng::new(SEED).uniform(32 * 32, 0.0, 1.0);
    let a = Grid::from_vec(32, 32, cells)?.map(|&u| f64::from(u8::from(u < 0.5)));
    let axs = fig.subplots(2, 1)?;
    axs[(0, 0)].spy(&a, &SpyOptions::default())?;
    let markers = SpyOptions {
        marker: Some(Marker::Symbol(MarkerSymbol::Point)),
        origin: Origin::Lower,
        ..SpyOptions::default()
    };
    axs[(1, 0)].spy(&a, &markers)?;
    Ok(())
}

pub(super) fn matshow(fig: &Figure) -> CaseResult {
    let cells = SpecRng::new(SEED).uniform(32 * 32, 0.0, 1.0);
    fig.subplot()
        .matshow(Grid::from_vec(32, 32, cells)?, ImshowOptions::default())?;
    Ok(())
}
