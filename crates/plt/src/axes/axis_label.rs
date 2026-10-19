//! The label of an axis: its text, font and placement (matplotlib's
//! `Axis.label`).

use plt_text::{HAlign, VAlign};

use super::axis_handle::Axis;
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

    /// The label's alignment along the axis: the text's horizontal
    /// alignment, before any rotation.
    pub(crate) fn halign(self) -> HAlign {
        match self {
            Self::Start => HAlign::Left,
            Self::Center => HAlign::Center,
            Self::End => HAlign::Right,
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
    /// The alignment of the lines of a multi-line label (`None` follows
    /// the alignment along the axis, as matplotlib's `multialignment`).
    pub(crate) multialign: Option<HAlign>,
    /// Whether the label is drawn (matplotlib's `label.set_visible`).
    pub(crate) visible: bool,
    /// Rotation in degrees instead of the axis default (0 for x, 90 for y).
    pub(crate) rotation: Option<f64>,
    /// Vertical alignment instead of the one that comes with the side.
    pub(crate) valign: Option<VAlign>,
}

impl AxisLabel {
    /// Moves the label to `side`, which brings back the vertical alignment
    /// of that side (matplotlib's `set_label_position`).
    pub(crate) fn move_to(&mut self, side: SpineSide) {
        self.side = side;
        self.valign = None;
    }
}

/// The font of axis labels: `axes.labelsize` and `axes.labelweight`
/// (matplotlib's `Axis.__init__`).
pub(crate) fn label_font(rc: &RcParams) -> FontProperties {
    FontProperties::from_rc(rc)
        .with_size(rc.axes_labelsize, rc.font_size)
        .with_weight(rc.axes_labelweight)
}

impl Axis {
    /// Sets the font families of the axis label (matplotlib's
    /// `set_xlabel(..., family=...)`).
    pub fn set_label_family<I: IntoIterator<Item = S>, S: Into<String>>(
        &self,
        families: I,
    ) -> &Self {
        let families: Vec<String> = families.into_iter().map(Into::into).collect();
        self.update(|axis| axis.label.font.families = families);
        self
    }

    /// Aligns the lines of a multi-line axis label (matplotlib's
    /// `set_xlabel(..., multialignment=...)`); `None` aligns them like the
    /// label itself.
    pub fn set_label_multialign(&self, align: Option<HAlign>) -> &Self {
        self.update(|axis| axis.label.multialign = align);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::AxesState;
    use crate::figure::Figure;

    #[test]
    fn label_family_and_multialign_change_only_that_label() {
        let ax = Figure::new().subplot();
        ax.xaxis()
            .set_label_family(["serif"])
            .set_label_multialign(Some(HAlign::Left));
        let (x, y) = ax
            .raw()
            .with::<AxesState, _>(|axes| (axes.xaxis.label.clone(), axes.yaxis.label.clone()));
        assert_eq!(x.font.families, ["serif"]);
        assert_eq!(x.multialign, Some(HAlign::Left));
        assert_ne!(y.font.families, ["serif"]);
        assert_eq!(y.multialign, None);
    }

    #[test]
    fn labels_align_along_their_axis() {
        assert_eq!(LabelLoc::Start.halign(), HAlign::Left);
        assert_eq!(LabelLoc::Center.halign(), HAlign::Center);
        assert_eq!(LabelLoc::End.halign(), HAlign::Right);
    }
}
