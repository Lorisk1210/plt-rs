"""
Where pltrs lays out natively and a program's code has a say, as in
matplotlib: measuring methods a program class overrides, asked by the
figure's tight box and the layouts (an axis' ``get_tightbbox`` and
``_get_ticklabel_bboxes``, a spine's, an Axes'
``get_default_bbox_extra_artists``, a figure legend's ``get_tightbbox``),
measured with the view autoscaled; tick labels whose ``set_x``/``set_y``
the program replaced (pygtc), measured at another dpi, and tick marks
turned by their marker's ``_transform``; an empty axis label's place
after a draw; in-place changes to an assigned ``child_axes`` list. Every
expected value was recorded from matplotlib 3.11.
"""

import io
import types

import numpy as np
import pytest

import pltrs
import pltrs.axis as maxis
import pltrs.legend as mlegend
import pltrs.lines as mlines
import pltrs.pyplot as plt
import pltrs.spines as mspines
import pltrs.text as mtext
import pltrs.transforms as mtransforms

DX, DY = 120.0, -30.0


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def rounded(values, digits=3):
    """*values* (a box's bounds or a sequence), rounded."""
    return np.round(getattr(values, "bounds", values), digits).tolist()


def moved(method):
    """A method calling the package's *method* and moving the box it
    gives by (DX, DY)."""
    def own(self, *args, **kwargs):
        box = method(self, *args, **kwargs)
        return None if box is None else box.translated(DX, DY)
    return own


def tight_size(fig):
    """The size in pixels of the figure saved with a tight box."""
    from PIL import Image
    buf = io.BytesIO()
    fig.savefig(buf, format="png", bbox_inches="tight")
    buf.seek(0)
    return Image.open(buf).size


def positions(fig, layout):
    """The Axes' positions under the layout engine *layout*."""
    fig.set_layout_engine(layout)
    fig.canvas.draw()
    return [rounded(ax.get_position()) for ax in fig.axes]


def axes_with(axes_class=None):
    fig = plt.figure(figsize=(3, 2), dpi=100)
    ax = fig.add_subplot(axes_class=axes_class) if axes_class else fig.add_subplot()
    ax.plot([0, 1], [0, 1], label="line")
    ax.set_title("title")
    return fig, ax


def own_xaxis_axes(method):
    """An Axes class whose x axis is of a class overriding *method*."""
    own = type("OwnXAxis", (maxis.XAxis,), {
        method: moved(getattr(maxis.XAxis, method)) if method == "get_tightbbox" else
        (lambda self, ticks, renderer=None: tuple(
            [box.translated(DX, DY) for box in boxes]
            for boxes in maxis.XAxis._get_ticklabel_bboxes(self, ticks, renderer)))})

    class OwnAxes(plt.Axes):
        def _init_axis(self):
            super()._init_axis()
            self.xaxis = own(self)
            self.spines.bottom.register_axis(self.xaxis)
            self.spines.top.register_axis(self.xaxis)
    return OwnAxes


@pytest.mark.parametrize("method", ["get_tightbbox", "_get_ticklabel_bboxes"])
def test_an_axis_measuring_itself_is_asked_by_tight_boxes_and_layouts(method):
    fig, ax = axes_with(own_xaxis_axes(method))
    assert tight_size(fig) == (418, 248)
    for layout, expected in [("constrained", [0.12, 0.289, 0.449, 0.584]),
                             ("tight", [0.186, 0.343, 0.348, 0.475])]:
        fig, ax = axes_with(own_xaxis_axes(method))
        assert positions(fig, layout) == [expected]


@pytest.mark.parametrize("method", ["get_window_extent", "get_tightbbox"])
def test_a_spine_measuring_itself_counts_as_it_measures(method):
    def make():
        fig, ax = axes_with()
        own = type("OwnSpine", (mspines.Spine,), {method: moved(getattr(mspines.Spine, method))})
        ax.spines["left"] = own.linear_spine(ax, "left")
        return fig
    assert tight_size(make()) == (298, 225)
    assert positions(make(), "constrained") == [[0.15, 0.171, 0.822, 0.702]]
    assert positions(make(), "tight") == [[0.186, 0.225, 0.748, 0.593]]


def test_an_axes_class_choosing_its_extra_artists_is_measured_with_them():
    class Picky(plt.Axes):
        def get_default_bbox_extra_artists(self):
            return [a for a in super().get_default_bbox_extra_artists()
                    if not isinstance(a, mtext.Text) or a.get_text() != "far"]

    def make():
        fig, ax = axes_with(Picky)
        ax.text(1.4, 0.5, "far", transform=ax.transAxes)
        return fig
    assert tight_size(make()) == (298, 218)
    assert positions(make(), "constrained") == [[0.15, 0.139, 0.822, 0.734]]
    assert positions(make(), "tight") == [[0.186, 0.193, 0.748, 0.625]]


def test_a_figure_legend_measuring_itself_counts_as_it_measures():
    fig, ax = axes_with()
    own = type("OwnLegend", (mlegend.Legend,), {
        "get_tightbbox": moved(mlegend.Legend.get_tightbbox)})
    legend = own(fig, ax.get_lines(), ["line"])
    fig.legends.append(legend)
    fig.canvas.draw()
    assert rounded(fig.get_tightbbox(), 2) == [-0.03, -0.02, 4.16, 1.99]


@pytest.mark.parametrize("layout, expected", [
    ("constrained", [0.15, 0.139, 0.576, 0.734]),
    ("tight", [0.186, 0.193, 0.54, 0.625]),
])
def test_a_line_measuring_itself_sees_the_view_autoscaled(layout, expected):
    fig, ax = axes_with()
    own = type("OwnLine", (mlines.Line2D,), {
        "get_window_extent": moved(mlines.Line2D.get_window_extent)})
    ax.add_line(own([0.2, 0.8], [0.5, 0.6], clip_on=False))
    assert positions(fig, layout) == [expected]


@pytest.mark.parametrize("pan", [False, True])
def test_tick_labels_follow_the_position_setters_a_program_gives_them(pan):
    # pygtc shifts its tick labels by replacing their set_x and set_y.
    fig, ax = plt.subplots(figsize=(3, 2), dpi=60)
    ax.plot([0, 1], [0, 1])
    for label in ax.get_xticklabels():
        label.custom_shift = 0.1
        label.set_x = types.MethodType(
            lambda self, x: mtext.Text.set_x(self, x + self.custom_shift), label)
    for label in ax.get_yticklabels():
        label.custom_shift = -0.05
        label.set_y = types.MethodType(
            lambda self, y: mtext.Text.set_y(self, y + self.custom_shift), label)
    if pan:
        ax.set_xlim(0.2, 1.2)
    fig.canvas.draw()
    xs = [rounded(t.get_window_extent(), 1) for t in ax.get_xticklabels()[:3]]
    ys = [rounded(t.get_window_extent(), 1) for t in ax.get_yticklabels()[:2]]
    if pan:
        assert xs == [[30.0, -1.0, 13.0, 8.3], [57.8, -1.0, 13.0, 8.3], [85.8, -1.0, 13.0, 8.3]]
    else:
        assert xs == [[-2.7, -1.0, 25.0, 8.3], [32.5, -1.0, 18.0, 8.3], [64.2, -1.0, 18.0, 8.3]]
    assert ys == [[-8.3, -13.0, 25.0, 8.3], [-1.3, 8.0, 18.0, 8.3]]
    assert [round(float(t.get_position()[0]), 3)
            for t in ax.get_xticklabels()[:3]] == ([0.3, 0.5, 0.7] if pan else [-0.15, 0.1, 0.35])


def test_axis_texts_measured_at_another_resolution():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([0, 1])
    ax.set_xlabel("x label")
    fig.canvas.draw()
    assert rounded(ax.get_xticklabels()[1].get_window_extent(dpi=100), 1) == [41.1, -1.6, 14.0, 13.9]
    assert rounded(ax.get_yticklabels()[1].get_window_extent(dpi=100), 1) == [13.8, 20.4, 14.0, 13.9]
    assert rounded(ax.xaxis.label.get_window_extent(dpi=100), 1) == [142.8, -17.5, 22.0, 13.9]


def test_a_tick_mark_turns_with_its_markers_transform():
    def drawn(turned):
        fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
        ax.plot([0, 1])
        fig.canvas.draw()
        if turned:
            for tick in ax.xaxis.get_major_ticks():
                tick.tick1line._marker._transform = (
                    mtransforms.Affine2D().scale(3.0).rotate_deg(45))
        fig.canvas.draw()
        return np.asarray(fig.canvas.buffer_rgba()).copy()
    plain, turned = drawn(False), drawn(True)
    changed = np.argwhere((plain != turned).any(axis=-1))
    assert changed.min(axis=0).tolist() == [84, 11]
    assert changed.max(axis=0).tolist() == [91, 86]


@pytest.mark.parametrize("label", ["", "hidden"])
def test_an_empty_or_hidden_axis_label_is_placed_by_the_draw(label):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=60)
    ax.plot([0, 1])
    ax.set_xlabel(label)
    ax.xaxis.label.set_visible(label != "hidden")
    assert rounded(ax.xaxis.label.get_position(), 2) == [0.5, 0.0]
    fig.canvas.draw()
    assert rounded(ax.xaxis.label.get_position(), 2) == [0.5, -4.3]
    assert rounded(ax.yaxis.label.get_position(), 2) == [-4.67, 0.5]


def test_child_axes_follow_changes_made_in_place_to_an_assigned_list():
    def pixels(fig):
        fig.canvas.draw()
        return np.asarray(fig.canvas.buffer_rgba()).copy()
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    a = ax.inset_axes([0.1, 0.6, 0.3, 0.3])
    b = ax.inset_axes([0.6, 0.1, 0.3, 0.3])
    ax.child_axes = [a, b]
    both = pixels(fig)
    ax.child_axes.remove(a)
    without = pixels(fig)
    x0, y0, x1, y1 = a.get_window_extent().extents
    rows = without.shape[0]
    assert (without[int(rows - y1 + 3):int(rows - y0 - 3), int(x0 + 3):int(x1 - 3), :3] == 255).all()
    ax.child_axes.append(a)
    assert np.array_equal(pixels(fig), both)


def _shifted_x(self, x):
    """A tick label's ``set_x`` of the program's (pygtc's), shifting it."""
    mtext.Text.set_x(self, x + 0.1)


def _shift_labels(ax):
    """Give the x tick labels of *ax* the shifting ``set_x``."""
    for label in ax.get_xticklabels():
        label.set_x = types.MethodType(_shifted_x, label)


@pytest.mark.parametrize("change, positions", [
    ("nothing", [-0.15, 0.1, 0.35]),
    ("cla", [-0.25, 0.0, 0.25]),
    ("reset_ticks", [-0.25, 0.0, 0.25]),
    ("set_xlim", [0.3, 0.5, 0.7]),
])
def test_replaced_tick_label_setters_meet_cleared_and_reset_ticks(change, positions):
    # matplotlib's clear and reset_ticks make the ticks anew: their labels'
    # setters are called no more.
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([0, 1])
    fig.canvas.draw()
    _shift_labels(ax)
    if change == "cla":
        ax.cla()
        ax.plot([0, 1])
    elif change == "reset_ticks":
        ax.xaxis.reset_ticks()
    elif change == "set_xlim":
        ax.set_xlim(0.2, 1.2)
    fig.canvas.draw()
    fig.savefig(io.BytesIO(), format="png")
    assert [round(float(t.get_position()[0]), 3) for t in ax.get_xticklabels()[:3]] == positions


def test_replaced_tick_label_setters_of_a_removed_axes_draw_without_it():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([0, 1])
    fig.canvas.draw()
    _shift_labels(ax)
    with pytest.raises(NotImplementedError):
        ax.get_xticklabels()[1].remove()
    ax.remove()
    fig.canvas.draw()
    fig.savefig(io.BytesIO(), format="png")


@pytest.mark.parametrize("layout, title, counts", [
    (None, False, [(3, 3), (3, 3), (11, 11)]),
    (None, True, [(3, 6), (3, 6), (11, 20)]),
    ("constrained", False, [(13, 13), (16, 16), (21, 21)]),
])
def test_replaced_tick_label_setters_are_called_where_matplotlib_updates_ticks(
        layout, title, counts):
    # matplotlib's `Tick.update_position` calls them at each tick update:
    # three per draw, more for a title placed or a layout.
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 1])
    if title:
        ax.set_title("t")
    if layout:
        fig.set_layout_engine(layout)
    fig.canvas.draw()
    calls = {"x": 0, "y": 0}
    for which, label, setter in (("x", ax.get_xticklabels()[0], "set_x"),
                                 ("y", ax.get_yticklabels()[0], "set_y")):
        def counted(self, value, _which=which, _setter=setter):
            calls[_which] += 1
            getattr(mtext.Text, _setter)(self, value)
        setattr(label, setter, types.MethodType(counted, label))
    seen = []
    for act in (lambda: fig.canvas.draw(),
                lambda: fig.savefig(io.BytesIO(), format="png"),
                lambda: fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")):
        act()
        seen.append((calls["x"], calls["y"]))
        calls.update(x=0, y=0)
    assert seen == counts
