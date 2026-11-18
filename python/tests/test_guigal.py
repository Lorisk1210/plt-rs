"""
The differential tester's programs that open a toolkit's window
(``spec/diff/gui.py``, ``gui.txt``, ``gui_shim.py``): the list, the
marking of a program's variants, and the shim ending a Tk program's main
loop once its window drew and went quiet, its figure saved, on an X server
of the run's own.
"""

import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import sys
import textwrap

import pytest

import pltrs

ROOT = pathlib.Path(__file__).resolve().parents[2]
#: The directory of the pltrs under test (the tree's, or an installed
#: wheel's), which the traced programs import beside the tree's alias.
PACKAGE_DIR = pathlib.Path(pltrs.__file__).resolve().parents[1]


def _load(name, file):
    """The module *file* of ``spec/diff``, loaded on its own as *name*."""
    spec = importlib.util.spec_from_file_location(name, ROOT / "spec/diff" / file)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gui():
    """``spec/diff/gui.py``, loaded on its own."""
    try:
        yield _load("diff_gui", "gui.py")
    finally:
        sys.modules.pop("diff_gui", None)


def test_listed_programs_keep_spaces_and_take_their_variables(gui, tmp_path):
    listed = tmp_path / "gui.txt"
    listed.write_text(textwrap.dedent("""\
        # a comment
        gallery/examples/user_interfaces/embedding_in_tk_sgskip

        gallery/examples/user_interfaces/mplcvd@qt MPLBACKEND=qtagg QT_API=pyqt6  # Qt
        corpus/book/Chapter 10 - Embedding Plots in GUI
        """))
    assert gui.listed(listed) == {
        "gallery/examples/user_interfaces/embedding_in_tk_sgskip": {},
        "gallery/examples/user_interfaces/mplcvd@qt": {"MPLBACKEND": "qtagg", "QT_API": "pyqt6"},
        "corpus/book/Chapter 10 - Embedding Plots in GUI": {},
    }
    assert gui.base_name("gallery/x/mplcvd@qt") == "gallery/x/mplcvd"
    assert gui.base_name("gallery/x/mplcvd") == "gallery/x/mplcvd"


def test_the_committed_list_names_existing_variants(gui):
    listed = gui.listed()
    assert all(not name.startswith(" ") for name in listed)
    variants = {name: variables for name, variables in listed.items() if "@" in name}
    assert {variables["MPLBACKEND"] for variables in variants.values()} == {
        "gtk3agg", "gtk4agg", "qtagg", "tkagg", "wxagg"}


def test_distributions_name_each_site_and_version(gui, tmp_path):
    site = tmp_path / "site-packages"
    (site / "colorspacious-1.1.2.dist-info").mkdir(parents=True)
    (site / "numpy-2.5.4.dist-info").mkdir()
    (site / "numpy").mkdir()
    assert gui.distributions([site]) == (b"site-packages/colorspacious-1.1.2.dist-info\n"
                                         b"site-packages/numpy-2.5.4.dist-info")


#: A Tk program: a canvas in a window of its own, and a Scale whose command
#: Tk runs from its first redraw (an idle task), before and after which
#: the window must have drawn for the loop to end.
TK_PROGRAM = """\
import tkinter as tk
from matplotlib.figure import Figure
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg

fig = Figure(figsize=(3, 2), dpi=50)
ax = fig.add_subplot()
ax.plot([0, 1], [0, 1])
root = tk.Tk()
canvas = FigureCanvasTkAgg(fig, master=root)
canvas.get_tk_widget().pack()

def update():
    print("update")
    ax.plot([0, 1], [1, 0])
    canvas.draw()

scale = tk.Scale(master=root, from_=0, to=5, orient=tk.HORIZONTAL, command=lambda v: update())
scale.set(3)
scale.pack()
tk.mainloop()
print("after the loop")
"""


def _trace(gui, program, out, mode="program"):
    """The record of *program* traced under pltrs with ``--gui`` on an X
    server of its own."""
    env = {k: v for k, v in os.environ.items() if not k.startswith(("MPL", "PLTRS"))}
    env |= {"MPLBACKEND": "agg", "PLTRS_LOOK": "matplotlib", "PYTHONHASHSEED": "0",
            "PYTHONPATH": os.pathsep.join([str(PACKAGE_DIR), str(ROOT / "python" / "alias")]),
            "MPLCONFIGDIR": str(out / "config")}
    try:
        with gui.display() as server:
            done = subprocess.run(
                [sys.executable, str(ROOT / "spec/diff/trace.py"), "--impl", "pltrs",
                 "--mode", mode, "--gui", str(program), str(out / "run")],
                env=env | server, capture_output=True, text=True, timeout=gui.TIMEOUT)
    except gui.Lacking as err:
        pytest.skip(str(err))
    assert done.returncode == 0, done.stderr[-2000:]
    return json.loads((out / "run" / "record.json").read_text())


needs_display = pytest.mark.skipif(
    shutil.which("Xvfb") is None or importlib.util.find_spec("tkinter") is None,
    reason="no Xvfb or no tkinter")


@needs_display
def test_a_tk_window_ends_its_loop_once_drawn_with_its_figure_saved(gui, tmp_path):
    program = tmp_path / "embedding.py"
    program.write_text(TK_PROGRAM)
    record = _trace(gui, program, tmp_path)
    assert record["uncaught"] is None
    # The Scale's command ran from the window's first redraw, then the loop
    # ended and the program went on.
    assert record["stdout"] == "update\nafter the loop\n"
    assert [figure["number"] for figure in record["figures"]] == ["window:1"]
    assert (tmp_path / "run" / "window-1.png").exists()


@needs_display
def test_each_loop_of_a_notebook_waits_for_its_own_window(gui, tmp_path):
    program = tmp_path / "cells.py"
    program.write_text("# %% [1]\n" + TK_PROGRAM + "\n# %% [2]\n" + TK_PROGRAM)
    record = _trace(gui, program, tmp_path, mode="cells")
    assert record["stdout"] == "update\nafter the loop\n" * 2
    assert [figure["number"] for figure in record["figures"]] == ["window:1", "window:2"]
