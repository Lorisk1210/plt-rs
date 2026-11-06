"""A pytest plugin running only the tests of some test files that take a
given fixture.

``cargo xtask ecosystem suite`` loads it with ``-p ecosystem_select`` for
the libraries whose plots are tested beside other things in the same
files, such as scikit-learn's ``sklearn/tests/test_calibration.py``, where
the tests that draw take its ``pyplot`` fixture. Each
``--only-taking=FIXTURE:FILE`` option (``FILE`` as the test ids name it)
deselects the tests of ``FILE`` that do not take ``FIXTURE``; the tests of
other files all run.
"""


def pytest_addoption(parser):
    """Add ``--only-taking``."""
    parser.addoption(
        "--only-taking",
        action="append",
        default=[],
        metavar="FIXTURE:FILE",
        help="run only the tests of FILE that take FIXTURE",
    )


def pytest_collection_modifyitems(config, items):
    """Deselect the tests of the files named that lack their fixture."""
    fixtures = {}
    for option in config.getoption("only_taking"):
        fixture, _, file = option.partition(":")
        fixtures[file] = fixture
    if not fixtures:
        return
    kept, dropped = [], []
    for item in items:
        fixture = fixtures.get(item.nodeid.split("::", 1)[0])
        if fixture is None or fixture in item.fixturenames:
            kept.append(item)
        else:
            dropped.append(item)
    if dropped:
        config.hook.pytest_deselected(items=dropped)
        items[:] = kept
