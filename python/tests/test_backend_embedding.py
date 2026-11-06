"""
Figures embedded in Qt, Tk and wx applications (the programs of
``python/tests/embedding``: a canvas with its navigation toolbar, a button
that redraws, a blitting animation and a resize), and what embedding
needs of the canvases: a toolkit's canvas blits with its widget, and an
animation ending leaves the figure as it was drawn.

The expected values are real matplotlib 3.11's, which the programs report
identically on Windows 11 (Qt 6.12 through PyQt6, Tk 9.0, wxPython 4.3)
and on Linux (Tk under Xvfb, Qt offscreen); each program's test skips
where its toolkit or a display is missing.
"""

import hashlib
import importlib.util
import inspect
import json
import os
import pathlib
import subprocess
import sys
import textwrap

import numpy as np
import pytest

import pltrs
from pltrs import _c_internal_utils, cbook
from pltrs.backend_bases import FigureCanvasBase
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure

#: The embedding programs.
PROGRAMS = pathlib.Path(__file__).with_name("embedding")

#: matplotlib's toolbar items, per toolkit.
TOOLBARS = {
    "qt": ["Home", "Back", "Forward", "Pan", "Zoom", "Subplots", "Customize", "Save"],
    "tk": ["Home", "Back", "Forward", "Pan", "Zoom", "Subplots", "Save"],
    "wx": ["Home", "Back", "Forward", "Pan", "Zoom", "Subplots", "Save"],
}

#: The size each toolkit reports for the canvas widget: logical pixels
#: (Qt) or physical ones (Tk, wx).
WIDGET_SIZE = {"qt": "canvas", "tk": "physical", "wx": "physical"}

#: The Qt bindings `.qt_compat` can use.
QT_BINDINGS = ["PyQt6", "PySide6", "PyQt5", "PySide2"]


def _environment(toolkit):
    """The environment a program of *toolkit* runs in, or why it cannot
    run here."""
    env = {**os.environ, "PLTRS_LOOK": "matplotlib",
           "PYTHONPATH": os.pathsep.join(filter(None, [
               str(pathlib.Path(pltrs.__file__).parents[1]),
               os.environ.get("PYTHONPATH")]))}
    if toolkit == "qt":
        if not any(importlib.util.find_spec(binding) for binding in QT_BINDINGS):
            return None, "no Qt binding"
        if sys.platform == "linux":
            # Qt's xcb plugin needs libraries a display does not imply
            # (libxcb-cursor); its widgets paint the same offscreen.
            env.setdefault("QT_QPA_PLATFORM", "offscreen")
        return env, None
    module = "tkinter" if toolkit == "tk" else "wx"
    if importlib.util.find_spec(module) is None:
        return None, f"no {module}"
    if sys.platform == "linux" and not _c_internal_utils.xdisplay_is_valid():
        return None, "no X display"
    return env, None


def _run(toolkit):
    """What the embedding program of *toolkit* prints: its checkpoints and
    its facts."""
    env, reason = _environment(toolkit)
    if reason:
        pytest.skip(reason)
    proc = subprocess.run(
        [sys.executable, str(PROGRAMS / f"embed_{toolkit}.py"), "--hold", "0"],
        cwd=PROGRAMS, env=env, capture_output=True, text=True, timeout=120)
    assert proc.returncode == 0, proc.stderr
    lines = proc.stdout.splitlines()
    return lines[:-1], json.loads(lines[-1])


@pytest.mark.parametrize("toolkit", ["qt", "tk", "wx"])
def test_embedded_figure_follows_its_application(toolkit):
    checkpoints, facts = _run(toolkit)
    steps = ["shown", "redrawn", "animated", "resized"]
    assert checkpoints == [f"checkpoint {step}" for step in steps]
    assert facts["toolbar"] == TOOLBARS[toolkit]
    # The button's redraw, the animation's first draw and the resize each
    # draw the figure once more; the animation's frames are blitted.
    draws = [facts["steps"][step]["draws"] for step in steps]
    assert draws == list(range(draws[0], draws[0] + 4)) and draws[0] >= 1
    assert facts["draws"] == draws[-1]
    assert facts["redraws"] == 1 and facts["ydata_changed"]
    assert facts["frames"] == 14
    for step in steps:
        sizes = facts["steps"][step]
        ratio = sizes["device_pixel_ratio"]
        assert sizes["figure_pixels"] == sizes["physical"]
        assert sizes["canvas"] == [int(p / ratio) for p in sizes["physical"]]
        assert sizes["widget"] == sizes[WIDGET_SIZE[toolkit]]
        assert sizes["dpi"] == pytest.approx(100 * ratio)
    shown, resized = facts["steps"]["shown"], facts["steps"]["resized"]
    assert all(r > s for r, s in zip(resized["widget"], shown["widget"]))


def test_agg_canvas_leaves_blitting_to_the_toolkits():
    """matplotlib's Agg canvas has no ``blit`` of its own, so a toolkit's
    canvas (``FigureCanvasQTAgg(FigureCanvasAgg, FigureCanvasQT)``) blits
    with its widget's."""
    assert FigureCanvasAgg.blit is FigureCanvasBase.blit


def _qt_blit_repaints():
    from pltrs.backends.backend_qtagg import FigureCanvasQTAgg
    from pltrs.backends.qt_compat import QtWidgets
    from pltrs.figure import Figure
    from pltrs.transforms import Bbox

    app = QtWidgets.QApplication.instance() or QtWidgets.QApplication([])
    canvas = FigureCanvasQTAgg(Figure(figsize=(5, 3), dpi=100))
    calls = []
    canvas.repaint = lambda *args: calls.append(args)
    canvas.resize(500, 300)
    canvas.blit(Bbox.from_bounds(62.5, 33.0, 387.5, 231.0))
    canvas.blit()
    print(calls)
    del app


def _run_qt(body):
    """Run the function *body* in a fresh interpreter with Qt; its output."""
    env, reason = _environment("qt")
    if reason:
        pytest.skip(reason)
    source = textwrap.dedent(inspect.getsource(body)) + f"\n{body.__name__}()\n"
    proc = subprocess.run([sys.executable, "-c", source], env=env,
                          capture_output=True, text=True, timeout=60)
    assert proc.returncode == 0, proc.stderr
    return proc.stdout.strip()


def test_qt_blit_repaints_the_blitted_region():
    assert _run_qt(_qt_blit_repaints) == "[(62, 36, 387, 231), (0, 0, 500, 300)]"


def test_qt_customize_icon_is_matplotlibs():
    """Qt's toolbar draws ``<name>.svg`` where there is one: the Customize
    button's is matplotlib's own."""
    svg = cbook._get_data_path("images", "qt4_editor_options.svg")
    assert hashlib.sha256(svg.read_bytes()).hexdigest() == (
        "b1dacd231613f812ef277d004a76914393f1177481e35fa981d688253d0aa818")


def test_animation_ending_leaves_the_figure_as_drawn():
    """matplotlib's ``set_animated`` stales nothing: an animation's end,
    which makes its artists normal again, keeps its last blitted frame."""
    fig = Figure()
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    (line,) = ax.plot(np.arange(3.0))
    fig.canvas.draw()
    line.set_animated(True)
    assert not (line.stale or ax.stale or fig.stale)
    line.set_animated(False)
    assert not (line.stale or ax.stale or fig.stale)
    line.set_linewidth(3)
    assert line.stale and fig.stale
