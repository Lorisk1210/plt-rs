"""
The program's code a draw runs, called where matplotlib calls it (the
functions of secondary axes and function scales, the properties an `Arc`
subclass computes), the changes it makes while the figure is drawn, and
the state a failed or odd 3-D draw leaves. The expected values are real
matplotlib 3.11's for the same programs.
"""

import io
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.patheffects as pe
import pltrs.pyplot as plt
from pltrs import collections as mcoll, colors, patches
from pltrs.mpl_toolkits.mplot3d import art3d


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _secondary_limits(layout, save):
    """The shapes of the arrays a secondary axis' forward function is
    called with while its figure is drawn (or saved)."""
    calls = []
    fig, ax = plt.subplots(layout=layout)
    ax.plot([0, 1, 2], [0, 1, 4])
    ax.set_title("t")

    def forward(x):
        calls.append(np.shape(x))
        return 2 * x

    ax.secondary_xaxis("top", functions=(forward, lambda x: x / 2))
    if save:
        fig.savefig(io.BytesIO(), format="png")
    else:
        fig.canvas.draw()
    return calls


@pytest.mark.parametrize("layout, save, count", [
    (None, False, 4), ("constrained", True, 12), ("tight", True, 12)])
def test_secondary_axes_take_their_hosts_limits_where_matplotlib_does(layout, save, count):
    # `_set_lims` at `_get_draw_artists`, where the host places its title,
    # twice where the axis is drawn, and where layouts measure it.
    assert _secondary_limits(layout, save) == [(2,)] * count


def test_autoscaling_maps_its_ends_as_one_column():
    shapes = []
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], [1, 4, 9])

    def forward(x):
        shapes.append(("f", np.shape(x)))
        return np.sqrt(x)

    def inverse(x):
        shapes.append(("i", np.shape(x)))
        return x**2

    ax.set_yscale("function", functions=(forward, inverse))
    shapes.clear()
    ax.set_yticks([1, 4, 9])
    assert shapes == [("f", (2, 1)), ("i", (2, 1))]


def test_an_arc_subclass_is_read_as_matplotlib_reads_it():
    reads = []

    class Computed(patches.Arc):
        def __init__(self, ax):
            self.ax = ax
            super().__init__((0.5, 0.5), 0.4, 0.4,
                             theta1=self.theta1, theta2=self.theta2)
            reads.append("made")
            ax.add_patch(self)
            reads.append("added")

        def _get_center(self):
            reads.append("center")
            return (0.5, 0.5)

        def _set_center(self, xy):
            reads.append("set_center")

        def _get_theta1(self):
            reads.append("theta1")
            return 10.0

        def _get_theta2(self):
            reads.append("theta2")
            return 80.0

        def _set_theta(self, value):
            reads.append("set_theta")

        def _get_size(self):
            reads.append("size")
            return 0.4

        _center = property(_get_center, _set_center)
        theta1 = property(_get_theta1, _set_theta)
        theta2 = property(_get_theta2, _set_theta)
        width = property(_get_size)
        height = property(_get_size)

    fig, ax = plt.subplots()
    Computed(ax)
    fig.canvas.draw()
    assert reads == [
        "theta1", "theta2", "set_center", "set_theta", "set_theta", "size",
        "size", "theta1", "theta2", "made", "center", "center", "added",
        "center", "center", "size", "size", "theta1", "theta2", "center",
        "center"]


def test_changes_made_while_drawing_end_as_matplotlib_ends_them():
    class Recolor(pe.AbstractPathEffect):
        def __init__(self, artist, other):
            super().__init__()
            self.artist, self.other = artist, other

        def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
            self.artist.set_color("red")
            self.other.set_linewidth(3)
            renderer.draw_path(gc, tpath, affine, rgbFace)

    fig, ax = plt.subplots()
    early, = ax.plot([0, 1], [1, 0], zorder=1)
    line, = ax.plot([0, 1], [0, 1], zorder=2)
    late, = ax.plot([0, 1], [0.5, 0.5], zorder=3)
    line.set_path_effects([Recolor(line, early)])
    seen = []
    fig.canvas.mpl_connect("draw_event", lambda event: seen.append(line.get_color()))
    fig.canvas.draw()
    # The draw event sees the change; the artist changed while it was
    # drawn ends fresh, the one drawn before stays stale.
    assert seen == ["red"]
    assert (fig.stale, line.stale, early.stale, late.stale) == (False, False, True, False)
    assert early.get_linewidth() == 3.0


def test_a_failed_3d_draw_stops_before_drawing_lines():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    line = art3d.Line3D([0, 1], [0, 1], [0, 1])
    ax.add_line(line)
    scatter = ax.scatter([0, 1, 2], [2, 1, 0], [1, 1, 1])
    scatter._offsets3d = ([0, 1], [0, 1], [5, 6, 7])
    with pytest.raises(ValueError):
        fig.canvas.draw()
    assert [list(v) for v in line.get_data()] == [[], []]


def test_nan_spreads_through_the_tight_box_of_3d_axes():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.scatter([0, 1, 2], [0.2, 0.5, 0.8], [0, 1, 2])
    ax.set_ylim(-1, 1)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        ax.set_yscale("logit")
        fig.canvas.draw()
        assert np.isnan(ax.get_tightbbox().extents).all()


@pytest.mark.parametrize("three", [False, True])
def test_a_new_norm_is_scaled_by_the_next_draw(three):
    fig = plt.figure()
    if three:
        ax = fig.add_subplot(projection="3d")
        col = art3d.Poly3DCollection([[(0, 0, 0), (1, 0, 0), (1, 1, 1)],
                                      [(0, 1, 0), (1, 1, 0), (0, 0, 1)]])
        ax.add_collection3d(col)
    else:
        ax = fig.add_subplot()
        col = mcoll.PolyCollection([[(0, 0), (1, 0), (1, 1)], [(0, 1), (1, 1), (0, 0)]])
        ax.add_collection(col)
    col.set_array(np.array([1.0, 3.0]))
    col.set_norm(colors.LogNorm())
    assert col.get_clim() == (None, None)
    fig.canvas.draw()
    assert col.get_clim() == (1.0, 3.0)


def test_a_program_projection_builds_its_trees_from_the_boxes_matplotlib_starts_with():
    from pltrs.axes import Axes
    from pltrs.projections import register_projection

    seen = []

    class Recording(Axes):
        name = "recording_trees"

        def _set_lim_and_transforms(self):
            super()._set_lim_and_transforms()
            seen.append((float(self.transAxes.get_matrix()[0, 0]),
                         float(self.transData.get_matrix()[0, 0])))

    register_projection(Recording)
    fig = plt.figure(figsize=(4, 3), dpi=100)
    # A subplot's cell replaces the unit box after; a rectangle is there
    # from the start.
    fig.add_subplot(projection="recording_trees")
    fig.add_axes((0.25, 0.25, 0.5, 0.5), projection="recording_trees")
    assert seen == [(400.0, 400.0), (200.0, 200.0)]
