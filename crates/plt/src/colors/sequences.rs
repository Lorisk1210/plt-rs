//! Named colour sequences (`matplotlib.color_sequences`): the built-in
//! ones and those registered at run time.

use std::cell::RefCell;
use std::fmt;

use plt_core::Rgba;

use super::sequence_data::SEQUENCES;
use super::{Color, ColorError};

thread_local! {
    /// Sequences registered at run time, in registration order.
    static REGISTERED: RefCell<Vec<(String, Vec<Color>)>> = const { RefCell::new(Vec::new()) };
}

/// Why a colour sequence cannot be registered or removed.
#[derive(Clone, Debug, PartialEq)]
pub enum ColorSequenceError {
    /// The name of a built-in sequence, which cannot be replaced or removed.
    Builtin(String),
    /// An entry that is not a colour.
    Color(ColorError),
}

impl fmt::Display for ColorSequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin(name) => {
                write!(
                    f,
                    "{name:?} is a reserved name for a builtin color sequence"
                )
            }
            Self::Color(err) => write!(f, "not a valid color specification: {err}"),
        }
    }
}

impl std::error::Error for ColorSequenceError {}

/// The colours of a built-in sequence.
fn builtin(name: &str) -> Option<Vec<Rgba>> {
    SEQUENCES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, colors)| {
            colors
                .iter()
                .map(|&[r, g, b]| Rgba::opaque(r, g, b))
                .collect()
        })
}

/// The colours of a built-in sequence such as `"tab10"` or `"Set1"`, or of
/// one registered with [`register_color_sequence`] (resolved now, so cycle
/// references follow the current property cycle); `None` for an unknown
/// name.
pub fn color_sequence(name: &str) -> Option<Vec<Rgba>> {
    builtin(name).or_else(|| {
        REGISTERED.with_borrow(|sequences| {
            let (_, colors) = sequences.iter().find(|(n, _)| n == name)?;
            colors.iter().map(|c| c.resolve().ok()).collect()
        })
    })
}

/// Names of all colour sequences: the built-in ones in matplotlib's order,
/// then the registered ones in registration order.
pub fn color_sequence_names() -> Vec<String> {
    let mut names: Vec<String> = SEQUENCES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    REGISTERED.with_borrow(|sequences| names.extend(sequences.iter().map(|(n, _)| n.clone())));
    names
}

/// Registers `colors` under `name` on this thread, replacing a sequence
/// registered under that name before (matplotlib's
/// `color_sequences.register`).
///
/// ```
/// use plt::colors::{Color, color_sequence, register_color_sequence, unregister_color_sequence};
/// register_color_sequence("rgb", &[Color::from("r"), Color::from("g"), Color::from("b")])?;
/// assert_eq!(color_sequence("rgb").map(|colors| colors.len()), Some(3));
/// unregister_color_sequence("rgb")?;
/// assert_eq!(color_sequence("rgb"), None);
/// # Ok::<(), plt::colors::ColorSequenceError>(())
/// ```
///
/// # Errors
///
/// When `name` is the name of a built-in sequence, or an entry is not a
/// colour.
pub fn register_color_sequence(name: &str, colors: &[Color]) -> Result<(), ColorSequenceError> {
    if builtin(name).is_some() {
        return Err(ColorSequenceError::Builtin(name.to_owned()));
    }
    for color in colors {
        color.resolve().map_err(ColorSequenceError::Color)?;
    }
    REGISTERED.with_borrow_mut(|sequences| {
        sequences.retain(|(n, _)| n != name);
        sequences.push((name.to_owned(), colors.to_vec()));
    });
    Ok(())
}

/// Removes the sequence registered under `name` (matplotlib's
/// `color_sequences.unregister`); names nothing is registered under are
/// ignored.
///
/// # Errors
///
/// When `name` is the name of a built-in sequence.
pub fn unregister_color_sequence(name: &str) -> Result<(), ColorSequenceError> {
    if builtin(name).is_some() {
        return Err(ColorSequenceError::Builtin(name.to_owned()));
    }
    REGISTERED.with_borrow_mut(|sequences| sequences.retain(|(n, _)| n != name));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_sequences_are_reserved() {
        let colors = [Color::from("r")];
        assert_eq!(
            register_color_sequence("tab10", &colors),
            Err(ColorSequenceError::Builtin("tab10".into()))
        );
        assert!(unregister_color_sequence("tab10").is_err());
        assert_eq!(color_sequence("tab10").map(|c| c.len()), Some(10));
    }

    #[test]
    fn registered_sequences_are_listed_replaced_and_removed() {
        assert!(matches!(
            register_color_sequence("bad", &[Color::from("not a color")]),
            Err(ColorSequenceError::Color(_))
        ));
        register_color_sequence("pair", &[Color::from("r"), Color::from("b")]).unwrap();
        register_color_sequence("pair", &[Color::from("k")]).unwrap();
        assert_eq!(
            color_sequence("pair"),
            Some(vec![Rgba::opaque(0.0, 0.0, 0.0)])
        );
        assert_eq!(
            color_sequence_names().last().map(String::as_str),
            Some("pair")
        );
        unregister_color_sequence("pair").unwrap();
        unregister_color_sequence("pair").unwrap();
        assert!(!color_sequence_names().iter().any(|n| n == "pair"));
    }
}
