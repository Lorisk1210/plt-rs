"""
Private state and assigned attributes the last rounds left: a spine of the
program's that places itself (or leaves itself unplaced) in its
``_ensure_position_is_set``; every attribute matplotlib's stubs declare
writable taking its own value back (`.Axis` label, offset text and tick
lists, an Axes' title, a figure's images, a 3-D Axes' boxes), the fields
matplotlib keeps that pltrs lacked (``_projection_init``, a contour set's
``locator``); a figure holding a program's subclass of each artist kind,
pickled and drawn again. Every expected value was recorded from matplotlib
3.11.
"""

import io
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.collections as mcollections
import pltrs.colors as mcolors
import pltrs.lines as mlines
import pltrs.patches as mpatches
import pltrs.pyplot as plt
import pltrs.spines as mspines
import pltrs.text as mtext
from pltrs.path import Path


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def rounded(values, digits=1):
    """*values* (a box's bounds or a sequence), rounded."""
    return np.round(getattr(values, "bounds", values), digits).tolist()


def _placed_outward(self):
    if self._position is None:
        self._position = ('outward', 10.0)
        self.set_position(self._position)


def _assigned_outward(self):
    if self._position is None:
        self._position = ('outward', 10.0)


def _bounded(self):
    if self._position is None:
        self._position = ('outward', 0.0)
        self.set_bounds(0.2, 0.8)


@pytest.mark.parametrize("hook, registered, spine, label, tick", [
    (None, False, [22.5, 13.2, 139.5, 0.0], [19.8, -1.0, 18.0, 8.3], 11.7),
    (None, True, [22.5, 10.3, 139.5, 2.9], [19.8, -1.0, 18.0, 8.3], 11.7),
    (_placed_outward, False, [22.5, 4.9, 139.5, 0.0], [19.8, -1.0, 18.0, 8.3], 11.7),
    (_placed_outward, True, [22.5, 1.9, 139.5, 2.9], [19.8, -9.3, 18.0, 8.3], 3.4),
    (_assigned_outward, False, [22.5, 17.4, 139.5, 0.0], [19.8, -1.0, 18.0, 8.3], 11.7),
    (_assigned_outward, True, [22.5, 14.5, 139.5, 2.9], [19.8, -1.0, 18.0, 8.3], 11.7),
    (_bounded, False, [54.2, 17.4, 76.1, 0.0], [19.8, -1.0, 18.0, 8.3], 11.7),
    (_bounded, True, [54.2, 14.5, 76.1, 2.9], [19.8, -1.0, 18.0, 8.3], 11.7),
])
def test_a_spine_of_the_program_is_placed_as_its_hook_places_it(
        hook, registered, spine, label, tick):
    # A spine whose `_ensure_position_is_set` only assigns `_position` stays
    # in data coordinates (the transform it was made with), as matplotlib's;
    # the ticks keep the transform of the spine they were made on until a
    # registered axis makes them anew.
    fig, ax = plt.subplots(figsize=(3, 2), dpi=60)
    ax.plot([0, 1])
    cls = mspines.Spine if hook is None else type(
        "Own", (mspines.Spine,), {"_ensure_position_is_set": hook})
    new = cls.linear_spine(ax, 'bottom')
    ax.spines['bottom'] = new
    if registered:
        new.register_axis(ax.xaxis)
    fig.canvas.draw()
    assert rounded(new.get_window_extent()) == spine
    assert rounded(ax.get_xticklabels()[1].get_window_extent()) == label
    assert rounded(ax.xaxis.get_major_ticks()[1].tick1line.get_window_extent())[1] == tick


@pytest.mark.parametrize("registered, size, spine, label, tick", [
    (False, (177, 118), [22.5, 0.7, 139.5, 0.0], [19.8, -1.0, 18.0, 8.3], 11.7),
    (True, (177, 131), [22.5, -2.2, 139.5, 2.9], [19.8, -13.5, 18.0, 8.3], -0.8),
])
def test_a_spine_with_no_axis_measures_as_its_line(registered, size, spine, label, tick):
    # Placed by hand: with no axis registered, the ticks stay where the
    # replaced spine made them and pad neither the spine nor a tight box.
    fig, ax = plt.subplots(figsize=(3, 2), dpi=60)
    ax.plot([0, 1])
    new = mspines.Spine.linear_spine(ax, 'bottom')
    ax.spines['bottom'] = new
    if registered:
        new.register_axis(ax.xaxis)
    new.set_position(('outward', 15))
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png", bbox_inches="tight")
    assert plt.imread(io.BytesIO(buffer.getvalue())).shape[1::-1] == size
    fig.canvas.draw()
    assert rounded(new.get_window_extent()) == spine
    assert rounded(ax.get_xticklabels()[1].get_window_extent()) == label
    assert rounded(ax.xaxis.get_major_ticks()[1].tick1line.get_window_extent())[1] == tick


@pytest.mark.parametrize("read", [
    lambda fig, ax: (ax.xaxis, "label"),
    lambda fig, ax: (ax.xaxis, "offsetText"),
    lambda fig, ax: (ax.yaxis, "majorTicks"),
    lambda fig, ax: (ax.yaxis, "minorTicks"),
    lambda fig, ax: (ax.xaxis, "isDefault_label"),
    lambda fig, ax: (ax, "title"),
    lambda fig, ax: (fig, "images"),
], ids=["label", "offsetText", "majorTicks", "minorTicks", "isDefault_label", "title",
        "images"])
def test_an_attribute_takes_its_own_value_back(read):
    # ROBUST3's fuzzer: `ax.xaxis.offsetText = ax.xaxis.offsetText` raised.
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    ax.plot([0, 1e6])
    fig.canvas.draw()
    before = np.asarray(fig.canvas.buffer_rgba()).copy()
    owner, name = read(fig, ax)
    value = getattr(owner, name)
    setattr(owner, name, value)
    assert getattr(owner, name) is value or getattr(owner, name) == value
    fig.canvas.draw()
    assert np.array_equal(np.asarray(fig.canvas.buffer_rgba()), before)


def test_assigned_tick_lists_and_default_label_read_back():
    fig, ax = plt.subplots()
    ticks = list(ax.yaxis.majorTicks)
    ax.yaxis.majorTicks = ticks
    assert ax.yaxis.majorTicks is ticks
    assert ax.xaxis.isDefault_label is True
    ax.xaxis.isDefault_label = False
    assert ax.xaxis.isDefault_label is False
    images = list(fig.images)
    fig.images = images
    assert fig.images is images


def test_a_3d_axes_takes_its_plane_boxes_back():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot([0, 1], [0, 1], [0, 1])
    bounds = ax.dataLim.bounds
    ax.dataLim.bounds = bounds
    assert ax.dataLim.bounds == bounds
    ax.viewLim.intervalx = ax.viewLim.intervalx
    fig.canvas.draw()


def test_axes_know_how_their_figure_made_them():
    fig, ax = plt.subplots()
    assert ax._projection_init[1] == {'sharex': None, 'sharey': None}
    colorbar = fig.colorbar(ax.imshow([[0, 1], [1, 0]]))
    assert colorbar.ax._projection_init[1] == {'label': '<colorbar>'}
    from pltrs.mpl_toolkits.axes_grid1 import host_subplot
    assert host_subplot(111, figure=plt.figure())._projection_init is None


@pytest.mark.parametrize("kwargs, kind, nbins, numticks", [
    ({}, "MaxNLocator", 8, None),
    ({"levels": 4}, "MaxNLocator", 5, None),
    ({"levels": [1, 2, 3]}, "NoneType", None, None),
    ({"norm": "log"}, "LogLocator", None, "auto"),
    ({"levels": 5, "norm": "log"}, "LogLocator", None, 5),
])
def test_a_contour_set_keeps_the_locator_of_its_levels(kwargs, kind, nbins, numticks):
    z = np.add.outer(np.linspace(1, 2, 8), np.linspace(1, 3, 9))
    if kwargs.get("norm") == "log":
        kwargs = {**kwargs, "norm": mcolors.LogNorm()}
    locator = plt.figure().add_subplot().contour(z, **kwargs).locator
    assert type(locator).__name__ == kind
    assert getattr(locator, "_nbins", None) == nbins
    assert getattr(locator, "numticks", None) == numticks


def test_a_contour_set_keeps_the_locator_given():
    z = np.add.outer(np.linspace(1, 2, 8), np.linspace(1, 3, 9))
    given = pltrs.ticker.MaxNLocator(5)
    assert plt.figure().add_subplot().contour(z, 3, locator=given).locator is given


class OwnLine(mlines.Line2D):
    """A line class of the program's, overriding nothing."""


class OwnCollection(mcollections.PathCollection):
    """A collection class of the program's with its own `get_paths`."""

    def get_paths(self):
        return super().get_paths()


class OwnPatch(mpatches.Rectangle):
    """A patch class of the program's with its own `draw`."""

    def draw(self, renderer):
        return super().draw(renderer)


class OwnText(mtext.Text):
    """A text class of the program's with its own `get_window_extent`."""

    def get_window_extent(self, renderer=None):
        return super().get_window_extent(renderer)


@pytest.mark.parametrize("drawn", [False, True])
def test_a_figure_of_program_subclasses_pickles_and_draws_alike(drawn):
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.add_line(OwnLine([0.1, 0.9], [0.2, 0.8], color="C1", lw=3))
    ax.add_collection(OwnCollection([Path.unit_circle()], sizes=[200],
                                    offsets=[(0.3, 0.3), (0.7, 0.6)],
                                    offset_transform=ax.transData))
    ax.add_patch(OwnPatch((0.2, 0.2), 0.4, 0.3, color="C2"))
    ax.add_artist(OwnText(0.3, 0.5, "text", fontsize=14))
    if drawn:
        fig.canvas.draw()
    fig.canvas.draw()
    before = np.asarray(fig.canvas.buffer_rgba()).copy()
    again = pickle.loads(pickle.dumps(fig))
    again.canvas.draw()
    assert np.array_equal(np.asarray(again.canvas.buffer_rgba()), before)
    kinds = sorted(type(a).__name__ for a in again.axes[0].get_children()
                   if type(a).__name__.startswith("Own"))
    assert kinds == ["OwnCollection", "OwnLine", "OwnPatch", "OwnText"]
    third = pickle.loads(pickle.dumps(again))
    third.canvas.draw()
    assert np.array_equal(np.asarray(third.canvas.buffer_rgba()), before)
