"""
What the tenth round of corpora (``cargo xtask diff --corpus``: libraries'
own documentation) showed pltrs doing otherwise than matplotlib 3.11,
reduced to matplotlib's API. Every expected value was recorded from
matplotlib.
"""

import io
import logging
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.colorizer import Colorizer
import pltrs.transforms as mtransforms
from pltrs.offsetbox import AnchoredOffsetbox, AnnotationBbox, HPacker, TextArea, VPacker
from pltrs.text import Text


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Lookups(logging.Handler):
    """Counts the font manager's lines about a family it does not have."""

    def __init__(self, family):
        super().__init__()
        self.family = family
        self.count = 0

    def emit(self, record):
        if self.family in record.getMessage():
            self.count += 1


def _lookups_per_draw(build, family):
    """The lookups of *family* logged by a figure's first and second draw."""
    handler = _Lookups(family)
    root = logging.getLogger()
    level = root.level
    root.addHandler(handler)
    root.setLevel(logging.WARNING)
    try:
        fig, ax = plt.subplots()
        build(ax)
        counts = []
        for _ in range(2):
            handler.count = 0
            fig.canvas.draw()
            counts.append(handler.count)
        return counts
    finally:
        root.removeHandler(handler)
        root.setLevel(level)


def _area(text, family, **props):
    return TextArea(text, textprops=dict(family=family, **props))


@pytest.mark.parametrize("name, build, expected", [
    # matplotlib's AnchoredOffsetbox.draw measures its child twice
    # (get_window_extent, then get_bbox), each measure laying the texts of
    # its text areas out (flexitext's rich texts).
    ("area", lambda ax, f: ax.add_artist(AnchoredOffsetbox(
        loc="center", child=_area("one", f))), [6, 4]),
    ("area of two lines", lambda ax, f: ax.add_artist(AnchoredOffsetbox(
        loc="center", child=_area("one\ntwo", f))), [8, 5]),
    ("packed areas", lambda ax, f: ax.add_artist(AnchoredOffsetbox(
        loc="center", child=HPacker(children=[_area("one", f), _area("two", f, size=20)]))),
     [14, 10]),
    ("nested packers", lambda ax, f: ax.add_artist(AnchoredOffsetbox(
        loc="center", child=VPacker(children=[HPacker(children=[_area("one", f)]),
                                              _area("two", f, size=20)]))), [15, 11]),
    # An annotation box measures its child once per draw.
    ("annotation box", lambda ax, f: ax.add_artist(AnnotationBbox(
        _area("one", f), (.5, .5), frameon=False)), [6, 4]),
])
def test_anchored_boxes_look_their_fonts_up_as_matplotlib_s(name, build, expected):
    family = f"No Such Family For {name}"
    assert _lookups_per_draw(lambda ax: build(ax, family), family) == expected


def test_legend_handles_of_collections_keep_their_alpha_arrays():
    # librosa's timbre tutorial sets them all opaque, alpha by alpha.
    fig, ax = plt.subplots()
    ax.scatter([1, 2, 3], [1, 2, 3], alpha=np.array([0.2, 0.5, 1.0]), label="a")
    ax.fill_between([0, 1], [0, 1], alpha=0.3, label="b")
    handles = ax.legend().legend_handles
    assert [None if h.get_alpha() is None else np.asarray(h.get_alpha()).tolist()
            for h in handles] == [[0.2, 0.5, 1.0], None]
    handles[0].set_alpha(np.ones_like(handles[0].get_alpha()))
    assert np.asarray(handles[0].get_alpha()).tolist() == [1.0, 1.0, 1.0]


def test_reference_lines_at_an_array_of_one_value():
    # librosa's tempo estimates are arrays of one value.
    fig, ax = plt.subplots()
    hline = ax.axhline(np.array([120.0]), color="r")
    vline = ax.axvline(np.array([3.0]))
    fig.canvas.draw()
    assert [np.asarray(y).tolist() for y in hline.get_ydata()] == [[120.0], [120.0]]
    assert [np.asarray(x).tolist() for x in vline.get_xdata()] == [[3.0], [3.0]]
    assert np.round(ax.get_ylim(), 3).tolist() == [113.4, 126.6]
    assert np.round(ax.get_xlim(), 3).tolist() == [2.835, 3.165]


def test_pyplot_scatter_into_a_3d_axes_takes_2d_scatter_parameters():
    # pyplot passes colorizer and plotnonfinite on (pymoo's fitness
    # landscapes scatter into the 3-D axes left current).
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    assert type(plt.scatter([0, 1], [0, 1], s=20, facecolors="none",
                            edgecolors="r")).__name__ == "Path3DCollection"
    points = ax.scatter([0, 1, 2], [0, 1, 2], c=[0.0, 1.0, 2.0],
                        colorizer=Colorizer(cmap="viridis"))
    fig.canvas.draw()
    assert points.get_cmap().name == "viridis"
    assert np.round(points.get_facecolor(), 3).tolist() == [
        [0.993, 0.906, 0.144, 0.776], [0.128, 0.567, 0.551, 0.886], [0.267, 0.005, 0.329, 1.0]]


def test_errorbar_bars_and_caps_take_its_transform():
    # mplhep draws the side histogram of hist's plot2d_full rotated.
    fig, ax = plt.subplots()
    rotated = mtransforms.Affine2D().rotate_deg(90).scale(-1, 1) + ax.transData
    line, caps, bars = ax.errorbar([0, 1], [1, 2], yerr=[0.1, 0.2], capsize=2,
                                   linestyle="none", transform=rotated)
    assert all(part.get_transform() == rotated for part in (line, *caps, *bars))


@pytest.mark.parametrize("fmt, xerr, expected", [
    ("none", None, [[6.8377, 0.5], [46.3246, 2.5]]),
    ("none", 0.5, [[6.8377, 0.0], [46.3246, 3.0]]),
    ("o", None, [[6.8377, 0.5], [46.3246, 2.5]]),
    ("o", 0.5, [[6.8377, 0.0], [46.3246, 3.0]]),
])
def test_errorbar_data_limits_follow_its_transform(fmt, xerr, expected):
    # The bars and the caps extend the data limits through the rotation
    # (hist's side histogram scaled its count axis to the counts alone).
    fig, ax = plt.subplots()
    rotated = mtransforms.Affine2D().rotate_deg(90).scale(-1, 1) + ax.transData
    y = np.array([10.0, 40.0, 20.0])
    ax.errorbar([0.5, 1.5, 2.5], y, yerr=np.sqrt(y), xerr=xerr, fmt=fmt, transform=rotated)
    assert np.round(ax.dataLim.get_points(), 4).tolist() == expected


class _TurnedLabel(Text):
    """A label computing its rotation in a property, as labellines' do."""

    def __init__(self, line, *args, **kwargs):
        self._line = line
        self._turn = 30.0
        super().__init__(*args, rotation=self._rotation, rotation_mode="anchor", **kwargs)
        line.axes._add_text(self)

    @property
    def _rotation(self):
        return self._turn

    @_rotation.setter
    def _rotation(self, value):
        self._turn = value


@pytest.mark.parametrize("pickler", [pickle.Pickler, pickle._Pickler])
def test_figures_pickle_with_the_pure_python_pickler(pickler):
    # multiprocess sends tasks with dill, a pure-Python pickler, which
    # pickled scqubits' figures of labellines' labels into every task.
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    _TurnedLabel(line, 1.5, 1.5, "a")
    buffer = io.BytesIO()
    pickler(buffer).dump(fig)
    restored = pickle.loads(buffer.getvalue())
    label, = restored.axes[0].texts
    assert type(label) is _TurnedLabel
    assert label._line.figure is restored
    assert label.get_rotation() == 30.0
