"""
What the generated family ``patches`` (``cargo xtask diff --generated
patches``: matplotlib's methods and functions replaced by assignment, as
``spec/diff/patches_census.json`` finds libraries replacing them) and the
probe of the setters matplotlib's artist constructors call showed pltrs
not asking, or asking otherwise than matplotlib 3.11: a module's
``scale_factory`` replaced (proplot, ultraplot), an axis object given an
``_update_ticks`` (ultraplot's tick cache), a title given an ``update``
(ultraplot's labels), the setters of a class of the program's that
matplotlib's constructors call (``Polygon.set_xy``, ``Line2D``'s), and
the hooks of a linear spine of the program's in a plain Axes. Every
expected value was recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
import pltrs.scale as mscale
from pltrs import collections, image, lines, patches, path, spines


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


@pytest.fixture
def factory_calls():
    """``scale.scale_factory`` replaced by one recording its calls, which
    lower-cases names as ultraplot's does; restored afterwards."""
    original = mscale.scale_factory
    calls = []

    def factory(scale, axis, *args, **kwargs):
        calls.append((scale, axis.axis_name))
        if isinstance(scale, mscale.ScaleBase):
            return scale
        return original(scale.lower(), axis, *args, **kwargs)

    mscale.scale_factory = factory
    yield calls
    mscale.scale_factory = original


def test_a_replaced_scale_factory_makes_the_scales_matplotlib_makes(factory_calls):
    fig, ax = plt.subplots()
    assert factory_calls == [("linear", "x"), ("linear", "y")]
    twin = ax.twinx()
    assert factory_calls[2:] == [("linear", "x"), ("linear", "y")]
    del factory_calls[:]
    ax.set_xscale("LOG")
    # Each shared axis makes a scale of its own.
    assert factory_calls == [("LOG", "x"), ("LOG", "x")]
    assert ax.get_xscale() == twin.get_xscale() == "log"
    assert ax.xaxis._scale is not twin.xaxis._scale
    del factory_calls[:]
    ax.cla()
    assert factory_calls == [("linear", "x"), ("linear", "x"), ("linear", "y")]


def test_an_axis_scale_is_the_same_object_until_it_changes():
    fig, ax = plt.subplots()
    scale = ax.xaxis._scale
    assert ax.xaxis._scale is scale
    ax.set_xscale("log")
    assert ax.xaxis._scale is not scale


def test_an_update_ticks_given_to_an_axis_is_called_and_chooses_the_ticks():
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 10], [0, 1])
    original = ax.xaxis._update_ticks
    calls = []

    def every_other():
        calls.append(1)
        return original()[::2]

    ax.xaxis._update_ticks = every_other
    fig.canvas.draw()
    assert len(calls) == 3
    drawn = [tick.get_loc() for tick in every_other()]
    assert drawn == [0.0, 5.0, 10.0]
    # Called directly, outside a draw, it is called once.
    del calls[:]
    ax.xaxis._update_ticks()
    assert len(calls) == 1


def test_an_update_given_to_a_title_is_called_by_set_title():
    fig, ax = plt.subplots()
    seen = []
    original = ax.title.update
    ax.title.update = lambda props: (seen.append(dict(props)), original(props))[1]
    ax.set_title("t", color="b")
    assert seen == [{"fontsize": "large", "fontweight": "normal",
                     "verticalalignment": "baseline", "horizontalalignment": "center"}]
    assert ax.title.get_color() == "b"


def test_the_constructor_sets_a_polygon_through_its_class():
    class Halved(patches.Polygon):
        def set_xy(self, xy):
            super().set_xy(np.asarray(xy, float) * 0.5 + 0.25)

    polygon = Halved([(0, 0), (1, 0), (1, 1)])
    assert polygon.get_xy().tolist() == [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.25]]


def test_the_constructor_sets_a_line_through_its_class():
    calls = []

    class Red(lines.Line2D):
        def set_color(self, color):
            calls.append(color)
            super().set_color("r")

        def set_xdata(self, x):
            calls.append("x")
            super().set_xdata(np.asarray(x) * 0.5)

    line = Red([0, 1], [0, 1])
    assert calls == ["C0", "x"]
    assert line.get_color() == "r"
    assert line.get_xdata().tolist() == [0.0, 0.5]


def test_the_constructor_sets_an_image_through_its_class():
    calls = []

    class Smooth(image.AxesImage):
        def set_interpolation(self, s):
            calls.append(s)
            super().set_interpolation("bilinear")

    fig, ax = plt.subplots()
    img = Smooth(ax)
    assert calls == [None]
    assert img.get_interpolation() == "bilinear"


def test_a_linear_spine_of_the_program_is_laid_out_by_its_hooks():
    calls = []

    class Short(spines.Spine):
        def _adjust_location(self):
            calls.append("adjust")
            super()._adjust_location()
            self._path.vertices[:, 0] = [0.25, 0.75]

    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 1], [0, 1])
    ax.spines["bottom"] = Short.linear_spine(ax, "bottom")
    fig.canvas.draw()
    # (matplotlib's draw adjusts it twice: once where the axis label is
    # placed beside it, which pltrs measures from the path it drew.)
    assert calls[0] == "adjust"
    extent = ax.spines["bottom"].get_window_extent()
    trans = ax.transData.transform
    assert extent.x0 == pytest.approx(trans((0.25, 0))[0], abs=1)
    assert extent.x1 == pytest.approx(trans((0.75, 0))[0], abs=1)


def test_a_removed_anchored_box_of_an_aux_box_is_not_laid_out():
    from pltrs import offsetbox
    fig, ax = plt.subplots()
    box = offsetbox.AuxTransformBox(ax.transData)
    box.add_artist(patches.Rectangle((0, 0), 1, 0.5))
    anchored = offsetbox.AnchoredOffsetbox("upper left", child=box)
    ax.add_artist(anchored)
    fig.canvas.draw()
    anchored.remove()
    fig.canvas.draw()


class _Measuring(collections.PathCollection):
    """A collection measuring itself, picklable by name."""

    def get_datalim(self, transData):
        return super().get_datalim(transData)


def test_a_figure_whose_collection_measures_itself_pickles():
    import io
    import pickle

    fig, ax = plt.subplots()
    ax.add_collection(_Measuring([path.Path.unit_circle()], offsets=[(0, 0)]))
    pickle.loads(pickle.dumps(fig)).savefig(io.BytesIO())


def test_an_axes_overriding_get_data_ratio_refuses_a_twin():
    from pltrs import axes

    class Ratio(axes.Axes):
        def get_data_ratio(self):
            return super().get_data_ratio()

    fig = plt.figure()
    ax = fig.add_subplot(axes_class=Ratio)
    with pytest.raises(ValueError, match="override 'get_data_ratio'"):
        ax.twinx()
    assert len(fig.axes) == 2


def test_offset_boxes_take_their_class_zorder_through_no_setter():
    from pltrs import offsetbox
    calls = []

    class Box(offsetbox.AnchoredOffsetbox):
        def set_zorder(self, level):
            calls.append(level)
            super().set_zorder(level)

    box = Box(loc=1, child=offsetbox.TextArea("v"))
    assert (calls, box.get_zorder()) == ([], 5)
    assert offsetbox.AnnotationBbox(offsetbox.TextArea("t"), (0.5, 0.5)).get_zorder() == 3
