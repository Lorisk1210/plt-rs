//! Colour specifications and their conversion to RGBA (`matplotlib.colors.to_rgba`).

use std::fmt::{self, Write as _};
use std::str::FromStr;

use plt_core::Rgba;

use super::named;

/// A colour as users specify it: anything matplotlib's `to_rgba` accepts.
///
/// Strings convert infallibly so properties can be set ergonomically
/// (`line.set_color("tab:orange")`); they are validated when resolved with
/// [`Color::resolve`], or eagerly with [`Color::parse`].
///
/// ```
/// use plt::colors::Color;
/// use plt_core::Rgba;
/// assert_eq!(Color::from("red").resolve()?, Rgba::opaque(1.0, 0.0, 0.0));
/// assert_eq!(Color::from("#00f8").resolve()?.a, 0x88 as f64 / 255.0);
/// assert!(Color::from("not a colour").resolve().is_err());
/// # Ok::<(), plt::colors::ColorError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    /// An explicit colour (components are validated when resolved).
    Rgba(Rgba),
    /// A colour string: a name, hex code, grey level, `"Cn"` or `"none"`.
    Spec(String),
}

impl Color {
    /// matplotlib's `"none"`: draw nothing.
    pub fn none() -> Self {
        Self::Spec("none".to_owned())
    }

    /// Validates a colour string now rather than when it is used.
    pub fn parse(spec: &str) -> Result<Self, ColorError> {
        let color = Self::Spec(spec.to_owned());
        if !is_cycle_reference(spec) {
            color.resolve_with(None, &[])?;
        }
        Ok(color)
    }

    /// Resolves to RGBA, looking up `"Cn"` in the current property cycle.
    pub fn resolve(&self) -> Result<Rgba, ColorError> {
        self.resolve_alpha(None)
    }

    /// Resolves to RGBA with an alpha override (ignored for `"none"`).
    pub fn resolve_alpha(&self, alpha: Option<f64>) -> Result<Rgba, ColorError> {
        let cycle = crate::rc::params().axes_prop_cycle.colors();
        self.resolve_with(alpha, &cycle)
    }

    /// Resolves to RGBA using an explicit colour cycle for `"Cn"`
    /// (`n` wraps around; an empty cycle means black, as in matplotlib).
    pub fn resolve_with(&self, alpha: Option<f64>, cycle: &[Rgba]) -> Result<Rgba, ColorError> {
        if let Some(a) = alpha
            && !(0.0..=1.0).contains(&a)
        {
            return Err(ColorError::AlphaOutOfRange(a));
        }
        match self {
            Self::Rgba(rgba) => from_components(*rgba, alpha),
            Self::Spec(spec) => match cycle_index(spec) {
                Some(n) => {
                    let base = if cycle.is_empty() {
                        Rgba::BLACK
                    } else {
                        cycle[n % cycle.len()]
                    };
                    from_components(base, alpha)
                }
                None => parse_spec(spec, alpha),
            },
        }
    }

    /// Whether this is the fully transparent `"none"` colour.
    pub fn is_none(&self) -> bool {
        match self {
            Self::Spec(spec) => spec.eq_ignore_ascii_case("none"),
            Self::Rgba(_) => false,
        }
    }
}

impl From<Rgba> for Color {
    fn from(rgba: Rgba) -> Self {
        Self::Rgba(rgba)
    }
}

impl From<&str> for Color {
    fn from(spec: &str) -> Self {
        Self::Spec(spec.to_owned())
    }
}

impl From<String> for Color {
    fn from(spec: String) -> Self {
        Self::Spec(spec)
    }
}

impl From<(f64, f64, f64)> for Color {
    fn from((r, g, b): (f64, f64, f64)) -> Self {
        Self::Rgba(Rgba::opaque(r, g, b))
    }
}

impl From<(f64, f64, f64, f64)> for Color {
    fn from((r, g, b, a): (f64, f64, f64, f64)) -> Self {
        Self::Rgba(Rgba::new(r, g, b, a))
    }
}

impl From<[f64; 3]> for Color {
    fn from([r, g, b]: [f64; 3]) -> Self {
        Self::Rgba(Rgba::opaque(r, g, b))
    }
}

impl From<[f64; 4]> for Color {
    fn from([r, g, b, a]: [f64; 4]) -> Self {
        Self::Rgba(Rgba::new(r, g, b, a))
    }
}

impl FromStr for Color {
    type Err = ColorError;

    fn from_str(spec: &str) -> Result<Self, ColorError> {
        Self::parse(spec)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spec(spec) => f.write_str(spec),
            Self::Rgba(rgba) => write!(f, "({}, {}, {}, {})", rgba.r, rgba.g, rgba.b, rgba.a),
        }
    }
}

/// An invalid colour specification.
#[derive(Clone, Debug, PartialEq)]
pub enum ColorError {
    /// Not something matplotlib recognises as a colour.
    Invalid(String),
    /// A `#` code of the wrong length.
    InvalidHex(String),
    /// A grey-level string outside `[0, 1]`.
    GreyOutOfRange(String),
    /// An RGBA component outside `[0, 1]`.
    ComponentOutOfRange(Rgba),
    /// An alpha override outside `[0, 1]`.
    AlphaOutOfRange(f64),
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(spec) => write!(f, "invalid RGBA argument: {spec:?}"),
            Self::InvalidHex(spec) => write!(f, "invalid hex color specifier: {spec:?}"),
            Self::GreyOutOfRange(spec) => {
                write!(
                    f,
                    "invalid string grayscale value {spec:?}; value must be within 0-1 range"
                )
            }
            Self::ComponentOutOfRange(rgba) => {
                write!(f, "RGBA values should be within 0-1 range, got {rgba:?}")
            }
            Self::AlphaOutOfRange(alpha) => {
                write!(f, "alpha must be between 0 and 1 inclusive, got {alpha}")
            }
        }
    }
}

impl std::error::Error for ColorError {}

/// Converts a colour string to RGBA (matplotlib's `to_rgba`), resolving
/// `"Cn"` against the current property cycle.
pub fn to_rgba(spec: &str, alpha: Option<f64>) -> Result<Rgba, ColorError> {
    Color::from(spec).resolve_alpha(alpha)
}

/// Whether matplotlib would accept `spec` as a colour.
pub fn is_color_like(spec: &str) -> bool {
    is_cycle_reference(spec) || parse_spec(spec, None).is_ok()
}

/// `#rrggbb` (or `#rrggbbaa` with `keep_alpha`), rounding half to even
/// like Python's `round`.
pub fn to_hex(rgba: Rgba, keep_alpha: bool) -> String {
    let channels: &[f64] = if keep_alpha {
        &[rgba.r, rgba.g, rgba.b, rgba.a]
    } else {
        &[rgba.r, rgba.g, rgba.b]
    };
    channels.iter().fold(String::from("#"), |mut hex, v| {
        let byte = (v * 255.0).round_ties_even() as u8;
        write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
        hex
    })
}

/// Whether `spec` is a property-cycle reference `"C<digits>"`.
pub fn is_cycle_reference(spec: &str) -> bool {
    cycle_index(spec).is_some()
}

fn cycle_index(spec: &str) -> Option<usize> {
    let digits = spec.strip_prefix('C')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Indices beyond usize wrap identically modulo any realistic cycle length.
    Some(digits.parse().unwrap_or(usize::MAX))
}

fn from_components(rgba: Rgba, alpha: Option<f64>) -> Result<Rgba, ColorError> {
    let rgba = alpha.map_or(rgba, |a| rgba.with_alpha(a));
    if rgba.to_array().iter().all(|v| (0.0..=1.0).contains(v)) {
        Ok(rgba)
    } else {
        Err(ColorError::ComponentOutOfRange(rgba))
    }
}

/// matplotlib's `_to_rgba_no_colorcycle` for strings.
fn parse_spec(spec: &str, alpha: Option<f64>) -> Result<Rgba, ColorError> {
    if spec.eq_ignore_ascii_case("none") {
        return Ok(Rgba::TRANSPARENT);
    }
    if let Some(rgba) = named::lookup(spec) {
        return Ok(alpha.map_or(rgba, |a| rgba.with_alpha(a)));
    }
    if let Some(digits) = spec.strip_prefix('#')
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return parse_hex(spec, digits, alpha);
    }
    match spec.trim().parse::<f64>() {
        Ok(grey) if (0.0..=1.0).contains(&grey) => {
            Ok(Rgba::new(grey, grey, grey, alpha.unwrap_or(1.0)))
        }
        Ok(_) => Err(ColorError::GreyOutOfRange(spec.to_owned())),
        Err(_) => Err(ColorError::Invalid(spec.to_owned())),
    }
}

fn parse_hex(spec: &str, digits: &str, alpha: Option<f64>) -> Result<Rgba, ColorError> {
    let nibble = |i: usize| f64::from(u8::from_str_radix(&digits[i..=i], 16).unwrap_or(0)) / 15.0;
    let byte = |i: usize| f64::from(u8::from_str_radix(&digits[i..i + 2], 16).unwrap_or(0)) / 255.0;
    let (rgb, own_alpha) = match digits.len() {
        3 => ([nibble(0), nibble(1), nibble(2)], 1.0),
        4 => ([nibble(0), nibble(1), nibble(2)], nibble(3)),
        6 => ([byte(0), byte(2), byte(4)], 1.0),
        8 => ([byte(0), byte(2), byte(4)], byte(6)),
        _ => return Err(ColorError::InvalidHex(spec.to_owned())),
    };
    let [r, g, b] = rgb;
    Ok(Rgba::new(r, g, b, alpha.unwrap_or(own_alpha)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(spec: &str) -> Result<Rgba, ColorError> {
        Color::from(spec).resolve_with(None, &[Rgba::opaque(0.1, 0.2, 0.3), Rgba::WHITE])
    }

    #[test]
    fn named_colours_follow_priority_and_case_rules() {
        assert_eq!(rgba("r"), Ok(Rgba::opaque(1.0, 0.0, 0.0)));
        assert!(rgba("R").is_err(), "single letters are case-sensitive");
        assert_eq!(rgba("Red"), Ok(Rgba::opaque(1.0, 0.0, 0.0)));
        assert_eq!(rgba("tab:grey"), rgba("tab:gray"));
        assert_eq!(
            rgba("xkcd:sky blue").map(|c| to_hex(c, false)),
            Ok("#75bbfd".into())
        );
        assert_eq!(
            rgba("g"),
            Ok(Rgba::opaque(0.0, 0.5, 0.0)),
            "base colours beat CSS4 green"
        );
    }

    #[test]
    fn hex_forms() {
        assert_eq!(rgba("#ff0000"), Ok(Rgba::opaque(1.0, 0.0, 0.0)));
        assert_eq!(rgba("#f00"), Ok(Rgba::opaque(1.0, 0.0, 0.0)));
        assert_eq!(rgba("#f008").map(|c| c.a), Ok(8.0 / 15.0));
        assert_eq!(rgba("#ff000080").map(|c| c.a), Ok(128.0 / 255.0));
        assert_eq!(rgba("#ff00"), Ok(Rgba::new(1.0, 1.0, 0.0, 0.0)));
        assert!(matches!(rgba("#ff00f"), Err(ColorError::InvalidHex(_))));
        assert!(matches!(rgba("#gg0000"), Err(ColorError::Invalid(_))));
    }

    #[test]
    fn grey_levels_none_and_cycle() {
        assert_eq!(rgba("0.25"), Ok(Rgba::opaque(0.25, 0.25, 0.25)));
        assert!(matches!(rgba("1.5"), Err(ColorError::GreyOutOfRange(_))));
        assert_eq!(rgba("none"), Ok(Rgba::TRANSPARENT));
        assert_eq!(rgba("C1"), Ok(Rgba::WHITE));
        assert_eq!(
            rgba("C2"),
            Ok(Rgba::opaque(0.1, 0.2, 0.3)),
            "cycle references wrap"
        );
        assert!(rgba("C").is_err());
    }

    #[test]
    fn alpha_handling() {
        let c = Color::from("red");
        assert_eq!(c.resolve_with(Some(0.5), &[]).map(|c| c.a), Ok(0.5));
        assert_eq!(
            Color::from("none").resolve_with(Some(0.5), &[]),
            Ok(Rgba::TRANSPARENT)
        );
        assert_eq!(
            c.resolve_with(Some(1.5), &[]),
            Err(ColorError::AlphaOutOfRange(1.5))
        );
        assert!(
            Color::from((1.5, 0.0, 0.0))
                .resolve_with(None, &[])
                .is_err()
        );
    }

    #[test]
    fn hex_output_rounds_half_to_even() {
        assert_eq!(to_hex(Rgba::new(0.5, 1.0, 0.0, 0.5), true), "#80ff0080");
        assert_eq!(to_hex(Rgba::new(0.5, 1.0, 0.0, 0.5), false), "#80ff00");
        assert_eq!(
            to_hex(Rgba::opaque(2.5 / 255.0, 0.0, 0.0), false),
            "#020000"
        );
    }

    #[test]
    fn eager_parsing() {
        assert!(Color::parse("C7").is_ok());
        assert!("navy".parse::<Color>().is_ok());
        assert!("nosuchcolour".parse::<Color>().is_err());
        assert!(is_color_like("xkcd:puke green"));
        assert!(!is_color_like("puke"));
    }
}
