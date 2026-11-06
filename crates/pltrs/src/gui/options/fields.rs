//! Form rows of the figure options dialog and the conversions between
//! artist properties and the text the fields show.

use pltrs_core::Rgba;
use pltrs_ui::{
    Choice, Dropdown, Flex, Label, Spacing, Swatch, TextInput, UiWindow, Widget, WidgetId,
};

use super::OptionsMessage;
use crate::dates::iso::fromisoformat;
use crate::dates::{TimeZone, ZonedDateTime, date2num, num2date};
use crate::markers::{Marker, MarkerKey, MarkerSymbol};
use crate::ticker::{g_sig_digits, pyfmt};

/// The most significant digits a double needs to keep its value.
const DOUBLE_DIGITS: usize = 17;

/// The space between the fields of a row holding two.
const PAIR_GAP: Spacing = Spacing::Md;

/// A form being built: sections of rows, each a name and a field.
pub(super) struct Form {
    form: pltrs_ui::Form<OptionsMessage>,
}

impl Form {
    /// An empty form.
    pub(super) fn new() -> Self {
        Self {
            form: pltrs_ui::Form::new(),
        }
    }

    /// Starts a section: the next rows are grouped under `title` (none
    /// when empty).
    pub(super) fn section(&mut self, title: &str) {
        self.form.section(title);
    }

    /// Adds a text field showing `text`.
    pub(super) fn text(&mut self, name: &str, text: impl Into<String>) -> WidgetId {
        self.form.row(name, edited_text(text))
    }

    /// Adds two text fields on one row, `name` showing `text` and
    /// `second_name` showing `second_text`; returns both ids.
    pub(super) fn pair(
        &mut self,
        (name, text): (&str, String),
        (second_name, second_text): (&str, String),
    ) -> (WidgetId, WidgetId) {
        let mut row = Flex::row().with_spacing(PAIR_GAP);
        let first = row.push(edited_text(text), 1.0);
        row.push(Label::new(second_name), 0.0);
        let second = row.push(edited_text(second_text), 1.0);
        self.form.row(name, row);
        (first, second)
    }

    /// Adds a text field for a colour showing `text`, after a swatch of
    /// the colour typed.
    pub(super) fn color(&mut self, name: &str, text: String) -> WidgetId {
        let mut row = Flex::row().with_spacing(Spacing::Sm);
        let swatch = row.push(Swatch::new(swatch_color(&text)), 0.0);
        let input = TextInput::new(text, move |text| OptionsMessage::Color { swatch, text });
        let id = row.push(input, 1.0);
        self.form.row(name, row);
        id
    }

    /// Adds segments for `options` with `chosen` selected.
    pub(super) fn choice(&mut self, name: &str, options: &[&str], chosen: usize) -> WidgetId {
        let mut choice = Choice::new(
            options.iter().map(|&option| option.to_owned()).collect(),
            |_| OptionsMessage::Edited,
        );
        choice.set_chosen(chosen);
        self.form.row(name, choice)
    }

    /// Adds a drop-down list of `options` with `chosen` chosen, for lists
    /// too long for segments (matplotlib's form shows every list as a
    /// combo box).
    pub(super) fn dropdown(&mut self, name: &str, options: &[&str], chosen: usize) -> WidgetId {
        let mut dropdown = Dropdown::new(
            options.iter().map(|&option| option.to_owned()).collect(),
            |_| OptionsMessage::Edited,
        );
        dropdown.set_chosen(chosen);
        self.form.row(name, dropdown)
    }

    /// Adds any widget across the row.
    pub(super) fn wide(&mut self, widget: impl Widget<OptionsMessage>) -> WidgetId {
        self.form.wide_row(widget)
    }

    /// The finished form.
    pub(super) fn finish(self) -> pltrs_ui::Form<OptionsMessage> {
        self.form
    }
}

/// A text field showing `text`, read when the dialog applies.
fn edited_text(text: impl Into<String>) -> TextInput<OptionsMessage> {
    TextInput::new(text, |_| OptionsMessage::Edited)
}

/// The colour a swatch shows for `text`, if it is a colour.
pub(super) fn swatch_color(text: &str) -> Option<Rgba> {
    parse_color("", text).ok()?.resolve().ok()
}

/// Why the form's values cannot be applied: the field and the problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FieldError {
    /// The text field holding the value refused.
    pub(super) field: WidgetId,
    /// What is wrong, naming the field.
    pub(super) message: String,
}

/// The text of text field `id`.
pub(super) fn text_of(window: &mut UiWindow<OptionsMessage>, id: WidgetId) -> String {
    window
        .ui_mut()
        .widget_mut::<TextInput<OptionsMessage>>(id)
        .map_or_else(String::new, |input| input.text().to_owned())
}

/// The value of text field `id` as `parse` reads its text, or what is
/// wrong with it.
pub(super) fn read_field<T>(
    window: &mut UiWindow<OptionsMessage>,
    id: WidgetId,
    parse: impl FnOnce(&str) -> Result<T, String>,
) -> Result<T, FieldError> {
    parse(&text_of(window, id)).map_err(|message| FieldError { field: id, message })
}

/// The chosen segment of choice `id`.
pub(super) fn chosen(window: &mut UiWindow<OptionsMessage>, id: WidgetId) -> usize {
    let ui = window.ui_mut();
    match ui.widget_mut::<Choice<OptionsMessage>>(id) {
        Some(choice) => choice.chosen(),
        None => ui
            .widget_mut::<Dropdown<OptionsMessage>>(id)
            .map_or(0, |dropdown| dropdown.chosen()),
    }
}

/// `x` as Python prints a float, the way matplotlib's form shows numbers.
pub(super) fn number_text(x: f64) -> String {
    pyfmt::repr(x)
}

/// Axis limit `x` rounded to the digits that tell it from the values
/// `resolution` away (a screen pixel along its axis), as matplotlib's
/// status bar rounds coordinates (`cbook._g_sig_digits`), then shown as
/// [`number_text`] shows numbers: `-0.49` rather than
/// `-0.49000000000000005`. Without a usable resolution, `x` is shown as
/// it is.
pub(super) fn limit_text(x: f64, resolution: Option<f64>) -> String {
    let rounded = resolution
        .filter(|_| x.is_finite())
        .and_then(|delta| g_sig_digits(x, delta).ok())
        .and_then(|digits| {
            let digits = digits.clamp(1, DOUBLE_DIGITS);
            format!("{x:.*e}", digits - 1).parse::<f64>().ok()
        });
    number_text(rounded.unwrap_or(x))
}

/// The number typed into the field called `name`.
pub(super) fn parse_number(name: &str, text: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .map_err(|_| format!("{name}: {text:?} is not a number"))
}

/// The axis limit typed into the field called `name`: a finite number.
pub(super) fn parse_limit(name: &str, text: &str) -> Result<f64, String> {
    let limit = parse_number(name, text)?;
    if limit.is_finite() {
        Ok(limit)
    } else {
        Err(format!("{name}: {text:?} is not a finite number"))
    }
}

/// How limits on axes of dates show: the date and time, to the second.
const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// The zone dates show in: the `timezone` rcParam, resolved as
/// matplotlib's date formatting resolves it (`dates._get_tzinfo`), or UTC
/// when the system knows no such zone.
pub(super) fn date_zone() -> TimeZone {
    TimeZone::named(&crate::rc::params().timezone).unwrap_or_default()
}

/// Date number `x` as a date in `zone`, in [`DATE_FORMAT`], if it is a
/// date.
pub(super) fn date_text(x: f64, zone: &TimeZone) -> Option<String> {
    num2date(x, zone)
        .ok()
        .map(|date| date.strftime(DATE_FORMAT))
}

/// The date typed into the field called `name`, as a date number: ISO
/// 8601 (the time may be left out), in `zone` unless it gives an offset.
pub(super) fn parse_date(name: &str, text: &str, zone: &TimeZone) -> Result<f64, String> {
    let invalid = |_| format!("{name}: {text:?} is not a date (YYYY-MM-DD HH:MM:SS)");
    let (local, offset) = fromisoformat(text.trim()).map_err(invalid)?;
    let zone = offset.map_or_else(|| zone.clone(), TimeZone::Fixed);
    date2num(&ZonedDateTime::from_local(local, zone)).map_err(invalid)
}

/// The colour typed into the field called `name`.
pub(super) fn parse_color(name: &str, text: &str) -> Result<crate::colors::Color, String> {
    let color = crate::colors::Color::from(text.trim());
    color
        .resolve()
        .map(|_| color)
        .map_err(|_| format!("{name}: {text:?} is not a colour"))
}

/// matplotlib's code for `marker` (`"o"`, `"None"`, `"$x$"`, ...), or an
/// empty text for markers without one.
pub(super) fn marker_text(marker: &Marker) -> String {
    match marker {
        Marker::Symbol(MarkerSymbol::Nothing(_)) => "None".to_owned(),
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
pub(super) fn parse_marker(name: &str, text: &str) -> Result<Marker, String> {
    let text = text.trim();
    let invalid = || format!("{name}: {text:?} is not a marker");
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
    fn limits_are_rounded_at_the_axis_resolution() {
        assert_eq!(limit_text(-0.490_000_000_000_000_05, Some(0.02)), "-0.49");
        assert_eq!(limit_text(10.290_000_000_000_001, Some(0.02)), "10.29");
        assert_eq!(limit_text(-0.05, Some(0.002)), "-0.05");
        assert_eq!(limit_text(1234.5, Some(10.0)), "1230.0");
        assert_eq!(limit_text(0.0, Some(0.01)), "0.0");
        assert_eq!(limit_text(1e-3, Some(0.01)), "0.001", "one digit at least");
        assert_eq!(limit_text(0.1 + 0.2, Some(0.0)), "0.30000000000000004");
        assert_eq!(limit_text(0.1 + 0.2, None), "0.30000000000000004");
        assert_eq!(limit_text(0.1 + 0.2, Some(f64::NAN)), "0.30000000000000004");
    }

    #[test]
    fn conversions_follow_matplotlib() {
        assert_eq!(number_text(2.0), "2.0");
        assert_eq!(parse_number("Min", " 1e3 "), Ok(1000.0));
        assert!(parse_number("Min", "abc").is_err());
        assert_eq!(parse_limit("Min", "-2.5"), Ok(-2.5));
        assert!(parse_limit("Min", "nan").is_err());
        assert!(parse_color("Color", "#ff000080").is_ok());
        assert!(parse_color("Color", "nope").is_err());
        assert_eq!(swatch_color("red"), Some(Rgba::new(1.0, 0.0, 0.0, 1.0)));
        assert_eq!(swatch_color("#ff000080").map(|c| c.a), Some(128.0 / 255.0));
        assert_eq!(swatch_color("nope"), None);
        assert_eq!(marker_text(&Marker::Symbol(MarkerSymbol::Circle)), "o");
        assert_eq!(marker_text(&Marker::Symbol(MarkerSymbol::CaretDown)), "7");
        assert_eq!(marker_text(&Marker::default()), "None");
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

    #[test]
    fn dates_show_and_read_back_in_their_zone() {
        let utc = TimeZone::Utc;
        assert_eq!(
            date_text(18_262.5, &utc).as_deref(),
            Some("2020-01-01 12:00:00")
        );
        assert_eq!(
            parse_date("Min", " 2020-01-01 12:00:00 ", &utc),
            Ok(18_262.5)
        );
        assert_eq!(parse_date("Min", "2020-01-01", &utc), Ok(18_262.0));
        assert_eq!(
            parse_date("X min", "soon", &utc),
            Err("X min: \"soon\" is not a date (YYYY-MM-DD HH:MM:SS)".to_owned())
        );
        assert_eq!(date_text(f64::NAN, &utc), None);
        let east = TimeZone::Fixed(crate::dates::UtcOffset::from_seconds(2 * 3600).unwrap());
        assert_eq!(
            date_text(18_262.5, &east).as_deref(),
            Some("2020-01-01 14:00:00")
        );
        assert_eq!(
            parse_date("Min", "2020-01-01 14:00:00", &east),
            Ok(18_262.5)
        );
        assert_eq!(
            parse_date("Min", "2020-01-01T12:00:00+00:00", &east),
            Ok(18_262.5),
            "an offset typed wins"
        );
    }

    #[test]
    fn dates_take_the_timezone_rc_param() {
        let zone = |name: &str| {
            crate::rc::context(|| {
                crate::rc::set("timezone", name).unwrap();
                date_zone()
            })
        };
        assert_eq!(zone("UTC"), TimeZone::Utc);
        assert_eq!(zone("Nowhere/Atlantis"), TimeZone::Utc, "unknown zones");
        if let Ok(paris) = TimeZone::named("Europe/Paris") {
            assert_eq!(zone("Europe/Paris"), paris);
        }
    }

    #[test]
    fn colour_fields_tell_their_swatch_what_is_typed() {
        use pltrs_platform::{ButtonState, Key, KeyEvent, Modifiers, PhysicalSize, WindowEvent};
        use pltrs_ui::Ui;

        let mut form = Form::new();
        let field = form.color("Color (RGBA)", "#ff0000ff".to_owned());
        let mut ui = Ui::new(form.finish(), PhysicalSize::new(400, 60), 1.0).unwrap();
        ui.set_focus(Some(field));
        let key = WindowEvent::Key(KeyEvent {
            key: Key::Backspace,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers: Modifiers::NONE,
            scancode: 22,
        });
        let messages = ui.handle(&key).unwrap();
        let [OptionsMessage::Color { swatch, text }] = messages.as_slice() else {
            panic!("one colour message, not {messages:?}");
        };
        assert_eq!(text, "#ff0000f");
        let shown = ui
            .widget_mut::<Swatch>(*swatch)
            .map(|swatch| swatch.color());
        assert_eq!(
            shown,
            Some(swatch_color("#ff0000ff")),
            "the swatch beside the field"
        );
    }
}
