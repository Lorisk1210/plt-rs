//! Pointing the user at the served figures: opening the system's web
//! browser (Python's `webbrowser.open`, as matplotlib's `WebAgg` uses it)
//! and printing the address.

use std::ffi::{OsStr, OsString};
use std::io;
use std::process::{Command, Stdio};
use std::thread;

use pltrs_core::os::Os;

/// The commands that open a URL on `os`, tried in order: those `$BROWSER`
/// lists (`browsers`, separated as `PATH` is), then the system's opener
/// of its URL associations: `open` on macOS; on Windows the shell's URL
/// handler, which takes the URL as it is (where `cmd /c start` would read
/// its `&`s); `xdg-open` on Linux, or `wslview` under WSL.
fn openers(os: Os, browsers: Option<&OsStr>) -> Vec<Vec<OsString>> {
    let mut openers: Vec<Vec<OsString>> = browsers
        .map(|list| {
            split_list(os, list)
                .into_iter()
                .map(|browser| vec![browser])
                .collect()
        })
        .unwrap_or_default();
    let defaults: &[&[&str]] = match os {
        Os::MacOs => &[&["open"]],
        Os::Windows => &[&["rundll32", "url.dll,FileProtocolHandler"]],
        Os::Linux => &[&["xdg-open"], &["wslview"]],
    };
    openers.extend(
        defaults
            .iter()
            .map(|command| command.iter().map(OsString::from).collect()),
    );
    openers
}

/// The non-empty entries of a list of programs separated as `os`
/// separates `PATH` (`;` on Windows, `:` elsewhere).
fn split_list(os: Os, list: &OsStr) -> Vec<OsString> {
    let separator = if os == Os::Windows { ';' } else { ':' };
    list.to_string_lossy()
        .split(separator)
        .filter(|entry| !entry.is_empty())
        .map(OsString::from)
        .collect()
}

/// Opens `url` in a web browser: starts the first opener that exists,
/// without waiting for the browser.
pub(crate) fn open(url: &str) -> io::Result<()> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, "no web browser found");
    for opener in openers(Os::CURRENT, std::env::var_os("BROWSER").as_deref()) {
        let Some((program, args)) = opener.split_first() else {
            continue;
        };
        let started = crate::process::spawn(
            Command::new(program)
                .args(args)
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(os: Os, browsers: Option<&str>) -> Vec<Vec<String>> {
        openers(os, browsers.map(OsStr::new))
            .into_iter()
            .map(|command| {
                command
                    .iter()
                    .map(|part| part.to_string_lossy().into_owned())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn each_system_opens_urls_its_own_way() {
        assert_eq!(commands(Os::Linux, None), [["xdg-open"], ["wslview"]]);
        assert_eq!(commands(Os::MacOs, None), [["open"]]);
        assert_eq!(
            commands(Os::Windows, None),
            [["rundll32", "url.dll,FileProtocolHandler"]]
        );
    }

    #[test]
    fn the_browser_variable_comes_first() {
        assert_eq!(
            commands(Os::Linux, Some("firefox::chromium")),
            [
                vec!["firefox"],
                vec!["chromium"],
                vec!["xdg-open"],
                vec!["wslview"]
            ]
        );
        assert_eq!(
            commands(Os::Windows, Some(r"C:\Firefox\firefox.exe;")),
            [
                vec![r"C:\Firefox\firefox.exe"],
                vec!["rundll32", "url.dll,FileProtocolHandler"]
            ]
        );
        assert_eq!(commands(Os::MacOs, Some("")), [["open"]]);
    }
}
