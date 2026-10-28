//! Scatter plots.

use plt::axes::{MappedColors, ScatterColors, ScatterOptions};
use plt::collections::EdgeColors;
use plt::colors::{Alpha, Color};
use plt::figure::Figure;
use plt::mplot3d::{Axes3D, Axes3DError, Scatter3DOptions};
use plt::numeric::linspace;

use super::{range, remove_ticks_and_titles};
use crate::figures::CaseResult;

/// `np.arange(start, start + 10)` as floats.
fn ten_from(start: u32) -> Vec<f64> {
    (start..start + 10).map(f64::from).collect()
}

/// `ax.scatter(x, x, x, **scatter)` with the default 3-D options.
fn diagonal(ax: &Axes3D, x: &[f64], scatter: ScatterOptions) -> Result<(), Axes3DError> {
    let options = Scatter3DOptions {
        scatter,
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(x, x, x, options).map(drop)
}

/// `marker=marker` on top of `options`.
fn marked(
    marker: &str,
    options: ScatterOptions,
) -> Result<ScatterOptions, Box<dyn std::error::Error>> {
    Ok(ScatterOptions {
        marker: Some(marker.parse()?),
        ..options
    })
}

/// `c=color` (or `color=color`).
fn colored(color: &str) -> ScatterOptions {
    ScatterOptions {
        colors: ScatterColors::Single(color.into()),
        ..ScatterOptions::default()
    }
}

/// `facecolor=face, edgecolor=edge`.
fn face_edge(face: &str, edge: &str) -> ScatterOptions {
    ScatterOptions {
        edgecolors: Some(EdgeColors::Colors(vec![edge.into()])),
        ..colored(face)
    }
}

pub(super) fn scatter3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    diagonal(&ax, &range(10), marked("o", colored("r"))?)?;
    diagonal(&ax, &ten_from(10), marked("^", colored("b"))?)?;
    diagonal(&ax, &[], marked("X", colored("r"))?)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn scatter3d_color(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    diagonal(&ax, &range(10), marked("o", face_edge("r", "none"))?)?;
    diagonal(&ax, &range(10), marked("o", face_edge("none", "r"))?)?;
    diagonal(&ax, &ten_from(10), marked("s", colored("b"))?)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `linewidth=np.arange(10)`.
fn widening() -> ScatterOptions {
    ScatterOptions {
        linewidths: Some(range(10)),
        ..ScatterOptions::default()
    }
}

pub(super) fn scatter3d_linewidth(fig: &Figure) -> CaseResult {
    diagonal(&fig.subplot_3d(), &range(10), marked("o", widening())?)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn cmap_alpha(fig: &Figure) -> CaseResult {
    let scatter = ScatterOptions {
        colors: MappedColors::new(vec![0.0, 1.0, f64::NAN, 3.0, 4.0]).into(),
        ..ScatterOptions::default()
    };
    let options = Scatter3DOptions {
        scatter,
        ..Scatter3DOptions::default()
    };
    fig.subplot_3d()
        .scatter_with(range(5), [0.0; 5], range(5), options)?;
    Ok(())
}

pub(super) fn linewidth_modification(fig: &Figure) -> CaseResult {
    let x = range(10);
    let options = Scatter3DOptions {
        scatter: marked("o", ScatterOptions::default())?,
        ..Scatter3DOptions::default()
    };
    let collection = fig.subplot_3d().scatter_with(&x, &x, &x, options)?;
    collection.set_linewidths(range(10));
    Ok(())
}

pub(super) fn modification(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let x = range(10);
    let options = Scatter3DOptions {
        depthshade: Some(true),
        scatter: marked("o", ScatterOptions::default())?,
        ..Scatter3DOptions::default()
    };
    let collection = ax.scatter_with(&x, &x, &x, options)?;
    collection.set_facecolor("C1").set_edgecolor("C2");
    collection.set_alphas([0.3, 0.7].repeat(5));
    ax.set_depthshade(&collection, false)?;
    collection
        .set_sizes(vec![75.0; 10])
        .set_linewidths(vec![3.0]);
    Ok(())
}

/// The colour `C{n}` per marker of the 10 × 10 grid of `scatter3d_sorting`:
/// `base` everywhere, `patches` over the row and column ranges given.
fn grid_colors(base: u8, patches: &[(u8, [usize; 4])]) -> Vec<Color> {
    (0..100)
        .map(|i| {
            let (row, col) = (i / 10, i % 10);
            let n = patches
                .iter()
                .find(|(_, [r0, r1, c0, c1])| {
                    (*r0..*r1).contains(&row) && (*c0..*c1).contains(&col)
                })
                .map_or(base, |(n, _)| *n);
            Color::from(format!("C{n}").as_str())
        })
        .collect()
}

pub(super) fn sorting(fig: &Figure) -> CaseResult {
    // np.mgrid[:10, :10], flattened: y is the row, x the column.
    let x: Vec<f64> = (0..100).map(|i| f64::from(i % 10)).collect();
    let y: Vec<f64> = (0..100).map(|i| f64::from(i / 10)).collect();
    let z = range(100);
    // Larger on the cells whose row and column are both even or both odd.
    let checkered = |large: f64, small: f64| -> Vec<f64> {
        (0..100)
            .map(|i| {
                if (i / 10 + i % 10) % 2 == 0 {
                    large
                } else {
                    small
                }
            })
            .collect()
    };
    let facecolors = grid_colors(
        0,
        &[(1, [0, 5, 0, 5]), (2, [6, 10, 0, 4]), (3, [6, 10, 6, 10])],
    );
    let edgecolors = grid_colors(
        4,
        &[(5, [1, 5, 1, 5]), (6, [5, 9, 1, 5]), (7, [5, 9, 5, 9])],
    );
    let scatter = ScatterOptions {
        sizes: Some(checkered(100.0, 25.0)),
        colors: ScatterColors::PerPoint(facecolors),
        edgecolors: Some(EdgeColors::Colors(edgecolors)),
        linewidths: Some(checkered(5.0, 2.0)),
        alpha: Alpha::Uniform(1.0),
        ..ScatterOptions::default()
    };
    let options = Scatter3DOptions {
        depthshade: Some(false),
        scatter,
        ..Scatter3DOptions::default()
    };
    fig.subplot_3d().scatter_with(&x, &y, &z, options)?;
    Ok(())
}

/// The marker draw-order tests: two large markers, blue at (-1, 1, 0) and
/// yellow at (1, -1, 0), seen edge-on from `azim`.
fn two_markers(fig: &Figure, azim: f64, axis_off: bool) -> CaseResult {
    let ax = fig.subplot_3d();
    if axis_off {
        ax.set_axis_off();
    }
    let scatter = ScatterOptions {
        sizes: Some(vec![3500.0]),
        colors: ScatterColors::PerPoint(vec!["b".into(), "y".into()]),
        ..ScatterOptions::default()
    };
    let options = Scatter3DOptions {
        scatter,
        ..Scatter3DOptions::default()
    };
    ax.scatter_with([-1.0, 1.0], [1.0, -1.0], [0.0, 0.0], options)?;
    ax.view_init(0.0, azim, 0.0);
    Ok(())
}

pub(super) fn draw_order_m50(fig: &Figure) -> CaseResult {
    two_markers(fig, -50.0, false)
}

pub(super) fn draw_order_130(fig: &Figure) -> CaseResult {
    two_markers(fig, 130.0, false)
}

pub(super) fn draw_order_view_rotated(fig: &Figure) -> CaseResult {
    two_markers(fig, 130.0, true)
}

pub(super) fn plot_scatter_masks(fig: &Figure) -> CaseResult {
    // x = y = np.linspace(0, 10, 100) and z = sin(x) cos(y); the test
    // masks the points of positive z, which plt-rs hides with NaN.
    let (x, z): (Vec<f64>, Vec<f64>) = linspace(0.0, 10.0, 100)
        .into_iter()
        .map(|v| {
            let z = v.sin() * v.cos();
            if z > 0.0 {
                (f64::NAN, f64::NAN)
            } else {
                (v, z)
            }
        })
        .unzip();
    let ax = fig.subplot_3d();
    ax.scatter(&x, &x, &z)?;
    ax.plot(&x, &x, &z)?;
    Ok(())
}
