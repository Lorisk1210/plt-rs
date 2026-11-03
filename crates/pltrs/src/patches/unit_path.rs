//! The interpolation steps of the outline all rectangles share.
//!
//! matplotlib's `Rectangle.get_path()` is one shared path,
//! `Path.unit_rectangle()`, for every rectangle, and its `bar` and
//! `axhspan` set `_interpolation_steps` on it: from then on every
//! rectangle, a program's included, is cut into that many pieces before a
//! projection bends it (a `Rectangle` spanning the circle of polar axes
//! after a polar bar chart is drawn as a disk). pltrs keeps the steps of
//! each patch ([`Patch::interpolation_steps`]); a program that relies on
//! matplotlib's sharing sets the shared steps here, which every rectangle
//! then draws with, on the thread that set them (on every thread sharing
//! its state, see [`pltrs_core::state`]).
//!
//! [`Patch::interpolation_steps`]: super::Patch::interpolation_steps

use std::cell::Cell;

pltrs_core::thread_state! {
    /// The steps every rectangle draws with, once set.
    static SHARED_STEPS: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Sets the interpolation steps every rectangle of the thread draws with
/// (matplotlib's `Path.unit_rectangle()._interpolation_steps = steps`);
/// `None`, the default, has each draw with its own.
///
/// ```
/// use pltrs::patches::{set_shared_rectangle_steps, shared_rectangle_steps};
/// set_shared_rectangle_steps(Some(100));
/// assert_eq!(shared_rectangle_steps(), Some(100));
/// set_shared_rectangle_steps(None);
/// ```
pub fn set_shared_rectangle_steps(steps: Option<usize>) {
    SHARED_STEPS.set(steps);
}

/// The interpolation steps every rectangle of the thread draws with, if
/// set (see [`set_shared_rectangle_steps`]).
pub fn shared_rectangle_steps() -> Option<usize> {
    SHARED_STEPS.get()
}
