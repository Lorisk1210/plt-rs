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
