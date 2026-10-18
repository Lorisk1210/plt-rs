//! The label of an axis: its text, font and placement (matplotlib's
//! `Axis.label`).

use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::spines::SpineSide;

/// Where an axis label sits along its axis (matplotlib's `loc` of
/// `set_xlabel`/`set_ylabel`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelLoc {
    /// At the left (x) or bottom (y) end, aligned to it.
    Start,
    /// Centred.
    #[default]
    Center,
    /// At the right (x) or top (y) end, aligned to it.
    End,
}

impl LabelLoc {
    /// The rc name (`"left"`, `"bottom"`, `"center"`, ...) for axis `name`.
    pub(crate) fn from_rc(name: &str) -> Self {
        match name {
            "left" | "bottom" => Self::Start,
            "right" | "top" => Self::End,
            _ => Self::Center,
        }
    }

    /// The position along the axis as a fraction of its length.
    pub(crate) fn fraction(self) -> f64 {
        match self {
            Self::Start => 0.0,
            Self::Center => 0.5,
            Self::End => 1.0,
        }
    }
}

/// The label of an axis (matplotlib's `Axis.label`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisLabel {
    /// Label text; empty labels are not drawn.
    pub(crate) text: String,
    /// Font (`axes.labelsize`, `axes.labelweight`).
    pub(crate) font: FontProperties,
    /// Colour (`axes.labelcolor`).
    pub(crate) color: Color,
    /// Gap to the tick labels in points (`axes.labelpad`).
    pub(crate) pad: f64,
    /// The side of the axes the label sits on.
    pub(crate) side: SpineSide,
    /// Where along the axis the label sits.
    pub(crate) loc: LabelLoc,
    /// Whether the label is drawn (matplotlib's `label.set_visible`).
    pub(crate) visible: bool,
}

/// The font of axis labels: `axes.labelsize` and `axes.labelweight`
/// (matplotlib's `Axis.__init__`).
pub(crate) fn label_font(rc: &RcParams) -> FontProperties {
    FontProperties::from_rc(rc)
        .with_size(rc.axes_labelsize, rc.font_size)
        .with_weight(rc.axes_labelweight)
}
