//! Property cycles (`axes.prop_cycle`, matplotlib's `cycler`).

use std::fmt::Write as _;

use plt_core::Rgba;

use super::literal::Expr;
use super::value::RcValue;
use crate::colors::{Color, color_sequence};
use crate::lines::LineStyle;
use crate::markers::Marker;

/// One property value inside a cycle.
#[derive(Clone, Debug, PartialEq)]
pub enum CycleValue {
    /// A colour (`color`, `facecolor`, `edgecolor`, marker colours).
    Color(Color),
    /// A number (`linewidth`, `markersize`, `markeredgewidth`, `alpha`).
    Number(f64),
    /// A line style.
    LineStyle(LineStyle),
    /// A marker.
    Marker(Marker),
    /// A dash pattern in points (`dashes`).
    Dashes(Vec<f64>),
    /// Any other textual value (`joinstyle`, `capstyle`, `fillstyle`, `hatch`, ...).
    Text(String),
}

/// A sequence of property sets that artists cycle through, e.g. the
/// default `cycler(color='tab10')`.
///
/// Like Python's `cycler`, rows are combined with [`zip`](Self::zip) (`+`),
/// [`product`](Self::product) (`*`) and [`concat`](Self::concat), and
/// repeated with [`repeat`](Self::repeat) (`* n`).
#[derive(Clone, Debug, PartialEq)]
pub struct Cycler {
    keys: Vec<String>,
    rows: Vec<Vec<CycleValue>>,
}

/// matplotlib's short property names.
const ALIASES: [(&str, &str); 9] = [
    ("c", "color"),
    ("lw", "linewidth"),
    ("ls", "linestyle"),
    ("fc", "facecolor"),
    ("ec", "edgecolor"),
    ("mfc", "markerfacecolor"),
    ("mec", "markeredgecolor"),
    ("mew", "markeredgewidth"),
    ("ms", "markersize"),
];

impl Cycler {
    /// A single-property cycle; the property name may be an alias.
    pub fn new(property: &str, values: Vec<CycleValue>) -> Self {
        let key = ALIASES
            .iter()
            .find(|(alias, _)| *alias == property)
            .map_or(property, |(_, full)| full);
        Self {
            keys: vec![key.to_owned()],
            rows: values.into_iter().map(|v| vec![v]).collect(),
        }
    }

    /// A colour cycle.
    pub fn colors_of<I: IntoIterator<Item = Color>>(colors: I) -> Self {
        Self::new("color", colors.into_iter().map(CycleValue::Color).collect())
    }

    /// Number of entries before the cycle repeats.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns `true` for a cycle without entries.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The cycled property names.
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// The `i`-th property set (wrapping around), as `(property, value)`.
    pub fn entry(&self, i: usize) -> impl Iterator<Item = (&str, &CycleValue)> {
        let row = (!self.rows.is_empty()).then(|| &self.rows[i % self.rows.len()]);
        self.keys
            .iter()
            .map(String::as_str)
            .zip(row.into_iter().flatten())
    }

    /// Every value of one property, in order (`cycler.by_key()[key]`).
    pub fn values(&self, key: &str) -> Option<Vec<&CycleValue>> {
        let column = self.keys.iter().position(|k| k == key)?;
        Some(self.rows.iter().map(|row| &row[column]).collect())
    }

    /// The resolved `color` values (empty when the cycle has no colours).
    pub fn colors(&self) -> Vec<Rgba> {
        self.values("color")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| match value {
                CycleValue::Color(color) => color.resolve_with(None, &[]).ok(),
                _ => None,
            })
            .collect()
    }

    /// Cycles both property sets in lockstep (`a + b`).
    pub fn zip(mut self, other: Self) -> Result<Self, String> {
        if self.len() != other.len() {
            return Err(format!(
                "can only add equal length cycles, not {} and {}",
                self.len(),
                other.len()
            ));
        }
        if let Some(dup) = other.keys.iter().find(|k| self.keys.contains(k)) {
            return Err(format!("cannot add cycles sharing the key {dup:?}"));
        }
        self.keys.extend(other.keys);
        for (row, extra) in self.rows.iter_mut().zip(other.rows) {
            row.extend(extra);
        }
        Ok(self)
    }

    /// Every combination of the two cycles (`a * b`).
    pub fn product(self, other: Self) -> Result<Self, String> {
        if let Some(dup) = other.keys.iter().find(|k| self.keys.contains(k)) {
            return Err(format!("cannot multiply cycles sharing the key {dup:?}"));
        }
        let rows = self
            .rows
            .iter()
            .flat_map(|a| {
                other
                    .rows
                    .iter()
                    .map(move |b| a.iter().chain(b).cloned().collect())
            })
            .collect();
        let keys = self.keys.into_iter().chain(other.keys).collect();
        Ok(Self { keys, rows })
    }

    /// The entries repeated `times` times, none for `times <= 0`
    /// (`cycle * n` and `n * cycle`, which repeat every value list).
    #[must_use]
    pub fn repeat(self, times: i64) -> Self {
        let times = usize::try_from(times).unwrap_or(0);
        let rows = std::iter::repeat_n(&self.rows, times)
            .flatten()
            .cloned()
            .collect();
        Self { rows, ..self }
    }

    /// The entries of `self` followed by those of `other` (same keys).
    pub fn concat(mut self, other: Self) -> Result<Self, String> {
        if self.keys != other.keys {
            return Err("can only concatenate cycles with identical keys".to_owned());
        }
        self.rows.extend(other.rows);
        Ok(self)
    }

    /// Python slice `[start:stop:step]` of the entries: negative indices
    /// count from the end and a negative step walks backwards. A zero step
    /// selects nothing (Python rejects it, and so does parsing an rc value).
    #[must_use]
    pub fn slice(&self, start: Option<i64>, stop: Option<i64>, step: Option<i64>) -> Self {
        let len = self.rows.len() as i64;
        let step = step.unwrap_or(1);
        let backwards = step < 0;
        // Python's `PySlice_AdjustIndices`.
        let adjust = |index: i64| match index {
            i if i < 0 => (i + len).max(if backwards { -1 } else { 0 }),
            i if i >= len => {
                if backwards {
                    len - 1
                } else {
                    len
                }
            }
            i => i,
        };
        let start = start.map_or(if backwards { len - 1 } else { 0 }, adjust);
        let stop = stop.map_or(if backwards { -1 } else { len }, adjust);
        let mut rows = Vec::new();
        let mut index = start;
        while (step > 0 && index < stop) || (backwards && index > stop) {
            rows.push(self.rows[index as usize].clone());
            index += step;
        }
        Self {
            keys: self.keys.clone(),
            rows,
        }
    }

    fn from_expr(expr: &Expr) -> Result<Self, String> {
        match expr {
            Expr::BinOp { add, left, right } => match (*add, &**left, &**right) {
                (false, Expr::Int(times), cycle) | (false, cycle, Expr::Int(times)) => {
                    Ok(Self::from_expr(cycle)?.repeat(*times))
                }
                (true, left, right) => Self::from_expr(left)?.zip(Self::from_expr(right)?),
                (false, left, right) => Self::from_expr(left)?.product(Self::from_expr(right)?),
            },
            Expr::Slice {
                bounds: [_, _, Some(0)],
                ..
            } => Err("slice step cannot be zero".to_owned()),
            Expr::Slice {
                value,
                bounds: [start, stop, step],
            } => Ok(Self::from_expr(value)?.slice(*start, *stop, *step)),
            Expr::Call { name, args, kwargs } if name == "concat" && kwargs.is_empty() => args
                .iter()
                .map(Self::from_expr)
                .reduce(|a, b| a?.concat(b?))
                .ok_or_else(|| "concat() needs arguments".to_owned())?,
            Expr::Call { name, args, kwargs } if name == "cycler" => Self::from_call(args, kwargs),
            other => Err(format!(
                "unsupported expression in cycler string: {other:?}"
            )),
        }
    }

    fn from_call(args: &[Expr], kwargs: &[(String, Expr)]) -> Result<Self, String> {
        match (args, kwargs) {
            ([inner], []) => Self::from_expr(inner),
            ([Expr::Str(property), values], []) => property_cycle(property, values),
            ([], [_, ..]) => kwargs
                .iter()
                .map(|(property, values)| property_cycle(property, values))
                .reduce(|a, b| a?.zip(b?))
                .expect("kwargs is non-empty"),
            _ => Err("cycler() takes (label, values), (cycler) or label=values".to_owned()),
        }
    }
}

/// Validates the values of one property as matplotlib's `_prop_validators`.
fn property_cycle(property: &str, values: &Expr) -> Result<Cycler, String> {
    let key = ALIASES
        .iter()
        .find(|(alias, _)| *alias == property)
        .map_or(property, |(_, full)| full);
    let parsed = match key {
        "color" => color_values(values, false)?,
        "facecolor" | "edgecolor" | "markerfacecolor" | "markeredgecolor" => {
            color_values(values, true)?
        }
        "linewidth" | "markersize" | "markeredgewidth" | "alpha" => {
            items(values, |item| item.as_number().map(CycleValue::Number))?
        }
        "linestyle" => items(values, |item| {
            LineStyle::from_expr(item).map(CycleValue::LineStyle)
        })?,
        "marker" => items(values, |item| match item {
            Expr::Str(code) => code.parse().ok().map(CycleValue::Marker),
            Expr::Int(n) => Marker::from_code(*n).ok().map(CycleValue::Marker),
            Expr::Number(n) => Marker::from_code(*n as i64).ok().map(CycleValue::Marker),
            _ => None,
        })?,
        "dashes" => items(values, |item| {
            let pattern: Option<Vec<f64>> =
                item.as_sequence()?.iter().map(Expr::as_number).collect();
            pattern.map(CycleValue::Dashes)
        })?,
        "joinstyle" | "capstyle" | "fillstyle" | "hatch" | "markevery" => {
            items(values, |item| match item {
                Expr::Str(text) => Some(CycleValue::Text(text.clone())),
                Expr::Int(n) => Some(CycleValue::Text(n.to_string())),
                Expr::Number(n) => Some(CycleValue::Text(n.to_string())),
                _ => None,
            })?
        }
        unknown => return Err(format!("unknown artist property: {unknown}")),
    };
    Ok(Cycler::new(key, parsed))
}

/// Applies `convert` to each element of a sequence, or to each
/// comma-separated item of a string.
fn items(
    values: &Expr,
    convert: impl Fn(&Expr) -> Option<CycleValue>,
) -> Result<Vec<CycleValue>, String> {
    let owned;
    let elements: &[Expr] = match values {
        Expr::Str(text) => {
            owned = text
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| Expr::parse(s).unwrap_or_else(|_| Expr::Str(s.to_owned())))
                .collect::<Vec<_>>();
            &owned
        }
        other => other
            .as_sequence()
            .ok_or_else(|| format!("expected a sequence, got {other:?}"))?,
    };
    elements
        .iter()
        .map(|item| convert(item).ok_or_else(|| format!("invalid cycle value {item:?}")))
        .collect()
}

/// Colour lists: a sequence of colours, or a string that is a
/// comma-separated list, a colour-sequence name (`"tab10"`) or a run of
/// single-letter colours (`"rgb"`). Cycle references `"C0"` to `"C9"` are
/// not allowed in the `color` property (matplotlib's
/// `validate_color_for_prop_cycle`; longer ones such as `"C85200"` are
/// hex colours without their `#`).
fn color_values(values: &Expr, allow_cycle_refs: bool) -> Result<Vec<CycleValue>, String> {
    let one = |spec: &str| -> Option<CycleValue> {
        let single_digit_reference =
            matches!(spec.as_bytes(), [b'C', digit] if digit.is_ascii_digit());
        let reference_ok = allow_cycle_refs || !single_digit_reference;
        reference_ok
            .then(|| Color::parse_rc(spec).ok().map(CycleValue::Color))
            .flatten()
    };
    match values {
        Expr::Str(text) => {
            let listed: Option<Vec<_>> = text
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(one)
                .collect();
            if let Some(colors) = listed {
                return Ok(colors);
            }
            if let Some(sequence) = color_sequence(text) {
                return Ok(sequence
                    .into_iter()
                    .map(|c| CycleValue::Color(c.into()))
                    .collect());
            }
            text.chars()
                .filter(|c| !c.is_whitespace())
                .map(|c| one(&c.to_string()))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    format!("{text:?} is neither a color sequence name nor a list of colors")
                })
        }
        other => items(other, |item| match item {
            Expr::Str(spec) => one(spec),
            Expr::Tuple(_) | Expr::List(_) => {
                let rgba: Vec<f64> = item
                    .as_sequence()?
                    .iter()
                    .map(Expr::as_number)
                    .collect::<Option<_>>()?;
                let color = match rgba.as_slice() {
                    [r, g, b] => Color::from((*r, *g, *b)),
                    [r, g, b, a] => Color::from((*r, *g, *b, *a)),
                    _ => return None,
                };
                color
                    .resolve_with(None, &[])
                    .ok()
                    .map(|_| CycleValue::Color(color))
            }
            _ => None,
        }),
    }
}

impl RcValue for Cycler {
    fn parse_rc(text: &str) -> Result<Self, String> {
        Self::from_expr(&Expr::parse(text)?)
    }

    fn format_rc(&self) -> String {
        let columns: Vec<String> = self
            .keys
            .iter()
            .enumerate()
            .map(|(column, key)| {
                let values: Vec<String> = self
                    .rows
                    .iter()
                    .map(|row| format_value(&row[column]))
                    .collect();
                format!("cycler('{key}', [{}])", values.join(", "))
            })
            .collect();
        columns.join(" + ")
    }
}

fn format_value(value: &CycleValue) -> String {
    match value {
        CycleValue::Color(Color::Spec(spec)) => format!("'{spec}'"),
        CycleValue::Color(Color::Rgba(c)) => format!("({}, {}, {}, {})", c.r, c.g, c.b, c.a),
        CycleValue::Number(n) => n.format_rc(),
        CycleValue::LineStyle(style) => format!("'{}'", style.format_rc()),
        CycleValue::Marker(marker) => format!("'{}'", marker.format_rc()),
        CycleValue::Dashes(pattern) => {
            let mut text = String::from("[");
            for (i, v) in pattern.iter().enumerate() {
                let sep = if i == 0 { "" } else { ", " };
                write!(text, "{sep}{v}").expect("writing to a String cannot fail");
            }
            text + "]"
        }
        CycleValue::Text(text) => format!("'{text}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Cycler, String> {
        Cycler::parse_rc(text)
    }

    #[test]
    fn slices_follow_python() {
        let rgb = || parse("cycler('color', 'rgb')").unwrap();
        assert_eq!(
            parse("cycler('color', 'rgb')[::-1]").unwrap(),
            parse("cycler('color', 'bgr')").unwrap()
        );
        assert_eq!(
            rgb().slice(Some(-2), None, None),
            parse("cycler('color', 'gb')").unwrap()
        );
        assert_eq!(
            rgb().slice(None, Some(0), Some(-1)),
            parse("cycler('color', 'bg')").unwrap()
        );
        assert_eq!(
            rgb().slice(Some(10), None, Some(-2)),
            parse("cycler('color', 'br')").unwrap()
        );
        assert!(parse("cycler('color', 'rgb')[::0]").is_err());
    }

    #[test]
    fn integers_repeat_cycles() {
        let twice = parse("cycler('color', 'rg') * 2").unwrap();
        assert_eq!(
            twice,
            parse("cycler('color', ['r', 'g', 'r', 'g'])").unwrap()
        );
        assert_eq!(parse("2 * cycler('color', 'rg')").unwrap(), twice);
        assert_eq!(parse("0 * cycler('color', 'rg')").unwrap().len(), 0);
        assert!(
            parse("2.0 * cycler('color', 'rg')").is_err(),
            "floats do not repeat"
        );
        assert!(parse("cycler('color', 'rg') + 2").is_err());
    }

    #[test]
    fn default_cycle_expands_tab10() {
        let cycle = parse("cycler(color='tab10')").expect("valid");
        assert_eq!(cycle.len(), 10);
        assert_eq!(
            cycle.colors()[0],
            Rgba::opaque(
                0.121_568_627_450_980_39,
                0.466_666_666_666_666_7,
                0.705_882_352_941_176_5
            )
        );
    }

    #[test]
    fn string_forms() {
        assert_eq!(parse("cycler('color', 'rgb')").expect("valid").len(), 3);
        assert_eq!(
            parse("cycler('color', ['E24A33', '#348ABD'])").map(|c| c.len()),
            Ok(2),
            "rc hex may omit '#'"
        );
        assert_eq!(
            parse("cycler('c', 'r, g, b')").expect("valid").keys(),
            ["color"]
        );
        assert!(
            parse("cycler('color', ['C1'])").is_err(),
            "no cycle references in the color cycle"
        );
        assert_eq!(
            parse("cycler('color', ['C85200'])").map(|c| c.colors()),
            Ok(vec![plt_core::Rgba::from_rgba8([0xC8, 0x52, 0x00, 0xFF])]),
            "six hex digits starting with C are a colour"
        );
    }

    #[test]
    fn combinators() {
        let zipped = parse("cycler(color='rgb') + cycler(lw=[1, 2, 3])").expect("valid");
        assert_eq!(zipped.keys(), ["color", "linewidth"]);
        assert_eq!(
            zipped.entry(4).nth(1),
            Some(("linewidth", &CycleValue::Number(2.0)))
        );
        let product = parse("cycler(color='rg') * cycler(ls=['-', '--'])").expect("valid");
        assert_eq!(product.len(), 4);
        assert!(parse("cycler(color='rg') + cycler(lw=[1])").is_err());
        let sliced = parse("cycler(color='rgbk')[1:3]").expect("valid");
        assert_eq!(sliced.len(), 2);
        let concatenated = parse("concat(cycler(color='rg'), cycler(color='b'))").expect("valid");
        assert_eq!(concatenated.len(), 3);
    }

    #[test]
    fn kwargs_zip_in_order() {
        let cycle = parse("cycler(color=['r', 'g'], marker=['o', 's'])").expect("valid");
        assert_eq!(cycle.keys(), ["color", "marker"]);
        assert!(parse("cycler(colour=['r'])").is_err());
    }

    #[test]
    fn round_trips_through_text() {
        let cycle = parse("cycler(color=['r', 'g']) + cycler(lw=[1, 2])").expect("valid");
        assert_eq!(parse(&cycle.format_rc()), Ok(cycle));
    }
}
