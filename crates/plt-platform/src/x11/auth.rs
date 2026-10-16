//! `MIT-MAGIC-COOKIE-1` credentials from Xauthority files.
//!
//! The file is a sequence of entries, each a big-endian `u16` family
//! followed by four length-prefixed (`u16` big-endian) byte strings:
//! address, display number (ASCII decimal), auth name and auth data.

use std::path::PathBuf;

/// Address family of a TCP/IPv4 server.
pub(crate) const FAMILY_INTERNET: u16 = 0;
/// Address family of a TCP/IPv6 server.
pub(crate) const FAMILY_INTERNET6: u16 = 6;
/// Address family of a local (Unix-socket) server; the address is the
/// host name.
pub(crate) const FAMILY_LOCAL: u16 = 256;
/// Entry matching any address.
const FAMILY_WILD: u16 = 0xffff;

/// The only authorization protocol this crate implements.
pub(crate) const MIT_MAGIC_COOKIE: &[u8] = b"MIT-MAGIC-COOKIE-1";

/// One Xauthority entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthEntry {
    family: u16,
    address: Vec<u8>,
    number: Vec<u8>,
    name: Vec<u8>,
    data: Vec<u8>,
}

/// Credentials sent in the connection setup.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Credentials {
    /// Authorization protocol name.
    pub(crate) name: Vec<u8>,
    /// Authorization data (the cookie).
    pub(crate) data: Vec<u8>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the secret cookie.
        f.debug_struct("Credentials")
            .field("name", &String::from_utf8_lossy(&self.name))
            .finish_non_exhaustive()
    }
}

/// Parses an Xauthority file. A truncated trailing entry is ignored, as
/// Xlib does.
pub(crate) fn parse(mut data: &[u8]) -> Vec<AuthEntry> {
    let mut entries = Vec::new();
    while let Some(entry) = parse_entry(&mut data) {
        entries.push(entry);
    }
    entries
}

fn parse_entry(data: &mut &[u8]) -> Option<AuthEntry> {
    let family = take_u16(data)?;
    let mut field = || -> Option<Vec<u8>> {
        let len = usize::from(take_u16(data)?);
        let (bytes, rest) = data.split_at_checked(len)?;
        *data = rest;
        Some(bytes.to_vec())
    };
    Some(AuthEntry {
        family,
        address: field()?,
        number: field()?,
        name: field()?,
        data: field()?,
    })
}

fn take_u16(data: &mut &[u8]) -> Option<u16> {
    let (bytes, rest) = data.split_first_chunk::<2>()?;
    *data = rest;
    Some(u16::from_be_bytes(*bytes))
}

/// Finds the cookie for a server, following `XauGetBestAuthByAddr`: the
/// entry's family and address must match (or the family be "wild"), and
/// its display number must match or be empty.
pub(crate) fn find(
    entries: &[AuthEntry],
    family: u16,
    address: &[u8],
    display: u16,
) -> Option<Credentials> {
    let number = display.to_string();
    entries
        .iter()
        .find(|entry| {
            let address_ok =
                entry.family == FAMILY_WILD || (entry.family == family && entry.address == address);
            let number_ok = entry.number.is_empty() || entry.number == number.as_bytes();
            address_ok && number_ok && entry.name == MIT_MAGIC_COOKIE
        })
        .map(|entry| Credentials {
            name: entry.name.clone(),
            data: entry.data.clone(),
        })
}

/// The Xauthority file: `$XAUTHORITY`, else `$HOME/.Xauthority`.
pub(crate) fn file_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XAUTHORITY").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").filter(|p| !p.is_empty())?;
    Some(PathBuf::from(home).join(".Xauthority"))
}

/// Loads credentials for a server from the user's Xauthority file. A
/// missing or unreadable file means "no credentials": many servers accept
/// local clients without a cookie.
pub(crate) fn load(family: u16, address: &[u8], display: u16) -> Option<Credentials> {
    let data = std::fs::read(file_path()?).ok()?;
    find(&parse(&data), family, address, display)
}

/// This machine's host name, as Xauthority's local entries record it.
pub(crate) fn hostname() -> Option<String> {
    let from_file = |path: &str| {
        std::fs::read_to_string(path)
            .ok()
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
    };
    from_file("/proc/sys/kernel/hostname")
        .or_else(|| from_file("/etc/hostname"))
        .or_else(|| std::env::var("HOSTNAME").ok().filter(|name| !name.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_bytes(family: u16, address: &[u8], number: &[u8], name: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = family.to_be_bytes().to_vec();
        for field in [address, number, name, data] {
            out.extend_from_slice(&(field.len() as u16).to_be_bytes());
            out.extend_from_slice(field);
        }
        out
    }

    /// The layout of a real XWayland (mutter) Xauthority file: one local and
    /// one wild entry, both with an empty display number.
    fn xwayland_file() -> Vec<u8> {
        let cookie = [0x7d, 0xd6, 0xea, 0x92, 0x35, 0x95, 0x75, 0xbd];
        let mut file = entry_bytes(FAMILY_LOCAL, b"fedora.fritz.box", b"", MIT_MAGIC_COOKIE, &cookie);
        file.extend(entry_bytes(FAMILY_WILD, b"fedora.fritz.box", b"", MIT_MAGIC_COOKIE, &[9; 8]));
        file
    }

    #[test]
    fn parses_entries() {
        let entries = parse(&xwayland_file());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].family, FAMILY_LOCAL);
        assert_eq!(entries[0].address, b"fedora.fritz.box");
        assert!(entries[0].number.is_empty());
        assert_eq!(entries[1].family, FAMILY_WILD);
    }

    #[test]
    fn truncated_tail_is_ignored() {
        let mut file = xwayland_file();
        file.extend_from_slice(&[1, 0, 0, 5, b'a']);
        assert_eq!(parse(&file).len(), 2);
        assert!(parse(&[]).is_empty());
        assert!(parse(&[0]).is_empty());
    }

    #[test]
    fn local_entry_matches_by_hostname_any_display() {
        let entries = parse(&xwayland_file());
        let creds = find(&entries, FAMILY_LOCAL, b"fedora.fritz.box", 0).unwrap();
        assert_eq!(creds.name, MIT_MAGIC_COOKIE);
        assert_eq!(creds.data[0], 0x7d);
        // Other hosts fall through to the wild entry.
        let creds = find(&entries, FAMILY_LOCAL, b"elsewhere", 3).unwrap();
        assert_eq!(creds.data, [9; 8]);
    }

    #[test]
    fn display_number_must_match_when_present() {
        let mut file = entry_bytes(FAMILY_LOCAL, b"h", b"1", MIT_MAGIC_COOKIE, &[1]);
        file.extend(entry_bytes(FAMILY_LOCAL, b"h", b"0", MIT_MAGIC_COOKIE, &[0]));
        let entries = parse(&file);
        assert_eq!(find(&entries, FAMILY_LOCAL, b"h", 0).unwrap().data, [0]);
        assert_eq!(find(&entries, FAMILY_LOCAL, b"h", 1).unwrap().data, [1]);
        assert!(find(&entries, FAMILY_LOCAL, b"h", 2).is_none());
        assert!(find(&entries, FAMILY_INTERNET, b"h", 0).is_none());
    }

    #[test]
    fn other_auth_protocols_are_skipped() {
        let mut file = entry_bytes(FAMILY_LOCAL, b"h", b"", b"XDM-AUTHORIZATION-1", &[1]);
        file.extend(entry_bytes(FAMILY_LOCAL, b"h", b"", MIT_MAGIC_COOKIE, &[2]));
        assert_eq!(find(&parse(&file), FAMILY_LOCAL, b"h", 0).unwrap().data, [2]);
    }

    #[test]
    fn credentials_debug_hides_cookie() {
        let creds = Credentials {
            name: MIT_MAGIC_COOKIE.to_vec(),
            data: vec![0xab; 16],
        };
        let text = format!("{creds:?}");
        assert!(text.contains("MIT-MAGIC-COOKIE-1") && !text.contains("171"));
    }
}
