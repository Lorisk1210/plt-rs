//! The default scalar formatters axes take with a linear or function
//! scale, counted for a binding: matplotlib's `Axis._set_scale` makes a
//! `ScalarFormatter` for those scales, whose constructor checks the text
//! font (`set_useMathText(None)` looks :rc:`font.family` up and warns
//! about cmr10), which a binding emulating matplotlib repeats once per
//! formatter made. A check that found nothing to say holds until the
//! parameters change: matplotlib's font manager caches the lookup, so the
//! formatters made meanwhile are not counted.

use std::cell::Cell;

use super::ScaleName;

thread_local! {
    /// The formatters made on this thread and not taken yet.
    static MADE: Cell<usize> = const { Cell::new(0) };
    /// The generation of the parameters under which the last check found
    /// nothing to say (see [`scalar_formatters_quiet`]).
    static QUIET: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Counts the default formatter an axis takes with the scale `name`, if
/// matplotlib's is a `ScalarFormatter`.
pub(crate) fn note_default_formatter(name: ScaleName) {
    if matches!(name, ScaleName::Linear | ScaleName::Function)
        && QUIET.get() != Some(crate::rc::generation())
    {
        MADE.set(MADE.get().saturating_add(1));
    }
}

/// How many default scalar formatters axes took on this thread since the
/// last call (see the module).
///
/// ```
/// use pltrs::prelude::*;
/// pltrs::scale::take_scalar_formatters_made();
/// let ax = Figure::new().subplot();
/// assert!(pltrs::scale::take_scalar_formatters_made() > 0);
/// ax.set_xscale("log")?;
/// assert_eq!(pltrs::scale::take_scalar_formatters_made(), 0);
/// ax.set_xscale("linear")?;
/// assert_eq!(pltrs::scale::take_scalar_formatters_made(), 1);
/// # Ok::<(), pltrs::scale::ScaleError>(())
/// ```
pub fn take_scalar_formatters_made() -> usize {
    MADE.replace(0)
}

/// Tells that checking the formatters taken found nothing to say under
/// the current parameters: those made under them are not counted.
///
/// ```
/// use pltrs::prelude::*;
/// pltrs::scale::take_scalar_formatters_made();
/// pltrs::scale::scalar_formatters_quiet();
/// Figure::new().subplot();
/// assert_eq!(pltrs::scale::take_scalar_formatters_made(), 0);
/// ```
pub fn scalar_formatters_quiet() {
    QUIET.set(Some(crate::rc::generation()));
}
