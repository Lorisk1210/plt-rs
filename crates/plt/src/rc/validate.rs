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

/// Accepts one of `allowed`, ignoring ASCII case.
pub(crate) fn one_of_ignore_case(
    allowed: &'static [&'static str],
) -> impl Fn(&String) -> Result<(), String> {
    move |value| {
        allowed
            .iter()
            .any(|a| a.eq_ignore_ascii_case(value))
            .then_some(())
            .ok_or_else(|| format!("{value:?} is not one of {allowed:?}"))
    }
}

/// Accepts `None` or one of `allowed`.
pub(crate) fn optional_one_of(
    allowed: &'static [&'static str],
) -> impl Fn(&Option<String>) -> Result<(), String> {
    let check = one_of(allowed);
    move |value| value.as_ref().map_or(Ok(()), &check)
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

/// Legend locations: a name, a code `0..=10`, or an `(x, y)` pair.
pub(crate) fn legend_loc(value: &str) -> Result<(), String> {
    const NAMES: [&str; 11] = [
        "best",
        "upper right",
        "upper left",
        "lower left",
        "lower right",
        "right",
        "center left",
        "center right",
        "lower center",
        "upper center",
        "center",
    ];
    if NAMES.iter().any(|n| n.eq_ignore_ascii_case(value)) {
        return Ok(());
    }
    if value.parse::<u8>().is_ok_and(|code| code <= 10) {
        return Ok(());
    }
    let pair = super::literal::Expr::parse(value).ok().and_then(|e| {
        e.as_sequence()
            .map(|items| items.iter().all(|i| i.as_number().is_some()) && items.len() == 2)
    });
    pair.unwrap_or(false)
        .then_some(())
        .ok_or_else(|| format!("{value} is not a valid legend location"))
}

/// Minor tick divisions: `auto` or a non-negative integer.
pub(crate) fn minor_ndivs(value: &str) -> Result<(), String> {
    (value.eq_ignore_ascii_case("auto") || value.parse::<u32>().is_ok())
        .then_some(())
        .ok_or_else(|| "'tick.minor.ndivs' must be 'auto' or non-negative int".to_owned())
}
