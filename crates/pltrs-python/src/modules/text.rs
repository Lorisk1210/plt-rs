//! `_native.text`: texts (matplotlib's `text`: titles, labels,
//! annotations).
//!
//! Alignments and rotation modes cross as matplotlib's names, which the
//! Python class has already checked; font properties as the tuple of
//! `font_manager` ([`FontProps`]).

use std::str::FromStr;

use pltrs::artist::ArtistHandle;
use pltrs::axes::Axes;
use pltrs::colors::Color;
use pltrs::figure::Figure;
use pltrs::lines::LineStyle;
use pltrs::patches::BoxStyle;
use pltrs::text::{
    Feature, HAlign, Language, LanguageRange, LineSpacing, RotationMode, Text, TextBoxProps, VAlign,
};
use pltrs_core::Point;

use super::font_manager::FontProps;
use crate::binding::dict_items;
use crate::binding::prelude::*;

artist_handle!(Text);

/// Parses one of matplotlib's names, raising `ValueError` with pltrs
/// (Rust)'s reason.
fn parse_name<T: FromStr<Err: std::fmt::Display>>(name: &str) -> PyResult<T> {
    name.parse()
        .map_err(|err: T::Err| PyErr::value_error(err.to_string()))
}

/// `Figure.text(x, y, s)`: a text at `(x, y)` in figure coordinates.
fn figure_text(figure: Figure, x: f64, y: f64, s: String) -> PyResult<Text> {
    Ok(figure.text(x, y, s))
}

/// `Axes.text(x, y, s)`: a text at `(x, y)` in data coordinates.
fn axes_text(axes: Axes, x: f64, y: f64, s: String) -> PyResult<Text> {
    Ok(axes.text(x, y, s))
}

/// `Axes.texts`: the texts placed in the axes with `text` and
/// `annotate`, in the order they were made.
fn axes_texts(axes: Axes) -> PyResult<Vec<Text>> {
    Ok(axes.texts())
}

/// The string shown.
fn get_text(text: Text) -> PyResult<String> {
    Ok(text.text())
}

/// Sets the string shown.
fn set_text(text: Text, value: String) -> PyResult<()> {
    text.set_text(value);
    Ok(())
}

/// The position in the text's coordinates.
fn get_position(text: Text) -> PyResult<(f64, f64)> {
    let position = text.position();
    Ok((position.x, position.y))
}

/// Moves the text.
fn set_position(text: Text, x: f64, y: f64) -> PyResult<()> {
    text.set_position(Point::new(x, y));
    Ok(())
}

/// The colour.
fn get_color(text: Text) -> PyResult<Color> {
    Ok(text.color())
}

/// Sets the colour.
fn set_color(text: Text, color: Color) -> PyResult<()> {
    text.set_color(color);
    Ok(())
}

/// The rotation as set, in degrees.
fn get_rotation(text: Text) -> PyResult<f64> {
    Ok(text.rotation())
}

/// Sets the rotation in degrees.
fn set_rotation(text: Text, degrees: f64) -> PyResult<()> {
    text.set_rotation(degrees);
    Ok(())
}

/// Whether the transform turns the rotation.
fn get_transform_rotates_text(text: Text) -> PyResult<bool> {
    Ok(text.transform_rotates_text())
}

/// Sets whether the transform turns the rotation.
fn set_transform_rotates_text(text: Text, rotates: bool) -> PyResult<()> {
    text.set_transform_rotates_text(rotates);
    Ok(())
}

/// The horizontal alignment's name.
fn get_horizontalalignment(text: Text) -> PyResult<&'static str> {
    Ok(text.halign().name())
}

/// Sets the horizontal alignment by name.
fn set_horizontalalignment(text: Text, align: String) -> PyResult<()> {
    text.set_halign(parse_name::<HAlign>(&align)?);
    Ok(())
}

/// The vertical alignment's name.
fn get_verticalalignment(text: Text) -> PyResult<&'static str> {
    Ok(text.valign().name())
}

/// Sets the vertical alignment by name.
fn set_verticalalignment(text: Text, align: String) -> PyResult<()> {
    text.set_valign(parse_name::<VAlign>(&align)?);
    Ok(())
}

/// The alignment of the lines, `None` when it follows the horizontal
/// alignment.
fn get_multialignment(text: Text) -> PyResult<Option<&'static str>> {
    Ok(text.multialign().map(HAlign::name))
}

/// Sets the alignment of the lines by name (`None` follows the horizontal
/// alignment).
fn set_multialignment(text: Text, align: Option<String>) -> PyResult<()> {
    text.set_multialign(align.as_deref().map(parse_name::<HAlign>).transpose()?);
    Ok(())
}

/// The rotation mode's name.
fn get_rotation_mode(text: Text) -> PyResult<&'static str> {
    Ok(match text.rotation_mode() {
        RotationMode::Default => "default",
        RotationMode::Anchor => "anchor",
        RotationMode::XTick => "xtick",
        RotationMode::YTick => "ytick",
    })
}

/// Sets the rotation mode by name.
fn set_rotation_mode(text: Text, mode: String) -> PyResult<()> {
    text.set_rotation_mode(parse_name::<RotationMode>(&mode)?);
    Ok(())
}

/// The line spacing as a multiple of the font size, `None` for `normal`.
fn get_linespacing(text: Text) -> PyResult<Option<f64>> {
    Ok(match text.linespacing() {
        LineSpacing::Normal => None,
        LineSpacing::Factor(factor) => Some(factor),
    })
}

/// Sets the line spacing (`None` for `normal`).
fn set_linespacing(text: Text, spacing: Option<f64>) -> PyResult<()> {
    text.set_linespacing(spacing.map_or(LineSpacing::Normal, LineSpacing::Factor));
    Ok(())
}

/// The font properties.
fn get_fontproperties(text: Text) -> PyResult<FontProps> {
    Ok(FontProps(text.font()))
}

/// Sets the font properties.
fn set_fontproperties(text: Text, props: FontProps) -> PyResult<()> {
    text.set_font(props.0);
    Ok(())
}

/// Whether the text is antialiased.
fn get_antialiased(text: Text) -> PyResult<bool> {
    Ok(text.antialiased())
}

/// Sets whether the text is antialiased.
fn set_antialiased(text: Text, antialiased: bool) -> PyResult<()> {
    text.set_antialiased(antialiased);
    Ok(())
}

/// Whether the lines are wrapped to fit in the figure.
fn get_wrap(text: Text) -> PyResult<bool> {
    Ok(text.wrap())
}

/// Sets whether the lines are wrapped to fit in the figure.
fn set_wrap(text: Text, wrap: bool) -> PyResult<()> {
    text.set_wrap(wrap);
    Ok(())
}

/// Whether `$...$` is typeset as math.
fn get_parse_math(text: Text) -> PyResult<bool> {
    Ok(text.parse_math())
}

/// Sets whether `$...$` is typeset as math.
fn set_parse_math(text: Text, parse_math: bool) -> PyResult<()> {
    text.set_parse_math(parse_math);
    Ok(())
}

/// Whether LaTeX typesets the text.
fn get_usetex(text: Text) -> PyResult<bool> {
    Ok(text.usetex())
}

/// Sets whether LaTeX typesets the text.
fn set_usetex(text: Text, usetex: bool) -> PyResult<()> {
    text.set_usetex(usetex);
    Ok(())
}

/// A text's box as `set_bbox` takes it: the `FancyBboxPatch` properties of
/// a dict whose aliases the Python side has resolved (`fc` is
/// `facecolor`).
struct BoxKwargs(TextBoxProps);

impl FromPy<'_> for BoxKwargs {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let mut props = TextBoxProps::default();
        for (key, value) in dict_items(object)? {
            let key: String = key.extract()?;
            let patch = &mut props.patch;
            match key.as_str() {
                "boxstyle" => {
                    props.boxstyle = Some(parse_name::<BoxStyle>(&value.extract::<String>()?)?);
                }
                "pad" => props.pad = Some(value.extract()?),
                "color" => patch.color = Some(value.extract()?),
                "facecolor" => patch.facecolor = Some(value.extract()?),
                "edgecolor" => patch.edgecolor = Some(value.extract()?),
                "linewidth" => patch.linewidth = Some(value.extract()?),
                "linestyle" => {
                    patch.linestyle = Some(parse_name::<LineStyle>(&value.extract::<String>()?)?);
                }
                "fill" => patch.fill = Some(value.extract()?),
                "hatch" => patch.hatch = Some(value.extract()?),
                "antialiased" => patch.antialiased = Some(value.extract()?),
                "alpha" => patch.base.alpha = value.extract()?,
                "zorder" => patch.base.zorder = Some(value.extract()?),
                _ => {
                    return Err(PyErr::new(
                        ExceptionKind::AttributeError,
                        format!("FancyBboxPatch.set() got an unexpected keyword argument '{key}'"),
                    ));
                }
            }
        }
        Ok(Self(props))
    }
}

/// `set_bbox(rectprops)`: draws a box behind the text (`None` removes it).
fn set_bbox(text: Text, props: Option<BoxKwargs>) -> PyResult<()> {
    text.set_bbox(props.map(|props| props.0));
    Ok(())
}

/// Whether the text has a box.
fn has_bbox(text: Text) -> PyResult<bool> {
    Ok(text.has_bbox())
}

/// `set_backgroundcolor(color)`: the face of the text's box, made with
/// this face and edge colour when there is none.
fn set_backgroundcolor(text: Text, color: Color) -> PyResult<()> {
    text.set_backgroundcolor(color);
    Ok(())
}

/// `set_fontfeatures(features)`: the OpenType features of the text, in
/// `HarfBuzz`'s syntax (`None` for none).
fn set_fontfeatures(text: Text, features: Option<Vec<String>>) -> PyResult<()> {
    let features: Vec<Feature> = features
        .unwrap_or_default()
        .iter()
        .map(|feature| parse_name::<Feature>(feature))
        .collect::<PyResult<_>>()?;
    text.set_fontfeatures(features);
    Ok(())
}

/// `set_language(language)`: one BCP 47 tag for the whole text, ranges
/// `(tag, start, end)` of it, or `None` for :rc:`text.language`.
fn set_language(
    text: Text,
    tag: Option<String>,
    ranges: Vec<(String, usize, usize)>,
) -> PyResult<()> {
    let language = match tag {
        Some(tag) => Some(Language::from(tag)),
        None if !ranges.is_empty() => Some(
            ranges
                .into_iter()
                .map(|(tag, start, end)| LanguageRange::new(tag.as_str(), start, end))
                .collect(),
        ),
        None => None,
    };
    text.set_language(language);
    Ok(())
}

/// The text's extent on the canvas as `(x0, y0, x1, y1)` in pixels, `None`
/// when it has none (matplotlib's `get_window_extent`).
fn get_window_extent(text: Text) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(text
        .window_extent()
        .map(|bbox| (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())))
}

/// Whether the display point `(x, y)` is on the text or its box.
fn contains(text: Text, x: f64, y: f64) -> PyResult<bool> {
    Ok(text.contains(Point::new(x, y)).is_some())
}

/// The text with the line breaks wrapping inserts.
fn get_wrapped_text(text: Text) -> PyResult<String> {
    Ok(text.wrapped_text()?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("figure_text", figure_text)?;
    module.add("axes_text", axes_text)?;
    module.add("axes_texts", axes_texts)?;
    module.add("get_text", get_text)?;
    module.add("set_text", set_text)?;
    module.add("get_position", get_position)?;
    module.add("set_position", set_position)?;
    module.add("get_color", get_color)?;
    module.add("set_color", set_color)?;
    module.add("get_rotation", get_rotation)?;
    module.add("set_rotation", set_rotation)?;
    module.add("get_transform_rotates_text", get_transform_rotates_text)?;
    module.add("set_transform_rotates_text", set_transform_rotates_text)?;
    module.add("get_horizontalalignment", get_horizontalalignment)?;
    module.add("set_horizontalalignment", set_horizontalalignment)?;
    module.add("get_verticalalignment", get_verticalalignment)?;
    module.add("set_verticalalignment", set_verticalalignment)?;
    module.add("get_multialignment", get_multialignment)?;
    module.add("set_multialignment", set_multialignment)?;
    module.add("get_rotation_mode", get_rotation_mode)?;
    module.add("set_rotation_mode", set_rotation_mode)?;
    module.add("get_linespacing", get_linespacing)?;
    module.add("set_linespacing", set_linespacing)?;
    module.add("get_fontproperties", get_fontproperties)?;
    module.add("set_fontproperties", set_fontproperties)?;
    module.add("get_antialiased", get_antialiased)?;
    module.add("set_antialiased", set_antialiased)?;
    module.add("get_wrap", get_wrap)?;
    module.add("set_wrap", set_wrap)?;
    module.add("get_parse_math", get_parse_math)?;
    module.add("set_parse_math", set_parse_math)?;
    module.add("get_usetex", get_usetex)?;
    module.add("set_usetex", set_usetex)?;
    module.add("get_window_extent", get_window_extent)?;
    module.add("contains", contains)?;
    module.add("set_bbox", set_bbox)?;
    module.add("has_bbox", has_bbox)?;
    module.add("set_backgroundcolor", set_backgroundcolor)?;
    module.add("set_fontfeatures", set_fontfeatures)?;
    module.add("set_language", set_language)?;
    module.add("get_wrapped_text", get_wrapped_text)
}
