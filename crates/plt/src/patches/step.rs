//! Stepwise constant outlines (matplotlib's `StepPatch`, drawn by `stairs`).

use std::fmt;

use plt_core::{Path, PathBuilder, Point};

use super::PatchShape;
use crate::orientation::Orientation;

/// Where the vertical edges of a [`StepPatch`] end.
#[derive(Clone, Debug, PartialEq)]
pub enum StepBaseline {
    /// Open steps: the outline starts and ends at the first and last value.
    None,
    /// One baseline value for every step (default 0).
    Value(f64),
    /// One baseline value per step; the outline is closed along them.
    PerStep(Vec<f64>),
}

/// Why a [`StepPatch`] cannot be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepPatchError {
    /// `edges` must have one entry more than `values`.
    EdgeCount {
        /// Number of edges.
        edges: usize,
        /// Number of values.
        values: usize,
    },
    /// Edges must not be NaN.
    NanEdge,
    /// A per-step baseline must have one entry per value.
    BaselineCount {
        /// Number of baseline values.
        baseline: usize,
        /// Number of values.
        values: usize,
    },
}

impl fmt::Display for StepPatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EdgeCount { edges, values } => write!(
                f,
                "size mismatch between \"values\" and \"edges\": expected `len(values) + 1 == \
                 len(edges)`, but `len(values) = {values}` and `len(edges) = {edges}`"
            ),
            Self::NanEdge => f.write_str("Nan values in \"edges\" are disallowed"),
            Self::BaselineCount { baseline, values } => {
                write!(f, "{baseline} baseline values do not match {values} steps")
            }
        }
    }
}

impl Default for StepBaseline {
    fn default() -> Self {
        Self::Value(0.0)
    }
}

impl std::error::Error for StepPatchError {}

/// A stepwise constant function between `edges`, as a patch outline.
#[derive(Clone, Debug, PartialEq)]
pub struct StepPatch {
    values: Vec<f64>,
    edges: Vec<f64>,
    baseline: StepBaseline,
    orientation: Orientation,
}

impl StepPatch {
    /// Steps of height `values[i]` between `edges[i]` and `edges[i + 1]`,
    /// with a baseline at 0.
    ///
    /// # Errors
    ///
    /// When the edge count is not `values.len() + 1` or an edge is NaN.
    pub fn new(values: Vec<f64>, edges: Vec<f64>) -> Result<Self, StepPatchError> {
        if edges.len() != values.len() + 1 {
            return Err(StepPatchError::EdgeCount {
                edges: edges.len(),
                values: values.len(),
            });
        }
        if edges.iter().any(|e| e.is_nan()) {
            return Err(StepPatchError::NanEdge);
        }
        Ok(Self {
            values,
            edges,
            baseline: StepBaseline::default(),
            orientation: Orientation::Vertical,
        })
    }

    /// The same steps with another baseline.
    ///
    /// # Errors
    ///
    /// When a per-step baseline has the wrong length.
    pub fn with_baseline(self, baseline: StepBaseline) -> Result<Self, StepPatchError> {
        if let StepBaseline::PerStep(base) = &baseline
            && base.len() != self.values.len()
        {
            return Err(StepPatchError::BaselineCount {
                baseline: base.len(),
                values: self.values.len(),
            });
        }
        Ok(Self { baseline, ..self })
    }

    /// The same steps growing along x (`Horizontal`) or y (`Vertical`).
    #[must_use]
    pub fn with_orientation(self, orientation: Orientation) -> Self {
        Self {
            orientation,
            ..self
        }
    }

    /// The step heights.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// The step boundaries.
    pub fn edges(&self) -> &[f64] {
        &self.edges
    }

    /// The baseline.
    pub fn baseline(&self) -> &StepBaseline {
        &self.baseline
    }

    /// The direction the values grow in.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// The outline of the steps `start..end` as `(position, value)` pairs.
    fn run_outline(&self, start: usize, end: usize) -> Vec<(f64, f64)> {
        let doubled =
            |values: &[f64]| -> Vec<f64> { values.iter().flat_map(|&v| [v, v]).collect() };
        let positions = doubled(&self.edges[start..=end]);
        let heights = doubled(&self.values[start..end]);
        let (first, last) = (heights[0], heights[heights.len() - 1]);
        match &self.baseline {
            StepBaseline::None => {
                let values = [vec![first], heights, vec![last]].concat();
                positions.into_iter().zip(values).collect()
            }
            &StepBaseline::Value(base) => {
                let values = [vec![base], heights, vec![base]].concat();
                positions.into_iter().zip(values).collect()
            }
            StepBaseline::PerStep(base) => {
                let mut back = doubled(&base[start..end]);
                back.reverse();
                let (base_first, base_last) = (back[0], back[back.len() - 1]);
                let mut around = positions.clone();
                around.extend(positions.iter().rev());
                let values = [
                    vec![base_last],
                    heights,
                    vec![base_first, base_first],
                    back,
                    vec![base_last],
                ]
                .concat();
                around.into_iter().zip(values).collect()
            }
        }
    }

    /// Maximal runs of steps whose value (and per-step baseline) is not NaN.
    fn finite_runs(&self) -> Vec<(usize, usize)> {
        let valid = |i: usize| {
            !self.values[i].is_nan()
                && match &self.baseline {
                    StepBaseline::Value(base) => !base.is_nan(),
                    StepBaseline::PerStep(base) => !base[i].is_nan(),
                    StepBaseline::None => true,
                }
        };
        let mut runs = Vec::new();
        let mut start = None;
        for i in 0..=self.values.len() {
            match (start, i < self.values.len() && valid(i)) {
                (None, true) => start = Some(i),
                (Some(s), false) => {
                    runs.push((s, i));
                    start = None;
                }
                _ => {}
            }
        }
        runs
    }
}

impl PatchShape for StepPatch {
    fn path(&self) -> Path {
        let mut builder = Path::builder();
        for (start, end) in self.finite_runs() {
            let mut points = self
                .run_outline(start, end)
                .into_iter()
                .map(|(position, value)| {
                    let (x, y) = self.orientation.place(position, value);
                    Point::new(x, y)
                });
            if let Some(first) = points.next() {
                builder = builder.move_to(first);
            }
            builder = points.fold(builder, PathBuilder::line_to);
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xy(patch: &StepPatch) -> Vec<(f64, f64)> {
        patch.path().vertices().iter().map(|p| (p.x, p.y)).collect()
    }

    #[test]
    fn outline_drops_to_the_baseline() {
        let patch = StepPatch::new(vec![1.0, 3.0], vec![0.0, 1.0, 2.0]).unwrap();
        assert_eq!(
            xy(&patch),
            [
                (0.0, 0.0),
                (0.0, 1.0),
                (1.0, 1.0),
                (1.0, 3.0),
                (2.0, 3.0),
                (2.0, 0.0)
            ]
        );
        let open = patch.clone().with_baseline(StepBaseline::None).unwrap();
        assert_eq!(xy(&open)[0], (0.0, 1.0));
        let sideways = patch.with_orientation(Orientation::Horizontal);
        assert_eq!(xy(&sideways)[1], (1.0, 0.0));
    }

    #[test]
    fn nan_values_split_the_outline() {
        let patch = StepPatch::new(vec![1.0, f64::NAN, 2.0], vec![0.0, 1.0, 2.0, 3.0]).unwrap();
        let path = patch.path();
        assert_eq!(path.vertices().len(), 8);
        assert_eq!(path.subpaths().len(), 2);
    }

    #[test]
    fn per_step_baseline_closes_the_outline() {
        let patch = StepPatch::new(vec![2.0, 3.0], vec![0.0, 1.0, 2.0])
            .unwrap()
            .with_baseline(StepBaseline::PerStep(vec![1.0, 0.5]))
            .unwrap();
        let points = xy(&patch);
        assert_eq!(points.len(), 12);
        assert_eq!(points[0], (0.0, 1.0));
        assert_eq!(points[5], (2.0, 0.5));
    }

    #[test]
    fn validates_shapes() {
        assert!(StepPatch::new(vec![1.0], vec![0.0]).is_err());
        assert_eq!(
            StepPatch::new(vec![1.0], vec![0.0, f64::NAN]),
            Err(StepPatchError::NanEdge)
        );
        let patch = StepPatch::new(vec![1.0], vec![0.0, 1.0]).unwrap();
        assert!(patch.with_baseline(StepBaseline::PerStep(vec![])).is_err());
    }
}
