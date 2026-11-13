"""
What the generated family ``overrides`` (``cargo xtask diff --generated
overrides``: matplotlib's classes subclassed with one member overridden,
as the libraries of the census ``spec/diff/overrides_census.json`` do)
showed pltrs not asking, or asking otherwise than matplotlib 3.11, each
with a small subclass of the kind libraries write: a transform mapping a
line's points in one call when the line is measured; a locator's own
``tick_values``; an Axes class building its own spines, asked by
matplotlib's ``__init__`` whatever its projection's name, and kept by a
pickle; an Axes class clearing or gridding its own way, or a polar one
with its own constructor, made by its constructor with ``add_subplot``'s
arguments; an artist measuring itself in a tight print, an Axes with
``bbox_extra_artists``; a norm called with its values alone; pyplot
handing ``data`` only when given; a scale of the program's pickled with
its figure; the hit tests of figures, axes and bare artists. Every
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
import pltrs.scale as mscale
import pltrs.ticker as mticker
import pltrs.transforms as mtransforms
from pltrs.artist import Artist
from pltrs.backend_bases import MouseEvent
from pltrs.colors import Normalize
from pltrs.projections import PolarAxes
from pltrs.spines import Spine
from pltrs.text import Text

#: The module the classes pickled by name live in.
_MODULE = types.ModuleType("gen5_classes")
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


class _Shift(mtransforms.Transform):
    """A non-affine transform recording the shapes it maps."""

    input_dims = output_dims = 2
    is_separable = False
    has_inverse = True

    def __init__(self):
        super().__init__()
        self.shapes = []

    def transform_non_affine(self, values):
        self.shapes.append(np.shape(values))
        return np.asarray(values, dtype=float) + [0, 0.1]

    def inverted(self):
        return _Shift()


def test_a_transform_maps_a_line_measured_in_one_call():
    fig, ax = plt.subplots(figsize=(3, 2))
    shift = _Shift()
    line, = ax.plot(X, np.sin(X), transform=shift + ax.transData)
    fig.canvas.draw()
    shift.shapes.clear()
    line.get_window_extent(fig.canvas.get_renderer())
    line.get_tightbbox(fig.canvas.get_renderer())
    assert shift.shapes == [(7, 2), (7, 2)]


def test_a_transform_is_mapped_through_its_non_affine_part():
    calls = []

    class Counted(_Shift):
        def transform(self, values):
            calls.append("transform")
            return super().transform(values)

    fig, ax = plt.subplots(figsize=(3, 2))
    line, = ax.plot(X, np.sin(X), transform=Counted() + ax.transData)
    fig.canvas.draw()
    line.get_window_extent(fig.canvas.get_renderer())
    assert calls == []


def test_a_locator_of_the_program_ticks_through_its_tick_values():
    asked = []

    class Halved(mticker.MaxNLocator):
        def tick_values(self, vmin, vmax):
            asked.append((round(vmin, 6), round(vmax, 6)))
            return super().tick_values(vmin, vmax)[::2]

    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot(X, np.sin(X))
    ax.set_xlim(0, 4)
    ax.xaxis.set_major_locator(Halved(4))
    fig.canvas.draw()
    assert asked[0] == (0.0, 4.0)
    assert list(ax.get_xticks()) == [0.0, 2.0, 4.0]


def test_an_axes_class_making_its_spines_is_asked_whatever_its_name():
    made = []

    @_kept
    class Thick(Spine):
        pass

    @_kept
    class Host(maxes.Axes):
        def _gen_axes_spines(self, locations=None, offset=0.0, units='inches'):
            made.append("spines")
            return {side: Thick.linear_spine(self, side)
                    for side in ('left', 'right', 'bottom', 'top')}

    fig = plt.figure(figsize=(3, 2))
    ax = fig.add_subplot(axes_class=Host)
    assert made == ["spines"]
    assert type(ax.spines['bottom']).__name__ == "Thick"
    fig2 = pickle.loads(pickle.dumps(fig))
    # matplotlib's unpickled Axes keeps its spines and makes none.
    assert made == ["spines"]
    assert type(fig2.axes[0].spines['left']).__name__ == "Thick"


def test_an_axes_class_clearing_its_own_way_is_made_by_its_constructor():
    calls = []

    class Cleared(maxes.Axes):
        def __init__(self, fig, *args, **kwargs):
            calls.append(("init", args))
            super().__init__(fig, *args, **kwargs)

        def clear(self):
            calls.append(("clear",))
            super().clear()

        def grid(self, visible=None, which='major', axis='both', **kwargs):
            calls.append(("grid", visible, which, axis))
            super().grid(visible, which, axis, **kwargs)

    fig = plt.figure()
    fig.add_subplot(axes_class=Cleared)
    fig.add_subplot(212, axes_class=Cleared)
    assert calls == [("init", (1, 1, 1)), ("clear",), ("grid", False, 'major', 'both'),
                     ("grid", False, 'major', 'both'),
                     ("init", (2, 1, 2)), ("clear",), ("grid", False, 'major', 'both'),
                     ("grid", False, 'major', 'both')]


def test_a_polar_class_with_a_constructor_of_its_own_is_made_by_it():
    made = []

    class Radar(PolarAxes):
        def __init__(self, *args, **kwargs):
            made.append(len(args))
            super().__init__(*args, **kwargs)

    fig = plt.figure()
    ax = fig.add_subplot(axes_class=Radar)
    assert made == [4]
    assert ax.name == "polar"


def test_an_artist_measuring_itself_is_measured_in_a_tight_print():
    measured = []

    class Boxed(Artist):
        def get_tightbbox(self, renderer=None):
            measured.append(type(renderer).__name__)
            return mtransforms.Bbox([[-20, -20], [10, 10]])

    fig, ax = plt.subplots(figsize=(3, 2))
    fig.add_artist(Boxed())
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png", bbox_inches="tight")
    assert measured
    buffer.seek(0)
    width, height = plt.imread(buffer).shape[1::-1]
    # The box reaches 20 pixels beyond the figure's corner.
    assert (width, height) > (300, 200)


def test_a_text_clipped_to_its_axes_is_left_out_of_their_tight_box():
    measured = []

    class Measured(Text):
        def get_window_extent(self, renderer=None, dpi=None):
            measured.append("extent")
            return super().get_window_extent(renderer, dpi)

    fig, ax = plt.subplots(figsize=(3, 2))
    ax.add_artist(Measured(0.5, 0.5, "text"))
    fig.canvas.draw()
    measured.clear()
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert measured == []


def test_an_axes_measuring_itself_gets_the_extra_artists():
    asked = []

    class Measured(maxes.Axes):
        def get_tightbbox(self, renderer=None, *args, **kwargs):
            asked.append(kwargs)
            return super().get_tightbbox(renderer, *args, **kwargs)

    fig = plt.figure(figsize=(3, 2))
    fig.add_subplot(axes_class=Measured)
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert asked == [{"bbox_extra_artists": None}]


def test_a_norm_is_called_with_its_values_alone():
    keywords = []

    class Squared(Normalize):
        def __call__(self, value, *args, **kwargs):
            keywords.append((args, kwargs))
            return super().__call__(value, *args, **kwargs) ** 2

    fig, ax = plt.subplots()
    ax.imshow(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)), norm=Squared())
    fig.canvas.draw()
    assert keywords and set(map(repr, keywords)) == {"((), {})"}


def test_pyplot_hands_data_only_when_given():
    given = []

    class Plotting(maxes.Axes):
        def plot(self, *args, **kwargs):
            given.append(sorted(kwargs))
            return super().plot(*args, **kwargs)

        def scatter(self, *args, **kwargs):
            given.append("data" in kwargs)
            return super().scatter(*args, **kwargs)

    plt.figure().add_subplot(axes_class=Plotting)
    plt.plot([1, 2, 3])
    plt.scatter([1, 2], [3, 4])
    plt.plot('a', 'b', 'r', data={'a': [1, 2], 'b': [3, 4]})
    assert given == [["scalex", "scaley"], False, ["data", "scalex", "scaley"]]


@pytest.mark.filterwarnings("ignore:The scale 'Doubled' uses an 'axis' parameter")
def test_a_figure_with_a_scale_of_the_program_pickles():
    @_kept
    class Doubled(mscale.LinearScale):
        name = "gen5-doubled"

    mscale.register_scale(Doubled)
    fig, ax = plt.subplots()
    ax.plot(X + 1, X)
    ax.set_xscale("gen5-doubled")
    copy = pickle.loads(pickle.dumps(fig))
    assert copy.axes[0].get_xscale() == "gen5-doubled"
    assert copy.axes[0].get_xlim() == pytest.approx(ax.get_xlim())


def test_hit_tests_of_figures_axes_and_bare_artists(caplog):
    fig, ax = plt.subplots(figsize=(3, 2))
    event = MouseEvent("button_press_event", fig.canvas, 150, 100)
    inside, _ = fig.contains(event)
    assert inside and isinstance(inside, np.bool_)
    inside, _ = ax.xaxis.contains(event)
    assert not inside and isinstance(inside, np.bool_)

    class Bare(Artist):
        pass

    bare = Bare()
    ax.add_artist(bare)
    with caplog.at_level("WARNING"):
        assert bare.contains(event) == (False, {})
    assert "'Bare' needs 'contains' method" in caplog.text


def test_axes_made_by_their_constructor_share_the_figure_grid():
    class Own(maxes.Axes):
        def __init__(self, fig, *args, **kwargs):
            super().__init__(fig, *args, **kwargs)

    fig = plt.figure(layout="constrained")
    first = fig.add_subplot(2, 2, 1, axes_class=Own)
    second = fig.add_subplot(2, 2, 2, axes_class=Own)
    assert first.get_subplotspec().get_gridspec() is second.get_subplotspec().get_gridspec()
