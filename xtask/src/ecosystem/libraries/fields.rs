//! The libraries of other fields that plot with matplotlib, in venvs of
//! their own (`Venvs::Group("fields")`): astronomy's (sunpy, aplpy's
//! astronomical images, photutils, regions, plasmapy), statistics' corner
//! plots (corner, whose tests compare images with matplotlib's
//! `testing` decorators), colormaps (cmasher, cmocean), animations
//! (animatplot), football pitches (mplsoccer), tables (plottable, in venvs
//! of its own), waffle charts (pywaffle), stereonets (mplstereonet) and
//! audio (librosa's display). Of the larger suites, only the test files
//! reaching matplotlib run.

use super::addons::{ADDON, ROOT_IS_SITE};
use super::{Baselines, LeftOut, Library, Making, Venvs};

/// The group of venvs the libraries run in.
const GROUP: Venvs = Venvs::Group("fields");

/// What a library of other fields has unless it says otherwise: an
/// add-on's description (its sdist, offline, run in the site) in this
/// group's venvs.
const FIELD: Library = Library {
    venvs: GROUP,
    ..ADDON
};

/// The pytest option dropping a library's own `addopts` (doctests of its
/// documentation, coverage, remote-data switches), which its runs here
/// leave out.
const NO_ADDOPTS: &str = "--override-ini=addopts=";

/// The pytest options keeping the warning about a configuration option of
/// a plugin the venvs lack (asdf's `asdf_schema_root`) from failing the
/// collection where the configuration turns warnings into errors.
const IGNORE_UNKNOWN_OPTIONS: &str = "-Wignore::pytest.PytestConfigWarning";

/// The option making the site pytest's root for a library whose tests run
/// in its package's directory (`Library::runs_in`), one level down.
const ROOT_IS_SITE_ABOVE: &str = "--rootdir=..";

/// The libraries.
pub(super) const LIBRARIES: [Library; 14] = [
    Library {
        name: "sunpy",
        package: "sunpy",
        copies: &[
            ("sunpy/conftest.py", "sunpy/conftest.py"),
            (
                "sunpy/timeseries/conftest.py",
                "sunpy/timeseries/conftest.py",
            ),
            ("sunpy/image/tests", "sunpy/image/tests"),
            ("sunpy/map/tests", "sunpy/map/tests"),
            ("sunpy/tests/tests", "sunpy/tests/tests"),
            (
                "sunpy/timeseries/sources/tests",
                "sunpy/timeseries/sources/tests",
            ),
            ("sunpy/timeseries/tests", "sunpy/timeseries/tests"),
            ("sunpy/util/tests", "sunpy/util/tests"),
            (
                "sunpy/visualization/animator/tests",
                "sunpy/visualization/animator/tests",
            ),
            (
                "sunpy/visualization/colormaps/tests",
                "sunpy/visualization/colormaps/tests",
            ),
            ("sunpy/visualization/tests", "sunpy/visualization/tests"),
        ],
        configuration: "pytest.ini",
        tests: &[
            "sunpy/image/tests/test_transform.py",
            "sunpy/map/tests/test_compositemap.py",
            "sunpy/map/tests/test_mapbase.py",
            "sunpy/map/tests/test_map_factory.py",
            "sunpy/map/tests/test_mapsequence.py",
            "sunpy/map/tests/test_plotting.py",
            "sunpy/map/tests/test_reproject_to.py",
            "sunpy/tests/tests/test_self_test.py",
            "sunpy/timeseries/sources/tests/test_eve.py",
            "sunpy/timeseries/sources/tests/test_fermi_gbm.py",
            "sunpy/timeseries/sources/tests/test_goes.py",
            "sunpy/timeseries/sources/tests/test_lyra.py",
            "sunpy/timeseries/sources/tests/test_noaa.py",
            "sunpy/timeseries/sources/tests/test_norh.py",
            "sunpy/timeseries/sources/tests/test_rhessi.py",
            "sunpy/timeseries/tests/test_timeseriesbase.py",
            "sunpy/util/tests/test_sysinfo.py",
            "sunpy/visualization/animator/tests/test_mapsequenceanimator.py",
            "sunpy/visualization/colormaps/tests/test_cm.py",
            "sunpy/visualization/tests/test_drawing.py",
            "sunpy/visualization/tests/test_visualization.py",
        ],
        options: &[NO_ADDOPTS],
        title: "sunpy's plotting tests",
        ..FIELD
    },
    Library {
        name: "aplpy",
        package: "aplpy",
        copies: &[
            ("conftest.py", "conftest.py"),
            ("aplpy/tests", "aplpy/tests"),
        ],
        tests: &["aplpy/tests"],
        // Without the doctests of its documentation.
        options: &[NO_ADDOPTS],
        title: "APLpy's tests",
        ..FIELD
    },
    Library {
        name: "photutils",
        package: "photutils",
        copies: &[
            ("photutils/conftest.py", "photutils/conftest.py"),
            ("photutils/aperture/tests", "photutils/aperture/tests"),
            ("photutils/background/tests", "photutils/background/tests"),
            ("photutils/profiles/tests", "photutils/profiles/tests"),
            ("photutils/psf/tests", "photutils/psf/tests"),
            (
                "photutils/segmentation/tests",
                "photutils/segmentation/tests",
            ),
            ("photutils/utils/tests", "photutils/utils/tests"),
        ],
        tests: &[
            "photutils/aperture/tests/test_aperture_common.py",
            "photutils/aperture/tests/test_bounding_box.py",
            "photutils/aperture/tests/test_photometry.py",
            "photutils/aperture/tests/test_polygon.py",
            "photutils/aperture/tests/test_positional_kwargs.py",
            "photutils/background/tests/test_background_2d.py",
            "photutils/profiles/tests/test_curve_of_growth.py",
            "photutils/profiles/tests/test_positional_kwargs.py",
            "photutils/profiles/tests/test_radial_profile.py",
            "photutils/psf/tests/test_epsf_builder.py",
            "photutils/psf/tests/test_epsf_stars.py",
            "photutils/psf/tests/test_groupers.py",
            "photutils/psf/tests/test_model_plotting.py",
            "photutils/segmentation/tests/test_catalog.py",
            "photutils/segmentation/tests/test_core.py",
            "photutils/utils/tests/test_colormaps.py",
            "photutils/utils/tests/test_misc.py",
            "photutils/utils/tests/test_optional_deps.py",
            "photutils/utils/tests/test_positional_kwargs.py",
        ],
        options: &[NO_ADDOPTS, IGNORE_UNKNOWN_OPTIONS],
        title: "photutils' plotting tests",
        ..FIELD
    },
    Library {
        name: "regions",
        package: "regions",
        copies: &[
            ("regions/conftest.py", "regions/conftest.py"),
            ("regions/core/tests", "regions/core/tests"),
            ("regions/io/ds9/tests", "regions/io/ds9/tests"),
            ("regions/shapes/tests", "regions/shapes/tests"),
        ],
        tests: &[
            "regions/core/tests/test_bounding_box.py",
            "regions/io/ds9/tests/test_ds9.py",
            "regions/shapes/tests/test_circle.py",
            "regions/shapes/tests/test_common.py",
            "regions/shapes/tests/test_ellipse.py",
            "regions/shapes/tests/test_line.py",
            "regions/shapes/tests/test_point.py",
            "regions/shapes/tests/test_polygon.py",
            "regions/shapes/tests/test_rectangle.py",
        ],
        options: &[NO_ADDOPTS, IGNORE_UNKNOWN_OPTIONS],
        title: "regions' plotting tests",
        ..FIELD
    },
    Library {
        name: "plasmapy",
        package: "plasmapy",
        copies: &[
            ("conftest.py", "conftest.py"),
            ("tests/plasma", "plasmapy/tests/plasma"),
            (
                "tests/diagnostics/charged_particle_radiography",
                "plasmapy/tests/diagnostics/charged_particle_radiography",
            ),
        ],
        tests: &[
            "plasmapy/tests/plasma/test_grids.py",
            "plasmapy/tests/diagnostics/charged_particle_radiography/test_synthetic_radiography.py",
        ],
        options: &[NO_ADDOPTS],
        title: "PlasmaPy's plotting tests",
        ..FIELD
    },
    Library {
        name: "corner",
        package: "corner",
        // With the baseline images its `image_comparison` reads.
        copies: &[("tests", "corner/tests")],
        tests: &["corner/tests"],
        options: &[ROOT_IS_SITE],
        title: "corner's tests",
        ..FIELD
    },
    Library {
        name: "cmasher",
        package: "cmasher",
        copies: &[("conftest.py", "conftest.py"), ("tests", "cmasher/tests")],
        tests: &["cmasher/tests"],
        // In one process: its conftest moves to the temporary directory as
        // it is imported, which a worker does before it collects the
        // tests named by their paths in the site.
        options: &["-n0"],
        title: "CMasher's tests",
        ..FIELD
    },
    Library {
        name: "cmocean",
        package: "cmocean",
        copies: &[("tests", "cmocean/tests")],
        configuration: "setup.cfg",
        tests: &["cmocean/tests"],
        options: &[ROOT_IS_SITE],
        title: "cmocean's tests",
        ..FIELD
    },
    Library {
        name: "animatplot",
        package: "animatplot",
        copies: &[("tests", "animatplot/tests")],
        // The helper its animation tests compare their frames with (a
        // `FileMovieWriter` of its own, `matplotlib.testing.compare`).
        left_out: Some(LeftOut {
            repository: "https://raw.githubusercontent.com/t-makaro/animatplot",
            tag: "{version}",
            files: &[(
                "tests/tools.py",
                "f821071396204b4dcb49c5955fb90f3ef00292487438661442bbfd6b62011ad9",
            )],
        }),
        configuration: "setup.cfg",
        // Its tests import their helpers by their own name (`tests.tools`)
        // and read and write their frames relative to the directory.
        runs_in: Some("animatplot"),
        tests: &["animatplot/tests"],
        options: &[ROOT_IS_SITE_ABOVE],
        // Its baseline frames are not in its sdist, and its repository's are
        // another matplotlib's: setup takes those the tests write.
        baselines: Some(Baselines {
            making: Making::Results("animatplot/tests/output_images"),
            read_from: Some("animatplot/tests/baseline_images"),
        }),
        title: "animatplot's animations",
        ..FIELD
    },
    Library {
        name: "mplsoccer",
        package: "mplsoccer",
        copies: &[("tests", "mplsoccer/tests")],
        tests: &["mplsoccer/tests"],
        options: &[ROOT_IS_SITE],
        title: "mplsoccer's pitches",
        ..FIELD
    },
    Library {
        name: "plottable",
        package: "plottable",
        copies: &[("tests", "plottable/tests")],
        configuration: "setup.cfg",
        tests: &["plottable/tests"],
        options: &[ROOT_IS_SITE],
        // Its wheel installs its tests as a top-level `tests` package, which
        // would take the place of the other libraries' own (animatplot's
        // `tests.tools`).
        venvs: Venvs::Own,
        title: "plottable's tables",
        ..FIELD
    },
    Library {
        name: "pywaffle",
        package: "pywaffle",
        copies: &[("tests", "pywaffle/tests")],
        tests: &["pywaffle/tests"],
        options: &[ROOT_IS_SITE],
        title: "PyWaffle's tests",
        ..FIELD
    },
    Library {
        name: "mplstereonet",
        package: "mplstereonet",
        // With the examples one of its tests runs, by their own name.
        copies: &[
            ("tests", "mplstereonet/tests"),
            ("examples", "mplstereonet/examples"),
        ],
        configuration: "setup.cfg",
        runs_in: Some("mplstereonet"),
        tests: &["mplstereonet/tests"],
        options: &[ROOT_IS_SITE_ABOVE],
        title: "mplstereonet's tests",
        ..FIELD
    },
    Library {
        name: "librosa",
        package: "librosa",
        copies: &[("tests", "librosa/tests")],
        configuration: "setup.cfg",
        tests: &[
            "librosa/tests/test_display.py",
            "librosa/tests/test_display_integration.py",
        ],
        // Without its coverage and image comparisons (its baselines are
        // another matplotlib's).
        options: &[NO_ADDOPTS],
        title: "librosa's display",
        ..FIELD
    },
];
