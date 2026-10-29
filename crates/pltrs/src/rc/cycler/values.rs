//! The values each property of a cycler takes (matplotlib's
//! `_prop_validators`), read from rc text and written back.

use std::fmt::Write as _;

use super::{ALIASES, CycleValue, Cycler};
use crate::colors::{Color, color_sequence};
use crate::lines::{LineStyle, MarkEvery};
use crate::markers::Marker;
use crate::rc::literal::Expr;
use crate::rc::value::RcValue;

/// Validates the values of one property as matplotlib's `_prop_validators`.
pub(super) fn property_cycle(property: &str, values: &Expr) -> Result<Cycler, String> {
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
        "hatch" => items(values, |item| match item {
            Expr::Str(text) if is_hatch(text) => Some(CycleValue::Text(text.clone())),
            _ => None,
        })?,
        "markevery" => items(values, markevery)?,
        "joinstyle" | "capstyle" | "fillstyle" => items(values, |item| match item {
            Expr::Str(text) => Some(CycleValue::Text(text.clone())),
            Expr::Int(n) => Some(CycleValue::Text(n.to_string())),
            Expr::Number(n) => Some(CycleValue::Text(n.to_string())),
            _ => None,
        })?,
        unknown => return Err(format!("unknown artist property: {unknown}")),
    };
    Ok(Cycler::new(key, parsed))
}

/// matplotlib's `_validate_markevery`: `None`, an integer step, a float
/// distance, a pair of integers or of floats, a list of integers or a
/// `slice(...)`.
fn markevery(item: &Expr) -> Option<CycleValue> {
    let ints = |items: &[Expr]| -> Option<Vec<i64>> {
        items
            .iter()
            .map(|e| match e {
                Expr::Int(n) => Some(*n),
                _ => None,
            })
            .collect()
    };
    let floats = |items: &[Expr]| -> Option<Vec<f64>> {
        items
            .iter()
            .map(|e| match e {
                Expr::Number(x) => Some(*x),
                _ => None,
            })
            .collect()
    };
    Some(CycleValue::MarkEvery(Some(match item {
        Expr::None => return Some(CycleValue::MarkEvery(None)),
        Expr::Int(step) => MarkEvery::Every {
            start: 0,
            step: *step,
        },
        Expr::Number(step) => MarkEvery::Distance {
            start: 0.0,
            step: *step,
        },
        Expr::Tuple(pair) if pair.len() == 2 => match (ints(pair), floats(pair)) {
            (Some(i), _) => MarkEvery::Every {
                start: i[0],
                step: i[1],
            },
            (_, Some(f)) => MarkEvery::Distance {
                start: f[0],
                step: f[1],
            },
            _ => return None,
        },
        Expr::List(indices) => MarkEvery::Indices(ints(indices)?),
        Expr::Call { name, args, kwargs } if name == "slice" && kwargs.is_empty() => {
            let bound = |e: &Expr| match e {
                Expr::None => Some(None),
                Expr::Int(n) => Some(Some(*n)),
                _ => None,
            };
            let bounds: Option<Vec<Option<i64>>> = args.iter().map(bound).collect();
            match bounds?.as_slice() {
                [stop] => MarkEvery::Slice {
                    start: None,
                    stop: *stop,
                    step: None,
                },
                [start, stop] => MarkEvery::Slice {
                    start: *start,
                    stop: *stop,
                    step: None,
                },
                [start, stop, step] => MarkEvery::Slice {
                    start: *start,
                    stop: *stop,
                    step: *step,
                },
                _ => return None,
            }
        }
        _ => return None,
    })))
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
    // matplotlib's list validators skip empty strings.
    elements
        .iter()
        .filter(|item| !matches!(item, Expr::Str(text) if text.is_empty()))
        .map(|item| convert(item).ok_or_else(|| format!("invalid cycle value {item:?}")))
        .collect()
}

/// matplotlib's `validate_hatch`: only the hatch pattern characters.
fn is_hatch(pattern: &str) -> bool {
    pattern.chars().all(|c| {
        matches!(
            c,
            '\\' | '/' | '|' | '-' | '+' | '*' | '.' | 'x' | 'o' | 'O'
        )
    })
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

/// A cycle value in the syntax `cycler(...)` takes back: opaque colours
/// as `(r, g, b)` tuples, as matplotlib's colour sequences hold them.
pub(super) fn format_value(value: &CycleValue) -> String {
    match value {
        CycleValue::Color(Color::Spec(spec)) => format!("'{spec}'"),
        CycleValue::Color(Color::Rgba(c)) if c.a >= 1.0 => format!("({}, {}, {})", c.r, c.g, c.b),
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
        CycleValue::MarkEvery(markevery) => format_markevery(markevery.as_ref()),
        CycleValue::Text(text) => format!("'{text}'"),
    }
}

/// A `markevery` value in Python syntax.
fn format_markevery(markevery: Option<&MarkEvery>) -> String {
    let join = |items: Vec<String>| items.join(", ");
    match markevery {
        None => "None".to_owned(),
        Some(MarkEvery::Every { start: 0, step }) => step.to_string(),
        Some(MarkEvery::Every { start, step }) => format!("({start}, {step})"),
        Some(MarkEvery::Slice { start, stop, step }) => {
            let bound = |b: &Option<i64>| b.map_or_else(|| "None".to_owned(), |b| b.to_string());
            format!("slice({}, {}, {})", bound(start), bound(stop), bound(step))
        }
        Some(MarkEvery::Indices(indices)) => {
            format!(
                "[{}]",
                join(indices.iter().map(ToString::to_string).collect())
            )
        }
        Some(MarkEvery::Mask(mask)) => {
            let flag = |set: &bool| if *set { "True" } else { "False" }.to_owned();
            format!("[{}]", join(mask.iter().map(flag).collect()))
        }
        Some(MarkEvery::Distance { start, step }) if *start == 0.0 => step.format_rc(),
        Some(MarkEvery::Distance { start, step }) => {
            format!("({}, {})", start.format_rc(), step.format_rc())
        }
    }
}
