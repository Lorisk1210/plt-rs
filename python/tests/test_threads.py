"""
pltrs objects used from any thread, as matplotlib's are: figures built,
drawn and saved in worker threads and thread pools, objects outliving the
thread that made them, the global state (rcParams, styles, warnings)
shared by every thread. The expected values are real matplotlib 3.11's
for the same programs.
"""

from concurrent.futures import ThreadPoolExecutor
import io
import os
import random
import sys
import threading
import warnings

import numpy as np
import pytest

import pltrs
from pltrs import ticker
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield


def _in_thread(function, *args):
    """``function(*args)`` run in a new thread, which has ended when this
    returns; what it returned, or the exception it raised, raised here."""
    outcome = []

    def run():
        try:
            outcome.append((True, function(*args)))
        except BaseException as err:
            outcome.append((False, err))

    thread = threading.Thread(target=run)
    thread.start()
    thread.join()
    ok, value = outcome[0]
    if not ok:
        raise value
    return value


def _png(n, formatter=False):
    """The PNG of a small figure, made, drawn and saved where it is
    called: the figure is the *n*-th of a family."""
    fig = Figure(figsize=(2, 1.5), dpi=40)
    ax = fig.subplots()
    x = np.linspace(0, 1, n + 3)
    ax.plot(x, np.sin(6 * x + n), label=f"line {n}")
    ax.set_title(f"figure {n}")
    ax.legend()
    if formatter:
        ax.xaxis.set_major_formatter(ticker.FuncFormatter(
            lambda value, pos: f"<{value:.1f}>"))
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png")
    return buffer.getvalue()


def test_figures_built_drawn_and_saved_in_a_worker():
    assert _in_thread(_png, 3) == _png(3)
    assert _in_thread(_png, 4, True) == _png(4, True)


def test_a_worker_draws_while_the_main_thread_works():
    done = threading.Event()
    rendered = []

    def worker():
        # The main thread draws until the worker ends, as it ends: a
        # failing worker fails the test rather than leaving it drawing.
        try:
            rendered.extend(_png(n, n % 2 == 0) for n in range(6))
        except BaseException as err:
            rendered.append(err)
        finally:
            done.set()

    thread = threading.Thread(target=worker)
    thread.start()
    own = []
    while not done.is_set():
        own.append(_png(len(own) % 6))
    thread.join()
    assert rendered == [_png(n, n % 2 == 0) for n in range(6)]
    assert own == [_png(n % 6) for n in range(len(own))]


def test_thread_pools_render_the_bytes_of_one_thread():
    jobs = [(n, n % 3 == 0) for n in range(16)]
    alone = [_png(*job) for job in jobs]
    with ThreadPoolExecutor(8) as pool:
        pooled = list(pool.map(lambda job: _png(*job), jobs))
    assert pooled == alone


def test_objects_outlive_the_thread_that_made_them():
    def make():
        fig = Figure(figsize=(2, 2), dpi=40)
        FigureCanvasAgg(fig)
        ax = fig.subplots()
        (line,) = ax.plot([0, 1, 2], [1, 0, 1])
        return fig, ax, line

    fig, ax, line = _in_thread(make)
    line.set_ydata([2, 3, 4])
    ax.relim()
    ax.autoscale_view()
    assert ax.lines[0] is line and line.axes is ax and ax.figure is fig
    assert ax.get_ylim() == pytest.approx((1.9, 4.1))
    fig.canvas.draw()
    assert fig.canvas.buffer_rgba().shape == (80, 80, 4)
    # Back to another thread again, after the first ended.
    assert _in_thread(lambda: list(line.get_ydata())) == [2, 3, 4]


def test_rc_context_in_a_thread_is_every_threads():
    entered = threading.Event()
    release = threading.Event()
    inside = []

    def worker():
        with pltrs.rc_context({"lines.linewidth": 7, "axes.titlesize": 20}):
            inside.append(pltrs.rcParams["lines.linewidth"])
            entered.set()
            release.wait()

    thread = threading.Thread(target=worker)
    thread.start()
    entered.wait()
    try:
        seen = pltrs.rcParams["lines.linewidth"]
        fig = Figure()
        (line,) = fig.subplots().plot([0, 1])
        title = fig.axes[0].set_title("t").get_fontsize()
        # Changed while the worker's context is open, and restored with it
        # (rc_context restores every parameter, as matplotlib's does).
        pltrs.rcParams["lines.markersize"] = 9
    finally:
        release.set()
        thread.join()
    assert (inside, seen, line.get_linewidth(), title) == ([7.0], 7.0, 7.0,
                                                           20.0)
    assert (pltrs.rcParams["lines.linewidth"], pltrs.rcParams[
        "lines.markersize"], pltrs.rcParams["axes.titlesize"]) == (
            1.5, 6.0, "large")


def test_rcparams_and_styles_set_in_a_worker_are_global():
    _in_thread(pltrs.rcParams.update, {"lines.color": "red"})
    assert pltrs.rcParams["lines.color"] == "red"
    _in_thread(pltrs.style.use, "ggplot")
    assert pltrs.rcParams["axes.facecolor"] == "#E5E5E5"


def test_warnings_of_a_worker_reach_its_filters():
    def warn():
        with warnings.catch_warnings(record=True) as record:
            warnings.simplefilter("always")
            fig = Figure()
            FigureCanvasAgg(fig)
            fig.add_axes([0.1, 0.1, 0.5, 0.5])
            fig.tight_layout()
            fig.text(0.5, 0.5, "\N{UPSIDE-DOWN FACE}")
            fig.canvas.draw()
        return [str(warning.message) for warning in record]

    assert _in_thread(warn) == [
        "This figure includes Axes that are not compatible with "
        "tight_layout, so results might be incorrect.",
        "Glyph 128579 (\\N{UPSIDE-DOWN FACE}) missing from font(s) "
        "DejaVu Sans.",
    ]


def test_a_figure_another_thread_draws_is_changed_after_its_draw():
    fig = Figure(figsize=(2, 2), dpi=30)
    FigureCanvasAgg(fig)
    ax = fig.subplots()
    ax.plot([0, 1, 2], [0, 1, 0])
    drawing = threading.Event()
    changed = []

    def label(value, pos):
        # The draw lets the worker run in the middle of it.
        drawing.set()
        threading.Event().wait(0.01)
        return f"{value:.1f}"

    def change():
        drawing.wait()
        ax.set_title("changed")
        ax.plot([1, 2])
        changed.append(len(ax.lines))

    ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
    thread = threading.Thread(target=change)
    thread.start()
    fig.canvas.draw()
    thread.join()
    assert changed == [2] and ax.get_title() == "changed"


def test_threads_using_pltrs_at_once():
    """Seeded rounds of figures, rc contexts, registrations and a figure
    every thread draws, in threads switching as often as Python lets them
    (``PLTRS_FUZZ_SCALE`` runs more rounds): none fails."""
    rounds = 15 * int(os.environ.get("PLTRS_FUZZ_SCALE", "1"))
    shared = Figure(figsize=(2, 2), dpi=30)
    FigureCanvasAgg(shared)
    shared_ax = shared.subplots()
    shared_ax.plot([0, 1, 2], [2, 0, 1])
    shared_ax.xaxis.set_major_formatter(ticker.FuncFormatter(
        lambda value, pos: f"{value:.2f}"))
    errors = []

    def figure(rng):
        _png(rng.randrange(20), rng.random() < 0.5)

    def rc(rng):
        with pltrs.rc_context({"lines.linewidth": rng.choice([1, 2, 3])}):
            _png(rng.randrange(5))

    def register(rng):
        cmap = pltrs.colormaps["viridis"].resampled(rng.randrange(2, 9))
        name = f"_test_threads_{rng.randrange(10 ** 9)}"
        pltrs.colormaps.register(cmap, name=name)
        pltrs.colormaps.unregister(name)

    def shared_figure(rng):
        shared_ax.set_xlim(0, 1 + rng.random())
        shared_ax.lines[0].set_ydata([rng.random() for _ in range(3)])
        shared.canvas.draw()

    def worker(seed):
        rng = random.Random(seed)
        try:
            for _ in range(rounds):
                rng.choice([figure, rc, register, shared_figure])(rng)
        except Exception as err:
            errors.append(err)

    interval = sys.getswitchinterval()
    sys.setswitchinterval(1e-6)
    try:
        threads = [threading.Thread(target=worker, args=(seed,))
                   for seed in range(6)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
    finally:
        sys.setswitchinterval(interval)
    assert errors == []
