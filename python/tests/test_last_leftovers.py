"""
The last differences ``cargo xtask diff --corpus`` (the corpora of
programs users wrote) and ``cargo xtask diff --docs`` (matplotlib's
signatures, docstrings and names) found. Every expected value was recorded
from matplotlib 3.11.
"""

import datetime
import inspect
import io
import logging
import threading
import types

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.collections import PathCollection
from pltrs.container import Container
from pltrs.path import Path


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Messages(logging.Handler):
    """Keeps the messages logged meanwhile."""

    def __init__(self):
        super().__init__(logging.WARNING)
        self.messages = []

    def emit(self, record):
        self.messages.append(record.getMessage())


@pytest.fixture
def logged():
    """The warnings logged during the test."""
    handler = _Messages()
    logger = logging.getLogger("pltrs")
    logger.addHandler(handler)
    yield handler.messages
    logger.removeHandler(handler)


# --- The corpora -------------------------------------------------------------

def test_polar_labels_are_padded_as_matplotlib_computes_the_padding():
    # Rougier's transforms-polar: the pad joins the label transform's affine
    # part, in inches through the figure's dpi_scale_trans, so that the
    # extents and tight_layout's parameters are matplotlib's to the last bit.
    fig = plt.figure(figsize=(5, 5), dpi=100)
    ax = fig.add_subplot(projection="polar")
    ax.set_ylim(-1, 1)
    ax.set_yticks([-1, -0.5, 0, 0.5, 1])
    fig.draw_without_rendering()
    renderer = fig.canvas.get_renderer()
    ticks = ax.xaxis.get_major_ticks()
    assert ticks[4].label1.get_window_extent(renderer).bounds[0] == 27.305555555555543
    assert ticks[6].label1.get_window_extent(renderer).bounds[1] == 28.6111111111111
    fig.tight_layout()
    assert fig.subplotpars.left == 0.10038888888888892
    assert fig.subplotpars.bottom == 0.08277777777777778


def test_every_font_lookup_of_a_draw_is_logged(logged):
    # More than a thousand texts looked up in one save, as in the
    # multi-Axes layouts of Rougier's book.
    fig = plt.figure(figsize=(4, 2))
    for i in range(700):
        fig.text(0.5, 0.5, f"t{i}", family="xkcd-pltrs-missing")
    logged.clear()
    fig.savefig(io.BytesIO(), format="png")
    assert sum("not found" in message for message in logged) == 2100


def test_the_box_of_an_axis_text_lays_the_text_out_again(logged):
    # matplotlib's update_bbox_position_size lays a text with a box out once
    # more, looking its font up again (Rougier's tick-labels-variation).
    fig, ax = plt.subplots(figsize=(4, 2))
    ax.set_xticks([0.5])
    ax.set_yticks([])
    label = ax.get_xticklabels()[0]
    label.set_family("xkcd-pltrs-missing")
    label.set_bbox(dict(boxstyle="round"))
    ax.set_xlabel("abc", family="xkcd-pltrs-missing", bbox=dict(boxstyle="round"))
    fig.tight_layout()
    logged.clear()
    fig.savefig(io.BytesIO(), format="pdf")
    assert sum("not found" in message for message in logged) == 18


def test_a_mathtext_font_pattern_names_its_weight_as_given(logged):
    with pltrs.rc_context({"mathtext.fontset": "custom",
                           "mathtext.rm": "DejaVu Sans:semibold"}):
        fig = plt.figure()
        fig.text(0.5, 0.5, r"$\mathrm{ab}$")
        fig.savefig(io.BytesIO(), format="png")
    assert sorted({m for m in logged if "weight" in m}) == [
        "findfont: Failed to find font weight semibold for DejaVu Sans, "
        "now using 700."]


def _triangles():
    """Three triangles in one path, and the vertex array it was made of."""
    vertices = np.array([(-0.25, -0.5), (0.0, 0.5), (0.25, -0.5), (0.0, 0.0)] * 3)
    codes = [Path.MOVETO, Path.LINETO, Path.LINETO, Path.CLOSEPOLY] * 3
    return Path(vertices, codes), vertices


def _pixels(fig):
    """The pixels of *fig* drawn on its canvas."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def _figure_of(collection):
    """A figure drawing *collection* alone in an Axes, and the Axes."""
    fig, ax = plt.subplots(figsize=(3, 3))
    ax.add_collection(collection)
    return fig, ax


def _drawn(path):
    """The pixels of a collection of *path* drawn alone in an Axes."""
    fig, ax = _figure_of(PathCollection([path], facecolor="white", edgecolor="black"))
    ax.set(xlim=(-1.5, 1.5), ylim=(-1.5, 1.5))
    return _pixels(fig)


def test_a_collection_draws_its_paths_as_the_program_changed_them():
    # from-python-to-numpy's marker_collection moves the markers of a path
    # collection by changing the path's vertex array in place.
    path, vertices = _triangles()
    collection = PathCollection([path], facecolor="white", edgecolor="black")
    vertices[4:8] += 1.0
    fig, ax = _figure_of(collection)
    assert ax.dataLim.bounds == (-0.25, -0.5, 1.5, 2.0)
    ax.set(xlim=(-1.5, 1.5), ylim=(-1.5, 1.5))
    moved = _pixels(fig)
    expected, _ = _triangles()
    expected.vertices[4:8] += 1.0
    assert np.array_equal(moved, _drawn(expected))
    vertices[8:] -= 1.0
    again = _pixels(fig)
    expected.vertices[8:] -= 1.0
    assert np.array_equal(again, _drawn(expected))
    assert not np.array_equal(again, moved)


# --- matplotlib's signatures, docstrings and names ----------------------------

def test_figure_base_takes_artist_properties():
    from pltrs.figure import FigureBase
    assert str(inspect.signature(FigureBase)) == "(**kwargs)"
    base = FigureBase(alpha=0.5, label="x")
    assert (base.get_alpha(), base.get_label(), base.stale) == (0.5, "x", True)
    with pytest.raises(AttributeError,
                       match=r"FigureBase.set\(\) got an unexpected keyword argument 'foo'"):
        FigureBase(foo=1)


def test_a_character_is_indexed_in_the_fallback_it_was_loaded_from():
    from pltrs.font_manager import findfont
    from pltrs.ft2font import FT2Font
    font = FT2Font(findfont("cmr10"),
                   _fallback_list=[FT2Font(findfont("DejaVu Sans"))])
    font.set_size(12, 72)
    e_acute = ord("é")
    assert (font.get_char_index(e_acute), font.get_char_index(e_acute, _fallback=False)) == (0, 0)
    font.load_char(e_acute)
    assert font.get_char_index(e_acute) == 171
    assert font.get_char_index(e_acute, _fallback=False) == 0
    font.clear()
    assert font.get_char_index(e_acute) == 0


def test_parasite_axes_document_their_position():
    from pltrs.artist import kwdoc
    from pltrs.mpl_toolkits.axes_grid1.parasite_axes import ParasiteAxes
    assert [line.strip() for line in kwdoc(ParasiteAxes).splitlines()
            if line.strip().startswith("position")] == [
        "position: [left, bottom, width, height] or `~matplotlib.transforms.Bbox`"]


def test_matplotlib_s_classes_take_matplotlib_s_members():
    from pltrs import collections, colorizer, offsetbox
    from pltrs.artist import Artist
    from pltrs.axis import Axis
    from pltrs.container import Container
    assert (Axis.converter.fget.__qualname__
            == "deprecate_privatize_attribute.__set_name__.<locals>.<lambda>")
    assert offsetbox.OffsetBox.axes.fget is Artist.axes.fget
    assert (Container.get_label, Container.set_label) == (Artist.get_label, Artist.set_label)
    assert collections.Collection.set_array is colorizer._ScalarMappable.set_array
    assert collections.Collection.changed is colorizer._ScalarMappable.changed
    assert collections.Collection.get_alpha is colorizer._ColorizerInterface.get_alpha


def test_members_taken_from_matplotlib_s_bases_keep_their_behaviour():
    fig, ax = plt.subplots()
    ax.plot([datetime.datetime(2020, 1, 1)], [1])
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert type(ax.xaxis.converter).__name__ == "_SwitchableDateConverter"
    assert ax.xaxis._converter is ax.xaxis.get_converter()
    container = Container([], label=5)
    assert container.get_label() == "5"
    container.set_label(None)
    assert (container.get_label(), container.stale) == (None, True)
    points = ax.scatter([1, 2], [3, 4])
    points.set_alpha([0.2, 0.4])
    assert points.get_alpha().tolist() == [0.2, 0.4]
    points.set_array(np.array([1.0, 2.0]))
    assert (points.get_array().tolist(), points.norm.vmin, points.norm.vmax) == (
        [1.0, 2.0], 1.0, 2.0)


def test_modules_and_classes_of_text_are_plain():
    from pltrs import dviread, text
    from pltrs.backends import backend_agg
    assert type(text) is types.ModuleType
    assert type(text.Text) is type and type(text.Annotation) is type
    assert type(backend_agg.RendererAgg) is type
    assert type(dviread.PsfontsMap) is type


def test_a_replaced_text_measurement_is_followed_until_it_is_restored(monkeypatch):
    from pltrs import text
    fig = plt.figure(dpi=100)
    label = fig.text(0.5, 0.5, "abcd")
    width = label.get_window_extent().width
    monkeypatch.setattr(text, "_get_text_metrics_with_cache",
                        lambda renderer, s, prop, ismath, dpi: (10.0 * len(s), 5.0, 1.0))
    assert label.get_window_extent().width == 40.0
    monkeypatch.undo()
    assert label.get_window_extent().width == width


def test_names_matplotlib_s_modules_import():
    from pltrs import layout_engine
    from pltrs.backends import backend_webagg
    from pltrs.testing import decorators
    assert pltrs.parse_version("3.10") < pltrs.parse_version("3.11rc1")
    assert str(decorators.parse_version("2.0.1")) == "2.0.1"
    thread = backend_webagg.webagg_server_thread
    # matplotlib's: "Thread-1 (<lambda>)", not started, not a daemon.
    assert isinstance(thread, threading.Thread) and thread.name.endswith(" (<lambda>)")
    assert not (thread.is_alive() or thread.daemon)
    assert backend_webagg.tornado.__name__ == "tornado"
    fig = plt.figure()
    grid = fig.add_gridspec(2, 2)
    axes = [fig.add_subplot(grid[0, 0]), fig.add_subplot(grid[1, :]),
            fig.add_axes([0.1, 0.1, 0.2, 0.2]),
            fig.add_subplot(fig.add_gridspec(1, 1, left=0.5)[0])]
    specs = layout_engine.get_subplotspec_list(axes)
    assert [None if spec is None else spec.get_geometry() for spec in specs] == [
        (2, 2, 0, 0), (2, 2, 2, 3), None, None]
