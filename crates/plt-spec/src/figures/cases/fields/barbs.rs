//! Barbs cases of `figures/cases_fields.py`.

use plt::axes::MappedColors;
use plt::colors::{Colorize, colormaps};
use plt::figure::Figure;
use plt::numeric::{arange, linspace_with, meshgrid};
use plt::quiver::{BarbIncrements, BarbPivot, BarbSizes, BarbsOptions, VectorField};

use crate::figures::{CaseResult, FigureCase};

/// Speeds of the 24 barbs (`_barb_data`).
const SPEEDS: [f64; 24] = [
    0.0, 3.0, 5.0, 10.0, 15.0, 25.0, 35.0, 50.0, 65.0, 80.0, 95.0, 110.0, 120.0, 140.0, 7.0, 12.0,
    22.0, 47.0, 52.0, 57.0, 8.0, 18.0, 28.0, 38.0,
];

/// `_barb_data()`: a 6 × 4 grid of winds of increasing speed turning
/// once around the compass.
fn barb_field() -> VectorField {
    let x = arange(0.0, 6.0, 1.0);
    let (xx, yy) = meshgrid(&x, &x[..4]);
    let angles = linspace_with(0.0, 2.0 * std::f64::consts::PI, 24, false);
    let u: Vec<f64> = SPEEDS
        .iter()
        .zip(&angles)
        .map(|(s, a)| s * a.cos())
        .collect();
    let v: Vec<f64> = SPEEDS
        .iter()
        .zip(&angles)
        .map(|(s, a)| s * a.sin())
        .collect();
    VectorField::new(xx.as_slice(), yy.as_slice(), u, v)
}

/// The barbs of `field` drawn with `options` in the cases' view.
fn barbs(fig: &Figure, field: VectorField, options: BarbsOptions) -> CaseResult {
    let ax = fig.subplot();
    ax.barbs(field, options)?;
    ax.set_xlim(-1.0, 6.0);
    ax.set_ylim(-1.0, 4.0);
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("fields/barbs_default", |fig| {
        barbs(fig, barb_field(), BarbsOptions::default())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("fields/barbs_length_sizes_fill", |fig| {
        let options = BarbsOptions {
            length: 8.0,
            fill_empty: true,
            sizes: BarbSizes {
                emptybarb: 0.3,
                spacing: 0.2,
                height: 0.4,
                width: 0.3,
            },
            ..BarbsOptions::default()
        };
        barbs(fig, barb_field(), options)
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("fields/barbs_flip_colors", |fig| {
        let options = BarbsOptions {
            flip: vec![true],
            barbcolor: Some("tab:red".into()),
            flagcolor: Some("tab:blue".into()),
            pivot: BarbPivot::Middle,
            ..BarbsOptions::default()
        };
        barbs(fig, barb_field(), options)
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("fields/barbs_increments_no_rounding", |fig| {
        let options = BarbsOptions {
            increments: BarbIncrements {
                half: 10.0,
                full: 20.0,
                flag: 100.0,
            },
            rounding: false,
            ..BarbsOptions::default()
        };
        barbs(fig, barb_field(), options)
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("fields/barbs_colormapped", |fig| {
        let field = barb_field();
        let speed: Vec<f64> = field
            .u
            .iter()
            .zip(&field.v)
            .map(|(u, v)| u.hypot(*v))
            .collect();
        let colors = MappedColors::new(speed).cmap(colormaps::get("plasma")?);
        barbs(fig, field.with_colors(colors), BarbsOptions::default())
    })
    .sized(5.0, 4.0, 100.0),
];
