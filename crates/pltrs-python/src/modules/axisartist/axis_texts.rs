//! The texts of axis artists drawn on their own (matplotlib's `LabelBase`,
//! `AxisLabel` and `TickLabels` made by their constructors and added to
//! an axes): the Python object keeps their properties and hands pltrs
//! (Rust) their whole description whenever one changes.

use pltrs::axes::Axes;
use pltrs::axisartist::{AxisTexts, AxisTextsArtist, AxisTextsKind, TextLook};
use pltrs::colors::Color;
use pltrs::text::{HAlign, LineSpacing, VAlign};
use pltrs::transforms::Point;

use super::super::axes_grid1::side_named;
use super::super::font_manager::FontProps;
use super::super::text::rotation_mode_name;
use crate::binding::prelude::*;

artist_handle!(AxisTextsArtist);

/// How texts are typeset beyond their size, colour, angle and alignment
/// ([`TextLook`]), crossing as the tuple of the font properties (as
/// `FontManager._props` makes them), the alignment of the lines (`None`
/// follows the texts'), the line spacing (`None` for `normal`) and the
/// rotation mode, both ways.
pub(super) struct PyTextLook(pub(super) TextLook);

impl FromPy<'_> for PyTextLook {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (font, multialign, linespacing, rotation_mode, antialiased, parse_math): (
            FontProps,
            Option<String>,
            Option<f64>,
            String,
            Option<bool>,
            Option<bool>,
        ) = object.extract()?;
        Ok(Self(TextLook {
            font: font.0,
            multialign: multialign
                .map(|name| named::<HAlign>(&name, "alignment"))
                .transpose()?,
            linespacing: linespacing.map_or(LineSpacing::Normal, LineSpacing::Factor),
            rotation_mode: named(&rotation_mode, "rotation mode")?,
            antialiased,
            parse_math,
        }))
    }
}

impl IntoPy for PyTextLook {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let look = self.0;
        let linespacing = match look.linespacing {
            LineSpacing::Normal => None,
            LineSpacing::Factor(factor) => Some(factor),
        };
        (
            FontProps(look.font),
            look.multialign.map(HAlign::name),
            linespacing,
            rotation_mode_name(look.rotation_mode),
            look.antialiased,
            look.parse_math,
        )
            .into_py(py)
    }
}

/// The look of the texts as Python describes it: the pad, the colour,
/// the font size, the rest of their typesetting, the rotation, the
/// vertical and horizontal alignment and the axis direction.
type Style = (f64, Color, f64, PyTextLook, f64, (String, String), String);

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
        let (pad, color, size, look, rotation, (valign, halign), direction) = style;
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
            look: look.0,
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

/// The display boxes `(x0, y0, x1, y1)` of the non-empty texts
/// (`TickLabels.get_window_extents` of texts made on their own).
fn axis_text_extents(artist: AxisTextsArtist) -> PyResult<Vec<(f64, f64, f64, f64)>> {
    Ok(artist
        .text_extents()
        .iter()
        .map(|b| (b.x0(), b.y0(), b.x1(), b.y1()))
        .collect())
}

/// The width, height and descent in pixels of each non-empty text
/// (`TickLabels.get_texts_widths_heights_descents`).
fn axis_text_metrics(artist: AxisTextsArtist) -> PyResult<Vec<(f64, f64, f64)>> {
    artist
        .texts_widths_heights_descents()
        .map_err(|err| PyErr::value_error(err.to_string()))
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
    module.add("set_axis_texts", set_axis_texts)?;
    module.add("axis_text_extents", axis_text_extents)?;
    module.add("axis_text_metrics", axis_text_metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};
    use pltrs::figure::Figure;

    /// A look of texts as Python describes it.
    const LOOK: &str = "look = ((['serif'], ('italic', 'normal'), ('bold', 'normal', 10.0), \
                        ('dejavusans', None)), 'right', 1.5, 'anchor', False, None)";

    #[test]
    fn descriptions_cross_whole() {
        with_python(|py| {
            let description = eval(
                py,
                &format!(
                    "{LOOK}; d = ('ticklabels', 0.0, [((0.2, 0.5), -90.0, '0.2')], \
                     (10.0, 'black', 10.0, look, 90.0, ('center', 'right'), 'left'))"
                ),
                "d",
            );
            let texts = PyAxisTexts::from_py(&description).expect("a description").0;
            assert_eq!(texts.kind, AxisTextsKind::TickLabels);
            assert_eq!(
                (texts.valign, texts.halign),
                (VAlign::Center, HAlign::Right)
            );
            assert_eq!(texts.look.font.families, ["serif"]);
            assert_eq!(
                (texts.look.multialign, texts.look.linespacing),
                (Some(HAlign::Right), LineSpacing::Factor(1.5))
            );
            assert_eq!(
                (texts.look.antialiased, texts.look.parse_math),
                (Some(false), None)
            );
            let back = PyTextLook(texts.look.clone()).into_py(py).expect("a tuple");
            let again = PyTextLook::from_py(&back).expect("the same look").0;
            assert_eq!(again, texts.look);
            let ax = Figure::new().subplot();
            let artist = add_axis_texts(ax, PyAxisTexts(texts.clone())).expect("added");
            set_axis_texts(artist.clone(), PyAxisTexts(AxisTexts { pad: 4.0, ..texts }))
                .expect("set");
            assert_eq!(artist.texts().pad, 4.0);
            let bad = eval(
                py,
                &format!(
                    "{LOOK}; b = ('nonsense', 0.0, [], (1.0, 'k', 1.0, look, 0.0, ('top', 'left'), 'left'))"
                ),
                "b",
            );
            assert!(PyAxisTexts::from_py(&bad).is_err());
        });
    }
}
