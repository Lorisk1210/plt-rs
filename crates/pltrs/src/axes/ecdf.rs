//! Empirical cumulative distribution functions (`Axes::ecdf`).

use std::cmp::Ordering;
use std::fmt;

use super::{Axes, PlotError};
use crate::artist::ArtistHandle;
use crate::lines::{DrawStyle, Line2D, LineProps};
use crate::numeric::{Number, argsort, to_f64s};
use crate::orientation::Orientation;
use crate::stats::pairwise_sum;

/// Options of [`Axes::ecdf`] (matplotlib's keyword arguments).
#[derive(Clone, Debug, Default)]
pub struct EcdfOptions {
    /// The weight of each value (`None` = equal weights).
    pub weights: Option<Vec<f64>>,
    /// Plot the complementary function, `1 - F(x)`.
    pub complementary: bool,
    /// Whether the values run along x (vertical) or y (horizontal).
    pub orientation: Orientation,
    /// Keep one step per distinct value.
    pub compress: bool,
    /// Properties of the step line (its draw style is chosen by `ecdf`).
    pub line: LineProps,
}

/// Why an empirical distribution cannot be plotted.
#[derive(Clone, Debug, PartialEq)]
pub enum EcdfError {
    /// There are no values.
    Empty,
    /// A value is NaN (sort NaNs to an end by replacing them with an
    /// infinity, or drop them).
    Nan,
    /// The weights are not one per value.
    WeightCount {
        /// Number of values.
        values: usize,
        /// Number of weights.
        weights: usize,
    },
    /// A draw style was given; `ecdf` chooses the steps itself.
    DrawStyle,
    /// The line could not be plotted.
    Plot(PlotError),
}

impl fmt::Display for EcdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("ecdf() needs at least one value"),
            Self::Nan => f.write_str("ecdf() does not support NaNs"),
            Self::WeightCount { values, weights } => {
                write!(f, "{weights} weights given for {values} values")
            }
            Self::DrawStyle => f.write_str("cannot pass a draw style to ecdf()"),
            Self::Plot(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for EcdfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Plot(err) => Some(err),
            _ => None,
        }
    }
}

/// The sorted values and the cumulative fraction of the weight up to and
/// including each, one per distinct value when `compress`.
fn cumulative(
    x: &[f64],
    weights: Option<&[f64]>,
    compress: bool,
) -> Result<(Vec<f64>, Vec<f64>), EcdfError> {
    if x.is_empty() {
        return Err(EcdfError::Empty);
    }
    if x.iter().any(|v| v.is_nan()) {
        return Err(EcdfError::Nan);
    }
    let order = argsort(x);
    let sorted: Vec<f64> = order.iter().map(|&i| x[i]).collect();
    let fractions: Vec<f64> = match weights {
        // Ending at exactly 1, like matplotlib.
        None => (1..=x.len()).map(|i| i as f64 / x.len() as f64).collect(),
        Some(weights) => {
            if weights.len() != x.len() {
                return Err(EcdfError::WeightCount {
                    values: x.len(),
                    weights: weights.len(),
                });
            }
            let ordered: Vec<f64> = order.iter().map(|&i| weights[i]).collect();
            let total = pairwise_sum(&ordered);
            ordered
                .iter()
                .scan(0.0, |sum, w| {
                    *sum += w / total;
                    Some(*sum)
                })
                .collect()
        }
    };
    if !compress {
        return Ok((sorted, fractions));
    }
    Ok(sorted
        .iter()
        .zip(&fractions)
        .enumerate()
        .filter(|&(i, (value, _))| {
            i == 0 || sorted[i - 1].partial_cmp(value) != Some(Ordering::Equal)
        })
        .map(|(_, (&value, &fraction))| (value, fraction))
        .unzip())
}

impl Axes {
    /// Plots the empirical cumulative distribution of `x` as a step line
    /// that starts at 0 and ends at 1 (or, complementary, the reverse),
    /// whose fraction axis sticks to `[0, 1]` (matplotlib's `ecdf`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::EcdfOptions;
    /// let ax = Figure::new().subplot();
    /// let line = ax.ecdf(&[3.0, 1.0, 2.0], EcdfOptions::default())?;
    /// assert_eq!(line.xdata(), [1.0, 1.0, 2.0, 3.0]);
    /// assert_eq!(line.ydata()[3], 1.0);
    /// # Ok::<(), pltrs::axes::EcdfError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `x` is empty or holds NaNs, the weights do not match the
    /// values, or `options.line` sets a draw style.
    pub fn ecdf(
        &self,
        x: impl IntoIterator<Item: Number>,
        options: EcdfOptions,
    ) -> Result<Line2D, EcdfError> {
        let x = &to_f64s(x);
        if options.line.drawstyle.is_some() {
            return Err(EcdfError::DrawStyle);
        }
        let (values, fractions) = cumulative(x, options.weights.as_deref(), options.compress)?;
        let first = values[0];
        let last = values[values.len() - 1];
        let (start, values_line, steps): (f64, Vec<f64>, DrawStyle) = if options.complementary {
            (
                1.0,
                [values.as_slice(), &[last]].concat(),
                DrawStyle::StepsPre,
            )
        } else {
            (
                0.0,
                [&[first], values.as_slice()].concat(),
                DrawStyle::StepsPost,
            )
        };
        let fraction_line: Vec<f64> = std::iter::once(start)
            .chain(
                fractions
                    .iter()
                    .map(|&f| if options.complementary { 1.0 - f } else { f }),
            )
            .collect();
        let (x_line, y_line, drawstyle) = match options.orientation {
            Orientation::Vertical => (values_line, fraction_line, steps),
            Orientation::Horizontal => (fraction_line, values_line, mirrored(steps)),
        };
        let line = self
            .plot_with(x_line, y_line, options.line.drawstyle(drawstyle))
            .map_err(EcdfError::Plot)?;
        line.raw().with_base_mut(|base| match options.orientation {
            Orientation::Vertical => base.sticky_edges.1 = vec![0.0, 1.0],
            Orientation::Horizontal => base.sticky_edges.0 = vec![0.0, 1.0],
        });
        Ok(line)
    }
}

/// The steps of a horizontal function: pre and post swap when the axes
/// do.
fn mirrored(steps: DrawStyle) -> DrawStyle {
    if steps == DrawStyle::StepsPost {
        DrawStyle::StepsPre
    } else {
        DrawStyle::StepsPost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn fractions_end_at_one_and_follow_the_weights() {
        let (values, fractions) = cumulative(&[3.0, 1.0, 2.0], None, false).unwrap();
        assert_eq!(values, [1.0, 2.0, 3.0]);
        assert_eq!(fractions, [1.0 / 3.0, 2.0 / 3.0, 1.0]);
        let (_, weighted) = cumulative(&[3.0, 1.0], Some(&[1.0, 3.0]), false).unwrap();
        assert_eq!(weighted, [0.75, 1.0]);
    }

    #[test]
    fn compression_keeps_the_first_of_equal_values() {
        let (values, fractions) = cumulative(&[2.0, 1.0, 2.0, 2.0], None, true).unwrap();
        assert_eq!(values, [1.0, 2.0]);
        assert_eq!(fractions, [0.25, 0.5]);
    }

    #[test]
    fn invalid_input_is_refused() {
        assert_eq!(cumulative(&[], None, false), Err(EcdfError::Empty));
        assert_eq!(
            cumulative(&[1.0, f64::NAN], None, false),
            Err(EcdfError::Nan)
        );
        assert_eq!(
            cumulative(&[1.0], Some(&[1.0, 2.0]), false),
            Err(EcdfError::WeightCount {
                values: 1,
                weights: 2
            })
        );
        let ax = Figure::new().subplot();
        let options = EcdfOptions {
            line: LineProps::default().drawstyle(DrawStyle::Default),
            ..EcdfOptions::default()
        };
        assert_eq!(ax.ecdf([1.0], options).err(), Some(EcdfError::DrawStyle));
    }

    #[test]
    fn complementary_horizontal_functions_fall_from_one() {
        let ax = Figure::new().subplot();
        let line = ax
            .ecdf(
                [1.0, 2.0],
                EcdfOptions {
                    complementary: true,
                    orientation: Orientation::Horizontal,
                    ..EcdfOptions::default()
                },
            )
            .unwrap();
        assert_eq!(line.xdata(), [1.0, 0.5, 0.0]);
        assert_eq!(line.ydata(), [1.0, 2.0, 2.0]);
        assert_eq!(line.drawstyle(), DrawStyle::StepsPost);
    }
}
