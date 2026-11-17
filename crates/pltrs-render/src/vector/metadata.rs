//! Document metadata: the title, creator and creation date the vector
//! backends share, with matplotlib's `SOURCE_DATE_EPOCH` convention for
//! reproducible builds, and the entries only SVG documents or PNG files
//! record.

use std::time::{SystemTime, UNIX_EPOCH};

use pltrs_codec::png;

/// A moment in UTC, in whole seconds since the Unix epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub i64);

/// A calendar date and time of day in UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Civil {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    /// 0 = Monday.
    weekday: usize,
}

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

impl Timestamp {
    /// The current time.
    pub fn now() -> Self {
        let seconds = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(elapsed) => elapsed.as_secs() as i64,
            Err(before) => -(before.duration().as_secs() as i64),
        };
        Self(seconds)
    }

    /// `SOURCE_DATE_EPOCH` when set to a valid number of seconds, else the
    /// current time (what matplotlib writes by default).
    pub fn source_date_or_now() -> Self {
        std::env::var("SOURCE_DATE_EPOCH")
            .ok()
            .and_then(|value| value.trim().parse().ok())
            .map_or_else(Self::now, Self)
    }

    /// Howard Hinnant's `civil_from_days`.
    fn civil(self) -> Civil {
        let days = self.0.div_euclid(86_400);
        let secs = self.0.rem_euclid(86_400) as u32;
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Civil {
            year,
            month,
            day,
            hour: secs / 3600,
            minute: secs / 60 % 60,
            second: secs % 60,
            // 1970-01-01 was a Thursday.
            weekday: (days + 3).rem_euclid(7) as usize,
        }
    }

    /// ISO 8601 with a UTC offset (`2026-10-17T07:27:00+00:00`), as the SVG
    /// backend writes `dc:date`.
    pub fn iso8601(self) -> String {
        let c = self.civil();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
            c.year, c.month, c.day, c.hour, c.minute, c.second
        )
    }

    /// A PDF date string (`D:20261017072700Z`).
    pub fn pdf(self) -> String {
        let c = self.civil();
        format!(
            "D:{:04}{:02}{:02}{:02}{:02}{:02}Z",
            c.year, c.month, c.day, c.hour, c.minute, c.second
        )
    }

    /// `%a %b %d %H:%M:%S %Y`, as the PostScript backend writes
    /// `%%CreationDate`.
    pub fn ctime(self) -> String {
        let c = self.civil();
        format!(
            "{} {} {:02} {:02}:{:02}:{:02} {}",
            WEEKDAYS[c.weekday],
            MONTHS[c.month as usize - 1],
            c.day,
            c.hour,
            c.minute,
            c.second,
            c.year
        )
    }
}

/// Whether a PDF document has been trapped for printing (its `Trapped`
/// entry).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trapped {
    /// Trapping has been done.
    True,
    /// Trapping has not been done.
    False,
    /// Not known.
    Unknown,
}

impl Trapped {
    /// The PDF name of the value.
    pub const fn name(self) -> &'static str {
        match self {
            Self::True => "True",
            Self::False => "False",
            Self::Unknown => "Unknown",
        }
    }
}

/// Metadata written into a saved document; `None` leaves an entry out.
///
/// Every vector format writes the title, creator and date; `svg` holds
/// what only SVG documents record, and the other entries are keys of the
/// PDF document information dictionary (SVG records the keywords too).
/// PNG files record the text entries of `png` alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    /// Document title.
    pub title: Option<String>,
    /// The application that made the document.
    pub creator: Option<String>,
    /// The library that wrote it (PDF `Producer`).
    pub producer: Option<String>,
    /// Creation date.
    pub date: Option<Timestamp>,
    /// Who made the document (PDF `Author`).
    pub author: Option<String>,
    /// What the document is about (PDF `Subject`).
    pub subject: Option<String>,
    /// Keywords (PDF `Keywords`, the SVG subject).
    pub keywords: Option<String>,
    /// When the document was last changed (PDF `ModDate`).
    pub modified: Option<Timestamp>,
    /// Whether the document is trapped (PDF `Trapped`).
    pub trapped: Option<Trapped>,
    /// A PDF document's information dictionary as the program wrote it,
    /// in place of the entries above: each key with its value written as
    /// a PDF object (matplotlib's `pdfRepr` of the dictionary
    /// `_create_pdf_info_dict` makes, keys it warns about included).
    pub pdf_info: Option<Vec<(String, Vec<u8>)>>,
    /// The further Dublin Core entries of SVG documents.
    pub svg: SvgMetadata,
    /// The text entries of PNG files.
    pub png: PngMetadata,
}

impl Default for Metadata {
    /// pltrs as creator and producer, dated by
    /// [`Timestamp::source_date_or_now`]; the PNG `Software` entry
    /// ([`PngMetadata::default`]).
    fn default() -> Self {
        let version = env!("CARGO_PKG_VERSION");
        Self {
            creator: Some(format!("pltrs v{version}")),
            producer: Some(format!("pltrs v{version}")),
            date: Some(Timestamp::source_date_or_now()),
            svg: SvgMetadata::default(),
            png: PngMetadata::default(),
            ..Self::none()
        }
    }
}

impl Metadata {
    /// No metadata at all (reproducible output).
    pub const fn none() -> Self {
        Self {
            title: None,
            creator: None,
            producer: None,
            date: None,
            author: None,
            subject: None,
            keywords: None,
            modified: None,
            trapped: None,
            pdf_info: None,
            svg: SvgMetadata::none(),
            png: PngMetadata::none(),
        }
    }
}

/// The Dublin Core entries of SVG documents besides the title, creator and
/// date every format writes (matplotlib's SVG `metadata` keys), and the
/// several agents, subjects and dates SVG can record where the shared
/// entries hold one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgMetadata {
    /// The resource type URI (`Type`), the DCMI still image by default.
    pub resource_type: Option<String>,
    /// The media type (`Format`), `image/svg+xml` by default.
    pub format: Option<String>,
    /// `Coverage`.
    pub coverage: Option<String>,
    /// `Description`.
    pub description: Option<String>,
    /// `Identifier`.
    pub identifier: Option<String>,
    /// `Language`.
    pub language: Option<String>,
    /// `Relation`.
    pub relation: Option<String>,
    /// `Source`.
    pub source: Option<String>,
    /// Several creators (`Creator` as a list), written in place of
    /// [`Metadata::creator`] when not empty.
    pub creators: Vec<String>,
    /// `Contributor` agents.
    pub contributors: Vec<String>,
    /// `Publisher` agents.
    pub publishers: Vec<String>,
    /// `Rights` holders.
    pub rights: Vec<String>,
    /// Several keywords (`Keywords` as a list), written in place of
    /// [`Metadata::keywords`] when not empty.
    pub keywords: Vec<String>,
    /// Dates in ISO 8601, several making a period (`Date` as a list),
    /// written in place of [`Metadata::date`] when not empty.
    pub dates: Vec<String>,
}

impl Default for SvgMetadata {
    /// The resource type and format matplotlib records by default.
    fn default() -> Self {
        Self {
            resource_type: Some("http://purl.org/dc/dcmitype/StillImage".to_owned()),
            format: Some("image/svg+xml".to_owned()),
            ..Self::none()
        }
    }
}

impl SvgMetadata {
    /// No entry at all.
    pub const fn none() -> Self {
        Self {
            resource_type: None,
            format: None,
            coverage: None,
            description: None,
            identifier: None,
            language: None,
            relation: None,
            source: None,
            creators: Vec::new(),
            contributors: Vec::new(),
            publishers: Vec::new(),
            rights: Vec::new(),
            keywords: Vec::new(),
            dates: Vec::new(),
        }
    }
}

/// The `Software` entry of pltrs' PNG files, where matplotlib writes its
/// own name and version (`Matplotlib version3.11.2, https://matplotlib.org/`).
pub const PNG_SOFTWARE: &str = concat!("pltrs v", env!("CARGO_PKG_VERSION"));

/// The text entries of a PNG file (matplotlib's PNG `metadata`): any
/// keywords, PNG's own being `Title`, `Author`, `Description`,
/// `Copyright`, `Creation Time`, `Software`, `Disclaimer`, `Warning`,
/// `Source` and `Comment`. They are written in order as `tEXt` chunks, or
/// as `iTXt` chunks when the text is not Latin-1, as Pillow writes them.
///
/// matplotlib merges the `metadata` dictionary over its default
/// `Software` entry and leaves out the keys set to `None`;
/// [`PngMetadata::with`] and [`PngMetadata::without`] do the same.
///
/// ```
/// use pltrs_render::PngMetadata;
/// // matplotlib's `metadata={"Title": "Signal", "Software": None}`.
/// let png = PngMetadata::default().with("Title", "Signal").without("Software");
/// assert_eq!(png.entries(), [("Title".to_owned(), "Signal".to_owned())]);
/// let defaults = PngMetadata::default();
/// assert_eq!(defaults.entries()[0].0, "Software");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PngMetadata {
    /// Keywords and texts, in file order, each keyword once.
    entries: Vec<(String, String)>,
}

impl Default for PngMetadata {
    /// The `Software` entry naming pltrs ([`PNG_SOFTWARE`]), as matplotlib
    /// names itself.
    fn default() -> Self {
        Self::none().with("Software", PNG_SOFTWARE)
    }
}

impl PngMetadata {
    /// No entry at all.
    pub const fn none() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// These entries with `keyword` set to `text`: in place when the
    /// keyword has an entry, else as a new last entry (a key of
    /// matplotlib's `metadata`).
    #[must_use]
    pub fn with(mut self, keyword: impl Into<String>, text: impl Into<String>) -> Self {
        let (keyword, text) = (keyword.into(), text.into());
        match self.entries.iter_mut().find(|(key, _)| *key == keyword) {
            Some(entry) => entry.1 = text,
            None => self.entries.push((keyword, text)),
        }
        self
    }

    /// These entries without `keyword` (a key of matplotlib's `metadata`
    /// set to `None`).
    #[must_use]
    pub fn without(mut self, keyword: &str) -> Self {
        self.entries.retain(|(key, _)| key != keyword);
        self
    }

    /// The keywords and texts, in file order.
    pub fn entries(&self) -> &[(String, String)] {
        &self.entries
    }

    /// `encoder` writing these entries as text chunks, in order.
    pub fn apply_to(&self, encoder: png::Encoder) -> png::Encoder {
        self.entries
            .iter()
            .fold(encoder, |encoder, (keyword, text)| {
                encoder.text(keyword, text)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_dates() {
        let t = Timestamp(1_792_222_020);
        assert_eq!(t.iso8601(), "2026-10-17T07:27:00+00:00");
        assert_eq!(t.pdf(), "D:20261017072700Z");
        assert_eq!(t.ctime(), "Sat Oct 17 07:27:00 2026");
        assert_eq!(Timestamp(0).ctime(), "Thu Jan 01 00:00:00 1970");
        assert_eq!(Timestamp(-1).iso8601(), "1969-12-31T23:59:59+00:00");
        assert_eq!(Timestamp(951_782_400).pdf(), "D:20000229000000Z");
    }

    #[test]
    fn defaults_name_pltrs() {
        let metadata = Metadata::default();
        assert!(metadata.creator.is_some_and(|c| c.starts_with("pltrs")));
        assert_eq!(Metadata::none().date, None);
    }

    #[test]
    fn png_entries_merge_like_matplotlib_dictionaries() {
        let png = PngMetadata::default()
            .with("Title", "a")
            .with("Software", "b")
            .with("Title", "c");
        let entries: Vec<(&str, &str)> = png
            .entries()
            .iter()
            .map(|(key, text)| (key.as_str(), text.as_str()))
            .collect();
        assert_eq!(entries, [("Software", "b"), ("Title", "c")]);
        let emptied = png.without("Software").without("Title");
        assert_eq!(emptied.entries(), []);
        assert_eq!(Metadata::none().png, PngMetadata::none());
        assert_eq!(PngMetadata::default().entries()[0].1, PNG_SOFTWARE);
    }
}
