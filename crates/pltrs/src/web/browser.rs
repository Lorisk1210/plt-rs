//! Pointing the user at the served figures: opening the system's web
//! browser (Python's `webbrowser.open`, as matplotlib's `WebAgg` uses it)
//! and printing the address.

use std::ffi::OsString;
use std::io;
use std::process::{Command, Stdio};
use std::thread;

/// The commands that open a URL on this system, tried in order.
fn openers() -> Vec<Vec<OsString>> {
    let mut openers: Vec<Vec<OsString>> = std::env::var_os("BROWSER")
        .map(|list| {
            std::env::split_paths(&list)
                .filter(|browser| !browser.as_os_str().is_empty())
                .map(|browser| vec![browser.into_os_string()])
                .collect()
        })
        .unwrap_or_default();
    let defaults: &[&[&str]] = if cfg!(target_os = "macos") {
        &[&["open"]]
    } else if cfg!(windows) {
        &[&["cmd", "/C", "start", ""]]
    } else {
        // `wslview` opens the Windows browser from WSL.
        &[&["xdg-open"], &["wslview"]]
    };
    openers.extend(
        defaults
            .iter()
            .map(|command| command.iter().map(OsString::from).collect()),
    );
    openers
}

/// Opens `url` in a web browser: starts the first opener that exists,
/// without waiting for the browser.
pub(crate) fn open(url: &str) -> io::Result<()> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, "no web browser found");
    for opener in openers() {
        let Some((program, args)) = opener.split_first() else {
            continue;
        };
        let started = Command::new(program)
            .args(args)
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match started {
            Ok(mut child) => {
                // Reap the opener whenever it exits.
                let _ = thread::Builder::new()
                    .name("pltrs-web-browser".into())
                    .spawn(move || child.wait());
                return Ok(());
            }
            Err(err) => last = err,
        }
    }
    Err(last)
}

/// Tells the user where the figures are (matplotlib prints the same line)
/// and, if `open_browser` is set, opens them in a browser. The address is printed
/// in any case: an opener may start and still find no browser, as on a
/// remote machine.
#[expect(
    clippy::print_stderr,
    reason = "like matplotlib's WebAgg, the server tells the user where the figures are"
)]
pub(crate) fn announce(url: &str, open_browser: bool) {
    if open_browser && let Err(err) = open(url) {
        eprintln!("Cannot open a web browser: {err}");
    }
    eprintln!("To view figure, visit {url}");
}
