//! The browser client: matplotlib's pages, script, style sheets and
//! toolbar icons, embedded unchanged (their license is in
//! `assets/LICENSE`), and the parts matplotlib's server generates; the
//! modern chrome's additions are in [`super::modern`].

use std::fmt::Write as _;

use super::{json, modern};
use crate::figure::SaveFormat;
use crate::look::ChromeLook;

/// A file served as it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Asset {
    /// The URL path.
    pub(crate) path: &'static str,
    /// The `Content-Type`.
    pub(crate) content_type: &'static str,
    /// The contents.
    pub(crate) bytes: &'static [u8],
}

const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";
const PNG: &str = "image/png";

/// An embedded file of `assets/`.
macro_rules! asset {
    ($path:literal, $content_type:expr, $file:literal) => {
        Asset {
            path: $path,
            content_type: $content_type,
            bytes: include_bytes!(concat!("assets/", $file)),
        }
    };
}

/// The static files, at matplotlib's URLs.
const STATIC: [Asset; 18] = [
    asset!("/_static/css/page.css", CSS, "css/page.css"),
    asset!("/_static/css/boilerplate.css", CSS, "css/boilerplate.css"),
    asset!("/_static/css/fbm.css", CSS, "css/fbm.css"),
    asset!("/_static/css/mpl.css", CSS, "css/mpl.css"),
    asset!("/_static/js/mpl_tornado.js", JS, "js/mpl_tornado.js"),
    asset!("/_images/home.png", PNG, "images/home.png"),
    asset!("/_images/home_large.png", PNG, "images/home_large.png"),
    asset!("/_images/back.png", PNG, "images/back.png"),
    asset!("/_images/back_large.png", PNG, "images/back_large.png"),
    asset!("/_images/forward.png", PNG, "images/forward.png"),
    asset!(
        "/_images/forward_large.png",
        PNG,
        "images/forward_large.png"
    ),
    asset!("/_images/move.png", PNG, "images/move.png"),
    asset!("/_images/move_large.png", PNG, "images/move_large.png"),
    asset!("/_images/zoom_to_rect.png", PNG, "images/zoom_to_rect.png"),
    asset!(
        "/_images/zoom_to_rect_large.png",
        PNG,
        "images/zoom_to_rect_large.png"
    ),
    asset!("/_images/filesave.png", PNG, "images/filesave.png"),
    asset!(
        "/_images/filesave_large.png",
        PNG,
        "images/filesave_large.png"
    ),
    asset!("/favicon.ico", PNG, "images/matplotlib.png"),
];

/// The static file at URL path `path`.
pub(crate) fn find(path: &str) -> Option<&'static Asset> {
    STATIC.iter().find(|asset| asset.path == path)
}

/// matplotlib's client script.
const MPL_JS: &str = include_str!("assets/js/mpl.js");

/// The page showing one figure (a Tornado template).
const SINGLE_FIGURE: &str = include_str!("assets/single_figure.html");

/// The page showing every figure (a Tornado template).
const ALL_FIGURES: &str = include_str!("assets/all_figures.html");

/// The loop of [`ALL_FIGURES`] over the figures.
const FIGURE_LOOP: (&str, &str) = ("{% for (fig_id, fig_manager) in figures %}", "{% end %}");

/// The toolbar: name, tooltip, icon and method of each button, `None`
/// between groups (`NavigationToolbar2WebAgg.toolitems`).
const TOOLBAR: [Option<[&str; 4]>; 8] = [
    Some(["Home", "Reset original view", "home", "home"]),
    Some(["Back", "Back to previous view", "back", "back"]),
    Some(["Forward", "Forward to next view", "forward", "forward"]),
    None,
    Some([
        "Pan",
        "Left button pans, Right button zooms\nx/y fixes axis, CTRL fixes aspect",
        "move",
        "pan",
    ]),
    Some([
        "Zoom",
        "Zoom to rectangle\nx/y fixes axis",
        "zoom_to_rect",
        "zoom",
    ]),
    None,
    Some(["Download", "Download plot", "filesave", "download"]),
];

/// The download formats offered, one extension per format in
/// matplotlib's order (the formats of its list plt-rs writes).
const DOWNLOADS: [SaveFormat; 10] = [
    SaveFormat::Eps,
    SaveFormat::Gif,
    SaveFormat::Jpeg,
    SaveFormat::Pdf,
    SaveFormat::Png,
    SaveFormat::Ps,
    SaveFormat::Raw,
    SaveFormat::Svg,
    SaveFormat::Tiff,
    SaveFormat::Webp,
];

/// The extension a format is offered under (matplotlib's first extension
/// of each format: `jpeg` rather than `jpg`).
fn offered_extension(format: SaveFormat) -> &'static str {
    match format {
        SaveFormat::Jpeg => "jpeg",
        other => other.extension(),
    }
}

/// The client script with the toolbar, the download formats and
/// `default_extension` preselected (`FigureManagerWebAgg.get_javascript`).
pub(crate) fn script(default_extension: &str) -> String {
    let items: Vec<String> = TOOLBAR
        .iter()
        .map(|item| {
            let fields = item.unwrap_or([""; 4]).map(json::string);
            format!("[{}]", fields.join(", "))
        })
        .collect();
    let extensions: Vec<String> = DOWNLOADS
        .iter()
        .map(|&format| json::string(offered_extension(format)))
        .collect();
    let mut out = String::with_capacity(MPL_JS.len() + 1024);
    out.push_str(MPL_JS);
    // Writing to a String cannot fail.
    let _ = write!(
        out,
        "mpl.toolbar_items = [{}];\n\nmpl.extensions = [{}];\n\nmpl.default_extension = {};",
        items.join(", "),
        extensions.join(", "),
        json::string(default_extension)
    );
    out
}

/// Fills a page template's placeholders as matplotlib's server does (the
/// pages live at the root, so the URL prefix is empty).
fn fill(template: &str, ws_uri: &str, chrome: ChromeLook) -> String {
    let page = template
        .replace("{{ prefix }}", "")
        .replace("{{ ws_uri }}", ws_uri);
    match chrome {
        ChromeLook::Matplotlib => page,
        ChromeLook::Modern => page.replacen(
            MPL_STYLESHEET,
            &format!(
                "{MPL_STYLESHEET}\n    <link rel=\"stylesheet\" href=\"{}\" type=\"text/css\">",
                modern::STYLESHEET
            ),
            1,
        ),
    }
}

/// The link to matplotlib's own style sheet in the page templates, which
/// the modern chrome's follows.
const MPL_STYLESHEET: &str =
    r#"<link rel="stylesheet" href="/_static/css/mpl.css" type="text/css">"#;

/// The page of figure `number` in the `chrome` look; `ws_uri` is the
/// WebSocket root (`ws://host/`).
pub(crate) fn single_figure_page(ws_uri: &str, number: u32, chrome: ChromeLook) -> String {
    fill(SINGLE_FIGURE, ws_uri, chrome).replace("{{ str(fig_id) }}", &number.to_string())
}

/// The page of every figure, in the order of `numbers`, in the `chrome`
/// look.
pub(crate) fn all_figures_page(ws_uri: &str, numbers: &[u32], chrome: ChromeLook) -> String {
    let page = fill(ALL_FIGURES, ws_uri, chrome);
    let (head, rest) = page
        .split_once(FIGURE_LOOP.0)
        .expect("the page template loops over the figures");
    let (body, tail) = rest
        .split_once(FIGURE_LOOP.1)
        .expect("the figure loop of the page template ends");
    let mut out = head.to_owned();
    for number in numbers {
        out.push_str(&body.replace("{{ str(fig_id) }}", &number.to_string()));
    }
    out.push_str(tail);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_ends_with_the_toolbar_and_formats() {
        let script = script("png");
        assert!(script.starts_with("/* Put everything inside the global mpl namespace */"));
        assert!(
            script.contains(
                r#"mpl.toolbar_items = [["Home", "Reset original view", "home", "home"], "#
            )
        );
        assert!(script.contains(r#"["", "", "", ""], ["Pan", "Left button pans, Right button zooms\nx/y fixes axis, CTRL fixes aspect", "move", "pan"]"#));
        assert!(script.contains(
            r#"mpl.extensions = ["eps", "gif", "jpeg", "pdf", "png", "ps", "raw", "svg", "tif", "webp"];"#
        ));
        assert!(script.ends_with(r#"mpl.default_extension = "png";"#));
        for format in DOWNLOADS {
            assert_eq!(
                offered_extension(format).parse::<SaveFormat>().ok(),
                Some(format)
            );
        }
    }

    #[test]
    fn pages_fill_their_templates() {
        let page = single_figure_page("ws://localhost:8988/", 3, ChromeLook::Matplotlib);
        assert!(page.contains(r#"<script src="/js/mpl.js"></script>"#));
        assert!(page.contains(r#"var uri = "ws://localhost:8988/" + 3 + "/ws";"#));
        assert!(!page.contains("{{") && !page.contains("{%"));
        let page = all_figures_page("ws://h/", &[1, 4], ChromeLook::Matplotlib);
        assert!(page.contains("ready(figure_ready(1));"));
        assert!(page.contains("ready(figure_ready(4));"));
        assert!(!page.contains("{{") && !page.contains("{%"));
        let empty = all_figures_page("ws://h/", &[], ChromeLook::Matplotlib);
        assert!(empty.contains("<div id=\"figures\""));
    }

    #[test]
    fn modern_pages_add_their_style_sheet_after_matplotlibs() {
        let link = format!(r#"<link rel="stylesheet" href="{}""#, modern::STYLESHEET);
        for page in [
            single_figure_page("ws://h/", 1, ChromeLook::Modern),
            all_figures_page("ws://h/", &[1], ChromeLook::Modern),
        ] {
            let mpl = page.find(MPL_STYLESHEET).expect("matplotlib's style sheet");
            assert!(page.find(&link).is_some_and(|modern| modern > mpl));
        }
        let classic = single_figure_page("ws://h/", 1, ChromeLook::Matplotlib);
        assert!(!classic.contains(&link));
    }

    #[test]
    fn static_files_are_found() {
        let icon = find("/_images/zoom_to_rect_large.png").expect("an icon");
        assert!(icon.bytes.starts_with(b"\x89PNG"));
        assert_eq!(
            find("/_static/css/mpl.css").map(|a| a.content_type),
            Some(CSS)
        );
        assert_eq!(find("/_static/../LICENSE"), None);
    }
}
