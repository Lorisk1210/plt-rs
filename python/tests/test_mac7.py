"""
Transforms of the program's asked where matplotlib asks them, usetex
settings read when the text is drawn, and a 3-D Axes drawn alone in the
view it is drawn in. The expected values are matplotlib 3.11.2's.
"""

import collections
import io
import shutil

import numpy as np
import pytest

import pltrs
from pltrs import collections as mcollections
from pltrs import patches, transforms
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

#: The calls of each counted method of the transforms below.
CALLS = collections.Counter()


def _counted(name):
    """A method calling the base class' *name*, counted in `CALLS`."""
    def method(self, *args, **kwargs):
        CALLS[name] += 1
        return getattr(super(type(self), self), name)(*args, **kwargs)
    method.__name__ = name
    return method


class _Shift(transforms.Transform):
    """A non-affine transform moving points up by *shift*."""

    input_dims = output_dims = 2
    is_separable = False
    has_inverse = True

    def __init__(self, shift=0.05):
        super().__init__()
        self.shift = shift

    def transform_non_affine(self, values):
        values = np.array(values, dtype=float)
        values[:, 1] += self.shift
        return values

    def inverted(self):
        return Shift(-self.shift)


Shift = type("Shift", (_Shift,), {name: _counted(name) for name in (
    "get_affine", "transform_non_affine", "transform_path_non_affine")})


class _Scale(transforms.Affine2DBase):
    """An affine transform computing its matrix from *factor* when asked."""

    def __init__(self, factor=0.9):
        super().__init__()
        self.factor = factor

    def get_matrix(self):
        return np.array([[self.factor, 0.0, 0.0], [0.0, self.factor, 0.0], [0.0, 0.0, 1.0]])


Scale = type("Scale", (_Scale,), {"get_matrix": _counted("get_matrix")})


def _axes(**kwargs):
    fig = Figure(figsize=(3, 2), **kwargs)
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    ax.set(xlim=(0, 1), ylim=(0, 1))
    return fig, ax


def _taken():
    counts = dict(CALLS)
    CALLS.clear()
    return counts


def test_every_measure_of_a_line_asks_its_transform_for_the_affine_part():
    # A composite of the program's transform keeps no answer of it: before,
    # the first measure filled a memo and `get_tightbbox` asked nothing.
    fig, ax = _axes()
    line, = ax.plot([0.1, 0.5, 0.9], [0.2, 0.8, 0.4], transform=Shift() + ax.transData)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    _taken()
    measured = []
    for measure in (line.get_window_extent, line.get_tightbbox, line.get_tightbbox):
        measure(renderer)
        measured.append(_taken())
    assert measured == [{"get_affine": 1, "transform_non_affine": 1}] * 3


def test_a_collection_maps_its_paths_through_the_transform_once_per_draw():
    fig, ax = _axes()
    ax.add_collection(mcollections.LineCollection(
        [[(0.1, 0.1), (0.9, 0.5)], [(0.2, 0.7), (0.6, 0.3)]], transform=Shift() + ax.transData))
    drawn = []
    for _ in range(2):
        fig.canvas.draw()
        drawn.append(_taken())
    assert drawn[1] == {"get_affine": 1, "transform_non_affine": 2,
                        "transform_path_non_affine": 2}


def test_offsets_and_patch_outlines_go_through_the_transform_at_once():
    fig, ax = _axes()
    ax.scatter([0.2, 0.5, 0.8], [0.3, 0.6, 0.4], transform=Shift() + ax.transData)
    fig.canvas.draw()
    _taken()
    fig.canvas.draw()
    assert _taken() == {"get_affine": 1, "transform_non_affine": 1}
    fig, ax = _axes()
    _taken()
    ax.add_patch(patches.Rectangle((0.2, 0.2), 0.4, 0.3, transform=Shift() + ax.transData))
    assert _taken() == {"get_affine": 1, "transform_non_affine": 1}
    fig.canvas.draw()
    _taken()
    fig.canvas.draw()
    assert _taken() == {"get_affine": 1, "transform_non_affine": 1,
                        "transform_path_non_affine": 1}


def test_an_image_maps_its_mesh_through_the_transform_as_arrays():
    # matplotlib maps the lookup mesh with a few calls (6 per draw); pltrs
    # called the transform once per output pixel (30,268 times here).
    fig, ax = _axes()
    ax.imshow(np.arange(12.0).reshape(3, 4), extent=(0.1, 0.9, 0.1, 0.9),
              transform=Shift() + ax.transData)
    fig.canvas.draw()
    _taken()
    fig.canvas.draw()
    assert 1 <= _taken()["transform_non_affine"] <= 12


@pytest.mark.parametrize("kind", ["line", "patch"])
def test_an_affine_transform_computing_its_matrix_is_asked_at_every_draw(kind):
    fig, ax = _axes()
    given = Scale() + ax.transData
    if kind == "line":
        ax.plot([0.1, 0.9], [0.2, 0.8], transform=given)
    else:
        ax.add_patch(patches.Rectangle((0.2, 0.2), 0.4, 0.3, transform=given))
    fig.canvas.draw()
    _taken()
    asked = []
    for _ in range(2):
        fig.canvas.draw()
        asked.append(_taken().get("get_matrix", 0))
    assert asked == [1, 1]


def test_an_affine_transform_computing_its_matrix_draws_as_it_computes_it():
    # matplotlib draws with the matrix the transform computes when asked.
    fig, ax = _axes()
    scale = _Scale(1.0)
    ax.plot([0.1, 0.9], [0.2, 0.8], transform=scale + ax.transData, lw=4)
    fig.canvas.draw()
    before = np.asarray(fig.canvas.buffer_rgba()).copy()
    scale.factor = 0.5
    ax.stale = True
    fig.canvas.draw()
    assert not np.array_equal(before, np.asarray(fig.canvas.buffer_rgba()))


needs_tex = pytest.mark.skipif(shutil.which("latex") is None, reason="needs TeX")


@needs_tex
def test_usetex_reads_the_preamble_when_the_text_is_drawn():
    with pltrs.rc_context({"text.usetex": True,
                           "text.latex.preamble": r"\usepackage{nosuchpackage}"}):
        fig, ax = _axes()
        ax.set_title("Title")
    # Saved after the context: drawn with the preamble of that moment.
    fig.savefig(io.BytesIO(), format="png")
    with pltrs.rc_context({"text.usetex": True}):
        fig, ax = _axes()
        ax.set_title("Title")
        fig.canvas.draw()
        pltrs.rcParams["text.latex.preamble"] = r"\usepackage{nosuchpackage}"
        with pytest.raises(RuntimeError, match="nosuchpackage"):
            fig.savefig(io.BytesIO(), format="png")


@pytest.mark.parametrize("kind", ["line", "scatter", "surface", "quiver"])
def test_a_3d_axes_drawn_alone_projects_in_its_view_of_that_moment(kind):
    # ultraplot's selective draws redraw a rotated 3-D Axes alone
    # (`draw_artist`) over a background without it.
    fig = Figure(figsize=(4, 3))
    FigureCanvasAgg(fig)
    axs = [fig.add_subplot(1, 2, i + 1, projection="3d") for i in range(2)]
    t = np.linspace(0, 6 * np.pi, 300)
    for ax in axs:
        if kind == "line":
            ax.plot(np.cos(t), np.sin(t), t)
        elif kind == "scatter":
            ax.scatter(np.cos(t[::20]), np.sin(t[::20]), t[::20])
        elif kind == "surface":
            x, y = np.meshgrid(np.linspace(-1, 1, 12), np.linspace(-1, 1, 12))
            ax.plot_surface(x, y, x * y)
        else:
            ax.quiver([0, 0.5], [0, 0.5], [0, 2], [1, 0], [0, 1], [2, 1])
    axs[0].set_visible(False)
    fig.canvas.draw()
    background = fig.canvas.copy_from_bbox(fig.bbox)
    axs[0].set_visible(True)
    fig.canvas.draw()
    axs[0].view_init(elev=35, azim=55, roll=5)
    fig.canvas.restore_region(background)
    fig.draw_artist(axs[0])
    alone = np.asarray(fig.canvas.buffer_rgba()).copy()
    fig.canvas.draw()
    assert np.array_equal(alone, np.asarray(fig.canvas.buffer_rgba()))
