//! The cases from `test_simplification.py`: lines and patches clipped to
//! the view, simplified, or broken by NaNs.

use std::f64::consts::PI;

use plt::axes::TickPosition;
use plt::gridspec::SubplotOverrides;
use plt::mlab::{Complex, fft};
use plt::patches::{PathShape, Rectangle};
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::ticker::MultipleLocator;
use plt::transforms::Point;
use plt_core::{Path, PathCode};

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

use PathCode::{ClosePoly as CL, Curve3 as C3, Curve4 as C4, LineTo as L, MoveTo as M};

/// A path of `(x, y)` vertices and their codes.
fn path(vertices: &[(f64, f64)], codes: &[PathCode]) -> Result<Path, plt_core::PathError> {
    let vertices = vertices.iter().map(|&(x, y)| Point::new(x, y)).collect();
    Path::new(vertices, codes.to_vec())
}

/// `test_clipping`: a sine wave seen through an inverted, narrow y range.
fn clipping(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 2.0, 0.01);
    let s: Vec<f64> = t.iter().map(|t| (2.0 * PI * t).sin()).collect();
    let ax = fig.subplot();
    ax.plot_with(t, s, LineProps::default().linewidth(1.0))?;
    ax.set_ylim(-0.20, -0.28);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_overflow`: a line to a point far outside the view.
fn overflow(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot([1.0, 2.0, 3.0, 2.0e5], [0.0, 1.0, 2.0, 3.0])?;
    ax.set_xlim(2.0, 6.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_diamond`: a diamond whose corners are all outside the view.
fn diamond(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot([0.0, 1.0, 0.0, -1.0, 0.0], [1.0, 0.0, -1.0, 0.0, 1.0])?;
    ax.set_xlim(-0.6, 0.6).set_ylim(-0.6, 0.6);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_simplify_curve`: quadratic curves broken by a NaN control point.
fn simplify_curve(fig: &Figure) -> CaseResult {
    let curve = path(
        &[
            (0.0, 0.0),
            (1.0, 0.0),
            (1.0, 1.0),
            (f64::NAN, 1.0),
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 2.0),
            (0.0, 0.0),
        ],
        &[M, C3, C3, C3, C3, C3, C3, CL],
    )?;
    let ax = fig.subplot();
    ax.add_patch(PathShape(curve)).set_facecolor("none");
    ax.set_xlim(0.0, 2.0).set_ylim(0.0, 2.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_closed_path_nan_removal`: closed and open paths, of lines and of
/// curves, with a NaN at each kind of position.
fn closed_path_nan_removal(fig: &Figure) -> CaseResult {
    let nan = f64::NAN;
    let outer = [
        (-3.0, nan),
        (3.0, -3.0),
        (3.0, 3.0),
        (-3.0, 3.0),
        (-3.0, -3.0),
    ];
    let inner = [
        (-2.0, -2.0),
        (2.0, -2.0),
        (2.0, 2.0),
        (-2.0, nan),
        (-2.0, -2.0),
    ];
    let loop_codes = [M, L, L, L, CL];
    let both: Vec<(f64, f64)> = outer.iter().chain(&inner).copied().collect();
    let subplots = [
        vec![path(&outer, &loop_codes)?, path(&inner, &loop_codes)?],
        vec![path(&both, &[loop_codes, loop_codes].concat())?],
        vec![
            path(
                &[
                    (-1.0, -1.0),
                    (1.0, -1.0),
                    (1.0, nan),
                    (0.0, 1.0),
                    (-1.0, 1.0),
                    (-1.0, -1.0),
                ],
                &[M, L, C3, C3, L, CL],
            )?,
            path(
                &[
                    (-3.0, -3.0),
                    (3.0, -3.0),
                    (3.0, 0.0),
                    (0.0, nan),
                    (-3.0, 3.0),
                    (-3.0, -3.0),
                ],
                &[M, L, C3, C3, L, L],
            )?,
        ],
        vec![
            path(
                &[
                    (-1.0, -1.0),
                    (1.0, -1.0),
                    (1.0, nan),
                    (0.0, 0.0),
                    (0.0, 1.0),
                    (-1.0, 1.0),
                    (-1.0, -1.0),
                ],
                &[M, L, C4, C4, C4, L, CL],
            )?,
            path(
                &[
                    (-2.0, -2.0),
                    (2.0, -2.0),
                    (2.0, 0.0),
                    (0.0, nan),
                    (0.0, 2.0),
                    (-2.0, 2.0),
                    (-2.0, -2.0),
                ],
                &[M, L, C4, C4, C4, L, L],
            )?,
            path(
                &[
                    (-3.0, -3.0),
                    (3.0, -3.0),
                    (3.0, 0.0),
                    (0.0, 0.0),
                    (0.0, nan),
                    (-3.0, 3.0),
                    (-3.0, -3.0),
                ],
                &[M, L, C4, C4, C4, L, L],
            )?,
        ],
    ];
    let axs = fig.subplots(2, 2)?;
    for (ax, paths) in axs.as_slice().iter().zip(subplots) {
        for path in paths {
            ax.add_patch(PathShape(path)).set_facecolor("none");
        }
        ax.set_xlim(-3.5, 3.5).set_ylim(-3.5, 3.5);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_closed_path_clipping`: eight closed notched squares, each
/// starting at another vertex, reaching out of the figure.
fn closed_path_clipping(fig: &Figure) -> CaseResult {
    let mut vertices = Vec::new();
    let mut codes = Vec::new();
    for roll in 0..8 {
        let offset = 0.1 * f64::from(roll) + 0.1;
        let mut pattern = vec![
            (-0.5, 1.5),
            (-0.5, -0.5),
            (1.5, -0.5),
            (1.5, 1.5),
            (1.0 - offset / 2.0, 1.5),
            (1.0 - offset / 2.0, offset),
            (offset / 2.0, offset),
            (offset / 2.0, 1.5),
        ];
        pattern.rotate_right(roll as usize);
        pattern.push(pattern[0]);
        codes.push(M);
        codes.extend([L; 7]);
        codes.push(CL);
        vertices.extend(pattern);
    }
    fig.add_artist(PathShape(path(&vertices, &codes)?))
        .set_facecolor("none");
    Ok(())
}

/// `test_hatch`: a hatched square seen through a tenth of its width.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn hatch(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0))
        .set_fill(false)
        .set_hatch("/");
    ax.set_xlim(0.45, 0.55).set_ylim(0.45, 0.55);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// numpy's `blackman(n)` window.
fn blackman(n: usize) -> Vec<f64> {
    let m = (n - 1) as f64;
    (0..n)
        .map(|i| {
            let phase = 2.0 * PI * i as f64 / m;
            0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos()
        })
        .collect()
}

/// `test_fft_peaks`: the spectrum of a windowed sine, two sharp peaks on
/// 65536 near-zero samples.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn fft_peaks(fig: &Figure) -> CaseResult {
    let n = 65536;
    let signal: Vec<Complex> = blackman(n)
        .into_iter()
        .enumerate()
        .map(|(t, w)| Complex::new((2.0 * PI * 0.01 * t as f64).sin() * w, 0.0))
        .collect();
    let spectrum: Vec<f64> = fft(&signal).into_iter().map(Complex::abs).collect();
    fig.subplot().plot_y(spectrum);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_clipper`: a zigzag on a figure-filling axes, clipped at its edges.
fn clipper(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.0),
        bottom: Some(0.0),
        wspace: Some(0.0),
        hspace: Some(0.0),
        ..SubplotOverrides::default()
    })?;
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.set_ylim(0.0, 5.0).set_autoscale_on(false);
    ax.plot_y([0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0, 5.0]);
    ax.xaxis().set_major_locator(MultipleLocator::new(1.0)?);
    ax.yaxis().set_major_locator(MultipleLocator::new(1.0)?);
    ax.xaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Bottom))?;
    ax.yaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Left))?;
    ax.set_xlim(5.0, 9.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_para_equal_perp`: a line folding back on itself, then 128
/// repeats of one point.
fn para_equal_perp(fig: &Figure) -> CaseResult {
    let head = [
        (0.0, 1.0),
        (1.0, 1.0),
        (2.0, 2.0),
        (1.0, 1.0),
        (0.0, 0.0),
        (-1.0, -1.0),
    ];
    let (x, y): (Vec<f64>, Vec<f64>) = head
        .into_iter()
        .chain([(0.0, 0.0), (1.0, 0.0)])
        .chain(std::iter::repeat_n((1.0, 0.0), 128))
        .map(|(x, y)| (x + 1.0, y + 1.0))
        .unzip();
    let ax = fig.subplot();
    ax.plot(&x, &y)?;
    ax.plot_fmt(&x, &y, "ro")?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_clipping_with_nans`: a sine with a NaN every 100 points, clipped
/// above and below.
fn clipping_with_nans(fig: &Figure) -> CaseResult {
    #[expect(clippy::approx_constant, reason = "the test's own 3.14, not π")]
    let mut x = linspace(0.0, 3.14 * 2.0, 3000);
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    for x in x.iter_mut().step_by(100) {
        *x = f64::NAN;
    }
    let ax = fig.subplot();
    ax.plot(x, y)?;
    ax.set_ylim(-0.25, 0.25);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_rendering/clipping", clipping),
    classic("mpl_rendering/overflow", overflow),
    classic("mpl_rendering/diamond", diamond),
    classic("mpl_rendering/simplify_curve", simplify_curve),
    classic(
        "mpl_rendering/closed_path_nan_removal",
        closed_path_nan_removal,
    ),
    classic("mpl_rendering/closed_path_clipping", closed_path_clipping).sized(5.0, 5.0, 100.0),
    classic("mpl_rendering/hatch", hatch),
    classic("mpl_rendering/fft_peaks", fft_peaks),
    classic("mpl_rendering/clipper", clipper).sized(2.0, 1.0, 100.0),
    classic("mpl_rendering/para_equal_perp", para_equal_perp),
    FigureCase::new("mpl_rendering/clipping_with_nans", clipping_with_nans),
];
