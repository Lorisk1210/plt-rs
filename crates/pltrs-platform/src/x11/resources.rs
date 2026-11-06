//! The display's scale factor and cursor size, from the `Xft.dpi` and
//! `Xcursor.size` entries of the resource database that desktops publish
//! in the root window's `RESOURCE_MANAGER` property.

use super::atoms::predefined;
use super::connection::Connection;
use super::reply;
use super::request;
use crate::cursor_art::GRID;
use crate::error::Error;

/// The DPI that corresponds to a scale factor of 1.
const BASE_DPI: f64 = 96.0;
/// Longest resource database read, in 4-byte units (4 MiB).
const MAX_DATABASE_WORDS: u32 = 1 << 20;
/// The cursor sizes taken from the database, in pixels.
const CURSOR_SIZES: std::ops::RangeInclusive<u32> = 8..=256;

/// What the resource database sets for drawing on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Resources {
    /// `Xft.dpi` / 96, 1 without a valid `Xft.dpi`.
    pub(crate) scale_factor: f64,
    /// The side of cursors in pixels: `Xcursor.size`, else 24 at the
    /// scale factor, as GTK and Qt size them.
    pub(crate) cursor_size: u32,
}

impl Resources {
    /// The settings of a resource database string (`name:\tvalue` lines).
    fn parse(database: &str) -> Self {
        let scale_factor = entry(database, "Xft.dpi")
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|dpi| dpi.is_finite() && *dpi > 0.0)
            .map_or(1.0, |dpi| dpi / BASE_DPI);
        let scaled = (f64::from(GRID) * scale_factor).round();
        let cursor_size = entry(database, "Xcursor.size")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|size| CURSOR_SIZES.contains(size))
            // Clamped into `CURSOR_SIZES`, the cast is exact.
            .unwrap_or_else(|| scaled.clamp(8.0, 256.0) as u32);
        Self {
            scale_factor,
            cursor_size,
        }
    }
}

/// The value of the entry `name`, if the database has one.
fn entry<'a>(database: &'a str, name: &str) -> Option<&'a str> {
    database
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(entry, _)| entry.trim() == name)
        .map(|(_, value)| value.trim())
}

/// Reads the settings of the screen with root window `root`.
pub(crate) fn query(conn: &mut Connection, root: u32) -> Result<Resources, Error> {
    let query = request::get_property(root, predefined::RESOURCE_MANAGER, MAX_DATABASE_WORDS);
    let reply = conn.round_trip(&query)?;
    let database = reply::property(&reply)?.value;
    Ok(Resources::parse(&String::from_utf8_lossy(database)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_xft_dpi_and_the_cursor_size() {
        let database = "Xcursor.size:\t48\nXft.antialias:\t1\nXft.dpi:\t192\nXft.hinting:\t1\n";
        let gnome = Resources::parse(database);
        assert_eq!((gnome.scale_factor, gnome.cursor_size), (2.0, 48));
        let plain = Resources::parse("Xft.dpi: 144.0");
        assert_eq!((plain.scale_factor, plain.cursor_size), (1.5, 36));
        assert_eq!(Resources::parse("Xcursor.size: 32").cursor_size, 32);
    }

    #[test]
    fn missing_or_invalid_entries() {
        for database in [
            "",
            "Xft.dpix:\t192",
            "Xft.dpi:\tlots",
            "Xft.dpi:\t-96",
            "Xft.dpi:\tNaN",
            "! a comment\nXft.dpi",
        ] {
            let resources = Resources::parse(database);
            assert_eq!(
                (resources.scale_factor, resources.cursor_size),
                (1.0, 24),
                "{database:?}"
            );
        }
        assert_eq!(Resources::parse("Xcursor.size: 4").cursor_size, 24);
        assert_eq!(Resources::parse("Xcursor.size: -1").cursor_size, 24);
        assert_eq!(Resources::parse("Xft.dpi: 1e9").cursor_size, 256);
    }
}
