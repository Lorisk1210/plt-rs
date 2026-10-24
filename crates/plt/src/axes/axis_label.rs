//! The label of an axis: its text, font and placement (matplotlib's
//! `Axis.label`, `set_label_text`, `set_label_position` and
//! `set_label_coords`).

use std::rc::Rc;

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::axis_handle::Axis;
use super::axis_layout::PlacedText;
use super::{AxesState, PlotError};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize, FontStyle, FontWeight};
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::text::{TextBox, TextBoxProps};
use crate::ticker::AxisName;
use crate::transforms::Bbox;

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
    /// A fixed anchor in axes fractions instead of the automatic place
    /// beyond the tick labels (matplotlib's `set_label_coords`).
    pub(crate) coords: Option<Point>,
    /// The box drawn behind the label (matplotlib's `label.set_bbox`).
    pub(crate) bbox: Option<TextBox>,
}

impl AxisLabel {
    /// Moves the label to `side`, which brings back the vertical alignment
    /// of that side (matplotlib's `set_label_position`).
    pub(crate) fn move_to(&mut self, side: SpineSide) {
        self.side = side;
        self.valign = None;
    }

    /// The rotation in degrees: the one set, else horizontal for x and
    /// vertical for y.
    pub(crate) fn rotation_on(&self, axis: AxisName) -> f64 {
        self.rotation.unwrap_or(match axis {
            AxisName::X => 0.0,
            AxisName::Y => 90.0,
        })
    }

    /// The vertical alignment: the one set, else that of the side (x
    /// labels hang below the axes and sit on their anchor above it, y
    /// labels sit on it left of the axes and hang on the right).
    pub(crate) fn valign_on(&self, axis: AxisName) -> VAlign {
        let high = self.side.is_high();
        self.valign.unwrap_or(match (axis, high) {
            (AxisName::X, false) | (AxisName::Y, true) => VAlign::Top,
            (AxisName::X, true) => VAlign::Baseline,
            (AxisName::Y, false) => VAlign::Bottom,
        })
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
    /// Sets the text of the axis label (matplotlib's `set_label_text`, as
    /// `set_xlabel` does).
    pub fn set_label_text(&self, text: impl Into<String>) -> &Self {
        let text = text.into();
        self.update(|axis| axis.label.text = text);
        self
    }

    /// The text of the axis label, empty when unset (matplotlib's
    /// `get_label_text`).
    pub fn label_text(&self) -> String {
        self.read(|axis| axis.label.text.clone())
    }

    /// Puts the axis label on `side` (matplotlib's `set_label_position`),
    /// with the vertical alignment of that side.
    ///
    /// # Errors
    ///
    /// When `side` is not a side of this axis (e.g. `Left` for x).
    pub fn set_label_position(&self, side: SpineSide) -> Result<&Self, PlotError> {
        self.check_side(side)?;
        self.update(|axis| axis.label.move_to(side));
        Ok(self)
    }

    /// The side the axis label sits on (matplotlib's
    /// `get_label_position`).
    pub fn label_position(&self) -> SpineSide {
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| axes.axis(self.name).label.side)
    }

    /// Rotates the axis label by `degrees` counter-clockwise, reduced to
    /// `[0, 360)` (matplotlib's `set_ylabel(..., rotation=...)`);
    /// unrotated, x labels are horizontal and y labels vertical.
    pub fn set_label_rotation(&self, degrees: f64) -> &Self {
        self.update(|axis| axis.label.rotation = Some(degrees.rem_euclid(360.0)));
        self
    }

    /// The rotation of the axis label in degrees: 0 for x and 90 for y
    /// unless set (matplotlib's `label.get_rotation()`).
    pub fn label_rotation(&self) -> f64 {
        self.read(|axis| axis.label.rotation_on(self.name))
    }

    /// Aligns the axis label vertically on its anchor (matplotlib's
    /// `set_ylabel(..., va=...)`) until the label moves to another side.
    /// By default x labels hang from their anchor below the axes (`Top`)
    /// and sit on it above (`Baseline`); y labels sit on it left of the
    /// axes (`Bottom`) and hang from it on the right (`Top`), in their
    /// rotated frame.
    pub fn set_label_valign(&self, valign: VAlign) -> &Self {
        self.update(|axis| axis.label.valign = Some(valign));
        self
    }

    /// The vertical alignment of the axis label: the one set, else that
    /// of its side (matplotlib's `label.get_verticalalignment()`).
    pub fn label_valign(&self) -> VAlign {
        self.read(|axis| axis.label.valign_on(self.name))
    }

    /// Colours the axis label (matplotlib's `set_xlabel(..., color=...)`).
    pub fn set_label_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|axis| axis.label.color = color);
        self
    }

    /// The colour of the axis label (matplotlib's `label.get_color()`).
    pub fn label_color(&self) -> Color {
        self.read(|axis| axis.label.color.clone())
    }

    /// Places the axis label along the axis (matplotlib's `loc` of
    /// `set_xlabel`).
    pub fn set_label_loc(&self, loc: LabelLoc) -> &Self {
        self.update(|axis| axis.label.loc = loc);
        self
    }

    /// Where along the axis its label sits.
    pub fn label_loc(&self) -> LabelLoc {
        self.read(|axis| axis.label.loc)
    }

    /// Sets the gap between the tick labels and the axis label in points
    /// (matplotlib's `labelpad`).
    pub fn set_label_pad(&self, pad: f64) -> &Self {
        self.update(|axis| axis.label.pad = pad);
        self
    }

    /// The gap between the tick labels and the axis label in points
    /// (matplotlib's `labelpad`).
    pub fn label_pad(&self) -> f64 {
        self.read(|axis| axis.label.pad)
    }

    /// Anchors the axis label at `(x, y)` in axes fractions instead of
    /// beyond the tick labels, keeping its alignment (matplotlib's
    /// `set_label_coords`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_ylabel("depth");
    /// ax.yaxis().set_label_coords(-0.3, 0.5);
    /// let label = ax.yaxis().label_extent().expect("a label");
    /// let (x0, _, width, _) = ax.position().bounds();
    /// let anchor = (x0 - 0.3 * width) * fig.size_pixels().0;
    /// assert!((label.x1() - anchor).abs() < 1e-9);
    /// ```
    pub fn set_label_coords(&self, x: f64, y: f64) -> &Self {
        self.update_labels(|axis| axis.label.coords = Some(Point::new(x, y)));
        self
    }

    /// The fixed anchor of the axis label in axes fractions, `None` while
    /// it goes beyond the tick labels.
    pub fn label_coords(&self) -> Option<Point> {
        self.read(|axis| axis.label.coords)
    }

    /// Draws a box behind the axis label, or none (matplotlib's
    /// `set_xlabel(..., bbox=...)`); its padding in font sizes is fixed
    /// for the label's current font size, as for any text.
    pub fn set_label_bbox(&self, props: Option<TextBoxProps>) -> &Self {
        self.axes.raw().with_mut::<AxesState, _>(|axes| {
            let rc = Rc::clone(&axes.rc);
            let label = &mut axes.axis_mut(self.name).label;
            label.bbox = props.map(|props| TextBox::new(props, label.font.size, &rc));
        });
        self
    }

    /// The box behind the axis label, if any (matplotlib's
    /// `label.get_bbox_patch()`).
    pub fn label_bbox(&self) -> Option<TextBox> {
        self.read(|axis| axis.label.bbox.clone())
    }

    /// The display box of the axis label in the figure's current layout
    /// (laid out first), if it is shown (matplotlib's
    /// `axis.label.get_window_extent()`).
    pub fn label_extent(&self) -> Option<Bbox> {
        self.with_texts(|texts, _| texts.label.as_ref().map(PlacedText::bbox))
            .flatten()
    }

    /// The display box of the offset (or scale) text in the figure's
    /// current layout (laid out first), if it is shown (matplotlib's
    /// `axis.offsetText.get_window_extent()`).
    pub fn offset_text_extent(&self) -> Option<Bbox> {
        self.with_texts(|texts, _| texts.offset.as_ref().map(PlacedText::bbox))
            .flatten()
    }

    /// Sets the font size of the axis label.
    pub fn set_label_fontsize(&self, size: FontSize) -> &Self {
        let base = crate::rc::params().font_size;
        self.update(|axis| axis.label.font = axis.label.font.clone().with_size(size, base));
        self
    }

    /// Sets the font weight of the axis label (matplotlib's
    /// `set_xlabel(..., fontweight=...)`).
    pub fn set_label_weight(&self, weight: FontWeight) -> &Self {
        self.update(|axis| axis.label.font.weight = weight);
        self
    }

    /// Sets the font style of the axis label (matplotlib's
    /// `set_xlabel(..., fontstyle=...)`).
    pub fn set_label_style(&self, style: FontStyle) -> &Self {
        self.update(|axis| axis.label.font.style = style);
        self
    }

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

    /// Sets every font property of the axis label at once, its size
    /// included (matplotlib's `set_xlabel(..., fontproperties=...)`).
    pub fn set_label_font(&self, font: FontProperties) -> &Self {
        self.update(|axis| axis.label.font = font);
        self
    }

    /// The font of the axis label: families, size, weight, style and the
    /// rest (matplotlib's `label.get_fontproperties()`).
    pub fn label_font(&self) -> FontProperties {
        self.read(|axis| axis.label.font.clone())
    }

    /// Aligns the lines of a multi-line axis label (matplotlib's
    /// `set_xlabel(..., multialignment=...)`); `None` aligns them like the
    /// label itself.
    pub fn set_label_multialign(&self, align: Option<HAlign>) -> &Self {
        self.update(|axis| axis.label.multialign = align);
        self
    }

    /// The alignment of the lines of a multi-line axis label, `None` when
    /// they align like the label.
    pub fn label_multialign(&self) -> Option<HAlign> {
        self.read(|axis| axis.label.multialign)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::patches::{BoxShape, BoxStyle};

    #[test]
    fn label_family_and_multialign_change_only_that_label() {
        let ax = Figure::new().subplot();
        ax.xaxis()
            .set_label_family(["serif"])
            .set_label_multialign(Some(HAlign::Left));
        assert_eq!(ax.xaxis().label_font().families, ["serif"]);
        assert_eq!(ax.xaxis().label_multialign(), Some(HAlign::Left));
        assert_ne!(ax.yaxis().label_font().families, ["serif"]);
        assert_eq!(ax.yaxis().label_multialign(), None);
    }

    #[test]
    fn labels_align_along_their_axis() {
        assert_eq!(LabelLoc::Start.halign(), HAlign::Left);
        assert_eq!(LabelLoc::Center.halign(), HAlign::Center);
        assert_eq!(LabelLoc::End.halign(), HAlign::Right);
    }

    #[test]
    fn label_weight_changes_only_that_axis_label() {
        let ax = Figure::new().subplot();
        ax.yaxis().set_label_weight(FontWeight::BOLD);
        assert_eq!(
            (
                ax.xaxis().label_font().weight,
                ax.yaxis().label_font().weight
            ),
            (FontWeight::NORMAL, FontWeight::BOLD)
        );
    }

    #[test]
    fn label_font_replaces_every_property_of_that_axis_label() {
        let ax = Figure::new().subplot();
        let font = FontProperties {
            families: vec!["Times New Roman".into()],
            style: FontStyle::Italic,
            ..FontProperties::from_rc(&crate::rc::params())
        };
        ax.xaxis().set_label_fontsize(FontSize::Points(20.0));
        ax.yaxis().set_label_font(font.clone());
        assert_eq!(ax.xaxis().label_font().size, 20.0);
        assert_eq!(ax.yaxis().label_font(), font);
    }

    #[test]
    fn label_rotations_are_reduced_to_a_turn() {
        let ax = Figure::new().subplot();
        assert_eq!(
            (ax.xaxis().label_rotation(), ax.yaxis().label_rotation()),
            (0.0, 90.0)
        );
        ax.yaxis().set_label_rotation(-90.0);
        assert_eq!(ax.yaxis().label_rotation(), 270.0);
        ax.yaxis().set_label_rotation(450.0);
        assert_eq!(ax.yaxis().label_rotation(), 90.0);
    }

    #[test]
    fn label_alignment_follows_the_side_until_set() {
        let ax = Figure::new().subplot();
        let (xaxis, yaxis) = (ax.xaxis(), ax.yaxis());
        assert_eq!(
            (xaxis.label_valign(), yaxis.label_valign()),
            (VAlign::Top, VAlign::Bottom)
        );
        xaxis.set_label_position(SpineSide::Top).expect("an x side");
        yaxis
            .set_label_position(SpineSide::Right)
            .expect("a y side");
        assert_eq!(
            (xaxis.label_valign(), yaxis.label_valign()),
            (VAlign::Baseline, VAlign::Top)
        );
        yaxis.set_label_valign(VAlign::Center);
        assert_eq!(yaxis.label_valign(), VAlign::Center);
        assert_eq!(yaxis.label_position(), SpineSide::Right);
        assert!(xaxis.set_label_position(SpineSide::Left).is_err());
    }

    #[test]
    fn label_properties_read_back_as_set() {
        let ax = Figure::new().subplot();
        let xaxis = ax.xaxis();
        assert_eq!(xaxis.label_text(), "");
        assert_eq!(xaxis.label_coords(), None);
        assert!(xaxis.label_bbox().is_none());
        xaxis
            .set_label_text("time")
            .set_label_color("red")
            .set_label_pad(7.5)
            .set_label_coords(0.5, -0.2)
            .set_label_bbox(Some(TextBoxProps::new(BoxStyle::new(BoxShape::Square))));
        assert_eq!(xaxis.label_text(), "time");
        assert_eq!(ax.label(AxisName::X), "time", "what set_xlabel sets");
        assert_eq!(xaxis.label_color(), Color::from("red"));
        assert_eq!(xaxis.label_pad(), 7.5);
        assert_eq!(xaxis.label_coords(), Some(Point::new(0.5, -0.2)));
        assert!(xaxis.label_bbox().is_some());
    }
}
