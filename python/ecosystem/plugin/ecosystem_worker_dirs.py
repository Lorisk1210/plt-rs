"""A pytest plugin giving each pytest-xdist worker a working directory of
its own.

plotnine's tests of ``ggsave`` and ``save_as_pdf_pages`` save plots under
names counted from 0 in each process (``filename-0.png``, ...) in the
working directory, check that the file is there and delete it: workers
sharing one directory write, check and delete each other's files, and
which of those tests fail depends on how pytest-xdist spreads them (under
matplotlib as under pltrs). ``cargo xtask ecosystem suite`` loads this
plugin with ``-p ecosystem_worker_dirs`` for such libraries: once a worker
has collected its tests (whose paths are relative to the run's directory),
it works in an empty directory of its own under the temporary directory.
"""

import os
import pathlib
import shutil
import tempfile

import pytest


@pytest.hookimpl(trylast=True)
def pytest_collection_finish(session):
    """Move a worker into its own directory, once its tests are collected."""
    worker = os.environ.get("PYTEST_XDIST_WORKER")
    if worker is None:
        return
    directory = pathlib.Path(tempfile.gettempdir()) / f"working-dir-{worker}"
    shutil.rmtree(directory, ignore_errors=True)
    directory.mkdir(parents=True)
    os.chdir(directory)
