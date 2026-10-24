//! Scale cases of `figures/cases_axes.py`.

use plt::axes::AxisName;
use plt::gridspec::SubplotOverrides;
use plt::prelude::*;
use plt::scale::{FuncScale, FuncTransform, ScaleOptions};

use crate::figures::FigureCase;
use crate::figures::cases::common::map;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("axes/xscale_log_minor", |fig| {
        let ax = fig.subplot();
        let x = logspace(-1.0, 3.0, 50, 10.0);
        ax.plot(&x, map(&x, f64::log10))?;
        ax.set_xscale("log")?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/yscale_log_base2_small_range", |fig| {
        let ax = fig.subplot();
        ax.plot([1.0, 2.0, 3.0], [3.0, 5.0, 7.0])?;
        let base2 = ScaleOptions {
            base: Some(2.0),
            ..ScaleOptions::default()
        };
        ax.set_scale_with(AxisName::Y, "log", base2)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/yscale_log_narrow_range", |fig| {
        let ax = fig.subplot();
        ax.plot([1.0, 2.0, 3.0], [20.0, 50.0, 70.0])?;
        ax.set_yscale("log")?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/yscale_symlog", |fig| {
        let ax = fig.subplot();
        let x = linspace(-50.0, 50.0, 101);
        ax.plot(&x, map(&x, |x| x.powi(3)))?;
        let options = ScaleOptions {
            linthresh: Some(10.0),
            ..ScaleOptions::default()
        };
        ax.set_scale_with(AxisName::Y, "symlog", options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/xscale_symlog_linscale", |fig| {
        let ax = fig.subplot();
        let x = linspace(-1000.0, 1000.0, 201);
        ax.plot(&x, map(&x, |x| x.signum() * x.abs().sqrt()))?;
        let options = ScaleOptions {
            linthresh: Some(1.0),
            linscale: Some(0.5),
            ..ScaleOptions::default()
        };
        ax.set_scale_with(AxisName::X, "symlog", options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/yscale_logit", |fig| {
        let ax = fig.subplot();
        fig.subplots_adjust(&SubplotOverrides {
            left: Some(0.2),
            ..SubplotOverrides::default()
        })?;
        ax.plot(linspace(-7.0, 7.0, 100), linspace(0.001, 0.999, 100))?;
        ax.set_yscale("logit")?;
        Ok(())
    }),
    FigureCase::new("axes/xscale_asinh", |fig| {
        let ax = fig.subplot();
        let x = linspace(-500.0, 500.0, 101);
        ax.plot(&x, map(&x, f64::asinh))?;
        ax.set_xscale("asinh")?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/xscale_function_sqrt", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 100.0, 50);
        ax.plot(&x, &x)?;
        let sqrt = FuncTransform::new(f64::sqrt, |x| x * x);
        ax.set_scale(AxisName::X, Box::new(FuncScale::from_transform(sqrt)));
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
