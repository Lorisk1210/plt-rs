//! Parsing of X display names (`$DISPLAY`).
//!
//! Supported forms: `:0`, `:0.1`, `unix:0`, `host:0`, `host:0.1`,
//! `tcp/host:0`, `unix/host:0`, and an absolute socket path such as
//! `XQuartz`'s `/private/tmp/com.apple.launchd.x/org.xquartz:0`.

use std::path::PathBuf;

use crate::error::Error;

/// Where the X server listens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    /// The local Unix-domain socket `/tmp/.X11-unix/X<display>`.
    Local,
    /// An explicit Unix-domain socket path.
    Path(PathBuf),
    /// TCP port `6000 + display` on a host.
    Tcp(String),
}

/// A parsed display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DisplayName {
    /// How to reach the server.
    pub(crate) target: Target,
    /// The display number.
    pub(crate) display: u16,
    /// The preferred screen.
    pub(crate) screen: u16,
}

impl DisplayName {
    /// Parses a display name.
    pub(crate) fn parse(name: &str) -> Result<Self, Error> {
        let invalid = || Error::InvalidDisplay(name.to_owned());
        if name.starts_with('/') {
            return Ok(Self {
                target: Target::Path(PathBuf::from(name)),
                display: 0,
                screen: 0,
            });
        }
        let colon = name.rfind(':').ok_or_else(invalid)?;
        let (host_part, number_part) = (&name[..colon], &name[colon + 1..]);
        let (protocol, host) = match host_part.split_once('/') {
            Some((protocol, host)) => (Some(protocol), host),
            None => (None, host_part),
        };
        if host.ends_with(':') {
            // `host::0` is DECnet, which nobody runs any more.
            return Err(invalid());
        }
        let (display, screen) = match number_part.split_once('.') {
            Some((display, screen)) => (display, Some(screen)),
            None => (number_part, None),
        };
        let display = parse_number(display).ok_or_else(invalid)?;
        let screen = screen.map_or(Some(0), parse_number).ok_or_else(invalid)?;
        let target = match (protocol, host) {
            (None | Some("unix" | "local"), "" | "unix") | (Some("unix" | "local"), _) => {
                Target::Local
            }
            (None | Some("tcp" | "inet" | "inet6"), host) => Target::Tcp(host.to_owned()),
            (Some(_), _) => return Err(invalid()),
        };
        Ok(Self {
            target,
            display,
            screen,
        })
    }

    /// The server's local socket path. On Linux the server also listens on
    /// the abstract socket of the same name.
    pub(crate) fn local_socket_path(&self) -> String {
        format!("/tmp/.X11-unix/X{}", self.display)
    }
}

fn parse_number(text: &str) -> Option<u16> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(name: &str) -> (Target, u16, u16) {
        let d = DisplayName::parse(name).unwrap();
        (d.target, d.display, d.screen)
    }

    #[test]
    fn local_forms() {
        assert_eq!(parsed(":0"), (Target::Local, 0, 0));
        assert_eq!(parsed(":1.2"), (Target::Local, 1, 2));
        assert_eq!(parsed("unix:3"), (Target::Local, 3, 0));
        assert_eq!(parsed("unix/anything:4.1"), (Target::Local, 4, 1));
        assert_eq!(
            DisplayName::parse(":7").unwrap().local_socket_path(),
            "/tmp/.X11-unix/X7"
        );
    }

    #[test]
    fn tcp_forms() {
        assert_eq!(
            parsed("localhost:10.0"),
            (Target::Tcp("localhost".into()), 10, 0)
        );
        assert_eq!(
            parsed("tcp/example.org:2"),
            (Target::Tcp("example.org".into()), 2, 0)
        );
        assert_eq!(parsed("10.0.0.1:0"), (Target::Tcp("10.0.0.1".into()), 0, 0));
    }

    #[test]
    fn socket_path_form() {
        let name = "/private/tmp/com.apple.launchd.abc/org.xquartz:0";
        assert_eq!(parsed(name), (Target::Path(PathBuf::from(name)), 0, 0));
    }

    #[test]
    fn invalid_names() {
        for name in [
            "",
            "0",
            ":",
            ":x",
            ":0.",
            ":0.y",
            "host::0",
            "decnet/host:0",
            ":99999",
        ] {
            assert!(
                matches!(DisplayName::parse(name), Err(Error::InvalidDisplay(_))),
                "{name:?} should be rejected"
            );
        }
    }
}
