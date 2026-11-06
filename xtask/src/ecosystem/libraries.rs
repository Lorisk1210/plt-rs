//! The libraries whose own test suites `cargo xtask ecosystem suite` runs:
//! where each suite is in the library's sdist, which of its tests reach
//! matplotlib (only those run) and the pytest options its runs add to the
//! library's own configuration.

/// A library whose test suite runs.
#[derive(Clone, Copy, Debug)]
pub(super) struct Library {
    /// Its distribution's name, as the requirements pin it.
    pub(super) name: &'static str,
    /// The package it installs (its import name).
    pub(super) package: &'static str,
    /// What the suite is, in the sdist, and where it goes in the site.
    pub(super) copies: &'static [(&'static str, &'static str)],
    /// The files of the release's source tree that the suite reads and
    /// the sdist leaves out.
    pub(super) left_out: Option<LeftOut>,
    /// The test files and directories that run, in the site.
    pub(super) tests: &'static [&'static str],
    /// The test files of [`Library::tests`] of which only some tests run.
    pub(super) only_taking: Option<OnlyTaking>,
    /// What the library's pytest runs add to its configuration.
    pub(super) options: &'static [&'static str],
    /// What the progress line calls the suite.
    pub(super) title: &'static str,
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

/// pandas' iris data, which the tests of its plots of several columns
/// read (`andrews_curves`, `parallel_coordinates`, `radviz`).
const IRIS: &str = "pandas/tests/io/data/csv/iris.csv";

/// The libraries whose suites run.
pub(super) const LIBRARIES: [Library; 9] = [
    Library {
        name: "pandas",
        package: "pandas",
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
        tests: &["pandas/tests/plotting"],
        only_taking: None,
        options: &[],
        title: "pandas' plotting tests",
    },
    Library {
        name: "seaborn",
        package: "seaborn",
        copies: &[("tests", "seaborn/tests")],
        left_out: None,
        tests: &["seaborn/tests"],
        only_taking: None,
        options: &[],
        title: "seaborn's tests",
    },
    Library {
        name: "xarray",
        package: "xarray",
        copies: &[
            ("conftest.py", "conftest.py"),
            ("xarray/tests", "xarray/tests"),
        ],
        left_out: None,
        tests: &["xarray/tests/test_plot.py"],
        only_taking: None,
        // Without `--strict-config` and pytest-mypy-plugins' option, which
        // only xarray's typing tests need (and pytest-asyncio, whose
        // setting its configuration has).
        options: &["--override-ini=addopts=--strict-markers"],
        title: "xarray's plotting tests",
    },
    Library {
        name: "networkx",
        package: "networkx",
        copies: &[
            ("networkx/conftest.py", "networkx/conftest.py"),
            ("networkx/drawing/tests", "networkx/drawing/tests"),
        ],
        left_out: None,
        tests: &[
            "networkx/drawing/tests/test_image_comparison_pylab_mpl.py",
            "networkx/drawing/tests/test_pylab.py",
        ],
        only_taking: None,
        // The image comparisons against its baselines, as its CI runs them,
        // their results in the site (not in the system's temporary
        // directory).
        options: &["--mpl", "--mpl-results-path=mpl-results"],
        title: "networkx' drawing tests",
    },
    Library {
        name: "scikit-learn",
        package: "sklearn",
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
        title: "scikit-learn's displays and plots",
    },
    Library {
        name: "statsmodels",
        package: "statsmodels",
        copies: &[
            ("statsmodels/conftest.py", "statsmodels/conftest.py"),
            ("statsmodels/graphics/tests", "statsmodels/graphics/tests"),
        ],
        left_out: None,
        tests: &["statsmodels/graphics/tests"],
        only_taking: None,
        // Without pytest-randomly's option (the plugin would shuffle every
        // library's tests), the durations and the JUnit report.
        options: &["--override-ini=addopts=--strict-markers"],
        title: "statsmodels' graphics tests",
    },
    Library {
        name: "geopandas",
        package: "geopandas",
        copies: &[
            ("geopandas/conftest.py", "geopandas/conftest.py"),
            ("geopandas/tests", "geopandas/tests"),
        ],
        left_out: Some(LeftOut {
            repository: "https://raw.githubusercontent.com/geopandas/geopandas",
            files: GEOPANDAS_DATA,
        }),
        tests: &["geopandas/tests/test_plotting.py"],
        only_taking: None,
        options: &[],
        title: "geopandas' plotting tests",
    },
    Library {
        name: "plotnine",
        package: "plotnine",
        copies: &[("tests", "plotnine/tests")],
        left_out: None,
        tests: &["plotnine/tests"],
        only_taking: None,
        // Without the coverage report (pytest-cov).
        options: &["--override-ini=addopts=--pyargs --import-mode=importlib"],
        title: "plotnine's tests",
    },
    Library {
        name: "astropy",
        package: "astropy",
        copies: &[
            ("conftest.py", "conftest.py"),
            ("astropy/conftest.py", "astropy/conftest.py"),
            ("astropy/visualization/tests", "astropy/visualization/tests"),
            (ASTROPY_WCSAXES, ASTROPY_WCSAXES),
        ],
        left_out: None,
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
        // (some of them do); they draw, but compare no images (`--mpl`),
        // as their baselines are hashes of matplotlib 3.8's images.
        options: &["--remote-data=any"],
        title: "astropy's visualization tests",
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
