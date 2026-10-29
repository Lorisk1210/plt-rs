//! Logit, asinh and function scales with their default and custom tickers.

use pltrs::axes::AxisName;
use pltrs::figure::Share;
use pltrs::layout::TightLayout;
use pltrs::prelude::*;
use pltrs::scale::{FuncScale, FuncTransform, ScaleOptions};
use pltrs::ticker::{FixedLocator, NullFormatter};

use crate::figures::CaseResult;
use crate::figures::cases::common::{inverse_mercator, logit_grid, mercator};

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
    let [logistic, laplacian, cauchy] = cdfs(&x);
    logit_grid(
        fig,
        &x,
        &[
            (logistic, "logistic"),
            (laplacian, "laplacian"),
            (cauchy, "cauchy"),
        ],
    )
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
        ScaleOptions {
            linear_width: Some(0.5),
            base: Some(2.0),
            ..ScaleOptions::default()
        },
    )?;
    fig.tight_layout(TightLayout::default())?;
    Ok(())
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
    ax.set_xlim(0.0, 180.0)?;
    ax.yaxis()
        .set_minor_formatter(NullFormatter)
        .set_major_locator(FixedLocator::new(arange(0.0, 90.0, 10.0)));
    Ok(())
}
