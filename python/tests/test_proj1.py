"""
A projection a program defines with an axis of its own (task PROJ1): an
`~.axes.Axes` subclass with a third `.Axis` under a name of its own, tick,
spine and transform classes of its own, as mpltern's ternary Axes has
three. No library is imported: the projection is defined here, a
rectilinear Axes with an axis along a parabola, and what it does was
recorded from matplotlib 3.11, under which this file passes too.

pltrs (Rust) holds an x and a y axis per Axes. The third axis is
matplotlib's own `.Axis` in Python (``pltrs._held_axis``), which pltrs
(Rust) draws and measures in the place of the Axes' axes; see
"Projections of programs" in ``python/DESIGN.md``.
"""

import collections
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
import pltrs.transforms as mtransforms
from pltrs import cbook
from pltrs.axes import Axes
from pltrs.axis import Axis, XAxis, XTick, YAxis
from pltrs.lines import Line2D
from pltrs.path import Path
from pltrs.projections import register_projection
from pltrs.spines import Spine
from pltrs.ticker import MultipleLocator

#: The calls of the projection's own methods, in order.
CALLS = []


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        del CALLS[:]
        yield
    plt.close("all")


class CurveTransform(mtransforms.Transform):
    """``(s, p)`` to Axes fractions: *s* along a parabola, *p* across it."""

    input_dims = output_dims = 2

    def transform_non_affine(self, values):
        values = np.asarray(values, float)
        s, p = values[:, 0], values[:, 1]
        return np.column_stack([0.1 + 0.8 * s, 0.2 + 0.6 * s ** 2 + (p - 0.5) * 0.1])

    def inverted(self):
        return InvertedCurveTransform()


class InvertedCurveTransform(mtransforms.Transform):
    input_dims = output_dims = 2

    def transform_non_affine(self, values):
        values = np.asarray(values, float)
        s = (values[:, 0] - 0.1) / 0.8
        return np.column_stack([s, (values[:, 1] - 0.2 - 0.6 * s ** 2) / 0.1 + 0.5])

    def inverted(self):
        return CurveTransform()


class CurveLabelShift(mtransforms.Affine2DBase):
    """A shift of *pad_points* across the chord of the curve, computed
    from where the Axes is when it is asked (as mpltern's shift of a tick
    label away from its corner is)."""

    def __init__(self, axes, pad_points, sign):
        super().__init__()
        self.axes, self.pad_points, self.sign = axes, pad_points, sign

    def get_matrix(self):
        start, end = self.axes._caxis_transform.transform([(0, 0.5), (1, 0.5)])
        along = (end - start) / np.linalg.norm(end - start)
        dpi = self.axes.get_figure(root=True).dpi
        x, y = self.sign * np.array([-along[1], along[0]]) * self.pad_points / 72 * dpi
        return np.array([[1.0, 0.0, x], [0.0, 1.0, y], [0.0, 0.0, 1.0]])


class CurveTick(XTick):
    """A tick of the curve axis: its lines through the axis' transform,
    its mark and label turned along the curve."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        transform = self.axes.get_caxis_transform()
        for line in (self.tick1line, self.tick2line, self.gridline):
            line.set_transform(transform)

    def _get_text1_transform(self):
        return self.axes.get_caxis_text1_transform(self._pad)

    def _get_text2_transform(self):
        return self.axes.get_caxis_text2_transform(self._pad)

    def update_position(self, loc):
        CALLS.append(("update_position", float(loc)))
        super().update_position(loc)
        ahead = self.axes._caxis_transform.transform([(loc, 0.5), (loc + 1e-3, 0.5)])
        angle = np.arctan2(*(ahead[1] - ahead[0])[::-1])
        self.tick1line._marker._transform = mtransforms.Affine2D().rotate(angle)
        self.label1.set_rotation(np.rad2deg(angle))
        self.label1.set_rotation_mode("anchor")


class CurveAxis(XAxis):
    """The axis along the curve, with limits of its own."""

    axis_name = "c"

    def _init(self):
        super()._init()
        self.label.set_transform(self.axes.transAxes)
        self.label_position = "curve"

    def _get_tick(self, major):
        tick_kw = self._major_tick_kw if major else self._minor_tick_kw
        return CurveTick(self.axes, 0, major=major, **tick_kw)

    def _copy_tick_props(self, src, dest):
        super()._copy_tick_props(src, dest)
        CALLS.append(("_copy_tick_props",))

    def get_view_interval(self):
        return self.axes.viewCLim.intervalx

    def set_view_interval(self, vmin, vmax, ignore=False):
        self.axes.viewCLim.intervalx = vmin, vmax

    def get_data_interval(self):
        return self.axes.viewCLim.intervalx

    def get_minpos(self):
        return 1e-300

    def get_tick_space(self):
        return 5

    def _update_label_position(self, renderer):
        CALLS.append(("_update_label_position",))
        boxes, _ = self._get_ticklabel_bboxes(self._update_ticks(), renderer)
        top = max(box.y1 for box in boxes)
        _, y = self.axes.transAxes.inverted().transform((0, top))
        self.label.set_position((0.3, y + 0.05))

    def _update_offset_text_position(self, bboxes, bboxes2):
        CALLS.append(("_update_offset_text_position", len(bboxes), len(bboxes2)))
        self.offsetText.set_position((1, self.axes.bbox.ymax))


class CurveSpine(Spine):
    """A spine along the curve, on one side of it, over the axis' limits."""

    def __init__(self, axes, spine_type, path, **kwargs):
        super().__init__(axes, spine_type, path, **kwargs)
        self.set_transform(axes.get_caxis_transform())

    def _adjust_location(self):
        CALLS.append(("_adjust_location", self.spine_type))
        self._path.vertices[:, 0] = self.axes.viewCLim.intervalx
        self._path.vertices[:, 1] = 0.0 if self.spine_type == "clow" else 1.0

    def get_spine_transform(self):
        return self.get_transform()


class CurveAxes(Axes):
    """A rectilinear Axes with a third axis along a parabola."""

    name = "proj1_curve"
    _axis_names = ("x", "y", "c")
    _shared_axes = {name: cbook.Grouper() for name in _axis_names}

    def __init__(self, *args, **kwargs):
        # matplotlib's clear reads the Axes each axis is shared with.
        self._sharec = None
        super().__init__(*args, **kwargs)

    @property
    def callbacks(self):
        # matplotlib's registry knows the x, y and z limits only.
        if "_curve_callbacks" not in self.__dict__:
            self._curve_callbacks = cbook.CallbackRegistry(
                signals=[f"{name}lim_changed" for name in self._axis_names])
        return self._curve_callbacks

    @callbacks.setter
    def callbacks(self, registry):
        # A clear starts over.
        self.__dict__.pop("_curve_callbacks", None)

    def set_figure(self, fig):
        self.viewCLim = mtransforms.Bbox.unit()
        super().set_figure(fig)

    def _set_lim_and_transforms(self):
        super()._set_lim_and_transforms()
        limits = mtransforms.BboxTransformFrom(
            mtransforms.TransformedBbox(self.viewCLim, self.transScale))
        self._caxis_transform = limits + CurveTransform() + self.transAxes

    def _init_axis(self):
        super()._init_axis()
        self.caxis = CurveAxis(self)
        self.spines["clow"].register_axis(self.caxis)

    def _gen_axes_spines(self, locations=None, offset=0.0, units="inches"):
        spines = super()._gen_axes_spines()
        for name in ("clow", "chigh"):
            path = Path([(0.0, 0.0), (1.0, 0.0)], _interpolation_steps=16)
            spines[name] = CurveSpine(self, name, path)
        return spines

    def get_caxis_transform(self, which="grid"):
        return self._caxis_transform

    def get_caxis_text1_transform(self, pad):
        return self._caxis_transform + CurveLabelShift(self, pad, -1), "top", "center"

    def get_caxis_text2_transform(self, pad):
        return self._caxis_transform + CurveLabelShift(self, pad, +1), "bottom", "center"

    def clear(self):
        CALLS.append(("clear",))
        self.viewCLim.intervalx = 0.0, 1.0
        super().clear()

    def autoscale_view(self, tight=None, scalex=True, scaley=True, scalec=True):
        # matplotlib asks for every axis of ``_axis_names``.
        super().autoscale_view(tight=tight, scalex=scalex, scaley=scaley)

    def get_clim(self):
        return tuple(self.viewCLim.intervalx)

    def set_clim(self, low, high):
        return self.caxis._set_lim(low, high, auto=None)


register_projection(CurveAxes)


class LiftedTitleAxes(CurveAxes):
    """A `CurveAxes` placing its titles its own way: above the label of
    its third axis, which it measures for it (as mpltern's Axes lifts its
    titles above the tick labels of its three axes)."""

    name = "proj1_lifted_title"

    def _update_title_position(self, renderer):
        CALLS.append(("_update_title_position",))
        top = self.caxis.get_tightbbox(renderer).y1
        _, y = self.transAxes.inverted().transform((0, top))
        for title in (self.title, self._left_title, self._right_title):
            title.set_position((title.get_position()[0], y + 0.25))


register_projection(LiftedTitleAxes)


def _curve_axes():
    """A figure of 500 by 400 pixels with one `CurveAxes`."""
    fig = plt.figure(figsize=(5, 4), dpi=100)
    return fig, fig.add_subplot(projection="proj1_curve")


def _plotted():
    """`_curve_axes` with a line, limits, a label, a locator, a grid and
    tick parameters on the third axis."""
    fig, ax = _curve_axes()
    ax.plot([0, 1], [0, 1])
    ax.set_xlim(0, 2)
    ax.caxis.set_label_text("along the curve", color="C1")
    ax.set_clim(2, 5)
    ax.caxis.set_major_locator(MultipleLocator(0.5))
    ax.caxis.grid(True, color="C2")
    ax.caxis.set_tick_params(direction="inout", length=8, labelsize=9)
    return fig, ax


def _names(calls):
    """How often each of the projection's methods was called."""
    return dict(collections.Counter(call[0] for call in calls))


def test_the_axes_has_its_third_axis_among_its_axes():
    fig, ax = _curve_axes()
    # Only the clear of matplotlib's ``__init__`` ran of the projection's
    # own: no tick of the third axis is made until one is asked for.
    assert CALLS == [("clear",)]
    assert [type(child).__name__ for child in ax.get_children()] == [
        "Spine", "Spine", "Spine", "Spine", "CurveSpine", "CurveSpine",
        "XAxis", "YAxis", "CurveAxis", "Text", "Text", "Text", "Rectangle"]
    assert {name: type(axis) for name, axis in ax._axis_map.items()} == {
        "x": XAxis, "y": YAxis, "c": CurveAxis}
    assert list(ax.spines) == ["left", "right", "bottom", "top", "clow", "chigh"]
    axis = ax.caxis
    assert isinstance(axis, Axis) and axis.axes is ax and axis.get_figure() is fig
    assert (axis.get_zorder(), ax.xaxis.get_zorder(), axis.get_visible()) == (1.5, 1.5, True)
    assert type(axis.label).__name__ == "Text"
    assert (axis.label.get_position(), axis.labelpad) == ((0.5, 0), 4.0)
    assert (axis.label_position, axis.get_label_position()) == ("curve", "curve")
    assert [type(tick) for tick in axis.majorTicks] == [CurveTick]
    assert (axis.get_scale(), axis.get_tick_space()) == ("linear", 5)
    assert [type(helper).__name__ for helper in (
        axis.get_major_locator(), axis.get_major_formatter(),
        axis.get_minor_locator())] == ["AutoLocator", "ScalarFormatter", "NullLocator"]
    assert axis.isDefault_majloc and axis.isDefault_label


def test_limits_of_the_third_axis_tell_the_registry_of_the_class():
    fig, ax = _curve_axes()
    ax.plot([0, 1], [0, 1])
    seen = []
    ax.callbacks.connect("clim_changed", lambda axes: seen.append(axes.get_clim()))
    ax.callbacks.connect("xlim_changed", lambda axes: seen.append(("x", axes.get_xlim())))
    # The pending autoscaling tells first, as matplotlib's `set_xlim` reads
    # the view limits it is about to set.
    ax.set_xlim(0, 2)
    assert ax.set_clim(2, 5) == (2.0, 5.0)
    assert seen == [("x", (-0.05, 1.05)), ("x", (0.0, 2.0)), (2.0, 5.0)]
    assert ax.caxis.get_view_interval().tolist() == [2.0, 5.0]
    assert not ax.caxis.get_inverted()
    with pytest.raises(ValueError, match="'zlim_changed' is not a valid value for signal"):
        ax.callbacks.connect("zlim_changed", print)


def test_ticks_labels_and_boxes_are_where_matplotlib_puts_them():
    fig, ax = _plotted()
    axis = ax.caxis
    assert axis.get_ticklocs().tolist() == [1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5]
    assert [text.get_text() for text in axis.get_ticklabels()] == [
        "1.5", "2.0", "2.5", "3.0", "3.5", "4.0", "4.5", "5.0", "5.5"]
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    ticks = axis.get_major_ticks()
    recorded = {
        2.0: ([85.916158, 68.750551, 105.918642, 81.254525], 0.0114),
        3.5: ([242.327345, 111.594775, 265.907155, 132.574285], 30.8088),
        5.0: ([399.490507, 250.358163, 421.919698, 273.714849], 50.0166),
    }
    for tick in ticks:
        if tick.get_loc() in recorded:
            extents, rotation = recorded[tick.get_loc()]
            assert tick.label1.get_window_extent(renderer).extents == pytest.approx(
                extents, abs=1e-4)
            assert tick.label1.get_rotation() == pytest.approx(rotation, abs=1e-4)
        assert tick.tick1line.get_marker() == "|"
        assert tick.tick1line.get_markersize() == 8.0
        assert tick.gridline.get_visible() and tick.gridline.get_color() == "C2"
    assert axis.label.get_window_extent(renderer).extents == pytest.approx(
        [123.25, 274.77826, 234.25, 289.114849], abs=1e-4)
    assert axis.label.get_position() == pytest.approx((0.3, 0.7958274319340686))
    assert axis.offsetText.get_position() == (1, 352.0)
    assert axis.offsetText.get_text() == ""
    assert axis.get_tightbbox(renderer).extents == pytest.approx(
        [85.916158, 68.750551, 421.919698, 289.114849], abs=1e-4)
    assert ax.get_tightbbox(renderer).extents == pytest.approx(
        [30.777778, 20.388889, 465.5, 352.0], abs=1e-4)
    assert fig.get_tightbbox(renderer).extents == pytest.approx(
        [0.307778, 0.203889, 4.655, 3.52], abs=1e-5)
    # The spines follow the limits of their axis, through its transform.
    assert ax.spines["clow"].get_window_extent(renderer).extents == pytest.approx(
        [101.25, 90.2, 411.25, 275.0], abs=1e-4)
    assert ax.spines["chigh"].get_path().vertices.tolist() == [[2.0, 1.0], [5.0, 1.0]]
    assert axis.get_tick_params() == {
        "length": 8, "direction": "inout", "gridOn": True, "grid_color": "C2",
        "labelsize": 9}
    assert not axis.stale


def test_the_methods_of_the_classes_run_once_per_draw():
    fig, ax = _plotted()
    ax.caxis.get_ticklabels()
    del CALLS[:]
    fig.canvas.draw()
    # One tick update of the draw, one of the label's placement: twice
    # the nine ticks.
    assert _names(CALLS) == {
        "_adjust_location": 2, "update_position": 18,
        "_update_label_position": 1, "_update_offset_text_position": 1}
    assert [call for call in CALLS if call[0] == "_adjust_location"] == [
        ("_adjust_location", "clow"), ("_adjust_location", "chigh")]
    # Neither the axis nor the spines are drawn with the axes off.
    ax.set_axis_off()
    del CALLS[:]
    fig.canvas.draw()
    assert CALLS == []
    # A hidden axis draws nothing, and the spines still do.
    ax.set_axis_on()
    ax.caxis.set_visible(False)
    del CALLS[:]
    fig.canvas.draw()
    assert _names(CALLS) == {"_adjust_location": 2}


def test_layouts_make_room_for_what_the_classes_place():
    # The third axis is measured by its own `get_tightbbox` and the titles
    # are placed by the class where matplotlib's `Axes.get_tightbbox`
    # places them, before a layout engine measures the Axes.
    positions = {}
    for layout in ("constrained", "tight", None):
        fig = plt.figure(figsize=(5, 4), dpi=100, layout=layout)
        ax = fig.add_subplot(projection="proj1_lifted_title")
        ax.set_title("lifted")
        ax.caxis.set_label_text("along the curve")
        ax.set_clim(2, 5)
        fig.canvas.draw()
        positions[layout] = ax.get_position().bounds
        renderer = fig.canvas.get_renderer()
        if layout is None:
            assert ax.title.get_position()[1] == pytest.approx(RECORDED_TITLE_Y)
            assert ax.get_tightbbox(renderer).extents == pytest.approx(
                RECORDED_TIGHT, abs=1e-3)
            assert ax.get_tightbbox(renderer, for_layout_only=True).extents == (
                pytest.approx(RECORDED_TIGHT_FOR_LAYOUT, abs=1e-3))
    for layout, bounds in positions.items():
        assert bounds == pytest.approx(RECORDED_POSITIONS[layout], abs=1e-4), layout


#: What `test_layouts_make_room_for_what_the_classes_place` reads under
#: matplotlib.
RECORDED_TITLE_Y = 1.0340814137956083
RECORDED_TIGHT = [30.777778, 20.388889, 461.0, 383.830409]
RECORDED_TIGHT_FOR_LAYOUT = [30.777778, 20.388889, 461.0, 383.830409]
RECORDED_POSITIONS = {
    "constrained": (0.0717784, 0.0694453, 0.8978876, 0.8375636),
    "tight": (0.0934444, 0.0965278, 0.8545556, 0.7863962),
    None: (0.125, 0.11, 0.775, 0.77),
}


def test_a_clear_starts_the_third_axis_over():
    fig, ax = _plotted()
    fig.canvas.draw()
    del CALLS[:]
    ax.clear()
    assert [call for call in CALLS if call[0] == "clear"] == [("clear",)]
    assert ax.get_clim() == (0.0, 1.0)
    assert ax.caxis.get_label_text() == ""
    assert type(ax.caxis.get_major_locator()).__name__ == "AutoLocator"
    # The tick parameters stay, but whether the grid is on.
    assert ax.caxis.get_tick_params() == {
        "length": 8, "direction": "inout", "gridOn": False, "grid_color": "C2",
        "labelsize": 9}
    assert type(ax.caxis) is CurveAxis and ax._axis_map["c"] is ax.caxis


def test_the_figure_shows_the_axis_the_grid_and_the_spines():
    fig, ax = _plotted()
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba()).astype(int)
    red, green, blue = pixels[..., 0], pixels[..., 1], pixels[..., 2]
    # The gridlines (C2, green), the label (C1, orange) and the rest in
    # black: tick marks, tick labels, the six spines.
    counts = {
        "green": int(((green > red + 40) & (green > blue + 40)).sum()),
        "orange": int(((red > 200) & (blue < 100) & (green > 80) & (green < 180)).sum()),
        "black": int(((red < 60) & (green < 60) & (blue < 60)).sum()),
    }
    assert counts == RECORDED_PIXELS


#: What `test_the_figure_shows_the_axis_the_grid_and_the_spines` counts
#: under matplotlib.
RECORDED_PIXELS = {"green": 158, "orange": 238, "black": 2885}


def test_a_pickled_figure_keeps_the_third_axis_and_draws_the_same():
    fig, ax = _plotted()
    fig.canvas.draw()
    before = np.asarray(fig.canvas.buffer_rgba()).copy()
    copy = pickle.loads(pickle.dumps(fig))
    other = copy.axes[0]
    assert type(other.caxis) is CurveAxis and other.caxis.axes is other
    assert other.get_clim() == (2.0, 5.0)
    assert list(other.spines) == ["left", "right", "bottom", "top", "clow", "chigh"]
    assert other.caxis.get_label_text() == "along the curve"
    copy.canvas.draw()
    assert np.array_equal(np.asarray(copy.canvas.buffer_rgba()), before)


def test_a_hidden_axis_stays_hidden_through_a_clear():
    # A projection hiding the x and y axis in ``_init_axis`` (mpltern's)
    # relies on it: matplotlib's clear leaves an axis its visibility.
    fig, ax = plt.subplots()
    ax.xaxis.set_visible(False)
    ax.clear()
    assert (ax.xaxis.get_visible(), ax.yaxis.get_visible()) == (False, True)


def test_a_line_draws_its_marker_through_the_transform_set_on_its_style():
    # How matplotlib's polar ticks and mpltern's turn their marks.
    fig, ax = plt.subplots(figsize=(2, 2), dpi=100)
    ax.set_axis_off()
    line, = ax.plot([0.5], [0.5], marker="|", markersize=40, markeredgewidth=3, color="k")
    ax.set(xlim=(0, 1), ylim=(0, 1))
    assert line._marker is line._marker
    assert line._marker.get_marker() == "|"

    def ink():
        fig.canvas.draw()
        dark = np.asarray(fig.canvas.buffer_rgba())[..., 0] < 128
        rows, columns = np.nonzero(dark)
        return int(np.ptp(columns)) + 1, int(np.ptp(rows)) + 1

    upright = ink()
    assert upright[0] < 6 < 35 < upright[1]
    line._marker._transform = mtransforms.Affine2D().rotate_deg(90)
    assert line.get_marker() == "|"
    turned = ink()
    assert turned[1] < 6 < 35 < turned[0]
    # Setting the marker again gives a style of its own transform.
    line.set_marker("|")
    assert ink() == upright


def test_a_text_follows_a_matrix_its_transform_computes_when_measured():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)

    class Shift(mtransforms.Affine2DBase):
        """A shift read from the Axes' label when the matrix is asked."""

        def get_matrix(self):
            x = float(ax.get_label() or 0)
            return np.array([[1.0, 0.0, x], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])

    text = ax.text(0.5, 0.5, "moved", transform=ax.transAxes + Shift())
    renderer = fig.canvas.get_renderer()
    start = text.get_window_extent(renderer).x0
    ax.set_label("25")
    assert text.get_window_extent(renderer).x0 == pytest.approx(start + 25)


class _GridOfItsOwn(Axes):
    """An Axes reading its ticks in its clear, to draw a grid of its own
    (as pysmithchart's Smith chart draws its arcs and their labels), and
    matplotlib's ``_gridOn`` (as mplstereonet's ``grid`` does)."""

    name = "proj1_grid_of_its_own"

    def clear(self):
        super().clear()
        self.xaxis.set_major_locator(MultipleLocator(0.25))
        self.read = (self.xaxis.get_majorticklocs().tolist(),
                     self.yaxis.get_majorticklocs().tolist(), self._gridOn)
        for x in self.read[0]:
            self.axvline(x, color="C3")


register_projection(_GridOfItsOwn)


def test_an_axes_reads_its_ticks_in_its_constructor():
    # matplotlib's `__init__` clears the Axes before its figure adds it.
    fig = plt.figure()
    expected = ([-0.25, 0.0, 0.25, 0.5, 0.75, 1.0, 1.25],
                [0.0, 0.2, 0.4, 0.6000000000000001, 0.8, 1.0], False)
    ax = fig.add_subplot(projection="proj1_grid_of_its_own")
    assert ax.read == expected and len(ax.lines) == 7
    alone = _GridOfItsOwn(fig, [0.1, 0.1, 0.5, 0.5])
    assert alone.read == expected and len(alone.lines) == 7
    with pltrs.rc_context({"axes.grid": True}):
        ax.clear()
    assert ax.read[2] is True and ax._gridOn is True


def test_a_text_box_takes_its_mutation_aspect():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=100)
    ax.set_axis_off()
    boxes = {}
    for aspect in (None, 0.25):
        props = {"boxstyle": "round,pad=0.5,rounding_size=1.5", "facecolor": "C0",
                 "edgecolor": "none"}
        if aspect is not None:
            props["mutation_aspect"] = aspect
        text = ax.text(0.5, 0.5, "wide box", ha="center", va="center", bbox=props,
                       color="none")
        assert text.get_bbox_patch().get_mutation_aspect() == (aspect or 1.0)
        fig.canvas.draw()
        pixels = np.asarray(fig.canvas.buffer_rgba()).astype(int)
        boxes[aspect] = int(((pixels[..., 2] > pixels[..., 0] + 40)).sum())
        text.remove()
    # The box style shapes the box at a quarter of its height, then
    # stretches it: its pad and corners shrink across the text.
    assert boxes == RECORDED_BOX_PIXELS


#: What `test_a_text_box_takes_its_mutation_aspect` counts under matplotlib.
RECORDED_BOX_PIXELS = {None: 1892, 0.25: 1314}


def test_the_colormap_data_of_matplotlibs_private_module():
    from pltrs._cm import _Set3_data, _jet_data, datad
    assert type(_Set3_data) is tuple and len(_Set3_data) == 12
    assert _Set3_data[0] == (0.5529411764705883, 0.8274509803921568, 0.7803921568627451)
    assert sorted(_jet_data) == ["blue", "green", "red"]
    assert _jet_data["red"][:2] == ((0.0, 0, 0), (0.35, 0, 0))
    assert datad["Set3"] == {"listed": _Set3_data} and len(datad) == 76
    with pytest.raises(ImportError):
        from pltrs._cm import _no_such_data  # noqa: F401


def test_a_transform_is_asked_for_its_inverse_only_where_one_is_needed():
    # A tick made by its constructor takes its label transforms of its
    # Axes; neither that nor a draw inverts what the program's class gave.
    asked = []

    class Halved(mtransforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            return np.asarray(values) * 0.5

    class Doubled(mtransforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            return np.asarray(values) * 2

        def inverted(self):
            asked.append("inverted")
            return Halved()

    fig, ax = plt.subplots()
    transform = Doubled() + ax.transAxes
    text = ax.text(0.1, 0.1, "a", transform=transform)
    line, = ax.plot([0, 0.2], [0, 0.2], transform=transform)
    tick = XTick(ax, 0)
    tick.label1.set_transform(transform)
    fig.canvas.draw()
    assert text.get_window_extent().bounds == pytest.approx(
        (179.2, 123.38341145833333, 9.0, 13.8888888888889))
    assert line.get_window_extent().x1 == pytest.approx(278.4)
    assert asked == []
    assert transform.inverted().transform((200.0, 150.0)) == pytest.approx(
        [0.12096774, 0.13149351])
    assert asked == ["inverted"]


def test_every_method_of_matplotlibs_axis_serves_a_held_axis():
    # pltrs' own: the functions of `_held_axis` reach a program's axis
    # class through the package's classes (``super()`` calls included).
    held = pytest.importorskip("pltrs._held_axis")
    for namespace, cls in ((held.Axis, Axis), (held.XAxis, XAxis), (held.YAxis, YAxis)):
        for name, member in vars(namespace).items():
            if callable(member) and not name.startswith("__"):
                assert callable(getattr(cls, name, None)), (cls.__name__, name)
    fig, ax = _curve_axes()
    assert ax.caxis._held and not ax.xaxis._held
    assert type(ax.caxis.majorTicks) is list
