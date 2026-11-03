//! The browser client: matplotlib's pages, script, style sheets and
//! toolbar icons, embedded unchanged (their license is in
//! `assets/LICENSE`; the toolbar icons are shared with the figure
//! windows, see [`crate::gui::images`]), and the parts matplotlib's server
//! generates; the modern chrome's additions are in [`super::modern`].

use std::fmt::Write as _;

use pltrs_core::json;

use super::modern;
use super::net::ServedFigure;
use crate::figure::SaveFormat;
use crate::gui::images;
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

/// A toolbar image of [`images`], which the figure windows show too.
macro_rules! toolbar_png {
    ($path:literal, $bytes:expr) => {
        Asset {
            path: $path,
            content_type: PNG,
            bytes: $bytes,
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
    toolbar_png!("/_images/home.png", images::HOME.normal),
    toolbar_png!("/_images/home_large.png", images::HOME.large),
    toolbar_png!("/_images/back.png", images::BACK.normal),
    toolbar_png!("/_images/back_large.png", images::BACK.large),
    toolbar_png!("/_images/forward.png", images::FORWARD.normal),
    toolbar_png!("/_images/forward_large.png", images::FORWARD.large),
    toolbar_png!("/_images/move.png", images::MOVE.normal),
    toolbar_png!("/_images/move_large.png", images::MOVE.large),
    toolbar_png!("/_images/zoom_to_rect.png", images::ZOOM_TO_RECT.normal),
    toolbar_png!(
        "/_images/zoom_to_rect_large.png",
        images::ZOOM_TO_RECT.large
    ),
    toolbar_png!("/_images/filesave.png", images::FILESAVE.normal),
    toolbar_png!("/_images/filesave_large.png", images::FILESAVE.large),
    asset!("/favicon.ico", PNG, "images/matplotlib.png"),
];

/// The static file at URL path `path`.
pub(crate) fn find(path: &str) -> Option<&'static Asset> {
    STATIC.iter().find(|asset| asset.path == path)
}

/// matplotlib's client script.
pub(crate) const MPL_JS: &str = include_str!("assets/js/mpl.js");

/// The page showing one figure (a Tornado template).
const SINGLE_FIGURE: &str = include_str!("assets/single_figure.html");

/// The page showing every figure (a Tornado template).
const ALL_FIGURES: &str = include_str!("assets/all_figures.html");

/// The files of matplotlib's browser client as its `web_backend`
/// directory holds them, each with its path there: the static files the
/// server serves under `/_static/`, the pages, the scripts the server
/// writes or does not use (the notebook's) and their license.
pub(crate) fn client_files() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    let served = STATIC.iter().filter_map(|asset| {
        let path = asset.path.strip_prefix("/_static/")?;
        Some((path, asset.bytes))
    });
    let others: [(&str, &[u8]); 6] = [
        ("all_figures.html", ALL_FIGURES.as_bytes()),
        ("single_figure.html", SINGLE_FIGURE.as_bytes()),
        (
            "ipython_inline_figure.html",
            include_bytes!("assets/ipython_inline_figure.html"),
        ),
        ("js/mpl.js", MPL_JS.as_bytes()),
        ("js/nbagg_mpl.js", include_bytes!("assets/js/nbagg_mpl.js")),
        ("LICENSE", include_bytes!("assets/LICENSE")),
    ];
    served.chain(others)
}

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
/// matplotlib's order (the formats of its list pltrs writes).
const DOWNLOADS: [SaveFormat; 12] = [
    SaveFormat::Avif,
    SaveFormat::Eps,
    SaveFormat::Gif,
    SaveFormat::Jpeg,
    SaveFormat::Pgf,
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
/// pages live at the root, so the URL prefix is empty), for a page showing
/// `figures` in the `chrome` look.
fn fill(template: &str, ws_uri: &str, chrome: ChromeLook, figures: &[ServedFigure]) -> String {
    let page = template
        .replace("{{ prefix }}", "")
        .replace("{{ ws_uri }}", ws_uri);
    match chrome {
        ChromeLook::Matplotlib => page,
        ChromeLook::Modern => modern::dress(&page, figures),
    }
}

/// The page of `figure` in the `chrome` look; `ws_uri` is the WebSocket
/// root (`ws://host/`).
pub(crate) fn single_figure_page(ws_uri: &str, figure: ServedFigure, chrome: ChromeLook) -> String {
    fill(SINGLE_FIGURE, ws_uri, chrome, &[figure])
        .replace("{{ str(fig_id) }}", &figure.number.to_string())
}

/// The page of every figure, in the order of `figures`, in the `chrome`
/// look.
pub(crate) fn all_figures_page(
    ws_uri: &str,
    figures: &[ServedFigure],
    chrome: ChromeLook,
) -> String {
    let page = fill(ALL_FIGURES, ws_uri, chrome, figures);
    let (head, rest) = page
        .split_once(FIGURE_LOOP.0)
        .expect("the page template loops over the figures");
    let (body, tail) = rest
        .split_once(FIGURE_LOOP.1)
        .expect("the figure loop of the page template ends");
    let mut out = head.to_owned();
    for figure in figures {
        out.push_str(&body.replace("{{ str(fig_id) }}", &figure.number.to_string()));
    }
    out.push_str(tail);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_client_files_are_matplotlibs_web_backend() {
        let paths: Vec<&str> = client_files().map(|(path, _)| path).collect();
        assert_eq!(
            paths,
            [
                "css/page.css",
                "css/boilerplate.css",
                "css/fbm.css",
                "css/mpl.css",
                "js/mpl_tornado.js",
                "all_figures.html",
                "single_figure.html",
                "ipython_inline_figure.html",
                "js/mpl.js",
                "js/nbagg_mpl.js",
                "LICENSE",
            ]
        );
        let nbagg = client_files().find(|(path, _)| *path == "js/nbagg_mpl.js");
        assert!(nbagg.is_some_and(|(_, js)| js.starts_with(b"/* global mpl */")));
    }

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
            r#"mpl.extensions = ["avif", "eps", "gif", "jpeg", "pgf", "pdf", "png", "ps", "raw", "svg", "tif", "webp"];"#
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
        let page = single_figure_page(
            "ws://localhost:8988/",
            ServedFigure::new(3, false),
            ChromeLook::Matplotlib,
        );
        assert!(page.contains(r#"<script src="/js/mpl.js"></script>"#));
        assert!(page.contains(r#"var uri = "ws://localhost:8988/" + 3 + "/ws";"#));
        assert!(!page.contains("{{") && !page.contains("{%"));
        let figures = [ServedFigure::new(1, false), ServedFigure::new(4, true)];
        let page = all_figures_page("ws://h/", &figures, ChromeLook::Matplotlib);
        assert!(page.contains("ready(figure_ready(1));"));
        assert!(page.contains("ready(figure_ready(4));"));
        assert!(!page.contains("{{") && !page.contains("{%"));
        let empty = all_figures_page("ws://h/", &[], ChromeLook::Matplotlib);
        assert!(empty.contains("<div id=\"figures\""));
    }

    #[test]
    fn modern_pages_load_the_chrome_and_name_dark_figures() {
        let figures = [ServedFigure::new(1, false), ServedFigure::new(4, true)];
        for page in [
            single_figure_page("ws://h/", ServedFigure::new(4, true), ChromeLook::Modern),
            all_figures_page("ws://h/", &figures, ChromeLook::Modern),
        ] {
            assert!(
                page.contains(r#"<link rel="stylesheet" href="/_static/css/pltrs-modern.css""#)
            );
            assert!(page.contains(r#"<script src="/_static/js/pltrs-modern.js"></script>"#));
            assert!(page.contains(r#" data-pltrs-dark="4">"#));
        }
        let modern = all_figures_page("ws://h/", &figures, ChromeLook::Modern);
        assert!(modern.contains("<title>Figures</title>"));
        let classic = all_figures_page("ws://h/", &figures, ChromeLook::Matplotlib);
        assert!(!classic.contains("pltrs-modern") && !classic.contains("pltrs-dark"));
        assert!(classic.contains("<title>MPL | WebAgg current figures</title>"));
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
