"""A pytest plugin putting the tests of each test file in the order of
their ids.

pytest-xdist runs a suite only if every worker collects the same tests in
the same order. A test parametrized over a set of functions, as nilearn's
``test_baseline_comparisons.py`` are, is collected in the order of the
set, which follows the functions' addresses and so differs between
processes. ``cargo xtask ecosystem suite`` loads this plugin with
``-p ecosystem_order`` for such libraries: the files keep their order, and
the tests of each are sorted by their ids, the same in every worker.
"""


def pytest_collection_modifyitems(items):
    """Sort the tests of each file by their ids."""
    files = {}
    for item in items:
        files.setdefault(item.nodeid.split("::", 1)[0], len(files))
    items.sort(key=lambda item: (files[item.nodeid.split("::", 1)[0]], item.nodeid))
