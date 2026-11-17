//! The third batch of libraries, chosen for how deep they reach into
//! matplotlib, in venvs of their own (`Venvs::Group("depth")`): quantum
//! states' plots (qutip), control systems' (python-control), cosmological
//! chains' (getdist), sky maps (healpy), seismology (obspy), astrophysical
//! volumes (yt), observation planning (astroplan), colour science
//! (colour-science), image processing's plots (scikit-image), fits
//! (lmfit), finance (pyfolio-reloaded), signal processing (mplsignal),
//! graphs on manifolds (pygsp), phylogenetic trees (biopython), geometries
//! (shapely), raincloud plots (ptitprince) and plotly's export of
//! matplotlib figures, which walks every artist. Of the larger suites,
//! only the test files reaching matplotlib run, without the libraries' own
//! `addopts` (coverage, doctests, `--maxfail`).

use super::addons::{ADDON, ROOT_IS_SITE};
use super::{Environment, Library, PLAIN, Venvs};

/// What a library of the batch has unless it says otherwise: an add-on's
/// description (its sdist, offline, run in the site) in this group's
/// venvs.
const DEPTH: Library = Library {
    venvs: Venvs::Group("depth"),
    ..ADDON
};

/// The pytest option dropping a library's own `addopts`.
const NO_ADDOPTS: &str = "--override-ini=addopts=";

/// The option making the site pytest's root for a library whose tests run
/// in a directory of its package two levels down (`Library::runs_in`).
const ROOT_IS_SITE_TWO_ABOVE: &str = "--rootdir=../..";

/// The libraries.
pub(super) const LIBRARIES: [Library; 17] = [
    Library {
        name: "qutip",
        package: "qutip",
        copies: &[("qutip/tests", "qutip/tests")],
        // `solver/test_results.py::TestMultiTrajResult::test_merge_result`
        // averages trajectories of unseeded `np.random` states within a
        // 10 % tolerance, which some runs miss under matplotlib as well (3
        // runs of 40 had one of its four cases fail; task ECO9).
        tests: &[
            "qutip/tests/test_ipynbtools.py",
            "qutip/tests/test_visualization.py",
            "qutip/tests/test_bloch.py",
            "qutip/tests/test_animation.py",
            "qutip/tests/solver/test_sode_method.py",
            "qutip/tests/solver/test_results.py",
        ],
        options: &[NO_ADDOPTS, ROOT_IS_SITE],
        title: "QuTiP's plots",
        ..DEPTH
    },
    Library {
        name: "control",
        package: "control",
        copies: &[("control/tests", "control/tests")],
        tests: &[
            "control/tests/slycot_convert_test.py",
            "control/tests/nyquist_test.py",
            "control/tests/descfcn_test.py",
            "control/tests/matlab_test.py",
            "control/tests/rlocus_test.py",
            "control/tests/freqresp_test.py",
            "control/tests/matlab2_test.py",
            "control/tests/phaseplot_test.py",
            "control/tests/freqplot_test.py",
            "control/tests/sisotool_test.py",
            "control/tests/convert_test.py",
            "control/tests/docstrings_test.py",
            "control/tests/nichols_test.py",
            "control/tests/timeplot_test.py",
            "control/tests/config_test.py",
            "control/tests/kwargs_test.py",
            "control/tests/response_test.py",
            "control/tests/statefbk_test.py",
            "control/tests/frd_test.py",
            "control/tests/discrete_test.py",
            "control/tests/pzmap_test.py",
            "control/tests/ctrlplot_test.py",
            "control/tests/statesp_test.py",
            "control/tests/timeresp_test.py",
        ],
        options: &[NO_ADDOPTS],
        title: "python-control's plots",
        ..DEPTH
    },
    Library {
        name: "getdist",
        package: "getdist",
        copies: &[("getdist/tests", "getdist/tests")],
        tests: &[
            "getdist/tests/getdist_test.py",
            "getdist/tests/test_distributions.py",
        ],
        options: &[NO_ADDOPTS, ROOT_IS_SITE],
        // Its file tests write chains to one directory under the temporary
        // directory (`gettdist_tests`) and remove it after each test, which
        // several workers race for (under matplotlib as well).
        environment: Environment {
            workers: Some("1"),
            ..PLAIN
        },
        title: "getdist's plots",
        ..DEPTH
    },
    Library {
        name: "healpy",
        package: "healpy",
        copies: &[("test", "healpy/test")],
        tests: &["healpy/test"],
        options: &[NO_ADDOPTS, ROOT_IS_SITE],
        // Its FITS tests write, read and remove the same files in the
        // working directory, which several workers race for.
        environment: Environment {
            workers: Some("1"),
            ..PLAIN
        },
        title: "healpy's tests",
        ..DEPTH
    },
    Library {
        name: "pyfolio-reloaded",
        package: "pyfolio",
        copies: &[("tests", "pyfolio/tests")],
        tests: &["pyfolio/tests"],
        options: &[NO_ADDOPTS],
        title: "pyfolio's tear sheets",
        ..DEPTH
    },
    Library {
        name: "plotly",
        package: "plotly",
        copies: &[
            ("plotly/matplotlylib/tests", "plotly/matplotlylib/tests"),
            (
                "plotly/matplotlylib/mplexporter/tests",
                "plotly/matplotlylib/mplexporter/tests",
            ),
        ],
        tests: &[
            "plotly/matplotlylib/mplexporter/tests/test_utils.py",
            "plotly/matplotlylib/tests/test_renderer.py",
            "plotly/matplotlylib/mplexporter/tests/test_basic.py",
        ],
        options: &[NO_ADDOPTS],
        title: "plotly's export of figures",
        ..DEPTH
    },
    Library {
        name: "astroplan",
        package: "astroplan",
        copies: &[("astroplan/plots/tests", "astroplan/plots/tests")],
        configuration: "setup.cfg",
        tests: &["astroplan/plots/tests/test_sky.py"],
        options: &[NO_ADDOPTS],
        title: "astroplan's sky charts",
        ..DEPTH
    },
    Library {
        name: "shapely",
        package: "shapely",
        copies: &[("src/shapely/tests", "shapely/tests")],
        tests: &["shapely/tests/test_plotting.py"],
        options: &[NO_ADDOPTS],
        title: "shapely's plotting",
        ..DEPTH
    },
    Library {
        name: "pygsp",
        package: "pygsp",
        copies: &[("pygsp/tests", "pygsp/tests")],
        // `test_graphs.py::test_laplacian` draws unseeded Erdős–Rényi graphs
        // of 100 nodes (p = 0.1) and fails when one has an isolated node,
        // whose normalized Laplacian diagonal is 0: about 0.3 % of each
        // graph, under matplotlib as well (task ECO10).
        tests: &["pygsp/tests/test_plotting.py", "pygsp/tests/test_graphs.py"],
        options: &[NO_ADDOPTS],
        title: "PyGSP's plots",
        ..DEPTH
    },
    Library {
        name: "lmfit",
        package: "lmfit",
        // With the example data its tests read beside them (`../examples`).
        copies: &[("tests", "lmfit/tests"), ("examples", "lmfit/examples")],
        tests: &["lmfit/tests/test_model.py"],
        options: &[NO_ADDOPTS],
        title: "lmfit's model plots",
        ..DEPTH
    },
    Library {
        name: "biopython",
        package: "Bio",
        // Of its large test directory, the tree drawings and the trees
        // they read, relative to the directory.
        copies: &[
            (
                "Tests/test_Phylo_matplotlib.py",
                "Bio/Tests/test_Phylo_matplotlib.py",
            ),
            ("Tests/PhyloXML", "Bio/Tests/PhyloXML"),
        ],
        runs_in: Some("Bio/Tests"),
        tests: &["Bio/Tests/test_Phylo_matplotlib.py"],
        options: &[NO_ADDOPTS, ROOT_IS_SITE_TWO_ABOVE],
        title: "Biopython's tree drawings",
        ..DEPTH
    },
    Library {
        name: "mplsignal",
        package: "mplsignal",
        copies: &[("tests", "mplsignal/tests")],
        tests: &["mplsignal/tests"],
        options: &[NO_ADDOPTS],
        title: "mplsignal's plots",
        ..DEPTH
    },
    Library {
        name: "ptitprince",
        package: "ptitprince",
        copies: &[("tests", "ptitprince/tests")],
        tests: &["ptitprince/tests"],
        options: &[NO_ADDOPTS],
        title: "ptitprince's raincloud plots",
        ..DEPTH
    },
    Library {
        name: "obspy",
        package: "obspy",
        copies: &[
            ("obspy/core/tests", "obspy/core/tests"),
            ("obspy/imaging/tests", "obspy/imaging/tests"),
            ("obspy/io/nordic/tests", "obspy/io/nordic/tests"),
            ("obspy/realtime/tests", "obspy/realtime/tests"),
            ("obspy/signal/tests", "obspy/signal/tests"),
            ("obspy/taup/tests", "obspy/taup/tests"),
        ],
        // `core/tests/test_trace.py::TestTrace::test_taper_scipy_plugins`
        // pops "cosine" from the cached table of taper entry points, after
        // which `test_stream.py::TestStream::test_taper` and
        // `test_method_chaining` fail in the same worker, under matplotlib
        // as well: which tests xdist puts together decides it (task ECO9).
        tests: &[
            "obspy/imaging/tests/test_plot.py",
            "obspy/imaging/tests/test_waveform.py",
            "obspy/core/tests/test_network.py",
            "obspy/core/tests/test_station.py",
            "obspy/imaging/tests/test_scan.py",
            "obspy/core/tests/test_utcdatetime.py",
            "obspy/core/tests/test_event.py",
            "obspy/imaging/tests/test_spectrogram.py",
            "obspy/signal/tests/test_detrend.py",
            "obspy/imaging/tests/test_beachball.py",
            "obspy/imaging/tests/test_backend.py",
            "obspy/core/tests/test_trace.py",
            "obspy/imaging/tests/test_mopad.py",
            "obspy/imaging/tests/test_ppsd.py",
            "obspy/core/tests/test_inventory.py",
            "obspy/imaging/tests/test_mopad_script.py",
            "obspy/core/tests/test_util_base.py",
            "obspy/signal/tests/test_spectral_estimation.py",
            "obspy/signal/tests/test_trigger.py",
            "obspy/signal/tests/test_tf_misfit.py",
            "obspy/core/tests/test_response.py",
            "obspy/core/tests/test_channel.py",
            "obspy/signal/tests/test_interpolation.py",
            "obspy/signal/tests/test_calibration.py",
            "obspy/core/tests/test_stream.py",
            "obspy/signal/tests/test_invsim.py",
            "obspy/io/nordic/tests/test_nordic.py",
            "obspy/taup/tests/test_taup_plotting.py",
            "obspy/realtime/tests/test_signal.py",
        ],
        options: &[NO_ADDOPTS, ROOT_IS_SITE],
        title: "ObsPy's plots",
        ..DEPTH
    },
    Library {
        name: "yt",
        package: "yt",
        copies: &[("yt/visualization/tests", "yt/visualization/tests")],
        tests: &["yt/visualization/tests"],
        options: &[NO_ADDOPTS],
        title: "yt's visualization tests",
        ..DEPTH
    },
    Library {
        name: "colour-science",
        package: "colour",
        copies: &[
            ("colour/plotting/tests", "colour/plotting/tests"),
            (
                "colour/plotting/tm3018/tests",
                "colour/plotting/tm3018/tests",
            ),
        ],
        tests: &["colour/plotting/tests", "colour/plotting/tm3018/tests"],
        options: &[NO_ADDOPTS],
        title: "colour-science's plotting",
        ..DEPTH
    },
    Library {
        name: "scikit-image",
        package: "skimage",
        // Its tests are a tree of their own beside the package, with the
        // configuration of their run (`tests/conftest.py`).
        copies: &[("tests", "tests")],
        tests: &[
            "tests/skimage/transform/test_radon_transform.py",
            "tests/skimage/filters/test_thresholding.py",
            "tests/skimage/_shared/test_geometry.py",
            "tests/skimage/draw/test_draw.py",
            "tests/skimage/feature/test_hog.py",
            "tests/skimage/io/test_plugin.py",
            "tests/skimage/feature/test_util.py",
            "tests/skimage/io/test_mpl_imshow.py",
        ],
        options: &[NO_ADDOPTS],
        title: "scikit-image's plotting",
        ..DEPTH
    },
];
