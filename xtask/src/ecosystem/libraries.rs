//! The libraries whose own test suites `cargo xtask ecosystem suite` runs:
//! where each suite is in the library's source tree (its sdist, or the
//! release's source archive where the sdist leaves the tests out), which
//! of its tests reach matplotlib (only those run), the pytest options its
//! runs add to the library's own configuration, the data its tests read
//! that setup fetches, whether they may reach the network, and how setup
//! remakes the baselines of its image comparisons under matplotlib 3.11.2
//! where its own are another version's.
//!
//! The libraries are listed by what they are for: data analysis and
//! machine learning (`analysis`), the sciences (`science`), plotting
//! itself (`plotting`), the add-ons building on matplotlib's parts
//! (`addons`) and the charts of other fields drawn with it (`charts`),
//! these last two in venvs of their own, and the libraries of other
//! fields (`fields`), in venvs of theirs.

mod addons;
mod analysis;
mod charts;
mod depth;
mod fields;
mod plotting;
mod science;

/// A library whose test suite runs.
#[derive(Clone, Copy, Debug)]
pub(super) struct Library {
    /// Its distribution's name, as the requirements pin it.
    pub(super) name: &'static str,
    /// The package it installs (its import name).
    pub(super) package: &'static str,
    /// Where its source tree comes from.
    pub(super) source: Source,
    /// What the suite is, in the source tree, and where it goes in the
    /// site.
    pub(super) copies: &'static [(&'static str, &'static str)],
    /// The files of the release's source tree that the suite reads and
    /// the sdist leaves out.
    pub(super) left_out: Option<LeftOut>,
    /// The file of the source tree holding the library's pytest
    /// configuration, which goes to the top of the site.
    pub(super) configuration: &'static str,
    /// The directory of the site the tests run in, if not the site
    /// itself: the package, for the suites that live beside it in the
    /// source tree and read their files relative to the working
    /// directory (mplfinance's `tests/reference_images`) or import
    /// themselves by their own name (matplotlib-venn's `tests.utils`).
    /// Their copies go into the package, which keeps test ids
    /// (`mplfinance/tests/...`) apart from the other libraries'.
    pub(super) runs_in: Option<&'static str>,
    /// The test files and directories that run, in the site.
    pub(super) tests: &'static [&'static str],
    /// The test files of [`Library::tests`] of which only some tests run.
    pub(super) only_taking: Option<OnlyTaking>,
    /// What the library's pytest runs add to its configuration.
    pub(super) options: &'static [&'static str],
    /// The data its tests read that is neither in its source tree nor in
    /// the installed package, which setup fetches.
    pub(super) data: Option<Data>,
    /// Whether its tests may reach the network.
    pub(super) network: Network,
    /// The baselines of its image comparisons, which setup makes under
    /// matplotlib 3.11.2 when the library's own are for another version.
    pub(super) baselines: Option<Baselines>,
    /// The venvs its suite runs in.
    pub(super) venvs: Venvs,
    /// What its runs need of their environment besides the venv.
    pub(super) environment: Environment,
    /// What the progress line calls the suite.
    pub(super) title: &'static str,
}

/// What a library's runs need of their environment besides the venv.
#[derive(Clone, Copy, Debug)]
pub(super) struct Environment {
    /// The variables they set.
    pub(super) variables: &'static [(&'static str, &'static str)],
    /// Whether they need an X display (lifelines' plotting tests skip
    /// without `DISPLAY`): `DISPLAY`'s, else an Xvfb of their own, started
    /// for the run.
    pub(super) display: bool,
    /// Whether pytest's cache runs (scanpy's image comparisons save into
    /// its `cache` fixture).
    pub(super) cache: bool,
    /// The pytest workers its runs take, if not the runner's (healpy's
    /// tests write and remove the same files in the working directory:
    /// one).
    pub(super) workers: Option<&'static str>,
}

/// Nothing besides the venv.
const PLAIN: Environment = Environment {
    variables: &[],
    display: false,
    cache: false,
    workers: None,
};

/// Which venvs a library's suite runs in (see `venvs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Venvs {
    /// The ecosystem's, whose pins the libraries share.
    Shared,
    /// Two of its own, pinned by `python/requirements-ecosystem-<name>.txt`:
    /// its pins clash with the shared ones.
    Own,
    /// Two shared by a group of libraries, named here, pinned by
    /// `python/requirements-ecosystem-<group>.txt`: libraries kept apart
    /// from the shared venvs, whose unpinned dependencies they would move.
    Group(&'static str),
}

/// Where a library's source tree comes from.
#[derive(Clone, Copy, Debug)]
pub(super) enum Source {
    /// Its sdist on `PyPI`.
    Sdist,
    /// The source archive of the release's tag, for the libraries whose
    /// sdists ship no tests: its URL, where `{version}` stands for the
    /// pinned version, and its SHA-256.
    Archive {
        /// The archive's URL.
        url: &'static str,
        /// Its SHA-256.
        sha256: &'static str,
    },
}

/// The files of a release's source tree that its sdist leaves out, which
/// setup adds to the unpacked sdist from the library's repository.
#[derive(Clone, Copy, Debug)]
pub(super) struct LeftOut {
    /// The raw files of the repository, a release's under its tag.
    pub(super) repository: &'static str,
    /// The release's tag, where `{version}` stands for the pinned version:
    /// [`V_VERSION`] for most (`v1.2.3`), the bare version for others.
    pub(super) tag: &'static str,
    /// The files, with their SHA-256.
    pub(super) files: &'static [(&'static str, &'static str)],
}

/// The tag most releases have: `v` and the version.
pub(super) const V_VERSION: &str = "v{version}";

/// Test files of which only the tests taking a fixture run (with the
/// pytest plugin `python/ecosystem/plugin/ecosystem_select.py`).
#[derive(Clone, Copy, Debug)]
pub(super) struct OnlyTaking {
    /// The fixture.
    pub(super) fixture: &'static str,
    /// The files, in the site.
    pub(super) files: &'static [&'static str],
}

/// The data a library's tests read that setup fetches once, checking
/// each file's SHA-256, into a directory of its own
/// (`target/ecosystem/data/<library>`), where the tests find it offline.
#[derive(Clone, Copy, Debug)]
pub(super) struct Data {
    /// The files.
    pub(super) files: &'static [DataFile],
    /// What the files are.
    pub(super) form: DataForm,
    /// How the tests find the directory.
    pub(super) found: Found,
}

/// What the files of a library's [`Data`] are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DataForm {
    /// Files the tests read as they are.
    Plain,
    /// Zip archives, unpacked into the data's directory beside them
    /// (yellowbrick's datasets, which it reads unpacked and checks against
    /// their archives).
    Zips,
}

/// A file of a library's [`Data`].
#[derive(Clone, Copy, Debug)]
pub(super) struct DataFile {
    /// Where it is downloaded from.
    pub(super) url: &'static str,
    /// Its path in the data's directory.
    pub(super) path: &'static str,
    /// Its SHA-256.
    pub(super) sha256: &'static str,
}

/// How a library's tests find its [`Data`].
#[derive(Clone, Copy, Debug)]
pub(super) enum Found {
    /// The directory this variable names (cartopy's `CARTOPY_DATA_DIR`).
    Variable(&'static str),
    /// The directory of the site the library caches its downloads in,
    /// made a link to it (shap's `shap/cached_data`).
    Link(&'static str),
}

/// Whether a library's tests may reach the network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Network {
    /// They may, as they did when their outcomes were first recorded
    /// (astropy's tests of remote data).
    Allowed,
    /// They run offline: HTTP and HTTPS go through a proxy that refuses
    /// every connection, so that a download they would make fails.
    Refused,
}

/// The baselines of a library's image comparisons that setup makes under
/// matplotlib 3.11.2 (`baselines`), and where its tests read them.
#[derive(Clone, Copy, Debug)]
pub(super) struct Baselines {
    /// How they are made.
    pub(super) making: Making,
    /// The directory of the site its tests read them from, which is made
    /// a link to them; `None`: pytest-mpl reads them from
    /// `--mpl-baseline-path`.
    pub(super) read_from: Option<&'static str>,
}

/// How setup makes a library's baselines.
#[derive(Clone, Copy, Debug)]
pub(super) enum Making {
    /// pytest-mpl's `--mpl-generate-path`: one run saves the figure of
    /// every test.
    PytestMpl,
    /// The images the tests save before they compare them, in this
    /// directory of the site, taken as plotnine's
    /// `tools/update_baseline_images.py` takes them: the tests run again
    /// and again, and each run adds the images that have no baseline yet,
    /// until none is missing (a test comparing several images stops at
    /// the first missing one).
    Results(&'static str),
}

/// The file of most source trees holding their pytest configuration.
const CONFIGURATION: &str = "pyproject.toml";

/// Every library whose suite runs.
pub(super) fn all() -> impl Iterator<Item = Library> {
    [
        analysis::LIBRARIES.as_slice(),
        science::LIBRARIES.as_slice(),
        plotting::LIBRARIES.as_slice(),
        addons::LIBRARIES.as_slice(),
        charts::LIBRARIES.as_slice(),
        fields::LIBRARIES.as_slice(),
        depth::LIBRARIES.as_slice(),
    ]
    .into_iter()
    .flatten()
    .copied()
}

/// The library named `name`.
#[cfg(test)]
pub(super) fn named(name: &str) -> Library {
    all()
        .find(|library| library.name == name)
        .unwrap_or_else(|| panic!("no library {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libraries_have_one_entry_each() {
        let mut names: Vec<&str> = all().map(|library| library.name).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    #[test]
    fn libraries_say_what_they_are() {
        for library in all() {
            assert!(!library.name.is_empty(), "{library:?}");
            assert!(!library.package.is_empty(), "{}", library.name);
            assert!(!library.tests.is_empty(), "{}", library.name);
            assert!(!library.title.is_empty(), "{}", library.name);
        }
    }

    #[test]
    fn data_files_have_their_digests() {
        for library in all() {
            for file in library.data.iter().flat_map(|data| data.files) {
                assert_eq!(file.sha256.len(), 64, "{}", file.path);
                assert!(file.url.starts_with("https://"), "{}", file.url);
            }
        }
    }
}
