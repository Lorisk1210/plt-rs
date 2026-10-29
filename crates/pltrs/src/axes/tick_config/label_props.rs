//! Text properties of tick labels (the keyword arguments of matplotlib's
//! `set_ticklabels`), for a whole tick level or for single major ticks.

use std::borrow::Cow;

use pltrs_text::{HAlign, RotationMode, VAlign};

use crate::axes::axis::AxisState;
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
}

impl TickLabelProps {
    /// The same properties with a relative font size turned into points
    /// against `base_size` (font sizes are resolved when they are set).
    fn with_absolute_size(&self, base_size: f64) -> Self {
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
        }
    }

    /// Sets the label properties of `style` that are set here; font sizes
    /// must be absolute ([`Self::with_absolute_size`]).
    fn apply(&self, style: &mut TickStyle) {
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
    }
}

impl AxisState {
    /// The style of the major tick at `index` (counting every tick the
    /// major locator places): the major style with that tick's label,
    /// mark and gridline properties on top.
    pub(crate) fn major_tick_style(&self, index: usize) -> Cow<'_, TickStyle> {
        let label = (self.major_label_overrides.get(index))
            .filter(|props| **props != TickLabelProps::default());
        let line = (self.major_line_overrides.get(index))
            .filter(|props| **props != TickLineProps::default());
        if label.is_none() && line.is_none() {
            return Cow::Borrowed(&self.major_style);
        }
        let mut style = self.major_style.clone();
        if let Some(props) = label {
            props.apply(&mut style);
        }
        if let Some(props) = line {
            props.apply(&mut style);
        }
        Cow::Owned(style)
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
        let props = props.with_absolute_size(crate::rc::params().font_size);
        self.update_labels(|axis| {
            let overrides = &mut axis.major_label_overrides;
            if overrides.len() <= index {
                overrides.resize_with(index + 1, TickLabelProps::default);
            }
            overrides[index] = overrides[index].overlay(&props);
        });
        self
    }
}
