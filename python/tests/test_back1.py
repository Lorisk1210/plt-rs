"""
What task BACK1 found pltrs doing otherwise than matplotlib 3.11: stale
callbacks a program sets, frames grabbed while an animation saves, what a
failed plotting call leaves, the loggers matplotlib makes, the data
directory's files, and what the vector backends write and leave on the
figure. Each expected value is recorded from matplotlib.
"""

import datetime
import io
import logging
import os
import re
import shutil
import time

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import animation
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.backends.backend_pdf import FigureCanvasPdf
from pltrs.backends.backend_ps import FigureCanvasPS
from pltrs.backends.backend_svg import FigureCanvasSVG
from pltrs.figure import Figure


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def drawn_scene():
    """A figure without pyplot, drawn, with a line, whose figure records
    the calls of its stale callback."""
    fig = Figure(figsize=(3, 2))
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    line, = ax.plot([0, 1], [0, 1])
    fig.canvas.draw()
    calls = []
    fig.stale_callback = lambda artist, val: calls.append((artist is fig, val))
    return fig, ax, line, calls


@pytest.mark.parametrize("change, count", [
    (lambda fig, ax, line: line.set_color("r"), 1),
    (lambda fig, ax, line: line.set_linewidth(3), 1),
    (lambda fig, ax, line: line.set_marker("o"), 1),
    (lambda fig, ax, line: line.set_label("x"), 1),
    (lambda fig, ax, line: line.set_zorder(5), 1),
    (lambda fig, ax, line: line.set_visible(False), 1),
    (lambda fig, ax, line: line.set_data([0, 1], [1, 0]), 2),
    (lambda fig, ax, line: line.set_xdata([0, 2]), 1),
    (lambda fig, ax, line: ax.title.set_fontsize(9), 1),
    (lambda fig, ax, line: ax.text(0, 0, "t").set_x(0.5), 1),
    (lambda fig, ax, line: fig.set_facecolor("r"), 1),
])
def test_program_stale_callbacks_see_native_changes(change, count):
    fig, ax, line, calls = drawn_scene()
    change(fig, ax, line)
    assert calls[-count:] == [(True, True)] * count
    assert fig.stale


@pytest.mark.parametrize("change", [
    lambda fig, ax, line: ax.set_xscale("log"),
    lambda fig, ax, line: ax.set_xticks([0, 0.5]),
    lambda fig, ax, line: ax.invert_xaxis(),
    lambda fig, ax, line: ax.set_xbound(0, 2),
    lambda fig, ax, line: ax.minorticks_on(),
    lambda fig, ax, line: ax.locator_params(nbins=3),
    lambda fig, ax, line: fig.set_size_inches(4, 3),
    lambda fig, ax, line: fig.subplots_adjust(left=0.3),
    lambda fig, ax, line: fig.tight_layout(),
    lambda fig, ax, line: ax.xaxis.set_alpha(0.5),
    lambda fig, ax, line: ax.pcolormesh(np.eye(2)).set_antialiased(True),
])
def test_every_change_that_stales_the_figure_reaches_its_callback(change):
    fig, ax, line, calls = drawn_scene()
    change(fig, ax, line)
    # matplotlib calls it once or more (set_xscale: 7 times).
    assert calls and all(call == (True, True) for call in calls)
    assert fig.stale


@pytest.mark.parametrize("change", [
    lambda line: line.set_gid("g"),
    lambda line: line.set_url("u"),
    lambda line: line.set_rasterized(True),
    lambda line: line.set_in_layout(False),
])
def test_setters_matplotlib_leaves_fresh(change):
    fig, ax, line, calls = drawn_scene()
    change(line)
    assert calls == []
    assert not fig.stale and not line.stale


def test_mirrors_of_parts_without_callbacks_report_nothing():
    fig, ax, line, calls = drawn_scene()
    text = ax.text(0.5, 0.5, "t")
    fig.canvas.draw()
    calls.clear()
    text.set_fontsize(9)  # Through the text's FontProperties mirror.
    assert calls == [(True, True)]


def test_axes_callback_of_the_program_runs_before_the_figures():
    fig, ax, line, calls = drawn_scene()
    order = []
    ax.stale_callback = lambda artist, val: (order.append("axes"), fig.__setattr__("stale", val))
    fig.stale_callback = lambda artist, val: order.append("figure")
    line.set_color("r")
    assert order == ["axes", "figure"]


class Grab(animation.AbstractMovieWriter):
    """A writer keeping the canvas' pixels of each frame."""

    def setup(self, fig, outfile, dpi=None):
        super().setup(fig, outfile, dpi=dpi)
        self.frames = []

    def grab_frame(self, **savefig_kwargs):
        self.fig.canvas.draw()
        self.frames.append(np.asarray(self.fig.canvas.buffer_rgba()).copy())

    def finish(self):
        pass


def test_frames_grabbed_while_a_blitted_animation_saves_show_it(tmp_path):
    fig, ax = plt.subplots(figsize=(2, 2), dpi=20)
    line, = ax.plot([0, 1], [0, 0])
    ax.set_ylim(-1, 1)
    anim = animation.FuncAnimation(
        fig, lambda i: (line.set_ydata([0, i / 3 - 0.5]), line)[1:], frames=3, blit=True)
    writer = Grab(fps=1)
    anim.save(tmp_path / "x.mem", writer=writer)
    assert [bool((writer.frames[k] != writer.frames[k + 1]).any()) for k in range(2)] == [True, True]


@pytest.mark.parametrize("canvas, method", [
    (FigureCanvasPS, "print_ps"), (FigureCanvasPS, "print_eps"),
    (FigureCanvasPdf, "print_pdf"), (FigureCanvasSVG, "print_svg"),
])
def test_vector_print_methods_leave_the_dpi_at_72(canvas, method):
    fig = Figure(figsize=(3, 2), dpi=120)
    printer = canvas(fig)
    fig.add_subplot().plot([0, 1])
    getattr(printer, method)(io.BytesIO())
    assert fig.dpi == 72
    assert fig.dpi_scale_trans.get_matrix()[0, 0] == 72


@pytest.mark.parametrize("fmt", ["ps", "pdf", "svg"])
def test_savefig_puts_the_dpi_back(fmt):
    fig = Figure(figsize=(3, 2), dpi=120)
    FigureCanvasPS(fig)
    fig.add_subplot().plot([0, 1])
    fig.savefig(io.BytesIO(), format=fmt)
    assert fig.dpi == 120


def pdf_info(fig, metadata):
    buf = io.BytesIO()
    fig.savefig(buf, format="pdf", metadata=metadata)
    return buf.getvalue()


def test_pdf_dates_are_written_as_matplotlib_writes_them():
    fig = Figure(figsize=(2, 2))
    tz = datetime.timezone(datetime.timedelta(hours=-5, minutes=-30))
    cases = [
        (datetime.datetime(2024, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc),
         b"D:20240102030405Z"),
        # matplotlib's formula of the offset (`timedelta.seconds` of a
        # negative offset, seconds after the hours).
        (datetime.datetime(2024, 1, 2, 3, 4, 5, tzinfo=tz), b"D:20240102030405-18'1800'"),
    ]
    for date, written in cases:
        data = pdf_info(fig, {"CreationDate": date, "ModDate": date})
        assert re.findall(rb"/CreationDate \(([^)]*)\)", data) == [written]
        assert re.findall(rb"/ModDate \(([^)]*)\)", data) == [written]
    # A naive date is local time with the zone's offset now.
    naive = datetime.datetime(2024, 1, 2, 3, 4, 5)
    z = time.altzone if time.daylight else time.timezone
    offset = ("Z" if z == 0 else "+%02d'%02d'" % (-z // 3600, -z % 3600) if z < 0
              else "-%02d'%02d'" % (z // 3600, z % 3600))
    data = pdf_info(fig, {"CreationDate": naive})
    assert re.findall(rb"/CreationDate \(([^)]*)\)", data) == [
        f"D:20240102030405{offset}".encode()]


def test_pdf_information_keeps_every_key_given():
    fig = Figure(figsize=(2, 2))
    data = pdf_info(fig, {"CreationDate": None, "Trapped": "True"})
    assert b"/Trapped /True" in data
    with pytest.warns(UserWarning, match="Bad value for infodict keyword Title. Got 5"):
        data = pdf_info(fig, {"CreationDate": None, "Title": 5})
    assert b"/Title 5" in data
    with pytest.warns(UserWarning, match="Unknown infodict keyword: 'Colour'"):
        data = pdf_info(fig, {"CreationDate": None, "Colour": "red"})
    assert b"/Colour (red)" in data
    with pytest.raises(AttributeError, match="'bool' object has no attribute 'translate'"):
        pdf_info(fig, {"Trapped": True})


def test_pdf_information_left_out_is_left_out():
    fig = Figure(figsize=(2, 2))
    data = pdf_info(fig, {"Creator": None, "Producer": None, "CreationDate": None})
    assert re.findall(rb"/(Creator|Producer|CreationDate)", data) == []
    data = pdf_info(fig, {"Creator": None, "Title": "T"})
    assert re.findall(rb"/(Creator|Producer|CreationDate|Title) ", data) == [
        b"Producer", b"CreationDate", b"Title"]


@pytest.mark.skipif(shutil.which("xelatex") is not None, reason="xelatex is installed")
def test_pgf_without_its_tex_says_so():
    fig = Figure(figsize=(2, 2))
    fig.text(0.5, 0.5, "x")
    with pytest.raises(RuntimeError, match=r"^'xelatex' not found; install it or change "
                       r"rcParams\['pgf.texsystem'\] to an available TeX implementation$"):
        fig.savefig(io.BytesIO(), format="pgf")


@pytest.mark.parametrize("call, message", [
    (lambda ax: ax.plot([1], ls="~~"), "'~~' is not a valid value for ls."),
    (lambda ax: ax.plot([1], ls="dashy"),
     "'dashy' is not a valid value for ls. Did you mean one of: 'dashed', 'dashdot'?"),
    (lambda ax: ax.plot([1], ls=5), "Unrecognized linestyle: 5"),
    (lambda ax: ax.grid(ls="~~"), "'~~' is not a valid value for ls."),
    (lambda ax: ax.axhline(ls="~~"), "'~~' is not a valid value for ls."),
])
def test_line_styles_are_refused_as_lines_refuse_them(call, message):
    fig, ax = plt.subplots()
    with pytest.raises(ValueError) as raised:
        call(ax)
    assert str(raised.value) == message


def test_a_negative_dash_fails_where_the_line_is_drawn():
    fig, ax = plt.subplots()
    ax.plot([1, 2], ls=(0, (1, -1)))
    assert len(ax.lines) == 1
    with pytest.raises(ValueError, match="All values in the dash list must be non-negative"):
        fig.savefig(io.BytesIO(), format="png")


def test_what_failed_plotting_calls_leave():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match=r"Invalid shape \(3,\) for image data"):
        ax.imshow(np.arange(3))
    assert ax.get_aspect() == 1.0 and not ax.images
    with pytest.raises(ValueError, match="'yerr' must not contain negative values"):
        ax.errorbar([1], [1], yerr=[-1])
    assert len(ax.lines) == 1 and not ax.containers and not ax.collections
    assert ax.get_xlim() == (0.0, 1.0)


def test_the_loggers_of_the_figure_modules_exist():
    names = set(logging.root.manager.loggerDict)
    prefix = pltrs.figure.__name__.rpartition(".")[0]
    for module in ("_constrained_layout", "_layoutgrid", "axes", "axes._axes", "axes._base",
                   "axis", "dates", "gridspec", "texmanager", "text", "textpath"):
        assert f"{prefix}.{module}" in names


def test_the_data_directory_holds_matplotlibs_files():
    path = pltrs.get_data_path()
    for name in ("matplotlibrc", "kpsewhich.lua", "sample_data", "images", "stylelib"):
        assert os.path.exists(os.path.join(path, name)), name
    with open(os.path.join(path, "matplotlibrc"), encoding="utf-8") as rc:
        assert "#axes.titlesize:     large" in rc.read()
