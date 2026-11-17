"""
S and P: every getter of drawn artists, swept.

S calls every ``get_*`` method that takes no argument of 27 kinds of
artist made from ints (and numpy ints) and drawn. P sets every property
whose setter's documentation accepts a float to an int, a numpy int64 and
a numpy float32, and reads it back.
"""

import inspect

import numpy as np

import pltrs.pyplot as plt
from pltrs import patches
from pltrs.artist import ArtistInspector

from .describe import describe

#: Getters the sweep leaves out: they return objects or need a renderer.
SKIPPED = {"get_children", "get_figure", "get_cursor_data", "get_window_extent",
           "get_tightbbox", "get_transform", "get_tick_iterators",
           "get_default_bbox_extra_artists", "get_renderer_cache", "get_shared_x_axes",
           "get_shared_y_axes", "get_tight_layout", "get_constrained_layout_pads"}

#: Properties P leaves alone: they take no number, or replace the data.
NOT_NUMBERS = {"figure", "transform", "clip_path", "data", "xdata", "ydata"}

#: The artists P does not set properties of.
UNSET = {"Figure", "XAxis", "Colorbar axes", "Axes", "Legend"}


def made_artists():
    """The 27 artists, drawn: ``{label: artist}``."""
    fig, (ax, ax2) = plt.subplots(1, 2, figsize=(5, 3), dpi=72)
    made = {
        "Line2D": ax.plot([1, 2, 3], [4, 5, 6], lw=2, ms=6, marker="o")[0],
        "Line2D np.int64": ax.plot(np.arange(3), np.arange(3) * 2, lw=np.int64(2))[0],
        "Rectangle": ax.add_patch(patches.Rectangle((1, 2), 3, 4, lw=2)),
        "Circle": ax.add_patch(patches.Circle((1, 1), 2)),
        "Ellipse": ax.add_patch(patches.Ellipse((0, 0), 4, 2, angle=45)),
        "Wedge": ax.add_patch(patches.Wedge((0, 0), 1, 30, 60)),
        "Polygon": ax.add_patch(patches.Polygon([[0, 0], [1, 0], [1, 1]])),
        "FancyBboxPatch": ax.add_patch(patches.FancyBboxPatch((1, 2), 3, 4)),
        "FancyArrowPatch": ax.add_patch(patches.FancyArrowPatch(
            (0, 0), (1, 1), shrinkA=3, mutation_scale=10)),
        "Arc": ax.add_patch(patches.Arc((0, 0), 2, 2, theta1=0, theta2=90)),
        "RegularPolygon": ax.add_patch(patches.RegularPolygon((0, 0), 5, radius=2)),
        "Text": ax.text(1, 2, "t", rotation=30, fontsize=12),
        "Annotation": ax.annotate("a", (1, 2), (3, 4),
                                  arrowprops=dict(arrowstyle="->", shrinkA=2)),
        "PathCollection": ax.scatter([1, 2], [3, 4], s=[10, 20], c=[1, 2], linewidths=2),
        "LineCollection": ax.vlines([1, 2], 0, [3, 4], linewidths=2),
        "PolyCollection": ax.fill_between([1, 2, 3], [0, 1, 0]),
        "EventCollection": ax.eventplot([1, 2], lineoffsets=1)[0],
        "BarContainer patch": ax.bar([1, 2], [3, 4])[0],
        "AxesImage": ax2.imshow(np.arange(6).reshape(2, 3), extent=(0, 3, 0, 2)),
        "QuadMesh": ax2.pcolormesh(np.arange(4).reshape(2, 2)),
        "QuadContourSet": ax2.contour(np.arange(16).reshape(4, 4), levels=[2, 5]),
        "Legend": ax.legend(["x"], loc=2, ncols=1, borderpad=1),
    }
    made["Colorbar axes"] = fig.colorbar(made["AxesImage"]).ax
    fig.canvas.draw()
    made["Axes"] = ax
    made["XAxis"] = ax.xaxis
    made["YTick"] = ax.yaxis.get_major_ticks()[1]
    made["Spine"] = ax.spines["left"]
    made["Figure"] = fig
    return made


def takes_no_argument(method):
    """Whether *method* can be called without arguments."""
    try:
        signature = inspect.signature(method)
    except (TypeError, ValueError):
        return False
    return not any(parameter.default is parameter.empty
                   and parameter.kind in (parameter.POSITIONAL_ONLY,
                                          parameter.POSITIONAL_OR_KEYWORD)
                   for parameter in signature.parameters.values())


def getters(rows):
    """S: every getter without arguments of every artist."""
    for label, artist in made_artists().items():
        for name in sorted(dir(artist)):
            if not name.startswith("get_") or name in SKIPPED:
                continue
            method = getattr(artist, name, None)
            if callable(method) and takes_no_argument(method):
                rows.row(f"S {label}", f"{name}()", method)


def properties(rows):
    """P: every float property set to numbers of other types, read back."""
    for label, artist in made_artists().items():
        if label in UNSET:
            continue
        inspector = ArtistInspector(artist)
        for prop in sorted(inspector.get_setters()):
            if "float" not in inspector.get_valid_values(prop) or prop in NOT_NUMBERS:
                continue
            getter = getattr(artist, "get_" + prop, None)
            if getter is None:
                continue
            for given in (2, np.int64(2), np.float32(2.5)):
                def probe(a=artist, p=prop, g=given, get=getter):
                    getattr(a, "set_" + p)(g)
                    return get()
                rows.row(f"P {label}", f"set_{prop}({describe(given)}) then get_{prop}()", probe)


SCENES = (getters, properties)
