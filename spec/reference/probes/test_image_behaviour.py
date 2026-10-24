"""Behaviour of images, ported from the non-image tests of matplotlib's
``lib/matplotlib/tests/test_image.py``.

Area ``image_behaviour``. Every op draws on ``Figure()`` (6.4 x 4.8 in at
100 dpi) with an Agg canvas and ``ax = fig.add_subplot()`` unless noted.

Ops
---
``image.cursor``  {scene, pixels} -> [str, ...]
    The status bar message (``NavigationToolbar2._mouse_event_to_message``)
    with the pointer at each display pixel of ``pixels``: the coordinates
    and, over the image, its cursor data. The pixels are those nearest to
    data points chosen in and around the image.

    ``upper`` / ``lower``  ``ax.imshow(arange(100).reshape(10, 10),
        origin=...)``, ``upper`` viewed over ``xlim=(-1, 11)``,
        ``ylim=(11, -1)``.
    ``extent``  ``ax.imshow(arange(100).reshape(10, 10), extent=[0, 0.5,
        0, 0.5])`` viewed over ``xlim=(-0.1, 1)``, ``ylim=(-0.1, 0.6)``.
    ``transformed``  ``ax.imshow(arange(100).reshape(10, 10),
        transform=Affine2D().scale(2).rotate(0.5) + ax.transData)`` viewed
        over ``xlim=(-20, 20)``, ``ylim=(-5, 25)``.
    ``nonuniform``  a ``NonUniformImage`` of ``x**2 + y**2`` at ``x = [0,
        1, 4, 9, 16]``, ``y = [0, 1, 2, 3, 4]`` viewed over ``xlim=(-2,
        16)``, ``ylim=(-2, 4)``.
    ``data:<name>``  ``ax.imshow(DATA[name])`` (``test_format_cursor_data``).

    Mirrors ``test_cursor_data``, ``test_cursor_data_nonuniform`` and
    ``test_format_cursor_data``.
``image.window_extent``  {transform} -> bbox
    ``Figure(figsize=(10, 10))`` with ``ax.set_position([0, 0, 1, 1])``,
    limits ``(0, 1)`` (x ``(1, 2)`` for ``"axes"``), and ``ax.imshow(im,
    extent=[0.4, 0.7, 0.2, 0.9], interpolation="nearest")`` in data or
    axes coordinates: ``get_window_extent()``
    (``test_get_window_extent_for_AxisImage``).
``image.limits``  {scene} -> {"xlim", "ylim", "aspect"}
    ``relim``: ``ax.imshow([[0]], extent=(0, 1, 0, 1))`` then ``ax.relim()``
    and ``ax.autoscale()`` (``test_relim``); ``axes``,
    ``affine`` and ``axes_aspect``: ``ax.imshow(arange(4).reshape(2, 2))``
    with ``transform=ax.transAxes``, ``Affine2D().scale(2) +
    ax.transData`` or ``transAxes`` and ``aspect=2``
    (``test_non_transdata_image_does_not_touch_aspect``); ``aspect`` is
    ``"auto"`` or the number.
``image.composite``  {fmt, composite} -> int
    Two images on one axes saved as ``svg`` or ``ps`` with
    ``image.composite_image`` on or off: the number of ``<image`` elements
    or ``colorimage`` operators (``test_composite``).
``image.exact_vmin``  {} -> [[r, g, b, a], ...]
    A 190 x 10 pixel figure filled by ``ax.imshow`` of one row of data
    (``-1``, ``0``, ``43``, ...) with ``cmap=autumn_r`` (under: light grey),
    ``vmin=0``, ``vmax=100``, ``aspect="auto"``: the middle row of the
    rendered pixels, where values equal to ``vmin`` must not take the under
    colour (``test_exact_vmin``).
``image.settings``  {} -> {"before", "after", "zero_radius_rejected"}
    ``ax.imshow(arange(12.0).reshape(3, 4))`` read back as made
    (``before``) and after ``set_filternorm(False)``, ``set_filterrad(2)``,
    ``set_resample(True)``, ``set_interpolation_stage("rgba")`` and
    ``set_data(arange(8.0).reshape(4, 2) * 10)`` (``after``): filternorm,
    filterrad, resample, interpolation_stage, clim, extent and the shape of
    ``get_array()``; then whether ``set_filterrad(0)`` is rejected.
``image.draw``  {scene} -> "drawn"
    ``all_nan``: ``ax.imshow(full((2, 2), nan))``; ``partly_nan``:
    ``ax.imshow([[1, 2], [3, nan]])`` (``test_mask_image_all``,
    ``test_full_invalid``, ``test_imshow_no_warn_invalid``).

Left out: imread/imsave tests (the ``imread`` and ``imsave`` areas), the
dtype tests (``float16``, ``float128``, ``bool``, ``uint16``,
``QuantityND``), argument validation (``test_imshow_10_10_2``,
``test_imshow_10_10_5``, ``test_image_array_alpha_validation``,
``test_rc_interpolation_stage``; typed in plt-rs), Python copies
(``test_axesimage_setdata``, ``test_figureimage_setdata``,
``test_setdata_xya``), ``test_axesimage_get_shape``, ``test_empty_imshow``
(an empty image is rejected by plt-rs's image data), private resampling
helpers (``test__resample_*``, ``test_resample_dtypes``,
``test_rgba_stage_demultiplies_resampled_alpha``: ``make_image`` output
before compositing is not observable), ``test_format_cursor_data_multinorm``
(multivariate colormaps through private attributes) and pixel checks,
which the ``mpl_tests_image`` figure group draws (``test_image_edges``,
``test_unclipped``, ``test_respects_bbox``, ``test_minimized_rasterized``).
``test_rc_interpolation_stage``, ``test_rgba_stage_demultiplies_resampled_alpha``
and ``test_resample_nonaffine`` are probed by the ``mpl_image`` area or
drawn by that group.
"""

import io

import matplotlib as mpl
import numpy as np
from matplotlib.backend_bases import MouseEvent, NavigationToolbar2
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.image import NonUniformImage
from matplotlib.transforms import Affine2D

from spec_support import ProbeSet

probes = ProbeSet("image_behaviour")

GRID100 = np.arange(100).reshape(10, 10)
DATA = {
    "big": [[10001, 10000]],
    "small": [[.123, .987]],
    "nan": [[np.nan, 1, 2]],
    "tiny_range": [[1, 1 + 1e-15]],
    "negative": [[-1, -1]],
    "zeros": [[0, 0]],
}


def _axes(**kw):
    fig = Figure(**kw)
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot()


def _nonuniform(ax):
    x = np.array([0, 1, 4, 9, 16])
    y = np.array([0, 1, 2, 3, 4])
    z = x[np.newaxis, :] ** 2 + y[:, np.newaxis] ** 2
    im = NonUniformImage(ax, extent=(x.min(), x.max(), y.min(), y.max()))
    im.set_data(x, y, z)
    ax.add_image(im)
    ax.set_xlim(x.min() - 2, x.max())
    ax.set_ylim(y.min() - 2, y.max())


def _build(ax, scene):
    if scene == "upper":
        ax.imshow(GRID100, origin="upper")
        ax.set_xlim(-1, 11)
        ax.set_ylim(11, -1)
    elif scene == "lower":
        ax.imshow(GRID100, origin="lower")
    elif scene == "extent":
        ax.imshow(GRID100, extent=[0, 0.5, 0, 0.5])
        ax.set_xlim(-0.1, 1)
        ax.set_ylim(-0.1, 0.6)
    elif scene == "transformed":
        ax.imshow(GRID100, transform=Affine2D().scale(2).rotate(0.5) + ax.transData)
        ax.set_xlim(-20, 20)
        ax.set_ylim(-5, 25)
    elif scene == "nonuniform":
        _nonuniform(ax)
    else:
        ax.imshow(DATA[scene.removeprefix("data:")])


def _scene(scene):
    fig, ax = _axes()
    _build(ax, scene)
    fig.canvas.draw()
    return fig, ax


def _pixels(scene, points):
    """The display pixels nearest to data ``points`` in ``scene``."""
    fig, ax = _scene(scene)
    return np.round(ax.transData.transform(points)).astype(int).tolist()


def _cursor(scene, pixels):
    fig, ax = _scene(scene)
    return [NavigationToolbar2._mouse_event_to_message(
        MouseEvent("motion_notify_event", fig.canvas, x, y)) for x, y in pixels]


IM = np.array([[0.25, 0.75, 1.0, 0.75], [0.1, 0.65, 0.5, 0.4],
               [0.6, 0.3, 0.0, 0.2], [0.7, 0.9, 0.4, 0.6]])


def _window_extent(transform):
    fig, ax = _axes(figsize=(10, 10), dpi=100)
    ax.set_position([0, 0, 1, 1])
    ax.set_xlim(1 if transform == "axes" else 0, 2 if transform == "axes" else 1)
    ax.set_ylim(0, 1)
    kw = {"transform": ax.transAxes} if transform == "axes" else {}
    image = ax.imshow(IM, extent=[0.4, 0.7, 0.2, 0.9], interpolation="nearest", **kw)
    fig.canvas.draw()
    return image.get_window_extent(fig.canvas.renderer).get_points().tolist()


def _limits(scene):
    fig, ax = _axes()
    grid = np.arange(4).reshape((2, 2))
    if scene == "relim":
        ax.imshow([[0]], extent=(0, 1, 0, 1))
        ax.relim()
        ax.autoscale()
    elif scene == "axes":
        ax.imshow(grid, transform=ax.transAxes)
    elif scene == "affine":
        ax.imshow(grid, transform=Affine2D().scale(2) + ax.transData)
    else:
        ax.imshow(grid, transform=ax.transAxes, aspect=2)
    aspect = ax.get_aspect()
    return {"xlim": list(ax.get_xlim()), "ylim": list(ax.get_ylim()),
            "aspect": aspect if aspect == "auto" else float(aspect)}


def _composite(fmt, composite):
    x, y = np.meshgrid(np.arange(-5, 5, 1), np.arange(-5, 5, 1))
    z = np.sin(y ** 2)
    with mpl.rc_context({"image.composite_image": composite}):
        fig, ax = _axes()
        ax.set_xlim(0, 3)
        ax.imshow(z, extent=[0, 1, 0, 1])
        ax.imshow(z[::-1], extent=[2, 3, 0, 1])
        buffer = io.BytesIO()
        fig.savefig(buffer, format=fmt)
    return buffer.getvalue().count(b"<image" if fmt == "svg" else b" colorimage")


def _exact_vmin():
    cmap = mpl.colormaps["autumn_r"].with_extremes(under="lightgrey")
    fig = Figure(figsize=(1.9, 0.1), dpi=100)
    FigureCanvasAgg(fig)
    ax = fig.add_axes((0, 0, 1, 1))
    data = np.array([[-1, -1, -1, 0, 0, 0, 0, 43, 79, 95, 66, 1, -1, -1, -1, 0, 0, 0, 34]],
                    dtype=float)
    ax.imshow(data, aspect="auto", cmap=cmap, vmin=0, vmax=100)
    ax.axis("off")
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba())[5].tolist()


def _draw(scene):
    fig, ax = _axes()
    ax.imshow(np.full((2, 2), np.nan) if scene == "all_nan" else [[1, 2], [3, np.nan]])
    fig.canvas.draw()
    return "drawn"


CURSOR_POINTS = {
    "upper": [[4, 4], [10.1, 4], [0.1, -0.1], [9.4, 0.2], [-0.4, 9.4]],
    "lower": [[4, 4], [9.4, 9.4], [0.2, 0.7]],
    "extent": [[0.26, 0.24], [0.75, 0.25], [0.01, -0.01], [0.49, 0.49]],
    "transformed": [[3, 10], [1, 1], [10, 10], [-5, 15], [0.5, 0.5]],
    "nonuniform": [[0.5, 0.5], [0.5, 1.5], [4.5, 0.5], [8.5, 0.5], [9.5, 2.5], [-1, 0.5],
                   [0.5, -1], [15.9, 3.9]],
}


def test_cursor():
    for scene, points in CURSOR_POINTS.items():
        probes.probe("image.cursor", _cursor, scene=scene, pixels=_pixels(scene, points))
    for name in DATA:
        scene = f"data:{name}"
        probes.probe("image.cursor", _cursor, scene=scene, pixels=_pixels(scene, [[0.1, 0.1]]))


def test_extents_and_limits():
    for transform in ["data", "axes"]:
        probes.probe("image.window_extent", _window_extent, transform=transform)
    for scene in ["relim", "axes", "affine", "axes_aspect"]:
        probes.probe("image.limits", _limits, scene=scene)


def _settings():
    fig, ax = _axes()
    im = ax.imshow(np.arange(12.0).reshape(3, 4))

    def read():
        return {
            "filternorm": im.get_filternorm(), "filterrad": float(im.get_filterrad()),
            "resample": im.get_resample(), "interpolation_stage": im.get_interpolation_stage(),
            "clim": [float(v) for v in im.get_clim()],
            "extent": [float(v) for v in im.get_extent()],
            "shape": list(im.get_array().shape),
        }

    before = read()
    im.set_filternorm(False)
    im.set_filterrad(2)
    im.set_resample(True)
    im.set_interpolation_stage("rgba")
    im.set_data(np.arange(8.0).reshape(4, 2) * 10)
    after = read()
    try:
        im.set_filterrad(0)
        rejected = False
    except ValueError:
        rejected = True
    return {"before": before, "after": after, "zero_radius_rejected": rejected}


def test_settings():
    probes.probe("image.settings", _settings)


def test_output():
    for fmt in ["svg", "ps"]:
        for composite in [True, False]:
            probes.probe("image.composite", _composite, fmt=fmt, composite=composite)
    probes.probe("image.exact_vmin", _exact_vmin)
    for scene in ["all_nan", "partly_nan"]:
        probes.probe("image.draw", _draw, scene=scene)
