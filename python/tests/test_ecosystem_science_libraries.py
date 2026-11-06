"""
What cartopy, nilearn, shap, sympy, missingno and mpl-scatter-density need
of matplotlib that its own suite does not reach (``cargo xtask ecosystem
suite`` runs their suites), each answered as matplotlib 3.11 answers it.
The expected values were recorded from matplotlib 3.11.2 running the same
code.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs import path as mpath
from pltrs import projections, spines, text, ticker, transforms
from pltrs.axes import Axes

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _rounded(values, digits=6):
    """*values* (nested sequences of numbers) rounded, as lists."""
    return np.round(np.asarray(values, dtype=float), digits).tolist()


def _dark_pixels(fig, below, rows=slice(None)):
    """The pixels of *fig*'s canvas, in *rows*, whose channels add up to
    less than *below*."""
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())[rows, :, :3]
    return int((pixels.sum(-1) < below).sum())


class _Doubled:
    """A program's transform-giver (cartopy's projections are ones)."""

    def _as_mpl_transform(self, axes):
        return transforms.Affine2D().scale(2) + axes.transData


class _FrameSpine(spines.Spine):
    """A spine of no side, as cartopy's ``GeoSpine`` is."""

    def __init__(self, axes):
        super().__init__(axes, "frame", mpath.Path(
            [(0.2, 0.2), (0.8, 0.2), (0.5, 0.8), (0.2, 0.2)], closed=True))

    def _adjust_location(self):
        pass


class _Framed(Axes):
    """A projection framed by a spine of its own, whose ``autoscale_view``
    keeps x in [0, 10] (as cartopy's ``GeoAxes`` keeps the map's)."""

    name = "framed_by_a_spine"

    def _gen_axes_spines(self, *args, **kwargs):
        sides = super()._gen_axes_spines(*args, **kwargs)
        for spine in sides.values():
            spine.set_visible(False)
        return {**sides, "frame": _FrameSpine(self)}

    def autoscale_view(self, tight=None, scalex=True, scaley=True):
        super().autoscale_view(tight=tight, scalex=scalex, scaley=scaley)
        if scalex and self.get_autoscalex_on():
            x0, x1 = self.get_xbound()
            self.set_xbound(max(x0, 0.0), min(x1, 10.0))


class _Constructed(Axes):
    """A projection taking an argument of its own."""

    name = "constructed_with_a_factor"

    def __init__(self, *args, factor=1.0, **kwargs):
        self.factor = factor
        super().__init__(*args, **kwargs)


class _WithFactor:
    """A projection given as an object, as cartopy's ``CRS`` are."""

    def _as_mpl_axes(self):
        return _Constructed, {"factor": 3.0}


projections.register_projection(_Framed)


def test_artists_and_figures_answer_their_private_registers():
    fig, (ax1, ax2) = plt.subplots(2)
    image = ax1.imshow([[1, 2], [3, 4]])
    assert image._axes is ax1 and ax1._axes is ax1
    assert not hasattr(fig, "_axes")
    assert fig._axstack.as_list() == [ax1, ax2]
    assert fig._axstack.current() is ax2
    fig._axstack.bubble(ax1)
    assert fig.gca() is ax1


def test_contourf_up_to_infinity_fills_its_band():
    fig, ax = plt.subplots(figsize=(2, 2))
    z = np.linspace(0, 1, 100).reshape(10, 10)
    with np.errstate(invalid="ignore"):
        filled = ax.contourf(z, levels=[0.4, np.inf], vmin=0.0, vmax=1.0)
        assert filled.get_facecolor().tolist() == [[0.0, 0.0, 0.0, 0.0]]
        assert _dark_pixels(fig, 700) == 1011


def test_data_interval_set_with_ignore_gives_way_to_the_next_data():
    fig, ax = plt.subplots()
    ax.xaxis.set_data_interval(-180, 180, True)
    assert tuple(map(float, ax.get_xlim())) == (0.0, 1.0)
    assert ax.ignore_existing_data_limits
    ax.plot([0, 10], [0, 5])
    assert tuple(map(float, ax.dataLim.intervalx)) == (0.0, 10.0)
    assert not ax.ignore_existing_data_limits
    ax.ignore_existing_data_limits = True
    ax.plot([20, 30], [0, 1])
    assert tuple(map(float, ax.dataLim.intervalx)) == (20.0, 30.0)


def test_transform_givers_place_text_markers_meshes_and_contours():
    fig, ax = plt.subplots()
    label = ax.text(1, 1, "x", transform=_Doubled())
    ax.scatter([1, 2], [1, 2], transform=_Doubled())
    mesh = ax.pcolormesh(np.arange(3), np.arange(3), np.ones((2, 2)),
                         transform=_Doubled())
    lines = ax.contour(np.arange(3), np.arange(3),
                       np.arange(9.0).reshape(3, 3), transform=_Doubled())
    fig.canvas.draw()
    assert _rounded(label.get_transform().transform((1, 1))) == [328.0, 237.6]
    assert _rounded(ax.dataLim.bounds) == [0.0, 0.0, 4.0, 4.0]
    assert type(lines.get_transform()).__name__ == "CompositeGenericTransform"
    assert mesh._coordinates.shape == (3, 3, 2)


def test_contour_keeps_the_transform_given_and_its_data_limits():
    fig, ax = plt.subplots()
    shift = transforms.Affine2D().translate(10, 0) + ax.transData
    x, y = np.meshgrid(np.linspace(0, 1, 5), np.linspace(0, 2, 6))
    lines = ax.contour(x, y, x + y, transform=shift)
    assert lines.get_transform() is shift
    assert _rounded(ax.get_xlim()) == [10.0, 11.0]
    assert _rounded(ax.get_ylim()) == [0.0, 2.0]
    ax.plot([0, 20], [0, 1])
    assert _rounded(ax.get_xlim()) == [-1.0, 21.0]


def test_pcolor_of_rgb_colours_leaves_out_masked_cells():
    colours = np.ma.masked_array(np.linspace(0, 1, 4 * 5 * 3).reshape(4, 5, 3))
    colours[1, 2] = np.ma.masked
    fig, ax = plt.subplots()
    cells = ax.pcolor(np.arange(6), np.arange(5), colours)
    fig.canvas.draw()
    assert len(cells.get_paths()) == 19
    assert _rounded(cells.get_facecolor()[[0, 6, 18]]) == [
        [0.0, 0.016949, 0.033898, 1.0],
        [0.305085, 0.322034, 0.338983, 1.0],
        [0.966102, 0.983051, 1.0, 1.0]]


def test_text_boxes_hidden_and_of_loose_text():
    fig, ax = plt.subplots()
    hidden = ax.text(0.5, 0.5, "hello", bbox={"pad": 0, "visible": False})
    assert not hidden.get_bbox_patch().get_visible()
    loose = text.Text(0.5, 0.5, "hello", transform=ax.transData,
                      bbox={"pad": 0})
    loose.set_figure(fig)
    loose.axes = ax
    fig.canvas.draw()
    loose.update_bbox_position_size(fig.canvas.get_renderer())
    box = loose.get_bbox_patch()
    assert loose.axes is ax
    extents = box.get_path().transformed(box.get_transform()).get_extents()
    assert _rounded(extents.bounds, 3) == [328.0, 234.263, 35.0, 14.337]


def test_clipping_to_the_patch_of_a_second_axes():
    fig, (_, ax2) = plt.subplots(2)
    line, = ax2.plot([0, 1], [0, 1], lw=5, color="k")
    line.set_clip_path(ax2.patch)
    assert _dark_pixels(fig, 100, rows=slice(240, None)) == 4769


def test_max_n_locator_raw_ticks_overridden():
    class Stepped(ticker.MaxNLocator):
        def _raw_ticks(self, vmin, vmax):
            self.set_params(steps=[1, 3, 6, 10])
            return ticker.MaxNLocator._raw_ticks(self, vmin, vmax)

    every_sixty = [-180.0, -120.0, -60.0, 0.0, 60.0, 120.0, 180.0]
    assert ticker.MaxNLocator(nbins=4)._raw_ticks(0, 10).tolist() == [
        0.0, 2.5, 5.0, 7.5, 10.0]
    assert Stepped(nbins=8).tick_values(-180, 180).tolist() == every_sixty
    fig, ax = plt.subplots()
    ax.set_xlim(-180, 180)
    ax.xaxis.set_major_locator(Stepped(nbins=8))
    assert ax.get_xticks().tolist() == every_sixty


def test_spine_of_no_side_frames_a_projection():
    fig = plt.figure(figsize=(3, 3))
    ax = fig.add_subplot(projection="framed_by_a_spine")
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    frame = ax.spines["frame"]
    assert sorted(ax.spines.keys()) == ["bottom", "frame", "left", "right",
                                        "top"]
    assert type(frame) is _FrameSpine and frame.spine_type == "frame"
    assert _dark_pixels(fig, 300) == 1081


def test_projection_autoscale_view_runs_when_data_is_added():
    fig = plt.figure()
    ax = fig.add_subplot(projection="framed_by_a_spine")
    ax.plot([-5, 20], [0, 1])
    assert tuple(map(float, ax.get_xlim())) == (0.0, 10.0)


def test_poly3d_face_colours_changed_in_place():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    faces = ax.plot_trisurf([0, 1, 0, 1], [0, 0, 1, 1], [0, 1, 1, 0],
                            triangles=[[0, 1, 2], [1, 3, 2]], color="white")
    faces._facecolor3d[1] = (1, 0, 0, 1)
    assert _rounded(faces._facecolors) == [
        [0.986788, 0.986788, 0.986788, 1.0], [1.0, 0.0, 0.0, 1.0]]


def test_inset_of_a_projection_given_as_an_object():
    fig, ax = plt.subplots()
    inset = ax.inset_axes([0.5, 0.5, 0.4, 0.4], projection=_WithFactor())
    fig.draw_without_rendering()
    assert type(inset) is _Constructed and inset.factor == 3.0
    assert inset in ax.child_axes
    assert _rounded(inset.get_position().bounds) == [
        0.5125, 0.495, 0.31, 0.308]
