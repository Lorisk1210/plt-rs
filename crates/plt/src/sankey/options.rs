//! The settings, subdiagram descriptions and results of a Sankey diagram.

use plt_core::Point;

use crate::patches::{Patch, PatchProps, PathShape};
use crate::text::Text;
use crate::ticker::LabelFormat;

/// Settings shared by every subdiagram of a [`Sankey`](super::Sankey) (matplotlib's
/// `Sankey(...)` arguments).
#[derive(Clone, Debug)]
pub struct SankeyOptions {
    /// Factor from flow quantities to widths in data units (default 1).
    pub scale: f64,
    /// Unit appended to the quantities; `None` hides the quantities
    /// (default `""`).
    pub unit: Option<String>,
    /// How quantities are written: a printf-style template applied to the
    /// flow's magnitude (default `"%G"`), or a function of the signed
    /// flow (matplotlib's `format`).
    pub format: LabelFormat,
    /// Space between paths breaking in or away at the top or bottom
    /// (default 0.25).
    pub gap: f64,
    /// Inner radius of the vertical paths (default 0.1).
    pub radius: f64,
    /// Size of the shoulders of output arrows (default 0.03).
    pub shoulder: f64,
    /// Distance of the labels from the dips and tips (default 0.15).
    pub offset: f64,
    /// Angle of the arrow heads in degrees (default 100).
    pub head_angle: f64,
    /// Minimum space between the outlines and the axes edges (default
    /// 0.4).
    pub margin: f64,
    /// Flows smaller than this are not drawn; connected flows must cancel
    /// to within it (default 1e-6).
    pub tolerance: f64,
}

impl Default for SankeyOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            unit: Some(String::new()),
            format: LabelFormat::from("%G"),
            gap: 0.25,
            radius: 0.1,
            shoulder: 0.03,
            offset: 0.15,
            head_angle: 100.0,
            margin: 0.4,
            tolerance: 1e-6,
        }
    }
}

/// The side a flow enters or leaves its trunk by (matplotlib's
/// orientations `0`, `1` and `-1`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FlowOrientation {
    /// Inputs from the left, outputs to the right (`0`).
    #[default]
    Side,
    /// From and to the top (`1`).
    Top,
    /// From and to the bottom (`-1`).
    Bottom,
}

/// The lengths of the vertical arrows before they join the trunk
/// (matplotlib's `pathlengths`).
#[derive(Clone, Debug, PartialEq)]
pub enum PathLengths {
    /// The length of the innermost arrows on the top and bottom; outer
    /// ones grow to stay clear of them, horizontal ones get none.
    Inner(f64),
    /// One length per flow.
    PerFlow(Vec<f64>),
}

impl Default for PathLengths {
    fn default() -> Self {
        Self::Inner(0.25)
    }
}

/// One subdiagram (matplotlib's `Sankey.add` arguments).
#[derive(Clone, Debug)]
pub struct Subdiagram {
    /// Text at the centre of the trunk.
    pub patchlabel: String,
    /// The flows: inputs positive, outputs negative (default `[1, -1]`).
    pub flows: Vec<f64>,
    /// The side of each flow (one for all, or one per flow).
    pub orientations: Vec<FlowOrientation>,
    /// The label of each flow (one for all, or one per flow); `None` shows
    /// nothing, a string shows the quantity below it.
    pub labels: Vec<Option<String>>,
    /// Distance between the bases of the inputs and outputs (default 1).
    pub trunklength: f64,
    /// The lengths of the vertical arrows.
    pub pathlengths: PathLengths,
    /// The earlier subdiagram this one connects to.
    pub prior: Option<usize>,
    /// The flow of the prior subdiagram and the flow of this one that
    /// meet (ignored without `prior`).
    pub connect: (usize, usize),
    /// Rotation in degrees (ignored when connected).
    pub rotation: f64,
    /// Style of the outline; the face colour defaults to the next colour
    /// of the property cycle.
    pub props: PatchProps,
}

impl Default for Subdiagram {
    fn default() -> Self {
        Self {
            patchlabel: String::new(),
            flows: vec![1.0, -1.0],
            orientations: vec![FlowOrientation::Side],
            labels: vec![Some(String::new())],
            trunklength: 1.0,
            pathlengths: PathLengths::default(),
            prior: None,
            connect: (0, 0),
            rotation: 0.0,
            props: PatchProps::default(),
        }
    }
}

/// A drawn subdiagram (one entry of matplotlib's `Sankey.finish()`).
#[derive(Clone, Debug)]
pub struct SankeyDiagram {
    /// The outline.
    pub patch: Patch<PathShape>,
    /// The flows as given.
    pub flows: Vec<f64>,
    /// The direction of each arrow in quarter turns (`None` for flows too
    /// small to draw).
    pub angles: Vec<Option<f64>>,
    /// The tip (or dip) of each flow; the trunk centre for hidden flows.
    pub tips: Vec<Point>,
    /// The text at the trunk centre.
    pub text: Text,
    /// The label of each flow.
    pub texts: Vec<Text>,
}
