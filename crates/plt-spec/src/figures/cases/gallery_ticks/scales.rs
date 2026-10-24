//! Scales: symlog against asinh, asinh linear widths and bases, Cauchy
//! deviates on asinh and symlog axes, equal aspect on log axes, the scales
//! overview, probability plots on logit axes and power-law colour
//! normalisations (`asinh_demo`, `aspect_loglog`, `scales`, `logit_demo`
//! and `power_norm`).

use std::f64::consts::{FRAC_2_SQRT_PI, PI, SQRT_2};

use plt::axes::{Adjustable, AxisName, Hist2dOptions, PcolorOptions};
use plt::colors::{Alpha, ColorizerOptions, PowerNorm};
use plt::figure::Share;
use plt::fonts::{FontSize, RelativeSize};
use plt::layout::{LayoutEngine, TightLayout};
use plt::legend::{LegendLoc, LegendOptions};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::scale::{FuncScale, FuncTransform, ScaleOptions};
use plt::stats::{AxisBins, Histogram2dOptions};
use plt::ticker::pyfmt;

use crate::figures::cases::common::{logit_grid, map};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The seed of the examples' random data.
const SEED: u64 = 19_680_801;

fn asinh_demo_1(fig: &Figure) -> CaseResult {
    let x = linspace(-3.0, 6.0, 500);
    let axs = fig.subplots_shared(1, 2, Share::All, Share::None)?;
    for (ax, scale) in axs.as_slice().iter().zip(["symlog", "asinh"]) {
        ax.plot(&x, &x)?;
        ax.set_yscale(scale)?;
        ax.grid(true);
        ax.set_title(scale);
    }
    Ok(())
}

fn asinh_demo_2(fig: &Figure) -> CaseResult {
    let x = linspace(-3.0, 6.0, 500);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots_shared(1, 3, Share::All, Share::None)?;
    for (ax, (a0, base)) in axs
        .as_slice()
        .iter()
        .zip([(0.2, 2.0), (1.0, 0.0), (5.0, 10.0)])
    {
        ax.set_title(format!("linear_width={}", pyfmt::general(a0, 3)));
        for (factor, label) in [(1.0, "y=x"), (10.0, "y=10x"), (100.0, "y=100x")] {
            ax.plot_with(
                &x,
                map(&x, |x| factor * x),
                LineProps::default().label(label),
            )?;
        }
        ax.set_scale_with(
            AxisName::Y,
            "asinh",
            ScaleOptions {
                linear_width: Some(a0),
                base: Some(base),
                ..ScaleOptions::default()
            },
        )?;
        ax.grid(true);
        ax.legend(
            LegendOptions::default()
                .loc(LegendLoc::Best)
                .fontsize(FontSize::Relative(RelativeSize::Small)),
        )?;
    }
    Ok(())
}

fn asinh_demo_3(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let r = map(&rng.uniform(5000, -PI / 2.02, PI / 2.02), |u| 3.0 * u.tan());
    let th = rng.uniform(r.len(), 0.0, 2.0 * PI);
    let (x, y): (Vec<f64>, Vec<f64>) = r
        .iter()
        .zip(&th)
        .map(|(r, th)| (r * th.cos(), r * th.sin()))
        .unzip();
    let ax = fig.subplot();
    ax.scatter_with(
        x,
        y,
        ScatterOptions {
            sizes: Some(vec![4.0]),
            alpha: Alpha::Uniform(0.5),
            ..ScatterOptions::default()
        },
    )?;
    ax.set_xscale("asinh")?;
    ax.set_yscale("symlog")?;
    ax.set_xlabel("asinh");
    ax.set_ylabel("symlog");
    ax.set_title("2D Cauchy random deviates");
    ax.set_xlim(-50.0, 50.0)?.set_ylim(-50.0, 50.0)?;
    ax.grid(true);
    Ok(())
}

fn aspect_loglog(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax1.set_xscale("log")?;
    ax1.set_yscale("log")?;
    ax1.set_xlim(1e1, 1e3)?.set_ylim(1e2, 1e3)?;
    ax1.set_aspect(Aspect::Ratio(1.0))?;
    ax1.set_title("adjustable = box");
    ax2.set_xscale("log")?;
    ax2.set_yscale("log")?;
    ax2.set_adjustable(Adjustable::Datalim)?;
    ax2.plot_fmt([1.0, 3.0, 10.0], [1.0, 9.0, 100.0], "o-")?;
    ax2.set_xlim(1e-1, 1e2)?.set_ylim(1e-1, 1e3)?;
    ax2.set_aspect(Aspect::Ratio(1.0))?;
    ax2.set_title("adjustable = datalim");
    Ok(())
}

fn scales(fig: &Figure) -> CaseResult {
    let x: Vec<f64> = (0..400).map(f64::from).collect();
    let y = linspace(0.002, 1.0, 400);
    let mean = y.iter().sum::<f64>() / y.len() as f64;
    let centred = map(&y, |y| y - mean);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(3, 2)?;
    let with = |linthresh: Option<f64>, linear_width: Option<f64>| ScaleOptions {
        linthresh,
        linear_width,
        ..ScaleOptions::default()
    };
    let panels = [
        ((0, 0), &y, "linear", with(None, None), "linear"),
        ((0, 1), &y, "log", with(None, None), "log"),
        ((1, 0), &centred, "symlog", with(Some(0.02), None), "symlog"),
        ((1, 1), &y, "logit", with(None, None), "logit"),
        ((2, 0), &centred, "asinh", with(None, Some(0.01)), "asinh"),
    ];
    for (cell, values, scale, options, title) in panels {
        let ax = &axs[cell];
        ax.plot(&x, values)?;
        ax.set_scale_with(AxisName::Y, scale, options)?;
        ax.set_title(title);
        ax.grid(true);
    }
    // `x**(1/2)` and `x**2` on arrays: numpy's square root and square.
    let root = FuncTransform::new(f64::sqrt, |x| x * x);
    let ax = &axs[(2, 1)];
    ax.plot(&x, &y)?;
    ax.set_scale(AxisName::Y, Box::new(FuncScale::from_transform(root)));
    ax.set_title("function: $x^{1/2}$");
    ax.grid(true);
    ax.set_yticks(arange(0.0, 1.2, 0.2));
    Ok(())
}

/// The error function, accurate to a few units in the last place (the
/// `math.erf` of the example): its Maclaurin series below 2.5, the
/// continued fraction of `erfc` above.
fn erf(x: f64) -> f64 {
    if x < 0.0 {
        return -erf(-x);
    }
    if x < 2.5 {
        let x2 = x * x;
        let (mut term, mut sum) = (x, x);
        for n in 1..100 {
            let n = f64::from(n);
            term *= -x2 / n;
            let next = term / (2.0 * n + 1.0);
            sum += next;
            if next.abs() < 1e-17 * sum {
                break;
            }
        }
        FRAC_2_SQRT_PI * sum
    } else {
        // erfc(x) = exp(-x²)/√π / (x + (1/2)/(x + 1/(x + (3/2)/(x + ...)))).
        let tail = (1..60)
            .rev()
            .fold(x, |fraction, k| x + f64::from(k) / 2.0 / fraction);
        1.0 - (-x * x).exp() / (PI.sqrt() * tail)
    }
}

fn logit_demo(fig: &Figure) -> CaseResult {
    let x = linspace(-10.0, 10.0, 10_000);
    let cdf_norm = map(&x, |w| erf(w / SQRT_2) / 2.0 + 0.5);
    let cdf_laplacian = map(&x, |x| {
        if x < 0.0 {
            0.5 * x.exp()
        } else {
            1.0 - 0.5 * (-x).exp()
        }
    });
    let cdf_cauchy = map(&x, |x| x.atan() / PI + 0.5);
    logit_grid(
        fig,
        &x,
        &[
            (cdf_norm, r"$\mathcal{N}$"),
            (cdf_laplacian, r"$\mathcal{L}$"),
            (cdf_cauchy, "Cauchy"),
        ],
    )
}

/// `n` draws of a two-dimensional normal distribution: standard normals
/// through the Cholesky factor of the covariance `[[a, b], [b, d]]`
/// (`multivariate_normal(mean, cov, size=n)`), as x and y values.
fn bivariate_normal(
    rng: &mut SpecRng,
    n: usize,
    mean: (f64, f64),
    [a, b, d]: [f64; 3],
) -> (Vec<f64>, Vec<f64>) {
    let l11 = a.sqrt();
    let l21 = b / l11;
    let l22 = (d - l21 * l21).sqrt();
    let z = rng.normal(2 * n, 0.0, 1.0);
    let (pairs, _) = z.as_chunks::<2>();
    pairs
        .iter()
        .map(|&[z0, z1]| (mean.0 + l11 * z0, mean.1 + l21 * z0 + l22 * z1))
        .unzip()
}

fn power_norm(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let (mut x, mut y) = bivariate_normal(&mut rng, 100_000, (10.0, 10.0), [3.0, 2.0, 3.0]);
    let (x2, y2) = bivariate_normal(&mut rng, 1000, (30.0, 20.0), [3.0, 1.0, 3.0]);
    x.extend(x2);
    y.extend(y2);
    let histogram = |colorizer: ColorizerOptions| Hist2dOptions {
        histogram: Histogram2dOptions {
            bins: [AxisBins::Count(100), AxisBins::Count(100)],
            ..Histogram2dOptions::default()
        },
        mesh: PcolorOptions {
            colorizer,
            ..PcolorOptions::default()
        },
        ..Hist2dOptions::default()
    };
    let axs = fig.subplots(2, 2)?;
    let axs = axs.as_slice();
    axs[0].set_title("Linear normalization");
    axs[0].hist2d(&x, &y, histogram(ColorizerOptions::default()))?;
    for (ax, gamma) in axs[1..].iter().zip([0.8, 0.5, 0.3]) {
        ax.set_title(format!(r"Power law $(\gamma={})$", pyfmt::fixed(gamma, 1)));
        ax.hist2d(
            &x,
            &y,
            histogram(ColorizerOptions::default().norm(PowerNorm::new(gamma))),
        )?;
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/asinh_demo_1", asinh_demo_1),
    FigureCase::new("gallery_ticks/asinh_demo_2", asinh_demo_2),
    FigureCase::new("gallery_ticks/asinh_demo_3", asinh_demo_3),
    FigureCase::new("gallery_ticks/aspect_loglog", aspect_loglog),
    FigureCase::new("gallery_ticks/scales", scales).sized(6.0, 8.0, 100.0),
    FigureCase::new("gallery_ticks/logit_demo", logit_demo).sized(6.4, 8.5, 100.0),
    FigureCase::new("gallery_ticks/power_norm", power_norm),
];

#[cfg(test)]
mod tests {
    use super::erf;

    #[test]
    fn erf_matches_libm() {
        // Python's math.erf.
        for (x, expected) in [
            (0.0, 0.0),
            (0.5, 0.520_499_877_813_046_5),
            (1.0, 0.842_700_792_949_714_9),
            (-2.0, -0.995_322_265_018_952_7),
            (2.5, 0.999_593_047_982_555),
            (3.0, 0.999_977_909_503_001_4),
            (6.0, 1.0),
        ] {
            assert!((erf(x) - expected).abs() <= 4.0 * f64::EPSILON, "{x}");
        }
    }
}
