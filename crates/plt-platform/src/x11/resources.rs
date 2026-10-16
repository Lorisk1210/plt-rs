//! The display's scale factor, from the `Xft.dpi` entry of the resource
//! database that desktops publish in the root window's `RESOURCE_MANAGER`
//! property.

use super::atoms::predefined;
use super::connection::Connection;
use super::reply;
use super::request;
use crate::error::Error;

/// The DPI that corresponds to a scale factor of 1.
const BASE_DPI: f64 = 96.0;
/// Longest resource database read, in 4-byte units (4 MiB).
const MAX_DATABASE_WORDS: u32 = 1 << 20;

/// Reads the scale factor of the screen with root window `root`: 1.0 when
/// no (valid) `Xft.dpi` is set.
pub(crate) fn query_scale_factor(conn: &mut Connection, root: u32) -> Result<f64, Error> {
    let query = request::get_property(root, predefined::RESOURCE_MANAGER, MAX_DATABASE_WORDS);
    let reply = conn.round_trip(&query)?;
    let database = reply::property(&reply)?.value;
    Ok(scale_factor(&String::from_utf8_lossy(database)).unwrap_or(1.0))
}

/// The scale factor implied by the `Xft.dpi` entry of a resource database
/// string (`name:\tvalue` lines), if present and valid.
fn scale_factor(database: &str) -> Option<f64> {
    database
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim() == "Xft.dpi")
        .and_then(|(_, value)| value.trim().parse::<f64>().ok())
        .filter(|dpi| dpi.is_finite() && *dpi > 0.0)
        .map(|dpi| dpi / BASE_DPI)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_xft_dpi() {
        let database = "Xcursor.size:\t24\nXft.antialias:\t1\nXft.dpi:\t192\nXft.hinting:\t1\n";
        assert_eq!(scale_factor(database), Some(2.0));
        assert_eq!(scale_factor("Xft.dpi: 144.0"), Some(1.5));
    }

    #[test]
    fn missing_or_invalid_dpi() {
        assert_eq!(scale_factor(""), None);
        assert_eq!(scale_factor("Xft.dpix:\t192"), None);
        assert_eq!(scale_factor("Xft.dpi:\tlots"), None);
        assert_eq!(scale_factor("Xft.dpi:\t-96"), None);
        assert_eq!(scale_factor("Xft.dpi:\tNaN"), None);
        assert_eq!(scale_factor("! a comment\nXft.dpi"), None);
    }
}
