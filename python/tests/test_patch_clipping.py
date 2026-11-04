"""
What an Axes clips its artists to, as matplotlib's ``set_clip_path(ax.patch)``
reports it: the patch's `TransformedPatchPath` and no clip box when the
patch is no `Rectangle` (polar, geographic Axes, Axes of the program's
making a circle), the box of the patch when it is one (the unit square, or
a smaller rectangle of the program's); for every artist kind the Axes
clips, the gridlines of its ticks and its axes; and what clearing the clip
path or the clip box leaves. Each ``scenario_*`` returns what its test
compares; the expected values were recorded from matplotlib 3.11.2 running
the same scenarios.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs.axes import Axes
from pltrs.collections import LineCollection
from pltrs.lines import Line2D
from pltrs.patches import Circle, Rectangle
from pltrs.projections import register_projection

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

register_projection(type("round_clipping", (Axes,), {
    "name": "round_clipping",
    "_gen_axes_patch": lambda self: Circle((0.5, 0.5), 0.5)}))
register_projection(type("inset_clipping", (Axes,), {
    "name": "inset_clipping",
    "_gen_axes_patch": lambda self: Rectangle((0.1, 0.1), 0.8, 0.8)}))

#: The projections compared, the rectilinear one last.
PROJECTIONS = ["polar", "mollweide", "round_clipping", "inset_clipping", None]


def _clipping(artist, ax):
    """The clip path (its class, whether it is the Axes' patch's) and the
    clip box (its class and bounds) of *artist*."""
    path, box = artist.get_clip_path(), artist.get_clip_box()
    return (None if path is None
            else (type(path).__name__, getattr(path, "_patch", None) is ax.patch),
            None if box is None
            else (type(box).__name__, [round(float(v), 1) for v in box.bounds]))


def scenario_artists():
    """The artists of each kind an Axes adds, in each projection, drawn."""
    fig = plt.figure(figsize=(6, 4), dpi=50)
    out = {}
    for i, projection in enumerate(PROJECTIONS):
        ax = fig.add_subplot(2, 3, i + 1, projection=projection)
        artists = {
            "line": ax.plot([0, 1], [0, 1])[0],
            "scatter": ax.scatter([0, 1], [0, 1]),
            "patch": ax.add_patch(Rectangle((0, 0), 0.5, 0.5)),
            "text": ax.text(0.1, 0.1, "t"),
            "annotation": ax.annotate("a", (0.2, 0.2)),
            "collection": ax.add_collection(LineCollection([[(0, 0), (1, 1)]])),
            "artist": ax.add_artist(Line2D([0, 1], [1, 0])),
            "fill": ax.fill_between([0, 1], [0, 1]),
        }
        if projection in (None, "round_clipping", "inset_clipping"):
            artists["image"] = ax.imshow(np.zeros((2, 2)))
        fig.canvas.draw()
        out[projection] = ({name: _clipping(artist, ax)
                            for name, artist in artists.items()},
                           type(ax.patch).__name__)
    return out


def test_artists():
    polar = ("TransformedPatchPath", True), None
    box = None, ("TransformedBbox", [119.6, 22.8, 68.4, 68.4])
    inset = None, ("TransformedBbox", [44.3, 29.6, 54.7, 54.7])
    kinds = ["line", "scatter", "patch", "text", "collection", "artist", "fill"]

    def every(clipping, image=False):
        found = dict.fromkeys(kinds, clipping)
        found["annotation"] = (None, None)
        if image:
            found["image"] = clipping
        return found

    assert scenario_artists() == {
        "polar": (every(polar), "Wedge"),
        "mollweide": (every(polar), "Circle"),
        "round_clipping": (every(polar, image=True), "Circle"),
        "inset_clipping": (every(inset, image=True), "Rectangle"),
        None: (every(box, image=True), "Rectangle"),
    }


def scenario_axis_parts():
    """The parts of the axes of an Axes in each projection, the same clip
    path asked again, and a line whose clip path, then clip box, is
    cleared."""
    fig = plt.figure(figsize=(6, 4), dpi=50)
    out = {}
    for i, projection in enumerate(["polar", "hammer", "round_clipping", None]):
        ax = fig.add_subplot(2, 2, i + 1, projection=projection)
        line, = ax.plot([0, 1], [0, 1])
        fig.canvas.draw()
        tick = ax.xaxis.get_major_ticks()[0]
        parts = {"gridline": tick.gridline, "tick1": tick.tick1line,
                 "label": tick.label1, "spine": list(ax.spines.values())[0],
                 "xaxis": ax.xaxis, "offset": ax.xaxis.offsetText,
                 "title": ax.title, "patch": ax.patch}
        found = {name: _clipping(part, ax) for name, part in parts.items()}
        found["same"] = line.get_clip_path() is line.get_clip_path()
        path = line.get_clip_path()
        if path is not None:
            found["extents"] = [round(float(v), 1) for v in
                                path.get_fully_transformed_path().get_extents().bounds]
        line.set_clip_path(None)
        found["cleared_path"] = _clipping(line, ax)
        line.set_clip_box(None)
        found["cleared_box"] = _clipping(line, ax)
        out[projection] = found
    return out


def test_axis_parts():
    path = ("TransformedPatchPath", True), None
    unclipped = None, None
    out = scenario_axis_parts()
    for projection, extents in [("polar", [55.3, 106.0, 70.0, 70.0]),
                                ("hammer", [164.3, 114.6, 105.7, 52.8]),
                                ("round_clipping", [37.5, 22.0, 105.7, 70.0])]:
        assert out[projection] == {
            "gridline": path, "tick1": unclipped, "label": unclipped,
            "spine": unclipped, "xaxis": path, "offset": unclipped,
            "title": unclipped, "patch": unclipped, "same": True,
            "extents": extents, "cleared_path": unclipped,
            "cleared_box": unclipped,
        }, projection
    box = None, ("TransformedBbox", [164.3, 22.0, 105.7, 70.0])
    assert out[None] == {
        "gridline": box, "tick1": unclipped, "label": unclipped,
        "spine": unclipped, "xaxis": box, "offset": unclipped,
        "title": unclipped, "patch": unclipped, "same": True,
        "cleared_path": box, "cleared_box": unclipped,
    }
