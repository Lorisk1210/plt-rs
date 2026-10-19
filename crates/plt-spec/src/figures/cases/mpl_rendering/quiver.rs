//! The cases from `test_quiver.py`: quivers, their keys and barbs.

use std::f64::consts::{PI, TAU};

use plt::axes::{Axes, MappedColors, PlotError, ScatterOptions};
use plt::colors::colormaps;
use plt::fonts::{FontProperties, FontSize, FontWeight, RelativeSize};
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::prelude::*;
use plt::quiver::{
    Angles, BarbPivot, BarbSizes, BarbsOptions, KeyCoordinates, LabelPosition, Pivot, Quiver,
    QuiverKeyOptions, QuiverOptions, QuiverUnits, VectorField,
};
use plt::rc::Aspect;

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// The test module's `draw_quiver`: `cos x`, `sin y` on the integer grid
/// of `[0, 2π)²`, at the grid indices.
fn draw_quiver(ax: &Axes, options: QuiverOptions) -> Result<Quiver, PlotError> {
    let x = arange(0.0, TAU, 1.0);
    let (xx, yy) = meshgrid(&x, &x);
    let u = xx.map(|x| x.cos());
    let v = yy.map(|y| y.sin());
    ax.quiver(VectorField::on_index_grid(&u, &v), options)
}

/// The quiver key font: the rc defaults in bold, at `size`.
fn bold(size: Option<FontSize>) -> FontProperties {
    let rc = plt::rc::params();
    let font = FontProperties::from_rc(&rc).with_weight(FontWeight::BOLD);
    match size {
        Some(size) => font.with_size(size, rc.font_size),
        None => font,
    }
}

/// `test_quiver_animate`: an animated quiver, which saving draws anyway.
fn quiver_animate(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let q = draw_quiver(&ax, QuiverOptions::default())?;
    q.set_animated(true);
    let options = QuiverKeyOptions {
        labelpos: LabelPosition::West,
        font: Some(bold(None)),
        ..QuiverKeyOptions::default()
    };
    ax.quiverkey(&q, (0.5, 0.88), 2.0, r"$2 \frac{m}{s}$", &options);
    Ok(())
}

/// `test_quiver_with_key`: a key placed in figure coordinates, tilted.
fn quiver_with_key(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_margins(0.1, 0.1)?;
    let q = draw_quiver(&ax, QuiverOptions::default())?;
    let options = QuiverKeyOptions {
        angle: -10.0,
        coordinates: KeyCoordinates::Figure,
        labelpos: LabelPosition::West,
        font: Some(bold(Some(FontSize::Relative(RelativeSize::Large)))),
        ..QuiverKeyOptions::default()
    };
    ax.quiverkey(
        &q,
        (0.5, 0.95),
        2.0,
        r"$2\, \mathrm{m}\, \mathrm{s}^{-1}$",
        &options,
    );
    Ok(())
}

/// `test_quiver_single`: one arrow with margins around it.
fn quiver_single(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_margins(0.1, 0.1)?;
    let field = VectorField::new([1.0], [1.0], [2.0], [2.0]);
    ax.quiver(field, QuiverOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_quiver_key_pivot`: a key on each side of the axes, its label
/// outside.
fn quiver_key_pivot(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = linspace(0.0, TAU, 10);
    let u = Grid::from_fn(10, 10, |i, _| t[i].sin());
    let v = Grid::from_fn(10, 10, |_, j| t[j].cos());
    let q = ax.quiver(VectorField::on_index_grid(&u, &v), QuiverOptions::default())?;
    ax.set_xlim(-2.0, 11.0).set_ylim(-2.0, 11.0);
    let keys = [
        ((0.5, 1.0), "N", LabelPosition::North),
        ((1.0, 0.5), "E", LabelPosition::East),
        ((0.5, 0.0), "S", LabelPosition::South),
        ((0.0, 0.5), "W", LabelPosition::West),
    ];
    for (position, label, labelpos) in keys {
        let options = QuiverKeyOptions {
            labelpos,
            ..QuiverKeyOptions::default()
        };
        ax.quiverkey(&q, position, 1.0, label, &options);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_quiver_key_xy`: keys matching arrows scaled in data units, with
/// angles in screen (`uv`) and data (`xy`) space.
fn quiver_key_xy(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 8.0, 1.0);
    let u: Vec<f64> = x.iter().map(|x| (x * PI / 4.0).cos()).collect();
    let v: Vec<f64> = x.iter().map(|x| (x * PI / 4.0).sin()).collect();
    let axs = fig.subplots(2, 1)?;
    for (ax, angles) in axs.as_slice().iter().zip([Angles::Uv, Angles::Xy]) {
        ax.set_xlim(-1.0, 8.0).set_ylim(-0.2, 0.2);
        let options = QuiverOptions {
            pivot: Pivot::Middle,
            units: QuiverUnits::Xy,
            width: Some(0.05),
            scale: Some(2.0),
            scale_units: Some(QuiverUnits::Xy),
            angles,
            ..QuiverOptions::default()
        };
        let field = VectorField::new(x.clone(), vec![0.0; 8], u.clone(), v.clone());
        let q = ax.quiver(field, options)?;
        for (x, angle) in [(0.2, 0.0), (0.5, 45.0), (0.8, 90.0)] {
            let options = QuiverKeyOptions {
                angle,
                color: Some("b".into()),
                ..QuiverKeyOptions::default()
            };
            ax.quiverkey(&q, (x, 0.8), 1.0, "", &options);
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The barb tests' 5 × 5 grid over `[-5, 5]²` with winds `12 · (x, y)`.
fn barb_field() -> VectorField {
    let x = linspace(-5.0, 5.0, 5);
    let (xx, yy) = meshgrid(&x, &x);
    let (u, v) = (xx.map(|x| 12.0 * x), yy.map(|y| 12.0 * y));
    VectorField::on_mesh(&xx, &yy, &u, &v)
}

/// The barb tests' options: filled empty barbs, no rounding, their sizes.
fn barb_options() -> BarbsOptions {
    BarbsOptions {
        fill_empty: true,
        rounding: false,
        sizes: BarbSizes {
            emptybarb: 0.25,
            spacing: 0.2,
            height: 0.3,
            ..BarbSizes::default()
        },
        ..BarbsOptions::default()
    }
}

/// `test_barbs`: barbs coloured by their speed.
fn barbs(fig: &Figure) -> CaseResult {
    let field = barb_field();
    let speed: Vec<f64> = field
        .u
        .iter()
        .zip(&field.v)
        .map(|(u, v)| u.hypot(*v))
        .collect();
    let colors = MappedColors::new(speed).with_cmap(colormaps::get("viridis")?);
    fig.subplot()
        .barbs(field.with_colors(colors), barb_options())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_barbs_pivot` and `test_barbs_flip`: barbs pivoting 1.7 staff
/// lengths from their tip, those below the x axis flipped when `flip`.
fn pivoted_barbs(fig: &Figure, flip: bool) -> Result<Axes, PlotError> {
    let field = barb_field();
    let options = BarbsOptions {
        pivot: BarbPivot::Offset(1.7),
        flip: if flip {
            field.y.iter().map(|&y| y < 0.0).collect()
        } else {
            vec![false]
        },
        ..barb_options()
    };
    let ax = fig.subplot();
    ax.barbs(field, options)?;
    Ok(ax)
}

/// `test_barbs_pivot`: pivoted barbs over black dots at their positions.
fn barbs_pivot(fig: &Figure) -> CaseResult {
    let ax = pivoted_barbs(fig, false)?;
    let field = barb_field();
    let options = ScatterOptions {
        sizes: Some(vec![49.0]),
        colors: ScatterColors::Single("black".into()),
        ..ScatterOptions::default()
    };
    ax.scatter_with(field.x, field.y, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_barbs_flip`: pivoted barbs, flipped below the x axis.
fn barbs_flip(fig: &Figure) -> CaseResult {
    pivoted_barbs(fig, true)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_quiver_xy`: one arrow from the origin to `(1, 1)` in data units.
fn quiver_xy(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    let options = QuiverOptions {
        angles: Angles::Xy,
        scale_units: Some(QuiverUnits::Xy),
        scale: Some(1.0),
        ..QuiverOptions::default()
    };
    ax.quiver(VectorField::new([0.0], [0.0], [1.0], [1.0]), options)?;
    ax.set_xlim(0.0, 1.1).set_ylim(0.0, 1.1);
    ax.grid(true);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_quiverkey_zorder`'s test figure: two keys in data coordinates
/// given `zorder` (`None` = the default).
fn quiverkey_zorder(fig: &Figure, zorder: Option<f64>) -> CaseResult {
    let x = arange(1.0, 6.0, 1.0);
    let (xx, yy) = meshgrid(&x, &x);
    let field = VectorField::new(xx.as_slice(), yy.as_slice(), [2.0], [2.0]);
    let ax = fig.subplot();
    let options = QuiverOptions {
        pivot: Pivot::Middle,
        ..QuiverOptions::default()
    };
    let q = ax.quiver(field, options)?;
    ax.set_xlim(0.5, 5.5).set_ylim(0.5, 5.5);
    for ((x, y), length, label, angle) in
        [((4.0, 4.0), 25.0, "U", 0.0), ((5.5, 2.0), 20.0, "V", 90.0)]
    {
        let options = QuiverKeyOptions {
            angle,
            coordinates: KeyCoordinates::Data,
            color: Some("blue".into()),
            zorder,
            ..QuiverKeyOptions::default()
        };
        ax.quiverkey(&q, (x, y), length, label, &options);
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_rendering/quiver_animate", quiver_animate),
    FigureCase::new("mpl_rendering/quiver_with_key", quiver_with_key),
    classic("mpl_rendering/quiver_single", quiver_single),
    FigureCase::new("mpl_rendering/quiver_key_pivot", quiver_key_pivot),
    classic("mpl_rendering/quiver_key_xy", quiver_key_xy),
    classic("mpl_rendering/barbs", barbs),
    classic("mpl_rendering/barbs_pivot", barbs_pivot),
    classic("mpl_rendering/barbs_flip", barbs_flip),
    classic("mpl_rendering/quiver_xy", quiver_xy),
    classic("mpl_rendering/quiverkey_zorder_0", |fig| {
        quiverkey_zorder(fig, Some(0.0))
    }),
    classic("mpl_rendering/quiverkey_zorder_2", |fig| {
        quiverkey_zorder(fig, Some(2.0))
    }),
    classic("mpl_rendering/quiverkey_zorder_5", |fig| {
        quiverkey_zorder(fig, Some(5.0))
    }),
    classic("mpl_rendering/quiverkey_zorder_none", |fig| {
        quiverkey_zorder(fig, None)
    }),
];
