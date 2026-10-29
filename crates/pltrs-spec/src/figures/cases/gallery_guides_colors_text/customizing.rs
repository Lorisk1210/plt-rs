//! rcParams changed at run time: line width and style, the property cycle,
//! temporary settings, and two style sheets applied on top of each other
//! (`customizing`). The settings a figure of the script sees are those set
//! before it and still in force, so each case lists them all.

use std::f64::consts::PI;

use pltrs::prelude::*;

use crate::figures::cases::common::GALLERY_SEED;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `plt.plot(data)` for the guide's 50 random values.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn plot_data(fig: &Figure) -> CaseResult {
    fig.subplot()
        .plot_y(SpecRng::new(GALLERY_SEED).normal(50, 0.0, 1.0));
    Ok(())
}

fn customizing_6(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 2.0 * PI, 50);
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    fig.subplot().plot_with(
        (0..y.len()).map(|i| i as f64).collect::<Vec<_>>(),
        y,
        LineProps::fmt("r-o")?,
    )?;
    Ok(())
}

/// The property cycle the guide sets second.
const RGBY: (&str, &str) = ("axes.prop_cycle", "cycler(color=['r', 'g', 'b', 'y'])");

/// The line settings of `mpl.rc('lines', linewidth=4, linestyle='-.')`.
const DASHDOT: [(&str, &str); 2] = [("lines.linewidth", "4"), ("lines.linestyle", "-.")];

/// Applies the dark background style over the ggplot one the case starts
/// from (`plt.style.use('ggplot')`, then `plt.style.context('dark_background')`).
fn dark_over_ggplot(rc: &mut pltrs::rc::RcParams) -> CaseResult {
    let dark = pltrs::style::sheet("dark_background").ok_or("dark_background is bundled")?;
    dark.apply(rc)?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_colors_text/customizing_1", plot_data)
        .with_rc(&[("lines.linewidth", "2"), ("lines.linestyle", "--")]),
    FigureCase::new("gallery_guides_colors_text/customizing_2", plot_data).with_rc(&[
        ("lines.linewidth", "2"),
        ("lines.linestyle", "--"),
        RGBY,
    ]),
    FigureCase::new("gallery_guides_colors_text/customizing_3", plot_data)
        .with_rc(&[DASHDOT[0], DASHDOT[1], RGBY]),
    FigureCase::new("gallery_guides_colors_text/customizing_4", plot_data).with_rc(&[
        ("lines.linewidth", "2"),
        ("lines.linestyle", ":"),
        RGBY,
    ]),
    FigureCase::new("gallery_guides_colors_text/customizing_5", plot_data).with_rc(&[
        ("lines.linewidth", "3"),
        ("lines.linestyle", "-"),
        RGBY,
    ]),
    FigureCase::new("gallery_guides_colors_text/customizing_6", customizing_6)
        .with_style("ggplot")
        .with_rc_setup(dark_over_ggplot)
        .with_rc(&DASHDOT),
];
