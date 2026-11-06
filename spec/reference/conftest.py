"""pytest configuration for the matplotlib reference run of the spec.

Default mode verifies that matplotlib reproduces the committed goldens
exactly. ``--update-goldens`` rewrites them from matplotlib instead.
"""

from __future__ import annotations

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

    from spec_support.golden import all_probe_sets, dump_json, values_equal

    config = session.config
    reporter = config.pluginmanager.get_plugin("terminalreporter")
    if config.getoption("--update-goldens"):
        if exitstatus != 0 or config.stash.get(_FILTERED, False):
            reporter.write_line("goldens NOT updated: run the full, passing suite", red=True)
            return
        for probes in all_probe_sets():
            if len(probes):
                probes.path.parent.mkdir(parents=True, exist_ok=True)
                probes.path.write_text(dump_json(probes.document()),
                                       encoding="utf-8", newline="\n")
        return

    problems = []
    for probes in all_probe_sets():
        if not len(probes):
            continue
        if not probes.path.exists():
            problems.append(f"{probes.area}: golden missing (run with --update-goldens)")
            continue
        committed = json.loads(probes.path.read_text(encoding="utf-8"))["probes"]
        committed = {_key(p): p for p in committed}
        fresh = json.loads(dump_json(probes.document()))["probes"]
        for probe in fresh:
            golden = committed.get(_key(probe))
            if golden is None:
                problems.append(f"{probes.area}: new probe {_key(probe)} not in golden")
            elif not values_equal(golden, probe):
                problems.append(f"{probes.area}: matplotlib disagrees with golden for {_key(probe)}")
    if problems:
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
        for line in problems:
            reporter.write_line(f"GOLDEN MISMATCH {line}", red=True)


def _key(probe: dict) -> str:
    import json

    return f"{probe['op']} {json.dumps(probe['args'], sort_keys=True)}"
