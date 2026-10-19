//! Small helpers several case groups share.

use std::f64::consts::PI;

use plt::axes::Axes;
use plt::axisartist::AuxTransform;
use plt::colors::Color;
use plt::contour::{ContourOptions, Levels};
use plt::figure::Figure;
use plt::fonts::{FontProperties, FontSize};
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
