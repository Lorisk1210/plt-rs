//! The registry of built-in colormaps (`matplotlib.colormaps`).
//!
//! Every base map from matplotlib's `_cm.py` and `_cm_listed.py`, the
//! `grey`/`gray` aliases, and a reversed `<name>_r` of each. Lookups build
//! a fresh [`Colormap`], so callers may modify the result freely.
//!
//! ```
//! use plt::colors::colormaps;
//! let viridis = colormaps::get("viridis")?;
//! assert_eq!(viridis.n(), 256);
//! assert_eq!(colormaps::get("viridis_r")?.lut()[0], viridis.lut()[255]);
//! assert!(colormaps::get("Viridis").is_err(), "names are case-sensitive");
//! # Ok::<(), plt::colors::ColormapError>(())
//! ```

mod builtin;
mod builtin_data;
mod listed_data;

use std::sync::OnceLock;

use builtin_data::BUILTINS;

use super::{Colormap, ColormapError};

/// Alternative spellings matplotlib registers: `(alias, original)`.
const ALIASES: &[(&str, &str)] = &[
    ("grey", "gray"),
    ("gist_grey", "gist_gray"),
    ("gist_yerg", "gist_yarg"),
    ("Grays", "Greys"),
];

/// Every registered name in matplotlib's order: the base maps, the
/// aliases, then each of those reversed (`<name>_r`).
pub fn names() -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let forward: Vec<&str> = BUILTINS
            .iter()
            .map(|(name, _)| *name)
            .chain(ALIASES.iter().map(|(alias, _)| *alias))
            .collect();
        let reversed = forward.iter().map(|name| format!("{name}_r"));
        forward
            .iter()
            .map(|name| (*name).to_owned())
            .chain(reversed)
            .collect()
    })
}

/// Whether a colormap is registered under `name`.
pub fn contains(name: &str) -> bool {
    base_name(name).is_some()
}

/// The colormap registered under `name`, with segmented maps sampled at
/// `rcParams["image.lut"]` entries.
pub fn get(name: &str) -> Result<Colormap, ColormapError> {
    let unknown = || ColormapError::Unknown(name.to_owned());
    let (base, reversed) = base_name(name).ok_or_else(unknown)?;
    let original = ALIASES
        .iter()
        .find(|(alias, _)| *alias == base)
        .map_or(base, |(_, original)| *original);
    let (_, definition) = BUILTINS
        .iter()
        .find(|(name, _)| *name == original)
        .ok_or_else(unknown)?;
    let lutsize =
        usize::try_from(crate::rc::params().image_lut).map_err(|_| ColormapError::ZeroEntries)?;
    let cmap = definition.build(base, lutsize)?;
    Ok(if reversed { cmap.reversed() } else { cmap })
}

/// The named colormap, or the default `rcParams["image.cmap"]` for `None`
/// (`matplotlib.colormaps.get_cmap`).
pub fn get_cmap(name: Option<&str>) -> Result<Colormap, ColormapError> {
    match name {
        Some(name) => get(name),
        None => get(&crate::rc::params().image_cmap),
    }
}

/// Splits a registered name into its base (or alias) name and whether it
/// is the `_r` reversal.
fn base_name(name: &str) -> Option<(&str, bool)> {
    let is_base = |n: &str| {
        BUILTINS.iter().any(|(base, _)| *base == n) || ALIASES.iter().any(|(a, _)| *a == n)
    };
    if is_base(name) {
        Some((name, false))
    } else {
        name.strip_suffix("_r")
            .filter(|base| is_base(base))
            .map(|base| (base, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::ColormapKind;

    #[test]
    fn registry_layout() {
        let names = names();
        assert_eq!(names.len(), 2 * (BUILTINS.len() + ALIASES.len()));
        assert_eq!(names[0], "magma");
        assert!(names.iter().all(|name| contains(name)));
        assert!(!contains("viridis_r_r") && !contains("nope"));
    }

    #[test]
    fn every_map_builds() {
        for name in names() {
            let cmap = get(name).unwrap();
            assert_eq!(cmap.name(), name);
            assert!(cmap.lut().iter().all(|c| (0.0..=1.0).contains(&c.r)));
        }
    }

    #[test]
    fn aliases_and_kinds() {
        assert_eq!(get("grey").unwrap(), get("gray").unwrap());
        assert_eq!(get("Grays_r").unwrap(), get("Greys_r").unwrap());
        assert_eq!(get("tab10").unwrap().kind(), ColormapKind::Listed);
        assert_eq!(get("jet").unwrap().kind(), ColormapKind::LinearSegmented);
        assert_eq!(get("twilight_shifted").unwrap().n(), 510);
        assert_eq!(get_cmap(None).unwrap().name(), "viridis");
        assert_eq!(
            get("notacmap").unwrap_err(),
            ColormapError::Unknown("notacmap".into())
        );
    }
}
