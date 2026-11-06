//! Extra validation rules referenced by the parameter table.

use std::str::FromStr;

use pltrs_text::mathtext::MathFallback;

/// matplotlib's complaint about `text` not being one of `allowed`
/// (`ValidateInStrings`, quoting as Python's `repr` does).
fn not_one_of(text: &str, allowed: &[&str]) -> String {
    let allowed: Vec<String> = allowed.iter().map(|a| python_repr(a)).collect();
    format!(
        "{} is not a valid value; supported values are [{}]",
        python_repr(text),
        allowed.join(", ")
    )
}

/// `text` as Python's `repr` writes a string: in single quotes, or in
/// double quotes when it holds a single quote and no double quote.
pub(super) fn python_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut repr = String::from(quote);
    for c in text.chars() {
        match c {
            '\\' => repr.push_str("\\\\"),
            '\n' => repr.push_str("\\n"),
            '\r' => repr.push_str("\\r"),
            '\t' => repr.push_str("\\t"),
            c if c == quote => {
                repr.push('\\');
                repr.push(c);
            }
            c => repr.push(c),
        }
    }
    repr.push(quote);
    repr
}

/// Parses exactly one of `allowed` into the keyword type `T` (matplotlib's
/// `ValidateInStrings`): `allowed` lists the spellings matplotlib accepts
/// for the parameter, which may be fewer than `T` parses.
pub(crate) fn choice<T: FromStr>(
    allowed: &'static [&'static str],
) -> impl Fn(&str) -> Result<T, String> {
    move |text| parse_choice(allowed.iter().find(|a| **a == text), text, allowed)
}

/// Parses one of `allowed` in any ASCII case into the keyword type `T`
/// (matplotlib's `ValidateInStrings(..., ignorecase=True)`).
pub(crate) fn choice_ignore_case<T: FromStr>(
    allowed: &'static [&'static str],
) -> impl Fn(&str) -> Result<T, String> {
    move |text| {
        let found = allowed.iter().find(|a| a.eq_ignore_ascii_case(text));
        parse_choice(found, text, allowed)
    }
}

/// The keyword `found` among `allowed` for `text`, parsed.
fn parse_choice<T: FromStr>(
    found: Option<&&str>,
    text: &str,
    allowed: &[&str],
) -> Result<T, String> {
    found
        .and_then(|spelling| spelling.parse().ok())
        .ok_or_else(|| not_one_of(text, allowed))
}

/// `mathtext.fallback`: `cm`, `stix`, `stixsans` or `none`, in any case
/// (matplotlib's `_validate_mathtext_fallback`).
pub(crate) fn mathtext_fallback(text: &str) -> Result<Option<MathFallback>, String> {
    if text.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    text.parse()
        .map(Some)
        .map_err(|_| format!("{text:?} is not a valid fallback font name"))
}

/// Accepts what numpy's `datetime64` parses, `NaT` included (matplotlib's
/// `_validate_date`).
pub(crate) fn numpy_datetime(value: &str) -> Result<(), String> {
    crate::dates::Datetime64::parse(value)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

/// `path.sketch`: `none` in any case, or three numbers, optionally in
/// parentheses (matplotlib's `validate_sketch`).
pub(crate) fn sketch(text: &str) -> Result<Option<[f64; 3]>, String> {
    let lower = text.trim().to_ascii_lowercase();
    let inner = lower
        .strip_prefix('(')
        .and_then(|t| t.strip_suffix(')'))
        .unwrap_or(&lower);
    if inner == "none" {
        return Ok(None);
    }
    <[f64; 3] as super::value::RcValue>::parse_rc(inner)
        .map(Some)
        .map_err(|_| format!("expected a (scale, length, randomness) tuple, not {text:?}"))
}

/// Accepts values in `[0, 1]`.
pub(crate) fn unit_interval() -> impl Fn(&f64) -> Result<(), String> {
    |value| {
        (0.0..=1.0)
            .contains(value)
            .then_some(())
            .ok_or_else(|| format!("{value} must be between 0 and 1"))
    }
}

/// Accepts values greater than -0.5 (axes margins).
pub(crate) fn above_minus_half() -> impl Fn(&f64) -> Result<(), String> {
    |value| {
        (*value > -0.5)
            .then_some(())
            .ok_or_else(|| format!("{value} must be greater than -0.5"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_quoted_as_python_quotes_them() {
        assert_eq!(python_repr("auto"), "'auto'");
        assert_eq!(python_repr("it's"), "\"it's\"");
        assert_eq!(python_repr("'\""), "'\\'\"'");
        assert_eq!(python_repr("a\\b\n"), "'a\\\\b\\n'");
        assert_eq!(python_repr(""), "''");
    }

    #[test]
    fn choices_parse_matplotlib_spellings_only() {
        let stages = choice::<crate::image::InterpolationStage>(&["auto", "data", "rgba"]);
        assert_eq!(stages("rgba"), Ok(crate::image::InterpolationStage::Rgba));
        assert_eq!(
            stages("antialiased"),
            Err(
                "'antialiased' is not a valid value; supported values are ['auto', 'data', 'rgba']"
                    .to_owned()
            ),
            "the type parses more names than matplotlib allows here"
        );
        let papers = choice_ignore_case::<pltrs_render::Paper>(&["a0", "letter"]);
        assert_eq!(
            papers("A0").map(|paper| paper.to_string()),
            Ok("a0".to_owned())
        );
        assert!(papers("a11").is_err());
        let engines = choice::<pltrs_text::tex::TexEngine>(&["latex", "latex+dvipng"]);
        assert_eq!(
            engines("latex+dvipng"),
            Ok(pltrs_text::tex::TexEngine::LatexDvipng)
        );
        assert!(engines("Latex").is_err());
    }

    #[test]
    fn fallbacks_and_dates() {
        assert_eq!(mathtext_fallback("STIX"), Ok(Some(MathFallback::Stix)));
        assert_eq!(mathtext_fallback("None"), Ok(None));
        assert_eq!(
            mathtext_fallback("comic"),
            Err("\"comic\" is not a valid fallback font name".to_owned())
        );
        assert!(numpy_datetime("1970-01-01T00:00").is_ok());
        assert!(numpy_datetime("bogus").is_err());
    }

    #[test]
    fn sketch_takes_none_or_three_numbers() {
        assert_eq!(sketch("none"), Ok(None));
        assert_eq!(sketch("(None)"), Ok(None));
        assert_eq!(sketch("(1, 100, 2)"), Ok(Some([1.0, 100.0, 2.0])));
        assert_eq!(sketch("1, 100, 2"), Ok(Some([1.0, 100.0, 2.0])));
        assert!(sketch("[1, 100, 2]").is_err());
        assert!(sketch("1, 2").is_err());
    }
}
