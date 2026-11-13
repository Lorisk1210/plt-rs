"""A pytest plugin closing pyplot's figures after every test.

MetPy's tests make their figures with pyplot and return them to
pytest-mpl, which closes them once compared; a test failing before (its
declarative tests download Natural Earth's maps, which an offline run
cannot) leaves its figure open. MetPy turns every warning into an error,
and pyplot warns when a 21st figure opens: the test that opens it fails,
under matplotlib as under pltrs. Which test that is depends on the tests
pytest-xdist gave the worker before, which follows their timing: three
runs under matplotlib passed 109, 108 and 109 tests, eight tests changing
outcome (``test_barb_unit_conversion``, ``test_add_unidata_logo``, ...).
``cargo xtask ecosystem suite`` loads this plugin with
``-p ecosystem_close_figures`` for such libraries: each test starts with
no figure open, as it would in a run of its own.
"""

import sys

import pytest


@pytest.hookimpl(trylast=True)
def pytest_runtest_teardown(item):
    """Close the figures pyplot holds, if the test imported it."""
    pyplot = sys.modules.get("matplotlib.pyplot")
    if pyplot is not None:
        pyplot.close("all")
