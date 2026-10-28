//! Units: values of a user type whose converter misbehaves — it returns
//! too few or too many numbers, NaN, infinities, numbers at the edges of
//! `f64` or errors, differently from one call to the next, picks odd
//! default units, and hands out axis information with impossible default
//! limits, labels, and tick helpers that return NaN, too many ticks or
//! the wrong number of labels — reach every plotting method that takes
//! values or extents with units. The axes change units between the
//! calls, mix user values with numbers and category names, and the
//! figure is drawn.

use std::cell::RefCell;
use std::rc::Rc;

use plt::axes::{
    Axes, Axis, BandAlign, BarOptions, ErrorSize, ErrorbarOptions, EventplotOptions,
    FillBetweenOptions, StackplotOptions, StemOptions,
};
use plt::figure::{Figure, SaveFormat};
use plt::lines::{LineProps, StepWhere};
use plt::patches::{DataRectangle, PatchProps};
use plt::text::AnnotateOptions;
use plt::ticker::{FormattedTicks, Formatter, Locator, TickContext, TickerError};
use plt::units::{
    AxisDelta, AxisDeltas, AxisValue, AxisValues, ConversionError, ConversionInterface,
    Convertible, CustomValues, UnitAxis, UnitAxisInfo,
};

use crate::harness::{Gen, fuzz};
use crate::support::{format, render};

/// What one call of the converter returns.
#[derive(Clone, Copy, Debug)]
enum Conversion {
    /// One number per value, scaled by the unit.
    Correct,
    /// One number fewer than values (none for no values).
    Short,
    /// Two numbers more than values.
    Long,
    /// No numbers at all.
    Empty,
    /// NaN for every value.
    NaN,
    /// Alternating infinities.
    Infinite,
    /// Numbers at the edges of `f64`.
    Extreme,
    /// A conversion error.
    Error,
}

/// What the tick helpers of the axis information do.
#[derive(Clone, Copy, Debug)]
enum Ticks {
    /// NaN, infinite and repeated ticks.
    Odd,
    /// Far more ticks than an axis draws.
    Many,
    /// No labels, or more labels than ticks.
    WrongLabels,
    /// An error.
    Error,
}

/// How the converter of the user values behaves for one input.
#[derive(Clone, Debug)]
struct Behaviour {
    /// What successive calls return, cycled.
    conversions: Vec<Conversion>,
    default_unit: Option<u8>,
    label: Option<String>,
    default_limits: Option<(f64, f64)>,
    ticks: Option<Ticks>,
    /// Whether extents add to values, and values scale.
    arithmetic: bool,
}

impl Behaviour {
    fn generate(g: &mut Gen) -> Self {
        let conversions = (0..=g.len(4))
            .map(|_| {
                g.pick(&[
                    Conversion::Correct,
                    Conversion::Correct,
                    Conversion::Correct,
                    Conversion::Short,
                    Conversion::Long,
                    Conversion::Empty,
                    Conversion::NaN,
                    Conversion::Infinite,
                    Conversion::Extreme,
                    Conversion::Error,
                ])
            })
            .collect();
        Self {
            conversions,
            default_unit: g.chance(0.7).then(|| g.below(4) as u8),
            label: g
                .chance(0.5)
                .then(|| g.soup(&["m", "$x^2$", "\n", "é", "$\\frac{", "%"], 4)),
            default_limits: g.chance(0.4).then(|| (g.number(), g.number())),
            ticks: g
                .chance(0.3)
                .then(|| g.pick(&[Ticks::Odd, Ticks::Many, Ticks::WrongLabels, Ticks::Error])),
            arithmetic: g.chance(0.7),
        }
    }
}

thread_local! {
    /// The behaviour of the converter for the input running, and how many
    /// times it was called.
    static BEHAVIOUR: RefCell<(Behaviour, usize)> = RefCell::new((
        Behaviour {
            conversions: vec![Conversion::Correct],
            default_unit: None,
            label: None,
            default_limits: None,
            ticks: None,
            arithmetic: true,
        },
        0,
    ));
}

/// A value of the user type.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Odd(f64);

/// An extent of the user type.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Gap(f64);

/// The units of axes of [`Odd`] values.
#[derive(Clone, Copy, Debug, PartialEq)]
struct OddUnit(u8);

/// Converts with the behaviour of the input running.
struct OddConverter;

/// The numbers `conversion` makes of `values` in `unit`.
fn convert(
    values: &[f64],
    unit: Option<&OddUnit>,
    conversion: Conversion,
) -> Result<Vec<f64>, ConversionError> {
    let scale = unit.map_or(1.0, |unit| f64::from(unit.0) + 0.5);
    let correct = || values.iter().map(|value| value * scale);
    Ok(match conversion {
        Conversion::Correct => correct().collect(),
        Conversion::Short => correct().skip(1).collect(),
        Conversion::Long => correct().chain([1.0, 2.0]).collect(),
        Conversion::Empty => Vec::new(),
        Conversion::NaN => vec![f64::NAN; values.len()],
        Conversion::Infinite => (0..values.len())
            .map(|i| {
                if i % 2 == 0 {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                }
            })
            .collect(),
        Conversion::Extreme => (0..values.len())
            .map(|i| [f64::MAX, -f64::MAX, 5e-324, 1e300][i % 4])
            .collect(),
        Conversion::Error => return Err(ConversionError::Failed("misbehaving".to_owned())),
    })
}

/// The conversion of the next call.
fn next_conversion() -> Conversion {
    BEHAVIOUR.with(|cell| {
        let (behaviour, calls) = &mut *cell.borrow_mut();
        *calls += 1;
        behaviour.conversions[*calls % behaviour.conversions.len()]
    })
}

impl ConversionInterface for OddConverter {
    type Value = Odd;
    type Unit = OddUnit;

    fn convert(
        &self,
        values: &[Odd],
        unit: Option<&OddUnit>,
        _: &UnitAxis,
    ) -> Result<Vec<f64>, ConversionError> {
        let numbers: Vec<f64> = values.iter().map(|odd| odd.0).collect();
        convert(&numbers, unit, next_conversion())
    }

    fn axisinfo(&self, _: Option<&OddUnit>, _: &UnitAxis) -> Option<UnitAxisInfo> {
        let behaviour = BEHAVIOUR.with(|cell| cell.borrow().0.clone());
        let ticks = behaviour.ticks;
        Some(UnitAxisInfo {
            majloc: ticks.map(|ticks| Rc::new(OddLocator(ticks)) as Rc<dyn Locator>),
            majfmt: ticks.map(|ticks| Rc::new(OddFormatter(ticks)) as Rc<dyn Formatter>),
            label: behaviour.label,
            default_limits: behaviour.default_limits,
            ..UnitAxisInfo::default()
        })
    }

    fn default_units(&self, _: &[Odd], _: &UnitAxis) -> Option<OddUnit> {
        BEHAVIOUR.with(|cell| cell.borrow().0.default_unit.map(OddUnit))
    }
}

impl Convertible for Odd {
    type Converter = OddConverter;

    fn converter() -> OddConverter {
        OddConverter
    }

    fn add_delta(&self, delta: &AxisDelta) -> Option<AxisValue> {
        let arithmetic = BEHAVIOUR.with(|cell| cell.borrow().0.arithmetic);
        let gap = delta.downcast_ref::<Gap>().filter(|_| arithmetic)?;
        Some(Odd(self.0 + gap.0).into())
    }

    fn scale(&self, factor: i64) -> Option<Self> {
        let arithmetic = BEHAVIOUR.with(|cell| cell.borrow().0.arithmetic);
        arithmetic.then_some(Odd(factor as f64 * self.0))
    }
}

/// Converts extents with the behaviour of the input running.
struct GapConverter;

impl ConversionInterface for GapConverter {
    type Value = Gap;
    type Unit = OddUnit;

    fn convert(
        &self,
        values: &[Gap],
        unit: Option<&OddUnit>,
        _: &UnitAxis,
    ) -> Result<Vec<f64>, ConversionError> {
        let numbers: Vec<f64> = values.iter().map(|gap| gap.0).collect();
        convert(&numbers, unit, next_conversion())
    }
}

impl Convertible for Gap {
    type Converter = GapConverter;

    fn converter() -> GapConverter {
        GapConverter
    }
}

/// A locator of the misbehaving axis information.
#[derive(Debug)]
struct OddLocator(Ticks);

impl Locator for OddLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        _: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        match self.0 {
            Ticks::Odd => Ok(vec![f64::NAN, vmin, vmin, f64::INFINITY, vmax, -0.0]),
            Ticks::Many => Ok((0..200_000)
                .map(|i| vmin + f64::from(i) * (vmax - vmin) / 2e5)
                .collect()),
            Ticks::WrongLabels => Ok(vec![vmin, vmax]),
            Ticks::Error => Err(TickerError::TooManyTicks(usize::MAX)),
        }
    }
}

/// A formatter of the misbehaving axis information.
#[derive(Debug)]
struct OddFormatter(Ticks);

impl Formatter for OddFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let labels = match self.0 {
            Ticks::WrongLabels if locs.len().is_multiple_of(2) => Vec::new(),
            Ticks::WrongLabels => vec!["$x$".to_owned(); locs.len() + 3],
            Ticks::Error => return Err(TickerError::TooManyTicks(locs.len())),
            Ticks::Odd | Ticks::Many => locs.iter().map(|loc| format!("{loc}")).collect(),
        };
        Ok(FormattedTicks {
            labels,
            offset: "$\\frac{".to_owned(),
        })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(format!("{value}"))
    }
}

/// One plotting call: values are [`Odd`]s unless the flag says numbers,
/// extents [`Gap`]s.
#[derive(Clone, Debug)]
enum Call {
    Plot(Vec<f64>, Vec<f64>, bool),
    Scatter(Vec<f64>, Vec<f64>),
    Bar(Vec<f64>, Vec<f64>, Vec<f64>, bool),
    BrokenBarh(Vec<(f64, f64)>),
    Line(f64, bool),
    Span(f64, f64, bool),
    FillBetween(Vec<f64>, Vec<f64>, Vec<f64>, bool),
    Stackplot(Vec<f64>, Vec<f64>),
    Stem(Vec<f64>, Vec<f64>),
    Errorbar(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>),
    Step(Vec<f64>, Vec<f64>),
    Fill(Vec<f64>, Vec<f64>),
    Segments(Vec<f64>, Vec<f64>, Vec<f64>, bool),
    Eventplot(Vec<Vec<f64>>),
    Annotate((f64, f64), Option<(f64, f64)>),
    Text(f64, f64),
    Rectangle((f64, f64), f64, f64),
    Ticks(Vec<f64>, bool, bool),
    Convert(Vec<f64>, bool),
    Units(Option<u8>, bool),
    Categories(usize, bool),
}

impl Call {
    fn generate(g: &mut Gen) -> Self {
        let values = |g: &mut Gen| {
            let len = g.len(12);
            g.numbers(len)
        };
        match g.below(22) {
            0 | 1 => Self::Plot(values(g), values(g), g.chance(0.2)),
            2 => Self::Scatter(values(g), values(g)),
            3 => Self::Bar(values(g), values(g), values(g), g.chance(0.5)),
            4 => Self::BrokenBarh((0..g.len(4)).map(|_| (g.number(), g.number())).collect()),
            5 => Self::Line(g.number(), g.chance(0.5)),
            6 => Self::Span(g.number(), g.number(), g.chance(0.5)),
            7 => Self::FillBetween(values(g), values(g), values(g), g.chance(0.5)),
            8 => Self::Stackplot(values(g), values(g)),
            9 => Self::Stem(values(g), values(g)),
            10 => Self::Errorbar(values(g), values(g), values(g), values(g)),
            11 => Self::Step(values(g), values(g)),
            12 => Self::Fill(values(g), values(g)),
            13 => Self::Segments(values(g), values(g), values(g), g.chance(0.5)),
            14 => Self::Eventplot((0..=g.len(3)).map(|_| values(g)).collect()),
            15 => Self::Annotate(
                (g.number(), g.number()),
                g.chance(0.5).then(|| (g.number(), g.number())),
            ),
            16 => Self::Text(g.number(), g.number()),
            17 => Self::Rectangle((g.number(), g.number()), g.number(), g.number()),
            18 => Self::Ticks(values(g), g.chance(0.5), g.chance(0.5)),
            19 => Self::Convert(values(g), g.chance(0.5)),
            20 => Self::Units(g.chance(0.8).then(|| g.below(4) as u8), g.chance(0.5)),
            _ => Self::Categories(g.len(4), g.chance(0.5)),
        }
    }
}

/// `numbers` as user values.
fn odd(numbers: &[f64]) -> AxisValues {
    numbers.iter().copied().map(Odd).collect::<Vec<_>>().into()
}

/// `numbers` as user extents.
fn gaps(numbers: &[f64]) -> AxisDeltas {
    numbers.iter().copied().map(Gap).collect::<Vec<_>>().into()
}

/// The axis an `x` flag names.
fn axis(ax: &Axes, x: bool) -> Axis {
    if x { ax.xaxis() } else { ax.yaxis() }
}

/// Makes `call` on `ax`; errors are fine.
fn make(ax: &Axes, call: &Call) {
    match call {
        Call::Plot(x, y, numbers) => {
            let x = if *numbers {
                AxisValues::Numbers(x.clone())
            } else {
                odd(x)
            };
            let _ = ax.plot_with(x, odd(y), LineProps::default());
        }
        Call::Scatter(x, y) => drop(ax.scatter(odd(x), odd(y))),
        Call::Bar(x, heights, widths, horizontal) => {
            let options = BarOptions {
                thickness: gaps(widths),
                base: odd(&widths[..widths.len() / 2]),
                ..BarOptions::default()
            };
            let _ = if *horizontal {
                ax.barh(odd(x), gaps(heights), options)
            } else {
                ax.bar(odd(x), gaps(heights), options)
            };
        }
        Call::BrokenBarh(ranges) => {
            let ranges: Vec<(Odd, Gap)> = ranges.iter().map(|&(x, w)| (Odd(x), Gap(w))).collect();
            ax.broken_barh(&ranges, (0.0, 1.0), BandAlign::Bottom);
        }
        Call::Line(at, x) => {
            if *x {
                ax.axvline(Odd(*at), LineProps::default());
            } else {
                ax.axhline(Odd(*at), LineProps::default());
            }
        }
        Call::Span(from, to, x) => {
            if *x {
                ax.axvspan(Odd(*from), Odd(*to));
            } else {
                ax.axhspan(Odd(*from), Odd(*to));
            }
        }
        Call::FillBetween(x, y1, y2, vertical) => {
            let options = FillBetweenOptions::default();
            let _ = if *vertical {
                ax.fill_betweenx(odd(x), odd(y1), odd(y2), options)
            } else {
                ax.fill_between(odd(x), odd(y1), odd(y2), options)
            };
        }
        Call::Stackplot(x, y) => {
            drop(ax.stackplot(odd(x), std::slice::from_ref(y), StackplotOptions::default()));
        }
        Call::Stem(locs, heads) => drop(ax.stem(odd(locs), odd(heads), StemOptions::default())),
        Call::Errorbar(x, y, lower, upper) => {
            let options = ErrorbarOptions {
                xerr: Some(gaps(lower).into()),
                yerr: Some(ErrorSize::Asymmetric {
                    lower: gaps(lower),
                    upper: gaps(upper),
                }),
                ..ErrorbarOptions::default()
            };
            let _ = ax.errorbar(odd(x), odd(y), options);
        }
        Call::Step(x, y) => drop(ax.step(odd(x), odd(y), StepWhere::Mid, LineProps::default())),
        Call::Fill(x, y) => drop(ax.fill(odd(x), odd(y), PatchProps::default())),
        Call::Segments(at, from, to, vertical) => {
            let _ = if *vertical {
                ax.vlines(odd(at), odd(from), odd(to))
            } else {
                ax.hlines(odd(at), odd(from), odd(to))
            };
        }
        Call::Eventplot(positions) => {
            let positions: Vec<AxisValues> = positions.iter().map(|p| odd(p)).collect();
            let _ = ax.eventplot(&positions, EventplotOptions::default());
        }
        Call::Annotate((x, y), text_at) => {
            let options = AnnotateOptions {
                xytext: text_at.map(|(x, y)| (Odd(x), Odd(y)).into()),
                ..AnnotateOptions::default()
            };
            let _ = ax.annotate("a", (Odd(*x), Odd(*y)), options);
        }
        Call::Text(x, y) => drop(ax.text(Odd(*x), Odd(*y), "t")),
        Call::Rectangle((x, y), width, height) => {
            if let Ok(rectangle) = DataRectangle::new((Odd(*x), Odd(*y)), Gap(*width), Gap(*height))
            {
                ax.add_patch(rectangle);
            }
        }
        _ => use_axis(ax, call),
    }
}

/// Makes the calls of `call` that change or ask an axis; errors are fine.
fn use_axis(ax: &Axes, call: &Call) {
    match call {
        Call::Ticks(ticks, x, minor) => {
            let axis = axis(ax, *x);
            if *minor {
                axis.set_minor_ticks(odd(ticks));
            } else {
                axis.set_ticks(odd(ticks));
            }
        }
        Call::Convert(values, x) => {
            let axis = axis(ax, *x);
            let _ = axis.update_units(odd(values));
            let _ = axis.convert_units(odd(values));
            if let Some(&first) = values.first() {
                let _ = axis.convert_value(Odd(first));
                let _ = ax.convert_point((Odd(first), Odd(first)));
            } else {
                // A user value holding no value at all.
                let none = || AxisValue::Custom(CustomValues::new(Vec::<Odd>::new()));
                let _ = axis.convert_value(none());
                let _ = ax.convert_point((none(), none()));
                drop(ax.text(none(), none(), "t"));
            }
        }
        Call::Units(unit, x) => {
            let axis = axis(ax, *x);
            match unit {
                Some(unit) => axis.set_units(OddUnit(*unit)),
                None => axis.set_units(()),
            };
        }
        Call::Categories(count, x) => {
            let names: Vec<String> = (0..*count).map(|i| format!("c{i}")).collect();
            let values: Vec<f64> = (0..*count).map(|i| i as f64).collect();
            let _ = if *x {
                ax.plot(names, values)
            } else {
                ax.plot(values, names)
            };
        }
        _ => {}
    }
}

#[test]
fn misbehaving_converters() {
    fuzz(
        "units",
        300,
        |g| {
            let calls: Vec<Call> = (0..=g.len(8)).map(|_| Call::generate(g)).collect();
            (Behaviour::generate(g), calls, format(g))
        },
        |(behaviour, calls, format)| {
            BEHAVIOUR.with(|cell| *cell.borrow_mut() = (behaviour.clone(), 0));
            let fig = Figure::with_size(2.0, 1.5, 40.0);
            let ax = fig.subplot();
            for call in calls {
                make(&ax, call);
            }
            render(&fig, *format);
            if *format != SaveFormat::Png {
                render(&fig, SaveFormat::Png);
            }
        },
    );
}
