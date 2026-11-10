"""
Smoke tests of what made outcomes change between runs of the same tree:
pltrs answering as matplotlib 3.11 answers in a state another test left
behind (the expected values were recorded from matplotlib 3.11.2 running
the same code), and the harness keeping runs apart where the tests
themselves leak state or race (colour codes, network, kernels' sockets).
"""

import argparse
import errno
import importlib.util
import os
import pathlib
import subprocess
import sys
import types
import warnings

import pytest

import pltrs
import pltrs.pyplot as plt

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

#: The source tree, its alias of pltrs as matplotlib, and the pltrs under
#: test (the tree's or an installed wheel's).
ROOT = pathlib.Path(__file__).resolve().parents[2]
ALIAS = ROOT / "python" / "alias"
PACKAGE_DIR = pathlib.Path(pltrs.__file__).resolve().parents[1]


def test_a_rasterized_3d_scatter_does_not_warn():
    # scanpy's scatter plots pass `rasterized=settings._vector_friendly`,
    # which its `test_violin` (`sc.set_figure_params()`) leaves true for
    # the tests after it in the same worker. matplotlib's 3-D scatter has
    # the 2-D collection take it before the collection becomes 3-D; a
    # `Path3DCollection` asked itself warns.
    ax = plt.figure().add_subplot(projection="3d")
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        points = ax.scatter([1, 2, 3], [1, 2, 3], [1, 2, 3], rasterized=True)
        coloured = ax.scatter([1, 2], [1, 2], [1, 2], c=[1, 2], rasterized=True)
    assert type(points).__name__ == "Path3DCollection"
    assert (points.get_rasterized(), coloured.get_rasterized()) == (True, True)
    with pytest.warns(UserWarning, match=r"Rasterization of '<.*Path3DCollection object at"):
        points.set_rasterized(True)


#: A conftest naming the colour code "r" as yellowbrick's import does.
CONFTEST = """
from matplotlib import colors
colors._colors_full_map["r"] = "#ca0b03"
"""

#: Tests of which the first leaves "r" named otherwise, as yellowbrick's
#: test of `set_color_codes` does.
CODE_TESTS = """
from matplotlib import colors

def test_names_the_codes_after_matplotlibs_colours():
    colors._colors_full_map["r"] = "#ff0000"

def test_draws_with_the_librarys_red():
    assert colors.to_hex("r") == "#ca0b03"
"""


def _pytest(directory, *options):
    """The summary line of pytest run in *directory* under pltrs as
    matplotlib, with the ecosystem's plugins on the path."""
    path = [ALIAS, PACKAGE_DIR, ROOT / "python" / "ecosystem" / "plugin"]
    env = {**os.environ, "PYTHONPATH": os.pathsep.join(map(str, path)), "MPLBACKEND": "agg"}
    done = subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider",
         "-c", str(directory / "pytest.ini"), *options, str(directory)],
        env=env, cwd=directory, capture_output=True, text=True)
    return done.stdout.strip().splitlines()[-1]


def test_each_test_starts_with_the_colour_codes_of_the_first(tmp_path):
    (tmp_path / "pytest.ini").write_text("[pytest]\n")
    (tmp_path / "conftest.py").write_text(CONFTEST)
    (tmp_path / "test_codes.py").write_text(CODE_TESTS)
    assert _pytest(tmp_path).startswith("1 failed, 1 passed")
    assert _pytest(tmp_path, "-p", "ecosystem_color_codes").startswith("2 passed")


#: Tests recording the directory each worker runs them in.
WORKER_TESTS = """
import os, pathlib, pytest

@pytest.mark.parametrize("n", range(8))
def test_records_its_directory(n):
    worker = os.environ["PYTEST_XDIST_WORKER"]
    record = pathlib.Path(os.environ["RECORDS"]) / f"{worker}-{n}"
    record.write_text(os.getcwd())
"""


def test_each_worker_works_in_a_directory_of_its_own(tmp_path, monkeypatch):
    # plotnine's tests of `ggsave` save `filename-0.png`, ... in the
    # working directory, counting from 0 in each worker.
    pytest.importorskip("xdist")
    (tmp_path / "pytest.ini").write_text("[pytest]\n")
    (tmp_path / "test_workers.py").write_text(WORKER_TESTS)
    records, temporary = tmp_path / "records", tmp_path / "tmp"
    records.mkdir()
    temporary.mkdir()
    monkeypatch.setenv("RECORDS", str(records))
    monkeypatch.setenv("TMPDIR", str(temporary))
    assert _pytest(tmp_path, "-n", "2", "-p", "ecosystem_worker_dirs").startswith("8 passed")
    directories = {record.name.split("-")[0]: record.read_text()
                   for record in records.iterdir()}
    assert directories == {worker: str(temporary / f"working-dir-{worker}")
                           for worker in ("gw0", "gw1")}


def _diff_driver():
    """The differential tester's driver (`spec/diff/driver.py`)."""
    spec = importlib.util.spec_from_file_location("driver", ROOT / "spec" / "diff" / "driver.py")
    driver = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(driver)
    return driver


def test_the_gallery_and_generated_programs_run_offline(tmp_path):
    # The gallery's timeline asks GitHub for matplotlib's releases, with a
    # list of them to fall back on: offline, both implementations take it.
    driver = _diff_driver()
    setup = driver.Setup(argparse.Namespace(
        root=ROOT, out=tmp_path, gallery=tmp_path, pltrs_python=sys.executable,
        mpl_python=None))
    program = driver.Program("gallery/examples/lines_bars_and_markers/timeline",
                             tmp_path / "timeline.py", "program")
    fetch = ("import urllib.request\n"
             "try:\n"
             "    urllib.request.urlopen('https://api.github.com/repos/matplotlib/"
             "matplotlib/releases?per_page=100', timeout=1)\n"
             "except Exception as err:\n"
             "    print(type(err).__name__, getattr(err, 'reason', err))\n")
    done = subprocess.run([sys.executable, "-c", fetch], capture_output=True, text=True,
                          env=setup.environment("pltrs", program))
    assert done.stdout.startswith(f"URLError [Errno {errno.ECONNREFUSED}] Connection refused")


def _jupyter_config():
    """What the notebooks' Jupyter configuration sets, read in a stand-in
    of traitlets' configuration object."""
    path = ROOT / "spec" / "notebooks" / "jupyter_config.py"
    config = types.SimpleNamespace(KernelManager=types.SimpleNamespace())
    exec(compile(path.read_text(), str(path), "exec"), {"c": config})
    return vars(config.KernelManager)


@pytest.mark.skipif(sys.platform != "linux", reason="kernels keep TCP elsewhere")
def test_kernels_of_the_notebook_runs_take_sockets_no_other_process_can():
    first, second = _jupyter_config(), _jupyter_config()
    assert first["transport"] == second["transport"] == "ipc"
    assert first["ip"].startswith("@pltrs-kernel-")
    assert first["ip"] != second["ip"]
