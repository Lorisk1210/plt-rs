"""
Smoke tests of what Jupyter's backends need of pltrs, found by running
notebooks under matplotlib and pltrs (`cargo xtask notebooks`): pyplot's
connection to IPython's ``post_execute`` event, the observers of a
figure's Axes (the toolbars'), the staleness a lone ``draw_artist``
leaves, the resolution vector canvases leave a figure drawn at, the
toolbar's draw time a print sets, matplotlib's messages for malformed
mathtext and rcParams, and the 3-D title before its first draw. The
expected values are real matplotlib 3.11's for the same code.
"""

import io
import sys
import types

import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.backend_bases import NavigationToolbar2
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.backends.backend_pdf import FigureCanvasPdf
from pltrs.backends.backend_svg import FigureCanvasSVG
from pltrs.figure import Figure


@pytest.fixture
def ipython(monkeypatch):
    """A stand-in IPython whose shell records the events registered."""
    registered = []

    class Events:
        def register(self, name, func):
            registered.append((name, func.__name__))

        def unregister(self, name, func):
            registered.remove((name, func.__name__))

    class Shell:
        events = Events()

        def enable_gui(self, gui):
            registered.append(("gui", gui))

    module = types.ModuleType("IPython")
    module.get_ipython = Shell
    module.version_info = (9, 18, 1)
    monkeypatch.setitem(sys.modules, "IPython", module)
    yield registered
    plt.ioff()


def test_pyplot_redraws_after_each_ipython_execution(ipython):
    plt.switch_backend("agg")
    assert ipython == [("post_execute", "_draw_all_if_interactive")]
    plt.ioff()
    assert ipython == []
    plt.ion()
    assert ipython == [("post_execute", "_draw_all_if_interactive")]


def test_axes_observers_hear_of_every_new_axes():
    fig = plt.figure()
    calls = []
    fig.add_axobserver(lambda f: calls.append(len(f.axes)))
    fig.add_subplot()
    fig.add_axes((0.1, 0.1, 0.2, 0.2))
    fig.subplots(1, 2)
    fig.add_axes(fig.axes[0])
    assert calls == [1, 2, 3, 4, 4]
    plt.close(fig)


def test_the_managers_toolbar_resets_its_views_for_new_axes():
    fig = plt.figure()
    updates = []

    class Toolbar(NavigationToolbar2):
        def update(self):
            updates.append(1)
            super().update()

        def set_message(self, s):
            pass

    fig.canvas.manager.toolbar = Toolbar(fig.canvas)
    fig.add_subplot()
    assert len(updates) == 1
    plt.close(fig)


def test_a_scatter_drawn_alone_leaves_its_axes_and_figure_stale():
    fig, ax = plt.subplots()
    scatter = ax.scatter([1, 2], [1, 2])
    line, = ax.plot([1, 2])
    fig.canvas.draw()
    assert (fig.stale, ax.stale, scatter.stale) == (False, False, False)
    ax.draw_artist(scatter)
    assert (fig.stale, ax.stale, scatter.stale) == (True, True, False)
    fig.canvas.draw()
    ax.draw_artist(line)
    assert (fig.stale, ax.stale, line.stale) == (False, False, False)
    plt.close(fig)


@pytest.mark.parametrize("canvas, dpi", [
    (FigureCanvasSVG, 72), (FigureCanvasPdf, 72), (FigureCanvasAgg, 100)])
def test_vector_canvases_leave_their_figure_at_their_resolution(canvas, dpi):
    fig = Figure(dpi=100)
    fig.add_subplot().plot([1, 2])
    canvas(fig)
    fig.canvas.draw()
    assert fig.dpi == dpi


def test_prints_time_the_toolbars_wait_cursor():
    fig = Figure()
    FigureCanvasAgg(fig)
    fig.add_subplot()
    toolbar = NavigationToolbar2(fig.canvas)
    assert not hasattr(toolbar, "_draw_time")
    fig.savefig(io.BytesIO(), format="png")
    assert hasattr(toolbar, "_draw_time")


def test_malformed_mathtext_fails_a_draw_with_the_parsers_message():
    fig, ax = plt.subplots()
    ax.set_title(r"$\sqrt{$")
    with pytest.raises(ValueError) as info:
        fig.canvas.draw()
    assert str(info.value) == (
        "\n\\sqrt{\n     ^\nParseSyntaxException: Expected \\sqrt{value}, "
        "found '{'  (at char 5), (line:1, col:6)")
    plt.close(fig)


@pytest.mark.parametrize("key, value, message", [
    ("lines.linewidth", "thick",
     "Key lines.linewidth: Could not convert 'thick' to float"),
    ("figure.max_open_warning", "many",
     "Key figure.max_open_warning: Could not convert 'many' to int"),
    ("axes.grid", "maybe", "Key axes.grid: Cannot convert 'maybe' to bool"),
])
def test_rcparams_refuse_values_with_matplotlibs_messages(key, value, message):
    with pytest.raises(ValueError) as info:
        pltrs.rcParams[key] = value
    assert str(info.value) == message


def test_3d_titles_sit_lower_until_drawn():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    title = ax.set_title("x")
    assert title.get_position() == (0.5, 0.92)
    assert repr(title) == "Text(0.5, 0.92, 'x')"
    fig.canvas.draw()
    assert ax.title.get_position() == (0.5, 1.0)
    plt.close(fig)


def test_jupyter_kernels_run_the_windows_between_messages():
    eventloops = pytest.importorskip("ipykernel.eventloops")
    from pltrs.backends import _pltrs_loop
    _pltrs_loop._register_ipython_hook()
    assert eventloops.loop_map[_pltrs_loop.FRAMEWORK] is not None
