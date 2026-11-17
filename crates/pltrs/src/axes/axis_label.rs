//! The label of an axis: its text, font and placement (matplotlib's
//! `Axis.label`, `set_label_text`, `set_label_position` and
//! `set_label_coords`).

use std::rc::Rc;

use pltrs_core::Point;
use pltrs_text::{HAlign, VAlign};

use super::axis_handle::Axis;
use super::label_loc::LabelLoc;
use super::{AxesState, PlotError};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize, FontStyle, FontWeight};
use crate::patheffects::PathEffect;
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::text::{TextBoxPatch, TextBoxProps};
use crate::ticker::AxisName;
use crate::transforms::Bbox;

/// The label of an axis (matplotlib's `Axis.label`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisLabel {
    /// Label text; empty labels are not drawn.
    pub(crate) text: String,
    /// Whether the text was not set, so that a unit converter may label
    /// the axis (matplotlib's `isDefault_label`).
    pub(crate) is_default: bool,
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
    /// A place along the axis in axes fractions instead of that of `loc`,
    /// keeping its alignment (matplotlib's `label.set_x` of an x axis,
    /// `set_y` of a y axis), until `loc` is set again.
    pub(crate) along: Option<f64>,
    /// The alignment along the axis instead of the one that comes with
    /// `loc` (matplotlib's `ha`).
    pub(crate) halign: Option<HAlign>,
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
    pub(crate) bbox: Option<TextBoxPatch>,
    /// Whether LaTeX typesets the label (matplotlib's `label.set_usetex`;
    /// `text.usetex` when the axis was made).
    pub(crate) usetex: bool,
    /// The path effects the label is drawn through in place of
    /// `path.effects` (matplotlib's `label.set_path_effects`).
    pub(crate) path_effects: Option<Vec<PathEffect>>,
}

pltrs_core::archive_struct!(AxisLabel {
    text,
    is_default,
    font,
    color,
    pad,
    side,
    loc,
    along,
    halign,
    multialign,
    visible,
    rotation,
    valign,
    coords,
    bbox,
    usetex,
    path_effects
});

impl AxisLabel {
    /// Sets the text, which unit converters then leave alone
    /// (matplotlib's `set_label_text`).
    pub(crate) fn set_text(&mut self, text: String) {
        self.text = text;
        self.is_default = false;
    }

    /// Moves the label to `side`, which brings back the vertical alignment
    /// of that side (matplotlib's `set_label_position`).
    pub(crate) fn move_to(&mut self, side: SpineSide) {
        self.side = side;
        self.valign = None;
    }

    /// Where along the axis the label sits, as a fraction of its length:
    /// the place set, else that of `loc`.
    pub(crate) fn along(&self) -> f64 {
        self.along.unwrap_or(self.loc.fraction())
    }

    /// The alignment along the axis, the text's horizontal alignment
    /// before any rotation: the one set, else that of `loc`.
    pub(crate) fn halign(&self) -> HAlign {
        self.halign.unwrap_or(self.loc.halign())
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
        self.update(|axis| axis.label.set_text(text));
        self
    }

    /// The text of the axis label, empty when unset (matplotlib's
    /// `get_label_text`).
    pub fn label_text(&self) -> String {
        self.read(|axis| axis.label.text.clone())
    }

    /// Whether the label text was never set, so that a unit converter's
    /// label replaces it (matplotlib's `isDefault_label`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.xaxis().is_default_label());
    /// ax.xaxis().set_label_text("time");
    /// assert!(!ax.xaxis().is_default_label());
    /// ```
    pub fn is_default_label(&self) -> bool {
        self.read(|axis| axis.label.is_default)
    }

    /// Says whether the label text counts as never set, which a unit
    /// converter's label then replaces (matplotlib's `isDefault_label`
    /// assigned).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.xaxis().set_default_label(false);
    /// assert!(!ax.xaxis().is_default_label());
    /// ```
    pub fn set_default_label(&self, default: bool) -> &Self {
        self.update(|axis| axis.label.is_default = default);
        self
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

    /// Has LaTeX typeset the axis label, or not (matplotlib's
    /// `label.set_usetex`).
    pub fn set_label_usetex(&self, usetex: bool) -> &Self {
        self.update(|axis| axis.label.usetex = usetex);
        self
    }

    /// Draws the axis label through `effects` in place of those of
    /// `path.effects` (matplotlib's `label.set_path_effects`); `None`
    /// draws it through the rcParam's again.
    ///
    /// ```
    /// use pltrs::patheffects::{GcUpdate, PathEffect, Stroke};
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let white = GcUpdate::default().linewidth(4.0).foreground("w");
    /// let halo = vec![PathEffect::with_stroke(Stroke::new(white))];
    /// ax.xaxis().set_label_text("depth").set_label_path_effects(Some(halo.clone()));
    /// assert_eq!(ax.xaxis().label_path_effects(), Some(halo));
    /// ```
    pub fn set_label_path_effects(&self, effects: Option<Vec<PathEffect>>) -> &Self {
        self.update(|axis| axis.label.path_effects = effects);
        self
    }

    /// The path effects the axis label has of its own
    /// ([`set_label_path_effects`](Self::set_label_path_effects)).
    pub fn label_path_effects(&self) -> Option<Vec<PathEffect>> {
        self.read(|axis| axis.label.path_effects.clone())
    }

    /// Whether LaTeX typesets the axis label (matplotlib's
    /// `label.get_usetex()`): `text.usetex` when the axes was made, until
    /// [`set_label_usetex`](Self::set_label_usetex).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(!ax.xaxis().label_usetex());
    /// assert!(ax.xaxis().set_label_usetex(true).label_usetex());
    /// ```
    pub fn label_usetex(&self) -> bool {
        self.read(|axis| axis.label.usetex)
    }

    /// Places the axis label along the axis, aligned to that place
    /// (matplotlib's `loc` of `set_xlabel`, which sets the position and
    /// the horizontal alignment together); an alignment set with
    /// [`Axis::set_label_halign`] gives way to that of `loc`. The place is
    /// a [`LabelLoc`], or matplotlib's name for it on an x or a y axis
    /// ([`XLabelLoc`](super::XLabelLoc), [`YLabelLoc`](super::YLabelLoc)).
    ///
    /// ```
    /// use pltrs::axes::{LabelLoc, XLabelLoc};
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.xaxis().set_label_loc(XLabelLoc::Right);
    /// assert_eq!(ax.xaxis().label_loc(), LabelLoc::End);
    /// ```
    pub fn set_label_loc(&self, loc: impl Into<LabelLoc>) -> &Self {
        let loc = loc.into();
        self.update(|axis| {
            axis.label.loc = loc;
            axis.label.along = None;
            axis.label.halign = None;
        });
        self
    }

    /// Where along the axis its label sits.
    pub fn label_loc(&self) -> LabelLoc {
        self.read(|axis| axis.label.loc)
    }

    /// Places the axis label at `fraction` of the axis' length, keeping
    /// its alignment (matplotlib's `label.set_x(fraction)` of an x axis,
    /// `set_y` of a y axis), until the next [`Axis::set_label_loc`].
    ///
    /// ```
    /// use pltrs::axes::LabelLoc;
    /// use pltrs::prelude::*;
    /// use pltrs::text::HAlign;
    /// let ax = Figure::new().subplot();
    /// ax.xaxis().set_label_loc(LabelLoc::End).set_label_along(0.9);
    /// assert_eq!(ax.xaxis().label_along(), 0.9);
    /// assert_eq!(ax.xaxis().label_halign(), HAlign::Right);
    /// ax.xaxis().set_label_loc(LabelLoc::Center);
    /// assert_eq!(ax.xaxis().label_along(), 0.5);
    /// ```
    pub fn set_label_along(&self, fraction: f64) -> &Self {
        self.update(|axis| axis.label.along = Some(fraction));
        self
    }

    /// Where along the axis its label sits, as a fraction of the axis'
    /// length: the place [`Axis::set_label_along`] set, else that of its
    /// [`LabelLoc`].
    pub fn label_along(&self) -> f64 {
        self.read(|axis| axis.label.along())
    }

    /// Aligns the axis label along its axis independently of where it sits
    /// (matplotlib's `set_ylabel(..., y=..., ha=...)`, with the position
    /// given by [`Axis::set_label_loc`] or [`Axis::set_label_coords`]):
    /// the text's horizontal alignment before any rotation, so a y label
    /// aligned `Left` starts at its anchor and runs upwards. The
    /// alignment stays when the label moves to the other side, until the
    /// next [`Axis::set_label_loc`].
    ///
    /// ```
    /// use pltrs::axes::LabelLoc;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// use pltrs::text::HAlign;
    /// let ax = Figure::new().subplot();
    /// ax.set_ylabel("test");
    /// let yaxis = ax.yaxis();
    /// yaxis.set_label_loc(LabelLoc::End).set_label_halign(HAlign::Left);
    /// yaxis.set_label_position(SpineSide::Right)?;
    /// assert_eq!(yaxis.label_halign(), HAlign::Left);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_label_halign(&self, halign: HAlign) -> &Self {
        self.update(|axis| axis.label.halign = Some(halign));
        self
    }

    /// The horizontal alignment of the axis label: the one set, else that
    /// of its place along the axis (matplotlib's
    /// `label.get_horizontalalignment()`).
    pub fn label_halign(&self) -> HAlign {
        self.read(|axis| axis.label.halign())
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
    /// use pltrs::prelude::*;
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
            label.bbox = props.map(|props| TextBoxPatch::new(props, label.font.size, &rc));
        });
        self
    }

    /// Puts `bbox`, a box as a text has it ([`Text::bbox_patch`]), behind
    /// the axis label, or none (matplotlib's `label.set_bbox` on the label
    /// itself).
    ///
    /// [`Text::bbox_patch`]: crate::text::Text::bbox_patch
    pub fn set_label_bbox_patch(&self, bbox: Option<TextBoxPatch>) -> &Self {
        self.update(|axis| axis.label.bbox = bbox);
        self
    }

    /// Shows or hides the axis label (matplotlib's `label.set_visible`).
    pub fn set_label_visible(&self, visible: bool) -> &Self {
        self.update(|axis| axis.label.visible = visible);
        self
    }

    /// Whether the axis label is drawn when it has text.
    pub fn label_visible(&self) -> bool {
        self.read(|axis| axis.label.visible)
    }

    /// The box behind the axis label, if any (matplotlib's
    /// `label.get_bbox_patch()`).
    pub fn label_bbox(&self) -> Option<TextBoxPatch> {
        self.read(|axis| axis.label.bbox.clone())
    }

    /// The display box of the axis label in the figure as it stands (not
    /// laid out first, the axes where it was last placed) and where the
    /// label stands (where it was last placed, see
    /// [`label_placed`](Self::label_placed)), if it is shown (matplotlib's
    /// `axis.label.get_window_extent()`).
    pub fn label_extent(&self) -> Option<Bbox> {
        self.with_painter_as_is(|painter, cx, rc| painter.axis_label_as_placed(cx, rc))
            .flatten()
            .and_then(Result::ok)
            .map(|label| label.bbox())
    }

    /// The display box of the offset (or scale) text in the figure as it
    /// stands (not laid out first), if it is shown (matplotlib's
    /// `axis.offsetText.get_window_extent()`). The other texts of the axis,
    /// which place it, are laid out unnoted (see
    /// [`take_font_notes`](pltrs_text::take_font_notes)): matplotlib
    /// measures the offset text alone, where its last draw placed it.
    pub fn offset_text_extent(&self) -> Option<Bbox> {
        self.with_painter_as_is(|painter, cx, rc| {
            let offset = crate::text::font_notes::unnoted(|| painter.texts(cx, rc)).offset?;
            offset.laid_out.measure_again(cx.dpi(), rc).ok()?;
            Some(offset.bbox())
        })
        .flatten()
    }

    /// Where the offset (or scale) text is anchored in display pixels
    /// with the axes where they are now, whether or not there is one:
    /// beyond the tick labels at the end of the axis (matplotlib's
    /// `offsetText.get_position()`, in display units across the axis).
    /// Reading it does not run the layout engine, as matplotlib places the
    /// text when it draws, nor note the fonts its texts look up (see
    /// [`take_font_notes`](pltrs_text::take_font_notes)), as reading
    /// matplotlib's measures nothing.
    pub fn offset_text_anchor(&self) -> Option<Point> {
        crate::text::font_notes::unnoted(|| self.with_texts_as_is(|texts, _| texts.offset_anchor))
            .flatten()
    }

    /// Where the last layout of the axis' texts (its last draw, or the
    /// last measure of them) anchored the offset (or scale) text in
    /// display pixels: matplotlib's `offsetText.get_position()`, which
    /// `_update_offset_text_position` sets when it draws or measures the
    /// axis; `None` before.
    pub fn offset_text_placed(&self) -> Option<Point> {
        self.read(|axis| axis.offset_placed.get())
    }

    /// Where the last layout of the axis' texts (its last draw, or the
    /// last measure of them) anchored the label in display pixels: beyond
    /// the spine, its ticks and the tick labels on its side (matplotlib's
    /// `label.get_position()` across the axis, which
    /// `_update_label_position` sets when it draws or measures the axis,
    /// whether the label has text and is shown or not); `None` before, and
    /// for a label placed by coordinates (`set_label_coords`).
    pub fn label_placed(&self) -> Option<Point> {
        self.read(|axis| {
            axis.label_placed
                .get()
                .filter(|_| axis.label.coords.is_none())
        })
    }

    /// The side the offset (or scale) text sits on: the side of the ticks
    /// for x (matplotlib's `offset_text_position`), left or right of the
    /// top end for y.
    pub fn offset_position(&self) -> SpineSide {
        self.read(|axis| axis.offset_side)
    }

    /// Puts the offset (or scale) text on `side` (matplotlib's
    /// `YAxis.set_offset_position`); moving the ticks to one side later
    /// takes it along.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// ax.yaxis().set_offset_position(SpineSide::Right)?;
    /// assert_eq!(ax.yaxis().offset_position(), SpineSide::Right);
    /// assert!(ax.yaxis().set_offset_position(SpineSide::Top).is_err());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `side` is not a side of this axis (e.g. `Top` for y).
    pub fn set_offset_position(&self, side: SpineSide) -> Result<&Self, PlotError> {
        self.check_side(side)?;
        self.update(|axis| axis.offset_side = side);
        Ok(self)
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
    fn label_alignment_set_apart_from_the_place_gives_way_to_the_next_loc() {
        let ax = Figure::new().subplot();
        let yaxis = ax.yaxis();
        assert_eq!(yaxis.label_halign(), HAlign::Center);
        yaxis
            .set_label_loc(LabelLoc::End)
            .set_label_halign(HAlign::Center);
        yaxis
            .set_label_position(SpineSide::Right)
            .expect("a y side");
        assert_eq!(
            (yaxis.label_loc(), yaxis.label_halign()),
            (LabelLoc::End, HAlign::Center)
        );
        yaxis.set_label_loc(LabelLoc::Start);
        assert_eq!(yaxis.label_halign(), HAlign::Left);
        assert_eq!(ax.xaxis().label_halign(), HAlign::Center);
    }

    #[test]
    fn labels_sit_anywhere_along_their_axis_keeping_their_alignment() {
        // matplotlib 3.11: in a 4 x 3 in figure at 100 dpi, an x label at
        // loc "right" moved to x = 0.7 ends at 267 px (its axes spans 50
        // to 360), a y label at loc "bottom" moved to y = 0.25 starts at
        // 90.75 px (33 to 264).
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot();
        ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
        ax.set_xlabel("time");
        ax.set_ylabel("depth");
        ax.xaxis().set_label_loc(LabelLoc::End).set_label_along(0.7);
        ax.yaxis()
            .set_label_loc(LabelLoc::Start)
            .set_label_along(0.25);
        fig.to_png().expect("drawn");
        let x = ax.xaxis().label_extent().expect("drawn").x1();
        let y = ax.yaxis().label_extent().expect("drawn").y0();
        assert!((x - 267.0).abs() < 1e-9, "{x}");
        assert!((y - 90.75).abs() < 1e-9, "{y}");
        assert_eq!(ax.xaxis().label_halign(), HAlign::Right);
        ax.xaxis().set_label_loc(LabelLoc::Center);
        assert_eq!(ax.xaxis().label_along(), 0.5, "the next loc places it");
    }

    #[test]
    fn labels_are_placed_where_the_last_layout_put_them() {
        // matplotlib 3.11: a 4 x 3 in figure at 100 dpi plotting (0, 0) to
        // (1, 1) has its x label at 3.83 px and its y label at 12.72 px
        // once drawn.
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot();
        ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
        ax.set_xlabel("x label");
        ax.set_ylabel("y label");
        assert_eq!(ax.xaxis().label_placed(), None, "not laid out yet");
        fig.to_png().expect("drawn");
        let x = ax.xaxis().label_placed().expect("placed").y;
        let y = ax.yaxis().label_placed().expect("placed").x;
        assert!((x - 3.833_333_333_333_327).abs() < 1e-9, "{x}");
        assert!((y - 12.722_222_222_222_223).abs() < 1e-9, "{y}");
        ax.xaxis().set_label_coords(0.5, -0.1);
        assert_eq!(ax.xaxis().label_placed(), None, "placed by coordinates");
    }

    #[test]
    fn offset_texts_keep_what_the_last_layout_gave() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot();
        ax.scatter(vec![1e6, 1e6 + 1.0], [0.0, 1.0])
            .expect("a scatter");
        let x = ax.xaxis();
        assert_eq!(
            (x.offset_text_placed(), x.kept_offset_text()),
            (None, String::new())
        );
        fig.to_png().expect("drawn");
        assert_eq!(x.kept_offset_text(), "+1e6");
        assert!(x.offset_text_placed().is_some());
        ax.set_xlim(0.0, 1.0).expect("finite limits");
        assert_eq!(x.kept_offset_text(), "+1e6", "until laid out again");
    }

    #[test]
    fn a_view_its_scale_maps_onto_one_point_places_its_texts_at_nan() {
        // Beyond 1 the logit scale maps the view onto one point: the data
        // transform is not finite, as matplotlib's, whose tick labels are
        // then measured at NaN, and the label and offset text of the
        // other axis beyond them.
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot();
        ax.scatter(vec![1e6, 1e6 + 1.0], [0.25, 0.75])
            .expect("a scatter");
        ax.set_xlabel("x");
        ax.set_ylim(0.5, 11.0).expect("limits");
        ax.set_yscale("logit").expect("a scale");
        let empty = fig.to_png().expect("drawn");
        let data = fig.resolve_transform(&ax.trans_data()).expect("resolved");
        let at = data.transform_point(Point::new(1e6, 0.75));
        assert!(at.x.is_nan() && at.y.is_nan(), "{at:?}");
        let x = ax.xaxis();
        assert_eq!(x.kept_offset_text(), "+1e6");
        assert!(x.offset_text_placed().is_some_and(|at| at.y.is_nan()));
        assert!(x.label_placed().is_some_and(|at| at.y.is_nan()));
        // Nothing of the axes but its patch is drawn.
        ax.set_axis_off();
        assert_eq!(fig.to_png().expect("drawn"), empty);
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
