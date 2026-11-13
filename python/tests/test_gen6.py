"""
What the second round on the generated family ``overrides`` (``cargo
xtask diff --generated overrides``) showed pltrs not asking, or asking
otherwise than matplotlib 3.11, each with a small subclass of the kind
libraries write: an Axes class's ``add_*`` methods, which matplotlib's
plotting methods add what they make with (mne's ``_NoOpAxes`` throws
collections away); the setters matplotlib's ``Text`` and ``Legend``
constructors call (ultraplot's curved text, MetPy's text collection,
ultraplot's legend); the children a class gives in ``pick`` (legendkit);
a figure class's ``draw`` in the draw before a tight print; a layout
engine of the program's run once per print; an axes_grid1 size of the
program's asked with a renderer and pickled with its figure. Every
expected value was recorded from matplotlib.
"""

import io
import pickle
import sys
import types

import numpy as np
import pytest

import pltrs
import pltrs.axes as maxes
import pltrs.pyplot as plt
from pltrs.artist import Artist
from pltrs.backend_bases import MouseEvent
from pltrs.figure import Figure
from pltrs.layout_engine import LayoutEngine
from pltrs.legend import Legend
from pltrs.text import Text
from pltrs.mpl_toolkits.axes_grid1 import axes_size, make_axes_locatable

#: The module the classes pickled by name live in.
_MODULE = types.ModuleType("gen6_classes")
sys.modules[_MODULE.__name__] = _MODULE


def _kept(cls):
    """*cls*, picklable by name, as a library's class is."""
    cls.__module__ = _MODULE.__name__
    cls.__qualname__ = cls.__name__
    setattr(_MODULE, cls.__name__, cls)
    return cls


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X = np.linspace(0, 2 * np.pi, 7)


class _Adding(maxes.Axes):
    """An Axes noting what its ``add_*`` methods are given."""

    def __init__(self, *args, **kwargs):
        self.added = []
        super().__init__(*args, **kwargs)

    def add_line(self, line):
        self.added.append(("line", type(line).__name__))
        return super().add_line(line)

    def add_patch(self, p):
        self.added.append(("patch", type(p).__name__))
        return super().add_patch(p)

    def add_collection(self, collection, autolim=True):
        self.added.append(("collection", type(collection).__name__, autolim))
        return super().add_collection(collection, autolim)

    def add_image(self, image):
        self.added.append(("image", type(image).__name__))
        return super().add_image(image)

    def _add_text(self, txt):
        self.added.append(("text", type(txt).__name__))
        return super()._add_text(txt)

    def add_table(self, tab):
        self.added.append(("table", type(tab).__name__))
        return super().add_table(tab)


def test_plotting_methods_add_through_the_classes_add_methods():
    ax = plt.figure().add_subplot(axes_class=_Adding)
    ax.added.clear()
    ax.plot(X, np.sin(X), X, np.cos(X))
    ax.scatter(X, X)
    ax.vlines([1, 2], 0, 1)
    ax.bar([1, 2], [3, 4])
    ax.imshow(np.eye(3))
    ax.text(0, 0, "t")
    ax.annotate("a", (0, 0))
    ax.table([["a", "b"]])
    assert ax.added == [
        ("line", "Line2D"), ("line", "Line2D"), ("collection", "PathCollection", True),
        ("collection", "LineCollection", False), ("patch", "Rectangle"), ("patch", "Rectangle"),
        ("image", "AxesImage"), ("text", "Text"), ("text", "Annotation"), ("table", "Table")]


def test_an_add_method_not_reaching_the_axes_leaves_the_artist_out():
    class NoOp(maxes.Axes):
        def add_collection(self, collection, *args, **kwargs):
            return collection

    ax = plt.figure().add_subplot(axes_class=NoOp)
    made = ax.scatter(X, X)
    assert made.axes is None
    assert list(ax.collections) == []


def test_text_and_legend_constructors_call_the_classes_setters():
    called = []

    class Curved(Text):
        def set_text(self, s):
            called.append(("text", s))
            super().set_text(s)

        def set_usetex(self, usetex):
            called.append(("usetex", usetex))
            super().set_usetex(usetex)

    Curved(0, 0, "curve")
    assert called == [("text", "curve"), ("usetex", None)]

    class Placed(Legend):
        def set_loc(self, loc=None):
            called.append(("loc", loc))
            super().set_loc(loc)

    fig, ax = plt.subplots()
    line, = ax.plot(X, X)
    called.clear()
    Placed(ax, [line], ["line"], loc="upper left")
    assert called == [("loc", "upper left")]


def test_pick_goes_through_the_children_a_class_gives():
    picked = []

    class Part(Artist):
        def contains(self, mouseevent):
            picked.append("part")
            return False, {}

    class Holder(Artist):
        def __init__(self):
            super().__init__()
            self.part = Part()

        def get_children(self):
            return [self.part]

        def contains(self, mouseevent):
            return False, {}

    fig, ax = plt.subplots(figsize=(3, 2))
    holder = Holder()
    ax.add_artist(holder)
    holder.part.set_figure(fig)
    holder.set_picker(True)
    holder.part.set_picker(True)
    fig.canvas.draw()
    fig.pick(MouseEvent("button_press_event", fig.canvas, 150, 100))
    assert picked == ["part"]


def test_a_figure_class_draws_before_a_tight_print():
    renderers = []

    @_kept
    class Drawn(Figure):
        def draw(self, renderer):
            renderers.append(type(renderer).__name__)
            super().draw(renderer)

    fig = plt.figure(FigureClass=Drawn, figsize=(3, 2))
    fig.add_subplot().plot(X, X)
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert renderers == ["RendererAgg", "RendererAgg"]


def test_a_layout_engine_of_the_program_runs_once_per_tight_print():
    runs = []

    class Engine(LayoutEngine):
        _adjust_compatible = True
        _colorbar_gridspec = True

        def execute(self, fig):
            runs.append(len(fig.axes))

    fig, ax = plt.subplots(figsize=(3, 2))
    fig.set_layout_engine(Engine())
    fig.canvas.draw()
    runs.clear()
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert runs == [1]


def test_an_axes_grid1_size_of_the_program_gets_a_renderer_and_pickles():
    renderers = []

    @_kept
    class Half(axes_size.Scaled):
        def get_size(self, renderer):
            renderers.append(type(renderer).__name__)
            return super().get_size(renderer)

    fig, ax = plt.subplots(figsize=(3, 2))
    make_axes_locatable(ax).append_axes("right", size=Half(0.5), pad=0.1)
    fig.canvas.draw()
    assert renderers and set(renderers) == {"RendererAgg"}
    copy = pickle.loads(pickle.dumps(fig))
    assert len(copy.axes) == 2


def test_axis_transforms_of_the_class_are_taken_anew_at_a_clear():
    import pltrs.transforms as mtransforms

    class Shifted(maxes.Axes):
        shift = 0.0

        def get_xaxis_transform(self, which='grid'):
            return (super().get_xaxis_transform(which)
                    + mtransforms.Affine2D().translate(0, self.shift))

    fig = plt.figure(figsize=(3, 2))
    ax = fig.add_subplot(axes_class=Shifted)
    ax.plot([0, 1], [0, 1])
    fig.canvas.draw()
    ax.shift = 20.0
    ax.cla()
    ax.plot([0, 1], [0, 1])
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    marks = [tick.tick1line.get_window_extent(renderer).y0
             for tick in ax.xaxis.get_major_ticks()[:2]]
    assert marks == pytest.approx([59.56944444444444] * 2)


def test_an_affine_class_mapping_points_itself_places_a_text():
    import pltrs.transforms as mtransforms
    mapped = []

    class Placing(mtransforms.Affine2D):
        def transform_affine(self, values):
            mapped.append(np.shape(values))
            return np.asarray(super().transform_affine(values)) + [0, 10]

    fig, ax = plt.subplots(figsize=(3, 2))
    text = ax.text(0, 0, "placed", transform=Placing().translate(100, 80))
    line, = ax.plot([0, 1], [0, 1], transform=Placing().scale(50))
    fig.canvas.draw()
    # The text's point goes through the method, the line through the matrix.
    assert mapped == [(2,)]
    renderer = fig.canvas.get_renderer()
    assert text.get_window_extent(renderer).y0 == pytest.approx(86.66341145833333)
