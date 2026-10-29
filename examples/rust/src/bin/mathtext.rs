//! Mathtext: normal distributions labelled with TeX-like formulas, typeset
//! without TeX, and the density written out in the axes.

use std::f64::consts::PI;

use pltrs::layout::LayoutEngine;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x = linspace(-5.0, 5.0, 400);
    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    for (mu, sigma) in [(0.0, 0.6), (0.0, 1.0), (-1.5, 1.6)] {
        let density = x.iter().map(|x| {
            (-(x - mu) * (x - mu) / (2.0 * sigma * sigma)).exp() / (sigma * (2.0 * PI).sqrt())
        });
        let label = format!(r"$\mu = {mu},\ \sigma = {sigma}$");
        ax.plot_with(
            &x,
            density,
            LineProps::default().linewidth(2.0).label(label),
        )?;
    }
    ax.text(
        1.2,
        0.48,
        r"$f(x) = \frac{1}{\sigma\sqrt{2\pi}}\,e^{-\frac{(x-\mu)^2}{2\sigma^2}}$",
    )
    .set_fontsize(15.0);
    ax.set_title(r"The normal distribution $\mathcal{N}(\mu, \sigma^2)$");
    ax.set_xlabel(r"$x$").set_ylabel(r"$f(x)$");
    ax.set_ylim(0.0, 0.75)?;
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperLeft))?;

    fig.savefig("mathtext.png")?;
    Ok(())
}
