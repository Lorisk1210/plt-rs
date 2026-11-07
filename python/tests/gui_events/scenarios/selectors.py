"""
The selectors of ``matplotlib.widgets`` used by hand: drawn, moved,
resized by their handles, with their state modifier keys, cleared.
"""

from . import at, one_axes, scenario
from .widgets import _on_close, _widgets


def _span(plt, log, **options):
    widgets = _widgets(plt)
    fig, ax = one_axes(plt)
    selector = widgets.SpanSelector(
        ax, lambda vmin, vmax: log.note("select", extents=[vmin, vmax]),
        "horizontal", useblit=True, interactive=True,
        props=dict(facecolor="tab:blue", alpha=0.5), **options)
    _on_close(fig, lambda: log.note("state", extents=list(selector.extents),
                                    visible=selector.get_visible()))
    fig._selector = selector
    return fig


@scenario("span")
def span(plt, log):
    return _span(plt, log)


@span.steps
def _(d):
    d.drag(1, [at(2, 5), at(3, 5), at(5, 5)])
    d.snapshot("drawn")
    # The right edge's handle, then the middle (moving the span).
    d.drag(1, [at(5, 5), at(6, 5), at(7, 5)])
    d.drag(1, [at(4, 3), at(5, 3), at(5.5, 3)])
    d.snapshot("moved")
    # A click outside the span removes it; a new span from right to left.
    d.move(*at(9, 5))
    d.click(1)
    d.drag(1, [at(8, 5), at(6, 5), at(1, 5)])
    d.key("escape")
    d.drag(1, [at(4, 5), at(6, 5)])
    d.close()


@scenario("span-outside")
def span_outside(plt, log):
    return _span(plt, log, ignore_event_outside=True, drag_from_anywhere=True)


@span_outside.steps
def _(d):
    d.drag(1, [at(2, 5), at(5, 5)])
    d.drag(1, [at(8, 5), at(9, 5)])
    d.drag(1, [at(3, 5), at(4, 5)])
    d.close()


def _box(plt, log, kind):
    widgets = _widgets(plt)
    fig, ax = one_axes(plt)
    selector = getattr(widgets, kind)(
        ax, lambda press, release: log.note(
            "select", press=[press.xdata, press.ydata],
            release=[release.xdata, release.ydata]),
        useblit=True, interactive=True, button=[1, 3],
        props=dict(facecolor="tab:red", alpha=0.3, fill=True))
    _on_close(fig, lambda: log.note(
        "state", extents=list(selector.extents), visible=selector.get_visible(),
        rotation=selector.rotation))
    fig._selector = selector
    return fig


def _box_steps(d):
    d.drag(1, [at(2, 2), at(4, 4), at(5, 6)])
    d.snapshot("drawn")
    # The top-right corner's handle; the right edge's; the middle (moving).
    d.drag(1, [at(5, 6), at(6, 7)])
    d.drag(1, [at(6, 4.5), at(7, 4.5)])
    d.drag(1, [at(4, 4), at(4.5, 3.5)])
    d.snapshot("resized")
    # Drawn again with the modifiers: square, from the centre, both.
    with d.hold("shift"):
        d.drag(1, [at(1, 1), at(3, 2)])
    with d.hold("ctrl"):
        d.drag(1, [at(5, 5), at(6, 7)])
    with d.hold("ctrl+shift"):
        d.drag(3, [at(5, 5), at(7, 6)])
    # Moved with space held, from anywhere.
    with d.hold("space"):
        d.drag(1, [at(9, 9), at(8, 8)])
    d.key("escape")
    d.snapshot("cleared")
    d.drag(1, [at(1, 1), at(1.05, 1.05)])
    d.close()


@scenario("rectangle")
def rectangle(plt, log):
    return _box(plt, log, "RectangleSelector")


rectangle.steps(_box_steps)


@scenario("ellipse")
def ellipse(plt, log):
    return _box(plt, log, "EllipseSelector")


ellipse.steps(_box_steps)


@scenario("lasso")
def lasso(plt, log):
    widgets = _widgets(plt)
    fig, ax = one_axes(plt)
    fig._selector = widgets.LassoSelector(
        ax, lambda verts: log.note("select", verts=[list(v) for v in verts]),
        useblit=True)
    return fig


@lasso.steps
def _(d):
    d.drag(1, [at(2, 2), at(5, 2), at(6, 5), at(4, 8), at(2, 6), at(2.2, 2.2)])
    d.drag(3, [at(5, 5), at(6, 6)])
    d.drag(1, [at(7, 7), at(8, 7)])
    d.close()


@scenario("polygon")
def polygon(plt, log):
    widgets = _widgets(plt)
    fig, ax = one_axes(plt)
    selector = widgets.PolygonSelector(
        ax, lambda verts: log.note("select", verts=[list(v) for v in verts]),
        useblit=True)
    _on_close(fig, lambda: log.note("state", verts=[list(v) for v in
                                                    selector.verts]))
    fig._selector = selector
    return fig


@polygon.steps
def _(d):
    for point in [at(2, 2), at(6, 2), at(6, 6), at(2, 6)]:
        d.move(*point)
        d.click(1)
    d.snapshot("open")
    d.move(*at(2, 2))
    d.click(1)
    d.snapshot("closed")
    # A vertex dragged; then the whole polygon with shift.
    d.drag(1, [at(6, 6), at(7, 7)])
    with d.hold("shift"):
        d.drag(1, [at(4, 4), at(5, 4)])
    # Ctrl moves a vertex of a closed polygon too; escape starts again.
    with d.hold("ctrl"):
        d.drag(1, [at(3, 2), at(3, 1)])
    d.key("escape")
    for point in [at(1, 1), at(3, 1)]:
        d.move(*point)
        d.click(1)
    # A vertex removed by a right click, then the polygon completed.
    d.move(*at(3, 1))
    d.click(3)
    for point in [at(3, 3), at(1, 3), at(1, 1)]:
        d.move(*point)
        d.click(1)
    d.close()
