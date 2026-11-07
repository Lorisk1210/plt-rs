"""A pytest plugin seeding numpy's global random generator before each
test from the test's id.

lifelines' conftest seeds every test with a number it draws from that
generator (``np.random.seed(np.random.randint(1000))``), which nothing
seeds, so the data of its tests, and some of their outcomes, change from
run to run (under matplotlib as under pltrs). ``cargo xtask ecosystem
suite`` loads this plugin with ``-p ecosystem_seed`` for such libraries:
each test then draws the same seed in every run and every worker, before
the library's own hooks run.
"""

import zlib

import numpy as np
import pytest


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_setup(item):
    """Seed numpy's global generator from the test's id."""
    np.random.seed(zlib.crc32(item.nodeid.encode()))
