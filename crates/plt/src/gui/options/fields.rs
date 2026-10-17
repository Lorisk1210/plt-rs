//! Form rows of the figure options dialog and the conversions between
//! artist properties and the text the fields show.

use plt_core::Rgba;
use plt_ui::{Choice, Flex, Label, TextInput, UiWindow, Widget, WidgetId};

use super::OptionsMessage;
use crate::markers::{Marker, MarkerKey, MarkerSymbol};
use crate::ticker::pyfmt;

/// Width of the field names, in logical pixels.
const NAME_WIDTH: f64 = 140.0;

/// A form being built: a column of rows, each a name and a field.
pub(super) struct Form {
    column: Flex<OptionsMessage>,
}

impl Form {
    /// An empty form.
    pub(super) fn new() -> Self {
        Self {
            column: Flex::column().with_spacing(4.0),
        }
    }

    /// Adds a heading.
    pub(super) fn heading(&mut self, text: &str) {
        self.column.push(Label::new(text).bold(), 0.0);
    }

    /// Adds a row named `name` holding `field`; returns the field's id.
    pub(super) fn row(&mut self, name: &str, field: impl Widget<OptionsMessage>) -> WidgetId {
        let mut row = Flex::row().with_spacing(8.0);
        row.push(Label::new(name).with_min_width(NAME_WIDTH), 0.0);
        let id = row.push(field, 1.0);
        self.column.push(row, 0.0);
        id
    }

    /// Adds a text field showing `text`.
    pub(super) fn text(&mut self, name: &str, text: impl Into<String>) -> WidgetId {
        self.row(name, TextInput::new(text, |_| OptionsMessage::Edited))
    }

    /// Adds segments for `options` with `chosen` selected.
    pub(super) fn choice(&mut self, name: &str, options: &[&str], chosen: usize) -> WidgetId {
        let mut choice = Choice::new(
            options.iter().map(|&option| option.to_owned()).collect(),
            |_| OptionsMessage::Edited,
        );
        choice.set_chosen(chosen);
        self.row(name, choice)
    }

    /// Adds any widget on its own line.
    pub(super) fn push(&mut self, widget: impl Widget<OptionsMessage>) -> WidgetId {
        self.column.push(widget, 0.0)
    }

    /// The finished column.
    pub(super) fn finish(self) -> Flex<OptionsMessage> {
        self.column
    }
}

/// Why the form's values cannot be applied: the field and the problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FieldError(pub(super) String);

/// The text of text field `id`.
pub(super) fn text_of(window: &mut UiWindow<OptionsMessage>, id: WidgetId) -> String {
    window
        .ui_mut()
        .widget_mut::<TextInput<OptionsMessage>>(id)
        .map_or_else(String::new, |input| input.text().to_owned())
}

/// The chosen segment of choice `id`.
pub(super) fn chosen(window: &mut UiWindow<OptionsMessage>, id: WidgetId) -> usize {
    window
        .ui_mut()
        .widget_mut::<Choice<OptionsMessage>>(id)
        .map_or(0, |choice| choice.chosen())
}

/// `x` as Python prints a float, the way matplotlib's form shows numbers.
pub(super) fn number_text(x: f64) -> String {
    pyfmt::repr(x)
}

/// The number typed into the field called `name`.
pub(super) fn parse_number(name: &str, text: &str) -> Result<f64, FieldError> {
    text.trim()
        .parse::<f64>()
        .map_err(|_| FieldError(format!("{name}: {text:?} is not a number")))
}

/// matplotlib's `to_hex(c, keep_alpha=True)`: `#rrggbbaa`.
pub(super) fn hex(color: Rgba) -> String {
    let channel = |v: f64| {
        // Python's `round` rounds half to even.
        let scaled = (v.clamp(0.0, 1.0) * 255.0).round_ties_even();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a value in 0..=255"
        )]
        let byte = scaled as u8;
        byte
    };
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        channel(color.r),
        channel(color.g),
        channel(color.b),
        channel(color.a)
    )
}

/// The colour typed into the field called `name`.
pub(super) fn parse_color(name: &str, text: &str) -> Result<crate::colors::Color, FieldError> {
    let color = crate::colors::Color::from(text.trim());
    color
        .resolve()
        .map(|_| color)
        .map_err(|_| FieldError(format!("{name}: {text:?} is not a colour")))
}

/// matplotlib's code for `marker` (`"o"`, `"None"`, `"$x$"`, ...), or an
/// empty text for markers without one.
pub(super) fn marker_text(marker: &Marker) -> String {
    match marker {
        Marker::Symbol(MarkerSymbol::Nothing) => "None".to_owned(),
        Marker::Symbol(symbol) => {
            MarkerSymbol::all()
                .find(|(_, s)| s == symbol)
                .map_or_else(String::new, |(key, _)| match key {
                    MarkerKey::Char(code) => code.to_owned(),
                    MarkerKey::Code(code) => code.to_string(),
                })
        }
        Marker::Math(text) => text.clone(),
        Marker::Polygon { .. } | Marker::Vertices(_) | Marker::Path(_) => String::new(),
    }
}

/// The marker typed into the field called `name`: a code, an integer code
/// or mathtext.
pub(super) fn parse_marker(name: &str, text: &str) -> Result<Marker, FieldError> {
    let text = text.trim();
    let invalid = || FieldError(format!("{name}: {text:?} is not a marker"));
    match text.parse::<i64>() {
        Ok(code) => MarkerSymbol::from_code(code)
            .map(Marker::Symbol)
            .ok_or_else(invalid),
        Err(_) => text.parse().map_err(|_| invalid()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_follow_matplotlib() {
        assert_eq!(
            hex(Rgba::new(0.121_568_6, 0.466_666_7, 0.705_882_4, 1.0)),
            "#1f77b4ff"
        );
        assert_eq!(number_text(2.0), "2.0");
        assert_eq!(parse_number("Min", " 1e3 "), Ok(1000.0));
        assert!(parse_number("Min", "abc").is_err());
        assert!(parse_color("Color", "#ff000080").is_ok());
        assert!(parse_color("Color", "nope").is_err());
        assert_eq!(marker_text(&Marker::Symbol(MarkerSymbol::Circle)), "o");
        assert_eq!(marker_text(&Marker::Symbol(MarkerSymbol::CaretDown)), "7");
        assert_eq!(marker_text(&Marker::Symbol(MarkerSymbol::Nothing)), "None");
        assert_eq!(
            parse_marker("Style", "4"),
            Ok(Marker::Symbol(MarkerSymbol::CaretLeft))
        );
        assert_eq!(
            parse_marker("Style", "s"),
            Ok(Marker::Symbol(MarkerSymbol::Square))
        );
        assert!(parse_marker("Style", "??").is_err());
    }
}
