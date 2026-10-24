//! Grouped bar charts (matplotlib's `grouped_bar`): one bar per dataset
//! in each category group.

use std::fmt;

use super::{Axes, BarAlign, BarContainer, BarOptions, PlotError};
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::numeric::Grid;
use crate::orientation::Orientation;
use crate::patches::PatchProps;
use crate::ticker::AxisName;

/// The heights of a grouped bar chart.
#[derive(Clone, Debug, PartialEq)]
pub enum GroupedBarData {
    /// One sequence per dataset, one value per category.
    Datasets(Vec<Vec<f64>>),
    /// Named datasets (matplotlib's dict input); the names label them.
    Named(Vec<(String, Vec<f64>)>),
    /// One row per category and one column per dataset.
    Table(Grid<f64>),
}

impl From<Vec<Vec<f64>>> for GroupedBarData {
    fn from(datasets: Vec<Vec<f64>>) -> Self {
        Self::Datasets(datasets)
    }
}

impl From<Grid<f64>> for GroupedBarData {
    fn from(table: Grid<f64>) -> Self {
        Self::Table(table)
    }
}

/// Options of [`Axes::grouped_bar`]. Each per-dataset style list is
/// cycled over the datasets; empty lists leave the style alone.
#[derive(Clone, Debug)]
pub struct GroupedBarOptions {
    /// Equidistant group centres (default `0, 1, 2, ...`).
    pub positions: Option<Vec<f64>>,
    /// Gap between groups, in bar widths (default 1.5).
    pub group_spacing: f64,
    /// Gap between the bars of a group, in bar widths (default 0).
    pub bar_spacing: f64,
    /// Where the bars of every dataset start along the value direction:
    /// one value or one per category (`bottom` of vertical bars, `left`
    /// of horizontal ones; default 0).
    pub base: Vec<f64>,
    /// Category labels at the group centres (empty: ticks unchanged).
    pub tick_labels: Vec<String>,
    /// Dataset labels for the legend (empty: none).
    pub labels: Vec<String>,
    /// Which way the bars grow.
    pub orientation: Orientation,
    /// Dataset colours (empty: the property cycle).
    pub colors: Vec<Color>,
    /// Dataset face colours, taking precedence over `colors`.
    pub facecolor: Vec<Color>,
    /// Dataset edge colours.
    pub edgecolor: Vec<Color>,
    /// Dataset edge widths in points.
    pub linewidth: Vec<f64>,
    /// Dataset edge styles.
    pub linestyle: Vec<LineStyle>,
    /// Dataset hatch patterns.
    pub hatch: Vec<String>,
    /// Properties of every bar.
    pub props: PatchProps,
}

impl Default for GroupedBarOptions {
    fn default() -> Self {
        Self {
            positions: None,
            group_spacing: 1.5,
            bar_spacing: 0.0,
            base: vec![0.0],
            tick_labels: Vec::new(),
            labels: Vec::new(),
            orientation: Orientation::Vertical,
            colors: Vec::new(),
            facecolor: Vec::new(),
            edgecolor: Vec::new(),
            linewidth: Vec::new(),
            linestyle: Vec::new(),
            hatch: Vec::new(),
            props: PatchProps::default(),
        }
    }
}

/// Why a grouped bar chart cannot be drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum GroupedBarError {
    /// No datasets.
    NoDatasets,
    /// Datasets of different lengths.
    Ragged {
        /// The offending dataset.
        dataset: usize,
        /// Its length.
        len: usize,
        /// The length of the first dataset.
        expected: usize,
    },
    /// Group positions that are not equally spaced.
    UnevenPositions,
    /// Labels given together with named datasets.
    LabelsWithNames,
    /// Labels whose number differs from the number of datasets.
    LabelCount {
        /// Labels given.
        labels: usize,
        /// Datasets.
        datasets: usize,
    },
    /// The bars of a dataset cannot be drawn.
    Bars(PlotError),
}

impl fmt::Display for GroupedBarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDatasets => f.write_str("'heights' contains no datasets"),
            Self::Ragged {
                dataset,
                len,
                expected,
            } => write!(
                f,
                "'heights' contains datasets with different number of elements. dataset 0 \
                 has {expected} elements but dataset {dataset} has {len} elements."
            ),
            Self::UnevenPositions => f.write_str("'positions' must be equidistant"),
            Self::LabelsWithNames => {
                f.write_str("'labels' cannot be used if 'heights' is a mapping")
            }
            Self::LabelCount { labels, datasets } => {
                write!(f, "{labels} labels given for {datasets} datasets")
            }
            Self::Bars(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for GroupedBarError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bars(err) => Some(err),
            _ => None,
        }
    }
}

impl From<PlotError> for GroupedBarError {
    fn from(err: PlotError) -> Self {
        Self::Bars(err)
    }
}

/// One dataset: a value per category and its legend label.
#[derive(Clone, Debug, PartialEq)]
struct Dataset {
    values: Vec<f64>,
    label: Option<String>,
}

/// The datasets after matplotlib's input handling.
fn datasets(heights: GroupedBarData, labels: &[String]) -> Result<Vec<Dataset>, GroupedBarError> {
    let (datasets, names) = match heights {
        GroupedBarData::Named(_) if !labels.is_empty() => {
            return Err(GroupedBarError::LabelsWithNames);
        }
        GroupedBarData::Named(named) => {
            let (names, datasets): (Vec<String>, _) = named.into_iter().unzip();
            (datasets, names)
        }
        GroupedBarData::Datasets(datasets) => (datasets, labels.to_vec()),
        GroupedBarData::Table(table) => {
            let columns = (0..table.cols()).map(|c| table.column(c)).collect();
            (columns, labels.to_vec())
        }
    };
    let expected = datasets.first().ok_or(GroupedBarError::NoDatasets)?.len();
    if let Some((dataset, values)) = datasets
        .iter()
        .enumerate()
        .find(|(_, values)| values.len() != expected)
    {
        return Err(GroupedBarError::Ragged {
            dataset,
            len: values.len(),
            expected,
        });
    }
    let labels: Vec<Option<String>> = if names.is_empty() {
        vec![None; datasets.len()]
    } else if names.len() == datasets.len() {
        names.into_iter().map(Some).collect()
    } else {
        return Err(GroupedBarError::LabelCount {
            labels: names.len(),
            datasets: datasets.len(),
        });
    };
    Ok(datasets
        .into_iter()
        .zip(labels)
        .map(|(values, label)| Dataset { values, label })
        .collect())
}

/// The group centres and the distance between groups.
fn group_layout(
    positions: Option<&[f64]>,
    groups: usize,
) -> Result<(Vec<f64>, f64), GroupedBarError> {
    let Some(centers) = positions else {
        return Ok(((0..groups).map(|g| g as f64).collect(), 1.0));
    };
    if centers.len() <= 1 {
        return Ok((centers.to_vec(), 1.0));
    }
    let steps: Vec<f64> = centers.windows(2).map(|w| w[1] - w[0]).collect();
    let mean = steps.iter().sum::<f64>() / steps.len() as f64;
    // numpy's `allclose` with its default tolerances.
    if steps
        .iter()
        .any(|d| (d - mean).abs() > 1e-8 + 1e-5 * mean.abs())
    {
        return Err(GroupedBarError::UnevenPositions);
    }
    Ok((centers.to_vec(), steps[0]))
}

/// Item `i` of a cycled per-dataset list, if the list is not empty.
fn cycled<T: Clone>(values: &[T], i: usize) -> Option<T> {
    (!values.is_empty()).then(|| values[i % values.len()].clone())
}

impl Axes {
    /// One group of bars per category, one bar per dataset in each group
    /// (matplotlib's `grouped_bar`); returns the bars of each dataset.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{GroupedBarData, GroupedBarOptions};
    /// let ax = Figure::new().subplot();
    /// let data = GroupedBarData::Datasets(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    /// let bars = ax.grouped_bar(data, GroupedBarOptions::default())?;
    /// assert_eq!(bars.len(), 2);
    /// // Two bars and a gap of 1.5 bar widths share each unit step.
    /// assert_eq!(bars[1].patches[0].shape().width, 1.0 / 3.5);
    /// # Ok::<(), plt::axes::GroupedBarError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When there are no datasets, they differ in length, the positions
    /// are not equidistant, the labels do not match the datasets, or the
    /// bars cannot be drawn.
    pub fn grouped_bar(
        &self,
        heights: impl Into<GroupedBarData>,
        options: GroupedBarOptions,
    ) -> Result<Vec<BarContainer>, GroupedBarError> {
        let datasets = datasets(heights.into(), &options.labels)?;
        let (centers, distance) =
            group_layout(options.positions.as_deref(), datasets[0].values.len())?;
        let n = datasets.len() as f64;
        let width = distance / (n + (n - 1.0) * options.bar_spacing + options.group_spacing);
        let spacing = options.bar_spacing * width;
        let margin = 0.5 * options.group_spacing * width;
        let containers = datasets
            .into_iter()
            .enumerate()
            .map(|(i, Dataset { values, label })| {
                let lefts: Vec<f64> = centers
                    .iter()
                    .map(|c| c - 0.5 * distance + margin + i as f64 * (width + spacing))
                    .collect();
                let mut props = options.props.clone();
                if let Some(linestyle) = cycled(&options.linestyle, i) {
                    props.linestyle = Some(linestyle);
                }
                let face = cycled(&options.facecolor, i).or_else(|| cycled(&options.colors, i));
                let bar = BarOptions {
                    thickness: width.into(),
                    base: options.base.clone().into(),
                    align: BarAlign::Edge,
                    label,
                    color: face.into_iter().collect(),
                    edgecolor: cycled(&options.edgecolor, i).into_iter().collect(),
                    linewidth: cycled(&options.linewidth, i).into_iter().collect(),
                    hatch: cycled(&options.hatch, i).into_iter().collect(),
                    props,
                    ..BarOptions::default()
                };
                match options.orientation {
                    Orientation::Vertical => self.bar(lefts, values, bar),
                    Orientation::Horizontal => self.barh(lefts, values, bar),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !options.tick_labels.is_empty() {
            let axis = match options.orientation {
                Orientation::Vertical => AxisName::X,
                Orientation::Horizontal => AxisName::Y,
            };
            self.axis_of(axis)
                .set_ticks(centers)
                .set_ticklabels(options.tick_labels)?;
        }
        Ok(containers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn bars_fill_each_group_between_margins() {
        let ax = Figure::new().subplot();
        let options = GroupedBarOptions {
            group_spacing: 0.5,
            bar_spacing: 0.1,
            orientation: Orientation::Horizontal,
            ..GroupedBarOptions::default()
        };
        let data = GroupedBarData::Named(vec![
            ("a".into(), vec![3.0, 5.0]),
            ("b".into(), vec![4.0, 2.0]),
        ]);
        let bars = ax.grouped_bar(data, options).unwrap();
        let width = 1.0 / (2.0 + 0.1 + 0.5);
        let second = bars[1].patches[1].shape();
        assert!((second.height - width).abs() < 1e-15);
        let expected = 1.0 - 0.5 + 0.25 * width + (width + 0.1 * width);
        assert!((second.xy.y - expected).abs() < 1e-15);
        assert_eq!(bars[0].label.as_deref(), Some("a"));
    }

    #[test]
    fn every_dataset_starts_at_the_base() {
        let ax = Figure::new().subplot();
        let options = GroupedBarOptions {
            base: vec![5.0, 15.0],
            ..GroupedBarOptions::default()
        };
        let data = vec![vec![0.0, 0.0], vec![20.0, 17.0]];
        let bars = ax.grouped_bar(data, options).unwrap();
        for container in &bars {
            assert_eq!(container.bottoms(), [5.0, 15.0]);
        }
        assert_eq!(bars[1].tops(), [25.0, 32.0]);
    }

    #[test]
    fn tables_hold_one_dataset_per_column() {
        let table = Grid::from_vec(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        let sets = datasets(table.into(), &[]).unwrap();
        let values: Vec<Vec<f64>> = sets.iter().map(|d| d.values.clone()).collect();
        assert_eq!(values, vec![vec![1.0, 4.0], vec![2.0, 5.0], vec![3.0, 6.0]]);
        assert!(sets.iter().all(|d| d.label.is_none()));
    }

    #[test]
    fn invalid_input_is_rejected() {
        let ragged = GroupedBarData::Datasets(vec![vec![1.0, 2.0], vec![1.0]]);
        assert_eq!(
            datasets(ragged, &[]).err(),
            Some(GroupedBarError::Ragged {
                dataset: 1,
                len: 1,
                expected: 2
            })
        );
        let named = GroupedBarData::Named(vec![("a".into(), vec![1.0])]);
        assert_eq!(
            datasets(named, &["x".into()]).err(),
            Some(GroupedBarError::LabelsWithNames)
        );
        assert_eq!(
            datasets(GroupedBarData::Datasets(Vec::new()), &[]).err(),
            Some(GroupedBarError::NoDatasets)
        );
        assert_eq!(
            group_layout(Some(&[0.0, 1.0, 3.0]), 3).err(),
            Some(GroupedBarError::UnevenPositions)
        );
        assert_eq!(
            group_layout(Some(&[1.0, 3.0, 5.0]), 3),
            Ok((vec![1.0, 3.0, 5.0], 2.0))
        );
    }
}
