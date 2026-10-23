//! Property cycles (`axes.prop_cycle`, matplotlib's `cycler`).

mod values;

use std::fmt;
use std::str::FromStr;

use plt_core::Rgba;

use self::values::{format_value, property_cycle};
use super::literal::Expr;
use super::value::RcValue;
use crate::colors::Color;
use crate::lines::{LineStyle, MarkEvery};
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
    /// Which points get markers (`markevery`; `None` marks every point).
    MarkEvery(Option<MarkEvery>),
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

/// A cycler expression that is malformed or names an unknown property or
/// an invalid value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CyclerError(String);

impl fmt::Display for CyclerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CyclerError {}

impl FromStr for Cycler {
    type Err = CyclerError;

    /// Parses and validates a cycler expression as matplotlib's
    /// `cycler(...)` and `axes.prop_cycle` do: `cycler(label, values)`,
    /// `cycler(label=values, ...)` and `cycler(cycler)` calls (property
    /// aliases such as `c` and `lw` allowed), combined with `+`, `*`,
    /// slices and `concat(...)`.
    ///
    /// ```
    /// use plt::rc::Cycler;
    /// let cycle: Cycler = "cycler(c=['r', 'g']) + cycler(marker=['.', 'x'])".parse()?;
    /// assert_eq!(cycle.keys(), ["color", "marker"]);
    /// assert!("cycler(foobar=[1, 2])".parse::<Cycler>().is_err());
    /// assert!("cycler('linewidth', 1)".parse::<Cycler>().is_err());
    /// assert!("cycler(color='rgb', c='cmy')".parse::<Cycler>().is_err());
    /// # Ok::<(), plt::rc::CyclerError>(())
    /// ```
    fn from_str(text: &str) -> Result<Self, CyclerError> {
        Self::parse_rc(text).map_err(CyclerError)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Cycler, String> {
        Cycler::parse_rc(text)
    }

    #[test]
    fn markevery_cycles_take_every_kind_of_value() {
        let cycle = parse(
            "cycler(markevery=[None, 8, (30, 8), [16, 24], slice(100, 200, 3), 0.1, (0.0, 0.1)])",
        )
        .unwrap();
        let values: Vec<CycleValue> = cycle.rows.iter().map(|row| row[0].clone()).collect();
        assert_eq!(values[0], CycleValue::MarkEvery(None));
        assert_eq!(
            values[2],
            CycleValue::MarkEvery(Some(MarkEvery::Every { start: 30, step: 8 }))
        );
        assert_eq!(
            values[4],
            CycleValue::MarkEvery(Some(MarkEvery::Slice {
                start: Some(100),
                stop: Some(200),
                step: Some(3)
            }))
        );
        assert_eq!(
            values[6],
            CycleValue::MarkEvery(Some(MarkEvery::Distance {
                start: 0.0,
                step: 0.1
            }))
        );
        assert!(parse("cycler(markevery=[(1, 0.5)])").is_err());
        assert!(parse("cycler(markevery=[[1.5]])").is_err());
        assert_eq!(parse(&cycle.format_rc()).unwrap(), cycle);
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
