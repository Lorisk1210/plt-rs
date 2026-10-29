//! `subplot` and `subplots_demo`: grids of subplots, stacked, shared,
//! squeezed together and polar.

use pltrs::axes::TickParams;
use pltrs::figure::Share;
use pltrs::gridspec::{GridSpec, SubplotOverrides};
use pltrs::layout::TightLayout;
use pltrs::prelude::*;

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};

/// `subplot`'s damped (`x1`, `y1`) and undamped (`x2`, `y2`) cosines.
fn tale() -> [(Vec<f64>, Vec<f64>); 2] {
    let x1 = linspace(0.0, 5.0, 50);
    let y1 = map(&x1, |x| (2.0 * std::f64::consts::PI * x).cos() * (-x).exp());
    let x2 = linspace(0.0, 2.0, 50);
    let y2 = map(&x2, |x| (2.0 * std::f64::consts::PI * x).cos());
    [(x1, y1), (x2, y2)]
}

/// Plots the two oscillations of `tale` into `top` and `bottom`, labelled.
fn tell_tale(top: &Axes, bottom: &Axes) -> CaseResult {
    let [(x1, y1), (x2, y2)] = tale();
    top.plot_fmt(x1, y1, "o-")?;
    top.set_ylabel("Damped oscillation");
    bottom.plot_fmt(x2, y2, ".-")?;
    bottom.set_xlabel("time (s)").set_ylabel("Undamped");
    Ok(())
}

fn subplot_1(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    fig.suptitle("A tale of 2 subplots");
    tell_tale(&axs[(0, 0)], &axs[(1, 0)])
}

fn subplot_2(fig: &Figure) -> CaseResult {
    let top = fig.add_subplot(2, 1, 1)?;
    top.set_title("A tale of 2 subplots");
    tell_tale(&top, &fig.add_subplot(2, 1, 2)?)
}

/// `subplots_demo`'s `x = np.linspace(0, 2 * np.pi, 400)`.
fn x() -> Vec<f64> {
    linspace(0.0, 2.0 * std::f64::consts::PI, 400)
}

/// `x + shift`.
fn shifted(shift: f64) -> Vec<f64> {
    map(&x(), |x| x + shift)
}

/// `y = np.sin(x ** 2)`, scaled by `gain` and raised to `power`.
fn y(gain: f64, power: i32) -> Vec<f64> {
    map(&x(), |x| gain * (x * x).sin().powi(power))
}

/// `y` plotted against `x` in each axes, then `-y` in each further one.
fn mirrored(axs: &[Axes], shift: f64) -> CaseResult {
    axs[0].plot(x(), y(1.0, 1))?;
    axs[1].plot(shifted(shift), y(-1.0, 1))?;
    Ok(())
}

fn subplots_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot(x(), y(1.0, 1))?;
    ax.set_title("A single plot");
    Ok(())
}

fn subplots_demo_2(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    fig.suptitle("Vertically stacked subplots");
    mirrored(axs.as_slice(), 0.0)
}

fn subplots_demo_4(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    fig.suptitle("Horizontally stacked subplots");
    mirrored(axs.as_slice(), 0.0)
}

fn subplots_demo_5(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let plots = [
        ("C0", 1.0, "Axis [0, 0]"),
        ("tab:orange", 1.0, "Axis [0, 1]"),
        ("tab:green", -1.0, "Axis [1, 0]"),
        ("tab:red", -1.0, "Axis [1, 1]"),
    ];
    for (ax, (color, gain, title)) in axs.as_slice().iter().zip(plots) {
        ax.plot_fmt(x(), y(gain, 1), color)?;
        ax.set_title(title);
    }
    for ax in axs.as_slice() {
        ax.set_xlabel("x-label").set_ylabel("y-label");
    }
    for ax in axs.as_slice() {
        ax.label_outer();
    }
    Ok(())
}

/// `_four_squares`: `y`, `y**2`, `-y` and `-y**2` in the default colour,
/// orange, green and red, shifted right by `shifts`, outer labels only.
fn four_squares(axs: &[Axes], shifts: [f64; 4]) -> CaseResult {
    let curves = [
        ("C0", 1.0, 1),
        ("tab:orange", 1.0, 2),
        ("tab:green", -1.0, 1),
        ("tab:red", -1.0, 2),
    ];
    for ((ax, shift), (color, gain, power)) in axs.iter().zip(shifts).zip(curves) {
        ax.plot_fmt(shifted(shift), y(gain, power), color)?;
        ax.label_outer();
    }
    Ok(())
}

fn subplots_demo_6(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    fig.suptitle("Sharing x per column, y per row");
    four_squares(axs.as_slice(), [0.0; 4])
}

fn subplots_demo_7(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    fig.suptitle("Axes values are scaled individually by default");
    mirrored(axs.as_slice(), 1.0)
}

fn subplots_demo_8(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    fig.suptitle("Aligning x-axis using sharex");
    mirrored(axs.as_slice(), 1.0)
}

/// `_three_shared`: `y**2`, `0.3 * y` with circles and `y` with pluses.
fn three_shared(axs: &[Axes]) -> CaseResult {
    axs[0].plot(x(), y(1.0, 2))?;
    axs[1].plot_fmt(x(), y(0.3, 1), "o")?;
    axs[2].plot_fmt(x(), y(1.0, 1), "+")?;
    Ok(())
}

/// Three stacked axes sharing both axes, titled `title`.
fn three_stacked(fig: &Figure, title: &str) -> Result<Vec<Axes>, Box<dyn std::error::Error>> {
    let axs = fig.subplots_shared(3, 1, Share::All, Share::All)?;
    fig.suptitle(title);
    three_shared(axs.as_slice())?;
    Ok(axs.into_vec())
}

fn subplots_demo_9(fig: &Figure) -> CaseResult {
    three_stacked(fig, "Sharing both axes")?;
    Ok(())
}

fn subplots_demo_10(fig: &Figure) -> CaseResult {
    let axs = three_stacked(fig, "Restored xtick labels on to Axes")?;
    axs[0].tick_params(TickParams {
        labelbottom: Some(true),
        ..TickParams::default()
    });
    Ok(())
}

/// `fig.add_gridspec(nrows, ncols, hspace=hspace, wspace=wspace)`.
fn spaced_grid(
    nrows: usize,
    ncols: usize,
    hspace: Option<f64>,
    wspace: Option<f64>,
) -> Result<GridSpec, pltrs::gridspec::GridSpecError> {
    GridSpec::with_ratios(
        nrows,
        ncols,
        None,
        None,
        SubplotOverrides {
            hspace,
            wspace,
            ..SubplotOverrides::default()
        },
    )
}

fn subplots_demo_11(fig: &Figure) -> CaseResult {
    let grid = spaced_grid(3, 1, Some(0.0), None)?;
    let axs = fig.subplots_on(&grid, Share::All, Share::All)?;
    fig.suptitle("Sharing both axes");
    three_shared(axs.as_slice())?;
    for ax in axs.as_slice() {
        ax.label_outer();
    }
    Ok(())
}

fn subplots_demo_12(fig: &Figure) -> CaseResult {
    let grid = spaced_grid(2, 2, Some(0.0), Some(0.0))?;
    let axs = fig.subplots_on(&grid, Share::Col, Share::Row)?;
    fig.suptitle("Sharing x per column, y per row");
    four_squares(axs.as_slice(), [0.0, 0.0, 1.0, 2.0])
}

fn subplots_demo_13(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    axs[(0, 0)].plot(x(), y(1.0, 1))?;
    axs[(0, 0)].set_title("main");
    axs[(1, 0)].plot(x(), y(1.0, 2))?;
    axs[(1, 0)].set_title("shares x with main");
    axs[(1, 0)].share_x(&axs[(0, 0)])?;
    axs[(0, 1)].plot(shifted(1.0), map(&y(1.0, 1), |y| y + 1.0))?;
    axs[(0, 1)].set_title("unrelated");
    axs[(1, 1)].plot(shifted(2.0), map(&y(1.0, 1), |y| y + 2.0))?;
    axs[(1, 1)].set_title("also unrelated");
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

fn subplots_demo_14(fig: &Figure) -> CaseResult {
    fig.add_polar_subplot(1, 2, 1)?.plot(x(), y(1.0, 1))?;
    fig.add_polar_subplot(1, 2, 2)?.plot(x(), y(1.0, 2))?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_subplots/subplot_1", subplot_1),
    FigureCase::new("gallery_subplots/subplot_2", subplot_2),
    FigureCase::new("gallery_subplots/subplots_demo_1", subplots_demo_1),
    FigureCase::new("gallery_subplots/subplots_demo_2", subplots_demo_2),
    FigureCase::new("gallery_subplots/subplots_demo_4", subplots_demo_4),
    FigureCase::new("gallery_subplots/subplots_demo_5", subplots_demo_5),
    FigureCase::new("gallery_subplots/subplots_demo_6", subplots_demo_6),
    FigureCase::new("gallery_subplots/subplots_demo_7", subplots_demo_7),
    FigureCase::new("gallery_subplots/subplots_demo_8", subplots_demo_8),
    FigureCase::new("gallery_subplots/subplots_demo_9", subplots_demo_9),
    FigureCase::new("gallery_subplots/subplots_demo_10", subplots_demo_10),
    FigureCase::new("gallery_subplots/subplots_demo_11", subplots_demo_11),
    FigureCase::new("gallery_subplots/subplots_demo_12", subplots_demo_12),
    FigureCase::new("gallery_subplots/subplots_demo_13", subplots_demo_13),
    FigureCase::new("gallery_subplots/subplots_demo_14", subplots_demo_14),
];
