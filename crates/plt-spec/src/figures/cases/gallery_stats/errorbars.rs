//! The error bars of the section: constant, variable and asymmetric
//! errors, upper and lower limits, subsampled error bars and error boxes
//! drawn as a patch collection.

use plt::axes::{ErrorSize, ErrorbarOptions};
use plt::collections::{CollectionPatch, CollectionProps, CollectionSpec};
use plt::figure::Share;
use plt::legend::LegendOptions;
use plt::offsetbox::AnchorLoc;
use plt::patches::Rectangle;
use plt::prelude::*;

use super::SEED;
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Error bars of `yerr` and nothing else changed.
fn yerr(errors: impl Into<ErrorSize>) -> ErrorbarOptions {
    ErrorbarOptions {
        yerr: Some(errors.into()),
        ..ErrorbarOptions::default()
    }
}

/// `x = np.arange(0.1, 4, 0.5)` and `y = np.exp(-x)`.
fn decay() -> (Vec<f64>, Vec<f64>) {
    let x = arange(0.1, 4.0, 0.5);
    let y = map(&x, |x| (-x).exp());
    (x, y)
}

fn errorbar(fig: &Figure) -> CaseResult {
    let (x, y) = decay();
    let options = ErrorbarOptions {
        xerr: Some(0.2.into()),
        ..yerr(0.4)
    };
    fig.subplot().errorbar(&x, y, options)?;
    Ok(())
}

fn errorbar_features(fig: &Figure) -> CaseResult {
    let (x, y) = decay();
    let error = map(&x, |x| 0.1 + 0.2 * x);
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (ax0, ax1) = (&axs[(0, 0)], &axs[(1, 0)]);
    let symmetric = ErrorbarOptions {
        fmt: "-o".parse()?,
        ..yerr(error.clone())
    };
    ax0.errorbar(&x, &y, symmetric)?;
    ax0.set_title("variable, symmetric error");
    let asymmetric = ErrorbarOptions {
        xerr: Some(ErrorSize::Asymmetric {
            lower: map(&error, |e| 0.4 * e),
            upper: error,
        }),
        fmt: "o".parse()?,
        ..ErrorbarOptions::default()
    };
    ax1.errorbar(&x, &y, asymmetric)?;
    ax1.set_title("variable, asymmetric error");
    ax1.set_yscale("log")?;
    Ok(())
}

/// Flags from `0`/`1` digits.
fn flags(digits: [u8; 10]) -> Vec<bool> {
    digits.iter().map(|&digit| digit == 1).collect()
}

fn errorbar_limits(fig: &Figure) -> CaseResult {
    let x = arange(0.5, 5.5, 0.5);
    let y = map(&x, |x| (-x).exp());
    let lolims = flags([0, 0, 1, 0, 1, 0, 0, 0, 1, 0]);
    let uplims = flags([0, 1, 0, 0, 0, 1, 0, 0, 0, 1]);
    let dotted = || ErrorbarOptions {
        xerr: Some(0.1.into()),
        line: LineProps::default().linestyle(LineStyle::Dotted),
        ..yerr(0.2)
    };
    let big_dots = || {
        LineProps::default()
            .marker(Marker::Symbol(MarkerSymbol::Circle))
            .markersize(8.0)
    };
    let ax = fig.subplot();
    let shifted = |dy: f64| map(&y, |y| y + dy);
    ax.errorbar(&x, &y, dotted())?;
    let upper = ErrorbarOptions {
        uplims: uplims.clone(),
        ..dotted()
    };
    ax.errorbar(&x, shifted(0.5), upper)?;
    let lower = ErrorbarOptions {
        lolims: lolims.clone(),
        ..dotted()
    };
    ax.errorbar(&x, shifted(1.0), lower)?;
    let both = ErrorbarOptions {
        lolims: lolims.clone(),
        uplims: uplims.clone(),
        line: big_dots().linestyle(LineStyle::Dotted),
        ..dotted()
    };
    ax.errorbar(&x, shifted(1.5), both)?;
    let mut errors = vec![0.2; 10];
    errors[3] = 0.3;
    errors[6] = 0.3;
    let only_at = |index: usize| (0..10).map(|i| i == index).collect();
    let crossed = ErrorbarOptions {
        xerr: Some(0.2.into()),
        xlolims: lolims,
        xuplims: uplims,
        uplims: only_at(3),
        lolims: only_at(6),
        line: big_dots().linestyle(LineStyle::None),
        ..yerr(errors)
    };
    ax.errorbar(&x, shifted(2.1), crossed)?;
    ax.set_xlim(0.0, 5.5)?;
    ax.set_title("Errorbar upper and lower limits");
    Ok(())
}

/// `[True, False] * 5` and `[False, True] * 5`.
fn alternating() -> (Vec<bool>, Vec<bool>) {
    let upper: Vec<bool> = (0..10).map(|i| i % 2 == 0).collect();
    let lower = upper.iter().map(|flag| !flag).collect();
    (upper, lower)
}

/// Error bar options labelled `label`.
fn labelled(options: ErrorbarOptions, label: &str) -> ErrorbarOptions {
    ErrorbarOptions {
        line: options.line.clone().label(label),
        ..options
    }
}

fn errorbar_limits_simple_1(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 10.0, 1.0);
    let y = map(&x, |x| 2.5 * (x / 20.0 * std::f64::consts::PI).sin());
    let errors = linspace(0.05, 0.2, 10);
    let (upper, lower) = alternating();
    let ax = fig.subplot();
    let shifted = |dy: f64| map(&y, |y| y + dy);
    let bars = || yerr(errors.clone());
    ax.errorbar(&x, shifted(3.0), labelled(bars(), "both limits (default)"))?;
    let up = ErrorbarOptions {
        uplims: vec![true],
        ..bars()
    };
    ax.errorbar(&x, shifted(2.0), labelled(up, "uplims=True"))?;
    let up_and_low = ErrorbarOptions {
        uplims: vec![true],
        lolims: vec![true],
        ..bars()
    };
    ax.errorbar(
        &x,
        shifted(1.0),
        labelled(up_and_low, "uplims=True, lolims=True"),
    )?;
    let subsets = ErrorbarOptions {
        uplims: upper,
        lolims: lower,
        ..bars()
    };
    ax.errorbar(&x, &y, labelled(subsets, "subsets of uplims and lolims"))?;
    ax.legend(LegendOptions::default().loc(AnchorLoc::LowerRight))?;
    Ok(())
}

fn errorbar_limits_simple_2(fig: &Figure) -> CaseResult {
    let x = map(&arange(0.0, 10.0, 1.0), |x| x / 10.0);
    let power = |p: i32| map(&x, |x| (x + 0.1).powi(p));
    let (upper, lower) = alternating();
    let bars = || ErrorbarOptions {
        xerr: Some(0.1.into()),
        ..ErrorbarOptions::default()
    };
    let ax = fig.subplot();
    let right = ErrorbarOptions {
        xlolims: vec![true],
        ..bars()
    };
    ax.errorbar(&x, power(2), labelled(right, "xlolims=True"))?;
    let subsets = ErrorbarOptions {
        xuplims: upper,
        xlolims: lower,
        ..bars()
    };
    ax.errorbar(
        map(&x, |x| x + 0.6),
        power(3),
        labelled(subsets, "subsets of xuplims and xlolims"),
    )?;
    let left = ErrorbarOptions {
        xuplims: vec![true],
        ..bars()
    };
    ax.errorbar(
        map(&x, |x| x + 1.2),
        power(4),
        labelled(left, "xuplims=True"),
    )?;
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn errorbar_subsample(fig: &Figure) -> CaseResult {
    let x = arange(0.1, 4.0, 0.1);
    let y1 = map(&x, |x| (-x).exp());
    let y2 = map(&x, |x| (-0.5 * x).exp());
    let y1err = map(&x, |x| 0.1 + 0.1 * x.sqrt());
    let y2err = map(&x, |x| 0.1 + 0.1 * (x / 2.0).sqrt());
    let axs = fig.subplots_shared(1, 3, Share::All, Share::None)?;
    let panels = [
        ("all errorbars", None, None),
        (
            "only every 6th errorbar",
            Some(MarkEvery::Every { start: 0, step: 6 }),
            Some(MarkEvery::Every { start: 0, step: 6 }),
        ),
        (
            "second series shifted by 3",
            Some(MarkEvery::Every { start: 0, step: 6 }),
            Some(MarkEvery::Every { start: 3, step: 6 }),
        ),
    ];
    for (ax, (title, every1, every2)) in axs.as_slice().iter().zip(panels) {
        ax.set_title(title);
        for (y, err, errorevery) in [(&y1, &y1err, every1), (&y2, &y2err, every2)] {
            let options = ErrorbarOptions {
                errorevery,
                ..yerr(err.clone())
            };
            ax.errorbar(&x, y, options)?;
        }
    }
    fig.suptitle("Errorbar subsampling");
    Ok(())
}

fn errorbars_and_boxes(fig: &Figure) -> CaseResult {
    let n = 5;
    let mut g = SpecRng::new(SEED);
    let x = arange(0.0, 5.0, 1.0);
    let y = map(&g.uniform(n, 0.0, 1.0), |u| u * 5.0);
    let xerr = map(&g.uniform(2 * n, 0.0, 1.0), |u| u + 0.1);
    let yerr = map(&g.uniform(2 * n, 0.0, 1.0), |u| u + 0.2);
    let (x_below, x_above) = xerr.split_at(n);
    let (y_below, y_above) = yerr.split_at(n);
    let ax = fig.subplot();
    let errorboxes: Vec<CollectionPatch> = (0..n)
        .map(|i| {
            let corner = (x[i] - x_below[i], y[i] - y_below[i]);
            let rectangle =
                Rectangle::new(corner, x_below[i] + x_above[i], y_below[i] + y_above[i]);
            CollectionPatch::new(&rectangle, PatchProps::default())
        })
        .collect();
    let boxes = CollectionSpec::patches(&errorboxes, false)?.with_props(
        CollectionProps::default()
            .facecolor("r")
            .alpha(0.5)
            .edgecolor("none"),
    );
    ax.add_collection(boxes);
    let options = ErrorbarOptions {
        xerr: Some(ErrorSize::Asymmetric {
            lower: x_below.to_vec(),
            upper: x_above.to_vec(),
        }),
        yerr: Some(ErrorSize::Asymmetric {
            lower: y_below.to_vec(),
            upper: y_above.to_vec(),
        }),
        fmt: "none".parse()?,
        ecolor: Some("k".into()),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&x, &y, options)?;
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/errorbar", errorbar),
    FigureCase::new("gallery_stats/errorbar_features", errorbar_features),
    FigureCase::new("gallery_stats/errorbar_limits", errorbar_limits).sized(7.0, 4.0, 100.0),
    FigureCase::new(
        "gallery_stats/errorbar_limits_simple_1",
        errorbar_limits_simple_1,
    ),
    FigureCase::new(
        "gallery_stats/errorbar_limits_simple_2",
        errorbar_limits_simple_2,
    ),
    FigureCase::new("gallery_stats/errorbar_subsample", errorbar_subsample).sized(12.0, 6.0, 100.0),
    FigureCase::new("gallery_stats/errorbars_and_boxes", errorbars_and_boxes),
];
