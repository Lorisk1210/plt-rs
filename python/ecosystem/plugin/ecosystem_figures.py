"""A pytest plugin letting go of the figures pytest-mpl's tests returned.

pytest-mpl keeps the figure each of its tests returns in its plugin's
``return_value`` dictionary, keyed by the test's name, and reads it back
in the same test's call phase to compare or save it; it closes the
figure but never drops the entry. A worker therefore holds every figure
of the tests it ran until the run ends: ultraplot's 2,000 tests grew a
worker of matplotlib's run to over 4 GB, eight of them past the
harness' memory cap. ``cargo xtask ecosystem suite`` loads this plugin
with ``-p ecosystem_figures`` for such libraries: after each test the
entries are dropped, which pytest-mpl no longer reads, so that the
figures go once their test is over, as they would without it.
"""

import pytest


@pytest.hookimpl(trylast=True)
def pytest_runtest_teardown(item):
    """Drop the figures pytest-mpl kept for the tests run so far."""
    for plugin in item.config.pluginmanager.get_plugins():
        kept = getattr(plugin, "return_value", None)
        if isinstance(kept, dict) and type(plugin).__module__ == "pytest_mpl.plugin":
            kept.clear()
