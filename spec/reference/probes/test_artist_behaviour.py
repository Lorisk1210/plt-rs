"""Behaviour shared by all artists, ported from the non-image tests of
matplotlib's ``lib/matplotlib/tests/test_artist.py``.

Area ``artist_behaviour``. Every op draws on ``Figure()`` (6.4 x 4.8 in at
100 dpi) with an Agg canvas and ``ax = fig.add_subplot()``.

Ops
---
``artist.default_zorder``  {} -> {kind: zorder}
    The z-order of a new artist of each kind made by the usual function:
    ``plot``, ``scatter``, ``text``, ``annotate``, ``imshow``,
    ``add_patch(Rectangle)``, ``axhline``, ``axvspan``, ``bar`` (its
    first rectangle), ``fill_between``, ``LineCollection``, ``contour``
    and ``contourf``, and of a line made with ``zorder=None`` or given
    ``set_zorder(123456)`` (``test_None_zorder``).
``artist.transform_of_none``  {} -> [bbox, bbox, bbox]
    Over ``xlim = ylim = (1, 3)``: the display extents of an ``Ellipse((2,
    2), 1, 1)`` in data coordinates, of ``Ellipse(xy_pix, 120, 120,
    transform=None)`` and of ``Ellipse(xy_pix, 100, 100,
    transform=IdentityTransform())``, ``xy_pix`` being ``(2, 2)`` in
    display coordinates; ``None`` means display coordinates
    (``test_patch_transform_of_none``).
``artist.remove``  {} -> {"image_mouseover", "line_mouseover", "images",
    "lines"}
    ``ax.imshow(arange(36).reshape(6, 6))`` and ``ax.plot(range(5))``:
    whether each reports cursor data (``mouseover``), then the number of
    images and lines left on the axes after ``remove()``
    (``test_remove``).
``artist.alpha``  {kind, alpha} -> alpha
    ``set_alpha(alpha)`` on a line, patch, text or scatter (``kind``), then
    a draw: the alpha, or an error for values outside ``[0, 1]`` and NaN
    (matplotlib rejects them in ``set_alpha``, plt-rs when it draws them).
    For ``kind == "scatter_array"`` ``alpha`` is a list, one per point
    (``_set_alpha_for_array``). Mirrors ``test_set_alpha`` and
    ``test_set_alpha_for_array``.
``artist.cursor_boundary``  {data, boundaries, ncolors, clip, extend} ->
    [str, ...]
    ``ax.imshow(data, cmap="RdBu_r", norm=BoundaryNorm(boundaries, ncolors,
    clip, extend))`` drawn, then the status bar message
    (``NavigationToolbar2._mouse_event_to_message``) with the pointer on
    each cell, at the display pixel nearest to a quarter of a cell right of
    and below its centre, in row-major order: the cursor data are
    formatted with as many significant digits as the boundaries' spacing
    calls for (``test_format_cursor_data_BoundaryNorm``).
``artist.base_properties``  {} -> {"before", "after", "figure_linewidth"}
    The properties every artist has, read back from ``ax.plot([0, 1], [0,
    1])`` as made (``before``) and after ``set_rasterized(True)``,
    ``set_animated(True)``, ``set_in_layout(False)``, ``set_snap(False)``,
    ``set_sketch_params(1, 100, 2)``, ``set_clip_on(False)``,
    ``set_clip_box(None)`` and ``set_url("u")`` (``after``): rasterized,
    animated, in_layout, snap, sketch, clip_on, url, whether the transform
    is set, whether the clip box is the axes box (``null`` without one),
    whether there is a clip path and the number of path effects; and the
    figure's ``get_linewidth()``.
``artist.cull_markers``  {} -> {"pdf_small", "svg_small"}
    20000 ``"k."`` markers at ``spec_support.rng(3)`` points in the unit
    square, viewed over ``xlim=(2, 3)`` (none shown): whether the PDF is
    under 8000 bytes and the SVG under 20000 (``test_cull_markers``).

Left out: ``test_collection_transform_of_none`` (``get_offset_transform()``
identity checks), ``test_properties``, ``test_setp``, ``test_artist_set*``,
``test_artist_inspector_*``, ``test_callbacks``, ``test_set_signature``,
``test_set_is_overwritten``, ``test_auto_no_rasterize`` and
``test_draw_wraper_forward_input`` (Python introspection, keyword handling
and object identity); ``test_get_figure`` is a probe of the ``mpl_artist``
area.
"""

import io

import numpy as np
from matplotlib.backend_bases import MouseEvent, NavigationToolbar2
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.collections import LineCollection
from matplotlib.colors import BoundaryNorm
from matplotlib.figure import Figure
from matplotlib.patches import Ellipse, Rectangle
from matplotlib.transforms import IdentityTransform

from spec_support import ProbeSet, rng

probes = ProbeSet("artist_behaviour")


def _axes():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot()


def _default_zorder():
    fig, ax = _axes()
    z = np.arange(16.0).reshape(4, 4)
    line_none, = ax.plot([0, 1], zorder=None)
    line_set, = ax.plot([0, 1])
    line_set.set_zorder(123456)
    artists = {
        "plot": ax.plot([0, 1])[0],
        "scatter": ax.scatter([0], [0]),
        "text": ax.text(0, 0, "t"),
        "annotate": ax.annotate("a", (0, 0)),
        "imshow": ax.imshow(z),
        "patch": ax.add_patch(Rectangle((0, 0), 1, 1)),
        "axhline": ax.axhline(0.5),
        "axvspan": ax.axvspan(0.2, 0.4),
        "bar": ax.bar([0], [1])[0],
        "fill_between": ax.fill_between([0, 1], [0, 1]),
        "line_collection": ax.add_collection(LineCollection([[(0, 0), (1, 1)]])),
        "contour": ax.contour(z),
        "contourf": ax.contourf(z),
        "line_none": line_none,
        "line_set": line_set,
    }
    return {kind: float(artist.get_zorder()) for kind, artist in artists.items()}


def _transform_of_none():
    fig, ax = _axes()
    ax.set_xlim(1, 3)
    ax.set_ylim(1, 3)
    xy_pix = ax.transData.transform((2, 2))
    patches = [ax.add_patch(Ellipse((2, 2), width=1, height=1)),
               ax.add_patch(Ellipse(xy_pix, width=120, height=120, transform=None)),
               ax.add_patch(Ellipse(xy_pix, width=100, height=100,
                                    transform=IdentityTransform()))]
    fig.canvas.draw()
    return [p.get_window_extent().get_points().tolist() for p in patches]


def _remove():
    fig, ax = _axes()
    image = ax.imshow(np.arange(36).reshape(6, 6))
    line, = ax.plot(range(5))
    fig.canvas.draw()
    observed = {"image_mouseover": image.get_mouseover(), "line_mouseover": line.get_mouseover()}
    image.remove()
    line.remove()
    observed["images"] = len(ax.images)
    observed["lines"] = len(ax.lines)
    return observed


def _alpha(kind, alpha):
    fig, ax = _axes()
    if kind == "line":
        artist = ax.plot([0, 1])[0]
    elif kind == "patch":
        artist = ax.add_patch(Rectangle((0, 0), 1, 1))
    elif kind == "text":
        artist = ax.text(0.5, 0.5, "t")
    else:
        artist = ax.scatter([0, 1], [0, 1])
    artist.set_alpha(alpha)
    fig.canvas.draw()
    result = artist.get_alpha()
    return result.tolist() if isinstance(result, np.ndarray) else result


def _cursor_boundary(data, boundaries, ncolors, clip, extend):
    fig, ax = _axes()
    norm = BoundaryNorm(boundaries, ncolors, clip=clip, extend=extend)
    ax.imshow(np.array(data, float), cmap="RdBu_r", norm=norm)
    fig.canvas.draw()
    messages = []
    rows, columns = np.shape(data)
    for row in range(rows):
        for column in range(columns):
            x, y = np.round(ax.transData.transform((column + 0.25, row + 0.25)))
            event = MouseEvent("motion_notify_event", fig.canvas, x, y)
            messages.append(NavigationToolbar2._mouse_event_to_message(event))
    return messages


def _base_properties():
    fig, ax = _axes()
    line, = ax.plot([0, 1], [0, 1])
    fig.canvas.draw()

    def read():
        box = line.get_clip_box()
        return {
            "rasterized": line.get_rasterized(), "animated": line.get_animated(),
            "in_layout": line.get_in_layout(), "snap": line.get_snap(),
            "sketch": line.get_sketch_params(), "clip_on": line.get_clip_on(),
            "url": line.get_url(), "transform_set": line.is_transform_set(),
            "clip_box_is_axes": None if box is None else bool(
                np.allclose(box.get_points(), ax.bbox.get_points())),
            "clip_path": line.get_clip_path() is not None,
            "path_effects": len(line.get_path_effects()),
        }

    before = read()
    line.set_rasterized(True)
    line.set_animated(True)
    line.set_in_layout(False)
    line.set_snap(False)
    line.set_sketch_params(1, 100, 2)
    line.set_clip_on(False)
    line.set_clip_box(None)
    line.set_url("u")
    return {"before": before, "after": read(), "figure_linewidth": fig.get_linewidth()}


def _cull_markers():
    generator = rng(3)
    x, y = generator.uniform(20000), generator.uniform(20000)
    fig, ax = _axes()
    ax.plot(x, y, "k.")
    ax.set_xlim(2, 3)
    pdf, svg = io.BytesIO(), io.BytesIO()
    fig.savefig(pdf, format="pdf")
    fig.savefig(svg, format="svg")
    return {"pdf_small": len(pdf.getvalue()) < 8000, "svg_small": len(svg.getvalue()) < 20000}


def test_scenes():
    probes.probe("artist.default_zorder", _default_zorder)
    probes.probe("artist.transform_of_none", _transform_of_none)
    probes.probe("artist.remove", _remove)
    probes.probe("artist.base_properties", _base_properties)
    probes.probe("artist.cull_markers", _cull_markers)


def test_alpha():
    for kind in ["line", "patch", "text", "scatter"]:
        for alpha in [0.0, 0.25, 1.0, None, 1.1, -0.1, np.nan]:
            probes.probe("artist.alpha", _alpha, kind=kind, alpha=alpha)
    for alpha in [[0.5, 0.75], [0.5, 1.1], [0.5, np.nan], [0.0, 1.0]]:
        probes.probe("artist.alpha", _alpha, kind="scatter_array", alpha=alpha)


X = [[0.9, 0.99, 0.999], [-1, 0, 1], [0.09, 0.009, 0.0009]]
COLUMN = [[-1.0], [0.0], [0.1], [0.5], [0.9], [1.0], [2.0]]


def test_cursor_boundaries():
    for n in [20, 200, 2000]:
        probes.probe("artist.cursor_boundary", _cursor_boundary, data=X,
                     boundaries=np.linspace(-1, 1, n).tolist(), ncolors=256 if n == 20 else n,
                     clip=False, extend="neither")
    for clip, extend in [(False, "neither"), (False, "min"), (False, "max"), (False, "both"),
                         (True, "neither")]:
        probes.probe("artist.cursor_boundary", _cursor_boundary, data=COLUMN,
                     boundaries=np.linspace(0, 1, 4).tolist(), ncolors=256, clip=clip,
                     extend=extend)
