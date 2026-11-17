"""
What the corpora's last differences (task CORP14) showed pltrs doing
otherwise than matplotlib 3.11, reduced to matplotlib's API. Every
expected value was recorded from matplotlib.
"""

import io
import logging
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager
from pltrs.offsetbox import AnchoredText


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def logged(work, family):
    """The font manager's messages about *family* that *work* logs, by
    kind: per lookup (``"not found."``) and per cached lookup that falls
    back to the default font (``"Falling back"``)."""
    messages = []

    class Handler(logging.Handler):
        def emit(self, record):
            if family in record.getMessage():
                messages.append(record.getMessage())

    handler = Handler()
    font_manager._log.addHandler(handler)
    try:
        work()
    finally:
        font_manager._log.removeHandler(handler)
    return (sum(message.endswith("not found.") for message in messages),
            sum("Falling back" in message for message in messages))


def test_each_renderer_parses_a_formula_and_looks_its_fonts_up():
    # numerical-methods-hawke's xkcd lectures: the formulas of a figure
    # drawn in an rc_context are parsed again, outside it, by the renderer
    # of the cut size a tight print draws with.
    def work():
        with plt.rc_context({"font.family": "Corp14MissingMath"}):
            fig, ax = plt.subplots()
            ax.text(0.5, 0.5, r"$x_0$")
            fig.canvas.draw()
        fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")

    assert logged(work, "Corp14MissingMath")[1] == 2


def test_a_formulas_lookups_are_cached_by_its_own_math_font_family():
    # pygtc sets mathtext.fontset for each plot and restores it after: a
    # formula parsed again later looks its fonts up with the family it
    # was made with, which the font manager found (and warned about) once.
    def work():
        with plt.rc_context({"mathtext.fontset": "stixsans"}):
            fig, ax = plt.subplots()
            ax.text(0.5, 0.5, r"$\theta$", family="Corp14MissingFontset")
            fig.canvas.draw()
        fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")

    assert logged(work, "Corp14MissingFontset")[1] == 1


@pytest.mark.parametrize("fmt", ["png", "svg", "pdf"])
@pytest.mark.parametrize("box, expected", [("legend", 18), ("anchored", 12), ("text", 6)])
def test_a_tight_print_measures_its_box_with_its_first_draws_renderer(box, expected, fmt):
    # pygtc's legend saved to a PDF cut to its tight box.
    family = f"Corp14Missing{box}{fmt}"

    def work():
        fig = plt.figure()
        ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])
        ax.set_axis_off()
        prop = {"family": family, "size": 9}
        if box == "legend":
            ax.legend([ax.plot([1, 2])[0]], ["a"], prop=prop)
        elif box == "anchored":
            ax.add_artist(AnchoredText("a", loc=1, prop=prop))
        else:
            ax.text(0.5, 0.5, "a", **prop)
        fig.savefig(io.BytesIO(), format=fmt, bbox_inches="tight")

    assert logged(work, family)[0] == expected


def test_a_failed_plotting_call_keeps_the_units_it_registered():
    # ml-with-python's profiling notebook: bars of categories without
    # heights fail on their shapes after the x axis took the categories.
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="shape mismatch"):
        ax.bar(["5-million", "10-million"], [])
    assert ax.xaxis.get_units() is not None
    assert np.allclose(ax.get_xlim(), (-0.055, 0.055))
    with pytest.raises(ValueError, match="shape mismatch"):
        fig.add_subplot(212).bar([1, 2], [])
    assert fig.axes[1].get_xlim() == (0.0, 1.0)


@pytest.mark.parametrize("dpi, clipped", [(100, True), (150, False)])
def test_an_artist_clipped_to_its_axes_compares_the_boxes_as_matplotlib(dpi, clipped):
    # qutip's circuits: at 150 dpi the axes' patch rounds the top of the
    # box (633.6000000000001), so an added text joins the tight box.
    fig, ax = plt.subplots(dpi=dpi)
    text = ax.add_artist(plt.Text(1, 1, "H"))
    assert bool(text._fully_clipped_to_axes()) is clipped
    assert (text in ax.get_default_bbox_extra_artists()) is not clipped


def test_a_text_the_tight_layout_measures_at_150_dpi_looks_its_font_up():
    family = "Corp14MissingQutip"

    def work():
        fig = plt.figure()
        ax = fig.add_subplot(111)
        fig.set_dpi(150)
        ax.add_artist(plt.Text(1, 1, "H", fontfamily=family))
        fig.tight_layout()

    assert logged(work, family)[0] == 2


@pytest.mark.parametrize("mode, zlim, autoscale", [
    ("tight", [-0.02083, 1.02083], False), ("auto", [-0.02083, 1.02083], True),
    ("image", [-0.02083, 1.02083], False), ("scaled", [-0.02083, 1.02083], False),
    ("square", [-0.02083, 1.02083], False), ("equal", [-0.14453, 1.14453], True)])
def test_the_modes_of_axis_autoscale_a_3d_axes_along_z(mode, zlim, autoscale):
    # qutip's process tomography makes its bars `ax.axis("tight")`.
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.bar3d([0, 1], [0, 1], [0, 0], [0.5, 0.5], [0.5, 0.5], [1.0, 0.5])
    ax.set_zlim3d([-0.0117, 1.0])
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        limits = ax.axis(mode)
    assert np.round(limits, 5).tolist()[4:] == zlim
    assert np.round(ax.get_zlim(), 5).tolist() == zlim
    assert ax.get_autoscalez_on() is autoscale and ax.get_autoscalex_on() is autoscale


@pytest.fixture
def trace():
    """The differential tester's ``spec/diff/trace.py``, loaded on its own
    (its directory, which it puts on ``sys.path``, taken off again)."""
    import importlib.util
    import pathlib
    import sys
    here = pathlib.Path(__file__).resolve().parents[2] / "spec" / "diff"
    path, modules = list(sys.path), set(sys.modules)
    spec = importlib.util.spec_from_file_location("diff_trace", here / "trace.py")
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
        yield module
    finally:
        sys.path[:] = path
        for name in set(sys.modules) - modules:
            del sys.modules[name]


def test_the_tracers_hooks_carry_no_recorder_to_other_processes(trace):
    # iminuit's notebooks hand the program's functions to joblib's
    # workers, which pickle them with the globals they read.
    hooks = (trace._record, trace._statement, trace._statement_error)
    read = {name for hook in hooks for name in hook.__code__.co_names}
    read |= set(trace._recorder.__code__.co_names)
    assert not any(isinstance(vars(trace).get(name), (trace.Recorder, type)) for name in read)
    # Where there is no recorder (another process), they record nothing.
    assert trace._record("1:0", 5) == 5
    assert trace._statement(1, 1) is None
    assert trace._statement_error(ValueError()) is None
