//! The rcParams of matplotlib's GUI backends that name a choice: the kind
//! of toolbar (`toolbar`) and how the macOS backend opens windows
//! (`macosx.window_mode`).

use crate::rc::keyword_names;

/// The toolbar of figure windows (`toolbar`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToolbarKind {
    /// `None`: no toolbar.
    None,
    /// `toolbar2`: the classic navigation toolbar.
    #[default]
    Toolbar2,
    /// `toolmanager`: the toolbar of matplotlib's tool manager.
    ToolManager,
}

impl ToolbarKind {
    /// matplotlib's name (`None`, `toolbar2` or `toolmanager`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Toolbar2 => "toolbar2",
            Self::ToolManager => "toolmanager",
        }
    }
}

keyword_names!(
    ToolbarKind,
    "toolbar",
    [Self::None, Self::Toolbar2, Self::ToolManager],
    ignore_case
);

/// How matplotlib's macOS backend opens new figure windows
/// (`macosx.window_mode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MacosxWindowMode {
    /// `system`: as the system settings say.
    #[default]
    System,
    /// `tab`: as tabs of one window.
    Tab,
    /// `window`: as separate windows.
    Window,
}

impl MacosxWindowMode {
    /// matplotlib's name (`system`, `tab` or `window`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Tab => "tab",
            Self::Window => "window",
        }
    }
}

keyword_names!(
    MacosxWindowMode,
    "window mode",
    [Self::System, Self::Tab, Self::Window]
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbars_parse_in_any_case() {
        for kind in [
            ToolbarKind::None,
            ToolbarKind::Toolbar2,
            ToolbarKind::ToolManager,
        ] {
            assert_eq!(kind.to_string().parse(), Ok(kind));
        }
        assert_eq!("TOOLBAR2".parse(), Ok(ToolbarKind::Toolbar2));
        assert_eq!("none".parse(), Ok(ToolbarKind::None));
        assert!("toolbar3".parse::<ToolbarKind>().is_err());
    }

    #[test]
    fn window_modes_round_trip_their_names() {
        for mode in [
            MacosxWindowMode::System,
            MacosxWindowMode::Tab,
            MacosxWindowMode::Window,
        ] {
            assert_eq!(mode.to_string().parse(), Ok(mode));
        }
        assert!("SYSTEM".parse::<MacosxWindowMode>().is_err());
    }
}
