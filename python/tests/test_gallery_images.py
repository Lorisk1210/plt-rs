"""
What the gallery examples drew otherwise than matplotlib 3.11 (``cargo
xtask diff --gallery``, its images): the behaviours behind them, each
expected value recorded from matplotlib.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import dates as mdates, sankey as msankey
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import host_subplot
from pltrs.offsetbox import AnchoredText
from pltrs.patches import BoxStyle, Rectangle
from pltrs.projections import register_projection


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_text_boxes_take_the_pad_of_their_dict():
    ax = plt.figure().add_subplot()
    texts = [ax.text(0, 0, "x", bbox=dict(boxstyle="square", pad=0)),
             ax.text(0, 0, "x", bbox=dict(boxstyle="round", pad=0.2)),
             ax.text(0, 0, "x", bbox=dict(boxstyle="round,pad=0.5", pad=0.2))]
    assert [t.get_bbox_patch().get_boxstyle().pad for t in texts] == [0.0, 0.2, 0.5]


def test_box_styles_are_listed_in_matplotlibs_order():
    assert list(BoxStyle.get_styles())[3:5] == ['rarrow', 'larrow']


def test_pcolor_with_edges_is_antialiased():
    ax = plt.figure().add_subplot()
    mesh = ax.pcolor(np.arange(6).reshape(2, 3), edgecolors="k", linewidths=4)
    assert [bool(v) for v in np.atleast_1d(mesh.get_antialiased())] == [True]


def test_scatter_masks_the_colours_of_masked_sizes():
    fig, ax = plt.subplots()
    size = np.ma.masked_where([True, False, True, False], [10.0, 20, 30, 40])
    points = ax.scatter([0, 1, 2, 3], [0, 1, 2, 3], s=size, c=[1.0, 2, 3, 4])
    fig.canvas.draw()
    assert np.ma.count_masked(points.get_array()) == 2
    assert (points.norm.vmin, points.norm.vmax) == (2.0, 4.0)


def test_sankey_faces_given_leave_the_cycle_alone():
    ax = plt.figure().add_subplot()
    diagram = msankey.Sankey(ax=ax, unit=None)
    diagram.add(flows=[1, -1], orientations=[0, 1], facecolor="k")
    diagram.add(flows=[1, -1], orientations=[-1, -1], prior=0, connect=(1, 0))
    assert np.allclose(diagram.diagrams[1].patch.get_facecolor(),
                       (0.1216, 0.4667, 0.7059, 1.0), atol=1e-4)


def test_patches_take_cycle_colours_when_made():
    with plt.style.context("seaborn-v0_8"):
        rect = Rectangle((0, 0), 1, 1, color="C1")
        fig, ax = plt.subplots()
        ax.add_patch(rect)
    fig.canvas.draw()
    assert np.allclose(rect.get_facecolor(), (0.3333, 0.6588, 0.4078, 1.0), atol=1e-4)


def test_anchored_boxes_have_their_frame_before_they_join():
    fig, ax = plt.subplots()
    box = AnchoredText("a", loc="upper left", frameon=True)
    box.patch.set_boxstyle("round,pad=0.,rounding_size=0.2")
    frame = box.patch
    ax.add_artist(box)
    fig.canvas.draw()
    assert box.patch is frame
    assert type(box.patch.get_boxstyle()).__name__ == "Round"
    assert not AnchoredText("b", loc="upper left", frameon=False).patch.get_visible()


def test_axis_labels_draw_their_boxes():
    fig, ax = plt.subplots()
    ax.set_ylabel("y", bbox=dict(facecolor="yellow"))
    fig.canvas.draw()
    assert np.allclose(ax.yaxis.label.get_bbox_patch().get_facecolor(), (1, 1, 0, 1))


def test_tick_marks_keep_their_edge_colour():
    ax = plt.figure().add_subplot()
    tick = ax.yaxis.get_major_ticks()[0]
    tick.tick1line.set_color("blue")
    assert tick.tick1line.get_markeredgecolor() == "black"


def test_auto_date_formatters_follow_the_frequency_of_the_view():
    fig, ax = plt.subplots(figsize=(8, 0.8))
    ax.set_xlim(np.datetime64("2000-02-01"), np.datetime64("2002-02-01"))
    ax.xaxis.set_major_formatter(mdates.AutoDateFormatter(ax.xaxis.get_major_locator()))
    fig.canvas.draw()
    assert [label.get_text() for label in ax.get_xticklabels()][:3] == [
        '2000-04', '2000-07', '2000-10']


def test_rectangles_share_the_interpolation_steps_of_bars_and_spans():
    ax = plt.figure().add_subplot()
    rect = Rectangle((0, 0), 1, 1)
    # The steps are every rectangle's, whatever drew bars before: they
    # start from 1 here, and the other tests get theirs back.
    shared = rect.get_path()
    saved = shared._interpolation_steps
    shared._interpolation_steps = 1
    try:
        before = Rectangle((0, 0), 2, 2).get_path()._interpolation_steps
        ax.bar([0, 1], [1, 2])
        after_bar = Rectangle((0, 0), 1, 1).get_path()._interpolation_steps
        ax.axhspan(0, 1)
        assert (before, after_bar, rect.get_path()._interpolation_steps) == (1, 100, 180)
    finally:
        shared._interpolation_steps = saved


def test_reading_the_offset_text_does_not_lay_the_figure_out():
    fig = plt.figure(layout="constrained")
    ax = fig.add_subplot()
    ax.xaxis.offsetText
    assert np.allclose(ax.get_position().bounds, (0.125, 0.11, 0.775, 0.77))


def test_limits_autoscale_with_the_parameters_of_their_moment():
    with plt.style.context("classic"):
        fig, axs = plt.subplots(ncols=2)
        axs[0].plot([0.3, 4.7], [0.2, 3.1])
        axs[1].set_xticks([0, 1], labels=["a", "b"])
        t = np.linspace(-10, 10, 100)
        axs[1].plot(t, 1.5 / (1 + np.exp(-t)))
        axs[1].set_xlim(-10, 10)
    fig.canvas.draw()
    # The ticks of one axes leave the other to autoscale at the draw; the
    # x limits set make the y axis autoscale under the classic style.
    assert np.allclose(axs[0].get_xlim(), (0.3, 4.7))
    assert np.allclose(axs[1].get_ylim(), (0.0, 1.6))


def test_text_is_hinted_as_the_parameters_say_when_drawn():
    def drawn(hinting):
        with plt.rc_context({"text.hinting": hinting}):
            fig, ax = plt.subplots(figsize=(2, 1))
            ax.set_title("Axes title classic")
        buf = io.BytesIO()
        fig.savefig(buf, format="rgba")
        return buf.getvalue()
    assert drawn("auto") == drawn("default")


def test_twins_of_axis_artist_hosts_show_matplotlibs_sides():
    host = host_subplot(111, axes_class=axisartist.Axes)
    twin = host.twinx()
    assert {k: host.axis[k].get_visible() for k in host.axis} == {
        'bottom': True, 'top': True, 'left': True, 'right': False}
    assert {k: twin.axis[k].get_visible() for k in twin.axis} == {
        'bottom': False, 'top': False, 'left': False, 'right': True}
    assert not host.yaxis.get_visible()


def test_annotation_arrows_keep_the_sketch_they_were_made_with():
    with plt.xkcd():
        ax = plt.figure().add_subplot()
        annotation = ax.annotate("a", xy=(0.5, 0.5), xytext=(0.1, 0.1),
                                 arrowprops=dict(arrowstyle="->"))
    assert annotation.arrow_patch.get_sketch_params() == (1.0, 100.0, 2.0)


def test_colorbars_of_3d_contours_tick_their_levels():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    X, Y = np.meshgrid(np.arange(4.0), np.arange(5.0))
    contours = ax.contourf(X, Y, X + Y, levels=[0, 2, 4, 6, 8])
    colorbar = fig.colorbar(contours)
    fig.canvas.draw()
    assert type(colorbar.locator).__name__ == "FixedLocator"
    assert list(colorbar.ax.get_yticks()) == [0.0, 2.0, 4.0, 6.0, 8.0]


def test_raster_prints_show_what_the_draw_event_draws():
    fig, ax = plt.subplots()
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    line, = ax.plot([0, 1], [0.5, 0.5])

    def moved_line_pixels():
        buf = io.BytesIO()
        fig.savefig(buf, format="rgba")
        pixels = np.frombuffer(buf.getvalue(), np.uint8).reshape(480, 640, 4)
        return int((pixels[345:360, 100:540, 0] < 128).sum())

    def on_draw(event):
        line.set_ydata([0.2, 0.2])
        fig.draw_artist(line)

    assert moved_line_pixels() == 0
    fig.canvas.mpl_connect("draw_event", on_draw)
    assert moved_line_pixels() == 880


def test_cycler_is_exported():
    from pltrs import cycler
    assert cycler(color=["r"]).by_key() == {"color": ["r"]}


def test_program_projections_bend_lines_and_place_tick_labels():
    from pltrs.axes import Axes
    from pltrs.path import Path
    from pltrs.transforms import Affine2D, Transform

    class Squash(Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            x, y = np.asarray(values, dtype=float).T
            return np.column_stack([x, y + 0.25 * np.sin(np.pi * x)])

        def transform_path_non_affine(self, path):
            ipath = path.interpolated(20)
            return Path(self.transform(ipath.vertices), ipath.codes)

        def inverted(self):
            return Unsquash()

    class Unsquash(Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            x, y = np.asarray(values, dtype=float).T
            return np.column_stack([x, y - 0.25 * np.sin(np.pi * x)])

    class SquashAxes(Axes):
        name = "squash_test"

        def _set_lim_and_transforms(self):
            super()._set_lim_and_transforms()
            self.transData = Squash() + Affine2D().scale(1, 0.5) + self.transAxes
            self._xaxis_text1_transform = (Affine2D().scale(1.0, 0.0) + self.transData
                                           + Affine2D().translate(0.0, 4.0))

        def get_xaxis_text1_transform(self, pad):
            return self._xaxis_text1_transform, 'bottom', 'center'

    register_projection(SquashAxes)
    fig = plt.figure()
    ax = fig.add_subplot(projection="squash_test")
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.set_xticks([0.5])
    line, = ax.plot([0, 1], [0.2, 0.2])
    fig.canvas.draw()
    # The middle of the line is bent up through the projection.
    middle = ax.transData.transform((0.5, 0.2))
    assert np.allclose(middle, (328.0, 135.96), atol=0.01)
    # The label sits 4 pixels above where the class' transform puts it.
    label = ax.xaxis.get_ticklabels()[0].get_window_extent()
    assert np.allclose(label.bounds, (317.0, 103.0, 22.0, 13.889), atol=0.01)
