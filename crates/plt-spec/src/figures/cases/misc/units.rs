//! Categorical and log-helper cases of `figures/cases_axes_misc.py`.

use plt::axes::{AxisName, ScatterOptions};
use plt::numeric::geomspace;
use plt::prelude::*;
use plt::scale::ScaleOptions;

use crate::figures::FigureCase;

/// Log scale options with this base.
fn base(base: f64) -> ScaleOptions {
    ScaleOptions {
        base: Some(base),
        ..ScaleOptions::default()
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/categorical_plot", |fig| {
        fig.subplot().plot_fmt(
            ["apple", "banana", "cherry", "date"],
            [3.0, 1.0, 4.0, 2.0],
            "o-",
        )?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("misc/categorical_bar_scatter_y", |fig| {
        let axs = fig.subplots(1, 2)?;
        let bars = BarOptions {
            color: ["C0", "C1", "C2"].map(Color::from).to_vec(),
            ..BarOptions::default()
        };
        axs[(0, 0)].bar(["a", "b", "c"], [5.0, 2.0, 7.0], bars)?;
        axs[(0, 1)].scatter_with(
            [1.0, 5.0, 3.0, 2.0],
            ["low", "high", "mid", "low"],
            ScatterOptions::default(),
        )?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("misc/categorical_mixed_updates", |fig| {
        let ax = fig.subplot();
        ax.plot(["a", "b", "c"], [1.0, 2.0, 3.0])?;
        ax.plot_fmt(["c", "d", "a", "e"], [3.0, 1.0, 2.0, 2.0], "s--")?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("misc/semilogx_base2", |fig| {
        let ax = fig.subplot();
        let x: Vec<f64> = (0..11).map(|i| 2f64.powi(i)).collect();
        ax.set_scale_with(AxisName::X, "log", base(2.0))?;
        ax.plot_fmt(&x, x.iter().map(|x| x.sqrt()).collect::<Vec<_>>(), "o-")?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("misc/semilogy_default", |fig| {
        let x = linspace(0.0, 5.0, 50);
        let y: Vec<f64> = x.iter().map(|x| (2.0 * x).exp()).collect();
        fig.subplot().semilogy(x, y)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("misc/loglog_mixed_bases", |fig| {
        let ax = fig.subplot();
        let x = geomspace(1e-2, 1e3, 40);
        let y: Vec<f64> = x.iter().map(|x| x.powf(1.5)).collect();
        ax.loglog(x, y)?;
        ax.set_scale_with(AxisName::Y, "log", base(2.0))?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("misc/symlog_linthresh", |fig| {
        let ax = fig.subplot();
        let x = linspace(-100.0, 100.0, 401);
        ax.plot(&x, x.iter().map(|x| x.powi(3)).collect::<Vec<_>>())?;
        let linear_x = ScaleOptions {
            linthresh: Some(1.0),
            ..ScaleOptions::default()
        };
        ax.set_scale_with(AxisName::X, "symlog", linear_x)?;
        let linear_y = ScaleOptions {
            linthresh: Some(10.0),
            linscale: Some(0.5),
            ..ScaleOptions::default()
        };
        ax.set_scale_with(AxisName::Y, "symlog", linear_y)?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
];
