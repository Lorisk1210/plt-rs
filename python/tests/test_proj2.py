"""
What a program's projection does beyond axes of its own (task PROJ2): a
class deriving from one of matplotlib's geographic Axes that replaces
the projection's transform, an Axes with a third axis in the layout
engines and the figure's helpers, and pans mapped back through the
program's transform. No library is imported: the geographic classes are
defined here (a sinusoidal projection on each of matplotlib's geographic
classes), the Axes with a third axis in ``curve_projection.py``. The
expected values (`RECORDED`) were recorded from matplotlib 3.11, under
which this file passes too.

A program's subclass of a geographic class is a projection class of the
program's: a rectilinear pltrs (Rust) Axes that matplotlib's own
``GeoAxes`` code makes geographic; see "Projections of programs" in
``python/DESIGN.md``.
"""

import collections
import io
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.patheffects as path_effects
import pltrs.pyplot as plt
import pltrs.transforms as mtransforms
from pltrs.axes import Axes
from pltrs.path import Path
from pltrs.projections import register_projection
from pltrs.projections.geo import (
    AitoffAxes, GeoAxes, HammerAxes, LambertAxes, MollweideAxes)
from pltrs.ticker import FixedLocator, NullFormatter

from curve_projection import CurveAxes, CurveAxis, curve_axes

#: How often each method of the sinusoidal projection was called, and the
#: ``(vertices, interpolation steps)`` of the paths it was asked to map.
CALLS = collections.Counter()
PATHS = []


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        CALLS.clear()
        del PATHS[:]
        yield
    plt.close("all")


class Sinusoidal(mtransforms.Transform):
    """Longitudes shrunk by the cosine of the latitude."""

    input_dims = output_dims = 2

    def __init__(self, resolution):
        super().__init__()
        self._resolution = resolution

    def transform_non_affine(self, values):
        CALLS["transform_non_affine"] += 1
        longitude, latitude = np.asarray(values, float).T
        return np.column_stack([longitude * np.cos(latitude), latitude])

    def transform_path_non_affine(self, path):
        CALLS["transform_path_non_affine"] += 1
        PATHS.append((len(path.vertices), path._interpolation_steps))
        ipath = path.interpolated(self._resolution)
        return Path(self.transform(ipath.vertices), ipath.codes)

    def inverted(self):
        CALLS["inverted"] += 1
        return InvertedSinusoidal(self._resolution)


class InvertedSinusoidal(mtransforms.Transform):
    input_dims = output_dims = 2

    def __init__(self, resolution):
        super().__init__()
        self._resolution = resolution

    def transform_non_affine(self, values):
        x, y = np.asarray(values, float).T
        return np.column_stack([x / np.cos(y), y])

    def inverted(self):
        return Sinusoidal(self._resolution)


def _sinusoidal(base):
    """A projection class deriving from *base*, one of matplotlib's
    geographic classes, with the sinusoidal transform for its own."""
    class SinusoidalAxes(base):
        name = f"proj2_sinusoidal_{base.__name__.lower()}"
        RESOLUTION = 20

        if base is GeoAxes:
            # What each of matplotlib's geographic classes does.
            def __init__(self, *args, **kwargs):
                self._longitude_cap = np.pi / 2.0
                super().__init__(*args, **kwargs)
                self.set_aspect(0.5, adjustable="box", anchor="C")
                self.clear()

        def _get_core_transform(self, resolution):
            CALLS["_get_core_transform"] += 1
            return Sinusoidal(resolution)

    # A class of the module, which a pickle of its Axes names.
    SinusoidalAxes.__name__ = SinusoidalAxes.__qualname__ = f"Sinusoidal{base.__name__}"
    globals()[SinusoidalAxes.__name__] = SinusoidalAxes
    register_projection(SinusoidalAxes)
    return SinusoidalAxes


BASES = (AitoffAxes, HammerAxes, MollweideAxes, LambertAxes, GeoAxes)
SINUSOIDAL = {base: _sinusoidal(base) for base in BASES}

#: What matplotlib 3.11 gives for a sinusoidal Axes of 500 by 400 pixels:
#: on `.LambertAxes`, whose affine part and aspect are its own, and on
#: the other classes.
_ON_LAMBERT = {
    "made": {"_get_core_transform": 1},
    "centre": (0, 0),
    "affine": [[0.25, 0.0, 0.5], [0.0, 0.25, 0.5], [0.0, 0.0, 1.0]],
    "aspect": 1.0,
    "y formatter": "NullFormatter",
    "first draw": {"transform_non_affine": 101, "transform_path_non_affine": 23},
    "second draw": {"transform_non_affine": 99, "transform_path_non_affine": 22},
    "data": [[278.8895, 213.4], [188.6761, 236.5]],
    "bbox": [102.25, 44.0, 308.0, 308.0],
    "y labels": ["", "", ""],
    "x label box": [35.16, 202.0, 39.0, 13.89],
    "y label box": [185.64, 87.13, 0.0, 0.0],
    "xlabel": [222.75, 24.108, 67.0, 14.337],
    "tight": [35.164, 24.108, 439.671, 327.892],
    "back": [[-0.081196, 0.025974]],
    "pixels": (442, 2998, 86),
}
_ON_THE_OTHERS = {
    "made": {"_get_core_transform": 2, "transform_non_affine": 2},
    "centre": None,
    "affine": [[0.159155, 0.0, 0.5], [0.0, 0.31831, 0.5], [0.0, 0.0, 1.0]],
    "aspect": 0.5,
    "y formatter": "ThetaFormatter",
    "first draw": {"transform_non_affine": 112, "transform_path_non_affine": 23},
    "second draw": {"transform_non_affine": 110, "transform_path_non_affine": 22},
    "data": [[274.383, 210.3345], [202.1273, 228.8363]],
    "bbox": [62.5, 101.125, 387.5, 193.75],
    "y labels": ["-75°", "-60°", "-45°"],
    "x label box": [75.29, 202.0, 39.0, 13.89],
    "y label box": [168.1, 102.25, 30.0, 13.89],
    "xlabel": [222.75, 81.233, 67.0, 14.337],
    "tight": [31.102, 81.233, 418.898, 213.642],
    "back": [[-0.101395, 0.032429]],
    "pixels": (358, 3103, 155),
}


def _bounds(bbox, digits):
    return [round(float(v), digits) for v in bbox.bounds]


def _close(got, expected, abs=1e-3):
    assert np.asarray(got, float) == pytest.approx(np.asarray(expected, float), abs=abs)


@pytest.mark.parametrize("base", BASES, ids=lambda base: base.__name__)
def test_a_subclass_of_a_geographic_class_draws_through_its_own_transform(base):
    recorded = _ON_LAMBERT if base is LambertAxes else _ON_THE_OTHERS
    fig = plt.figure(figsize=(5, 4), dpi=100)
    ax = fig.add_subplot(projection=SINUSOIDAL[base].name)
    # matplotlib's constructor asks the class for its transform, with the
    # projection's resolution and with 1 for the affine part.
    assert dict(CALLS) == recorded["made"]
    assert isinstance(ax.transProjection, Sinusoidal)
    assert (ax._longitude_cap, type(ax._longitude_cap)) == (np.deg2rad(75), np.float64)
    assert ax.RESOLUTION == 20
    centre = (ax._center_longitude, ax._center_latitude) if base is LambertAxes else None
    assert centre == recorded["centre"]
    _close(ax.transAffine.get_matrix(), recorded["affine"], abs=1e-6)
    _close(ax._xaxis_pretransform.get_matrix(),
           [[1.0, 0.0, 0.0], [0.0, 2.617994, -1.308997], [0.0, 0.0, 1.0]], abs=1e-6)
    assert ax._xaxis_transform is ax.get_xaxis_transform()
    assert ax._yaxis_transform is ax.get_yaxis_transform()
    assert ax.get_xaxis_text1_transform(0) == (ax._xaxis_text1_transform, "bottom", "center")
    assert ax.get_xlim() == (-np.pi, np.pi) and ax.get_ylim() == (-np.pi / 2, np.pi / 2)
    assert (ax.get_aspect(), ax.get_adjustable(), ax.get_anchor()) == (
        recorded["aspect"], "box", "C")
    assert list(ax.spines) == ["geo"] and type(ax.patch).__name__ == "Circle"
    assert ax.xaxis.get_ticks_position() == ax.yaxis.get_ticks_position() == "unknown"
    assert type(ax.xaxis.get_major_locator()) is FixedLocator
    assert type(ax.yaxis.get_major_formatter()).__name__ == recorded["y formatter"]
    assert (len(ax.get_xticks()), len(ax.get_yticks())) == (11, 11)

    ax.plot(np.linspace(-2, 2, 30), np.linspace(-1, 1, 30))
    ax.set_xlabel("longitude")
    ax.grid(True)
    # A draw maps each of the 22 gridlines, which ask for matplotlib's 180
    # interpolation steps, and the line; the next maps the gridlines only:
    # the line keeps its path until the transform is invalidated.
    CALLS.clear()
    fig.canvas.draw()
    assert dict(CALLS) == recorded["first draw"]
    assert collections.Counter(PATHS) == {(2, 180): 22, (30, 1): 1}
    CALLS.clear()
    del PATHS[:]
    fig.canvas.draw()
    assert dict(CALLS) == recorded["second draw"]
    assert collections.Counter(PATHS) == {(2, 180): 22}

    _close(ax.transData.transform([(0.3, 0.2), (-1.0, 0.5)]), recorded["data"])
    _close(ax.bbox.bounds, recorded["bbox"])
    assert [t.get_text() for t in ax.get_xticklabels()][:3] == ["-150°", "-120°", "-90°"]
    assert [t.get_text() for t in ax.get_yticklabels()][:3] == recorded["y labels"]
    _close(ax.get_xticklabels()[0].get_window_extent().bounds, recorded["x label box"], abs=0.01)
    # (A label without text is a point where it would be.)
    _close(ax.get_yticklabels()[0].get_window_extent().bounds, recorded["y label box"], abs=0.01)
    # The class made one spine, around the outline: the label hangs below
    # the Axes, not below a bottom spine.
    _close(ax.xaxis.label.get_window_extent().bounds, recorded["xlabel"])
    assert ax.xaxis.label.get_window_extent().y1 == pytest.approx(
        ax.bbox.y0 - ax.xaxis.labelpad * fig.dpi / 72)
    _close(ax.get_tightbbox().bounds, recorded["tight"])
    # The transform is asked for its inverse where a point goes back.
    CALLS.clear()
    _close(ax.transData.inverted().transform([(250.0, 200.0)]), recorded["back"], abs=1e-6)
    assert dict(CALLS) == {"inverted": 1}

    rgba = np.asarray(fig.canvas.buffer_rgba())
    line = int(((rgba[..., 2] > 150) & (rgba[..., 0] < 100)).sum())
    grid = int(((rgba[..., 0] == 176) & (rgba[..., 1] == 176)).sum())
    black = int((rgba[..., :3].sum(axis=2) == 0).sum())
    assert (line, grid, black) == recorded["pixels"]


def test_a_line_with_markers_maps_its_vertices_once():
    # The markers go where the line's path put its vertices: a draw maps
    # the path and its vertices once, the next maps nothing.
    fig = plt.figure(figsize=(5, 4), dpi=100)
    ax = fig.add_subplot(projection=SINUSOIDAL[HammerAxes].name)
    ax.grid(False)
    ax.plot([-1, 1, 1], [-1, -1, 1], "o-")
    CALLS.clear()
    fig.canvas.draw()
    assert dict(CALLS) == RECORDED["markers", "first draw"]
    assert PATHS == [(3, 1)]
    CALLS.clear()
    fig.canvas.draw()
    assert dict(CALLS) == RECORDED["markers", "second draw"]


@pytest.mark.parametrize("base", BASES, ids=lambda base: base.__name__)
def test_the_grids_and_limits_of_a_subclass_are_matplotlibs(base):
    fig = plt.figure(figsize=(5, 4), dpi=100)
    ax = fig.add_subplot(projection=SINUSOIDAL[base].name)
    ax.set_longitude_grid(60)
    ax.set_latitude_grid(30)
    ax.set_longitude_grid_ends(60)
    _close(ax.get_xticks(), np.deg2rad([-120, -60, 0, 60, 120]), abs=1e-12)
    _close(ax.get_yticks(), np.deg2rad([-60, -30, 0, 30, 60]), abs=1e-12)
    assert ax._longitude_cap == np.deg2rad(60)
    _close(ax._xaxis_pretransform.get_matrix(),
           [[1.0, 0.0, 0.0], [0.0, 2.094395, -1.047198], [0.0, 0.0, 1.0]], abs=1e-6)
    assert ax.xaxis.get_major_formatter()(np.deg2rad(60)) == "60°"
    with pytest.raises(TypeError, match="Changing axes limits of a geographic"):
        ax.set_xlim(0, 1)
    with pytest.raises(NotImplementedError):
        ax.set_yscale("log")
    with pytest.raises(TypeError):
        ax.invert_xaxis()
    assert (ax.can_zoom(), ax.can_pan(), ax.get_data_ratio()) == (False, False, 1.0)
    assert ax.format_coord(0.5, 0.25) == "14.323945°N, 28.647890°E"
    # A clear brings matplotlib's grids back, through the class' methods.
    ax.clear()
    assert (len(ax.get_xticks()), len(ax.get_yticks())) == (11, 11)
    assert ax._longitude_cap == np.deg2rad(75)
    fig.canvas.draw()


def test_the_geographic_classes_keep_matplotlibs_private_attributes():
    # Those of pltrs (Rust)'s own Axes, which programs read.
    fig = plt.figure()
    for index, name in enumerate(["aitoff", "hammer", "mollweide"]):
        ax = fig.add_subplot(2, 2, index + 1, projection=name)
        assert ax._longitude_cap == np.deg2rad(75)
        assert ax.RESOLUTION == 75
        assert ax._xaxis_transform is ax.get_xaxis_transform()
        ax.set_longitude_grid_ends(60)
        assert ax._longitude_cap == np.deg2rad(60)
    lambert = fig.add_subplot(2, 2, 4, projection="lambert",
                              center_longitude=0.5, center_latitude=0.25)
    assert (lambert._center_longitude, lambert._center_latitude) == (0.5, 0.25)
    assert lambert._longitude_cap == np.deg2rad(75)
    transform = lambert._get_core_transform(7)
    assert (transform._center_longitude, transform._center_latitude,
            transform._resolution) == (0.5, 0.25, 7)
    _close(lambert._get_affine_transform().get_matrix(),
           [[0.25, 0, 0.5], [0, 0.25, 0.5], [0, 0, 1]])


# (matplotlib warns of a class overriding ``cla`` where it is defined.)
with pytest.warns(PendingDeprecationWarning, match="Overriding `Axes.cla`"):
    class _Rotated(LambertAxes):
        """A `.LambertAxes` turned by a quarter, with a grid of its own and a
        hidden Axes it makes while it is made (as mplstereonet's Axes keeps a
        polar one for its azimuth labels)."""

        name = "proj2_rotated"

        def __init__(self, *args, **kwargs):
            self.helper = None
            super().__init__(*args, **kwargs)

        def _get_affine_transform(self):
            return mtransforms.Affine2D().rotate(np.pi / 2).scale(0.25).translate(0.5, 0.5)

        def set_longitude_grid(self, degrees):
            self.xaxis.set_major_locator(FixedLocator(np.deg2rad([-90, 0, 90])))
            self.xaxis.set_major_formatter(self.ThetaFormatter(degrees))

        def cla(self):
            Axes.cla(self)
            if self.helper is None:
                fig = self.get_figure()
                self.helper = fig.add_axes(self.get_position(True), frameon=False,
                                           label="helper")
            self.set_longitude_grid(10)
            self.set_latitude_grid(45)
            self.set_longitude_grid_ends(80)
            self.yaxis.set_tick_params(label1On=False)
            Axes.set_xlim(self, -np.pi, np.pi)
            Axes.set_ylim(self, -np.pi / 2.0, np.pi / 2.0)


register_projection(_Rotated)


def test_a_subclass_replaces_the_affine_part_the_grids_and_the_clear():
    fig = plt.figure(figsize=(5, 4), dpi=100)
    ax = fig.add_subplot(projection="proj2_rotated")
    # The Axes joins its figure once its constructor returns: after the
    # Axes the constructor added.
    assert [a.get_label() for a in fig.axes] == ["helper", ""]
    assert fig.gca() is ax
    _close(ax.transAffine.get_matrix(), [[0, -0.25, 0.5], [0.25, 0, 0.5], [0, 0, 1]])
    # matplotlib's clear runs the class' ``cla``, then sets its own grids
    # through the class' methods.
    _close(ax.get_xticks(), np.deg2rad([-90, 0, 90]), abs=1e-12)
    assert len(ax.get_yticks()) == 11
    assert ax._longitude_cap == np.deg2rad(75)
    assert type(ax.yaxis.get_major_formatter()) is NullFormatter
    fig.canvas.draw()
    # North is to the left of a map turned by a quarter.
    _close(ax.transData.transform([(0.0, 0.0), (0.0, 1.0)]), RECORDED["turned"])


def test_labels_are_drawn_through_the_path_effects_they_were_given():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100, facecolor="k")
    ax.set_xlabel("label")
    ax.set_xticks([0.5])
    ax.set_yticks([])
    halo = [path_effects.withStroke(linewidth=4, foreground="w")]

    def white_below_the_axes():
        fig.canvas.draw()
        rgba = np.asarray(fig.canvas.buffer_rgba())
        below = rgba[int(fig.bbox.height - ax.bbox.y0) + 2:]
        return int((below[..., :3].sum(axis=2) == 765).sum())

    assert white_below_the_axes() == 0
    ax.xaxis.label.set_path_effects(halo)
    with_label = white_below_the_axes()
    for label in ax.get_xticklabels():
        label.set_path_effects(halo)
    with_both = white_below_the_axes()
    assert (with_label, with_both) == RECORDED["halos"]
    assert ax.xaxis.label.get_path_effects() == halo
    ax.xaxis.label.set_path_effects([])
    ax.get_xticklabels()[0].set_path_effects([])
    assert white_below_the_axes() == 0


def test_layout_engines_ask_a_held_axis_for_its_box_for_a_layout():
    asked = []

    class Asked(CurveAxis):
        def get_tightbbox(self, renderer=None, *, for_layout_only=False):
            asked.append(for_layout_only)
            return super().get_tightbbox(renderer, for_layout_only=for_layout_only)

    class AskedAxes(CurveAxes):
        name = "proj2_asked"

        def _init_axis(self):
            Axes._init_axis(self)
            self.caxis = Asked(self)
            self.spines["clow"].register_axis(self.caxis)

    register_projection(AskedAxes)
    for layout in ("constrained", "tight"):
        del asked[:]
        fig = plt.figure(figsize=(5, 4), dpi=100, layout=layout)
        ax = fig.add_subplot(projection="proj2_asked")
        ax.caxis.set_label_text("a long label of the curve axis " * 3)
        fig.canvas.draw()
        assert set(asked) == {True}, layout
        _close(ax.get_position().bounds, RECORDED["long label", layout], abs=1e-4)
        del asked[:]
        ax.get_tightbbox()
        fig.get_tightbbox()
        assert set(asked) == {False}
        del asked[:]
        ax.get_tightbbox(for_layout_only=True)
        assert set(asked) == {True}
    # An axis out of the layout makes no room for itself.
    ax.caxis.set_in_layout(False)
    fig.tight_layout()
    _close(ax.get_position().bounds, RECORDED["out of the layout"], abs=1e-4)


def test_the_helpers_of_the_figure_and_the_axes_take_a_third_axis():
    fig = plt.figure(figsize=(5, 4), dpi=100)
    axs = [fig.add_subplot(2, 2, i + 1, projection="proj1_curve") for i in range(4)]
    for i, ax in enumerate(axs):
        ax.set_xlabel("x" * (i + 1))
        ax.set_ylabel("y")
        ax.set_title("title\n" * (i % 2) + "t")
        ax.plot([0, 10 ** (i + 1)], [0, 10 ** (3 * i)])
        ax.caxis.set_label_text("c")
    fig.align_labels()
    fig.align_titles()
    fig.canvas.draw()
    _close([ax.xaxis.label.get_position() for ax in axs], RECORDED["aligned x"])
    _close([ax.yaxis.label.get_position() for ax in axs], RECORDED["aligned y"])
    _close([ax.caxis.label.get_position() for ax in axs], [(0.3, 0.801)] * 4)
    for ax in axs:
        ax.label_outer()
    fig.canvas.draw()
    assert [ax.get_xlabel() for ax in axs] == ["", "", "xxx", "xxxx"]
    assert [ax.caxis.get_label_text() for ax in axs] == ["c"] * 4

    ax = axs[2]
    extra = ax.text(1.3, 0.5, "outside", transform=ax.transAxes)
    fig.canvas.draw()
    _close(ax.get_tightbbox(bbox_extra_artists=[]).bounds, RECORDED["tight", "no artists"])
    _close(ax.get_tightbbox(bbox_extra_artists=[extra]).bounds, RECORDED["tight", "extra"])
    _close(fig.get_tightbbox().bounds, RECORDED["tight", "figure"])
    saved = io.BytesIO()
    fig.savefig(saved, bbox_inches="tight", format="rgba")
    assert len(saved.getvalue()) // 4 == RECORDED["saved pixels"]

    # `tick_params` reaches the axes pltrs (Rust) holds; the third axis
    # has its own, as in matplotlib.
    ax.tick_params(axis="both", labelsize=6, colors="r")
    ax.caxis.set_tick_params(labelsize=5, labelcolor="g")
    fig.canvas.draw()
    assert ax.xaxis.get_ticklabels()[0].get_fontsize() == 6
    assert (ax.caxis.get_ticklabels()[0].get_fontsize(),
            ax.caxis.get_ticklabels()[0].get_color()) == (5, "g")
    with pytest.raises(ValueError):
        ax.tick_params(axis="c", labelsize=5)
    ax.axis("off")
    fig.canvas.draw()
    assert not ax.axison and ax.caxis.get_visible()
    _close(ax.get_tightbbox().bounds, RECORDED["tight", "axis off"])
    ax.set_axis_on()
    ax.grid(True)
    fig.canvas.draw()
    assert ax.xaxis.get_gridlines()[0].get_visible()
    assert not ax.caxis.get_gridlines()[0].get_visible()


def test_axes_sharing_an_axis_join_the_groups_of_their_class():
    fig = plt.figure(figsize=(5, 4), dpi=100)
    a = fig.add_subplot(2, 2, 1, projection="proj1_curve")
    b = fig.add_subplot(2, 2, 2, projection="proj1_curve", sharex=a)
    c = fig.add_subplot(2, 2, 3, projection="proj1_curve")
    c.sharey(a)
    # The class keeps groups of its own for its three axes
    # (``_shared_axes``), which matplotlib's ``sharex`` joins.
    assert [a.get_shared_x_axes().joined(a, other) for other in (b, c)] == [True, False]
    assert [a.get_shared_y_axes().joined(a, other) for other in (b, c)] == [False, True]
    assert [len(ax.xaxis._get_shared_axes()) for ax in (a, b, c)] == [2, 2, 1]
    assert [len(ax.caxis._get_shared_axes()) for ax in (a, b, c)] == [1, 1, 1]
    assert (b._sharex, b._sharec) == (a, None)
    a.set_xlim(1, 4)
    a.set_ylim(-3, 3)
    a.set_clim(0.2, 0.8)
    assert (b.get_xlim(), c.get_ylim()) == ((1.0, 4.0), (-3.0, 3.0))
    # Nothing shares the third axis: matplotlib has no ``sharec``.
    assert (b.get_clim(), c.get_clim()) == ((0.0, 1.0), (0.0, 1.0))
    fig.canvas.draw()
    twin = a.twinx(axes_class=CurveAxes)
    assert (type(twin), twin._sharex) == (CurveAxes, a)
    assert a.get_shared_x_axes().joined(a, twin)
    assert (twin.get_clim(), twin.caxis.get_visible(), twin.xaxis.get_visible(),
            twin.patch.get_visible()) == ((0.0, 1.0), True, False, False)
    plain = a.twiny()
    assert type(plain) is Axes and plain.get_shared_y_axes().joined(a, plain)
    fig.canvas.draw()


def test_a_held_axis_keeps_its_figure_when_its_axes_leaves():
    fig = plt.figure(figsize=(5, 4), dpi=100)
    sub = fig.subfigures(1, 2)[0]
    a = sub.add_subplot(projection="proj1_curve")
    b = fig.add_subplot(1, 2, 2, projection="proj1_curve")
    fig.canvas.draw()
    a.remove()
    b.remove()
    # matplotlib decouples the Axes from the figure, not its axes.
    assert (a.get_figure(), b.get_figure(), len(fig.axes)) == (None, None, 0)
    assert a.caxis.get_figure(root=False) is sub and a.caxis.get_figure(root=True) is fig
    assert b.caxis.figure is fig and b.caxis.axes is b
    tick = b.caxis.get_major_ticks()[0]
    assert tick.figure is fig and b.caxis.label.figure is fig
    assert b.get_clim() == (0.0, 1.0) and len(b.caxis.get_ticklocs()) == 6
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'dpi'"):
        b.caxis.get_tightbbox()
    with pytest.raises(ValueError, match="must have been created in the present figure"):
        fig.add_axes(b)
    c = fig.add_subplot(projection="proj1_curve")
    fig.clear()
    assert (len(fig.axes), c.caxis.figure, c.caxis.axes) == (0, fig, c)


def test_a_pickled_figure_keeps_a_geographic_subclass_and_draws_the_same():
    fig = plt.figure(figsize=(5, 4), dpi=100)
    ax = fig.add_subplot(projection=SINUSOIDAL[HammerAxes].name)
    ax.plot([0, 1], [0, 1])
    ax.grid(True)
    ax.set_longitude_grid_ends(60)
    fig.canvas.draw()
    copy = pickle.loads(pickle.dumps(fig))
    copy.canvas.draw()
    again = copy.axes[0]
    assert type(again) is SINUSOIDAL[HammerAxes]
    assert isinstance(again.transProjection, Sinusoidal)
    assert again._longitude_cap == np.deg2rad(60)
    assert np.array_equal(np.asarray(fig.canvas.buffer_rgba()),
                          np.asarray(copy.canvas.buffer_rgba()))


class _PixelSkew(mtransforms.Affine2D):
    """A skew in display pixels around the bottom of a box (as MetPy's
    Skew-T applies its own after ``transData``)."""

    def __init__(self, bbox):
        super().__init__()
        self._bbox = bbox
        self.set_children(bbox)
        self.invalidate()

    def get_matrix(self):
        if self._invalid:
            self._mtx = np.array([[1.0, 0.5, -0.5 * self._bbox.ymin],
                                  [0.0, 1.0, 0.0],
                                  [0.0, 0.0, 1.0]])
            self._invalid = 0
            self._inverted = None
        return self._mtx


class _SkewedInPixels(Axes):
    """An Axes whose ``transData`` does not end with ``transAxes``."""

    name = "proj2_skewed_in_pixels"

    def _set_lim_and_transforms(self):
        super()._set_lim_and_transforms()
        self.transData += _PixelSkew(self.bbox)


register_projection(_SkewedInPixels)


def test_a_transform_applied_after_transData_follows_the_axes_when_they_move():
    fig = plt.figure(figsize=(5, 4), dpi=100)
    ax = fig.add_subplot(projection="proj2_skewed_in_pixels")
    ax.plot([0.25, 0.25], [0.0, 1.0], color="r", linewidth=3)
    ax.set(xlim=(0, 1), ylim=(0, 1), xticks=[], yticks=[])
    fig.canvas.draw()
    # The aspect makes the Axes a third as high: the line still spans it.
    ax.set_aspect(1 / 3)
    fig.canvas.draw()
    _close(ax.bbox.bounds, RECORDED["skewed", "box"])
    _close(ax.transData.transform([(0.25, 0.0), (0.25, 1.0)]), RECORDED["skewed", "line"])
    rgba = np.asarray(fig.canvas.buffer_rgba())
    red = np.nonzero((rgba[..., 0] > 200) & (rgba[..., 1] < 60))
    top, bottom = fig.bbox.height - ax.bbox.y1, fig.bbox.height - ax.bbox.y0
    assert red[0].min() == pytest.approx(top, abs=2) and red[0].max() == pytest.approx(bottom, abs=2)
    assert len(red[0]) == RECORDED["skewed", "red pixels"]


def test_matplotlibs_parser_of_format_strings_is_where_libraries_call_it():
    # A private function of matplotlib's that healpy's projection Axes call.
    from pltrs.axes import _axes, _base
    assert _axes._process_plot_format("r--") == ("--", "None", "r")
    assert _base._process_plot_format("go") == ("None", "o", "g")
    with pytest.raises(ValueError):
        _base._process_plot_format("zz")


@pytest.mark.parametrize("projection", [None, "proj1_curve"])
def test_pans_and_zooms_go_back_through_the_transform_as_matplotlib_inverts_it(projection):
    # To the last bit: matplotlib inverts ``transData`` part by part and
    # multiplies the inverses, for the Axes of a program's class too.
    fig = plt.figure(figsize=(5.3, 4.1), dpi=100)
    ax = fig.add_subplot(projection=projection)
    ax.plot([0.13, 2.71, 3.3], [1.7, 0.33, 9.1])
    fig.canvas.draw()
    limits = []
    for x0, y0, x1, y1, button in [(200, 150, 237, 181, 1), (301.5, 99.25, 150.75, 260.5, 1),
                                   (250, 200, 290, 170, 3), (120, 300, 121, 333, 1)]:
        ax.start_pan(x0, y0, button)
        ax.drag_pan(button, None, x1, y1)
        ax.end_pan()
        limits.append((*ax.get_xlim(), *ax.get_ylim()))
        fig.canvas.draw()
    for box in [(150, 120, 310, 290), (200.5, 150.25, 260.75, 220.125), (240, 200, 1.5)]:
        ax._set_view_from_bbox(box)
        limits.append((*ax.get_xlim(), *ax.get_ylim()))
        fig.canvas.draw()
    assert [tuple(map(float, each)) for each in limits] == RECORDED["pans and zooms"]


def test_redrawing_in_the_frame_applies_the_aspect():
    fig, ax = plt.subplots(figsize=(5, 4), dpi=100)
    fig.canvas.draw()
    ax.axis("equal")
    assert ax.get_ylim() == (-0.05500000000000001, 0.05500000000000001)
    ax.redraw_in_frame()
    _close(ax.get_ylim(), RECORDED["equal after redraw"], abs=1e-12)


#: What matplotlib 3.11 gives.
RECORDED = {('markers', 'first draw'): {'transform_non_affine': 68, 'transform_path_non_affine': 1},
 ('markers', 'second draw'): {'transform_non_affine': 66},
 'turned': [[256.25, 198.0], [182.4185, 198.0]],
 'halos': (95, 273),
 ('long label', 'constrained'): [0.47923, 0.06945, 0.24566, 0.90695],
 ('long label', 'tight'): [0.4215, 0.09653, 0.467, 0.85278],
 'out of the layout': [0.09344, 0.09653, 0.85456, 0.85278],
 'aligned x': [[0.5, 182.8333], [0.5, 182.8333], [0.5, 14.8333], [0.5, 14.8333]],
 'aligned y': [[16.2222, 0.5], [222.5859, 0.5], [16.2222, 0.5], [222.5859, 0.5]],
 ('tight', 'no artists'): [2.3333, 0.9444, 246.2968, 204.0516],
 ('tight', 'extra'): [2.3333, 0.9444, 341.1439, 204.0516],
 ('tight', 'figure'): [0.0233, 0.0094, 4.6216, 3.9406],
 'saved pixels': 199548,
 ('tight', 'axis off'): [62.5, 44.0, 280.9773, 160.9961],
 ('skewed', 'box'): [62.5, 133.4167, 387.5, 129.1667],
 ('skewed', 'line'): [[159.375, 133.4167], [223.9583, 262.5833]],
 ('skewed', 'red pixels'): 516,
 'pans and zooms': [(-0.3426059038344491,
                     3.1443940961655503,
                     -1.055782229965157,
                     8.591217770034842),
                    (0.9371634205721242,
                     4.424163420572124,
                     -5.983177700348431,
                     3.6638222996515664),
                    (1.2505048936585657,
                     4.037069464704793,
                     -7.14091746848248,
                     4.865659321704213),
                    (1.2437208046237604,
                     4.030285375669988,
                     -8.395960338536838,
                     3.610616451649852),
                    (1.8118882612886942,
                     2.897342506857523,
                     -5.547393339534674,
                     0.9179790213514121),
                    (2.166659368939614,
                     2.325876942391889,
                     -3.393975842711196,
                     -1.9629720615362984),
                    (2.1891094021206507,
                     2.295254451088834,
                     -3.1599324621769425,
                     -2.2059299413936775)],
 'equal after redraw': [-0.0437161290323, 0.0437161290323]}
