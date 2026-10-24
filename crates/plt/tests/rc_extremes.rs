//! Drawing under extreme rcParams never panics: every parameter that
//! accepts zero, negative, tiny, huge, infinite or NaN values gets each of
//! them in turn, and a figure with lines, markers, bars, text and a legend
//! is rendered to every output format. Drawing may fail with an error;
//! it must not panic.

use std::panic::{AssertUnwindSafe, catch_unwind};

use plt::figure::{SaveFormat, SaveOptions};
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::rc::RcParams;

/// The values every parameter is tried with.
const VALUES: [&str; 10] = [
    "0", "-1", "-0.5", "1e6", "1e-9", "nan", "inf", "-inf", "100", "0.0001",
];

/// Draws the test figure to every format; `false` when `value` is not
/// valid for `key`.
fn draw_with(key: &str, value: &str) -> bool {
    plt::rc::context(|| {
        if plt::rc::set(key, value).is_err() {
            return false;
        }
        let fig = Figure::with_size(2.0, 1.5, 40.0);
        let ax = fig.subplot();
        let line = LineProps::default().label("a");
        let _ = ax.plot_with(vec![0.0, 1.0, 2.0], vec![1.0, 3.0, 2.0], line);
        let _ = ax.scatter(vec![0.5, 1.5], vec![2.0, 1.0]);
        let _ = ax.bar(vec![0.0, 1.0], [1.0, 2.0], BarOptions::default());
        ax.set_title("t $x^2$");
        ax.set_xlabel("x");
        let _ = ax.legend(LegendOptions::default());
        ax.text(0.5, 0.5, "text");
        for format in [
            SaveFormat::Png,
            SaveFormat::Svg,
            SaveFormat::Pdf,
            SaveFormat::Eps,
        ] {
            let _ = fig.to_format(format, SaveOptions::default());
        }
        true
    })
}

#[test]
#[ignore = "draws about 1700 figures; run with --ignored"]
fn extreme_parameters_never_panic() {
    let mut panics = Vec::new();
    for key in RcParams::KEYS {
        for value in VALUES {
            if catch_unwind(AssertUnwindSafe(|| draw_with(key, value))).is_err() {
                panics.push(format!("{key}={value}"));
            }
        }
    }
    assert!(panics.is_empty(), "panicked: {panics:#?}");
}
