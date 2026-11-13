//! The libraries drawing the charts of their fields on matplotlib's Axes,
//! in the add-ons' venvs: statannotations, `UpSetPlot`, pingouin, optuna,
//! mlxtend, `MetPy`, squarify and `SciencePlots`.

use super::addons::{ADDON, ROOT_IS_SITE};
use super::{Baselines, Data, DataFile, DataForm, Found, Library, Making, Source};

/// The libraries.
pub(super) const LIBRARIES: [Library; 8] = [
    Library {
        name: "statannotations",
        package: "statannotations",
        copies: &[("tests", "statannotations/tests")],
        configuration: "setup.cfg",
        tests: &["statannotations/tests"],
        options: &[ROOT_IS_SITE],
        title: "statannotations' tests",
        ..ADDON
    },
    Library {
        name: "upsetplot",
        package: "upsetplot",
        // Its sdist ships no tests.
        source: Source::Archive {
            url: "https://github.com/jnothman/UpSetPlot/archive/refs/tags/{version}.tar.gz",
            sha256: "e96d4bdd4e0b2305d7c3eb39bb016dcb3ee6333ffdc9475ecabb23005c4c5ad1",
        },
        copies: &[("upsetplot/tests", "upsetplot/tests")],
        configuration: "setup.cfg",
        tests: &["upsetplot/tests"],
        // Without the coverage and the doctests.
        options: &["--override-ini=addopts="],
        title: "UpSetPlot's tests",
        ..ADDON
    },
    Library {
        name: "pingouin",
        package: "pingouin",
        copies: &[(PINGOUIN_PLOTS, "pingouin/tests/test_plotting.py")],
        tests: &["pingouin/tests/test_plotting.py"],
        // Without the durations of the slowest tests.
        options: &["--override-ini=addopts=--showlocals"],
        title: "pingouin's plots",
        ..ADDON
    },
    Library {
        name: "optuna",
        package: "optuna",
        // Its tests of its plots, which import each other as the package
        // `tests` of the source tree's top.
        copies: &[
            ("tests/__init__.py", "tests/__init__.py"),
            (OPTUNA_PLOTS, OPTUNA_PLOTS),
        ],
        // Those drawing with matplotlib: its backend's own, and those
        // parametrized over both backends.
        tests: &[
            "tests/visualization_tests/matplotlib_tests",
            "tests/visualization_tests/test_contour.py",
            "tests/visualization_tests/test_edf.py",
            "tests/visualization_tests/test_intermediate_plot.py",
            "tests/visualization_tests/test_parallel_coordinate.py",
            "tests/visualization_tests/test_param_importances.py",
            "tests/visualization_tests/test_pareto_front.py",
            "tests/visualization_tests/test_slice.py",
            "tests/visualization_tests/test_timeline.py",
            "tests/visualization_tests/test_visualizations.py",
        ],
        title: "optuna's plots",
        ..ADDON
    },
    Library {
        name: "mlxtend",
        package: "mlxtend",
        // Its sdist ships no tests.
        source: Source::Archive {
            url: "https://github.com/rasbt/mlxtend/archive/refs/tags/v{version}.tar.gz",
            sha256: "408a90e6db7e56535d5946e7e103229fcac019e9a1089f57519068d4d08601a1",
        },
        copies: &[(MLXTEND_PLOTS, MLXTEND_PLOTS)],
        tests: &[MLXTEND_PLOTS],
        title: "mlxtend's plots",
        ..ADDON
    },
    Library {
        name: "metpy",
        package: "metpy",
        copies: &[("conftest.py", "conftest.py"), ("tests/plots", METPY_PLOTS)],
        tests: &[METPY_PLOTS],
        // pyplot's figures closed after each test, as the figures its
        // failing tests leave open make the next ones fail on pyplot's
        // warning, which tests depending on the workers' timing.
        options: &[
            "--mpl-results-path=mpl-results",
            "-p",
            "ecosystem_close_figures",
        ],
        data: Some(Data {
            files: METPY_DATA,
            form: DataForm::Plain,
            found: Found::Variable("TEST_DATA_DIR"),
        }),
        // Its baselines are older matplotlib's.
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: Some("metpy/tests/plots/baseline"),
        }),
        title: "MetPy's plots",
        ..ADDON
    },
    Library {
        name: "squarify",
        package: "squarify",
        copies: &[("tests", "squarify/tests")],
        configuration: "setup.cfg",
        tests: &["squarify/tests"],
        options: &[ROOT_IS_SITE],
        title: "squarify's tests",
        ..ADDON
    },
    Library {
        name: "scienceplots",
        package: "scienceplots",
        copies: &[("src/scienceplots/tests", "scienceplots/tests")],
        tests: &["scienceplots/tests"],
        title: "SciencePlots' styles",
        ..ADDON
    },
];

/// pingouin's tests of its plots.
const PINGOUIN_PLOTS: &str = "tests/test_plotting.py";
/// optuna's tests of its plots.
const OPTUNA_PLOTS: &str = "tests/visualization_tests";
/// mlxtend's tests of its plots.
const MLXTEND_PLOTS: &str = "mlxtend/plotting/tests";
/// `MetPy`'s tests of its plots.
const METPY_PLOTS: &str = "metpy/tests/plots";

/// A file of `MetPy`'s test data at the release's tag, as its `get_test_data`
/// would download it into `TEST_DATA_DIR`, with its SHA-256 (which its
/// manifest lists).
macro_rules! metpy_data {
    ($name:literal, $sha256:literal) => {
        DataFile {
            url: concat!(
                "https://github.com/Unidata/MetPy/raw/v1.7.1/staticdata/",
                $name
            ),
            path: $name,
            sha256: $sha256,
        }
    };
}

/// The data `MetPy`'s tests of its plots read.
const METPY_DATA: &[DataFile] = &[
    metpy_data!(
        "AK-REGIONAL_8km_3.9_20160408_1445.gini",
        "4919b80abe95a765a42a6e3904ef1e38b8be804dc52276ef5db6b587f85abd55"
    ),
    metpy_data!(
        "CAM_test.nc",
        "ab62e6dca907c34eb44b842c67eb3eb817127306feb3524fa6d83670b628727c"
    ),
    metpy_data!(
        "GFS_global.nc",
        "feae73f72340ee9e8b04fca92f182638a99316ad7262152809b7ccb9b6691e10"
    ),
    metpy_data!(
        "GFS_test.nc",
        "b69ae13179428667f6bc14dada1d5f9af4d3737b2b76a79a6001664e1525df3c"
    ),
    metpy_data!(
        "irma_gfs_example.nc",
        "98e582bca9467e9c77aafa1e5079d6bedebaa97fabe79711bd911b32d92c32e2"
    ),
    metpy_data!(
        "metar_20190701_1200.txt",
        "740715c698718557871ce8c93b43771f0f1b643ab3f5e43cf7cf76678de35880"
    ),
    metpy_data!(
        "NAM_test.nc",
        "12338ad06d5bd223e99e2872b20a9c80d58af0c546731e4b00a6619adc247cd0"
    ),
    metpy_data!(
        "narr_example.nc",
        "00bbe42b4dc90cc95cf5d12f3f5e7d5508cb2592d72dc8081ebacea8a526fb1c"
    ),
    metpy_data!(
        "NHEM-MULTICOMP_1km_IR_20151208_2100.gini",
        "c144b29284aa915e6fd1b8f01939c656f2c72c3d7a9e0af5397f93067fe0d952"
    ),
    metpy_data!(
        "SFC_obs.csv",
        "582d434442880f8414a555f22f7faf9b07b1170f390628e68c5c182e4431f426"
    ),
    metpy_data!(
        "UPA_obs.csv",
        "106fa93db40f32cf7cb5e969e5dacb3d4e3387f7a3379ba90cc15149754fc640"
    ),
    metpy_data!(
        "WPC_sfc_fronts_20210628_1800.txt",
        "45cc32bf2649befc1d23c7b3b1245c68aea0cf7e7d2e4189e00509e471c18e89"
    ),
];
