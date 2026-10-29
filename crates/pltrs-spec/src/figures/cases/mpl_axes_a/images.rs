//! An image of a sheared cone, and the same image clipped by one of its
//! contour lines.

use pltrs::axes::ImshowOptions;
use pltrs::contour::{ContourOptions, Levels, QuadField};
use pltrs::numeric::Grid;
use pltrs::prelude::*;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

/// Side of the cone image.
const N: usize = 100;

/// `_cone()`: `sqrt(x² + y² - xy)` on an N×N integer grid centred on zero.
fn cone() -> Grid<f64> {
    let half = (N / 2) as i64;
    Grid::from_fn(N, N, |i, j| {
        let (x, y) = (i as i64 - half, j as i64 - half);
        ((x * x + y * y - x * y) as f64).sqrt()
    })
}

pub(super) fn imshow(fig: &Figure) -> CaseResult {
    fig.subplot().imshow(cone(), ImshowOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn imshow_clip(fig: &Figure) -> CaseResult {
    let r = cone();
    let ax = fig.subplot();
    let options = ContourOptions {
        levels: Levels::Values(vec![N as f64 / 4.0]),
        ..ContourOptions::default()
    };
    let contours = ax.contour(&QuadField::new(r.clone()), options)?;
    let outline = contours
        .collections()
        .first()
        .and_then(|lines| lines.paths().into_iter().next())
        .ok_or("the contour set has a path")?;
    let clipped = ImshowOptions {
        clip_path: Some((outline, ax.trans_data())),
        ..ImshowOptions::default()
    };
    ax.imshow(r, clipped)?;
    Ok(())
}
