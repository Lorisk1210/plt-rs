//! Small helpers several case groups share.

use std::f64::consts::PI;

use plt::axes::{Axes, BarLabelOptions};
use plt::axisartist::AuxTransform;
use plt::colors::Color;
use plt::contour::{ContourOptions, Levels};
use plt::figure::Figure;
use plt::fonts::{FontProperties, FontSize};
use plt::numeric::linspace;
use plt::text::{Language, LanguageRange};
use plt::transforms::Affine2D;

/// Character ranges of a language.
pub(super) fn ranges(ranges: &[(&str, usize, usize)]) -> Language {
    ranges
        .iter()
        .map(|&(tag, start, end)| LanguageRange::new(tag, start, end))
        .collect()
}

/// The default font at `points`.
pub(super) fn sized(points: f64) -> FontProperties {
    let rc = plt::rc::params();
    FontProperties::from_rc(&rc).with_size(FontSize::Points(points), rc.font_size)
}

/// `Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform()`.
pub(super) fn polar_degrees() -> AuxTransform {
    AuxTransform::affine(Affine2D::scaling(PI / 180.0, 1.0)).then(AuxTransform::polar())
}

/// Takes the ticks off `ax` (`ax.set(xticks=[], yticks=[])`).
pub(super) fn tickless(ax: &Axes) {
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
}

/// `f` applied to each value.
pub(super) fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}

/// Running sums of `values` (`np.cumsum`).
pub(super) fn cumsum(values: &[f64]) -> Vec<f64> {
    values
        .iter()
        .scan(0.0, |sum, v| {
            *sum += v;
            Some(*sum)
        })
        .collect()
}

/// `bar_label` options with only `padding` set.
pub(super) fn padded(padding: f64) -> BarLabelOptions {
    BarLabelOptions {
        padding: vec![padding],
        ..BarLabelOptions::default()
    }
}

/// The raised sine of matplotlib's `markevery` tests and demo: 200 points
/// of `sin(x) + 1.11` for `x` in `[0.11, 9.89]` (`_markevery_sine`).
pub(super) fn markevery_sine() -> (Vec<f64>, Vec<f64>) {
    let delta = 0.11;
    let x = map(&linspace(0.0, 10.0 - 2.0 * delta, 200), |x| x + delta);
    let y = map(&x, |x| x.sin() + 1.0 + delta);
    (x, y)
}

/// One axes with both limits `(0, 10)` (`plt.axis([0, 10, 0, 10])`).
pub(super) fn blank(fig: &Figure) -> Axes {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    ax
}

/// About `n` automatic contour levels (`levels=n`).
pub(super) fn count(n: i64) -> ContourOptions {
    ContourOptions {
        levels: Levels::Count(n),
        ..ContourOptions::default()
    }
}

/// Explicit contour levels (`levels=[...]`).
pub(super) fn values(levels: &[f64]) -> ContourOptions {
    ContourOptions {
        levels: Levels::Values(levels.to_vec()),
        ..ContourOptions::default()
    }
}

/// Colours by name (`colors=[...]`).
pub(super) fn colors(names: &[&str]) -> Vec<Color> {
    names.iter().map(|&name| Color::from(name)).collect()
}

/// Hatch patterns, `None` leaving a band plain (`hatches=[...]`).
pub(super) fn hatches(patterns: &[Option<&str>]) -> Vec<Option<String>> {
    patterns.iter().map(|p| p.map(str::to_owned)).collect()
}
