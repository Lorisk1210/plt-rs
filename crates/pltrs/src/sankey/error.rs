//! Why a Sankey diagram cannot be drawn.

use std::fmt;

use crate::axes::PlotError;
use crate::ticker::pyfmt::FormatError;

/// Invalid Sankey settings or subdiagrams.
#[derive(Clone, Debug, PartialEq)]
pub enum SankeyError {
    /// A negative gap, which would make paths overlap.
    NegativeGap,
    /// A radius larger than the gap, which would make paths overlap.
    RadiusAboveGap,
    /// A negative head angle, which would make inputs look like outputs.
    NegativeHeadAngle,
    /// A negative tolerance.
    NegativeTolerance,
    /// A negative trunk length.
    NegativeTrunk,
    /// A per-flow argument with neither one value nor one per flow.
    Count {
        /// The argument.
        argument: &'static str,
        /// Values given.
        len: usize,
        /// Number of flows.
        flows: usize,
    },
    /// A prior diagram that does not exist.
    UnknownPrior {
        /// The requested diagram.
        prior: usize,
        /// Diagrams so far.
        diagrams: usize,
    },
    /// A connection to a flow that does not exist.
    UnknownFlow {
        /// The requested flow.
        flow: usize,
        /// Flows of that diagram.
        flows: usize,
    },
    /// A connection to a flow too small to be drawn.
    HiddenFlow(usize),
    /// Connected flows that do not cancel out.
    Unbalanced {
        /// Their sum.
        error: f64,
        /// The tolerance.
        tolerance: f64,
    },
    /// The quantity format accepts no number.
    Format(FormatError),
    /// The view cannot fit the diagram: no subdiagram was added, or the
    /// margin is not finite.
    Limits(PlotError),
}

impl fmt::Display for SankeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeGap => f.write_str(
                "'gap' is negative, which is not allowed because it would cause the paths \
                 to overlap",
            ),
            Self::RadiusAboveGap => f.write_str(
                "'radius' is greater than 'gap', which is not allowed because it would cause \
                 the paths to overlap",
            ),
            Self::NegativeHeadAngle => f.write_str(
                "'head_angle' is negative, which is not allowed because it would cause inputs \
                 to look like outputs and vice versa",
            ),
            Self::NegativeTolerance => {
                f.write_str("'tolerance' is negative, but it must be a magnitude")
            }
            Self::NegativeTrunk => f.write_str(
                "'trunklength' is negative, which is not allowed because it would cause poor \
                 layout",
            ),
            Self::Count {
                argument,
                len,
                flows,
            } => write!(
                f,
                "the lengths of 'flows' ({flows}) and '{argument}' ({len}) are incompatible"
            ),
            Self::UnknownPrior { prior, diagrams } => write!(
                f,
                "the index of the prior diagram is {prior}, but there are only {diagrams} \
                 other diagrams"
            ),
            Self::UnknownFlow { flow, flows } => write!(
                f,
                "the connection index is {flow}, but that diagram has only {flows} flows"
            ),
            Self::HiddenFlow(flow) => write!(
                f,
                "the connection cannot be made: flow {flow} is smaller than the tolerance"
            ),
            Self::Unbalanced { error, tolerance } => write!(
                f,
                "the scaled sum of the connected flows is {error}, which is not within the \
                 tolerance ({tolerance})"
            ),
            Self::Format(err) => err.fmt(f),
            Self::Limits(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for SankeyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Format(err) => Some(err),
            Self::Limits(err) => Some(err),
            _ => None,
        }
    }
}
