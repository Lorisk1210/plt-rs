"""A pytest plugin giving every test the colour codes the first one had.

yellowbrick names matplotlib's single-letter colour codes (``"b"``,
``"g"``, ``"r"``, ...) after its own palette when imported
(``set_aesthetic``), and its test of ``set_color_codes`` leaves them named
after matplotlib's colours (``set_color_codes("reset")``). The red of
every later ``"r"`` in that worker changes, and with it the images of the
tests drawing with it (``test_binning``'s lines), under matplotlib as
under pltrs: the baselines setup makes, and the comparisons with them,
then depend on which tests pytest-xdist gave that worker before.
``cargo xtask ecosystem suite`` loads this plugin with
``-p ecosystem_color_codes`` for such libraries: each test starts with the
codes the worker's first test started with, those the library's import
set.
"""

import pytest

#: The single-letter colour codes.
CODES = "bgrmyck"

#: The colours the codes named when the worker's first test started.
_FIRST = {}


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_setup(item):
    """Name the codes as they were before the worker's first test."""
    # The library's conftest, collected by now, imported matplotlib.
    from matplotlib import colors

    if not _FIRST:
        _FIRST.update((code, colors._colors_full_map[code]) for code in CODES)
        return
    for code, color in _FIRST.items():
        # Setting a colour empties the conversions' cache.
        colors._colors_full_map[code] = color
