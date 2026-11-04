//! Zones as Python's `zoneinfo.ZoneInfo` reads `TZif` data (CPython
//! 3.14's `Lib/zoneinfo`): the 64-bit transitions of version 2+ files
//! and the `TZ` string after them, which give the rules past the last
//! transition. The `tzdata` package's files are slim, with all their
//! data there, while [`TzFile`](super::TzFile), `dateutil`'s reading,
//! takes the version 1 data alone.

use super::posix::{self, LocalType, TzRule};
use crate::dates::{DateError, DateTime, TimeDelta};

/// The 44 bytes of a `TZif` header.
const HEADER_LEN: usize = 44;

/// A zone read the way `zoneinfo.ZoneInfo` reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneInfo {
    /// The zone's key (`"US/Pacific"`).
    key: String,
    /// Transition instants, seconds since the Unix epoch (UTC).
    transitions_utc: Vec<i64>,
    /// The transitions on the wall clock for the first reading of a
    /// repeated wall time (`_trans_local[0]`).
    wall_first: Vec<i64>,
    /// The same for the second reading (`_trans_local[1]`).
    wall_second: Vec<i64>,
    /// The local type in effect after each transition (`_ttinfos`).
    after_transition: Vec<LocalType>,
    /// The type before the first transition (`_tti_before`).
    before: Option<LocalType>,
    /// What the zone does after its last transition (`_tz_after`).
    after: TzRule,
}

pltrs_core::archive_struct!(ZoneInfo {
    key,
    transitions_utc,
    wall_first,
    wall_second,
    after_transition,
    before,
    after
});

/// The counts of a `TZif` header: `isutcnt`, `isstdcnt`, `leapcnt`,
/// `timecnt`, `typecnt`, `charcnt`.
struct Counts {
    is_ut: usize,
    is_std: usize,
    leap: usize,
    time: usize,
    types: usize,
    chars: usize,
}

impl ZoneInfo {
    /// Reads `TZif` data as `zoneinfo`'s `load_data` does; `key` names
    /// the zone.
    ///
    /// ```
    /// use pltrs::dates::ZoneInfo;
    /// // A slim file: no version 1 data, the rules in its `TZ` string.
    /// let mut data = b"TZif2".to_vec();
    /// data.extend([0; 15]);
    /// let counts = |types: i32, chars: i32| {
    ///     [0, 0, 0, 0, types, chars].iter().flat_map(|c: &i32| c.to_be_bytes()).collect::<Vec<u8>>()
    /// };
    /// data.extend(counts(1, 1));
    /// data.extend([0; 6]); // the version 1 type, and its one character
    /// data.push(0);
    /// data.extend(b"TZif2");
    /// data.extend([0; 15]);
    /// data.extend(counts(1, 4));
    /// data.extend((-8 * 3600_i32).to_be_bytes());
    /// data.extend([0, 0]);
    /// data.extend(b"PST\0\nPST8PDT,M3.2.0,M11.1.0\n");
    /// let zone = ZoneInfo::parse(&data, "US/Pacific")?;
    /// assert_eq!(zone.key(), "US/Pacific");
    /// # Ok::<(), pltrs::dates::DateError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`DateError::InvalidTimeZoneData`] for data that is no `TZif`
    /// file, is cut short, or holds no local time type or a malformed
    /// `TZ` string.
    pub fn parse(bytes: &[u8], key: &str) -> Result<Self, DateError> {
        let mut reader = Reader { bytes, pos: 0 };
        let (version, mut counts) = reader.header()?;
        let mut time_size = 4;
        if version >= 2 {
            let v1 = counts.time * 5
                + counts.types * 6
                + counts.chars
                + counts.leap * 8
                + counts.is_std
                + counts.is_ut;
            reader.take(v1)?;
            counts = reader.header()?.1;
            time_size = 8;
        }
        let transitions_utc = (0..counts.time)
            .map(|_| reader.signed(time_size))
            .collect::<Result<Vec<_>, _>>()?;
        let indices: Vec<usize> = reader
            .take(counts.time)?
            .iter()
            .map(|&i| usize::from(i))
            .collect();
        let raw_types = (0..counts.types)
            .map(|_| Ok((reader.signed(4)?, reader.take(1)?[0], reader.take(1)?[0])))
            .collect::<Result<Vec<_>, DateError>>()?;
        let chars = reader.take(counts.chars)?;
        let types: Vec<(LocalType, bool)> = raw_types
            .into_iter()
            .map(|(offset, is_dst, index)| {
                let abbreviation = abbreviation(chars, usize::from(index));
                (
                    LocalType {
                        offset,
                        abbreviation,
                    },
                    is_dst != 0,
                )
            })
            .collect();
        if indices.iter().any(|&i| i >= types.len()) {
            return Err(DateError::InvalidTimeZoneData("bad local time type index"));
        }
        let rule = if version >= 2 {
            reader.take(counts.is_ut + counts.is_std + counts.leap * 12)?;
            Some(reader.footer()?)
        } else {
            None
        };
        let offsets: Vec<i64> = types.iter().map(|(t, _)| t.offset).collect();
        let (wall_first, wall_second) = local_transitions(&indices, &transitions_utc, &offsets);
        let after_transition: Vec<LocalType> =
            indices.iter().map(|&i| types[i].0.clone()).collect();
        let before = types
            .iter()
            .find(|(_, is_dst)| !is_dst)
            .map(|(t, _)| t.clone())
            .or_else(|| after_transition.first().cloned());
        let after = match rule.filter(|rule| !rule.is_empty()) {
            Some(rule) => posix::parse(&rule)?,
            None => TzRule::Fixed(
                after_transition
                    .last()
                    .or(types.last().map(|(t, _)| t))
                    .cloned()
                    .ok_or(DateError::InvalidTimeZoneData("no local time type"))?,
            ),
        };
        Ok(Self {
            key: key.to_owned(),
            transitions_utc,
            wall_first,
            wall_second,
            after_transition,
            before,
            after,
        })
    }

    /// The zone's key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The local type at a wall-clock time (`_find_trans`): `utcoffset`
    /// and `tzname` read it.
    fn at_local(&self, local: DateTime, fold: bool) -> Option<&LocalType> {
        let ts = timestamp(local);
        let transitions = if fold {
            &self.wall_second
        } else {
            &self.wall_first
        };
        match (transitions.first(), transitions.last()) {
            (Some(&first), _) if ts < first => self.before.as_ref(),
            (Some(_), Some(&last)) if ts <= last => {
                let index = transitions.partition_point(|&t| t <= ts) - 1;
                self.after_transition.get(index)
            }
            _ => Some(match &self.after {
                TzRule::Seasons(seasons) => seasons.at_local(ts, local.year(), fold),
                TzRule::Fixed(fixed) => fixed,
            }),
        }
    }

    /// The offset east of UTC, in seconds, at a wall-clock time.
    pub(crate) fn offset_at_local(&self, local: DateTime, fold: bool) -> i64 {
        self.at_local(local, fold).map_or(0, |t| t.offset)
    }

    /// The abbreviation at a wall-clock time.
    pub(crate) fn name_at_local(&self, local: DateTime, fold: bool) -> &str {
        self.at_local(local, fold)
            .map_or("", |t| t.abbreviation.as_str())
    }

    /// The local type at a UTC instant and whether its wall time is the
    /// second of a repeated one (`fromutc`).
    fn at_utc(&self, utc: DateTime) -> (Option<&LocalType>, bool) {
        let ts = timestamp(utc);
        let transitions = &self.transitions_utc;
        match (transitions.first(), transitions.last(), &self.after) {
            (Some(&first), _, _) if ts < first => (self.before.as_ref(), false),
            (None, _, TzRule::Seasons(seasons)) => {
                let (local, fold) = seasons.at_utc(ts, utc.year());
                (Some(local), fold)
            }
            (Some(_), Some(&last), TzRule::Seasons(seasons)) if ts > last => {
                let (local, fold) = seasons.at_utc(ts, utc.year());
                (Some(local), fold)
            }
            (None, _, TzRule::Fixed(fixed)) => (Some(fixed), false),
            _ => {
                let index = transitions.partition_point(|&t| t <= ts);
                let (previous, current) = if transitions.len() > 1 && ts >= transitions[1] {
                    (
                        self.after_transition.get(index - 2),
                        self.after_transition.get(index - 1),
                    )
                } else if ts > transitions[transitions.len() - 1] {
                    let fixed = match &self.after {
                        TzRule::Fixed(fixed) => Some(fixed),
                        TzRule::Seasons(_) => None,
                    };
                    (self.after_transition.last(), fixed)
                } else {
                    (self.before.as_ref(), self.after_transition.first())
                };
                let fold = match (previous, current) {
                    (Some(previous), Some(current)) => {
                        previous.offset - current.offset > ts - transitions[index - 1]
                    }
                    _ => false,
                };
                (current, fold)
            }
        }
    }

    /// The wall-clock time and fold of a UTC instant.
    pub(crate) fn local_from_utc(&self, utc: DateTime) -> Result<(DateTime, bool), DateError> {
        let (local, fold) = self.at_utc(utc);
        let offset = local.map_or(0, |t| t.offset);
        let local = utc.checked_add(TimeDelta::from_micros(offset * 1_000_000))?;
        Ok((local, fold))
    }
}

/// Seconds since 1970-01-01 of a wall-clock time, its microseconds left
/// out (`_get_local_timestamp`).
fn timestamp(dt: DateTime) -> i64 {
    dt.timestamp_micros().div_euclid(1_000_000)
}

/// The transitions on the wall clock (`_ts_to_local`): each shifted by
/// the larger of the offsets around it for the first reading of a
/// repeated wall time, by the smaller for the second.
fn local_transitions(indices: &[usize], utc: &[i64], offsets: &[i64]) -> (Vec<i64>, Vec<i64>) {
    let Some(&first_index) = indices.first() else {
        return (Vec::new(), Vec::new());
    };
    let first = if offsets.len() > 1 {
        (offsets[0], offsets[first_index])
    } else {
        (offsets[0], offsets[0])
    };
    let pairs = std::iter::once(first).chain(
        indices
            .windows(2)
            .map(|pair| (offsets[pair[0]], offsets[pair[1]])),
    );
    let (mut earlier, mut later) = (Vec::with_capacity(utc.len()), Vec::with_capacity(utc.len()));
    for ((a, b), &instant) in pairs.zip(utc) {
        earlier.push(instant + a.max(b));
        later.push(instant + a.min(b));
    }
    (earlier, later)
}

/// The abbreviation starting at `index`, up to the next NUL; without one,
/// all but the last character (Python's `chars[index:-1]`).
fn abbreviation(chars: &[u8], index: usize) -> String {
    let start = index.min(chars.len());
    let tail = &chars[start..];
    let end = tail
        .iter()
        .position(|&b| b == 0)
        .map_or(chars.len().saturating_sub(1).max(start), |end| start + end);
    String::from_utf8_lossy(&chars[start..end]).into_owned()
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

    /// A big-endian signed integer of `size` (4 or 8) bytes.
    fn signed(&mut self, size: usize) -> Result<i64, DateError> {
        let bytes = self.take(size)?;
        let mut value = if bytes[0] & 0x80 == 0 { 0 } else { -1_i64 };
        for &byte in bytes {
            value = (value << 8) | i64::from(byte);
        }
        Ok(value)
    }

    /// A header: the version (0 for version 1) and the counts.
    fn header(&mut self) -> Result<(u8, Counts), DateError> {
        let header = self.take(HEADER_LEN)?;
        if &header[..4] != b"TZif" {
            return Err(DateError::InvalidTimeZoneData("missing TZif magic"));
        }
        let version = header[4].saturating_sub(b'0');
        let mut counts = [0_usize; 6];
        let (fields, _) = header[20..].as_chunks::<4>();
        for (count, field) in counts.iter_mut().zip(fields) {
            let value = i32::from_be_bytes(*field);
            *count = usize::try_from(value)
                .map_err(|_| DateError::InvalidTimeZoneData("negative count"))?;
        }
        let [is_ut, is_std, leap, time, types, chars] = counts;
        Ok((
            version,
            Counts {
                is_ut,
                is_std,
                leap,
                time,
                types,
                chars,
            },
        ))
    }

    /// The `TZ` string between the two newlines ending a version 2+ file.
    fn footer(&mut self) -> Result<String, DateError> {
        if self.take(1)? != b"\n" {
            return Err(DateError::InvalidTimeZoneData("missing TZ string"));
        }
        let rest = &self.bytes[self.pos..];
        let end = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or(DateError::InvalidTimeZoneData("unterminated TZ string"))?;
        self.pos += end + 1;
        String::from_utf8(rest[..end].to_vec())
            .map_err(|_| DateError::InvalidTimeZoneData("TZ string is not text"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A version 2 file of `America/Los_Angeles`' last transitions of 2020
    /// (in its 64-bit data only, as slim files are), with its `TZ` string.
    fn slim_pacific() -> Vec<u8> {
        let header = |types: i32, chars: i32, times: i32| {
            let mut out = b"TZif2".to_vec();
            out.extend([0; 15]);
            for count in [0, 0, 0, times, types, chars] {
                out.extend(i32::to_be_bytes(count));
            }
            out
        };
        let mut data = header(1, 1, 0);
        data.extend([0; 7]);
        data.extend(header(2, 8, 2));
        // 2020-03-08 10:00 UTC and 2020-11-01 09:00 UTC.
        for instant in [1_583_661_600_i64, 1_604_221_200] {
            data.extend(instant.to_be_bytes());
        }
        data.extend([1, 0]);
        data.extend((-8 * 3600_i32).to_be_bytes());
        data.extend([0, 0]);
        data.extend((-7 * 3600_i32).to_be_bytes());
        data.extend([1, 4]);
        data.extend(b"PST\0PDT\0\nPST8PDT,M3.2.0,M11.1.0\n");
        data
    }

    fn local(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime {
        DateTime::new(year, month, day, hour, minute, 0, 0).unwrap()
    }

    #[test]
    fn slim_files_follow_their_tz_string_past_the_last_transition() {
        let zone = ZoneInfo::parse(&slim_pacific(), "US/Pacific").unwrap();
        assert_eq!(zone.name_at_local(local(2020, 7, 1, 12, 0), false), "PDT");
        assert_eq!(zone.name_at_local(local(2020, 12, 1, 12, 0), false), "PST");
        assert_eq!(zone.name_at_local(local(2030, 7, 1, 12, 0), false), "PDT");
        assert_eq!(
            zone.offset_at_local(local(2030, 1, 1, 12, 0), false),
            -8 * 3600
        );
        assert_eq!(
            zone.name_at_local(local(2000, 7, 1, 12, 0), false),
            "PST",
            "before"
        );
    }

    #[test]
    fn repeated_wall_times_read_by_fold() {
        let zone = ZoneInfo::parse(&slim_pacific(), "US/Pacific").unwrap();
        let repeated = local(2020, 11, 1, 1, 30);
        assert_eq!(zone.name_at_local(repeated, false), "PDT");
        assert_eq!(zone.name_at_local(repeated, true), "PST");
        let first = local(2020, 11, 1, 8, 30);
        let second = local(2020, 11, 1, 9, 30);
        assert_eq!(zone.local_from_utc(first).unwrap(), (repeated, false));
        assert_eq!(zone.local_from_utc(second).unwrap(), (repeated, true));
        let later = zone.local_from_utc(local(2031, 11, 2, 9, 30)).unwrap();
        assert_eq!(later, (local(2031, 11, 2, 1, 30), true), "by the TZ string");
    }

    #[test]
    fn version_1_files_keep_their_last_type() {
        let mut data = b"TZif".to_vec();
        data.extend([0; 16]);
        for count in [0, 0, 0, 0, 1, 4] {
            data.extend(i32::to_be_bytes(count));
        }
        data.extend(3600_i32.to_be_bytes());
        data.extend([0, 0]);
        data.extend(b"CET\0");
        let zone = ZoneInfo::parse(&data, "Fixed").unwrap();
        assert_eq!(zone.offset_at_local(local(2020, 1, 1, 0, 0), false), 3600);
        assert_eq!(
            zone.local_from_utc(local(2020, 1, 1, 0, 0)).unwrap(),
            (local(2020, 1, 1, 1, 0), false)
        );
    }

    #[test]
    fn broken_data_is_refused() {
        let data = slim_pacific();
        assert!(ZoneInfo::parse(b"nope", "x").is_err());
        for cut in [10, 50, data.len() - 3] {
            assert!(ZoneInfo::parse(&data[..cut], "x").is_err(), "{cut}");
        }
        let mut bad_index = data.clone();
        let at = bad_index.len() - 46;
        bad_index[at] = 9;
        assert!(ZoneInfo::parse(&bad_index, "x").is_err());
    }

    #[test]
    fn abbreviations_without_terminator_drop_the_last_character() {
        assert_eq!(abbreviation(b"LMT\0AHST", 4), "AHS");
        assert_eq!(abbreviation(b"LMT\0AHST\0", 5), "HST");
        assert_eq!(abbreviation(b"", 3), "");
    }
}
