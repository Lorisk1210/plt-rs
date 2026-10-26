//! Runtime configuration: random text for every rcParam (mutations of its
//! default, numbers at the edges, soups of punctuation), figures drawn
//! under the values accepted, and random rc files and style sheets.

use plt::figure::{Figure, SaveFormat};
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::rc::{RcFile, RcParams, parse_rc_text};
use plt::style::Style;

use crate::harness::{Gen, fuzz};
use crate::support::{format, render};

/// Pieces of rc values: list and tuple syntax, quotes, keywords.
const VALUE_PIECES: [&str; 24] = [
    ",", " ", "(", ")", "[", "]", "'", "\"", "#", ":", "-", ".", "none", "auto", "True", "False",
    "C0", "red", "0", "1", "e", "cycler(", "=", "\\",
];

/// A value for `key`: its default, mutated, or a soup.
fn value_for(g: &mut Gen, defaults: &RcParams, key: &str) -> String {
    let default = defaults.get(key).unwrap_or_default();
    match g.below(4) {
        0 => g.soup(&VALUE_PIECES, 10),
        1 => g.number_text(),
        _ => g.mutate(&default, &VALUE_PIECES),
    }
}

/// Assignments to random rcParams.
fn assignments(g: &mut Gen, count: usize) -> Vec<(&'static str, String)> {
    let defaults = RcParams::matplotlib();
    (0..count)
        .map(|_| {
            let key = g.pick(RcParams::KEYS);
            (key, value_for(g, &defaults, key))
        })
        .collect()
}

#[test]
fn rc_values() {
    fuzz(
        "rc values",
        10_000,
        |g| assignments(g, 1).remove(0),
        |(key, value)| {
            let mut rc = RcParams::matplotlib();
            if rc.set(key, value).is_ok() {
                // What `get` prints must read back, but for infinities and
                // NaN in sequences: matplotlib prints them as Python does,
                // which `literal_eval` does not read either.
                let text = rc.get(key).expect("a known key");
                let non_finite = text.contains("inf") || text.contains("nan");
                assert!(
                    non_finite || rc.set(key, &text).is_ok(),
                    "{key}: {text:?} does not read back"
                );
            }
        },
    );
}

/// Draws a small figure with lines, markers, bars, text, mathtext, an
/// image, a colorbar and a legend.
pub(crate) fn draw_standard_figure(format: SaveFormat) {
    let fig = Figure::with_size(2.0, 1.5, 40.0);
    let ax = fig.subplot();
    let line = LineProps::default().label("a");
    let _ = ax.plot_with(vec![0.0, 1.0, 2.0], vec![1.0, 3.0, 2.0], line);
    let _ = ax.scatter(vec![0.5, 1.5], vec![2.0, 1.0]);
    let _ = ax.bar(vec![0.0, 1.0], [1.0, 2.0], BarOptions::default());
    ax.set_title("t $x^2$");
    ax.set_xlabel("x");
    ax.text(0.5, 0.5, "text");
    let image = plt::numeric::Grid::from_fn(3, 3, |i, j| (i * j) as f64);
    if let Ok(image) = ax.imshow(image, plt::axes::ImshowOptions::default()) {
        let _ = fig.colorbar(&image, plt::colorbar::ColorbarOptions::default());
    }
    let _ = ax.legend(LegendOptions::default());
    render(&fig, format);
}

#[test]
fn drawing_under_random_rc() {
    fuzz(
        "rc drawing",
        300,
        |g| {
            let count = 1 + g.below(3);
            (assignments(g, count), format(g))
        },
        |(assignments, format)| {
            plt::rc::context(|| {
                for (key, value) in assignments {
                    let _ = plt::rc::set(key, value);
                }
                draw_standard_figure(*format);
            });
        },
    );
}

/// A style: a bundled sheet by name (maybe mangled) or random parameters.
#[derive(Debug)]
enum StyleSpec {
    Named(String),
    Params(Vec<(&'static str, String)>),
}

#[test]
fn style_sheets() {
    let names = plt::style::available();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    fuzz(
        "styles",
        200,
        |g| {
            let styles: Vec<StyleSpec> = (0..=g.below(3))
                .map(|_| {
                    if g.chance(0.6) {
                        StyleSpec::Named(g.text(&names, &["-", "_", "/", ".", "file:", "seaborn"]))
                    } else {
                        let count = 1 + g.below(4);
                        StyleSpec::Params(assignments(g, count))
                    }
                })
                .collect();
            (styles, format(g))
        },
        |(styles, format)| {
            let styles: Vec<Style> = styles
                .iter()
                .map(|style| match style {
                    StyleSpec::Named(name) => Style::Named(name.clone()),
                    StyleSpec::Params(pairs) => {
                        let pairs: Vec<(&str, &str)> = pairs
                            .iter()
                            .map(|(key, value)| (*key, value.as_str()))
                            .collect();
                        Style::params(&pairs)
                    }
                })
                .collect();
            plt::rc::context(|| {
                for style in &styles {
                    let _ = plt::style::use_styles(std::slice::from_ref(style));
                }
                draw_standard_figure(*format);
            });
        },
    );
}

/// The text of an rc file: assignments to random rcParams (some without a
/// colon, some soups of punctuation), one per line.
pub(crate) fn rc_text(g: &mut Gen) -> String {
    let defaults = RcParams::matplotlib();
    let lines = g.len(8);
    let mut text = String::new();
    for _ in 0..lines {
        let key = g.pick(RcParams::KEYS);
        let value = value_for(g, &defaults, key);
        let line = match g.below(6) {
            0 => g.soup(&[":", "#", "\"", " ", "\t", "key", "\\"], 8),
            1 => format!("{key} {value}"),
            _ => format!("{key}: {value}"),
        };
        text.push_str(&line);
        text.push(g.pick(&['\n', '\n', '\r']));
    }
    text
}

#[test]
fn rc_files() {
    fuzz("rc files", 5_000, rc_text, |text| {
        let _ = parse_rc_text(text);
        let _ = RcFile::parse(text);
        let _ = RcParams::matplotlib().apply_rc_text(text);
    });
}
