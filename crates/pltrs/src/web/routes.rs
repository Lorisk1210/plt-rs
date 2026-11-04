//! The URLs of matplotlib's `WebAgg` server and the answers to plain HTTP
//! requests: the pages, the client script, static files and downloads.

use std::sync::mpsc;
use std::time::Duration;

use super::assets::{self, Asset};
use super::http::{Request, Response, Status};
use super::modern::{self, ModernFile};
use super::net::{DownloadError, Inbound, Shared};
use crate::figure::SaveFormat;
use crate::look::ChromeLook;

/// How long a download may wait for the server's thread to render it.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

/// What a URL path names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    /// A static file.
    Static(&'static Asset),
    /// A file of the modern chrome that matplotlib does not have.
    Modern(&'static ModernFile),
    /// The client script (`/js/mpl.js`).
    Script,
    /// The page of every figure (`/`).
    AllFigures,
    /// The page of one figure (`/<number>`).
    Figure(u32),
    /// The WebSocket of a figure (`/<number>/ws`).
    WebSocket(u32),
    /// A figure saved in a format (`/<number>/download.<format>`).
    Download(u32, SaveFormat),
    /// Nothing.
    NotFound,
}

/// A figure number: decimal digits that fit.
fn figure_number(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// The route of URL path `path` under the URL prefix `prefix` (empty for
/// none; matplotlib's `url_prefix`).
pub(crate) fn route(path: &str, prefix: &str) -> Route {
    let Some(path) = path.strip_prefix(prefix) else {
        return Route::NotFound;
    };
    if let Some(asset) = assets::find(path) {
        return Route::Static(asset);
    }
    if let Some(file) = modern::find(path) {
        return Route::Modern(file);
    }
    match path {
        "" | "/" => return Route::AllFigures,
        "/js/mpl.js" => return Route::Script,
        _ => {}
    }
    let Some(rest) = path.strip_prefix('/') else {
        return Route::NotFound;
    };
    let (number, tail) = match rest.split_once('/') {
        Some((number, tail)) => (number, Some(tail)),
        None => (rest, None),
    };
    let Some(number) = figure_number(number) else {
        return Route::NotFound;
    };
    match tail {
        None => Route::Figure(number),
        Some("ws") => Route::WebSocket(number),
        Some(tail) => tail
            .strip_prefix("download.")
            .filter(|format| {
                !format.is_empty()
                    && format
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.')
            })
            .and_then(|format| format.parse().ok())
            .map_or(Route::NotFound, |format| Route::Download(number, format)),
    }
}

/// The media type of a saved figure.
fn content_type(format: SaveFormat) -> &'static str {
    match format {
        SaveFormat::Png => "image/png",
        SaveFormat::Svg | SaveFormat::Svgz => "image/svg+xml",
        SaveFormat::Pdf => "application/pdf",
        SaveFormat::Ps | SaveFormat::Eps => "application/postscript",
        SaveFormat::Pgf => "text/plain; charset=utf-8",
        SaveFormat::Jpeg => "image/jpeg",
        SaveFormat::Tiff => "image/tiff",
        SaveFormat::Gif => "image/gif",
        SaveFormat::Webp => "image/webp",
        SaveFormat::Avif => "image/avif",
        SaveFormat::Raw => "application/octet-stream",
    }
}

/// Whether `host` is a plain `name[:port]` or `[v6]:port` authority, safe
/// to write into a page.
fn is_plain_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 255
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
}

/// The WebSocket root of the pages (`ws://host/`, the URL prefix
/// included), from the request's `Host`, or the server's address for
/// clients that send none.
fn ws_uri(request: &Request, shared: &Shared) -> Option<String> {
    let prefix = &shared.url_prefix;
    match request.header("host") {
        Some(host) => is_plain_host(host).then(|| format!("ws://{host}{prefix}/")),
        None => Some(format!("ws://{}{prefix}/", shared.local_addr)),
    }
}

/// The answer to a plain (not upgrading) request for `route`.
pub(crate) fn respond(route: Route, request: &Request, shared: &Shared) -> Response {
    match route {
        Route::Static(asset) => match modern::find(asset.path) {
            // The modern chrome replaces matplotlib's toolbar images.
            Some(file) if shared.chrome == ChromeLook::Modern => modern_file(file),
            _ => Response::ok(asset.content_type, asset.bytes),
        },
        Route::Modern(file) => modern_file(file),
        Route::Script => Response::ok(
            "text/javascript; charset=utf-8",
            shared.script.as_bytes().to_vec(),
        ),
        Route::AllFigures | Route::Figure(_) => {
            let Some(ws_uri) = ws_uri(request, shared) else {
                return Response::error(Status::BadRequest);
            };
            let root = assets::PageRoot {
                ws_uri: &ws_uri,
                prefix: &shared.url_prefix,
            };
            let page = match route {
                Route::Figure(number) => match shared.figure(number) {
                    Some(figure) => assets::single_figure_page(root, figure, shared.chrome),
                    None => return Response::error(Status::NotFound),
                },
                _ => assets::all_figures_page(root, &shared.figures(), shared.chrome),
            };
            Response::ok("text/html; charset=utf-8", page.into_bytes())
        }
        Route::Download(number, format) => download(number, format, shared),
        // An upgrade is answered elsewhere; a plain request cannot use it.
        Route::WebSocket(_) => Response::error(Status::BadRequest),
        Route::NotFound => Response::error(Status::NotFound),
    }
}

/// A file of the modern chrome, as it is.
fn modern_file(file: &'static ModernFile) -> Response {
    Response::ok(file.content_type, &*file.bytes)
}

/// Asks the server's thread to save figure `number` in `format`.
fn download(number: u32, format: SaveFormat, shared: &Shared) -> Response {
    let (reply, answer) = mpsc::channel();
    if shared
        .send(Inbound::Download {
            figure: number,
            format,
            reply,
        })
        .is_err()
    {
        return Response::error(Status::ServiceUnavailable);
    }
    match answer.recv_timeout(DOWNLOAD_TIMEOUT) {
        Ok(Ok(bytes)) => {
            let response = Response::ok(content_type(format), bytes);
            if format == SaveFormat::Svgz {
                response.with_header("Content-Encoding", "gzip")
            } else {
                response
            }
        }
        Ok(Err(DownloadError::NoFigure)) => Response::error(Status::NotFound),
        Ok(Err(DownloadError::Failed)) => Response::error(Status::InternalServerError),
        Err(_) => Response::error(Status::ServiceUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_route_like_matplotlib() {
        assert_eq!(route("/", ""), Route::AllFigures);
        assert_eq!(route("", ""), Route::AllFigures);
        assert_eq!(route("/js/mpl.js", ""), Route::Script);
        assert_eq!(route("/2", ""), Route::Figure(2));
        assert_eq!(route("/007", ""), Route::Figure(7));
        assert_eq!(route("/2/ws", ""), Route::WebSocket(2));
        assert_eq!(
            route("/2/download.svg", ""),
            Route::Download(2, SaveFormat::Svg)
        );
        assert_eq!(
            route("/2/download.jpeg", ""),
            Route::Download(2, SaveFormat::Jpeg)
        );
        assert!(matches!(route("/favicon.ico", ""), Route::Static(_)));
        assert_eq!(
            route("/2/download.webp", ""),
            Route::Download(2, SaveFormat::Webp)
        );
        assert_eq!(
            route("/2/download.avif", ""),
            Route::Download(2, SaveFormat::Avif)
        );
        for missing in [
            "/2/",
            "/x",
            "/-1",
            "/99999999999",
            "/2/download.",
            "/2/download.PNG",
            "/2/download.heic",
            "/2/ws/more",
            "//",
            "/_static/js/mpl.js",
            "/_images/../../etc/passwd",
        ] {
            assert_eq!(route(missing, ""), Route::NotFound, "{missing}");
        }
    }

    #[test]
    fn urls_route_under_a_prefix() {
        assert_eq!(route("/mpl", "/mpl"), Route::AllFigures);
        assert_eq!(route("/mpl/2/ws", "/mpl"), Route::WebSocket(2));
        assert_eq!(route("/mpl/js/mpl.js", "/mpl"), Route::Script);
        assert!(matches!(
            route("/mpl/_static/css/mpl.css", "/mpl"),
            Route::Static(_)
        ));
        for outside in ["/2", "/mplx/2", "/js/mpl.js", ""] {
            assert_eq!(route(outside, "/mpl"), Route::NotFound, "{outside}");
        }
    }

    #[test]
    fn hosts_are_checked_before_use() {
        assert!(is_plain_host("127.0.0.1:8988"));
        assert!(is_plain_host("[::1]:8988"));
        assert!(is_plain_host("my-host.local"));
        assert!(!is_plain_host("evil\";alert(1);//"));
        assert!(!is_plain_host("a b"));
        assert!(!is_plain_host(""));
        assert_eq!(content_type(SaveFormat::Eps), "application/postscript");
    }
}
