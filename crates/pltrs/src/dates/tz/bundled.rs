//! The time zones `dateutil` bundles (`dateutil.zoneinfo`): its
//! `dateutil-zoneinfo.tar.gz`, as python-dateutil 2.9.0 ships it, a
//! gzip-compressed tar archive of the "fat" `TZif` files of tzdata 2024a
//! (whose zones are in the public domain).
//!
//! `dateutil.tz.gettz` falls back to the archive for a name no system
//! directory holds. Windows has no such directory, so there every zone
//! matplotlib knows comes from it. The archive is read the first time a
//! zone is looked up in it.
//!
//! A link of the archive is the zone it links to, as in dateutil, whose
//! `tzfile` for a link is the one of its target, under the target's name
//! (`Europe/Berlin` is a link to `Atlantic/Jan_Mayen` there).

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::OnceLock;

/// The archive.
const ARCHIVE: &[u8] = include_bytes!("dateutil-zoneinfo.tar.gz");

/// The archive's file that is no zone (`dateutil.zoneinfo.METADATA_FN`).
const METADATA: &str = "METADATA";

/// The size of a tar block, and of a tar header.
const BLOCK: usize = 512;

/// The zones of the archive, once read.
static ZONES: OnceLock<Zones> = OnceLock::new();

/// The archive, read.
#[derive(Debug, Default)]
struct Zones {
    /// The tar archive the gzip member holds.
    tar: Vec<u8>,
    /// Each name the archive knows, with the name of the zone it is and
    /// where that zone's data lies in `tar`.
    names: BTreeMap<String, (String, Range<usize>)>,
}

/// The zone the archive calls `name`: the name of the zone it is (the
/// target, for a link) and its `TZif` data. `None` when the archive has
/// no such zone.
pub(super) fn zone(name: &str) -> Option<(&'static str, &'static [u8])> {
    let zones = ZONES.get_or_init(|| read(ARCHIVE).unwrap_or_default());
    let (zone, range) = zones.names.get(name)?;
    Some((zone.as_str(), zones.tar.get(range.clone())?))
}

/// The zones of the gzip-compressed tar archive `archive`, `None` when it
/// is not one.
fn read(archive: &[u8]) -> Option<Zones> {
    let tar = gunzip(archive)?;
    let mut names = BTreeMap::new();
    let mut links = Vec::new();
    for entry in entries(&tar) {
        let entry = entry?;
        match entry.kind {
            Kind::File if entry.name != METADATA => {
                names.insert(entry.name.clone(), (entry.name, entry.data));
            }
            Kind::Link(target) => links.push((entry.name, target)),
            Kind::File | Kind::Other => {}
        }
    }
    for (name, target) in links {
        if let Some(zone) = names.get(&target).cloned() {
            names.insert(name, zone);
        }
    }
    Some(Zones { tar, names })
}

/// What an entry of a tar archive is.
#[derive(Debug, PartialEq, Eq)]
enum Kind {
    /// A regular file.
    File,
    /// A hard or symbolic link, to the entry it names.
    Link(String),
    /// A directory, or anything else.
    Other,
}

/// An entry of a tar archive.
#[derive(Debug)]
struct Entry {
    /// Its path.
    name: String,
    /// What it is.
    kind: Kind,
    /// Where its data lies in the archive.
    data: Range<usize>,
}

/// The entries of the tar archive `tar` (ustar headers), up to its end
/// (an empty header, or the end of the data); `None` for a malformed one.
fn entries(tar: &[u8]) -> impl Iterator<Item = Option<Entry>> + '_ {
    let mut pos = 0;
    std::iter::from_fn(move || {
        let header = tar.get(pos..pos + BLOCK)?;
        if header.iter().all(|&byte| byte == 0) {
            return None;
        }
        let entry = entry(header, pos + BLOCK, tar.len());
        pos = match &entry {
            Some(entry) => entry.data.start + entry.data.len().div_ceil(BLOCK) * BLOCK,
            None => tar.len(),
        };
        Some(entry)
    })
}

/// The entry the tar header `header` describes, its data starting at
/// `start` in an archive of `len` bytes.
fn entry(header: &[u8], start: usize, len: usize) -> Option<Entry> {
    let name = text(&header[..100])?;
    let prefix = text(&header[345..500])?;
    let name = if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    };
    let size = usize::from_str_radix(text(&header[124..136])?.trim(), 8).ok()?;
    let end = start.checked_add(size).filter(|&end| end <= len)?;
    let kind = match header[156] {
        b'0' | 0 => Kind::File,
        b'1' | b'2' => Kind::Link(text(&header[157..257])?),
        _ => Kind::Other,
    };
    Some(Entry {
        name,
        kind,
        data: start..end,
    })
}

/// The NUL-terminated text of a header field.
fn text(field: &[u8]) -> Option<String> {
    let end = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(field.len());
    String::from_utf8(field[..end].to_vec()).ok()
}

/// The data of the gzip member (RFC 1952) at the start of `member`, `None`
/// when it is not one or its checksum fails.
fn gunzip(member: &[u8]) -> Option<Vec<u8>> {
    /// The header flags: a header checksum, extra fields, a file name, a
    /// comment.
    const FHCRC: u8 = 2;
    const FEXTRA: u8 = 4;
    const FNAME: u8 = 8;
    const FCOMMENT: u8 = 16;
    let [0x1f, 0x8b, 8, flags, ..] = *member else {
        return None;
    };
    let mut pos = 10;
    if flags & FEXTRA != 0 {
        let len = member.get(pos..pos + 2)?;
        pos += 2 + usize::from(u16::from_le_bytes([len[0], len[1]]));
    }
    for flag in [FNAME, FCOMMENT] {
        if flags & flag != 0 {
            pos += member.get(pos..)?.iter().position(|&byte| byte == 0)? + 1;
        }
    }
    if flags & FHCRC != 0 {
        pos += 2;
    }
    let mut data = Vec::new();
    let used =
        pltrs_codec::deflate::decompress_into(member.get(pos..)?, &mut data, usize::MAX).ok()?;
    let trailer = member.get(pos + used..pos + used + 4)?;
    let crc = u32::from_le_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    (crc == pltrs_codec::checksum::crc32(&data)).then_some(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::DateTime;
    use crate::dates::tz::{TzFile, ZoneRules};

    #[test]
    fn every_zone_of_the_archive_reads() {
        let zones = read(ARCHIVE).expect("the archive reads");
        // dateutil 2.9.0's archive: 353 zones and 245 links.
        assert_eq!(zones.names.len(), 598);
        for (name, (zone, range)) in &zones.names {
            let parsed = TzFile::parse(&zones.tar[range.clone()], zone);
            assert!(parsed.is_ok(), "{name}: {parsed:?}");
        }
    }

    #[test]
    fn links_are_the_zones_they_name() {
        let (name, data) = zone("Europe/Berlin").unwrap();
        assert_eq!(name, "Atlantic/Jan_Mayen", "dateutil's name for it");
        assert_eq!(zone("Atlantic/Jan_Mayen"), Some((name, data)));
        assert_eq!(zone("US/Eastern").unwrap().0, "America/New_York");
        let berlin = TzFile::parse(data, name).unwrap();
        let summer = DateTime::new(2020, 7, 1, 12, 0, 0, 0).unwrap();
        assert_eq!(berlin.name_at_local(summer, false), "CEST");
        assert_eq!(berlin.offset_at_local(summer, false).to_string(), "+02:00");
    }

    #[test]
    fn unknown_names_and_the_metadata_are_no_zones() {
        assert_eq!(zone("No/Such_Zone"), None);
        assert_eq!(zone(METADATA), None);
        assert_eq!(zone(""), None);
    }

    #[test]
    fn malformed_archives_read_as_none() {
        assert!(read(b"").is_none());
        assert!(read(b"\x1f\x8b\x08\x00").is_none());
        let mut corrupt = ARCHIVE.to_vec();
        let last = corrupt.len() - 5;
        corrupt[last] ^= 1;
        assert!(read(&corrupt).is_none(), "a failing checksum");
        assert!(read(&ARCHIVE[..ARCHIVE.len() / 2]).is_none(), "cut short");
    }

    #[test]
    fn tar_entries_stop_at_bad_headers() {
        let mut header = [0_u8; BLOCK];
        header[..4].copy_from_slice(b"zone");
        header[124..135].copy_from_slice(b"77777777777");
        assert!(
            entry(&header, BLOCK, 2 * BLOCK).is_none(),
            "data past the end"
        );
        header[124..135].copy_from_slice(b"00000000012");
        let entry = entry(&header, BLOCK, 2 * BLOCK).unwrap();
        assert_eq!(entry.name, "zone");
        assert_eq!(entry.kind, Kind::File);
        assert_eq!(entry.data, BLOCK..BLOCK + 10);
        let mut archive = header.to_vec();
        archive.resize(2 * BLOCK, 0);
        assert_eq!(entries(&archive).count(), 1);
    }
}
