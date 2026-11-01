//! Labels of bar and pie charts: `bar_label` and `pie_label`.

use pltrs::axes::{
    Axes, BarLabelError, BarLabelOptions, BarLabelType, PieContainer, PieLabelAlignment,
    PieLabelOptions, PieLabels,
};
use pltrs::container::Container;
use pltrs::patches::{Patch, Wedge};
use pltrs::text::Text;
use pltrs::ticker::LabelFormat;

use crate::binding::prelude::*;

impl From<BarLabelError> for PyErr {
    /// matplotlib's `ValueError` for paddings that do not match the bars,
    /// or a template that formats no value.
    fn from(err: BarLabelError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// How bar values are written: a `%`- or `{}`-style template, or a
/// function of the value.
pub(super) struct PyLabelFormat(LabelFormat);

impl FromPy<'_> for PyLabelFormat {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(template) = object.extract::<String>() {
            return Ok(Self(LabelFormat::Template(template)));
        }
        Ok(Self(crate::modules::archive::label_format(object)?))
    }
}

/// Labels the bars of `container` (matplotlib's `bar_label`): `labels`
/// (empty: the formatted values), the values formatted with `fmt`, at the
/// bar `"edge"` or `"center"`, `padding` points away (one for all or one
/// per bar).
pub(super) fn bar_label(
    axes: Axes,
    container: Container,
    labels: Vec<String>,
    fmt: PyLabelFormat,
    label_type: String,
    padding: Vec<f64>,
) -> PyResult<Vec<Text>> {
    let Container::Bar(bars) = container else {
        return Err(PyErr::type_error(
            "bar_label() takes the container of a bar plot",
        ));
    };
    let options = BarLabelOptions {
        labels,
        fmt: fmt.0,
        label_type: if label_type == "center" {
            BarLabelType::Center
        } else {
            BarLabelType::Edge
        },
        padding,
    };
    Ok(axes
        .bar_label(&bars, options)?
        .into_iter()
        .map(|annotation| (*annotation).clone())
        .collect())
}

/// A pie: `(wedges, values, fractions)`.
type Pie = (Vec<Patch<Wedge>>, Vec<f64>, Vec<f64>);

/// Labels the wedges of a pie (matplotlib's `pie_label`): one text per
/// wedge, or a template of `absval` and `frac` when `template` (typeset
/// with TeX when `usetex`, `None` for `text.usetex`), at `distance`
/// radii, turned to the wedges when `rotate`, aligned `"center"`,
/// `"outer"` or `"auto"`.
pub(super) fn pie_label(
    axes: Axes,
    (wedges, values, fracs): Pie,
    (labels, template, usetex): (Vec<String>, bool, Option<bool>),
    distance: f64,
    rotate: bool,
    alignment: String,
) -> PyResult<Vec<Text>> {
    let mut pie = PieContainer {
        wedges,
        values,
        fracs,
        labels: Vec::new(),
        pct_labels: Vec::new(),
        added_labels: Vec::new(),
    };
    let labels = match (template, labels.first()) {
        (true, Some(template)) => PieLabels::Format(template.clone()),
        _ => PieLabels::Texts(labels),
    };
    let options = PieLabelOptions {
        distance,
        rotate,
        alignment: match alignment.as_str() {
            "center" => PieLabelAlignment::Center,
            "outer" => PieLabelAlignment::Outer,
            _ => PieLabelAlignment::Auto,
        },
        usetex,
        ..PieLabelOptions::default()
    };
    Ok(axes.pie_label(&mut pie, labels, options)?)
}
