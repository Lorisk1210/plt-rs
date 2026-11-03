//! Text properties of tick labels (the keyword arguments of matplotlib's
//! `set_ticklabels`), for a whole tick level or for single ticks.

use std::borrow::Cow;

use pltrs_text::{HAlign, RotationMode, VAlign};

use crate::axes::axis::{AxisState, TickOverrides};
use crate::axes::tick_lines::TickLineProps;
use crate::axes::ticks::TickStyle;
use crate::axes::{Axis, TickLevel};
use crate::colors::Color;
use crate::fonts::{FontSize, FontStyle, FontWeight};

/// Text properties of tick labels (the keyword arguments of matplotlib's
/// `set_ticklabels`); unset fields keep their current value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickLabelProps {
    /// Rotation in degrees.
    pub rotation: Option<f64>,
    /// How rotated labels are aligned.
    pub rotation_mode: Option<RotationMode>,
    /// Horizontal alignment.
    pub halign: Option<HAlign>,
    /// Vertical alignment.
    pub valign: Option<VAlign>,
    /// Alignment of the lines of multi-line labels (`multialignment`).
    pub multialign: Option<HAlign>,
    /// Text colour.
    pub color: Option<Color>,
    /// Font size.
    pub fontsize: Option<FontSize>,
    /// Font weight.
    pub fontweight: Option<FontWeight>,
    /// Font style (upright, italic or oblique).
    pub fontstyle: Option<FontStyle>,
    /// Whether the labels count in layouts and push the axis label and
    /// the offset text away (matplotlib's `set_in_layout`).
    pub in_layout: Option<bool>,
    /// The hyperlink of the labels in vector output (matplotlib's
    /// `set_url` on a tick or its labels).
    pub url: Option<String>,
    /// Font family in place of `font.family` (matplotlib's
    /// `labelfontfamily`).
    pub family: Option<String>,
    /// Whether the label on the first side (bottom or left) is drawn
    /// (matplotlib's `label1.set_visible`, `label1On`).
    pub label1_visible: Option<bool>,
    /// Whether the label on the second side (top or right) is drawn
    /// (matplotlib's `label2.set_visible`, `label2On`).
    pub label2_visible: Option<bool>,
}

pltrs_core::archive_struct!(TickLabelProps {
    rotation,
    rotation_mode,
    halign,
    valign,
    multialign,
    color,
    fontsize,
    fontweight,
    fontstyle,
    in_layout,
    url,
    family,
    label1_visible,
    label2_visible
});

impl TickLabelProps {
    /// The same properties with a relative font size turned into points
    /// against `base_size` (font sizes are resolved when they are set).
    pub(crate) fn with_absolute_size(&self, base_size: f64) -> Self {
        Self {
            fontsize: (self.fontsize).map(|size| FontSize::Points(size.to_points(base_size))),
            ..self.clone()
        }
    }

    /// `self` with every property set in `newer` replaced.
    fn overlay(&self, newer: &Self) -> Self {
        Self {
            rotation: newer.rotation.or(self.rotation),
            rotation_mode: newer.rotation_mode.or(self.rotation_mode),
            halign: newer.halign.or(self.halign),
            valign: newer.valign.or(self.valign),
            multialign: newer.multialign.or(self.multialign),
            color: newer.color.clone().or_else(|| self.color.clone()),
            fontsize: newer.fontsize.or(self.fontsize),
            fontweight: newer.fontweight.or(self.fontweight),
            fontstyle: newer.fontstyle.or(self.fontstyle),
            in_layout: newer.in_layout.or(self.in_layout),
            url: newer.url.clone().or_else(|| self.url.clone()),
            family: newer.family.clone().or_else(|| self.family.clone()),
            label1_visible: newer.label1_visible.or(self.label1_visible),
            label2_visible: newer.label2_visible.or(self.label2_visible),
        }
    }

    /// Sets the label properties of `style` that are set here; font sizes
    /// must be absolute ([`Self::with_absolute_size`]).
    pub(crate) fn apply(&self, style: &mut TickStyle) {
        if let Some(rotation) = self.rotation {
            style.label_rotation = rotation;
        }
        if let Some(mode) = self.rotation_mode {
            style.label_rotation_mode = mode;
        }
        if self.halign.is_some() {
            style.label_halign = self.halign;
        }
        if self.valign.is_some() {
            style.label_valign = self.valign;
        }
        if self.multialign.is_some() {
            style.label_multialign = self.multialign;
        }
        if let Some(color) = &self.color {
            style.label_color = color.clone();
        }
        if let Some(size) = self.fontsize {
            style.label_size = size.to_points(style.label_size);
        }
        if self.fontweight.is_some() {
            style.label_weight = self.fontweight;
        }
        if self.fontstyle.is_some() {
            style.label_style = self.fontstyle;
        }
        if let Some(in_layout) = self.in_layout {
            style.label_in_layout = in_layout;
        }
        if self.url.is_some() {
            style.label_url.clone_from(&self.url);
        }
        if self.family.is_some() {
            style.label_family.clone_from(&self.family);
        }
        if let Some(visible) = self.label1_visible {
            style.labels.first = visible;
        }
        if let Some(visible) = self.label2_visible {
            style.labels.second = visible;
        }
    }
}

impl AxisState {
    /// The style of the major (or `minor`) tick at `index`, counting every
    /// tick the locator places: the style of its level with that tick's
    /// label (on the first side), mark and gridline properties and its pad
    /// on top.
    pub(crate) fn tick_style_at(&self, minor: bool, index: usize) -> Cow<'_, TickStyle> {
        self.tick_label_style_at(minor, index, true)
    }

    /// The style the label on the first or the second side of the major
    /// (or `minor`) tick at `index` is drawn with: as
    /// [`tick_style_at`](Self::tick_style_at), with what that label has of
    /// its own (matplotlib's `Tick.label1` and `label2` are texts of their
    /// own).
    pub(crate) fn tick_label_style_at(
        &self,
        minor: bool,
        index: usize,
        first_side: bool,
    ) -> Cow<'_, TickStyle> {
        let (level, overrides) = if minor {
            (&self.minor_style, &self.minor_overrides)
        } else {
            (&self.major_style, &self.major_overrides)
        };
        overrides.style_at(level, index, first_side)
    }
}

impl TickOverrides {
    /// The style the label on the first or the second side of the tick
    /// at `index` of a level styled `level` is drawn with: `level` with
    /// the properties of that label, of the tick's mark and gridline and
    /// its pad on top.
    pub(crate) fn style_at<'a>(
        &self,
        level: &'a TickStyle,
        index: usize,
        first_side: bool,
    ) -> Cow<'a, TickStyle> {
        let labels = if first_side {
            &self.labels
        } else {
            &self.second_labels
        };
        let label = (labels.get(index)).filter(|props| **props != TickLabelProps::default());
        let line = (self.lines.get(index)).filter(|props| **props != TickLineProps::default());
        let pad = self.pads.get(index).copied().flatten();
        if label.is_none() && line.is_none() && pad.is_none() {
            return Cow::Borrowed(level);
        }
        let mut style = level.clone();
        if let Some(props) = label {
            props.apply(&mut style);
        }
        if let Some(props) = line {
            props.apply(&mut style);
        }
        if let Some(pad) = pad {
            style.pad = pad;
        }
        Cow::Owned(style)
    }

    /// Sets the properties of the label on the first or the second side
    /// of the tick at `index` to `change(old)`.
    pub(crate) fn change_label(
        &mut self,
        index: usize,
        first_side: bool,
        change: impl FnOnce(&TickLabelProps) -> TickLabelProps,
    ) {
        let labels = if first_side {
            &mut self.labels
        } else {
            &mut self.second_labels
        };
        if labels.len() <= index {
            labels.resize_with(index + 1, TickLabelProps::default);
        }
        labels[index] = change(&labels[index]);
    }

    /// The properties the label on the first or the second side of the
    /// tick at `index` has of its own.
    pub(crate) fn label(&self, index: usize, first_side: bool) -> TickLabelProps {
        let labels = if first_side {
            &self.labels
        } else {
            &self.second_labels
        };
        labels.get(index).cloned().unwrap_or_default()
    }
}

impl Axis {
    /// Changes the text properties of the labels of one tick level.
    pub fn set_ticklabel_props(&self, which: TickLevel, props: TickLabelProps) -> &Self {
        let props = props.with_absolute_size(crate::rc::params().font_size);
        self.update_labels(|axis| {
            if which.major() {
                props.apply(&mut axis.major_style);
            }
            if which.minor() {
                props.apply(&mut axis.minor_style);
            }
        });
        self
    }

    /// Changes the text properties of the labels of the major tick at
    /// `index` among those the major locator places, in view or not
    /// (matplotlib's `axis.get_major_ticks()[index].label1.set(...)` and
    /// the same on `label2`). They apply on top of the properties of the
    /// major tick labels, to that tick only.
    ///
    /// ```
    /// use pltrs::axes::TickLabelProps;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot();
    /// ax.set_xticks([0.0, 0.5, 1.0]);
    /// let tilted = TickLabelProps { rotation: Some(45.0), ..TickLabelProps::default() };
    /// ax.xaxis().set_major_ticklabel_props(1, tilted);
    /// ```
    pub fn set_major_ticklabel_props(&self, index: usize, props: TickLabelProps) -> &Self {
        for first_side in [true, false] {
            self.change_tick_label_props(
                false,
                index,
                first_side,
                TickLabelProps::overlay,
                props.clone(),
            );
        }
        self
    }

    /// Replaces the text properties of the labels of the major (or
    /// `minor`) tick at `index` with `props`, unset fields following the
    /// labels of its level (what matplotlib's `Tick.label1` and `label2`
    /// keep of their own once changed one by one).
    ///
    /// ```
    /// use pltrs::axes::TickLabelProps;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot();
    /// ax.xaxis().set_ticks([0.0, 0.5]).set_minor_ticks([0.25]);
    /// let hidden = TickLabelProps { label1_visible: Some(false), ..TickLabelProps::default() };
    /// ax.xaxis().set_tick_label_props(true, 0, hidden.clone());
    /// assert_eq!(ax.xaxis().tick_label_props(true, 0), hidden);
    /// assert_eq!(ax.xaxis().tick_label_props(false, 0), TickLabelProps::default());
    /// ```
    pub fn set_tick_label_props(&self, minor: bool, index: usize, props: TickLabelProps) -> &Self {
        for first_side in [true, false] {
            self.set_tick_label_props_on(minor, index, first_side, props.clone());
        }
        self
    }

    /// Replaces the text properties of the label on the first (bottom or
    /// left) or the second side of the major (or `minor`) tick at `index`
    /// with `props`, unset fields following the labels of its level
    /// (matplotlib's `Tick.label1` and `label2` changed one by one); the
    /// label on the other side keeps its own.
    ///
    /// ```
    /// use pltrs::axes::TickLabelProps;
    /// use pltrs::figure::Figure;
    /// use pltrs::text::VAlign;
    /// let ax = Figure::new().subplot();
    /// ax.set_yticks([0.0, 0.5]);
    /// let centered = TickLabelProps { valign: Some(VAlign::Center), ..TickLabelProps::default() };
    /// ax.yaxis().set_tick_label_props_on(false, 0, true, centered.clone());
    /// assert_eq!(ax.yaxis().tick_label_props_on(false, 0, true), centered);
    /// assert_eq!(ax.yaxis().tick_label_props_on(false, 0, false), TickLabelProps::default());
    /// ```
    pub fn set_tick_label_props_on(
        &self,
        minor: bool,
        index: usize,
        first_side: bool,
        props: TickLabelProps,
    ) -> &Self {
        self.change_tick_label_props(minor, index, first_side, |_, new| new.clone(), props)
    }

    /// Sets the properties of the label on the first or the second side
    /// of the major (or `minor`) tick at `index` to `change(old, props)`.
    fn change_tick_label_props(
        &self,
        minor: bool,
        index: usize,
        first_side: bool,
        change: impl FnOnce(&TickLabelProps, &TickLabelProps) -> TickLabelProps,
        props: TickLabelProps,
    ) -> &Self {
        let props = props.with_absolute_size(crate::rc::params().font_size);
        self.update_labels(|axis| {
            (axis.overrides_mut(minor)).change_label(index, first_side, |old| change(old, &props));
        });
        self
    }

    /// The text properties the label on the first side of the major (or
    /// `minor`) tick at `index` has of its own (see
    /// [`set_tick_label_props`](Self::set_tick_label_props)).
    pub fn tick_label_props(&self, minor: bool, index: usize) -> TickLabelProps {
        self.tick_label_props_on(minor, index, true)
    }

    /// The text properties the label on the first or the second side of
    /// the major (or `minor`) tick at `index` has of its own (see
    /// [`set_tick_label_props_on`](Self::set_tick_label_props_on)).
    pub fn tick_label_props_on(
        &self,
        minor: bool,
        index: usize,
        first_side: bool,
    ) -> TickLabelProps {
        self.read(|axis| axis.overrides(minor).label(index, first_side))
    }

    /// The style the label on the first or the second side of the major
    /// (or `minor`) tick at `index` is drawn with: the style of its level
    /// with the tick's and the label's own properties on top.
    pub fn tick_label_style_at(&self, minor: bool, index: usize, first_side: bool) -> TickStyle {
        self.read(|axis| {
            axis.tick_label_style_at(minor, index, first_side)
                .into_owned()
        })
    }
}
