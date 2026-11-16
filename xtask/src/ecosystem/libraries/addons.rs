//! The libraries that build on matplotlib's parts, in venvs of their own
//! (`Venvs::Group`): exporters that walk a figure and read every artist
//! (mpld3, matplot2tikz), add-ons driving its events or placing artists
//! on it (mplcursors, mpl-interactions, matplotlib-scalebar,
//! matplotlib-label-lines), and libraries subclassing its classes
//! (ultraplot, mplhep, holoviews' matplotlib backend, mpltern, windrose,
//! python-ternary, pycirclize).

use super::{Baselines, CONFIGURATION, Library, Making, Network, PLAIN, Source, Venvs};

/// The group of venvs the add-ons run in.
pub(super) const GROUP: Venvs = Venvs::Group("addons");

/// What an add-on's description has unless it says otherwise: its sdist,
/// its configuration in `pyproject.toml`, run in the site, offline, in the
/// group's venvs.
pub(super) const ADDON: Library = Library {
    name: "",
    package: "",
    source: Source::Sdist,
    copies: &[],
    left_out: None,
    configuration: CONFIGURATION,
    runs_in: None,
    tests: &[],
    only_taking: None,
    options: &[],
    data: None,
    network: Network::Refused,
    baselines: None,
    venvs: GROUP,
    environment: PLAIN,
    title: "",
};

/// The pytest option dropping a library's own `addopts` (coverage, the
/// notebooks' and doctests' plugins), which its runs here leave out.
const NO_ADDOPTS: &str = "--override-ini=addopts=";

/// The option making the site pytest's root, for the libraries whose
/// configuration file has no section of pytest's (which would otherwise
/// find the root above the site, and name the tests from there).
pub(super) const ROOT_IS_SITE: &str = "--rootdir=.";

/// pytest-mpl's results in the site, not in the system's temporary
/// directory.
const MPL_RESULTS: &str = "--mpl-results-path=mpl-results";

/// The pytest plugin letting go of the figures pytest-mpl keeps until a
/// run ends (`python/ecosystem/plugin/ecosystem_figures.py`), for the
/// libraries returning many: eight workers of ultraplot's run held them
/// past the memory cap.
const RELEASE_FIGURES: &str = "ecosystem_figures";

/// The libraries.
pub(super) const LIBRARIES: [Library; 13] = [
    Library {
        name: "mpld3",
        package: "mpld3",
        copies: &[
            (MPLD3_TESTS, MPLD3_TESTS),
            (MPLEXPORTER_TESTS, MPLEXPORTER_TESTS),
        ],
        configuration: "setup.cfg",
        // Not its snapshots of the browser's drawing, which need node and
        // diffimg.
        tests: &[
            "mpld3/tests/test_elements.py",
            "mpld3/tests/test_empty_collections.py",
            "mpld3/tests/test_figure.py",
            "mpld3/tests/test_html.py",
            "mpld3/tests/test_js_libs.py",
            "mpld3/tests/test_plugin.py",
            "mpld3/tests/test_show.py",
            MPLEXPORTER_TESTS,
        ],
        options: &[ROOT_IS_SITE],
        title: "mpld3's export of figures",
        ..ADDON
    },
    Library {
        name: "matplot2tikz",
        package: "matplot2tikz",
        copies: &[("tests", "matplot2tikz/tests")],
        // Some of its tests read the figure left open before them (pyplot's
        // current figure): `test_horizontal_alignment.py` after
        // `test_hatch.py` in one worker fails under matplotlib and pltrs
        // alike (an UnboundLocalError of the hatched rectangle), so which
        // tests xdist puts together decides it, a dependency of the suite's
        // own (task ECO9).
        tests: &["matplot2tikz/tests"],
        // Its own options but the coverage.
        options: &["--override-ini=addopts=--import-mode=append"],
        title: "matplot2tikz' export of figures",
        ..ADDON
    },
    Library {
        name: "mplcursors",
        package: "mplcursors",
        copies: &[("tests", "mplcursors/tests")],
        tests: &["mplcursors/tests"],
        title: "mplcursors' tests",
        ..ADDON
    },
    Library {
        name: "mpl-interactions",
        package: "mpl_interactions",
        copies: &[("tests", "mpl_interactions/tests")],
        tests: &["mpl_interactions/tests"],
        // Without its notebooks (nbval) and doctests.
        options: &[NO_ADDOPTS],
        title: "mpl-interactions' tests",
        ..ADDON
    },
    Library {
        name: "matplotlib-scalebar",
        package: "matplotlib_scalebar",
        copies: &[("tests", "matplotlib_scalebar/tests")],
        tests: &["matplotlib_scalebar/tests"],
        title: "matplotlib-scalebar's tests",
        ..ADDON
    },
    Library {
        name: "matplotlib-label-lines",
        package: "labellines",
        copies: &[("labellines/test.py", "labellines/test.py")],
        tests: &["labellines/test.py"],
        options: &[MPL_RESULTS],
        // Its baselines are older matplotlib's.
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: Some("labellines/baseline"),
        }),
        title: "matplotlib-label-lines' tests",
        ..ADDON
    },
    Library {
        name: "ultraplot",
        package: "ultraplot",
        // With the files of the source tree its tests of its configuration,
        // releases and documentation read.
        copies: &[
            ("ultraplot/tests", "ultraplot/tests"),
            ("tools", "tools"),
            ("docs", "docs"),
            ("CITATION.cff", "CITATION.cff"),
            ("environment.yml", "environment.yml"),
            ("README.rst", "README.rst"),
        ],
        // `test_axes.py::test_panels_suplabels_three_hor_panels` draws one
        // of two layouts (RMS 3.68 apart, one column of panels moved) by
        // where the process' objects land: under matplotlib 8 runs of 30
        // drew the other, under pltrs 3 of 28, the same pixels in both;
        // without address randomisation (`setarch -R`) each drew one 24
        // times of 24. It fails whenever a run draws the layout its
        // baseline does not have, in either mode (task ECO9).
        tests: &["ultraplot/tests"],
        options: &[MPL_RESULTS, "-p", RELEASE_FIGURES],
        // Its sdist has none (its CI makes them from the release before).
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: None,
        }),
        title: "ultraplot's tests",
        ..ADDON
    },
    Library {
        name: "mplhep",
        package: "mplhep",
        copies: &[("tests", "mplhep/tests")],
        tests: &["mplhep/tests"],
        options: &[MPL_RESULTS, "-p", RELEASE_FIGURES],
        // Its sdist has none.
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: None,
        }),
        title: "mplhep's tests",
        ..ADDON
    },
    Library {
        name: "holoviews",
        package: "holoviews",
        copies: &[("holoviews/tests", "holoviews/tests")],
        tests: &["holoviews/tests/plotting/matplotlib"],
        // Its own options but the colours of the terminal.
        options: &["--override-ini=addopts=--strict-config --strict-markers"],
        title: "holoviews' matplotlib backend",
        ..ADDON
    },
    Library {
        name: "mpltern",
        package: "mpltern",
        copies: &[("tests", "mpltern/tests")],
        configuration: "pytest.ini",
        tests: &["mpltern/tests"],
        title: "mpltern's tests",
        ..ADDON
    },
    Library {
        name: "windrose",
        package: "windrose",
        // Its sdist ships no tests.
        source: Source::Archive {
            url: "https://github.com/python-windrose/windrose/archive/refs/tags/v{version}.tar.gz",
            sha256: "79488c7ce8c90b538d37d623bc8a465ece84469c5f50533a160b3ae452c3a535",
        },
        copies: &[("tests", "windrose/tests")],
        tests: &["windrose/tests"],
        // Its image comparisons with its own baselines, whose tolerances
        // (10 and 20) take matplotlib 3.11's images.
        options: &["--mpl", MPL_RESULTS],
        title: "windrose's tests",
        ..ADDON
    },
    Library {
        name: "python-ternary",
        package: "ternary",
        copies: &[("tests", "ternary/tests")],
        configuration: "setup.cfg",
        tests: &["ternary/tests"],
        options: &[ROOT_IS_SITE],
        title: "python-ternary's tests",
        ..ADDON
    },
    Library {
        name: "pycirclize",
        package: "pycirclize",
        copies: &[("tests", "pycirclize/tests")],
        tests: &["pycirclize/tests"],
        // Without the coverage.
        options: &[NO_ADDOPTS],
        title: "pycirclize's tests",
        ..ADDON
    },
];

/// mpld3's tests.
const MPLD3_TESTS: &str = "mpld3/tests";
/// The tests of mpld3's exporter, which walks a figure's artists.
const MPLEXPORTER_TESTS: &str = "mpld3/mplexporter/tests";
