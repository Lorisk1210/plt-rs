//! Twin and shared axes cases of `figures/cases_axes.py`.

use plt::axes::SecondaryLocation;
use plt::figure::Share;
use plt::gridspec::SubplotOverrides;
use plt::prelude::*;
use plt::scale::FuncTransform;
use plt::spines::SpineSide;

use super::{line, of_t, t};
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("axes/twinx", |fig| {
        let ax = fig.subplot();
        fig.subplots_adjust(&SubplotOverrides {
            right: Some(0.85),
            ..SubplotOverrides::default()
        })?;
        ax.plot_with(
            t(),
            of_t(|t| (t / 3.0).exp()),
            LineProps::default().color("tab:blue"),
        )?;
        ax.set_ylabel("exp").yaxis().set_label_color("tab:blue");
        let ax2 = ax.twinx()?;
        ax2.plot_with(t(), of_t(f64::sin), LineProps::default().color("tab:red"))?;
        ax2.set_ylabel("sin").yaxis().set_label_color("tab:red");
        Ok(())
    }),
    FigureCase::new("axes/twiny", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.set_xlabel("bottom");
        let ax2 = ax.twiny()?;
        ax2.plot_fmt(of_t(|t| t * 100.0), of_t(f64::cos), "C1")?;
        ax2.set_xlabel("top");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/twinx_in_front_zorder", |fig| {
        let ax = fig.subplot();
        let bars = BarOptions {
            color: vec![Color::from("0.8")],
            ..BarOptions::default()
        };
        ax.bar(arange(0.0, 5.0, 1.0), [3.0, 4.0, 2.0, 5.0, 1.0], bars)?;
        let ax2 = ax.twinx()?;
        ax2.plot_with(
            arange(0.0, 5.0, 1.0),
            [10.0, 30.0, 20.0, 40.0, 25.0],
            LineProps::fmt("o-")?.linewidth(3.0),
        )?;
        ax.set_zorder(ax2.zorder() + 1.0);
        ax.set_patch_visible(false);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/secondary_xaxis_functions", |fig| {
        let ax = fig.subplot();
        let deg = linspace(0.0, 360.0, 50);
        let wave: Vec<f64> = deg.iter().map(|d| d.to_radians().sin()).collect();
        ax.plot(&deg, wave)?;
        ax.set_xlabel("degrees");
        let radians = FuncTransform::new(f64::to_radians, f64::to_degrees);
        let sec = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), Some(radians))?;
        sec.set_xlabel("radians");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/secondary_yaxis_location", |fig| {
        let ax = fig.subplot();
        fig.subplots_adjust(&SubplotOverrides {
            right: Some(0.85),
            ..SubplotOverrides::default()
        })?;
        ax.plot(arange(0.0, 20.0, 1.0), linspace(-10.0, 40.0, 20))?;
        ax.set_ylabel("Celsius");
        let fahrenheit = FuncTransform::new(|c| c * 1.8 + 32.0, |f| (f - 32.0) / 1.8);
        let sec =
            ax.secondary_yaxis(SecondaryLocation::Side(SpineSide::Right), Some(fahrenheit))?;
        sec.set_ylabel("Fahrenheit");
        Ok(())
    }),
    FigureCase::new("axes/sharex_sharey_label_outer", |fig| {
        let axs = fig.subplots_shared(2, 2, Share::All, Share::All)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            let (shift, gain) = (i as f64, i as f64 + 1.0);
            ax.plot(t(), of_t(|t| (t + shift).sin() * gain))?;
            ax.set_xlabel("x");
            ax.set_ylabel("y");
            ax.label_outer();
        }
        Ok(())
    }),
    FigureCase::new("axes/sharex_after_creation", |fig| {
        let ax1 = fig.add_subplot(2, 1, 1)?;
        let ax2 = fig.add_subplot(2, 1, 2)?;
        ax2.share_x(&ax1)?;
        ax1.plot(t(), of_t(f64::sin))?;
        ax2.plot(of_t(|t| t * 0.5), of_t(f64::cos))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
