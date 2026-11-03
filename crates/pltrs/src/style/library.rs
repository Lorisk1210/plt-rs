//! The style library (matplotlib's `style.library`): the bundled sheets
//! plus the sheets of the user library directories, merged by name.

use std::borrow::Cow;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::rc::{RcEntry, RcError, RcFile, RcParams, RcWarning, parse_rc_text};

/// A sheet of the style library: a bundled sheet, a sheet read from a user
/// library directory, or a bundled sheet a user sheet of the same name
/// adds to (matplotlib's nested `dict.update`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleSheet {
    name: Cow<'static, str>,
    bundled: Option<&'static str>,
    user: Vec<RcEntry>,
}

/// Declares the bundled sheets from `sheets/<name>.mplstyle`.
macro_rules! sheets {
    ($($name:literal),* $(,)?) => {
        /// Every bundled sheet as `(name, text)`, sorted by name.
        const BUNDLED: &[(&str, &str)] = &[
            $(($name, include_str!(concat!("sheets/", $name, ".mplstyle"))),)*
        ];
    };
}

sheets![
    "Solarize_Light2",
    "_classic_test_patch",
    "_mpl-gallery",
    "_mpl-gallery-nogrid",
    "bmh",
    "classic",
    "dark_background",
    "fast",
    "fivethirtyeight",
    "ggplot",
    "grayscale",
    "petroff10",
    "petroff6",
    "petroff8",
    "pltrs-modern",
    "pltrs-modern-dark",
    "seaborn-v0_8",
    "seaborn-v0_8-bright",
    "seaborn-v0_8-colorblind",
    "seaborn-v0_8-dark",
    "seaborn-v0_8-dark-palette",
    "seaborn-v0_8-darkgrid",
    "seaborn-v0_8-deep",
    "seaborn-v0_8-muted",
    "seaborn-v0_8-notebook",
    "seaborn-v0_8-paper",
    "seaborn-v0_8-pastel",
    "seaborn-v0_8-poster",
    "seaborn-v0_8-talk",
    "seaborn-v0_8-ticks",
    "seaborn-v0_8-white",
    "seaborn-v0_8-whitegrid",
    "tableau-colorblind10",
];

/// The extension of style files (matplotlib's `STYLE_EXTENSION`).
const EXTENSION: &str = "mplstyle";

/// The user sheets of a library, by name, each with its settings.
type UserSheets = Vec<(String, Vec<RcEntry>)>;

impl StyleSheet {
    fn bundled((name, text): (&'static str, &'static str)) -> Self {
        Self {
            name: Cow::Borrowed(name),
            bundled: Some(text),
            user: Vec::new(),
        }
    }

    /// The sheet's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The sheet's settings in file order (matplotlib's
    /// `rc_params_from_file(..., use_default_template=False)`): those of
    /// the bundled sheet, then those of the user sheets of the same name.
    pub fn entries(&self) -> Result<Vec<RcEntry>, RcError> {
        let mut entries = self
            .bundled
            .map(parse_rc_text)
            .transpose()?
            .unwrap_or_default();
        entries.extend(self.user.iter().cloned());
        Ok(entries)
    }

    /// Applies the sheet's settings to `rc`, all of them, as matplotlib's
    /// `rcParams.update(library[name])` does.
    pub fn apply(&self, rc: &mut RcParams) -> Result<(), RcError> {
        for entry in self.entries()? {
            rc.set(&entry.key, &entry.value)
                .map_err(|err| RcError::AtLine {
                    line: entry.line,
                    source: Box::new(err),
                })?;
        }
        Ok(())
    }
}

/// A thread's user library: its directories and the sheets last read from
/// them.
#[derive(Default)]
struct UserLibrary {
    /// The directories [`set_user_library_paths`] gave; until then the
    /// default ones.
    paths: Option<Vec<PathBuf>>,
    /// The sheets [`reload_library`] read; until then those of the default
    /// directories.
    sheets: Option<UserSheets>,
}

pltrs_core::thread_state! {
    /// The thread's user library (matplotlib's `style.library`, beyond the
    /// bundled sheets).
    static USER_LIBRARY: RefCell<UserLibrary> = RefCell::default();
}

/// The sheets of the default user library directories, read once per
/// process.
static DEFAULT_SHEETS: OnceLock<UserSheets> = OnceLock::new();

/// The sheets of the default user library directories, read the first time
/// a library is consulted (matplotlib reads them when it starts); none
/// while the user configuration is
/// [ignored](crate::config::ignore_user_config).
fn default_sheets() -> &'static [(String, Vec<RcEntry>)] {
    if crate::config::user_config_ignored() {
        return &[];
    }
    DEFAULT_SHEETS.get_or_init(|| read_library(&crate::config::default_style_library_paths()).0)
}

/// The bundled sheet called `name`, whatever the user library holds.
pub(crate) fn bundled_sheet(name: &str) -> Option<StyleSheet> {
    BUNDLED
        .iter()
        .copied()
        .find(|&(bundled, _)| bundled == name)
        .map(StyleSheet::bundled)
}

/// The sheets bundled with pltrs, sorted by name, without the user
/// library (matplotlib's `style._base_library`, the sheets of its
/// `stylelib` directory).
///
/// ```
/// let names: Vec<String> = pltrs::style::bundled_sheets().iter().map(|s| s.name().to_owned()).collect();
/// assert!(names.contains(&"classic".to_owned()) && names.contains(&"_classic_test_patch".to_owned()));
/// ```
pub fn bundled_sheets() -> Vec<StyleSheet> {
    BUNDLED.iter().copied().map(StyleSheet::bundled).collect()
}

/// Every sheet of the library, sorted by name (matplotlib's
/// `style.library`).
pub fn library() -> Vec<StyleSheet> {
    let mut sheets: Vec<StyleSheet> = BUNDLED.iter().copied().map(StyleSheet::bundled).collect();
    USER_LIBRARY.with(|user| {
        let user = user.borrow();
        for (name, entries) in user.sheets.as_deref().unwrap_or_else(|| default_sheets()) {
            match sheets.iter_mut().find(|sheet| sheet.name == name.as_str()) {
                Some(sheet) => sheet.user.extend(entries.iter().cloned()),
                None => sheets.push(StyleSheet {
                    name: Cow::Owned(name.clone()),
                    bundled: None,
                    user: entries.clone(),
                }),
            }
        }
    });
    sheets.sort_by(|a, b| a.name.cmp(&b.name));
    sheets
}

/// The library sheet called `name`.
pub fn sheet(name: &str) -> Option<StyleSheet> {
    library().into_iter().find(|sheet| sheet.name == name)
}

/// The names of the public sheets, sorted (matplotlib's
/// `style.available`): names starting with `_` are private.
pub fn available() -> Vec<String> {
    library()
        .into_iter()
        .map(|sheet| sheet.name.into_owned())
        .filter(|name| !name.starts_with('_'))
        .collect()
}

/// The directories [`reload_library`] reads user sheets from (matplotlib's
/// `style.USER_LIBRARY_PATHS`): those [`set_user_library_paths`] gave,
/// else `stylelib` in the [configuration directory](crate::config), whose
/// sheets the library holds from the start. There is no default directory
/// while the user configuration is
/// [ignored](crate::config::ignore_user_config).
pub fn user_library_paths() -> Vec<PathBuf> {
    USER_LIBRARY
        .with(|user| user.borrow().paths.clone())
        .unwrap_or_else(crate::config::default_style_library_paths)
}

/// Replaces the user library directories; the library changes with the
/// next [`reload_library`]. Like the rc parameters, the library belongs
/// to the current thread.
pub fn set_user_library_paths(paths: Vec<PathBuf>) {
    USER_LIBRARY.with(|user| user.borrow_mut().paths = Some(paths));
}

/// Reads the user sheets again (matplotlib's `style.reload_library`):
/// every `*.mplstyle` file of the user library directories, in directory
/// order and by file name within a directory, adds a sheet named after
/// the file or adds its settings to the sheet of that name. Returns the
/// lines skipped in those files (which the first, implicit reading of the
/// default directories does not report); files that cannot be read are
/// skipped as well.
///
/// ```
/// use pltrs::style;
/// let dir = std::env::temp_dir().join(format!("pltrs-style-doc-{}", std::process::id()));
/// std::fs::create_dir_all(&dir)?;
/// std::fs::write(dir.join("pinkish.mplstyle"), "image.cmap: pink\nfoo: bar\n")?;
/// style::set_user_library_paths(vec![dir.clone()]);
/// let skipped = style::reload_library();
/// assert_eq!(skipped.len(), 1, "foo is not a parameter");
/// assert!(style::available().contains(&"pinkish".to_owned()));
/// style::set_user_library_paths(Vec::new());
/// style::reload_library();
/// assert!(style::sheet("pinkish").is_none());
/// std::fs::remove_dir_all(&dir)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn reload_library() -> Vec<RcWarning> {
    let (sheets, warnings) = read_library(&user_library_paths());
    USER_LIBRARY.with(|user| user.borrow_mut().sheets = Some(sheets));
    warnings
}

/// The user sheets of the directories `paths`, merged as
/// [`reload_library`] describes, and the lines skipped in their files.
fn read_library(paths: &[PathBuf]) -> (UserSheets, Vec<RcWarning>) {
    let mut sheets = UserSheets::new();
    let mut warnings = Vec::new();
    for dir in paths {
        for (name, file) in read_style_directory(dir) {
            warnings.extend(file.warnings);
            match sheets.iter_mut().find(|(seen, _)| *seen == name) {
                Some((_, entries)) => entries.extend(file.entries),
                None => sheets.push((name, file.entries)),
            }
        }
    }
    (sheets, warnings)
}

/// The style files of `dir` by name, sorted by file name; files that
/// cannot be read are left out.
fn read_style_directory(dir: &Path) -> Vec<(String, RcFile)> {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = listing
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == EXTENSION))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_owned();
            Some((name, RcFile::read(&path).ok()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_sheet_is_valid() {
        for sheet in library() {
            let mut rc = RcParams::matplotlib();
            sheet
                .apply(&mut rc)
                .unwrap_or_else(|err| panic!("{}: {err}", sheet.name()));
            assert!(!sheet.entries().unwrap().is_empty(), "{}", sheet.name());
        }
        assert_eq!(available().len(), 30);
        let mut names = available();
        names.sort_unstable();
        assert_eq!(names, available(), "the library is sorted");
    }

    #[test]
    fn private_sheets_are_bundled_but_not_available() {
        let mut rc = RcParams::matplotlib();
        sheet("_mpl-gallery").unwrap().apply(&mut rc).unwrap();
        assert!(rc.axes_grid);
        assert_eq!(rc.figure_figsize, [2.0, 2.0]);
        assert!(sheet("_mpl-gallery-nogrid").is_some());
        assert!(sheet("_classic_test_patch").is_some());
        assert!(available().iter().all(|name| !name.starts_with('_')));
    }

    #[test]
    fn sheets_change_only_what_they_list() {
        let mut rc = RcParams::matplotlib();
        sheet("ggplot").unwrap().apply(&mut rc).unwrap();
        assert!(rc.axes_grid);
        assert_eq!(rc.patch_linewidth, 0.5);
        assert_eq!(rc.lines_linewidth, 1.5, "not in the sheet");
    }

    #[test]
    fn user_sheets_add_to_bundled_ones_and_later_directories_win() {
        let root = std::env::temp_dir().join(format!("pltrs-style-lib-{}", std::process::id()));
        let (first, second) = (root.join("a"), root.join("b"));
        for dir in [&first, &second] {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(first.join("ggplot.mplstyle"), "lines.linewidth: 7\n").unwrap();
        std::fs::write(first.join("_mine.mplstyle"), "image.cmap: pink\n").unwrap();
        std::fs::write(second.join("_mine.mplstyle"), "image.cmap: gray\n").unwrap();
        std::fs::write(second.join("notes.txt"), "not a style\n").unwrap();
        set_user_library_paths(vec![first, second]);
        assert_eq!(reload_library(), Vec::new());
        let mut rc = RcParams::matplotlib();
        sheet("ggplot").unwrap().apply(&mut rc).unwrap();
        assert_eq!((rc.lines_linewidth, rc.patch_linewidth), (7.0, 0.5));
        sheet("_mine").unwrap().apply(&mut rc).unwrap();
        assert_eq!(rc.image_cmap, "gray");
        assert!(!available().contains(&"_mine".to_owned()));
        set_user_library_paths(Vec::new());
        reload_library();
        assert!(sheet("_mine").is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_library_changes_only_when_reloaded() {
        let dir = std::env::temp_dir().join(format!("pltrs-style-reload-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pltrs-modern.mplstyle"), "lines.linewidth: 9\n").unwrap();
        std::fs::write(dir.join("bad.mplstyle"), "lines.linewidth: wide\n").unwrap();
        assert_eq!(
            user_library_paths(),
            Vec::<PathBuf>::new(),
            "unit tests have no default directory"
        );
        set_user_library_paths(vec![dir.clone()]);
        assert_eq!(user_library_paths(), std::slice::from_ref(&dir));
        assert!(sheet("bad").is_none(), "not read yet");
        let warnings = reload_library();
        assert_eq!(warnings.len(), 1);
        assert_eq!(sheet("bad").unwrap().entries().unwrap(), Vec::new());
        let mut rc = RcParams::matplotlib();
        sheet("pltrs-modern").unwrap().apply(&mut rc).unwrap();
        assert_eq!(rc.lines_linewidth, 9.0);
        let mut rc = RcParams::matplotlib();
        bundled_sheet("pltrs-modern")
            .unwrap()
            .apply(&mut rc)
            .unwrap();
        assert_eq!(rc.lines_linewidth, 1.75, "bundled sheets ignore the user's");
        assert!(bundled_sheet("bad").is_none());
        set_user_library_paths(Vec::new());
        reload_library();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
