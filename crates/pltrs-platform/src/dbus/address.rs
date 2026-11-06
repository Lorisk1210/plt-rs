//! D-Bus server addresses ("Server Addresses" of the specification):
//! `unix:path=/run/user/1000/bus`, `unix:abstract=/tmp/dbus-x,guid=...`,
//! alternatives separated by `;`, values percent-escaped.

use std::path::PathBuf;

/// Where a Unix-socket bus listens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SocketAddress {
    /// A socket file.
    Path(PathBuf),
    /// A name in Linux's abstract socket namespace.
    Abstract(Vec<u8>),
}

/// The value of an address, `%XX` escapes resolved; `None` if one is
/// broken.
fn unescape(value: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut rest = value.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%' {
            let digits = tail.get(..2)?;
            let text = std::str::from_utf8(digits).ok()?;
            bytes.push(u8::from_str_radix(text, 16).ok()?);
            rest = &tail[2..];
        } else {
            bytes.push(byte);
            rest = tail;
        }
    }
    Some(bytes)
}

/// The Unix-socket addresses among `addresses`, in their order.
pub(crate) fn parse(addresses: &str) -> Vec<SocketAddress> {
    addresses
        .split(';')
        .filter_map(|address| {
            let options = address.strip_prefix("unix:")?;
            options.split(',').find_map(|option| {
                let (key, value) = option.split_once('=')?;
                let value = unescape(value)?;
                match key {
                    "path" => Some(SocketAddress::Path(PathBuf::from(
                        String::from_utf8(value).ok()?,
                    ))),
                    "abstract" => Some(SocketAddress::Abstract(value)),
                    _ => None,
                }
            })
        })
        .collect()
}

/// The session bus's addresses: `$DBUS_SESSION_BUS_ADDRESS`, else the
/// socket in `$XDG_RUNTIME_DIR` (`bus`), where systemd's user bus is.
pub(crate) fn session() -> Vec<SocketAddress> {
    match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(addresses) if !addresses.is_empty() => parse(&addresses),
        _ => std::env::var_os("XDG_RUNTIME_DIR")
            .map(|runtime| SocketAddress::Path(PathBuf::from(runtime).join("bus")))
            .into_iter()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_addresses_are_parsed() {
        assert_eq!(
            parse("unix:path=/run/user/1000/bus"),
            [SocketAddress::Path("/run/user/1000/bus".into())]
        );
        assert_eq!(
            parse("unix:abstract=/tmp/dbus-AbC,guid=0123;unix:path=/tmp/a%20b%2cc"),
            [
                SocketAddress::Abstract(b"/tmp/dbus-AbC".to_vec()),
                SocketAddress::Path("/tmp/a b,c".into()),
            ]
        );
        assert_eq!(
            parse("tcp:host=localhost,port=1;unix:guid=1,path=/s"),
            [SocketAddress::Path("/s".into())]
        );
        assert_eq!(parse("unix:path=/a%2"), []);
        assert_eq!(parse("unix:path=/a%zz"), []);
        assert_eq!(parse(""), []);
        assert_eq!(parse("unix:tmpdir=/tmp"), [], "for servers only");
    }
}
