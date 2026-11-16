"""pytest configuration for the matplotlib reference run of the spec.

Default mode verifies that matplotlib reproduces the committed goldens:
exactly on Linux x86-64, where they are recorded, and as far as pltrs is
held to them on a platform with goldens of its own. ``--update-goldens``
rewrites them from matplotlib instead: the common goldens on Linux x86-64,
and on such a platform only its own, for what matplotlib computes there
that fails for the common ones (see ``spec_support.golden``).
"""

from __future__ import annotations

import locale
import pathlib
import sys

import pytest

sys.path.insert(0, str(pathlib.Path(__file__).parent))

import matplotlib  # noqa: E402

matplotlib.use("Agg")

from matplotlib.testing import setup as _mpl_testing_setup  # noqa: E402

_FILTERED = pytest.StashKey[bool]()


def reference_rc() -> None:
    """Reset matplotlib to the exact configuration the goldens assume."""
    _mpl_testing_setup()
    # Dates written in the C locale's words and order (`%c`, `%x`, `%X`), as
    # pltrs writes them: matplotlib's setup asks for en_US.UTF-8, which some
    # hosts have (Windows: "1/1/2000 12:00:00 AM") and the one recording the
    # common goldens does not.
    locale.setlocale(locale.LC_TIME, "C")
    # Unhinted glyph outlines: geometrically exact and reproducible without
    # a FreeType bytecode interpreter.
    matplotlib.rcParams["text.hinting"] = "none"


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption(
        "--update-goldens",
        action="store_true",
        help="rewrite spec/goldens from matplotlib instead of verifying them",
    )


@pytest.fixture(autouse=True)
def _reference_configuration() -> None:
    reference_rc()


@pytest.fixture(scope="session")
def update_goldens(request: pytest.FixtureRequest) -> bool:
    return bool(request.config.getoption("--update-goldens"))


def pytest_deselected(items: list[pytest.Item]) -> None:
    if items:
        items[0].config.stash[_FILTERED] = True


def pytest_sessionfinish(session: pytest.Session, exitstatus: int) -> None:
    """Write or verify probe goldens once every probe test has run.

    Verification is a subset check, so running a single module or test is
    fine. Updating requires an unfiltered, fully passing run so that goldens
    are never truncated.
    """
    import json

    from spec_support.golden import (GoldenAnswers, agrees, all_probe_sets, dump_json,
                                     golden_probes, probe_key, record_probes)

    config = session.config
    reporter = config.pluginmanager.get_plugin("terminalreporter")
    if config.getoption("--update-goldens"):
        if exitstatus != 0 or config.stash.get(_FILTERED, False):
            reporter.write_line("goldens NOT updated: run the full, passing suite", red=True)
            return
        for probes in all_probe_sets():
            if len(probes):
                record_probes(probes)
        return

    problems = []
    for probes in all_probe_sets():
        if not len(probes):
            continue
        committed = golden_probes(probes.area)
        if committed is None:
            problems.append(f"{probes.area}: golden missing (run with --update-goldens)")
            continue
        fresh = json.loads(dump_json(probes.document()))["probes"]
        answers = GoldenAnswers(committed, {probe_key(probe): probe for probe in fresh})
        for probe in fresh:
            golden = answers.key(probe)
            if golden is None:
                problems.append(f"{probes.area}: new probe {probe_key(probe)} not in golden")
            elif not agrees(committed[golden], probe):
                problems.append(
                    f"{probes.area}: matplotlib disagrees with golden for {probe_key(probe)}")
    if problems:
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
        for line in problems:
            reporter.write_line(f"GOLDEN MISMATCH {line}", red=True)

