//! Event plots (matplotlib's `eventplot`): one row of short lines per
//! dataset, one line per event.

use plt_core::{Path, Point};

use super::children::DataUpdate;
use super::{Axes, PlotError};
use crate::artist::Artist;
use crate::collections::{Collection, CollectionArtist, CollectionKind, EdgeColors};
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::orientation::Orientation;
use crate::units::AxisValues;

/// Options of [`Axes::eventplot`]. Every per-dataset list holds one value
/// for all datasets or one per dataset.
#[derive(Clone, Debug, PartialEq)]
pub struct EventplotOptions {
    /// `Horizontal` (the default) places events along x, drawn as vertical
    /// lines; `Vertical` places them along y.
    pub orientation: Orientation,
    /// Centre of each row of lines; a single value `o` puts the rows at
    /// `0, o, 2o, ...`.
    pub lineoffsets: Vec<f64>,
    /// Length of the lines of each row.
    pub linelengths: Vec<f64>,
    /// Line width of each row in points (empty: `lines.linewidth`).
    pub linewidths: Vec<f64>,
    /// Colour of each row (empty: `lines.color`).
    pub colors: Vec<Color>,
    /// Opacity of each row (empty: opaque).
    pub alpha: Vec<f64>,
    /// Line style of each row.
    pub linestyles: Vec<LineStyle>,
}

impl Default for EventplotOptions {
    fn default() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            lineoffsets: vec![1.0],
            linelengths: vec![1.0],
            linewidths: Vec::new(),
            colors: Vec::new(),
            alpha: Vec::new(),
            linestyles: vec![LineStyle::Solid],
        }
    }
}

impl Axes {
    /// Draws each dataset of `positions` as a row of lines, one at each
    /// event (matplotlib's `eventplot`); returns one collection per row.
    /// The events are numbers, dates or categories of the axis they lie
    /// along; `options.lineoffsets` are numbers of the other axis (see
    /// [`Axis::convert_value`](crate::axes::Axis::convert_value)).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::EventplotOptions;
    /// let ax = Figure::new().subplot();
    /// let rows = ax.eventplot(&[vec![1.0, 3.0], vec![2.0]], EventplotOptions::default())?;
    /// assert_eq!(rows.len(), 2);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a per-dataset option is empty (colours excepted) or holds
    /// neither one value nor one per dataset.
    pub fn eventplot<P: Clone + Into<AxisValues>>(
        &self,
        positions: &[P],
        options: EventplotOptions,
    ) -> Result<Vec<Collection>, PlotError> {
        let along = match options.orientation {
            Orientation::Horizontal => self.xaxis(),
            Orientation::Vertical => self.yaxis(),
        };
        let positions: Vec<Vec<f64>> = positions
            .iter()
            .map(|events| along.convert_units(events.clone()))
            .collect();
        let n = positions.len();
        let offsets = row_offsets(&options.lineoffsets, n)?;
        let lengths = per_dataset("linelengths", &options.linelengths, n)?;
        let styles = per_dataset("linestyles", &options.linestyles, n)?;
        let widths = optional_per_dataset("linewidths", &options.linewidths, n)?;
        let colors = optional_per_dataset("colors", &options.colors, n)?;
        let alphas = optional_per_dataset("alpha", &options.alpha, n)?;
        let rc = crate::rc::params();
        let rows = positions
            .iter()
            .enumerate()
            .map(|(i, events)| {
                let paths = event_lines(events, options.orientation, offsets[i], lengths[i]);
                let mut row = CollectionArtist::new(CollectionKind::Line, paths, rc.clone());
                let color = colors
                    .get(i)
                    .map_or_else(|| rc.lines_color.clone(), Clone::clone);
                row.edgecolors = EdgeColors::Colors(vec![color]);
                row.linewidths = widths.get(i).map(|&w| vec![w]);
                row.linestyles = vec![styles[i].clone()];
                row.base_mut().alpha = alphas.get(i).copied();
                row.base_mut().transform = Some(self.trans_data());
                Collection::from_raw(self.adopt(row, &DataUpdate::default()))
            })
            .collect();
        if positions.iter().any(|row| !row.is_empty()) {
            let events = positions.iter().flatten().copied();
            let low = events.clone().fold(f64::INFINITY, f64::min);
            let high = events.fold(f64::NEG_INFINITY, f64::max);
            // matplotlib pads by whole line lengths, not half.
            let line_low = offsets.iter().zip(&lengths).map(|(o, l)| o - l);
            let line_high = offsets.iter().zip(&lengths).map(|(o, l)| o + l);
            let line_low = line_low.fold(f64::INFINITY, f64::min);
            let line_high = line_high.fold(f64::NEG_INFINITY, f64::max);
            let corners = match options.orientation {
                Orientation::Horizontal => [(low, line_low), (high, line_high)],
                Orientation::Vertical => [(line_low, low), (line_high, high)],
            };
            let corners = corners.map(|(x, y)| Point::new(x, y)).to_vec();
            self.update_data(&DataUpdate::data(corners));
        }
        Ok(rows)
    }
}

/// The sorted events of one row as lines across `offset ± length / 2`.
fn event_lines(events: &[f64], orientation: Orientation, offset: f64, length: f64) -> Vec<Path> {
    let mut sorted = events.to_vec();
    sorted.sort_by(f64::total_cmp);
    let (start, end) = (offset + length / 2.0, offset - length / 2.0);
    sorted
        .into_iter()
        .map(|at| {
            let (a, b) = match orientation {
                Orientation::Horizontal => (Point::new(at, start), Point::new(at, end)),
                Orientation::Vertical => (Point::new(start, at), Point::new(end, at)),
            };
            Path::polyline(vec![a, b])
        })
        .collect()
}

/// The row centres: one per dataset, or `0, o, 2o, ...` from a single `o`.
fn row_offsets(offsets: &[f64], n: usize) -> Result<Vec<f64>, PlotError> {
    match offsets {
        [] => Err(PlotError::DatasetCount {
            argument: "lineoffsets",
            len: 0,
            datasets: n,
        }),
        &[step] if n != 1 => Ok((0..n).map(|i| step * i as f64).collect()),
        _ => per_dataset("lineoffsets", offsets, n),
    }
}

/// `values` broadcast to one per dataset.
fn per_dataset<T: Clone>(
    argument: &'static str,
    values: &[T],
    n: usize,
) -> Result<Vec<T>, PlotError> {
    match values {
        [value] => Ok(vec![value.clone(); n]),
        _ if values.len() == n && n > 0 => Ok(values.to_vec()),
        _ => Err(PlotError::DatasetCount {
            argument,
            len: values.len(),
            datasets: n,
        }),
    }
}

/// Like [`per_dataset`], but an empty list keeps the defaults.
fn optional_per_dataset<T: Clone>(
    argument: &'static str,
    values: &[T],
    n: usize,
) -> Result<Vec<T>, PlotError> {
    if values.is_empty() {
        Ok(Vec::new())
    } else {
        per_dataset(argument, values, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn rows_stack_from_zero_with_a_single_offset() {
        assert_eq!(row_offsets(&[2.0], 3).unwrap(), [0.0, 2.0, 4.0]);
        assert_eq!(row_offsets(&[2.0], 1).unwrap(), [2.0]);
        assert!(row_offsets(&[], 2).is_err());
        assert!(row_offsets(&[1.0, 2.0], 3).is_err());
    }

    #[test]
    fn events_are_sorted_lines_across_the_offset() {
        let lines = event_lines(&[3.0, 1.0], Orientation::Horizontal, 1.0, 0.5);
        assert_eq!(
            lines[0].vertices(),
            [Point::new(1.0, 1.25), Point::new(1.0, 0.75)]
        );
        let vertical = event_lines(&[2.0], Orientation::Vertical, 0.0, 1.0);
        assert_eq!(
            vertical[0].vertices(),
            [Point::new(0.5, 2.0), Point::new(-0.5, 2.0)]
        );
    }

    #[test]
    fn limits_pad_by_whole_line_lengths() {
        let ax = Figure::new().subplot();
        ax.eventplot(&[vec![0.0, 10.0]], EventplotOptions::default())
            .unwrap();
        ax.set_margins(0.0, 0.0).unwrap();
        assert_eq!(ax.xlim(), (0.0, 10.0));
        assert_eq!(ax.ylim(), (0.0, 2.0));
    }

    #[test]
    fn empty_datasets_leave_the_limits_alone() {
        let ax = Figure::new().subplot();
        let rows = ax
            .eventplot(
                &[Vec::<f64>::new(), Vec::new()],
                EventplotOptions::default(),
            )
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(ax.xlim(), (0.0, 1.0));
    }
}
