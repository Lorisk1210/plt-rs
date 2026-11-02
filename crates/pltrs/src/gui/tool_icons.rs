//! The icons of toolbar buttons by matplotlib's image names (`home`,
//! `zoom_to_rect`, ... of `mpl-data/images`): pltrs' line icons in the
//! modern chrome, matplotlib's toolbar images in matplotlib's.

use pltrs_ui::{Icon, ToolIcon};

use super::icons;
use super::images::{self, ToolbarImage};
use crate::look::ChromeLook;

/// The icon of matplotlib's toolbar image `name` in the chrome `look`,
/// if pltrs draws that image.
pub(crate) fn named(name: &str, look: ChromeLook) -> Option<ToolIcon> {
    let (line, image): (fn() -> Icon, ToolbarImage) = match name {
        "home" => (icons::home, images::HOME),
        "back" => (icons::back, images::BACK),
        "forward" => (icons::forward, images::FORWARD),
        "move" => (icons::pan, images::MOVE),
        "zoom_to_rect" => (icons::zoom, images::ZOOM_TO_RECT),
        "subplots" => (icons::subplots, images::SUBPLOTS),
        "qt4_editor_options" => (icons::options, images::FIGURE_OPTIONS),
        "filesave" => (icons::save, images::FILESAVE),
        "help" => (icons::help, images::HELP),
        _ => return None,
    };
    Some(match look {
        ChromeLook::Modern => line().into(),
        ChromeLook::Matplotlib => image.picture().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matplotlibs_toolbar_images_have_icons_in_both_chromes() {
        for name in [
            "home",
            "back",
            "forward",
            "move",
            "zoom_to_rect",
            "subplots",
            "qt4_editor_options",
            "filesave",
            "help",
        ] {
            let modern = named(name, ChromeLook::Modern);
            assert!(matches!(modern, Some(ToolIcon::Vector(_))), "{name}");
            let matplotlib = named(name, ChromeLook::Matplotlib);
            assert!(matches!(matplotlib, Some(ToolIcon::Picture(_))), "{name}");
        }
        assert!(named("matplotlib", ChromeLook::Modern).is_none());
    }
}
