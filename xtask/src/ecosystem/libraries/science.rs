//! The libraries of the sciences: astropy, cartopy, sympy, nilearn, arviz
//! (its plots, `arviz-plots`), scanpy and mne.

use super::{
    Baselines, CONFIGURATION, Data, DataFile, DataForm, Environment, Found, Library, Making,
    Network, PLAIN, Source, Venvs,
};

/// The libraries.
pub(super) const LIBRARIES: [Library; 7] = [
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
        data: None,
        network: Network::Allowed,
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: None,
        }),
        venvs: Venvs::Shared,
        environment: PLAIN,
        title: "astropy's visualization tests",
    },
    Library {
        name: "cartopy",
        package: "cartopy",
        source: Source::Sdist,
        copies: &[("lib/cartopy/tests", "cartopy/tests")],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["cartopy/tests/mpl"],
        only_taking: None,
        // Without the tests of web services (map tiles, WMS and WMTS),
        // which download at every run; pytest-mpl's results in the site.
        options: &["-m", "not network", "--mpl-results-path=mpl-results"],
        data: Some(Data {
            files: NATURAL_EARTH,
            form: DataForm::Plain,
            found: Found::Variable("CARTOPY_DATA_DIR"),
        }),
        network: Network::Refused,
        baselines: None,
        venvs: Venvs::Shared,
        environment: PLAIN,
        title: "cartopy's plotting tests",
    },
    Library {
        name: "sympy",
        package: "sympy",
        source: Source::Sdist,
        copies: &[
            ("sympy/conftest.py", "sympy/conftest.py"),
            ("sympy/plotting/tests", "sympy/plotting/tests"),
            (SYMPY_CONTROL, SYMPY_CONTROL),
            (SYMPY_QUANTUM, SYMPY_QUANTUM),
            (SYMPY_EXTERNAL, SYMPY_EXTERNAL),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &[
            "sympy/plotting/tests",
            "sympy/physics/control/tests/test_control_plots.py",
            "sympy/physics/quantum/tests/test_circuitplot.py",
            "sympy/external/tests/test_importtools.py",
        ],
        only_taking: None,
        options: &[],
        data: None,
        network: Network::Refused,
        baselines: None,
        venvs: Venvs::Shared,
        environment: PLAIN,
        title: "sympy's plotting tests",
    },
    Library {
        name: "nilearn",
        package: "nilearn",
        source: Source::Sdist,
        copies: &[
            ("nilearn/conftest.py", "nilearn/conftest.py"),
            ("nilearn/plotting/tests", "nilearn/plotting/tests"),
            (NILEARN_DISPLAYS, NILEARN_DISPLAYS),
            (NILEARN_IMAGES, NILEARN_IMAGES),
            (NILEARN_MATRICES, NILEARN_MATRICES),
            (NILEARN_SURFACES, NILEARN_SURFACES),
            (NILEARN_GLM, NILEARN_GLM),
            (NILEARN_MASKERS, NILEARN_MASKERS),
            (NILEARN_REPORTING, NILEARN_REPORTING),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        // Its image comparisons by name, which its conftest leaves out of
        // the directories it collects under any matplotlib but the oldest
        // it supports, whose images its baselines are: they compare with
        // the baselines setup makes. The other tests beside them by name
        // too, as pytest collects a directory named beside one of its
        // files as that file alone.
        tests: &[
            "nilearn/plotting/tests/test_baseline_comparisons.py",
            "nilearn/plotting/tests/test_cm.py",
            "nilearn/plotting/tests/test_engine_utils.py",
            "nilearn/plotting/tests/test_find_cuts.py",
            "nilearn/plotting/tests/test_html_connectome.py",
            "nilearn/plotting/tests/test_html_stat_map.py",
            "nilearn/plotting/tests/test_img_comparisons.py",
            "nilearn/plotting/tests/test_js_plotting_utils.py",
            "nilearn/plotting/tests/test_utils.py",
            NILEARN_DISPLAYS,
            NILEARN_IMAGES,
            NILEARN_MATRICES,
            NILEARN_SURFACES,
            "nilearn/glm/tests/test_baseline_comparisons.py",
            "nilearn/glm/tests/test_glm_reporter.py",
            "nilearn/maskers/tests/test_baseline_comparisons.py",
            "nilearn/maskers/tests/test_html_report.py",
            NILEARN_REPORTING,
        ],
        only_taking: None,
        // Without `--strict-config` and the HTML report's template, which
        // needs pytest-html-reporter; its fetchers are mocked by its
        // conftest. Its tests parametrized over sets of functions in the
        // same order in every worker (`ecosystem_order`).
        options: &[
            "--override-ini=addopts=-ra --strict-markers",
            "--mpl-results-path=mpl-results",
            "-p",
            "ecosystem_order",
        ],
        data: None,
        network: Network::Refused,
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: None,
        }),
        venvs: Venvs::Shared,
        environment: PLAIN,
        title: "nilearn's plotting and reporting tests",
    },
    Library {
        name: "arviz-plots",
        package: "arviz_plots",
        source: Source::Archive {
            url: "https://github.com/arviz-devs/arviz-plots/archive/refs/tags/v{version}.tar.gz",
            sha256: "15a735a39b7f6b48aa2f8631784cff0d1630f4324c08ea8e40b228a1fa88df37",
        },
        copies: &[("tests", "arviz_plots/tests")],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["arviz_plots/tests"],
        only_taking: None,
        // Its tests drawing with matplotlib, not those of bokeh, plotly
        // or its backend drawing nothing: those parametrized with it, or
        // marked with it, and its `test_matplotlib.py`.
        options: &["-k", "matplotlib"],
        data: None,
        network: Network::Refused,
        baselines: None,
        venvs: Venvs::Shared,
        environment: PLAIN,
        title: "arviz' plots with matplotlib",
    },
    Library {
        name: "scanpy",
        package: "scanpy",
        source: Source::Sdist,
        // The logo its spatial tests read from the project's root, two
        // levels above its package, which is the run's directory, above
        // the site.
        copies: &[
            ("tests", "scanpy/tests"),
            (
                "docs/_static/img/Scanpy_Logo_RGB.png",
                "../docs/_static/img/Scanpy_Logo_RGB.png",
            ),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        // Its plotting tests, and those beside them that plot.
        tests: &[
            "scanpy/tests/plotting",
            "scanpy/tests/test_embedding_density.py",
            "scanpy/tests/test_paga.py",
        ],
        only_taking: None,
        options: &[],
        data: Some(Data {
            files: SCANPY_DATA,
            form: DataForm::Plain,
            // Where its tests' plugin has it keep its datasets, in pytest's
            // cache.
            found: Found::Link(".pytest_cache/d/scanpy-data"),
        }),
        network: Network::Refused,
        // Its baselines are matplotlib 3.11's images, compared with a
        // tolerance of 15.
        baselines: None,
        venvs: Venvs::Own,
        // pytest's cache, whose `cache` fixture its image comparisons save
        // their differences in.
        environment: Environment {
            cache: true,
            ..PLAIN
        },
        title: "scanpy's plotting tests",
    },
    Library {
        name: "mne",
        package: "mne",
        // Its sdist ships no tests.
        source: Source::Archive {
            url: "https://github.com/mne-tools/mne-python/archive/refs/tags/v{version}.tar.gz",
            sha256: "b9d1e28d3e7c608975be64afc8cc3c145a32ba1992288f31b791456b2c8cef79",
        },
        // Its visualisation tests, with the recordings they read beside
        // the readers' tests (those needing the large MNE-testing-data,
        // which it downloads, skip without it).
        copies: &[
            ("mne/conftest.py", "mne/conftest.py"),
            ("mne/viz/tests", "mne/viz/tests"),
            ("mne/io/tests/data", "mne/io/tests/data"),
            ("mne/io/kit/tests/data", "mne/io/kit/tests/data"),
        ],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &["mne/viz/tests"],
        only_taking: None,
        // Without the coverage, JUnit, durations and doctest options of its
        // configuration.
        options: &["--override-ini=addopts="],
        data: None,
        network: Network::Refused,
        baselines: None,
        venvs: Venvs::Own,
        environment: PLAIN,
        title: "mne's visualisation tests",
    },
];

/// astropy's tests of `WCSAxes`.
const ASTROPY_WCSAXES: &str = "astropy/visualization/wcsaxes/tests";

/// sympy's tests of its control systems' plots.
const SYMPY_CONTROL: &str = "sympy/physics/control/tests";
/// sympy's tests of its quantum circuits, which it draws.
const SYMPY_QUANTUM: &str = "sympy/physics/quantum/tests";
/// sympy's tests of its optional imports, matplotlib's among them.
const SYMPY_EXTERNAL: &str = "sympy/external/tests";

/// nilearn's tests of its displays.
const NILEARN_DISPLAYS: &str = "nilearn/plotting/displays/tests";
/// nilearn's tests of its plots of images.
const NILEARN_IMAGES: &str = "nilearn/plotting/image/tests";
/// nilearn's tests of its plots of matrices.
const NILEARN_MATRICES: &str = "nilearn/plotting/matrix/tests";
/// nilearn's tests of its plots of surfaces.
const NILEARN_SURFACES: &str = "nilearn/plotting/surface/tests";
/// nilearn's tests of its general linear models, with their reports.
const NILEARN_GLM: &str = "nilearn/glm/tests";
/// nilearn's tests of its maskers, with their reports.
const NILEARN_MASKERS: &str = "nilearn/maskers/tests";
/// nilearn's tests of its reports' tables.
const NILEARN_REPORTING: &str = "nilearn/reporting/tests";

/// A file of Natural Earth's release v5.1.2 (from its repository's tag),
/// where cartopy's downloader would put it: its scale, theme and name, and
/// its SHA-256.
macro_rules! natural_earth {
    ($scale:literal, $theme:literal, $name:literal, $sha256:literal) => {
        DataFile {
            url: concat!(
                "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/",
                $scale,
                "_",
                $theme,
                "/ne_",
                $scale,
                "_",
                $name
            ),
            path: concat!(
                "shapefiles/natural_earth/",
                $theme,
                "/ne_",
                $scale,
                "_",
                $name
            ),
            sha256: $sha256,
        }
    };
}

/// The Natural Earth maps cartopy's tests draw (those its downloader
/// fetches in a run of them that may reach the network, the same bytes as
/// the release's), which they find in `CARTOPY_DATA_DIR`.
const NATURAL_EARTH: &[DataFile] = &[
    natural_earth!(
        "110m",
        "cultural",
        "admin_0_boundary_lines_land.cpg",
        "3ad3031f5503a4404af825262ee8232cc04d4ea6683d42c5dd0a2f2a27ac9824"
    ),
    natural_earth!(
        "110m",
        "cultural",
        "admin_0_boundary_lines_land.dbf",
        "373d7a150b9f8dcb6ec918b52eea0f0361ad40ef433602a1b9f59b4de668daec"
    ),
    natural_earth!(
        "110m",
        "cultural",
        "admin_0_boundary_lines_land.prj",
        "3259f0e55290a82b1350646f604e8a7ee1e2136c0320a40fad838ab40819fff8"
    ),
    natural_earth!(
        "110m",
        "cultural",
        "admin_0_boundary_lines_land.shp",
        "d19f784f92626816c064f878571f61bc43ec7bfc407666ce6a450d6e757afff0"
    ),
    natural_earth!(
        "110m",
        "cultural",
        "admin_0_boundary_lines_land.shx",
        "849899c47c51c6edd04750307b21cb109ae4c7e8ca3c5fee8606f8646dbe2787"
    ),
    natural_earth!(
        "110m",
        "physical",
        "coastline.cpg",
        "3ad3031f5503a4404af825262ee8232cc04d4ea6683d42c5dd0a2f2a27ac9824"
    ),
    natural_earth!(
        "110m",
        "physical",
        "coastline.dbf",
        "ac7e59083a862bafc20b86d73595fdf2a7c0693a411cf3ba91910a5de9df9bc3"
    ),
    natural_earth!(
        "110m",
        "physical",
        "coastline.prj",
        "3259f0e55290a82b1350646f604e8a7ee1e2136c0320a40fad838ab40819fff8"
    ),
    natural_earth!(
        "110m",
        "physical",
        "coastline.shp",
        "a1f8285d0e0a4858392cd78fc74fa3ee4cd9088947cf6e3693729dc852d70b60"
    ),
    natural_earth!(
        "110m",
        "physical",
        "coastline.shx",
        "53d8c141edfb7e05c59feecc7bb64b0e22bc2e282dfa1024d20ae54c3ef7b01e"
    ),
    natural_earth!(
        "110m",
        "physical",
        "lakes.cpg",
        "3ad3031f5503a4404af825262ee8232cc04d4ea6683d42c5dd0a2f2a27ac9824"
    ),
    natural_earth!(
        "110m",
        "physical",
        "lakes.dbf",
        "ee65c43bdb14463f9a31eda58f7245e3ae0796ac606a67bea0e8386080878cbf"
    ),
    natural_earth!(
        "110m",
        "physical",
        "lakes.prj",
        "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"
    ),
    natural_earth!(
        "110m",
        "physical",
        "lakes.shp",
        "e65b633578c284776b452bcea07126747d3d6484a507ca43866097059a389dfd"
    ),
    natural_earth!(
        "110m",
        "physical",
        "lakes.shx",
        "954c018f338248ba6ec6fde3cfe0f8742a4bdb2f886fe888baf6d7b5433c39ab"
    ),
    natural_earth!(
        "110m",
        "physical",
        "land.cpg",
        "3ad3031f5503a4404af825262ee8232cc04d4ea6683d42c5dd0a2f2a27ac9824"
    ),
    natural_earth!(
        "110m",
        "physical",
        "land.dbf",
        "db7cf6d2de2811df09bd7fcc6f243ab78a715b83571a0cb7b36b4e2af3297caa"
    ),
    natural_earth!(
        "110m",
        "physical",
        "land.prj",
        "3259f0e55290a82b1350646f604e8a7ee1e2136c0320a40fad838ab40819fff8"
    ),
    natural_earth!(
        "110m",
        "physical",
        "land.shp",
        "8689e6932b8e370e2ca4587cf3ba21e460b1235db37b6ed3c172c35b4a6088de"
    ),
    natural_earth!(
        "110m",
        "physical",
        "land.shx",
        "2719254764a70262a34333581d582d503b8af5d6626e6da4eb2b5f86e7316faa"
    ),
    natural_earth!(
        "110m",
        "physical",
        "ocean.cpg",
        "3ad3031f5503a4404af825262ee8232cc04d4ea6683d42c5dd0a2f2a27ac9824"
    ),
    natural_earth!(
        "110m",
        "physical",
        "ocean.dbf",
        "6e852a0a16f4fd4a02fcaf015b9c0f93c2374ab5f344b07346affdc2949ac1c4"
    ),
    natural_earth!(
        "110m",
        "physical",
        "ocean.prj",
        "3259f0e55290a82b1350646f604e8a7ee1e2136c0320a40fad838ab40819fff8"
    ),
    natural_earth!(
        "110m",
        "physical",
        "ocean.shp",
        "8006f16c5af40875fbe8e975bef6d4b6c1ffc7b767491bdc277fbc0b62ba5d02"
    ),
    natural_earth!(
        "110m",
        "physical",
        "ocean.shx",
        "b3388def7b13abb0e94c91053a989219c35aaaa233fb0029b68cdc5d84487c1a"
    ),
    natural_earth!(
        "110m",
        "physical",
        "rivers_lake_centerlines.cpg",
        "3ad3031f5503a4404af825262ee8232cc04d4ea6683d42c5dd0a2f2a27ac9824"
    ),
    natural_earth!(
        "110m",
        "physical",
        "rivers_lake_centerlines.dbf",
        "64a480e1e56f12c1603ab7295901923d7376b27df5f6c393d921091fe18b28f7"
    ),
    natural_earth!(
        "110m",
        "physical",
        "rivers_lake_centerlines.prj",
        "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"
    ),
    natural_earth!(
        "110m",
        "physical",
        "rivers_lake_centerlines.shp",
        "4a5d41e67457fbd3d0c82ef4ca972f42f21faf8a639d0b0368725d38940f72c2"
    ),
    natural_earth!(
        "110m",
        "physical",
        "rivers_lake_centerlines.shx",
        "1df95cdde9bd66307f27c806850227296a94a53ebcf33ec65a6164a93683b82b"
    ),
    natural_earth!(
        "50m",
        "physical",
        "coastline.cpg",
        "3ad3031f5503a4404af825262ee8232cc04d4ea6683d42c5dd0a2f2a27ac9824"
    ),
    natural_earth!(
        "50m",
        "physical",
        "coastline.dbf",
        "f100fa5a99786c63ec9eb2cd430e389bc6235b8e26d2712d33f9bfd6cb6aadfc"
    ),
    natural_earth!(
        "50m",
        "physical",
        "coastline.prj",
        "98aaf3d1c0ecadf1a424a4536de261c3daf4e373697cb86c40c43b989daf52eb"
    ),
    natural_earth!(
        "50m",
        "physical",
        "coastline.shp",
        "797d675af9613f80b51ab6049fa32e589974d7a97c6497ca56772965f179ed26"
    ),
    natural_earth!(
        "50m",
        "physical",
        "coastline.shx",
        "0ff1792f2d16b58246d074215edd9d12fa280880ecaad61a91b9382fee854065"
    ),
];

/// The datasets of scanpy's plotting tests, which `scanpy.datasets`
/// downloads from scverse's example data.
const SCANPY_DATA: &[DataFile] = &[
    DataFile {
        url: "https://exampledata.scverse.org/scanpy/pbmc3k_raw.h5ad",
        path: "pbmc3k_raw.h5ad",
        sha256: "89a96f1beaa2dd83a687666d3f19a4513ac27a2a2d12581fcd77afed7ea653a1",
    },
    DataFile {
        url: "https://exampledata.scverse.org/scanpy/pbmc3k.h5ad",
        path: "pbmc3k_processed.h5ad",
        sha256: "0db367b991dd95809732b218539ede489bea99113807f62ebd7ccc970025fe38",
    },
];
