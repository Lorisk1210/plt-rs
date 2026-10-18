//! Extra validation rules referenced by the parameter table.

/// Accepts exactly one of `allowed`.
pub(crate) fn one_of(allowed: &'static [&'static str]) -> impl Fn(&String) -> Result<(), String> {
    move |value| {
        allowed
            .contains(&value.as_str())
            .then_some(())
            .ok_or_else(|| format!("{value:?} is not one of {allowed:?}"))
    }
}

/// Parses one of `allowed` in any ASCII case, keeping `allowed`'s spelling
/// (matplotlib's `ValidateInStrings(..., ignorecase=True)`).
pub(crate) fn one_of_ignore_case(
    allowed: &'static [&'static str],
) -> impl Fn(&str) -> Result<String, String> {
    move |text| {
        allowed
            .iter()
            .find(|a| a.eq_ignore_ascii_case(text))
            .map(|a| (*a).to_owned())
            .ok_or_else(|| format!("{text:?} is not one of {allowed:?}"))
    }
}

/// `mathtext.fallback`: `cm`, `stix`, `stixsans` or `none`, in any case
/// (matplotlib's `_validate_mathtext_fallback`).
pub(crate) fn mathtext_fallback(text: &str) -> Result<Option<String>, String> {
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "none" => Ok(None),
        "cm" | "stix" | "stixsans" => Ok(Some(lower)),
        _ => Err(format!("{text:?} is not a valid fallback font name")),
    }
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
    fn choices_keep_their_spelling() {
        let papers = one_of_ignore_case(&["a0", "letter"]);
        assert_eq!(papers("A0"), Ok("a0".to_owned()));
        assert!(papers("a11").is_err());
        assert_eq!(mathtext_fallback("STIX"), Ok(Some("stix".to_owned())));
        assert_eq!(mathtext_fallback("None"), Ok(None));
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
