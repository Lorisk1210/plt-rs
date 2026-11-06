//! The libraries whose own test suites `cargo xtask ecosystem suite` runs:
//! where each suite is in the library's source tree (its sdist, or the
//! release's source archive where the sdist leaves the tests out), which
//! of its tests reach matplotlib (only those run), the pytest options its
//! runs add to the library's own configuration, and how setup remakes the
//! baselines of its image comparisons under matplotlib 3.11.2 where its
//! own are another version's.

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
    /// The baselines of its image comparisons, which setup makes under
    /// matplotlib 3.11.2 when the library's own are for another version.
    pub(super) baselines: Option<Baselines>,
    /// What the progress line calls the suite.
    pub(super) title: &'static str,
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
    /// The raw files of the repository; a release's are under its tag,
    /// `v<version>`.
    pub(super) repository: &'static str,
    /// The files, with their SHA-256.
    pub(super) files: &'static [(&'static str, &'static str)],
}

/// Test files of which only the tests taking a fixture run (with the
/// pytest plugin `python/ecosystem/plugin/ecosystem_select.py`).
#[derive(Clone, Copy, Debug)]
pub(super) struct OnlyTaking {
    /// The fixture.
    pub(super) fixture: &'static str,
    /// The files, in the site.
    pub(super) files: &'static [&'static str],
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

/// pandas' iris data, which the tests of its plots of several columns
/// read (`andrews_curves`, `parallel_coordinates`, `radviz`).
const IRIS: &str = "pandas/tests/io/data/csv/iris.csv";

/// The libraries whose suites run.
pub(super) const LIBRARIES: [Library; 12] = [
    Library {
        name: "pandas",
        package: "pandas",
        source: Source::Sdist,
        copies: &[
            ("pandas/conftest.py", "pandas/conftest.py"),
            ("pandas/tests/__init__.py", "pandas/tests/__init__.py"),
            ("pandas/tests/plotting", "pandas/tests/plotting"),
            (IRIS, IRIS),
        ],
        left_out: Some(LeftOut {
            repository: "https://raw.githubusercontent.com/pandas-dev/pandas",
            files: &[(
                IRIS,
                "904d791b308aa79bf4f06669f63be3f1ab331a23bd508051065e00f700f573f9",
            )],
        }),
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["pandas/tests/plotting"],
        only_taking: None,
        options: &[],
        baselines: None,
        title: "pandas' plotting tests",
    },
    Library {
        name: "seaborn",
        package: "seaborn",
        source: Source::Sdist,
        copies: &[("tests", "seaborn/tests")],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["seaborn/tests"],
        only_taking: None,
        options: &[],
        baselines: None,
        title: "seaborn's tests",
    },
    Library {
        name: "xarray",
        package: "xarray",
        source: Source::Sdist,
        copies: &[
            ("conftest.py", "conftest.py"),
            ("xarray/tests", "xarray/tests"),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["xarray/tests/test_plot.py"],
        only_taking: None,
        // Without `--strict-config` and pytest-mypy-plugins' option, which
        // only xarray's typing tests need (and pytest-asyncio, whose
        // setting its configuration has).
        options: &["--override-ini=addopts=--strict-markers"],
        baselines: None,
        title: "xarray's plotting tests",
    },
    Library {
        name: "networkx",
        package: "networkx",
        source: Source::Sdist,
        copies: &[
            ("networkx/conftest.py", "networkx/conftest.py"),
            ("networkx/drawing/tests", "networkx/drawing/tests"),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &[
            "networkx/drawing/tests/test_image_comparison_pylab_mpl.py",
            "networkx/drawing/tests/test_pylab.py",
        ],
        only_taking: None,
        // The image comparisons against its baselines, as its CI runs them,
        // their results in the site (not in the system's temporary
        // directory).
        options: &["--mpl", "--mpl-results-path=mpl-results"],
        baselines: None,
        title: "networkx' drawing tests",
    },
    Library {
        name: "scikit-learn",
        package: "sklearn",
        source: Source::Sdist,
        copies: &[
            ("sklearn/conftest.py", "sklearn/conftest.py"),
            (SKLEARN_INSPECTION, SKLEARN_INSPECTION),
            (SKLEARN_METRICS, SKLEARN_METRICS),
            (
                "sklearn/model_selection/tests",
                "sklearn/model_selection/tests",
            ),
            ("sklearn/tests", "sklearn/tests"),
            ("sklearn/tree/tests", "sklearn/tree/tests"),
            ("sklearn/utils/tests", "sklearn/utils/tests"),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &[
            "sklearn/inspection/_plot/tests/test_boundary_decision_display.py",
            "sklearn/metrics/_plot/tests/test_common_curve_display.py",
            "sklearn/metrics/_plot/tests/test_confusion_matrix_display.py",
            "sklearn/metrics/_plot/tests/test_det_curve_display.py",
            "sklearn/metrics/_plot/tests/test_precision_recall_display.py",
            "sklearn/metrics/_plot/tests/test_predict_error_display.py",
            "sklearn/metrics/_plot/tests/test_roc_curve_display.py",
            SKLEARN_PARTIAL_DEPENDENCE,
            SKLEARN_CURVES,
            SKLEARN_CALIBRATION,
            SKLEARN_TREES,
            SKLEARN_PLOTTING,
        ],
        only_taking: Some(OnlyTaking {
            fixture: "pyplot",
            files: &[
                SKLEARN_PARTIAL_DEPENDENCE,
                SKLEARN_CURVES,
                SKLEARN_CALIBRATION,
                SKLEARN_TREES,
                SKLEARN_PLOTTING,
            ],
        }),
        options: &[],
        baselines: None,
        title: "scikit-learn's displays and plots",
    },
    Library {
        name: "statsmodels",
        package: "statsmodels",
        source: Source::Sdist,
        copies: &[
            ("statsmodels/conftest.py", "statsmodels/conftest.py"),
            ("statsmodels/graphics/tests", "statsmodels/graphics/tests"),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["statsmodels/graphics/tests"],
        only_taking: None,
        // Without pytest-randomly's option (the plugin would shuffle every
        // library's tests), the durations and the JUnit report.
        options: &["--override-ini=addopts=--strict-markers"],
        baselines: None,
        title: "statsmodels' graphics tests",
    },
    Library {
        name: "geopandas",
        package: "geopandas",
        source: Source::Sdist,
        copies: &[
            ("geopandas/conftest.py", "geopandas/conftest.py"),
            ("geopandas/tests", "geopandas/tests"),
        ],
        left_out: Some(LeftOut {
            repository: "https://raw.githubusercontent.com/geopandas/geopandas",
            files: GEOPANDAS_DATA,
        }),
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["geopandas/tests/test_plotting.py"],
        only_taking: None,
        options: &[],
        baselines: None,
        title: "geopandas' plotting tests",
    },
    Library {
        name: "plotnine",
        package: "plotnine",
        source: Source::Sdist,
        copies: &[("tests", "plotnine/tests")],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["plotnine/tests"],
        only_taking: None,
        // Without the coverage report (pytest-cov).
        options: &["--override-ini=addopts=--pyargs --import-mode=importlib"],
        baselines: Some(Baselines {
            making: Making::Results("plotnine/tests/result_images"),
            read_from: Some("plotnine/tests/baseline_images"),
        }),
        title: "plotnine's tests",
    },
    Library {
        name: "astropy",
        package: "astropy",
        source: Source::Sdist,
        copies: &[
            ("conftest.py", "conftest.py"),
            ("astropy/conftest.py", "astropy/conftest.py"),
            ("astropy/visualization/tests", "astropy/visualization/tests"),
            (ASTROPY_WCSAXES, ASTROPY_WCSAXES),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &[
            "astropy/visualization/tests/test_basic_rgb.py",
            "astropy/visualization/tests/test_histogram.py",
            "astropy/visualization/tests/test_lupton_rgb.py",
            "astropy/visualization/tests/test_norm.py",
            "astropy/visualization/tests/test_time.py",
            "astropy/visualization/tests/test_units.py",
            ASTROPY_WCSAXES,
        ],
        only_taking: None,
        // The figure tests, which astropy marks as reading remote data
        // (some of them do). They compare their images with the baselines
        // setup makes (astropy's own are hashes of matplotlib 3.8's
        // images), with each test's tolerance, pytest-mpl's results in the
        // site (not in the system's temporary directory).
        options: &["--remote-data=any", "--mpl-results-path=mpl-results"],
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: None,
        }),
        title: "astropy's visualization tests",
    },
    Library {
        name: "mplfinance",
        package: "mplfinance",
        source: Source::Archive {
            url: "https://github.com/matplotlib/mplfinance/archive/refs/tags/v{version}.tar.gz",
            sha256: "1703ea263d7de3b7e72825265de346ff1f050f8ffcb52210a044c1971fdd513a",
        },
        copies: &[
            ("tests", "mplfinance/tests"),
            ("examples/data", "mplfinance/examples/data"),
        ],
        left_out: None,
        configuration: "pytest.ini",
        runs_in: Some("mplfinance"),
        tests: &["mplfinance/tests"],
        only_taking: None,
        options: &[],
        // Its reference images are older matplotlib's.
        baselines: Some(Baselines {
            making: Making::Results("mplfinance/tests/test_images"),
            read_from: Some("mplfinance/tests/reference_images"),
        }),
        title: "mplfinance's tests",
    },
    Library {
        name: "matplotlib-venn",
        package: "matplotlib_venn",
        source: Source::Archive {
            url: "https://github.com/konstantint/matplotlib-venn/archive/refs/tags/{version}.tar.gz",
            sha256: "6111d8463101e1fe1c567a12d0fa8fff4315ce9acddcbbc5f0915d85d49cb019",
        },
        copies: &[("tests", "matplotlib_venn/tests")],
        left_out: None,
        configuration: "setup.cfg",
        runs_in: Some("matplotlib_venn"),
        // The package, as `py.test` in the source tree collects it: the
        // tests, and the examples of its modules (`--doctest-modules`).
        tests: &["matplotlib_venn"],
        only_taking: None,
        options: &[],
        baselines: None,
        title: "matplotlib-venn's tests",
    },
    Library {
        name: "pytest-mpl",
        package: "pytest_mpl",
        source: Source::Sdist,
        copies: &[("tests", "pytest_mpl/tests")],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["pytest_mpl/tests"],
        only_taking: None,
        // As the second run of its CI, which compares the images, the
        // results in the site.
        options: &["--mpl", "--mpl-results-path=mpl-results"],
        baselines: None,
        title: "pytest-mpl's tests",
    },
];

/// scikit-learn's tests of its inspection displays.
const SKLEARN_INSPECTION: &str = "sklearn/inspection/_plot/tests";

/// scikit-learn's tests of its metrics' displays.
const SKLEARN_METRICS: &str = "sklearn/metrics/_plot/tests";

/// The test files of scikit-learn whose tests taking `pyplot` run.
const SKLEARN_PARTIAL_DEPENDENCE: &str =
    "sklearn/inspection/_plot/tests/test_plot_partial_dependence.py";
/// See [`SKLEARN_PARTIAL_DEPENDENCE`].
const SKLEARN_CURVES: &str = "sklearn/model_selection/tests/test_plot.py";
/// See [`SKLEARN_PARTIAL_DEPENDENCE`].
const SKLEARN_CALIBRATION: &str = "sklearn/tests/test_calibration.py";
/// See [`SKLEARN_PARTIAL_DEPENDENCE`].
const SKLEARN_TREES: &str = "sklearn/tree/tests/test_export.py";
/// See [`SKLEARN_PARTIAL_DEPENDENCE`].
const SKLEARN_PLOTTING: &str = "sklearn/utils/tests/test_plotting.py";

/// astropy's tests of `WCSAxes`.
const ASTROPY_WCSAXES: &str = "astropy/visualization/wcsaxes/tests";

/// geopandas' test data: the Natural Earth countries and cities and New
/// York's boroughs, which its sdist leaves out.
const GEOPANDAS_DATA: &[(&str, &str)] = &[
    (
        "geopandas/tests/data/nybb_16a.zip",
        "b4a98fc3b77be5af58676de9560c2782b902b83d1d8c7914dea8d0e75177e2a1",
    ),
    (
        "geopandas/tests/data/naturalearth_cities/naturalearth_cities.cpg",
        "09fc313075748ce8ead962229ed89c919d5a9ff71974ee725a0b48bb87d975a2",
    ),
    (
        "geopandas/tests/data/naturalearth_cities/naturalearth_cities.dbf",
        "e144e2db1ba39f352193c70870ddf745e1faef70990160f17d56eeea485e9a33",
    ),
    (
        "geopandas/tests/data/naturalearth_cities/naturalearth_cities.prj",
        "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4",
    ),
    (
        "geopandas/tests/data/naturalearth_cities/naturalearth_cities.shp",
        "b9e20f6dd2896285370d049f3e09159b64317a4102fbbc2390171246e8b3a7d0",
    ),
    (
        "geopandas/tests/data/naturalearth_cities/naturalearth_cities.shx",
        "f173bcd955c36a35a9b190ae315019ff7fa2b48ddaae538a6ed2f04c60a08a52",
    ),
    (
        "geopandas/tests/data/naturalearth_lowres/naturalearth_lowres.cpg",
        "09fc313075748ce8ead962229ed89c919d5a9ff71974ee725a0b48bb87d975a2",
    ),
    (
        "geopandas/tests/data/naturalearth_lowres/naturalearth_lowres.dbf",
        "5cfbcaa21ce5fad798abf2ec65ab0db59538f9bb8a37273ef62b6aa8121487fd",
    ),
    (
        "geopandas/tests/data/naturalearth_lowres/naturalearth_lowres.prj",
        "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4",
    ),
    (
        "geopandas/tests/data/naturalearth_lowres/naturalearth_lowres.shp",
        "1f689e60b357e1e98702d5d9f774e95e77fc6b324487cadf57eb9317d533ce12",
    ),
    (
        "geopandas/tests/data/naturalearth_lowres/naturalearth_lowres.shx",
        "7933917ebd636eb80822c1bddfa31dd7597ca9e6f89b21e23283c594ed6482c5",
    ),
];
