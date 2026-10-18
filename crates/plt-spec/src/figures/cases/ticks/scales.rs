//! Logit, asinh and function scales with their default and custom tickers.

use plt::axes::AxisName;
use plt::figure::Share;
use plt::layout::TightLayout;
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::scale::{FuncScale, FuncTransform, ScaleOptions};
use plt::ticker::{FixedLocator, NullFormatter};

use crate::figures::CaseResult;

/// `_cdfs(x)`: the logistic, Laplace and Cauchy distribution functions.
fn cdfs(x: &[f64]) -> [Vec<f64>; 3] {
    let map = |f: fn(f64) -> f64| x.iter().copied().map(f).collect::<Vec<_>>();
    [
        map(|x| 1.0 / (1.0 + (-x).exp())),
        map(|x| {
            if x < 0.0 {
                0.5 * x.exp()
            } else {
                1.0 - 0.5 * (-x).exp()
            }
        }),
        map(|x| x.atan() / std::f64::consts::PI + 0.5),
    ]
}

pub(super) fn logit_demo(fig: &Figure) -> CaseResult {
    let x = linspace(-10.0, 10.0, 1000);
    let curves = cdfs(&x);
    let axs = fig.subplots(3, 2)?;
    for ax in axs.as_slice() {
        for (y, label) in curves.iter().zip(["logistic", "laplacian", "cauchy"]) {
            ax.plot(&x, y)?.set_label(label);
        }
        ax.legend(LegendOptions::default())?;
        ax.grid(true);
    }
    let survival = ScaleOptions {
        one_half: Some("1/2".into()),
        use_overline: Some(true),
        ..ScaleOptions::default()
    };
    for (row, title) in [(0, "logit scale"), (1, "logit scale"), (2, "linear scale")] {
        for col in 0..2 {
            let ax = &axs[(row, col)];
            ax.set_title(title);
            match row {
                0 => {
                    ax.set_yscale("logit")?;
                }
                1 => {
                    ax.set_scale_with(AxisName::Y, "logit", &survival)?;
                }
                _ => {}
            }
            if col == 1 {
                ax.set_xlim(0.0, 10.0);
            }
            match (row, col) {
                (2, 0) => ax.set_ylim(0.0, 1.0),
                (2, 1) => ax.set_ylim(0.8, 1.0),
                (_, 0) => ax.set_ylim(1e-5, 1.0 - 1e-5),
                _ => ax.set_ylim(0.8, 1.0 - 5e-3),
            };
        }
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) fn asinh_demo(fig: &Figure) -> CaseResult {
    let x = linspace(-3.0, 600.0, 500);
    let axs = fig.subplots_shared(1, 3, Share::All, Share::None)?;
    for ax in axs.as_slice() {
        ax.plot(&x, &x)?;
        ax.grid(true);
    }
    axs[(0, 0)].set_title("symlog");
    axs[(0, 0)].set_yscale("symlog")?;
    axs[(0, 1)].set_title("asinh");
    axs[(0, 1)].set_yscale("asinh")?;
    axs[(0, 2)].set_title("asinh, base 2");
    axs[(0, 2)].set_scale_with(
        AxisName::Y,
        "asinh",
        &ScaleOptions {
            linear_width: Some(0.5),
            base: Some(2.0),
            ..ScaleOptions::default()
        },
    )?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// `_mercator(a)`: the Mercator projection of latitude `a` in degrees.
fn mercator(a: f64) -> f64 {
    let a = a.to_radians();
    (a.tan() + 1.0 / a.cos()).abs().ln().to_degrees()
}

/// `_inverse_mercator(a)`.
fn inverse_mercator(a: f64) -> f64 {
    a.to_radians().sinh().atan().to_degrees()
}

pub(super) fn function_scale_mercator(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = arange(0.0, 170.0, 0.1);
    let half: Vec<f64> = t.iter().map(|t| t / 2.0).collect();
    ax.plot_with(&t, half, LineProps::fmt("-")?.linewidth(2.0))?;
    let transform = FuncTransform::new(mercator, inverse_mercator);
    ax.set_scale(AxisName::Y, Box::new(FuncScale::from_transform(transform)));
    ax.set_title("function: Mercator");
    ax.grid(true);
    ax.set_xlim(0.0, 180.0);
    ax.yaxis()
        .set_minor_formatter(NullFormatter)
        .set_major_locator(FixedLocator::new(arange(0.0, 90.0, 10.0)));
    Ok(())
}
