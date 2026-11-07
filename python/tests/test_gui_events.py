"""
pltrs' own windows against matplotlib's QtAgg windows, event for event:
each scenario of `gui_events` shows a figure in a real window of a
private X server, feeds it synthetic input through the server (XTEST),
and compares what the program sees — every ``mpl_connect`` event with
its attributes, what the widgets and animations report, and the pixels
where a widget shows its state — with what matplotlib's program saw
under QtAgg (``gui_events/goldens``, recorded with
``python -m gui_events --record``). The same runs on Wayland, through a
weston nested in the X server, leave out what Wayland does not tell (see
`gui_events.compare.without_crossings`). Skipped where no Xvfb (or no
weston) can run.

``python -m gui_events`` (from ``python/tests``) runs matplotlib and
pltrs side by side instead; see ``gui_events/__main__.py``.
"""

import sys

import pytest

from gui_events import goldens
from gui_events.driver import differences_of, run
from gui_events.scenarios import SCENARIOS
from gui_events.xdisplay import Unavailable, private_display

pytestmark = pytest.mark.skipif(not sys.platform.startswith("linux"),
                                reason="X11 only")


@pytest.fixture(scope="module")
def display():
    """A private X server for the module's tests."""
    try:
        with private_display() as name:
            yield name
    except Unavailable as error:
        pytest.skip(str(error))


def _check(name, display, tmp_path, wayland):
    try:
        got = run(name, "pltrs", "pltrs", display, tmp_path, wayland)
    except Unavailable as error:
        pytest.skip(str(error))
    golden = goldens.load(name)
    assert (golden is None) == (SCENARIOS[name].reference is None), \
        "a golden for every scenario compared with matplotlib"
    assert differences_of(name, golden, got, tmp_path, wayland=wayland) == []


@pytest.mark.parametrize("name", list(SCENARIOS))
def test_scenario_as_matplotlibs_qtagg(name, display, tmp_path):
    _check(name, display, tmp_path, wayland=False)


@pytest.mark.parametrize("name", [name for name, scenario in SCENARIOS.items()
                                  if not scenario.x11_only])
def test_scenario_on_wayland_as_on_x11(name, display, tmp_path):
    _check(name, display, tmp_path, wayland=True)
