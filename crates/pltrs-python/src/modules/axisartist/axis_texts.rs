//! The texts of axis artists drawn on their own (matplotlib's `LabelBase`,
//! `AxisLabel` and `TickLabels` made by their constructors and added to
//! an axes): the Python object keeps their properties and hands pltrs
//! (Rust) their whole description whenever one changes.

use pltrs::axes::Axes;
use pltrs::axisartist::{AxisTexts, AxisTextsArtist, AxisTextsKind};
use pltrs::colors::Color;
use pltrs::fonts::FontWeight;
use pltrs::text::{HAlign, VAlign};
use pltrs::transforms::Point;

use super::super::axes_grid1::side_named;
use crate::binding::prelude::*;

artist_handle!(AxisTextsArtist);

/// The look of the texts as Python describes it: the pad, the colour,
/// the font size and weight, the rotation, the vertical and horizontal
/// alignment and the axis direction.
type Style = (f64, Color, f64, String, f64, (String, String), String);

/// The texts as Python describes them: their kind (`"label"`,
/// `"axislabel"` or `"ticklabels"`), the distance of a `"label"` in
/// pixels, the texts with their reference points and angles, and their
/// [`Style`].
type Description = (String, f64, Vec<((f64, f64), f64, String)>, Style);

/// [`AxisTexts`] crossing from Python as a [`Description`].
struct PyAxisTexts(AxisTexts);

/// The value of `T` called `name`, `ValueError` naming `what` otherwise.
fn named<T: std::str::FromStr>(name: &str, what: &str) -> PyResult<T> {
    name.parse()
        .map_err(|_| PyErr::value_error(format!("{name:?} is not a valid {what}")))
}

impl FromPy<'_> for PyAxisTexts {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (kind, radius, texts, style): Description = object.extract()?;
        let (pad, color, size, weight, rotation, (valign, halign), direction) = style;
        let kind = match kind.as_str() {
            "label" => AxisTextsKind::Label { radius },
            "axislabel" => AxisTextsKind::AxisLabel,
            "ticklabels" => AxisTextsKind::TickLabels,
            _ => return Err(PyErr::value_error(format!("no axis texts {kind:?}"))),
        };
        Ok(Self(AxisTexts {
            kind,
            texts: texts
                .into_iter()
                .map(|((x, y), angle, text)| (Point::new(x, y), angle, text))
                .collect(),
            pad,
            color,
            size,
            weight: named::<FontWeight>(&weight, "weight")?,
            rotation,
            valign: named::<VAlign>(&valign, "alignment")?,
            halign: named::<HAlign>(&halign, "alignment")?,
            direction: side_named(&direction)?,
        }))
    }
}

/// `ax.add_artist(label)` of a `LabelBase`, `AxisLabel` or `TickLabels`
/// made on its own.
fn add_axis_texts(axes: Axes, texts: PyAxisTexts) -> PyResult<AxisTextsArtist> {
    Ok(axes.add_axis_texts(texts.0))
}

/// Hands pltrs (Rust) the texts again after a property changed.
fn set_axis_texts(artist: AxisTextsArtist, texts: PyAxisTexts) -> PyResult<()> {
    artist.set_texts(texts.0);
    Ok(())
}

/// The `(va, ha, rotation)` that `set_axis_direction(direction)` gives
/// an `AxisLabel` (`kind` `"axislabel"`) or `TickLabels`
/// (`"ticklabels"`).
fn axis_direction_defaults(kind: String, direction: String) -> PyResult<(String, String, f64)> {
    let kind = match kind.as_str() {
        "axislabel" => AxisTextsKind::AxisLabel,
        "ticklabels" => AxisTextsKind::TickLabels,
        _ => {
            return Err(PyErr::value_error(format!(
                "{kind:?} has no axis direction"
            )));
        }
    };
    let (valign, halign, angle) = kind
        .direction_defaults(side_named(&direction)?)
        .expect("axis labels and tick labels follow conventions");
    Ok((valign.name().to_owned(), halign.name().to_owned(), angle))
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("axis_direction_defaults", axis_direction_defaults)?;
    module.add("add_axis_texts", add_axis_texts)?;
    module.add("set_axis_texts", set_axis_texts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};
    use pltrs::figure::Figure;

    #[test]
    fn descriptions_cross_whole() {
        with_python(|py| {
            let description = eval(
                py,
                "d = ('ticklabels', 0.0, [((0.2, 0.5), -90.0, '0.2')], \
                 (10.0, 'black', 10.0, 'normal', 90.0, ('center', 'right'), 'left'))",
                "d",
            );
            let texts = PyAxisTexts::from_py(&description).expect("a description").0;
            assert_eq!(texts.kind, AxisTextsKind::TickLabels);
            assert_eq!(
                (texts.valign, texts.halign),
                (VAlign::Center, HAlign::Right)
            );
            let ax = Figure::new().subplot();
            let artist = add_axis_texts(ax, PyAxisTexts(texts.clone())).expect("added");
            set_axis_texts(artist.clone(), PyAxisTexts(AxisTexts { pad: 4.0, ..texts }))
                .expect("set");
            assert_eq!(artist.texts().pad, 4.0);
            let bad = eval(
                py,
                "b = ('nonsense', 0.0, [], (1.0, 'k', 1.0, 'normal', 0.0, ('top', 'left'), 'left'))",
                "b",
            );
            assert!(PyAxisTexts::from_py(&bad).is_err());
        });
    }
}
