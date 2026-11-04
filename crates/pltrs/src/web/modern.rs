//! pltrs' modern chrome for the web pages (see [`crate::look`]): a style
//! sheet over matplotlib's, written from the design tokens of pltrs'
//! windows, the Inter faces it loads, a script using the client's hooks,
//! and pltrs' toolbar icons served in place of matplotlib's, at the same
//! URLs. As a window does, a figure drawn on a dark background takes the
//! dark theme, and a page whose figures all are is dark.

mod tokens;

use std::borrow::Cow;
use std::sync::OnceLock;

use pltrs_codec::png::{Encoder, ImageRef};
use pltrs_ui::{Icon, Theme};

use super::net::ServedFigure;
use crate::colors::Rgba;
use crate::figure::Figure;
use crate::gui::icons;
use crate::look::ChromeLook;

/// The style sheet's URL path.
const STYLESHEET: &str = "/_static/css/pltrs-modern.css";

/// The script's URL path.
const SCRIPT: &str = "/_static/js/pltrs-modern.js";

/// The style sheet's rules, which the design tokens precede.
const CSS: &str = include_str!("assets/css/pltrs-modern.css");

/// The script.
const JS: &str = include_str!("assets/js/pltrs-modern.js");

/// The bundled font files the style sheet loads.
const FONTS: [&str; 2] = ["Inter-Regular.ttf", "Inter-SemiBold.ttf"];

/// Draws one of pltrs' toolbar icons.
type DrawIcon = fn() -> Icon;

/// matplotlib's toolbar images and the pltrs icons replacing them.
const ICONS: [(&str, DrawIcon); 6] = [
    ("home", icons::home),
    ("back", icons::back),
    ("forward", icons::forward),
    ("move", icons::pan),
    ("zoom_to_rect", icons::zoom),
    ("filesave", icons::save),
];

/// The sides of the toolbar images, as matplotlib names them: the plain
/// image and the `_large` one for high-density screens, at the theme's
/// icon size and twice it, so that the icons are drawn pixel for pixel.
const ICON_SIZES: [(&str, u32); 2] = [("", 20), ("_large", 40)];

/// The opening tag of the page templates.
const HTML: &str = r#"<html lang="en">"#;

/// The link to matplotlib's own style sheet in the page templates, which
/// the modern chrome's follows.
const MPL_STYLESHEET: &str =
    r#"<link rel="stylesheet" href="{{ prefix }}/_static/css/mpl.css" type="text/css">"#;

/// The client script in the page templates, which the modern chrome's
/// follows.
const MPL_SCRIPT: &str = r#"<script src="{{ prefix }}/js/mpl.js"></script>"#;

/// The title of the page of every figure, matplotlib's and the modern
/// chrome's.
const ALL_FIGURES_TITLE: (&str, &str) = (
    "<title>MPL | WebAgg current figures</title>",
    "<title>Figures</title>",
);

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

/// Every file: the style sheet, the script, the fonts and the toolbar
/// images.
fn files() -> Vec<ModernFile> {
    let stylesheet = ModernFile {
        path: STYLESHEET.to_owned(),
        content_type: "text/css; charset=utf-8",
        bytes: Cow::Owned(format!("{}\n{CSS}", tokens::declarations()).into_bytes()),
    };
    let script = ModernFile {
        path: SCRIPT.to_owned(),
        content_type: "text/javascript; charset=utf-8",
        bytes: Cow::Borrowed(JS.as_bytes()),
    };
    let fonts = FONTS.iter().filter_map(|&name| {
        Some(ModernFile {
            path: format!("/_static/fonts/{name}"),
            content_type: "font/ttf",
            bytes: Cow::Borrowed(pltrs_text::bundled_font_data(name)?),
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
    [stylesheet, script]
        .into_iter()
        .chain(fonts)
        .chain(images)
        .collect()
}

/// `icon` drawn in `ink` as a `side`-pixel PNG.
fn toolbar_png(icon: &Icon, side: u32, ink: Rgba) -> Vec<u8> {
    let rgba = icon
        .render(side, ink)
        .expect("a toolbar icon fits in an image");
    let image = ImageRef::rgba8(side, side, &rgba).expect("the icon fills its square");
    Encoder::new()
        .encode(image)
        .expect("an RGBA image encodes as PNG")
}

/// Whether the modern chrome shows `figure` in the dark theme: when it is
/// drawn on a dark background, as a window chooses (see
/// [`ChromeLook::theme`]); a background that does not resolve is taken as
/// white.
pub(crate) fn is_dark(figure: &Figure) -> bool {
    let background = figure.facecolor().resolve().unwrap_or(Rgba::WHITE);
    ChromeLook::Modern.theme(background) == Theme::dark()
}

/// `page`, one of matplotlib's page templates showing `figures` (its
/// `{{ prefix }}` not filled in yet), in the modern chrome: its style
/// sheet after matplotlib's, its script after matplotlib's client, both
/// under the prefix, the figures drawn on a dark background listed for
/// the script, and the whole page dark when every figure is.
pub(crate) fn dress(page: &str, figures: &[ServedFigure]) -> String {
    let dark: Vec<String> = figures
        .iter()
        .filter(|figure| figure.dark)
        .map(|figure| figure.number.to_string())
        .collect();
    let class = if !figures.is_empty() && dark.len() == figures.len() {
        format!(r#" class="{}""#, tokens::DARK_CLASS)
    } else {
        String::new()
    };
    let html = format!(
        r#"{}{class} data-pltrs-dark="{}">"#,
        HTML.trim_end_matches('>'),
        dark.join(" ")
    );
    let stylesheet = format!(
        "{MPL_STYLESHEET}\n    <link rel=\"stylesheet\" href=\"{{{{ prefix }}}}{STYLESHEET}\" type=\"text/css\">"
    );
    let script = format!("{MPL_SCRIPT}\n    <script src=\"{{{{ prefix }}}}{SCRIPT}\"></script>");
    page.replacen(HTML, &html, 1)
        .replacen(MPL_STYLESHEET, &stylesheet, 1)
        .replacen(MPL_SCRIPT, &script, 1)
        .replacen(ALL_FIGURES_TITLE.0, ALL_FIGURES_TITLE.1, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chrome_serves_its_style_sheet_script_fonts_and_icons() {
        let css = find(STYLESHEET).expect("the style sheet");
        assert!(css.content_type.starts_with("text/css"));
        let text = std::str::from_utf8(&css.bytes).unwrap();
        assert!(text.starts_with(":root {\n  color-scheme: light;"));
        assert!(text.ends_with(CSS));
        for name in FONTS {
            let font = find(&format!("/_static/fonts/{name}")).expect("a font");
            assert!(text.contains(name));
            assert_eq!(font.content_type, "font/ttf");
        }
        let script = find(SCRIPT).expect("the script");
        assert!(script.content_type.starts_with("text/javascript"));
        for (suffix, side) in ICON_SIZES {
            let icon = find(&format!("/_images/zoom_to_rect{suffix}.png")).expect("an icon");
            let decoded = pltrs_codec::png::decode(&icon.bytes).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (side, side));
        }
        assert!(
            find("/_images/matplotlib.png").is_none(),
            "only toolbar images"
        );
    }

    #[test]
    fn icons_are_drawn_at_the_themes_icon_size() {
        let theme = Theme::modern();
        assert_eq!(f64::from(ICON_SIZES[0].1), theme.icon_size);
        assert_eq!(f64::from(ICON_SIZES[1].1), 2.0 * theme.icon_size);
    }

    #[test]
    fn pages_take_the_style_sheet_and_script_after_matplotlibs() {
        let page = format!(
            "{HTML}\n{MPL_STYLESHEET}\n{MPL_SCRIPT}\n{}",
            ALL_FIGURES_TITLE.0
        );
        let dressed = dress(&page, &[ServedFigure::new(1, false)]);
        let position = |text: &str| dressed.find(text).expect(text);
        assert!(position(STYLESHEET) > position(MPL_STYLESHEET));
        assert!(position(SCRIPT) > position(MPL_SCRIPT));
        assert!(dressed.contains("<title>Figures</title>"));
        assert!(dressed.contains(r#"<html lang="en" data-pltrs-dark="">"#));
    }

    #[test]
    fn pages_name_their_dark_figures() {
        let mixed = [
            ServedFigure::new(1, true),
            ServedFigure::new(2, false),
            ServedFigure::new(4, true),
        ];
        assert_eq!(
            dress(HTML, &mixed),
            r#"<html lang="en" data-pltrs-dark="1 4">"#
        );
        assert_eq!(
            dress(HTML, &[ServedFigure::new(3, true)]),
            r#"<html lang="en" class="pltrs-dark" data-pltrs-dark="3">"#
        );
        assert_eq!(dress(HTML, &[]), r#"<html lang="en" data-pltrs-dark="">"#);
    }

    #[test]
    fn figures_on_dark_backgrounds_are_dark() {
        let figure = Figure::new();
        assert!(is_dark(figure.set_facecolor("#1e2024")));
        assert!(!is_dark(figure.set_facecolor("white")));
        assert!(
            !is_dark(figure.set_facecolor("none")),
            "a transparent figure shows the page"
        );
    }
}
