//! Values that are not plain numbers: dates with the concise formatter,
//! category names, and a type of our own converted by a unit converter.

use plt::axes::Axis;
use plt::dates::{AutoDateLocator, ConciseDateFormatter, DateTime, TimeDelta};
use plt::prelude::*;
use plt::ticker::StrMethodFormatter;
use plt::units::{ConversionError, ConversionInterface, Convertible, UnitAxisInfo};

use super::data::Samples;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "concise_dates",
        "A time series with the concise date formatter",
        concise_dates,
    ),
    Entry::figure(
        "categorical",
        "Categorical axes: bars, scatter and lines",
        categorical,
    ),
    Entry::figure(
        "custom_units",
        "A unit converter of our own: °C and °F",
        custom_units,
    ),
];

/// Two years of a daily measurement and its 30-day mean. The concise
/// formatter writes each tick as briefly as its neighbours allow and
/// puts the rest (the year) in the offset at the end of the axis.
fn concise_dates() -> Built {
    let fig = Figure::with_size(7.0, 4.5, 100.0);
    let axs = fig.subplots(2, 1)?;
    let start = DateTime::new(2024, 1, 1, 0, 0, 0, 0)?;
    let days = (0..730)
        .map(|day| start.checked_add(TimeDelta::new(day, 0, 0)?))
        .collect::<Result<Vec<_>, _>>()?;
    let mut samples = Samples::new(47);
    let seasonal =
        |day: f64| 12.0 - 9.0 * (2.0 * std::f64::consts::PI * (day + 10.0) / 365.25).cos();
    let daily: Vec<f64> = (0..730)
        .map(|d| seasonal(f64::from(d)) + samples.normal(0.0, 2.5))
        .collect();
    let monthly: Vec<f64> = (0..730_usize)
        .map(|d| {
            let window = &daily[d.saturating_sub(15)..(d + 15).min(730)];
            window.iter().sum::<f64>() / window.len() as f64
        })
        .collect();
    for ax in axs.as_slice() {
        ax.plot_with(
            &days[..],
            &daily,
            LineProps::default().linewidth(0.6).color("0.7"),
        )?;
        ax.plot_with(&days[..], &monthly, LineProps::default().linewidth(2.0))?;
        ax.xaxis()
            .set_major_locator(AutoDateLocator::new())
            .set_major_formatter(ConciseDateFormatter::new());
        ax.set_ylabel("°C");
    }
    // The second axes zooms into one spring: ticks become days.
    let (from, to) = (
        DateTime::new(2025, 3, 1, 0, 0, 0, 0)?,
        DateTime::new(2025, 5, 1, 0, 0, 0, 0)?,
    );
    axs[(1, 0)].set_xlim(from, to)?;
    axs[(0, 0)].set_title("Daily temperature and its 30-day mean");
    Ok(fig)
}

/// Category names placed at 0, 1, 2, ... on their axis in the order they
/// first appear, whatever is plotted with them.
fn categorical() -> Built {
    let fig = Figure::with_size(8.0, 3.4, 100.0);
    let axs = fig.subplots(1, 3)?;
    let fruit = ["apple", "banana", "cherry", "date"];
    axs[(0, 0)].bar(fruit, [12.0, 7.0, 15.0, 4.0], BarOptions::default())?;
    axs[(0, 0)].set_title("bar");
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Mon", "Wed", "Fri"];
    let tasks = [
        "write", "review", "write", "test", "ship", "test", "review", "write",
    ];
    let options = ScatterOptions {
        sizes: Some(vec![60.0]),
        ..ScatterOptions::default()
    };
    axs[(0, 1)].scatter_with(days, tasks, options)?;
    axs[(0, 1)].set_title("scatter: two category axes");
    axs[(0, 2)].plot_fmt(
        ["Mon", "Tue", "Wed", "Thu", "Fri"],
        [3.0, 5.0, 4.0, 6.0, 2.0],
        "o-",
    )?;
    axs[(0, 2)].plot_fmt(["Mon", "Wed", "Fri"], [2.0, 3.0, 4.0], "s--")?;
    axs[(0, 2)].set_title("lines");
    Ok(fig)
}

/// A temperature that knows its unit.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Temperature {
    degrees: f64,
    scale: Scale,
}

/// The unit of a temperature, and of an axis of temperatures.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scale {
    Celsius,
    Fahrenheit,
}

impl Scale {
    /// The symbol written after the numbers.
    const fn symbol(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
        }
    }
}

/// Converts temperatures to numbers in an axis' scale, and gives the
/// axis its tick labels and label (matplotlib's `ConversionInterface`).
struct TemperatureConverter;

impl ConversionInterface for TemperatureConverter {
    type Value = Temperature;
    type Unit = Scale;

    fn convert(
        &self,
        values: &[Temperature],
        unit: Option<&Scale>,
        _: &Axis,
    ) -> Result<Vec<f64>, ConversionError> {
        let to_celsius = |t: &Temperature| match t.scale {
            Scale::Celsius => t.degrees,
            Scale::Fahrenheit => (t.degrees - 32.0) / 1.8,
        };
        Ok(values
            .iter()
            .map(|t| match unit.copied().unwrap_or(t.scale) {
                Scale::Celsius => to_celsius(t),
                Scale::Fahrenheit => to_celsius(t) * 1.8 + 32.0,
            })
            .collect())
    }

    fn axisinfo(&self, unit: Option<&Scale>, _: &Axis) -> Option<UnitAxisInfo> {
        let symbol = unit?.symbol();
        let format = StrMethodFormatter::new(&format!("{{x:.0f}} {symbol}")).ok()?;
        Some(
            UnitAxisInfo::default()
                .majfmt(format)
                .label(format!("temperature ({symbol})")),
        )
    }

    fn default_units(&self, values: &[Temperature], _: &Axis) -> Option<Scale> {
        values.first().map(|t| t.scale)
    }
}

impl Convertible for Temperature {
    type Converter = TemperatureConverter;

    fn converter() -> TemperatureConverter {
        TemperatureConverter
    }
}

/// The same week of temperatures, measured partly in °C and partly in
/// °F, on an axis in °C and on one switched to °F with `set_units`.
fn custom_units() -> Built {
    let fig = Figure::with_size(8.0, 3.6, 100.0);
    let axs = fig.subplots(1, 2)?;
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let celsius = |degrees| Temperature {
        degrees,
        scale: Scale::Celsius,
    };
    let fahrenheit = |degrees| Temperature {
        degrees,
        scale: Scale::Fahrenheit,
    };
    let week = vec![
        celsius(14.0),
        celsius(17.5),
        fahrenheit(68.0),
        fahrenheit(71.6),
        celsius(19.0),
        celsius(16.0),
        fahrenheit(59.0),
    ];
    for ax in axs.as_slice() {
        // The axis takes its unit from the first value: °C.
        ax.plot_fmt(days, week.clone(), "o-")?;
    }
    // Switching the unit converts the line again, and relabels the axis.
    axs[(0, 1)].yaxis().set_units(Scale::Fahrenheit);
    axs[(0, 0)].set_title("axis in °C");
    axs[(0, 1)].set_title("the same axis switched to °F");
    Ok(fig)
}
