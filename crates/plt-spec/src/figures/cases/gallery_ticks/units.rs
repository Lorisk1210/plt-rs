//! The gallery's "Units" section. The examples plot the values of the
//! gallery's `basic_units` module (`crate::user_units::basic_units`):
//! centimetres and inches, radians and degrees, seconds, hertz and
//! minutes, which plt converts through `BasicUnitConverter` as matplotlib
//! does, each axis taking its unit, label, locator and formatter from it.
//! matplotlib's `xunits=` and `yunits=` keywords become
//! [`Axis::set_units`](plt::axes::Axis::set_units) before the plotting
//! call, as `plot` sets them before its values update the units, and
//! after [`Axis::update_units`](plt::axes::Axis::update_units) for the
//! other methods, which let the values pick the units first.
//! `evans_test` defines its own `Foo` values and converter.

use plt::axes::{Axis, AxisName, ErrorSize, TickParams};
use plt::layout::{LayoutEngine, TightLayout};
use plt::legend::LegendOptions;
use plt::patches::{Arc, Ellipse, PatchShape};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, HAlign, TextRotationMode as RotationMode, VAlign};
use plt::ticker::{FormatStrFormatter, IndexLocator};
use plt::units::{AxisInfo, ConversionError, ConversionInterface, Convertible};

use crate::figures::cases::common::{black_arrow, map};
use crate::figures::{CaseResult, FigureCase};
use crate::user_units::basic_units::{
    BasicUnit, CM, DEGREES, HERTZ, INCH, MINUTES, RADIANS, SECS, TaggedValue, cos,
};

fn annotate_with_units(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.annotate("Note 01", (0.5 * CM, 0.5 * CM), AnnotateOptions::default())?;
    let text_in_data = ax.convert_point((0.8 * CM, 0.95 * CM));
    for (xytext, textcoords) in [
        (text_in_data, "data"),
        ((0.8, 0.95).into(), "axes fraction"),
    ] {
        ax.annotate(
            "local max",
            (3.0 * CM, 1.0 * CM),
            AnnotateOptions {
                xytext: Some(xytext),
                textcoords: Some(textcoords.parse()?),
                arrowprops: Some(black_arrow()),
                ..AnnotateOptions::default()
            },
        )?
        .set_halign(HAlign::Right)
        .set_valign(VAlign::Top);
    }
    ax.set_xlim(0.0 * CM, 4.0 * CM)?
        .set_ylim(0.0 * CM, 4.0 * CM)?;
    Ok(())
}

/// The units are set before any value is converted, when the axes have no
/// converter to label them.
fn artist_tests(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.xaxis().set_units(CM);
    ax.yaxis().set_units(CM);
    // A `Line2D` of centimetres given to `add_line`.
    let x = ax.xaxis().convert_units(vec![0.0 * CM, 1.5 * CM]);
    let y = ax.yaxis().convert_units(vec![0.0 * CM, 2.5 * CM]);
    ax.add_line(&x, &y, LineProps::default().linewidth(2.0).color("black"))?;
    // A `Text` given to `add_artist`, which clips it to the axes box.
    ax.text(3.0 * CM, 2.5 * CM, "text label")
        .set_halign(HAlign::Left)
        .set_valign(VAlign::Bottom)
        .set_clip_on(true);
    ax.set_xlim(-1.0 * CM, 10.0 * CM)?
        .set_ylim(-1.0 * CM, 10.0 * CM)?;
    ax.grid(true);
    ax.set_title("Artists with units");
    Ok(())
}

/// `cm * np.arange(0, 10, 2)`.
fn even_centimetres() -> Vec<TaggedValue> {
    [0.0, 2.0, 4.0, 6.0, 8.0].map(|v| v * CM).to_vec()
}

/// The extents of `ends` from `start` along `axis`, as matplotlib's
/// `_convert_dx` converts them: the converted `start + end` less the
/// converted start.
fn extents(axis: &Axis, start: TaggedValue, ends: &[TaggedValue]) -> Vec<f64> {
    let origin = axis.convert_value(start);
    ends.iter()
        .map(|&end| axis.convert_value(start + end) - origin)
        .collect()
}

/// `bar(cms, cms, bottom=0 * cm, width=width, xunits=..., yunits=...)`:
/// the values pick the units, which `units` then replace, and the width
/// is 0.8 without units.
fn unit_bars(
    ax: &Axes,
    width: Option<TaggedValue>,
    units: Option<(BasicUnit, BasicUnit)>,
) -> CaseResult {
    let cms = even_centimetres();
    let bottom = 0.0 * CM;
    ax.xaxis().update_units(cms.clone());
    ax.yaxis().update_units([bottom]);
    if let Some((x, y)) = units {
        ax.xaxis().set_units(x);
        ax.yaxis().set_units(y);
    }
    let thickness = width.map_or(0.8, |width| extents(&ax.xaxis(), cms[0], &[width])[0]);
    let heights = extents(&ax.yaxis(), bottom, &cms);
    let base = ax.yaxis().convert_value(bottom);
    ax.bar(
        cms,
        heights,
        BarOptions {
            thickness: vec![thickness],
            base: vec![base],
            ..BarOptions::default()
        },
    )?;
    Ok(())
}

fn bar_demo2(fig: &Figure) -> CaseResult {
    let width = Some(0.8 * CM);
    let axs = fig.subplots(2, 2)?;
    unit_bars(&axs[(0, 0)], None, None)?;
    unit_bars(&axs[(0, 1)], width, Some((CM, INCH)))?;
    unit_bars(&axs[(1, 0)], width, Some((INCH, CM)))?;
    axs[(1, 0)].set_xlim(2.0, 6.0)?;
    unit_bars(&axs[(1, 1)], width, Some((INCH, INCH)))?;
    axs[(1, 1)].set_xlim(2.0 * CM, 6.0 * CM)?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The y axis is in inches but, its units set before any value was
/// converted, not labelled.
fn bar_unit_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.yaxis().set_units(INCH);
    let ind: Vec<f64> = (0..5).map(f64::from).collect();
    let width = 0.35;
    let groups = [
        (
            "Tea",
            [15.0, 10.0, 8.0, 12.0, 5.0],
            [2.0, 1.0, 1.0, 4.0, 2.0],
            0.0,
        ),
        (
            "Coffee",
            [14.0, 19.0, 7.0, 5.0, 10.0],
            [3.0, 5.0, 2.0, 1.0, 2.0],
            width,
        ),
    ];
    let bottom = 0.0 * CM;
    for (label, means, std, shift) in groups {
        let yaxis = ax.yaxis();
        ax.bar(
            map(&ind, |i| i + shift),
            extents(&yaxis, bottom, &means.map(|v| v * CM)),
            BarOptions {
                thickness: vec![width],
                base: vec![yaxis.convert_value(bottom)],
                label: Some(label.to_owned()),
                errors: ErrorbarOptions {
                    yerr: Some(ErrorSize::Symmetric(extents(
                        &yaxis,
                        bottom,
                        &std.map(|v| v * CM),
                    ))),
                    ..ErrorbarOptions::default()
                },
                ..BarOptions::default()
            },
        )?;
    }
    ax.set_title("Cup height by group and beverage choice");
    ax.set_xticks(map(&ind, |i| i + width / 2.0))
        .set_xticklabels(["G1", "G2", "G3", "G4", "G5"])?;
    ax.legend(LegendOptions::default())?;
    ax.autoscale_view(None);
    Ok(())
}

/// A figure of `ellipse_with_units`: the ellipse drawn by `outline` over
/// the polygon approximating it, with automatic and equal aspect. The
/// example's arithmetic drops the centimetres before matplotlib sees the
/// values, so they are plain numbers here too.
fn ellipse_compare<S: PatchShape>(fig: &Figure, outline: impl Fn() -> S) -> CaseResult {
    let (xcenter, ycenter) = (0.38, 0.52);
    let (width, height) = (1e-1, 3e-1);
    let angle: f64 = -30.0;
    let theta = map(&arange(0.0, 360.0, 1.0), f64::to_radians);
    let rtheta = angle.to_radians();
    let (cos, sin) = (rtheta.cos(), rtheta.sin());
    let (x, y): (Vec<f64>, Vec<f64>) = theta
        .iter()
        .map(|t| {
            let (x, y) = (0.5 * width * t.cos(), 0.5 * height * t.sin());
            (cos * x + -sin * y + xcenter, sin * x + cos * y + ycenter)
        })
        .unzip();
    let outlined = PatchProps::default().linewidth(2.0).fill(false).zorder(2.0);
    let ax = fig.add_subplot(2, 1, 1)?;
    ax.set_aspect(Aspect::Auto)?;
    ax.fill(
        &x,
        &y,
        PatchProps::default()
            .alpha(0.2)
            .facecolor("yellow")
            .edgecolor("yellow")
            .linewidth(1.0)
            .zorder(1.0),
    )?;
    ax.add_patch_with(outline(), outlined.clone());
    let ax = fig.add_subplot(2, 1, 2)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.fill(
        &x,
        &y,
        PatchProps::default()
            .alpha(0.2)
            .facecolor("green")
            .edgecolor("green")
            .zorder(1.0),
    )?;
    ax.add_patch_with(outline(), outlined);
    Ok(())
}

fn ellipse_with_units_1(fig: &Figure) -> CaseResult {
    ellipse_compare(fig, || Ellipse {
        angle: -30.0,
        ..Ellipse::new((0.38, 0.52), 1e-1, 3e-1)
    })
}

fn ellipse_with_units_2(fig: &Figure) -> CaseResult {
    ellipse_compare(fig, || Arc::new((0.38, 0.52), 1e-1, 3e-1).with_angle(-30.0))
}

/// `evans_test`'s value carrying a scalar conversion factor as its unit.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Foo {
    /// The value times the unit.
    scaled: f64,
    /// The factor.
    unit: f64,
}

impl Foo {
    /// `Foo(val, unit)`.
    fn new(value: f64, unit: f64) -> Self {
        Self {
            scaled: value * unit,
            unit,
        }
    }

    /// The value in `unit`, its own by default (`Foo.value`).
    fn value(self, unit: Option<f64>) -> f64 {
        self.scaled / unit.unwrap_or(self.unit)
    }
}

/// `evans_test`'s converter of [`Foo`] values: in units 1 and 2, a tick
/// every eighth value labelled `VAL: x` and the axis labelled `foo`.
struct FooConverter;

impl ConversionInterface for FooConverter {
    type Value = Foo;
    type Unit = f64;

    fn convert(
        &self,
        values: &[Foo],
        unit: Option<&f64>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        Ok(values
            .iter()
            .map(|value| value.value(unit.copied()))
            .collect())
    }

    fn axisinfo(&self, unit: Option<&f64>, _: &Axis) -> Option<AxisInfo> {
        if !unit.is_some_and(|unit| [1.0, 2.0].contains(unit)) {
            return None;
        }
        Some(AxisInfo {
            major_locator: Some(std::rc::Rc::new(IndexLocator::new(8.0, 0.0))),
            major_formatter: Some(std::rc::Rc::new(FormatStrFormatter::new("VAL: %s").ok()?)),
            label: Some("foo".to_owned()),
            ..AxisInfo::default()
        })
    }

    fn default_units(&self, values: &[Foo], _: &Axis) -> Option<f64> {
        values.first().map(|value| value.unit)
    }
}

impl Convertible for Foo {
    type Converter = FooConverter;

    fn converter() -> FooConverter {
        FooConverter
    }
}

/// `evans_test`'s `Foo` values `0, 2, ..., 48` against their index, in
/// `xunits` if given, with tick labels turned by 30°.
fn foo_plot(ax: &Axes, xunits: Option<f64>, line: LineProps) -> CaseResult {
    let x: Vec<Foo> = (0..50)
        .step_by(2)
        .map(|v| Foo::new(f64::from(v), 1.0))
        .collect();
    let y: Vec<f64> = (0..x.len()).map(|i| i as f64).collect();
    if let Some(unit) = xunits {
        ax.xaxis().set_units(unit);
    }
    ax.plot_with(x, y, line)?;
    ax.tick_params(&TickParams {
        axis: Some(AxisName::X),
        labelrotation: Some(30.0),
        labelrotation_mode: Some(RotationMode::XTick),
        ..TickParams::default()
    });
    Ok(())
}

fn evans_test(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    fig.suptitle("Custom units");
    fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        bottom: Some(0.2),
        ..plt::gridspec::SubplotOverrides::default()
    })?;
    foo_plot(ax2, Some(2.0), LineProps::fmt("o")?)?;
    ax2.set_title("xunits = 2.0");
    foo_plot(ax1, None, LineProps::default())?;
    ax1.set_title("default units");
    Ok(())
}

fn radian_demo(fig: &Figure) -> CaseResult {
    let x: Vec<TaggedValue> = arange(0.0, 15.0, 0.01)
        .into_iter()
        .map(|v| v * RADIANS)
        .collect();
    let cosines = cos(&x)?;
    let axs = fig.subplots(2, 1)?;
    for (ax, unit) in [(&axs[(0, 0)], RADIANS), (&axs[(1, 0)], DEGREES)] {
        ax.xaxis().set_units(unit);
        ax.plot(x.clone(), cosines.clone())?;
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn units_sample(fig: &Figure) -> CaseResult {
    let cms = even_centimetres();
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 2)?;
    for ((row, col), units) in [
        ((0, 0), None),
        ((0, 1), Some((CM, INCH))),
        ((1, 0), Some((INCH, CM))),
        ((1, 1), Some((INCH, INCH))),
    ] {
        let ax = &axs[(row, col)];
        if let Some((x, y)) = units {
            ax.xaxis().set_units(x);
            ax.yaxis().set_units(y);
        }
        ax.plot(cms.clone(), cms.clone())?;
    }
    axs[(1, 0)].set_xlim(-1.0, 4.0)?;
    axs[(1, 1)].set_xlim(3.0 * CM, 6.0 * CM)?;
    Ok(())
}

/// `secs * MaskedArray(1..=8, mask)`: the masked seconds are NaN.
fn masked_seconds() -> Vec<TaggedValue> {
    let mask = [true, false, true, false, false, false, true, false];
    (1..=8)
        .zip(mask)
        .map(|(v, masked)| {
            let seconds = if masked { f64::NAN } else { f64::from(v) };
            seconds * SECS
        })
        .collect()
}

/// Seconds against seconds, hertz and minutes on axes sharing x; the
/// axes of a grid sharing x have no inner x labels to replace, and the
/// bottom axis takes the unit of x before it has a converter.
fn units_scatter(fig: &Figure) -> CaseResult {
    let xsecs = masked_seconds();
    let axs = fig.subplots_shared(3, 1, plt::figure::Share::All, plt::figure::Share::None)?;
    let [ax1, ax2, ax3] = [(0, 0), (1, 0), (2, 0)].map(|at| &axs[at]);
    ax1.scatter_with(xsecs.clone(), xsecs.clone(), ScatterOptions::default())?;
    ax1.yaxis().set_units(SECS);
    for (ax, yunits) in [(ax2, HERTZ), (ax3, MINUTES)] {
        ax.yaxis().update_units(xsecs.clone());
        ax.yaxis().set_units(yunits);
        ax.scatter_with(xsecs.clone(), xsecs.clone(), ScatterOptions::default())?;
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/annotate_with_units", annotate_with_units),
    FigureCase::new("gallery_ticks/artist_tests", artist_tests),
    FigureCase::new("gallery_ticks/bar_demo2", bar_demo2),
    FigureCase::new("gallery_ticks/bar_unit_demo", bar_unit_demo),
    FigureCase::new("gallery_ticks/ellipse_with_units_1", ellipse_with_units_1),
    FigureCase::new("gallery_ticks/ellipse_with_units_2", ellipse_with_units_2),
    FigureCase::new("gallery_ticks/evans_test", evans_test),
    FigureCase::new("gallery_ticks/radian_demo", radian_demo),
    FigureCase::new("gallery_ticks/units_sample", units_sample),
    FigureCase::new("gallery_ticks/units_scatter", units_scatter),
];
