//! Labelling the wedges of a pie (matplotlib's `pie_label`).

use super::pie::{PieContainer, PieError};
use super::{Axes, PlotError};
use crate::colors::Color;
use crate::fonts::FontSize;
use crate::patches::{Patch, Wedge};
use crate::text::{HAlign, Text, VAlign};
use crate::ticker::pyfmt::{StrTemplate, TemplateValue};

/// The labels [`Axes::pie_label`] writes.
#[derive(Clone, Debug, PartialEq)]
pub enum PieLabels {
    /// One text per wedge.
    Texts(Vec<String>),
    /// A `str.format` template with the fields `absval` (the wedge's value)
    /// and `frac` (its fraction of the pie), such as
    /// `"{absval:d} ({frac:.0%})"`. Values that are all whole numbers
    /// format as integers, as matplotlib's integer arrays do.
    Format(String),
}

impl From<&str> for PieLabels {
    fn from(template: &str) -> Self {
        Self::Format(template.to_owned())
    }
}

impl From<Vec<String>> for PieLabels {
    fn from(texts: Vec<String>) -> Self {
        Self::Texts(texts)
    }
}

impl From<&[&str]> for PieLabels {
    fn from(texts: &[&str]) -> Self {
        Self::Texts(texts.iter().map(|&text| text.to_owned()).collect())
    }
}

/// How [`Axes::pie_label`] aligns the labels (matplotlib's `alignment`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PieLabelAlignment {
    /// Centred on their points.
    Center,
    /// Away from the centre: labels on the left end at their points, labels
    /// on the right start there (and, when rotated, labels on top sit
    /// above, labels below hang below).
    Outer,
    /// [`PieLabelAlignment::Outer`] outside the pie (distance > 1),
    /// [`PieLabelAlignment::Center`] inside it.
    #[default]
    Auto,
}

/// Options of [`Axes::pie_label`] (matplotlib's keyword arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct PieLabelOptions {
    /// Distance of the labels from the centre in radii (`distance`).
    pub distance: f64,
    /// Whether each label is turned to its wedge's angle (`rotate`).
    pub rotate: bool,
    /// How the labels are aligned (`alignment`).
    pub alignment: PieLabelAlignment,
    /// Font size of the labels (from `textprops`).
    pub size: Option<FontSize>,
    /// Colour of the labels (from `textprops`).
    pub color: Option<Color>,
}

impl Default for PieLabelOptions {
    /// matplotlib's defaults: centred labels at 0.6 radii.
    fn default() -> Self {
        Self {
            distance: 0.6,
            rotate: false,
            alignment: PieLabelAlignment::Auto,
            size: None,
            color: None,
        }
    }
}

/// Where a set of pie labels goes.
pub(super) struct LabelPlacement {
    pub(super) distance: f64,
    /// Align labels away from the pie rather than centring them.
    pub(super) outer: bool,
    pub(super) rotate: bool,
}

/// The texts `template` writes for each wedge of `container`.
fn formatted(template: &str, container: &PieContainer) -> Result<Vec<String>, PieError> {
    let template =
        StrTemplate::parse_named(template, &["absval", "frac"]).map_err(PieError::LabelFormat)?;
    let whole = container
        .values
        .iter()
        .all(|v| v.fract() == 0.0 && v.abs() < 9.0e15);
    container
        .values
        .iter()
        .zip(&container.fracs)
        .map(|(&value, &frac)| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the value is a whole number well within i64"
            )]
            let absval = if whole {
                TemplateValue::Int(value as i64)
            } else {
                TemplateValue::Float(value)
            };
            template
                .render_values(&[absval, TemplateValue::Float(frac)], |text| text)
                .map_err(PieError::LabelFormat)
        })
        .collect()
}

impl Axes {
    /// Labels the wedges of `container` (matplotlib's `pie_label`), and
    /// records the labels in it.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{PieLabelOptions, PieOptions};
    /// let ax = Figure::new().subplot();
    /// let mut pie = ax.pie(&[1.0, 3.0], PieOptions::default())?;
    /// let texts = ax.pie_label(&mut pie, "{absval:d} ({frac:.0%})", PieLabelOptions::default())?;
    /// assert_eq!(texts[1].text(), "3 (75%)");
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a list of labels does not have one label per wedge, or the
    /// template is invalid for the values.
    pub fn pie_label(
        &self,
        container: &mut PieContainer,
        labels: impl Into<PieLabels>,
        options: PieLabelOptions,
    ) -> Result<Vec<Text>, PlotError> {
        let texts = match labels.into() {
            PieLabels::Format(template) => {
                formatted(&template, container).map_err(PlotError::Pie)?
            }
            PieLabels::Texts(texts) if texts.len() != container.wedges.len() => {
                return Err(PlotError::Pie(PieError::LabelCount {
                    labels: texts.len(),
                    wedges: container.wedges.len(),
                }));
            }
            PieLabels::Texts(texts) => texts,
        };
        let outer = match options.alignment {
            PieLabelAlignment::Center => false,
            PieLabelAlignment::Outer => true,
            PieLabelAlignment::Auto => options.distance > 1.0,
        };
        let placement = LabelPlacement {
            distance: options.distance,
            outer,
            rotate: options.rotate,
        };
        let made = self.place_pie_labels(
            &container.wedges,
            &texts,
            &placement,
            options.size,
            options.color.as_ref(),
        );
        container.added_labels.push(made.clone());
        Ok(made)
    }

    /// A text per wedge at `placement.distance` radii along the wedge's
    /// bisector (the placement of matplotlib's `pie_label`).
    pub(super) fn place_pie_labels(
        &self,
        wedges: &[Patch<Wedge>],
        labels: &[String],
        placement: &LabelPlacement,
        size: Option<FontSize>,
        color: Option<&Color>,
    ) -> Vec<Text> {
        wedges
            .iter()
            .zip(labels)
            .map(|(wedge, label)| {
                let shape = wedge.shape();
                let thetam =
                    2.0 * std::f64::consts::PI * 0.5 * (shape.theta1 + shape.theta2) / 360.0;
                let x = shape.center.x + placement.distance * shape.r * thetam.cos();
                let y = shape.center.y + placement.distance * shape.r * thetam.sin();
                let halign = match (placement.outer, x > 0.0) {
                    (false, _) => HAlign::Center,
                    (true, true) => HAlign::Left,
                    (true, false) => HAlign::Right,
                };
                let (valign, rotation) = if placement.rotate {
                    let valign = match (placement.outer, y > 0.0) {
                        (false, _) => VAlign::Center,
                        (true, true) => VAlign::Bottom,
                        (true, false) => VAlign::Top,
                    };
                    let flip = if x > 0.0 { 0.0 } else { 180.0 };
                    (valign, thetam.to_degrees() + flip)
                } else {
                    (VAlign::Center, 0.0)
                };
                let text = self.text(x, y, label.clone());
                text.set_rotation(rotation)
                    .set_halign(halign)
                    .set_valign(valign);
                if let Some(size) = size {
                    text.set_fontsize(size);
                }
                if let Some(color) = color {
                    text.set_color(color.clone());
                }
                text
            })
            .collect()
    }
}
