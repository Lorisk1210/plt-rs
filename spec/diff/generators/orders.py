"""Calls in unusual orders: on artists removed, on closed figures and
cleared or deleted Axes, artists moved across Axes and figures, and the
same call twice.

Per artist subject of ``artists`` (an artist ``obj`` in ``ax`` of
``fig``), one program runs each scenario on the subject made anew:

* ``removed``: the artist drawn, removed, then read, set, measured,
  removed again, added back and drawn;
* ``closed``: its figure closed, then the artist set, the figure drawn
  and saved, plotted on, and closed again;
* ``moved``: the artist added to an Axes and a figure of another figure,
  and both drawn;
* ``cleared``: its Axes cleared, then its figure, the artist read and
  removed, and both drawn;
* ``deleted``: its Axes deleted from the figure, plotted on, added back
  and deleted twice.

``FIGURES`` are programs of whole figures in unusual orders: Axes,
colorbars, legends and twins across figures, pyplot's state after
closing, layout engines changed late, scales changed after drawing.
"""

from __future__ import annotations

from typing import Any, Iterator

from . import artists, common
from .api import ENDING
from .subjects import PRELUDE

#: How an artist of each kind is added to an Axes, the most specific first.
ADDERS = (("Line2D", "add_line"), ("Patch", "add_patch"), ("Collection", "add_collection"),
          ("AxesImage", "add_image"), ("Table", "add_table"), ("Container", "add_container"))

REMOVED = [
    "fig.canvas.draw()", "obj.remove()", "obj.axes", "obj.figure", "obj.get_figure()", "obj.stale",
    "obj.get_visible()", "obj.set_visible(False)", "obj.set_alpha(0.5)", "obj.get_alpha()",
    "obj.get_window_extent()", "obj.get_tightbbox()", "obj.get_transform()", "obj.remove()", "obj.pchanged()",
    "fig.canvas.draw()",
]
CLOSED = [
    "plt.close(fig)", "plt.fignum_exists(fig.number)", "obj.set_alpha(0.3)", "obj.get_alpha()",
    "fig.canvas.draw()", "fig.savefig(io.BytesIO(), format='png')", "plt.gcf() is fig", "plt.close(fig)",
    "plt.get_fignums()",
]
MOVED = [
    "fig2, ax2 = plt.subplots()", "fig2.add_artist(obj)", "obj.figure is fig2", "obj.set_figure(fig2)",
    "fig.canvas.draw()", "fig2.canvas.draw()",
]
CLEARED = [
    "fig.canvas.draw()", "fig.clear()", "obj.axes", "obj.figure", "obj.stale", "obj.remove()",
    "obj.get_window_extent()", "fig.canvas.draw()",
]
WITH_AXES = {
    "removed": ["ax.add_artist(obj)", "obj.axes is ax", "obj.remove()", "fig.canvas.draw()"],
    "closed": ["ax.plot([1, 2])", "ax.get_lines()"],
    "moved": ["ax2.add_artist(obj)", "obj.axes is ax2", "obj.axes = ax2"],
    "cleared": ["ax.cla()", "ax.get_children()", "ax.plot([1, 2])"],
    "deleted": ["fig.canvas.draw()", "fig.delaxes(ax)", "obj.axes", "ax.figure", "ax.plot([1, 2])",
                "fig.canvas.draw()", "fig.add_axes(ax)", "fig.axes", "fig.delaxes(ax)", "fig.delaxes(ax)",
                "fig.canvas.draw()"],
}

#: Programs of figures in unusual orders.
FIGURES: list[tuple[str, list[str]]] = [
    ("axes_across_figures", [
        "fig1, ax1 = plt.subplots()", "fig2, ax2 = plt.subplots()", "fig2.add_axes(ax1)", "ax2.sharex(ax1)",
        "ax2.get_shared_x_axes().joined(ax1, ax2)", "ax1.plot([1, 2])", "fig1.canvas.draw()", "fig2.canvas.draw()",
        "ax2.get_xlim()", "fig2.delaxes(ax1)", "fig1.sca(ax2)", "plt.sca(ax1)", "plt.gca() is ax1",
    ]),
    ("colorbar_across_figures", [
        "fig1, ax1 = plt.subplots()", "fig2, ax2 = plt.subplots()", "im = ax1.imshow(z)", "cb = fig2.colorbar(im)",
        "cb.ax.figure is fig2", "cb2 = fig2.colorbar(im, ax=ax1)", "fig1.canvas.draw()", "fig2.canvas.draw()",
        "im.remove()", "cb.update_normal(im)", "fig2.canvas.draw()", "cb.remove()", "cb.remove()",
    ]),
    ("legend_across_figures", [
        "fig1, ax1 = plt.subplots()", "fig2, ax2 = plt.subplots()", "l1, = ax1.plot([1, 2], label='one')",
        "leg = ax2.legend(handles=[l1])", "[t.get_text() for t in leg.get_texts()]", "l1.remove()",
        "fig2.canvas.draw()", "leg.remove()", "leg.remove()", "ax2.legend()", "fig2.legend(handles=[l1])",
        "fig2.canvas.draw()",
    ]),
    ("twins_removed", [
        "fig, ax = plt.subplots()", "tx = ax.twinx()", "ty = ax.twiny()", "tx.plot([1, 3])", "fig.canvas.draw()",
        "tx.remove()", "ax.get_shared_x_axes().get_siblings(ax)", "fig.canvas.draw()", "ax.remove()",
        "ty.get_xlim()", "fig.canvas.draw()", "ax.twinx()", "fig.axes",
    ]),
    ("pyplot_after_close", [
        "fig = plt.figure()", "plt.close('all')", "plt.get_fignums()", "plt.gcf() is fig", "plt.gca()",
        "plt.close(fig)", "plt.close('nope')", "plt.close(12345)", "plt.figure(fig.number)", "plt.get_fignums()",
        "plt.close(fig.number)", "plt.close(None)", "plt.close(3.5)", "plt.figure(num='named')",
        "plt.close('named')", "plt.figure(num='named') is plt.figure(num='named')", "plt.close('all')",
    ]),
    ("layout_changed_late", [
        "fig, axs = plt.subplots(2, 2)", "fig.canvas.draw()", "fig.set_layout_engine('constrained')",
        "fig.colorbar(axs[0, 0].imshow(z), ax=axs)", "fig.canvas.draw()", "fig.set_layout_engine('tight')",
        "fig.set_layout_engine('none')", "fig.subplots_adjust(left=0.3)", "fig.get_layout_engine()",
        "fig.set_layout_engine('compressed')", "fig.canvas.draw()", "fig.tight_layout()",
        "[a.get_position().bounds for a in fig.axes]",
    ]),
    ("scale_after_draw", [
        "fig, ax = plt.subplots()", "ax.plot([-1, 0, 1, 10], [-5, 0, 5, 50])", "fig.canvas.draw()",
        "ax.set_xscale('log')", "ax.get_xlim()", "fig.canvas.draw()", "ax.set_yscale('logit')", "ax.get_ylim()",
        "fig.canvas.draw()", "ax.set_xscale('linear')", "ax.autoscale()", "ax.get_xlim()",
        "ax.set_yscale('symlog')", "fig.canvas.draw()", "ax.get_ylim()",
    ]),
    ("limits_before_data", [
        "fig, ax = plt.subplots()", "ax.set_xlim(5, 1)", "ax.plot([0, 10], [0, 1])", "ax.get_xlim()",
        "ax.set_ylim(bottom=2)", "ax.autoscale()", "ax.get_ylim()", "ax.invert_xaxis()", "ax.get_xlim()",
        "ax.set_xlim(None, None)", "ax.set_aspect('equal')", "fig.canvas.draw()", "ax.get_position().bounds",
        "ax.margins(-0.4)", "ax.margins(0.6)", "ax.get_xlim()",
    ]),
    ("artists_after_clf", [
        "fig = plt.figure()", "ax = fig.add_subplot()", "line, = ax.plot([1, 2])", "t = fig.text(0.5, 0.5, 't')",
        "fig.clf()", "line.axes", "line.figure", "t.figure", "ax.figure", "fig.axes", "ax.plot([3, 4])",
        "fig.canvas.draw()", "fig.add_axes(ax)", "fig.add_subplot(ax)", "fig.canvas.draw()",
    ]),
    ("draw_twice_changed", [
        "fig, ax = plt.subplots()", "line, = ax.plot([1, 2, 3])", "fig.canvas.draw()", "line.set_data([], [])",
        "fig.canvas.draw()", "ax.relim()", "ax.autoscale_view()", "ax.get_xlim()", "line.set_data([1], [2])",
        "ax.relim()", "ax.autoscale_view()", "ax.get_xlim()", "fig.canvas.draw()", "fig.canvas.draw()",
        "line.get_window_extent().bounds",
    ]),
]


def _adder(obj: Any) -> str | None:
    """The ``Axes`` method adding an artist of ``obj``'s kind."""
    names = {c.__name__ for c in type(obj).__mro__}
    return next((method for kind, method in ADDERS if kind in names), None)


def _scenarios(setup: list[str], namespace: dict[str, Any]) -> list[str]:
    """The subject's scenarios, each on the subject made anew."""
    obj = namespace.get("obj")
    if not callable(getattr(obj, "remove", None)) or "fig" not in namespace:
        return []
    with_axes = "ax" in namespace and namespace.get("ax") is not obj
    adder = _adder(obj)
    scenarios = {"removed": REMOVED, "closed": CLOSED, "moved": MOVED, "cleared": CLEARED, "deleted": []}
    out = []
    for name, statements in scenarios.items():
        extra = WITH_AXES[name] if with_axes else []
        if name == "moved" and with_axes and adder:
            extra = [*extra, f"ax2.{adder}(obj)"]
        if statements or extra:
            out += [*setup, *statements, *extra]
    return out


def programs() -> Iterator[tuple[str, list[str]]]:
    """A program per artist subject, then the figures' programs."""
    for name, setup in artists.SUBJECTS:
        try:
            namespace = common.namespace(PRELUDE + setup)
        except Exception:  # noqa: BLE001 - a subject matplotlib cannot make here is left out
            continue
        body = _scenarios(setup, namespace)
        if body:
            yield name, PRELUDE + body + ENDING
    for name, statements in FIGURES:
        yield name, PRELUDE + statements + ENDING
