"""A pytest plugin keeping pytest's own test ids.

yellowbrick's conftest replaces the id of every test it collects by the
first lines of the docstrings of its class and function, for
pytest-spec's display (``pytest_itemcollected``). Those ids repeat
(parametrized tests share their docstring) and do not name the tests, so
pytest-xdist cannot match its workers' collections, and the outcomes of
``cargo xtask ecosystem suite``, recorded by test id, would mix. The
runs of such a library load this plugin with ``-p ecosystem_ids``: each
test gets back the id pytest gave it once the library's hooks ran.
"""

import pytest

#: Where a test keeps the id pytest gave it.
_OWN_ID = pytest.StashKey[str]()


@pytest.hookimpl(tryfirst=True)
def pytest_itemcollected(item):
    """Keep the id pytest gave the test, before the library's hooks."""
    item.stash[_OWN_ID] = item.nodeid


@pytest.hookimpl(trylast=True)
def pytest_collection_modifyitems(items):
    """Give every test back the id pytest gave it."""
    for item in items:
        own = item.stash.get(_OWN_ID, None)
        if own is not None:
            item._nodeid = own
