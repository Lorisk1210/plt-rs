//! IANA time zones read from compiled `TZif` files, with `dateutil.tz.tzfile`
//! semantics, found where `dateutil.tz.gettz` finds them.
//!
//! Like dateutil, only the version-1 (32-bit) section of the file is used:
//! transitions cover 1901-2038, earlier times use the first standard-time
//! type and later times the last standard-time type. The POSIX footer of
//! version 2+ files is ignored.

use std::path::{Component, Path, PathBuf};

use crate::dates::timedelta::MICROS_PER_SECOND;
use crate::dates::{DateError, DateTime, TimeDelta};

use super::UtcOffset;

/// Directories searched for zone files, in dateutil's order (which is
/// also `zoneinfo`'s default `TZPATH`).
pub(super) const TZ_PATHS: [&str; 4] = [
    "/usr/share/zoneinfo",
    "/usr/lib/zoneinfo",
    "/usr/share/lib/zoneinfo",
    "/etc/zoneinfo",
];

/// One local time type of a zone (`ttinfo`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalTimeType {
    /// Seconds east of UTC.
    offset: i64,
    /// Whether this is daylight-saving time.
    is_dst: bool,
    /// The abbreviation, e.g. `"EST"`.
    abbreviation: String,
}

pltrs_core::archive_struct!(LocalTimeType {
    offset,
    is_dst,
    abbreviation
});

/// A time zone loaded from a `TZif` file (`dateutil.tz.tzfile`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TzFile {
    /// The zone name or path it was loaded from.
    name: String,
    /// Transition instants, seconds since the Unix epoch (UTC).
    transitions_utc: Vec<i64>,
    /// The same transitions on dateutil's "standard local" clock.
    transitions_local: Vec<i64>,
    /// The local time type in effect after each transition.
    transition_types: Vec<usize>,
    types: Vec<LocalTimeType>,
    /// The type used after the last transition (`ttinfo_std`).
    standard: Option<usize>,
    /// The type used before the first transition (`ttinfo_before`).
    before: Option<usize>,
}

pltrs_core::archive_struct!(TzFile {
    name,
    transitions_utc,
    transitions_local,
    transition_types,
    types,
    standard,
    before
});

impl TzFile {
    /// Loads the zone `name` as `dateutil.tz.gettz` does: the file at
    /// `name` if it is an absolute path, else from the system zoneinfo
    /// directories, else from the zones dateutil bundles, which pltrs
    /// bundles too (all there is on Windows).
    pub fn find(name: &str) -> Result<Self, DateError> {
        Self::find_in(name, &TZ_PATHS)
    }

    /// [`find`](Self::find), searching the directories `dirs`.
    fn find_in(name: &str, dirs: &[&str]) -> Result<Self, DateError> {
        let name = name.strip_prefix(':').unwrap_or(name);
        let unknown = || DateError::UnknownTimeZone(name.to_owned());
        let relative = Path::new(name);
        if name.is_empty()
            || relative
                .components()
                .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(unknown());
        }
        let absolute = relative.is_absolute();
        let candidates: Vec<PathBuf> = if absolute {
            vec![relative.to_path_buf()]
        } else {
            dirs.iter()
                .flat_map(|dir| {
                    [
                        Path::new(dir).join(name),
                        Path::new(dir).join(name.replace(' ', "_")),
                    ]
                })
                .collect()
        };
        candidates
            .iter()
            .filter(|path| path.is_file())
            .find_map(|path| {
                let bytes = std::fs::read(path).ok()?;
                Self::parse(&bytes, name).ok()
            })
            .or_else(|| {
                let (zone, data) = (!absolute).then(|| super::bundled::zone(name)).flatten()?;
                Self::parse(data, zone).ok()
            })
            .ok_or_else(unknown)
    }

    /// Parses `TZif` data; `name` is kept for display.
    pub fn parse(bytes: &[u8], name: &str) -> Result<Self, DateError> {
        let mut reader = Reader { bytes, pos: 0 };
        if reader.take(4)? != b"TZif" {
            return Err(DateError::InvalidTimeZoneData("missing TZif magic"));
        }
        reader.take(16)?;
        let mut counts = [0_usize; 6];
        for count in &mut counts {
            *count = usize::try_from(reader.i32()?)
                .map_err(|_| DateError::InvalidTimeZoneData("negative count"))?;
        }
        let [_, _, _, time_count, type_count, char_count] = counts;
        let transitions_utc = (0..time_count)
            .map(|_| reader.i32().map(i64::from))
            .collect::<Result<Vec<_>, _>>()?;
        let transition_types: Vec<usize> = reader
            .take(time_count)?
            .iter()
            .map(|&i| usize::from(i))
            .collect();
        let raw_types = (0..type_count)
            .map(|_| Ok((reader.i32()?, reader.take(1)?[0], reader.take(1)?[0])))
            .collect::<Result<Vec<_>, DateError>>()?;
        let chars = reader.take(char_count)?;
        let types = raw_types
            .into_iter()
            .map(|(offset, is_dst, index)| LocalTimeType {
                offset: i64::from(offset),
                is_dst: is_dst != 0,
                abbreviation: abbreviation(chars, usize::from(index)),
            })
            .collect::<Vec<_>>();
        if types.is_empty() || transition_types.iter().any(|&t| t >= types.len()) {
            return Err(DateError::InvalidTimeZoneData("bad local time type index"));
        }
        let (standard, before) = special_types(&types, &transition_types);
        let transitions_local = local_transitions(&types, &transition_types, &transitions_utc);
        Ok(Self {
            name: name.to_owned(),
            transitions_utc,
            transitions_local,
            transition_types,
            types,
            standard,
            before,
        })
    }

    /// The zone's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The offset at a wall-clock time (`utcoffset`).
    pub(crate) fn offset_at_local(&self, local: DateTime, fold: bool) -> UtcOffset {
        self.type_at_local(local, fold)
            .and_then(|t| UtcOffset::from_seconds(t.offset as i32).ok())
            .unwrap_or(UtcOffset::UTC)
    }

    /// The abbreviation at a wall-clock time (`tzname`).
    pub(crate) fn name_at_local(&self, local: DateTime, fold: bool) -> &str {
        self.type_at_local(local, fold)
            .map_or("", |t| t.abbreviation.as_str())
    }

    /// The wall-clock time and fold of a UTC instant (`fromutc`).
    pub(crate) fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
        let index = last_transition(&self.transitions_utc, utc);
        let offset = self.type_for(index).map_or(0, |t| t.offset);
        let local = utc.checked_add(TimeDelta::from_micros(offset * MICROS_PER_SECOND))?;
        let fold = self.is_ambiguous(local, index);
        Ok((local, fold))
    }

    fn type_at_local(&self, local: DateTime, fold: bool) -> Option<&LocalTimeType> {
        self.standard?;
        let index = last_transition(&self.transitions_local, local);
        let resolved = match index {
            Some(i) if i > 0 && !fold && self.is_ambiguous(local, index) => Some(i - 1),
            other => other,
        };
        self.type_for(resolved)
    }

    /// dateutil's `_get_ttinfo`: the type in effect after transition
    /// `index` (`None` when there are no transitions).
    fn type_for(&self, index: Option<isize>) -> Option<&LocalTimeType> {
        let chosen = match index {
            None => self.standard,
            Some(i) if i + 1 >= self.transitions_local.len() as isize => self.standard,
            Some(i) if i < 0 => self.before,
            Some(i) => Some(self.transition_types[i as usize]),
        };
        chosen.map(|t| &self.types[t])
    }

    /// Whether the wall time `local` occurs twice around transition `index`.
    fn is_ambiguous(&self, local: DateTime, index: Option<isize>) -> bool {
        let Some(i) = index.filter(|&i| i > 0) else {
            return false;
        };
        let (Some(current), Some(previous)) = (self.type_for(index), self.type_for(Some(i - 1)))
        else {
            return false;
        };
        let shift = previous.offset - current.offset;
        timestamp_seconds(local) < (self.transitions_local[i as usize] + shift) as f64
    }
}

/// dateutil's `_datetime_to_timestamp`: seconds since the epoch, ignoring
/// any zone.
fn timestamp_seconds(dt: DateTime) -> f64 {
    dt.timestamp_micros() as f64 / MICROS_PER_SECOND as f64
}

/// `bisect_right(transitions, timestamp) - 1`, or `None` without
/// transitions.
fn last_transition(transitions: &[i64], dt: DateTime) -> Option<isize> {
    if transitions.is_empty() {
        return None;
    }
    let timestamp = timestamp_seconds(dt);
    let count = transitions.partition_point(|&t| t as f64 <= timestamp);
    Some(count as isize - 1)
}

/// The NUL-terminated abbreviation starting at `index` (dateutil slices up
/// to the terminator, or drops the last character when there is none).
fn abbreviation(chars: &[u8], index: usize) -> String {
    let tail = chars.get(index..).unwrap_or_default();
    let end = tail
        .iter()
        .position(|&b| b == 0)
        .unwrap_or_else(|| tail.len().saturating_sub(1));
    String::from_utf8_lossy(&tail[..end]).into_owned()
}

/// dateutil's `ttinfo_std` (last standard type, else last DST type) and
/// `ttinfo_before` (first standard type, else the first type).
fn special_types(
    types: &[LocalTimeType],
    transition_types: &[usize],
) -> (Option<usize>, Option<usize>) {
    let before = types.iter().position(|t| !t.is_dst).or(Some(0));
    if transition_types.is_empty() {
        return (Some(0), before);
    }
    let latest = |dst: bool| {
        transition_types
            .iter()
            .rev()
            .copied()
            .find(|&t| types[t].is_dst == dst)
    };
    (latest(false).or_else(|| latest(true)), before)
}

/// dateutil's `trans_list`: each transition shifted by the standard
/// (non-DST) offset in effect before it.
fn local_transitions(types: &[LocalTimeType], transition_types: &[usize], utc: &[i64]) -> Vec<i64> {
    let mut last: Option<(bool, i64, i64)> = None; // (is_dst, offset, base offset)
    let mut last_dst_shift = 0;
    transition_types
        .iter()
        .zip(utc)
        .map(|(&t, &instant)| {
            let current = &types[t];
            let mut dst_shift = 0;
            if let Some((was_dst, last_offset, _)) = last
                && current.is_dst
            {
                if !was_dst {
                    dst_shift = current.offset - last_offset;
                }
                if dst_shift == 0 && last_dst_shift != 0 {
                    dst_shift = last_dst_shift;
                }
                last_dst_shift = dst_shift;
            }
            let base = current.offset - dst_shift;
            let adjustment = match last {
                Some((was_dst, _, last_base)) if base != last_base && current.is_dst != was_dst => {
                    last_base
                }
                _ => base,
            };
            last = Some((current.is_dst, current.offset, base));
            instant + adjustment
        })
        .collect()
}

/// A cursor over big-endian `TZif` data.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], DateError> {
        let end = self
            .pos
            .checked_add(len)
            .filter(|&end| end <= self.bytes.len())
            .ok_or(DateError::InvalidTimeZoneData("truncated file"))?;
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn i32(&mut self) -> Result<i32, DateError> {
        let bytes = self.take(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-type zone: EST (-5 h) and EDT (-4 h) with transitions at the
    /// 2020 US DST changes.
    fn eastern() -> TzFile {
        let spring = DateTime::new(2020, 3, 8, 7, 0, 0, 0)
            .unwrap()
            .timestamp_micros()
            / 1_000_000;
        let autumn = DateTime::new(2020, 11, 1, 6, 0, 0, 0)
            .unwrap()
            .timestamp_micros()
            / 1_000_000;
        let mut data = b"TZif2".to_vec();
        data.extend([0; 15]);
        for count in [0_i32, 0, 0, 3, 2, 8] {
            data.extend(count.to_be_bytes());
        }
        for t in [spring - 86_400 * 200, spring, autumn] {
            data.extend((t as i32).to_be_bytes());
        }
        data.extend([0, 1, 0]);
        data.extend((-18_000_i32).to_be_bytes());
        data.extend([0, 0]);
        data.extend((-14_400_i32).to_be_bytes());
        data.extend([1, 4]);
        data.extend(b"EST\0EDT\0");
        TzFile::parse(&data, "Test/Eastern").unwrap()
    }

    #[test]
    fn offsets_follow_transitions_and_folds() {
        let zone = eastern();
        let winter = DateTime::new(2020, 1, 15, 12, 0, 0, 0).unwrap();
        let summer = DateTime::new(2020, 7, 1, 12, 0, 0, 0).unwrap();
        assert_eq!(zone.offset_at_local(winter, false).to_string(), "-05:00");
        assert_eq!(zone.offset_at_local(summer, false).to_string(), "-04:00");
        assert_eq!(zone.name_at_local(summer, false), "EDT");
        // 01:30 on 2020-11-01 happens twice.
        let repeated = DateTime::new(2020, 11, 1, 1, 30, 0, 0).unwrap();
        assert_eq!(zone.name_at_local(repeated, false), "EDT");
        assert_eq!(zone.name_at_local(repeated, true), "EST");
        let utc = DateTime::new(2020, 11, 1, 6, 30, 0, 0).unwrap();
        assert_eq!(zone.local_from_utc(utc), Ok((repeated, true)));
    }

    #[test]
    fn names_no_directory_holds_come_from_dateutils_zones() {
        let zone = TzFile::find_in("US/Eastern", &[]).unwrap();
        assert_eq!(zone.name(), "America/New_York", "dateutil's name for it");
        let summer = DateTime::new(2020, 7, 1, 12, 0, 0, 0).unwrap();
        assert_eq!(zone.name_at_local(summer, false), "EDT");
        assert!(TzFile::find_in("US/Eastern", &["/no/such/dir"]).is_ok());
        assert!(
            TzFile::find_in("/no/such/dir/US/Eastern", &[]).is_err(),
            "a path names a file only"
        );
        assert!(TzFile::find_in("No/Such_Zone", &[]).is_err());
    }

    #[test]
    fn rejects_garbage_and_unknown_names() {
        assert!(TzFile::parse(b"nope", "x").is_err());
        assert!(TzFile::parse(b"TZif2", "x").is_err());
        assert!(TzFile::find("../../etc/passwd").is_err());
        assert!(TzFile::find("No/Such_Zone").is_err());
    }
}
