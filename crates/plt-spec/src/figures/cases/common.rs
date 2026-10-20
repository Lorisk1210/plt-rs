//! Small helpers several case groups share.

use std::f64::consts::PI;

use plt::artist::ArtistHandle;
use plt::axes::{Axes, AxisName, BarLabelOptions, TickLevel, TickParams};
use plt::axisartist::AuxTransform;
use plt::colors::Color;
use plt::contour::{ContourOptions, Levels};
use plt::figure::{Figure, Mosaic, MosaicCell, MosaicError};
use plt::fonts::{FontProperties, FontSize};
use plt::layout::TightLayout;
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::numeric::linspace;
use plt::patches::{ArrowStyle, BoxStyle, ConnectionStyle, PatchProps};
use plt::scale::ScaleOptions;
use plt::spines::SpineSide;
use plt::text::{
    ArrowProps, FancyArrowProps, Language, LanguageRange, LegacyArrowProps, TextBoxProps,
};
use plt::ticker::{
    AutoLocator, AutoMinorLocator, FixedLocator, IndexLocator, LinearLocator, LogLocator,
    MaxNLocator, MultipleLocator, Nbins, NullLocator, NumTicks,
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

/// The bare strip of the gallery's tick demos (the start of `setup` in
/// `tick-locators` and `tick-formatters`, and of `plot_axis` in
/// `date_formatters_locators`): no y ticks, only the bottom spine, and
/// major and minor ticks 5 and 2.5 points long.
pub(super) fn tick_strip(ax: &Axes) {
    ax.yaxis().set_major_locator(NullLocator);
    let spines = ax.spines();
    for side in [SpineSide::Left, SpineSide::Right, SpineSide::Top] {
        spines.get(side).set_visible(false);
    }
    for (which, width, length) in [(TickLevel::Major, 1.0, 5.0), (TickLevel::Minor, 0.75, 2.5)] {
        ax.tick_params(&TickParams {
            which,
            width: Some(width),
            length: Some(length),
            ..TickParams::default()
        });
    }
}

/// The gallery's "Tick locators" figure: eight strips set up by `strip`
/// with their names, one per locator, in a tight layout.
pub(super) fn locator_strips(
    fig: &Figure,
    strip: impl Fn(&Axes, &str) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let axs = fig.subplots(8, 1)?;
    let ax = |i: usize| &axs[(i, 0)];
    strip(ax(0), "NullLocator()")?;
    ax(0)
        .xaxis()
        .set_major_locator(NullLocator)
        .set_minor_locator(NullLocator);
    strip(ax(1), "MultipleLocator(0.5, offset=0.2)")?;
    ax(1)
        .xaxis()
        .set_major_locator(MultipleLocator::new(0.5)?.with_offset(0.2))
        .set_minor_locator(MultipleLocator::new(0.1)?);
    strip(ax(2), "FixedLocator([0, 1, 5])")?;
    ax(2)
        .xaxis()
        .set_major_locator(FixedLocator::new(vec![0.0, 1.0, 5.0]))
        .set_minor_locator(FixedLocator::new(linspace(0.2, 0.8, 4)));
    strip(ax(3), "LinearLocator(numticks=3)")?;
    ax(3)
        .xaxis()
        .set_major_locator(LinearLocator::new(Some(3)))
        .set_minor_locator(LinearLocator::new(Some(31)));
    strip(ax(4), "IndexLocator(base=0.5, offset=0.25)")?;
    ax(4).plot_with(
        [0.0, 1.0, 2.0, 3.0, 4.0],
        [0.0; 5],
        LineProps::default().color("white"),
    )?;
    ax(4)
        .xaxis()
        .set_major_locator(IndexLocator::new(0.5, 0.25));
    strip(ax(5), "AutoLocator()")?;
    ax(5)
        .xaxis()
        .set_major_locator(AutoLocator::new())
        .set_minor_locator(AutoMinorLocator::new(None));
    strip(ax(6), "MaxNLocator(n=4)")?;
    ax(6)
        .xaxis()
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(4)))
        .set_minor_locator(MaxNLocator::new(Nbins::Fixed(40)));
    strip(ax(7), "LogLocator(base=10, numticks=15)")?;
    ax(7).set_xlim(1e3, 1e10);
    ax(7).set_xscale("log")?;
    ax(7)
        .xaxis()
        .set_major_locator(LogLocator::new(10.0)?.with_numticks(NumTicks::Fixed(15)));
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The six axes of the gallery's logit scale demo, plotting `curves`
/// (values over `x` in `[-10, 10]` and their labels) with a legend and a
/// grid: the top rows on logit scales (the middle one writing values near
/// 1 as complements of small numbers), the bottom row linear, the right
/// column zoomed on the values near 1 over positive `x`.
pub(super) fn logit_grid(
    fig: &Figure,
    x: &[f64],
    curves: &[(Vec<f64>, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    let axs = fig.subplots(3, 2)?;
    for ax in axs.as_slice() {
        for (y, label) in curves {
            ax.plot(x, y)?.set_label(*label);
        }
        ax.legend(LegendOptions::default())?;
        ax.grid(true);
    }
    let survival = ScaleOptions {
        one_half: Some("1/2".into()),
        use_overline: Some(true),
        ..ScaleOptions::default()
    };
    for (row, title) in [(0, "logit scale"), (1, "logit scale"), (2, "linear scale")] {
        for col in 0..2 {
            let ax = &axs[(row, col)];
            ax.set_title(title);
            match row {
                0 => {
                    ax.set_yscale("logit")?;
                }
                1 => {
                    ax.set_scale_with(AxisName::Y, "logit", &survival)?;
                }
                _ => {}
            }
            if col == 1 {
                ax.set_xlim(0.0, 10.0);
            }
            match (row, col) {
                (2, 0) => ax.set_ylim(0.0, 1.0),
                (2, 1) => ax.set_ylim(0.8, 1.0),
                (_, 0) => ax.set_ylim(1e-5, 1.0 - 1e-5),
                _ => ax.set_ylim(0.8, 1.0 - 5e-3),
            };
        }
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
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
