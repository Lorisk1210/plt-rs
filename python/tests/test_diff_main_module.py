"""
The differential tester runs a program as its own ``__main__`` module
(``spec/diff/trace.py``), as ``python program.py`` does: pickle finds
the classes and functions the program defines, and a process it spawns
imports ``__main__`` from the program's file. A notebook's is the
kernel's user module, which a spawned child does not run again.
"""

import json
import os
import pathlib
import subprocess
import sys

import pltrs

ROOT = pathlib.Path(__file__).resolve().parents[2]
#: The directory of the pltrs under test (the tree's, or an installed
#: wheel's), which the traced programs import beside the tree's alias.
PACKAGE_DIR = pathlib.Path(pltrs.__file__).resolve().parents[1]

PROGRAM = """\
import multiprocessing
import pickle
import sys
import matplotlib
matplotlib.use("agg")


class Point:
    def __init__(self, x):
        self.x = x


def square(v):
    return v * v


if __name__ == "__main__":
    print(pickle.loads(pickle.dumps(Point(3))).x, pickle.loads(pickle.dumps(square))(4))
    with multiprocessing.get_context("spawn").Pool(2) as pool:
        print(pool.map(square, [1, 2, 3]))
    print(sys.modules["__main__"].Point is Point, __file__.endswith("program.py"))
"""


NOTEBOOK = """\
# %%
import multiprocessing
import pathlib
import pickle
import matplotlib
matplotlib.use("agg")
with open(pathlib.Path(__file__).with_name("runs.txt"), "a") as runs:
    runs.write("ran ")
print("cell ran")


def square(v):
    return v * v


# %%
print(pickle.loads(pickle.dumps(square))(4))
child = multiprocessing.get_context("spawn").Process(target=square, args=(2,))
child.start()
child.join(120)
print(child.exitcode)
"""


def _trace(program, out, *mode):
    env = {k: v for k, v in os.environ.items() if not k.startswith(("MPL", "PLTRS"))}
    env |= {"MPLBACKEND": "agg", "PLTRS_LOOK": "matplotlib", "PYTHONHASHSEED": "0",
            "PYTHONPATH": os.pathsep.join([str(PACKAGE_DIR), str(ROOT / "python" / "alias")]),
            "MPLCONFIGDIR": str(out / "config")}
    done = subprocess.run([sys.executable, str(ROOT / "spec/diff/trace.py"), "--impl", "pltrs",
                           *mode, str(program), str(out / "run")], env=env, capture_output=True, text=True, timeout=300)
    assert done.returncode == 0, done.stderr[-2000:]
    return json.loads((out / "run" / "record.json").read_text())


def test_a_program_pickles_and_spawns_with_what_it_defines(tmp_path):
    program = tmp_path / "program.py"
    program.write_text(PROGRAM)
    record = _trace(program, tmp_path)
    assert record["uncaught"] is None
    assert record["stdout"].splitlines() == ["3 16", "[1, 4, 9]", "True True"]


def test_a_notebooks_spawned_child_does_not_run_it_again(tmp_path):
    # As in Jupyter's kernel: the child finds no function of the notebook
    # by reference (it fails, where one that ran the notebook again would
    # have found it), and the notebook's cells ran once.
    program = tmp_path / "notebook.py"
    program.write_text(NOTEBOOK)
    record = _trace(program, tmp_path, "--mode", "cells")
    assert record["stdout"].splitlines() == ["cell ran", "16", "1"]
    assert (tmp_path / "runs.txt").read_text() == "ran "
