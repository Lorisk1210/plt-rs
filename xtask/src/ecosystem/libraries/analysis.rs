//! The libraries of data analysis and machine learning: pandas, seaborn,
//! xarray, networkx, scikit-learn, statsmodels, geopandas, plotnine and
//! shap.

use super::{
    Baselines, CONFIGURATION, Data, DataFile, Found, LeftOut, Library, Making, Network, OnlyTaking,
    Source,
};

/// The libraries.
pub(super) const LIBRARIES: [Library; 9] = [
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
        data: None,
        network: Network::Allowed,
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
        data: None,
        network: Network::Allowed,
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
        data: None,
        network: Network::Allowed,
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
        data: None,
        network: Network::Allowed,
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
        data: None,
        network: Network::Allowed,
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
        data: None,
        network: Network::Allowed,
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
        data: None,
        network: Network::Allowed,
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
        data: None,
        network: Network::Allowed,
        baselines: Some(Baselines {
            making: Making::Results("plotnine/tests/result_images"),
            read_from: Some("plotnine/tests/baseline_images"),
        }),
        title: "plotnine's tests",
    },
    Library {
        name: "shap",
        package: "shap",
        source: Source::Sdist,
        copies: &[
            ("tests/conftest.py", "shap/tests/conftest.py"),
            ("tests/plots", "shap/tests/plots"),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["shap/tests/plots"],
        only_taking: None,
        options: &["--mpl-results-path=mpl-results"],
        data: Some(Data {
            files: SHAP_DATA,
            found: Found::Link("shap/cached_data"),
        }),
        network: Network::Refused,
        baselines: None,
        title: "shap's plots",
    },
];

/// pandas' iris data, which the tests of its plots of several columns
/// read (`andrews_curves`, `parallel_coordinates`, `radviz`).
const IRIS: &str = "pandas/tests/io/data/csv/iris.csv";

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

/// The datasets of shap's plots, at the commit of its repository its
/// `shap.datasets` downloads them from.
const SHAP_DATA: &[DataFile] = &[
    DataFile {
        url: "https://raw.githubusercontent.com/shap/shap/fc3e290e97ce12f76d1175d24c6e3023b4ca7d69/data/adult.data",
        path: "adult.data",
        sha256: "df25a4e32ed6f1bd4b3910d21a7bd661a09061eced7cb45555a519d9667cc87b",
    },
    DataFile {
        url: "https://raw.githubusercontent.com/shap/shap/fc3e290e97ce12f76d1175d24c6e3023b4ca7d69/data/imagenet50_224x224.npy",
        path: "imagenet50_224x224.npy",
        sha256: "7027529080119f98d72abab00865814348a550afc32defbca6eef9b336dec45d",
    },
    DataFile {
        url: "https://raw.githubusercontent.com/shap/shap/fc3e290e97ce12f76d1175d24c6e3023b4ca7d69/data/imagenet50_labels.csv",
        path: "imagenet50_labels.csv",
        sha256: "147c2226a3c7b55ff807009f23979e09838c0f34b2bfe2b0fa35024624cfcb31",
    },
];
