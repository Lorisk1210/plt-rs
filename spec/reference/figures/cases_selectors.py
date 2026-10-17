"""Cursors and selector widgets driven by synthetic mouse and key events.

Each case builds a widget, feeds it events the way
``matplotlib.testing.widgets.click_and_drag`` does, and keeps the artists
the widget leaves in the axes. Limits are set explicitly so that data and
display coordinates agree before the first draw.
"""

from __future__ import annotations

import numpy as np
from matplotlib import widgets
from matplotlib.backend_bases import MouseEvent
from matplotlib.testing.widgets import click_and_drag

from spec_support import case

T = np.linspace(0, 10, 60)


def _plot(ax):
    ax.plot(T, np.sin(T))
    ax.set_xlim(0, 10)
    ax.set_ylim(-1.5, 1.5)


def _move(ax, xy):
    MouseEvent._from_ax_coords("motion_notify_event", ax, xy)._process()


def _noop(*args):
    pass


@case("selectors/cursor")
def _(fig):
    ax = fig.subplots()
    _plot(ax)
    cursor = widgets.Cursor(ax, color="red", linewidth=1)
    _move(ax, (4.0, 0.5))
    del cursor


@case("selectors/multicursor")
def _(fig):
    top, bottom = fig.subplots(2, 1)
    _plot(top)
    bottom.plot(T, np.cos(T), "g")
    bottom.set_xlim(0, 10)
    bottom.set_ylim(-1.5, 1.5)
    multi = widgets.MultiCursor((top, bottom), useblit=False, horizOn=True, color="r", lw=1)
    _move(bottom, (6.0, 0.25))
    del multi


@case("selectors/span_interactive")
def _(fig):
    ax = fig.subplots()
    _plot(ax)
    span = widgets.SpanSelector(ax, _noop, "horizontal", interactive=True,
                                props=dict(facecolor="tab:blue", alpha=0.3))
    click_and_drag(span, (2.0, 0.0), (6.0, 0.0))
    # Resize with the upper handle.
    click_and_drag(span, (6.0, 0.5), (7.5, 0.5))
    del span


@case("selectors/span_vertical_snap")
def _(fig):
    ax = fig.subplots()
    _plot(ax)
    span = widgets.SpanSelector(ax, _noop, "vertical", interactive=True,
                                snap_values=np.linspace(-1.5, 1.5, 13),
                                handle_props=dict(color="k", linewidth=2))
    click_and_drag(span, (5.0, -0.9), (5.0, 0.8))
    del span


@case("selectors/span_drag_from_anywhere")
def _(fig):
    ax = fig.subplots()
    _plot(ax)
    span = widgets.SpanSelector(ax, _noop, "horizontal", interactive=True,
                                drag_from_anywhere=True,
                                props=dict(facecolor="orange", alpha=0.4))
    span.extents = (3.0, 5.0)
    click_and_drag(span, (4.0, 0.0), (6.0, 0.0))
    del span


@case("selectors/span_move_key")
def _(fig):
    left, right = fig.subplots(1, 2)
    for ax in (left, right):
        _plot(ax)
    moved = widgets.SpanSelector(left, _noop, "horizontal", interactive=True)
    click_and_drag(moved, (2.0, 0.0), (4.0, 0.0))
    click_and_drag(moved, (8.0, 0.0), (9.0, 0.0), key=" ")
    # Spans no longer than minspan are removed on release.
    removed = widgets.SpanSelector(right, _noop, "horizontal", interactive=True, minspan=1.0)
    click_and_drag(removed, (2.0, 0.0), (6.0, 0.0))
    click_and_drag(removed, (8.0, 0.0), (8.5, 0.0))
    del moved, removed


@case("selectors/rectangle_interactive")
def _(fig):
    ax = fig.subplots()
    _plot(ax)
    rect = widgets.RectangleSelector(ax, _noop, interactive=True)
    click_and_drag(rect, (2.0, -1.0), (6.0, 1.0))
    # Resize with the north-east corner.
    click_and_drag(rect, (6.0, 1.0), (8.0, 1.2))
    del rect


@case("selectors/rectangle_square_center")
def _(fig):
    left, right = fig.subplots(1, 2)
    for ax in (left, right):
        _plot(ax)
    square = widgets.RectangleSelector(left, _noop, interactive=True,
                                       props=dict(facecolor="tab:green", alpha=0.3))
    click_and_drag(square, (2.0, -1.0), (6.0, 0.0), key="shift")
    centred = widgets.RectangleSelector(right, _noop, interactive=True,
                                        handle_props=dict(markeredgecolor="r", markersize=9))
    click_and_drag(centred, (5.0, 0.0), (7.0, 0.8), key="control")
    del square, centred


@case("selectors/rectangle_rotated")
def _(fig):
    left, right = fig.subplots(1, 2)
    for ax in (left, right):
        _plot(ax)
    set_angle = widgets.RectangleSelector(left, _noop, interactive=True)
    set_angle.extents = (3.0, 7.0, -0.5, 0.5)
    set_angle.rotation = 25
    dragged = widgets.RectangleSelector(right, _noop, interactive=True)
    click_and_drag(dragged, (3.0, -0.5), (7.0, 0.5))
    click_and_drag(dragged, (7.0, -0.5), (7.0, 0.2), key="r")
    del set_angle, dragged


@case("selectors/rectangle_move_key")
def _(fig):
    ax = fig.subplots()
    _plot(ax)
    rect = widgets.RectangleSelector(ax, _noop, interactive=True, drag_from_anywhere=True)
    click_and_drag(rect, (1.0, -1.0), (3.0, 0.0))
    click_and_drag(rect, (8.0, 1.0), (9.0, 1.2), key=" ")
    click_and_drag(rect, (3.0, -0.3), (4.0, -0.5))
    del rect


@case("selectors/ellipse")
def _(fig):
    left, right = fig.subplots(1, 2)
    for ax in (left, right):
        _plot(ax)
    ellipse = widgets.EllipseSelector(left, _noop, interactive=True,
                                      props=dict(facecolor="tab:blue", edgecolor="k", alpha=0.3))
    click_and_drag(ellipse, (2.0, -1.0), (8.0, 1.0))
    plain = widgets.EllipseSelector(right, _noop, interactive=True,
                                    spancoords="pixels", minspanx=30, minspany=30)
    click_and_drag(plain, (2.0, -1.0), (7.0, 0.5))
    plain.rotation = -30
    del ellipse, plain


def _press(ax, xy, button=1):
    MouseEvent._from_ax_coords("button_press_event", ax, xy, button)._process()


def _release(ax, xy, button=1):
    MouseEvent._from_ax_coords("button_release_event", ax, xy, button)._process()


def _place_vertex(ax, xy, button=1):
    """matplotlib's test helper ``polygon_place_vertex``."""
    _move(ax, xy)
    _press(ax, xy, button)
    _release(ax, xy, button)


LOOP = [(2.0 + 1.5 * np.cos(a), 1.0 * np.sin(a)) for a in np.linspace(0, 5, 12)]


@case("selectors/lasso")
def _(fig):
    left, right = fig.subplots(1, 2)
    for ax in (left, right):
        _plot(ax)
    selector = widgets.LassoSelector(left, _noop, useblit=False, props=dict(color="m", lw=2))
    _press(left, LOOP[0])
    for xy in LOOP[1:]:
        MouseEvent._from_ax_coords("motion_notify_event", left, xy, 1)._process()
    lasso = widgets.Lasso(right, (5.0, 0.0), _noop, useblit=False, props=dict(color="tab:red"))
    for xy in [(6.0, 0.5), (7.0, 0.2), (6.5, -0.6), (5.5, -0.4)]:
        MouseEvent._from_ax_coords("motion_notify_event", right, xy, 1)._process()
    del selector, lasso


@case("selectors/polygon_complete")
def _(fig):
    ax = fig.subplots()
    _plot(ax)
    polygon = widgets.PolygonSelector(ax, _noop)
    for xy in [(2.0, -1.0), (6.0, -1.2), (7.0, 0.5), (4.0, 1.2), (2.0, -1.0)]:
        _place_vertex(ax, xy)
    # Drag the third vertex.
    click_and_drag(polygon, (7.0, 0.5), (8.0, 0.8))
    del polygon


@case("selectors/polygon_open")
def _(fig):
    left, right = fig.subplots(1, 2)
    for ax in (left, right):
        _plot(ax)
    polygon = widgets.PolygonSelector(left, _noop, props=dict(color="tab:green", lw=3),
                                      handle_props=dict(marker="s", markersize=5))
    for xy in [(2.0, -1.0), (6.0, -1.0), (7.0, 0.5)]:
        _place_vertex(left, xy)
    _move(left, (3.0, 1.0))
    moved = widgets.PolygonSelector(right, _noop)
    moved.verts = [(2.0, -1.0), (5.0, -1.0), (3.5, 1.0)]
    click_and_drag(moved, (3.0, 0.0), (5.0, 0.2), key="shift")
    del polygon, moved


@case("selectors/polygon_box")
def _(fig):
    ax = fig.subplots()
    _plot(ax)
    polygon = widgets.PolygonSelector(ax, _noop, draw_bounding_box=True,
                                      box_props=dict(facecolor="none", edgecolor="tab:orange"))
    for xy in [(2.0, -1.0), (6.0, -1.0), (4.0, 1.0), (2.0, -1.0)]:
        _place_vertex(ax, xy)
    # Stretch with the north-east corner of the box.
    click_and_drag(polygon._box, (6.0, 1.0), (8.0, 1.3))
    del polygon
