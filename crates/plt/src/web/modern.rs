//! plt-rs' modern chrome for the web pages (see [`crate::look`]): a style
//! sheet over matplotlib's, the Inter faces it loads, and plt-rs' toolbar
//! icons served in place of matplotlib's, at the same URLs.

use std::borrow::Cow;
use std::sync::OnceLock;

use plt_codec::png::{Encoder, ImageRef};
use plt_ui::{Icon, Theme};

use crate::gui::icons;

/// The style sheet's URL path.
pub(crate) const STYLESHEET: &str = "/_static/css/plt-modern.css";

/// The style sheet.
const CSS: &str = include_str!("assets/css/plt-modern.css");

/// The bundled font files the style sheet loads.
const FONTS: [&str; 2] = ["Inter-Regular.ttf", "Inter-SemiBold.ttf"];

/// Draws one of plt-rs' toolbar icons.
type DrawIcon = fn() -> Icon;

/// matplotlib's toolbar images and the plt-rs icons replacing them.
const ICONS: [(&str, DrawIcon); 6] = [
    ("home", icons::home),
    ("back", icons::back),
    ("forward", icons::forward),
    ("move", icons::pan),
    ("zoom_to_rect", icons::zoom),
    ("filesave", icons::save),
];

/// The sides of the toolbar images, as matplotlib names them: the plain
/// image and the `_large` one for high-density screens.
const ICON_SIZES: [(&str, u32); 2] = [("", 24), ("_large", 48)];

/// A file of the modern chrome.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ModernFile {
    /// The URL path.
    pub(crate) path: String,
    /// The `Content-Type`.
    pub(crate) content_type: &'static str,
    /// The contents.
    pub(crate) bytes: Cow<'static, [u8]>,
}

/// The modern chrome's file at URL path `path`.
pub(crate) fn find(path: &str) -> Option<&'static ModernFile> {
    static FILES: OnceLock<Vec<ModernFile>> = OnceLock::new();
    FILES
        .get_or_init(files)
        .iter()
        .find(|file| file.path == path)
}

/// Every file: the style sheet, its fonts and the toolbar images.
fn files() -> Vec<ModernFile> {
    let stylesheet = ModernFile {
        path: STYLESHEET.to_owned(),
        content_type: "text/css; charset=utf-8",
        bytes: Cow::Borrowed(CSS.as_bytes()),
    };
    let fonts = FONTS.iter().filter_map(|&name| {
        Some(ModernFile {
            path: format!("/_static/fonts/{name}"),
            content_type: "font/ttf",
            bytes: Cow::Borrowed(plt_text::bundled_font_data(name)?),
        })
    });
    let ink = Theme::modern().text;
    let images = ICONS.iter().flat_map(|&(name, icon)| {
        ICON_SIZES.map(|(suffix, side)| ModernFile {
            path: format!("/_images/{name}{suffix}.png"),
            content_type: "image/png",
            bytes: Cow::Owned(toolbar_png(&icon(), side, ink)),
        })
    });
    std::iter::once(stylesheet)
        .chain(fonts)
        .chain(images)
        .collect()
}

/// `icon` drawn in `ink` as a `side`-pixel PNG.
fn toolbar_png(icon: &Icon, side: u32, ink: plt_core::Rgba) -> Vec<u8> {
    let rgba = icon
        .render(side, ink)
        .expect("a toolbar icon fits in an image");
    let image = ImageRef::rgba8(side, side, &rgba).expect("the icon fills its square");
    Encoder::new()
        .encode(image)
        .expect("an RGBA image encodes as PNG")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chrome_serves_its_style_sheet_fonts_and_icons() {
        let css = find(STYLESHEET).expect("the style sheet");
        assert!(css.content_type.starts_with("text/css"));
        for name in FONTS {
            let font = find(&format!("/_static/fonts/{name}")).expect("a font");
            assert!(std::str::from_utf8(&css.bytes).unwrap().contains(name));
            assert_eq!(font.content_type, "font/ttf");
        }
        let large = find("/_images/zoom_to_rect_large.png").expect("an icon");
        let decoded = plt_codec::png::decode(&large.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (48, 48));
        assert!(
            find("/_images/matplotlib.png").is_none(),
            "only toolbar images"
        );
    }
}
