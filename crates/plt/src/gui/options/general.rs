//! The general form of an axes page in the figure options dialog: the
//! title, the limits, label and scale of both axes, and the legend switch.

use plt_ui::{Checkbox, UiWindow, WidgetId};

use super::OptionsMessage;
use super::fields::{
    FieldError, Form, chosen, date_text, date_zone, limit_text, parse_date, parse_limit,
    read_field, text_of,
};
use crate::axes::{Axes, AxisName};
use crate::dates::TimeZone;
use crate::scale::ScaleName;
use crate::units::UnitConverter;

/// The scales offered, as matplotlib names them.
static SCALES: [ScaleName; 4] = [
    ScaleName::Linear,
    ScaleName::Log,
    ScaleName::Symlog,
    ScaleName::Logit,
];

/// The field of an axis limit, showing it rounded, or as a date on an
/// axis of dates (as matplotlib's editor shows them).
#[derive(Debug, Clone)]
pub(super) struct LimitField {
    id: WidgetId,
    /// The limit, which the field keeps while its text is the one shown.
    value: f64,
    /// The text the field was given.
    shown: String,
    /// On an axis of dates, the zone the limit, a date number, is shown
    /// and typed in.
    zone: Option<TimeZone>,
}

impl LimitField {
    /// The text showing limit `value` on an axis with `resolution` (see
    /// [`limit_text`]), as a date in `zone` on an axis of dates.
    fn text(value: f64, resolution: Option<f64>, zone: Option<&TimeZone>) -> String {
        zone.and_then(|zone| date_text(value, zone))
            .unwrap_or_else(|| limit_text(value, resolution))
    }

    /// The limit typed into the field called `name`, or the exact limit
    /// it showed if the text was not edited (so that applying unedited
    /// fields changes no limit).
    fn read(&self, window: &mut UiWindow<OptionsMessage>, name: &str) -> Result<f64, FieldError> {
        read_field(window, self.id, |text| {
            if text == self.shown {
                Ok(self.value)
            } else if let Some(zone) = &self.zone {
                parse_date(name, text, zone)
            } else {
                parse_limit(name, text)
            }
        })
    }
}

/// The fields of one axis.
#[derive(Debug, Clone)]
pub(super) struct AxisFields {
    name: AxisName,
    min: LimitField,
    max: LimitField,
    label: WidgetId,
    scale: WidgetId,
}

impl AxisFields {
    /// The text fields: the limits and the label.
    pub(super) fn text_fields(&self) -> [WidgetId; 3] {
        [self.min.id, self.max.id, self.label]
    }
}

/// The settings of an axis read from its fields.
#[derive(Debug)]
pub(super) struct AxisPlan {
    name: AxisName,
    limits: (f64, f64),
    label: String,
    scale: ScaleName,
}

impl AxisPlan {
    /// Sets the axis' scale, limits and label on `ax`.
    pub(super) fn apply(self, ax: &Axes) {
        let (min, max) = self.limits;
        // An unknown scale cannot come from the fixed choices.
        match self.name {
            AxisName::X => {
                if ax.xscale() != self.scale {
                    let _ = ax.set_xscale(self.scale);
                }
                ax.set_xlim(min, max)
                    .expect("the form takes finite limits only")
                    .set_xlabel(self.label);
            }
            AxisName::Y => {
                if ax.yscale() != self.scale {
                    let _ = ax.set_yscale(self.scale);
                }
                ax.set_ylim(min, max)
                    .expect("the form takes finite limits only")
                    .set_ylabel(self.label);
            }
        }
    }
}

/// The title, both axes and the legend switch.
pub(super) fn general_form(
    axes: &Axes,
    shown_title: &str,
) -> (
    WidgetId,
    [AxisFields; 2],
    WidgetId,
    plt_ui::Form<OptionsMessage>,
) {
    let mut form = Form::new();
    form.section("");
    let title = form.text("Title", shown_title);
    let mut axis = |name: AxisName| {
        let (heading, (min, max), scale, axis) = match name {
            AxisName::X => ("X axis", axes.xlim(), axes.xscale(), axes.xaxis()),
            AxisName::Y => ("Y axis", axes.ylim(), axes.yscale(), axes.yaxis()),
        };
        form.section(heading);
        let zone = (axis.converter().as_ref())
            .is_some_and(UnitConverter::is_dates)
            .then(date_zone);
        let shown = |value| {
            let resolution = axes.pixel_resolution(name, value);
            LimitField::text(value, resolution, zone.as_ref())
        };
        let (min_text, max_text) = (shown(min), shown(max));
        let (min_id, max_id) = form.pair(("Min", min_text.clone()), ("Max", max_text.clone()));
        let min = LimitField {
            id: min_id,
            value: min,
            shown: min_text,
            zone: zone.clone(),
        };
        let max = LimitField {
            id: max_id,
            value: max,
            shown: max_text,
            zone,
        };
        let label = form.text("Label", axes.label(name));
        let current = SCALES.iter().position(|s| *s == scale).unwrap_or(0);
        let scale = form.choice("Scale", &SCALES.each_ref().map(ScaleName::as_str), current);
        AxisFields {
            name,
            min,
            max,
            label,
            scale,
        }
    };
    let fields = [axis(AxisName::X), axis(AxisName::Y)];
    form.section("");
    let legend = form.wide(Checkbox::new("(Re-)Generate automatic legend", |_| {
        OptionsMessage::Edited
    }));
    (title, fields, legend, form.finish())
}

/// The settings typed into the fields of one axis.
pub(super) fn read_axis(
    window: &mut UiWindow<OptionsMessage>,
    fields: &AxisFields,
) -> Result<AxisPlan, FieldError> {
    let axis = match fields.name {
        AxisName::X => "X",
        AxisName::Y => "Y",
    };
    let min = fields.min.read(window, &format!("{axis} min"))?;
    let max = fields.max.read(window, &format!("{axis} max"))?;
    Ok(AxisPlan {
        name: fields.name,
        limits: (min, max),
        label: text_of(window, fields.label),
        scale: SCALES[chosen(window, fields.scale).min(SCALES.len() - 1)].clone(),
    })
}
