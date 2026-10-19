//! Small helpers several case groups share.

use std::f64::consts::PI;

use plt::axes::{Axes, BarLabelOptions};
use plt::axisartist::AuxTransform;
use plt::colors::Color;
use plt::contour::{ContourOptions, Levels};
use plt::figure::{Figure, Mosaic, MosaicCell, MosaicError};
use plt::fonts::{FontProperties, FontSize};
use plt::numeric::linspace;
use plt::patches::{ArrowStyle, BoxStyle, ConnectionStyle, PatchProps};
use plt::text::{
    ArrowProps, FancyArrowProps, Language, LanguageRange, LegacyArrowProps, TextBoxProps,
};
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

/// `bbox=dict(boxstyle=style, **patch)`.
pub(super) fn styled_box(
    style: &str,
    patch: PatchProps,
) -> Result<Option<TextBoxProps>, Box<dyn std::error::Error>> {
    Ok(Some(
        TextBoxProps::new(style.parse::<BoxStyle>()?).with_patch(patch),
    ))
}

/// The legacy annotation arrow of matplotlib's examples and tests,
/// `arrowprops=dict(facecolor='black', shrink=0.05)`: black, shortened by
/// 5%.
pub(super) fn black_arrow() -> ArrowProps {
    ArrowProps::Legacy(LegacyArrowProps {
        shrink: 0.05,
        patch: PatchProps::default().facecolor("black"),
        ..LegacyArrowProps::default()
    })
}

/// `arrowprops=dict(arrowstyle=style, connectionstyle=connection)`.
pub(super) fn arrow(
    style: &str,
    connection: &str,
) -> Result<FancyArrowProps, Box<dyn std::error::Error>> {
    Ok(FancyArrowProps {
        connectionstyle: connection.parse::<ConnectionStyle>()?,
        ..FancyArrowProps::new(style.parse::<ArrowStyle>()?)
    })
}

/// A mosaic of labelled cells (`"."` for an empty one), as nested lists
/// of labels give it.
pub(super) fn mosaic(rows: &[&[&str]]) -> Result<Mosaic, MosaicError> {
    let cell = |label: &&str| match *label {
        "." => MosaicCell::Empty,
        label => MosaicCell::Label(label.to_owned()),
    };
    Mosaic::new(
        rows.iter()
            .map(|row| row.iter().map(cell).collect())
            .collect(),
    )
}
