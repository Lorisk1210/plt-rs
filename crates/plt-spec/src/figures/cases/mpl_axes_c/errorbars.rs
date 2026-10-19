//! Error bar tests: symmetric, asymmetric, log and polar error bars,
//! limits, colours from the property cycle, `errorevery`, missing values,
//! and the z-order of error bars and reference lines.

use std::f64::consts::PI;

use plt::axes::{ErrorSize, ErrorbarOptions, LocatorParams};
use plt::colors::Color;
use plt::figure::{Figure, Share};
use plt::lines::{LineProps, LineStyle, MarkEvery};
use plt::markers::{Marker, MarkerSymbol};
use plt::numeric::{arange, linspace};
use plt::rc::ColorOrAuto;
use plt::ticker::Nbins;

use super::{classic, map, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_c/errorbar_1", |fig| {
        let x = arange(0.1, 4.0, 0.5);
        let ax = fig.gca();
        let options = ErrorbarOptions {
            xerr: Some(0.2.into()),
            yerr: Some(0.4.into()),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, map(&x, |x| (-x).exp()), options)?;
        ax.set_title("Simplest errorbars, 0.2 in x, 0.4 in y");
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/errorbar_2", variable_errorbars),
    classic("mpl_axes_c/mixed_errorbar_polar_caps", polar_caps),
    classic("mpl_axes_c/errorbar_cycle_ecolor", |fig| {
        let x = arange(0.1, 4.0, 0.5);
        let ax = fig.subplot();
        for n in 0..4 {
            let y = map(&x, |x| (-x + f64::from(n)).exp());
            let options = ErrorbarOptions {
                yerr: Some(map(&y, |y| y * 0.25).into()),
                line: LineProps::fmt("o-")?,
                ecolor: Some(Color::from("black")),
                ..ErrorbarOptions::default()
            };
            ax.errorbar(&x, &y, options)?;
        }
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/errorbar_limits", limits),
    classic("mpl_axes_c/errorbar_with_prop_cycle", |fig| {
        let ax = fig.subplot();
        let x = [2.0, 4.0, 10.0];
        let styles = [
            ErrorbarOptions::default(),
            ErrorbarOptions {
                line: LineProps::default().color("tab:green"),
                ..ErrorbarOptions::default()
            },
            ErrorbarOptions {
                fmt: "tab:blue".into(),
                ..ErrorbarOptions::default()
            },
        ];
        for (low, style) in [0.0, 2.0, 4.0].into_iter().zip(styles) {
            let options = ErrorbarOptions {
                yerr: Some(0.5.into()),
                ..style
            };
            ax.errorbar(x, [low, low + 1.0, low + 2.0], options)?;
        }
        ax.set_xlim(1.0, 11.0);
        Ok(())
    })
    .with_rc_setup(|rc| {
        rc.set(
            "axes.prop_cycle",
            "cycler(ls=['--', ':', '-.'], marker=['s', 's', 'o'], mfc=['k', 'y', 'c'], \
             color=['b', 'g', 'r'])",
        )?;
        Ok(())
    }),
    classic("mpl_axes_c/errorbar_every", every),
    classic("mpl_axes_c/errorbar_nan", missing_values),
    classic("mpl_axes_c/errorbar_masked_negative", missing_values),
    FigureCase::new("mpl_axes_c/eb_line_zorder_1", |fig| {
        let ax = fig.gca();
        let x: Vec<f64> = (0..10).map(f64::from).collect();
        ax.plot_with(
            x.clone(),
            x,
            LineProps::default().linewidth(10.0).zorder(5.0),
        )?;
        let wide = |color: &str, zorder: f64| {
            LineProps::default()
                .color(color)
                .linewidth(10.0)
                .zorder(zorder)
        };
        ax.axhline(1.0, wide("red", 1.0));
        ax.axhline(5.0, wide("green", 10.0));
        ax.axvline(7.0, wide("m", 7.0));
        ax.axvline(2.0, wide("k", 3.0));
        ax.set_title("axvline and axhline zorder test");
        Ok(())
    }),
    FigureCase::new("mpl_axes_c/eb_line_zorder_2", |fig| {
        let ax = fig.gca();
        let x: Vec<f64> = (0..10).map(f64::from).collect();
        let options = ErrorbarOptions {
            yerr: Some(x.clone().into()),
            line: LineProps::default().linewidth(5.0).color("r").zorder(5.0),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, [0.0; 10], options)?;
        for j in 0..10 {
            let j = f64::from(j);
            let black = || LineProps::default().linewidth(5.0).color("k").zorder(j);
            ax.axhline(j, black());
            ax.axhline(-j, black());
        }
        ax.set_title("errorbar zorder test");
        Ok(())
    }),
];

/// The second figure of `test_errorbar`: symmetric, asymmetric and log
/// error bars on four axes sharing x.
fn variable_errorbars(fig: &Figure) -> CaseResult {
    let x = arange(0.1, 4.0, 0.5);
    let y = map(&x, |x| (-x).exp());
    let yerr = map(&x, |x| 0.1 + 0.2 * x.sqrt());
    let xerr = map(&yerr, |e| 0.1 + e);
    let double = |errors: &[f64]| map(errors, |e| 2.0 * e);
    let axs = fig.subplots_shared(2, 2, Share::All, Share::None)?;

    let ax = &axs[(0, 0)];
    let options = ErrorbarOptions {
        yerr: Some(yerr.clone().into()),
        fmt: "o".into(),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&x, &y, options)?;
    ax.set_title("Vert. symmetric");
    ax.locator_params(&LocatorParams {
        nbins: Some(Nbins::Fixed(4)),
        ..LocatorParams::default()
    })?;

    let ax = &axs[(0, 1)];
    let options = ErrorbarOptions {
        xerr: Some(xerr.clone().into()),
        fmt: "o".into(),
        line: LineProps::default().alpha(0.4),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&x, &y, options)?;
    ax.set_title("Hor. symmetric w/ alpha");

    let ax = &axs[(1, 0)];
    let options = ErrorbarOptions {
        yerr: Some(ErrorSize::Asymmetric {
            lower: yerr.clone(),
            upper: double(&yerr),
        }),
        xerr: Some(ErrorSize::Asymmetric {
            lower: xerr.clone(),
            upper: double(&xerr),
        }),
        fmt: "--o".into(),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&x, &y, options)?;
    ax.set_title("H, V asymmetric");

    let ax = &axs[(1, 1)];
    ax.set_yscale("log")?;
    let yerr_lower = y
        .iter()
        .zip(&yerr)
        .map(|(y, e)| y - (y - e).max(1e-2))
        .collect();
    let options = ErrorbarOptions {
        yerr: Some(ErrorSize::Asymmetric {
            lower: yerr_lower,
            upper: double(&yerr),
        }),
        xerr: Some(xerr.into()),
        fmt: "o".into(),
        ecolor: Some(Color::from("g")),
        capthick: Some(2.0),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&x, &y, options)?;
    ax.set_title("Mixed sym., log y");
    ax.set_ylim(1e-2, 1e1);

    fig.suptitle("Variable errorbars");
    Ok(())
}

/// `test_mixed_errorbar_polar_caps`: symmetric, long, asymmetric and
/// overlapping error bars on polar axes.
fn polar_caps(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let point = |xerr: ErrorSize, yerr: ErrorSize| ErrorbarOptions {
        xerr: Some(xerr),
        yerr: Some(yerr),
        fmt: "o".into(),
        ..ErrorbarOptions::default()
    };
    ax.errorbar([1.0, 2.0, 3.0], [0.9; 3], point(0.35.into(), 0.2.into()))?;
    ax.errorbar(
        [PI / 2.0 + 0.1, PI + 0.1],
        [1.8, 2.2],
        point((0.8 * PI).into(), 0.15.into()),
    )?;
    let asymmetric = point(
        ErrorSize::Asymmetric {
            lower: vec![0.3, 0.3, 0.2],
            upper: vec![0.2, 0.3, 0.3],
        },
        ErrorSize::Asymmetric {
            lower: vec![0.35, 0.5, 0.5],
            upper: vec![0.5, 0.35, 0.5],
        },
    );
    ax.errorbar(
        [4.0 * PI / 3.0 + 0.1, 5.0 * PI / 3.0 + 0.1, 2.0 * PI - 0.1],
        [1.1; 3],
        asymmetric,
    )?;
    ax.errorbar([2.1], [3.1], point(10.0.into(), 0.2.into()))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `n` flags, set at `indices`.
fn flags(n: usize, indices: &[usize]) -> Vec<bool> {
    (0..n).map(|i| indices.contains(&i)).collect()
}

/// `test_errorbar_limits`: upper and lower limits in y and in x.
fn limits(fig: &Figure) -> CaseResult {
    let x = arange(0.5, 5.5, 0.5);
    let n = x.len();
    let shifted = |dy: f64| map(&x, |x| (-x).exp() + dy);
    let ax = fig.subplot();
    let dotted = |color: &str| {
        LineProps::default()
            .linestyle(LineStyle::Dotted)
            .color(color)
    };
    let bars = |line: LineProps| ErrorbarOptions {
        xerr: Some(0.1.into()),
        yerr: Some(0.2.into()),
        line,
        ..ErrorbarOptions::default()
    };
    let upper = flags(n, &[1, 5, 9]);
    let lower = flags(n, &[2, 4, 8]);

    ax.errorbar(&x, shifted(0.0), bars(dotted("blue")))?;
    let options = ErrorbarOptions {
        uplims: upper.clone(),
        ..bars(dotted("green"))
    };
    ax.errorbar(&x, shifted(0.5), options)?;
    let options = ErrorbarOptions {
        lolims: lower.clone(),
        ..bars(dotted("red"))
    };
    ax.errorbar(&x, shifted(1.0), options)?;
    let options = ErrorbarOptions {
        lolims: lower.clone(),
        uplims: upper.clone(),
        ..bars(
            dotted("magenta")
                .marker(Marker::Symbol(MarkerSymbol::Circle))
                .markersize(6.0),
        )
    };
    ax.errorbar(&x, shifted(1.5), options)?;

    let mut yerr = vec![0.2; n];
    yerr[3] = 0.3;
    yerr[6] = 0.3;
    let options = ErrorbarOptions {
        xerr: Some(0.2.into()),
        yerr: Some(yerr.into()),
        xlolims: lower,
        xuplims: upper,
        uplims: flags(n, &[3]),
        lolims: flags(n, &[6]),
        capsize: Some(0.0),
        line: LineProps::default()
            .marker(Marker::Symbol(MarkerSymbol::Circle))
            .markersize(6.0)
            .linestyle(LineStyle::None)
            .markeredgecolor(ColorOrAuto::Color("blue".into()))
            .color("cyan"),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&x, shifted(2.1), options)?;
    ax.set_xlim(0.0, 5.5);
    ax.set_title("Errorbar upper and lower limits");
    Ok(())
}

/// `test_errorbar_every`: `errorevery` as offset and step, `markevery`
/// passed to the data line, slices and masks.
fn every(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 1.0, 15);
    let mut y = map(&x, |x| x * (1.0 - x));
    let yerr = map(&y, |y| y / 6.0);
    let ax = fig.subplot();
    let capped = |line: LineProps| ErrorbarOptions {
        yerr: Some(yerr.clone().into()),
        capsize: Some(4.0),
        line,
        ..ErrorbarOptions::default()
    };
    for (color, shift) in ["r", "g", "b", "k"].into_iter().zip([0, 0, 2, 7]) {
        y = map(&y, |y| y + 0.02);
        let options = ErrorbarOptions {
            errorevery: Some(MarkEvery::Every {
                start: shift,
                step: 4,
            }),
            ..capped(LineProps::default().color(color))
        };
        ax.errorbar(&x, &y, options)?;
    }
    let options = ErrorbarOptions {
        fmt: "o".into(),
        ..capped(LineProps::default().markevery(Some(MarkEvery::Every { start: 1, step: 4 })))
    };
    ax.errorbar(&x, map(&y, |y| y + 0.1), options)?;
    let selections = [
        (
            "C0",
            MarkEvery::Slice {
                start: Some(2),
                stop: None,
                step: Some(3),
            },
        ),
        ("C1", MarkEvery::Mask([false, true, false].repeat(5))),
    ];
    for (color, selection) in selections {
        let line = LineProps::default()
            .color(color)
            .markevery(Some(selection.clone()));
        let options = ErrorbarOptions {
            fmt: "o".into(),
            errorevery: Some(selection),
            ..capped(line)
        };
        ax.errorbar(&x, map(&y, |y| y + 0.2), options)?;
    }
    Ok(())
}

/// `test_errorbar_nan` and `test_errorbar_masked_negative`: missing
/// values (NaN, or masked) skip their points and bars.
fn missing_values(fig: &Figure) -> CaseResult {
    let options = ErrorbarOptions {
        yerr: Some(vec![4.0, 5.0, f64::NAN, f64::NAN, 6.0].into()),
        ..ErrorbarOptions::default()
    };
    fig.subplot().errorbar(
        [0.0, 1.0, 2.0, 3.0, 4.0],
        [1.0, 2.0, f64::NAN, f64::NAN, 3.0],
        options,
    )?;
    Ok(())
}
