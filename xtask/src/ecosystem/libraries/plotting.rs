//! The libraries that make plots of their own kinds, and pytest-mpl:
//! mplfinance, matplotlib-venn, pytest-mpl, mpl-scatter-density and
//! missingno.

use super::{Baselines, CONFIGURATION, Library, Making, Network, PLAIN, Source, Venvs};

/// The libraries.
pub(super) const LIBRARIES: [Library; 5] = [
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
        data: None,
        network: Network::Allowed,
        baselines: Some(Baselines {
            making: Making::Results("mplfinance/tests/test_images"),
            read_from: Some("mplfinance/tests/reference_images"),
        }),
        venvs: Venvs::Shared,
        environment: PLAIN,
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
        data: None,
        network: Network::Allowed,
        baselines: None,
        venvs: Venvs::Shared,
        environment: PLAIN,
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
        data: None,
        network: Network::Allowed,
        baselines: None,
        venvs: Venvs::Shared,
        environment: PLAIN,
        title: "pytest-mpl's tests",
    },
    Library {
        name: "mpl-scatter-density",
        package: "mpl_scatter_density",
        source: Source::Sdist,
        copies: &[(SCATTER_DENSITY_TESTS, SCATTER_DENSITY_TESTS)],
        left_out: None,
        configuration: CONFIGURATION,
        runs_in: None,
        tests: &[SCATTER_DENSITY_TESTS],
        only_taking: None,
        // As its CI's image runs, the results in the site.
        options: &["--mpl-results-path=mpl-results"],
        data: None,
        network: Network::Refused,
        // Its baselines are matplotlib 3.0's.
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: Some("mpl_scatter_density/tests/baseline/3.0.x"),
        }),
        venvs: Venvs::Shared,
        environment: PLAIN,
        title: "mpl-scatter-density's tests",
    },
    Library {
        name: "missingno",
        package: "missingno",
        source: Source::Sdist,
        copies: &[("tests", "missingno/tests")],
        left_out: None,
        configuration: "setup.cfg",
        runs_in: None,
        tests: &["missingno/tests"],
        only_taking: None,
        // Its tests named from the site, which has no configuration of
        // pytest to make it the root.
        options: &["--rootdir=.", "--mpl-results-path=mpl-results"],
        data: None,
        network: Network::Refused,
        // Its sdist has none.
        baselines: Some(Baselines {
            making: Making::PytestMpl,
            read_from: None,
        }),
        venvs: Venvs::Shared,
        environment: PLAIN,
        title: "missingno's tests",
    },
];

/// mpl-scatter-density's tests, in its package.
const SCATTER_DENSITY_TESTS: &str = "mpl_scatter_density/tests";
