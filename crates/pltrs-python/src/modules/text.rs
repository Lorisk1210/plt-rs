//! `_native.text`: texts (matplotlib's `text`: titles, labels,
//! annotations).
//!
//! Alignments and rotation modes cross as matplotlib's names, which the
//! Python class has already checked; font properties as the tuple of
//! `font_manager` ([`FontProps`]).

use std::str::FromStr;

use pltrs::axes::Axes;
use pltrs::colors::Color;
use pltrs::figure::Figure;
use pltrs::lines::LineStyle;
use pltrs::patches::{FancyBboxPatch, Patch};
use pltrs::text::{
    Feature, HAlign, Language, LanguageRange, LineSpacing, RotationMode, Text, TextBoxProps, VAlign,
};
use pltrs_core::Point;

use super::font_manager::FontProps;
use super::patch_styles::PyBoxStyle;
use crate::binding::dict_items;
use crate::binding::prelude::*;

mod copy;

artist_handle!(Text);

/// Parses one of matplotlib's names, raising `ValueError` with pltrs
/// (Rust)'s reason.
pub(in crate::modules) fn parse_name<T: FromStr<Err: std::fmt::Display>>(
    name: &str,
) -> PyResult<T> {
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

/// The text `value` at `(x, y)`, outside any figure (`Text(x, y, text)`),
/// for an axes or a figure to adopt (`_native.artist.adopt_into_axes`).
fn new(x: f64, y: f64, value: String) -> PyResult<Text> {
    Ok(Text::loose(x, y, value))
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

/// The rotation in degrees: as set, or as drawn when the transform
/// rotates the text (matplotlib's `get_rotation`).
fn get_rotation(text: Text) -> PyResult<f64> {
    Ok(if text.transform_rotates_text() {
        text.drawn_rotation()
    } else {
        text.rotation()
    })
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

/// The name of the rotation mode `mode`.
pub(super) fn rotation_mode_name(mode: RotationMode) -> &'static str {
    match mode {
        RotationMode::Default => "default",
        RotationMode::Anchor => "anchor",
        RotationMode::XTick => "xtick",
        RotationMode::YTick => "ytick",
    }
}

/// The rotation mode's name.
fn get_rotation_mode(text: Text) -> PyResult<&'static str> {
    Ok(rotation_mode_name(text.reported_rotation_mode()))
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

/// Sets the font properties (a font file they name is opened when the
/// text is measured, as matplotlib opens it).
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
                    props.boxstyle = Some(value.extract::<PyBoxStyle>()?.0);
                }
                "pad" => props.pad = Some(value.extract()?),
                "mutation_aspect" => props.mutation_aspect = Some(value.extract()?),
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
                "visible" => patch.base.visible = Some(value.extract()?),
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

/// The box of the text as a loose fancy box patch of its look, `None`
/// without a box (matplotlib's `get_bbox_patch`; see
/// `TextBoxPatch::to_patch`).
fn bbox_patch(text: Text) -> PyResult<Option<Patch<FancyBboxPatch>>> {
    Ok(text.bbox_patch().map(|bbox| bbox.to_patch()))
}

/// Gives the box of the text the look of `patch`, a patch `bbox_patch`
/// made and the program changed.
fn restyle_bbox(text: Text, patch: Patch<FancyBboxPatch>) -> PyResult<()> {
    text.restyle_bbox(&patch);
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

/// Whether the text is where its axes' scales can show it
/// (matplotlib's `_point_in_data_domain` of its position).
fn in_data_domain(text: Text) -> PyResult<bool> {
    Ok(text.in_data_domain())
}

/// A language tag with the characters `[start, end)` it covers.
type LanguageSpan = (String, usize, usize);

/// The text's language as `(tag, ranges)`: one BCP 47 tag for the whole
/// text, or its `(tag, start, end)` ranges, or neither (`get_language`).
fn get_language(text: Text) -> PyResult<(Option<String>, Vec<LanguageSpan>)> {
    Ok(match text.language() {
        Some(Language::Tag(tag)) => (Some(tag.as_str().to_owned()), Vec::new()),
        Some(Language::Ranges(ranges)) => (
            None,
            ranges
                .into_iter()
                .map(|range| (range.tag.as_str().to_owned(), range.start, range.end))
                .collect(),
        ),
        None => (None, Vec::new()),
    })
}

/// The text with the line breaks wrapping inserts.
fn get_wrapped_text(text: Text) -> PyResult<String> {
    Ok(text.wrapped_text()?)
}

/// A laid-out line: `(line, (width, ascent, descent), (x, y))`.
type LaidOutLine = (String, (f64, f64, f64), (f64, f64));

/// The layout of a text as matplotlib's `Text._get_layout` gives it, in
/// pixels relative to the text's position: the extents of the rotated
/// block, the lines with their metrics and baseline origins, and the
/// text-aligned frame `((x, y), (width, height))`.
type TextLayout = (
    (f64, f64, f64, f64),
    Vec<LaidOutLine>,
    ((f64, f64), (f64, f64)),
);

/// The layout of the text (`Text._get_layout`).
fn get_layout(text: Text) -> PyResult<TextLayout> {
    let laid_out = text.layout()?;
    let block = laid_out.block;
    let lines = laid_out
        .lines
        .into_iter()
        .zip(&block.lines)
        .map(|(line, placed)| {
            (
                line,
                (placed.width, placed.ascent, placed.descent),
                (placed.origin.x, placed.origin.y),
            )
        })
        .collect();
    let (corner, size) = block.frame;
    Ok((block.bbox.extents(), lines, ((corner.x, corner.y), size)))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("figure_text", figure_text)?;
    module.add("axes_text", axes_text)?;
    module.add("axes_texts", axes_texts)?;
    module.add("new", new)?;
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
    module.add("set_bbox", set_bbox)?;
    module.add("has_bbox", has_bbox)?;
    module.add("bbox_patch", bbox_patch)?;
    module.add("restyle_bbox", restyle_bbox)?;
    module.add("set_backgroundcolor", set_backgroundcolor)?;
    module.add("set_fontfeatures", set_fontfeatures)?;
    module.add("set_language", set_language)?;
    module.add("get_language", get_language)?;
    module.add("in_data_domain", in_data_domain)?;
    module.add("get_wrapped_text", get_wrapped_text)?;
    module.add("get_layout", get_layout)?;
    copy::register(module)
}
