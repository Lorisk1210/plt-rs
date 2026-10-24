//! What figure-window shortcuts do to the axes under the pointer: toggling
//! grids and logarithmic axes (matplotlib's `key_press_handler`).

use super::keys::KeyAction;
use crate::axes::{Axes, AxisName, GridOptions, TickLevel};

/// The next grid state in matplotlib's cycle: none, x, both, y.
fn next_grids(state: (bool, bool)) -> (bool, bool) {
    match state {
        (false, false) => (true, false),
        (true, false) => (true, true),
        (true, true) => (false, true),
        (false, true) => (false, false),
    }
}

/// Applies a grid or scale shortcut to the axes under the pointer.
pub(crate) fn apply_to_axes(ax: &Axes, action: KeyAction) {
    match action {
        KeyAction::Grid => {
            let (x, y) = next_grids((
                ax.has_grid(AxisName::X, true),
                ax.has_grid(AxisName::Y, true),
            ));
            for (name, on) in [(AxisName::X, x), (AxisName::Y, y)] {
                // Turning the major grid off turns the minor grid off too.
                let which = if on {
                    TickLevel::Major
                } else {
                    TickLevel::Both
                };
                ax.grid_with(
                    on,
                    GridOptions {
                        which,
                        axis: Some(name),
                        ..GridOptions::default()
                    },
                );
            }
        }
        KeyAction::GridMinor => {
            let (x, y) = next_grids((
                ax.has_grid(AxisName::X, false),
                ax.has_grid(AxisName::Y, false),
            ));
            for (name, on) in [(AxisName::X, x), (AxisName::Y, y)] {
                ax.grid_with(
                    on,
                    GridOptions {
                        which: TickLevel::Both,
                        axis: Some(name),
                        ..GridOptions::default()
                    },
                );
            }
        }
        KeyAction::LogX => toggle_log(ax, AxisName::X),
        KeyAction::LogY => toggle_log(ax, AxisName::Y),
        _ => {}
    }
}

/// Switches an axis between linear and logarithmic scaling (other scales
/// are left alone).
fn toggle_log(ax: &Axes, name: AxisName) {
    let current = match name {
        AxisName::X => ax.xscale(),
        AxisName::Y => ax.yscale(),
    };
    let next = match current {
        "log" => "linear",
        "linear" => "log",
        _ => return,
    };
    let applied = match name {
        AxisName::X => ax.set_xscale(next).is_ok(),
        AxisName::Y => ax.set_yscale(next).is_ok(),
    };
    debug_assert!(applied, "linear and log are registered scales");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn grid_shortcuts_cycle_like_matplotlib() {
        let ax = Figure::new().subplot();
        apply_to_axes(&ax, KeyAction::Grid);
        assert!(ax.has_grid(AxisName::X, true) && !ax.has_grid(AxisName::Y, true));
        apply_to_axes(&ax, KeyAction::Grid);
        assert!(ax.has_grid(AxisName::Y, true));
        apply_to_axes(&ax, KeyAction::Grid);
        assert!(!ax.has_grid(AxisName::X, true) && ax.has_grid(AxisName::Y, true));
    }

    #[test]
    fn log_shortcuts_toggle() {
        let ax = Figure::new().subplot();
        apply_to_axes(&ax, KeyAction::LogY);
        assert_eq!(ax.yscale(), "log");
        apply_to_axes(&ax, KeyAction::LogY);
        assert_eq!(ax.yscale(), "linear");
    }
}
