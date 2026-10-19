//! Angle labels in degrees or hours with minutes and seconds
//! (matplotlib's `FormatterDMS` and `FormatterHMS`).

use crate::axisartist::grid_finder::{GridError, GridFormatter};
use crate::spines::SpineSide;
use crate::ticker::pyfmt::{PrintfFormat, integer, repr};
use crate::ticker::pymath::{divmod, np_sign, pow10};

/// The marks written after the units, minutes and seconds.
struct Marks {
    unit: &'static str,
    minute: &'static str,
    second: &'static str,
}

/// Degrees, minutes and seconds of arc.
const DMS: Marks = Marks {
    unit: r"^{\circ}",
    minute: r"^{\prime}",
    second: r"^{\prime\prime}",
};

/// Hours, minutes and seconds of time.
const HMS: Marks = Marks {
    unit: r"^\mathrm{h}",
    minute: r"^\mathrm{m}",
    second: r"^\mathrm{s}",
};

/// matplotlib's `_get_number_fraction`: when the factor is a power of ten
/// times 1, 60 or 3600, that base factor and the number of decimals.
fn number_fraction(factor: f64) -> (f64, Option<i32>) {
    for threshold in [1.0, 60.0, 3600.0] {
        if factor <= threshold {
            break;
        }
        let d = divmod(factor, threshold).0;
        let int_log_d = d.log10().floor() as i32;
        #[expect(
            clippy::float_cmp,
            reason = "matplotlib tests for an exact power of ten"
        )]
        let power_of_ten = pow10(f64::from(int_log_d)) == d && d != 1.0;
        if power_of_ten {
            return (
                divmod(factor, pow10(f64::from(int_log_d))).0,
                Some(int_log_d),
            );
        }
    }
    (factor, None)
}

/// Python's `'%0{width}d' % value`.
fn zero_padded(value: f64, width: i32) -> Result<String, GridError> {
    Ok(PrintfFormat::parse(&format!("%0{width}d"))?.format(value)?)
}

/// The labels of `values` (scaled by `factor`) with `marks` (matplotlib's
/// `FormatterDMS.__call__`).
fn angle_labels(marks: &Marks, factor: f64, values: &[f64]) -> Result<Vec<String>, GridError> {
    if values.is_empty() {
        return Ok(Vec::new());
    }
    let Marks {
        unit,
        minute,
        second,
    } = marks;
    let signs: Vec<f64> = values.iter().map(|&v| np_sign(v)).collect();
    let sign_texts: Vec<&str> = values
        .iter()
        .map(|&v| if v < 0.0 { "-" } else { "" })
        .collect();
    let (factor, fraction) = number_fraction(factor);
    let mut magnitudes: Vec<f64> = values.iter().map(|v| v.abs()).collect();
    let mut fractions = Vec::new();
    if let Some(digits) = fraction {
        let scale = pow10(f64::from(digits));
        for magnitude in &mut magnitudes {
            let (whole, part) = divmod(*magnitude, scale);
            *magnitude = whole;
            fractions.push(zero_padded(part, digits)?);
        }
    }
    let minutes_of = |v: f64| divmod(v, 60.0);
    match factor {
        1.0 => signs
            .iter()
            .zip(&magnitudes)
            .enumerate()
            .map(|(i, (&s, &v))| {
                let degrees = integer(s * v.trunc());
                Ok(match fractions.get(i) {
                    None => format!("${degrees}{unit}$"),
                    Some(f) => format!("${degrees}.{f}{unit}$"),
                })
            })
            .collect(),
        60.0 => sign_texts
            .iter()
            .zip(&magnitudes)
            .enumerate()
            .map(|(i, (s, &v))| {
                let (d, m) = minutes_of(v);
                let (d, m) = (integer(d), zero_padded(m, 2)?);
                Ok(match fractions.get(i) {
                    None => format!("${s}{d}{unit}\\,{m}{minute}$"),
                    Some(f) => format!("${s}{d}{unit}\\,{m}.{f}{minute}$"),
                })
            })
            .collect(),
        3600.0 => {
            let reversed = signs.last() == Some(&-1.0);
            if reversed {
                magnitudes.reverse();
            }
            let mut sign_texts = sign_texts;
            if reversed {
                sign_texts.reverse();
            }
            let mut labels = Vec::with_capacity(magnitudes.len());
            let mut previous = String::new();
            for (i, (s, &v)) in sign_texts.iter().zip(&magnitudes).enumerate() {
                let (d, rest) = divmod(v, 3600.0);
                let (m, sec) = minutes_of(rest);
                let seconds = match fractions.get(i) {
                    None => format!("{}{second}$", zero_padded(sec, 2)?),
                    Some(f) => format!("{}.{f}{second}$", zero_padded(sec, 2)?),
                };
                let head = format!(
                    "${s}{}{unit}\\,{}{minute}\\,",
                    integer(d),
                    zero_padded(m, 2)?
                );
                labels.push(if head == previous {
                    format!("${s}{seconds}")
                } else {
                    let label = format!("{head}{seconds}");
                    previous = head;
                    label
                });
            }
            if reversed {
                labels.reverse();
            }
            Ok(labels)
        }
        _ => Ok(signs
            .iter()
            .zip(&magnitudes)
            .map(|(&s, &v)| format!("${}^{{\\circ}}$", repr(s * v)))
            .collect()),
    }
}

/// Labels levels in degrees as degrees, minutes and seconds of arc in
/// mathtext (matplotlib's `FormatterDMS`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatterDms;

impl GridFormatter for FormatterDms {
    fn labels(&self, _: SpineSide, factor: f64, values: &[f64]) -> Result<Vec<String>, GridError> {
        angle_labels(&DMS, factor, values)
    }
}

/// Labels levels in degrees as hours, minutes and seconds of time in
/// mathtext (matplotlib's `FormatterHMS`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatterHms;

impl GridFormatter for FormatterHms {
    fn labels(&self, _: SpineSide, factor: f64, values: &[f64]) -> Result<Vec<String>, GridError> {
        let hours: Vec<f64> = values.iter().map(|v| v / 15.0).collect();
        angle_labels(&HMS, factor, &hours)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_write_degrees_minutes_and_seconds() {
        let minutes = FormatterDms
            .labels(SpineSide::Bottom, 60.0, &[0.0, 90.0, -45.0])
            .unwrap();
        assert_eq!(
            minutes,
            [
                r"$0^{\circ}\,00^{\prime}$",
                r"$1^{\circ}\,30^{\prime}$",
                r"$-0^{\circ}\,45^{\prime}$"
            ]
        );
        // Repeated degrees and minutes are left out; tenths of seconds
        // come from a factor of 36000 (matplotlib's labels).
        let seconds = FormatterDms
            .labels(
                SpineSide::Bottom,
                36000.0,
                &[0.0, 5.0, 15.0, -25.0, 36015.0],
            )
            .unwrap();
        assert_eq!(
            seconds,
            [
                r"$0^{\circ}\,00^{\prime}\,00.0^{\prime\prime}$",
                r"$00.5^{\prime\prime}$",
                r"$01.5^{\prime\prime}$",
                r"$-0^{\circ}\,00^{\prime}\,02.5^{\prime\prime}$",
                r"$1^{\circ}\,00^{\prime}\,01.5^{\prime\prime}$"
            ]
        );
        let hours = FormatterHms
            .labels(SpineSide::Left, 1.0, &[120.0, 135.0])
            .unwrap();
        assert_eq!(hours, [r"$8^\mathrm{h}$", r"$9^\mathrm{h}$"]);
        assert_eq!(
            FormatterDms.labels(SpineSide::Top, 1.0, &[]),
            Ok(Vec::new())
        );
    }
}
