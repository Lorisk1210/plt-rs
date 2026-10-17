//! What figure-window shortcuts do to a figure: saving it, toggling grids
//! and logarithmic axes (matplotlib's `key_press_handler`).

use std::path::PathBuf;

use super::keys::KeyAction;
use crate::axes::{Axes, AxisName, GridOptions, TickLevel};
use crate::figure::{Figure, SaveError};

/// Saves `figure` as `<title>.png` in the working directory, numbering
/// the name if the file exists; returns the path written.
///
/// # Errors
///
/// When the figure cannot be rendered or written.
pub(super) fn save(figure: &Figure, title: &str) -> Result<PathBuf, SaveError> {
    let stem: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let path = (0..10_000_u32)
        .map(|n| match n {
            0 => PathBuf::from(format!("{stem}.png")),
            n => PathBuf::from(format!("{stem}-{n}.png")),
        })
        .find(|path| !path.exists())
        .ok_or_else(|| {
            SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("every {stem}-N.png name is taken"),
            ))
        })?;
    figure.savefig(&path)?;
    Ok(path)
}

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
pub(super) fn apply_to_axes(ax: &Axes, action: KeyAction) {
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
                    &GridOptions {
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
                    &GridOptions {
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
