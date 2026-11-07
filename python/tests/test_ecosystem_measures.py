"""
What cartopy, shap, astropy, networkx, yellowbrick, scanpy and mne need
of matplotlib that its own suite does not reach (``cargo xtask ecosystem
suite`` runs their suites): measuring and placing artists and titles of
the program's classes, the paths their transforms and styles get, and
picking before a figure is drawn. The expected values were recorded from
matplotlib 3.11.2 running the same code.
"""

from unittest import mock

import numpy as np

import pltrs
import pltrs.pyplot as plt
from pltrs import artist as martist, backend_bases, collections as mcoll
from pltrs import patches as mpatches, spines as mspines
from pltrs.colors import LinearSegmentedColormap
from pltrs.mpl_toolkits.axes_grid1 import make_axes_locatable
from pltrs.path import Path
from pltrs.transforms import Bbox, Transform

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _press(fig, x, y):
    """A press of the first button at display point *x*, *y*, as programs
    fake it."""
    fig.canvas.callbacks.process("button_press_event", backend_bases.MouseEvent(
        "button_press_event", fig.canvas, x, y, button=1))


class _Labels(martist.Artist):
    """Measures itself as a fixed box, counting its measures (cartopy's
    gridliner measures its labels)."""

    def __init__(self):
        super().__init__()
        self.calls = 0

    def get_tightbbox(self, renderer=None):
        self.calls += 1
        return Bbox([[10, 10], [30, 30]])

    def draw(self, renderer):
        pass


def test_an_axes_tight_box_asks_its_artists_of_the_program():
    fig, ax = plt.subplots()
    labels = _Labels()
    labels.set_clip_on(False)
    ax.add_artist(labels)
    box = ax.get_tightbbox()
    assert labels.calls == 1
    assert np.allclose(box.extents, [10, 10, 587, 427.6762], atol=1e-4)


class _Margin(martist.Artist):
    """Measures a margin left of its axes growing with the axes' width,
    which constrained layout's second pass measures again."""

    def get_tightbbox(self, renderer=None):
        box = self.axes.bbox
        return Bbox([[box.x0 - 0.2 * box.width, box.y0], [box.x1, box.y1]])

    def draw(self, renderer):
        pass


def test_constrained_layout_measures_the_program_in_each_pass():
    fig, ax = plt.subplots(layout="constrained")
    margin = _Margin()
    margin.set_clip_on(False)
    ax.add_artist(margin)
    fig.draw_without_rendering()
    assert np.allclose(ax.get_position().bounds, [0.1695, 0.0579, 0.8068, 0.9225],
                       atol=1e-4)


class _Lifted(pltrs.axes.Axes):
    """Places its titles its own way, above where matplotlib's go
    (cartopy's ``GeoAxes`` lifts them above its gridliner's labels)."""

    def _update_title_position(self, renderer):
        super()._update_title_position(renderer)
        x, _ = self.title.get_position()
        self.title.set_position((x, 1.25))


def test_titles_go_where_the_class_puts_them():
    plt.rcParams["axes.titley"] = None
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=_Lifted)
    ax.set_title("t")
    fig.draw_without_rendering()
    assert ax.title.get_position()[1] == 1.25
    assert ax._autotitlepos is True
    assert fig.add_subplot(212)._autotitlepos is True
    with pltrs.rc_context({"axes.titley": 1.0}):
        assert fig.add_subplot(211)._autotitlepos is False


def test_tight_autoscaling_is_an_attribute_of_the_axes():
    with pltrs.rc_context({"axes.autolimit_mode": "round_numbers"}):
        fig, ax = plt.subplots()
        ax.plot([0.13, 0.87], [1.3, 8.7])
        ax._tight = True
        ax.autoscale_view()
        assert ax._tight is True
        assert np.allclose(ax.get_xlim(), (0.093, 0.907))
        assert np.allclose(ax.get_ylim(), (0.93, 9.07))


def test_titles_clear_insets_placed_by_their_locator():
    fig = plt.figure()
    ax = fig.gca()
    ax.plot(np.linspace(-0.3, 0.7, 12), np.arange(12))
    ax.set_ylim(0, 12.25)
    xlim = ax.get_xlim()
    ax.inset_axes((xlim[0], 12, xlim[1] - xlim[0], 0.25), transform=ax.transData)
    ax.set_title("Decision Plot")
    fig.tight_layout()
    assert np.allclose(ax.get_position().bounds, [0.0668, 0.0804, 0.8926, 0.8439],
                       atol=1e-4)


def test_the_offset_text_keeps_its_size_through_tick_params():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [1e-9, 2e-9])
    ax.tick_params(labelsize=20)
    fig.draw_without_rendering()
    assert ax.yaxis.offsetText.get_fontsize() == 10
    assert round(ax.yaxis.offsetText.get_window_extent().height, 4) == 13.8889
    ax.yaxis.offsetText.set_fontsize(15)
    fig.draw_without_rendering()
    assert round(ax.yaxis.offsetText.get_window_extent().height, 4) == 20.8333


def _route(posA, posB, *args, **kwargs):
    """A connection style that is a plain callable, keeping the keywords
    it gets on itself and ignoring the shrinking (networkx's self loops)."""
    _route.keywords = sorted(kwargs)
    return Path([posA, (posA[0], posB[1]), posB])


def test_plain_callable_connection_styles_route_unshrunk():
    fig, ax = plt.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    arrow = mpatches.FancyArrowPatch((2, 2), (8, 8), connectionstyle=_route,
                                     arrowstyle="-", shrinkA=20, shrinkB=20)
    ax.add_patch(arrow)
    fig.draw_without_rendering()
    assert _route.keywords == ["patchA", "patchB", "shrinkA", "shrinkB"]
    assert np.allclose(arrow.get_path().vertices, [[2, 2], [2, 8], [8, 8]])


def test_rc_lists_take_numpy_numbers():
    plt.rcParams["figure.figsize"] = [np.float64(5.5), np.float32(4)]
    assert plt.rcParams["figure.figsize"] == [5.5, 4.0]


def test_lines_made_with_none_take_their_defaults():
    fig, ax = plt.subplots()
    assert ax.axhline(0.5, color=None).get_color() == "C0"
    assert ax.axvline(0.5, color=None, linewidth=None).get_linewidth() == 1.5


def test_collections_have_matplotlibs_colour_and_offset_attributes():
    fig, ax = plt.subplots()
    plain = ax.scatter([1, 2], [3, 4])
    coloured = ax.scatter([1, 2], [3, 4], edgecolors=["r", "b"])
    assert plain._edgecolors == "face"
    assert coloured._edgecolors.tolist() == [[1, 0, 0, 1], [0, 0, 1, 1]]
    assert np.asarray(plain._offsets).tolist() == [[1, 3], [2, 4]]
    assert mcoll.PathCollection([])._offsets is None


def test_picks_before_a_draw_find_the_axes_where_they_stand():
    fig = plt.figure()
    ax = fig.add_subplot()
    ax.imshow(np.zeros((100, 100)), extent=(0, 100, 0, 100), picker=True)
    got = []
    fig.canvas.mpl_connect("pick_event", lambda event: got.append(
        (round(event.mouseevent.xdata, 6), round(event.mouseevent.ydata, 6))))
    for click in [(12, 8), (46, 48)]:
        _press(fig, *ax.transData.transform(click))
    assert got == [(12, 8), (46, 48)]


def test_tick_labels_made_pickable_are_picked():
    fig, ax = plt.subplots()
    ax.set_yticks([0, 0.5, 1])
    ax.set_yticklabels(["a", "b", "c"], picker=True)
    fig.canvas.draw()
    got = []
    fig.canvas.mpl_connect("pick_event", lambda event: got.append(event.artist.get_text()))
    box = ax.get_yticklabels()[1].get_window_extent()
    _press(fig, box.intervalx.mean(), box.intervaly.mean())
    assert got == ["b"]


def test_colours_of_nans_read_back_as_given():
    fig, ax = plt.subplots()
    (line,) = ax.plot([0, 1], [0, 1], color=np.array([np.nan] * 3))
    assert isinstance(line.get_color(), np.ndarray)
    assert np.isnan(line.get_color()).all()


class _Given(mcoll.Collection):
    """A collection whose class gives its paths (cartopy's
    ``FeatureArtist`` projects its geometries)."""

    def get_paths(self):
        return [Path([(1, 2), (3, 5)])]


def test_collections_of_the_program_are_measured_through_their_paths():
    fig, ax = plt.subplots()
    ax.add_collection(_Given())
    assert ax.dataLim.bounds == (1, 2, 2, 3)
    ax.relim()
    assert ax.dataLim.bounds == (1, 2, 2, 3)
    fig, ax = plt.subplots()
    ax.add_collection(_Given(), autolim=False)
    assert ax.dataLim.bounds == (np.inf, np.inf, -np.inf, -np.inf)


def test_path_patches_draw_the_path_given_as_it_changes():
    fig, ax = plt.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    path = Path([(1, 1), (2, 1), (2, 2), (1, 1)])
    patch = mpatches.PathPatch(path)
    ax.add_patch(patch)
    assert patch.get_path() is path
    assert np.allclose(patch.get_window_extent().extents, [129.6, 89.76, 179.2, 126.72])
    path.vertices = np.array([(1, 1), (5, 1), (5, 5), (1, 1)], float)
    assert np.allclose(patch.get_window_extent().extents, [129.6, 89.76, 328, 237.6])


def test_spines_of_a_given_path_give_it_back():
    fig, ax = plt.subplots()
    path = Path([(0.2, 0.2), (0.8, 0.2), (0.5, 0.9), (0.2, 0.2)])
    spine = mspines.Spine(ax, "geo", path)
    spine.set_transform(ax.transAxes)
    assert spine.get_path() is path
    assert spine.get_transform() == ax.transAxes


def test_colorbar_blocks_map_through_the_colorbars_colormap():
    cmap = LinearSegmentedColormap("mine", {
        "red": ((0, 0, 0), (1, 1, 1)), "green": ((0, 0, 0), (1, 1, 1)),
        "blue": ((0, 0, 0), (1, 1, 1))})
    fig, ax = plt.subplots()
    image = ax.imshow(np.arange(9.).reshape(3, 3), cmap=cmap)
    colorbar = fig.colorbar(image)
    (mesh,) = [child for child in colorbar.ax.get_children()
               if isinstance(child, mcoll.QuadMesh)]
    assert mesh.cmap is cmap and mesh.norm is image.norm


def test_dividers_read_nothing_of_what_they_divide_until_they_divide():
    assert type(make_axes_locatable(mock.MagicMock())).__name__ == "AxesDivider"


class _Recording(Transform):
    """A non-affine transform keeping the paths it maps."""

    input_dims = output_dims = 2
    is_affine = False

    def __init__(self):
        super().__init__()
        self.paths = []

    def transform_non_affine(self, values):
        return np.asarray(values, float)

    def transform_path_non_affine(self, path):
        self.paths.append(path)
        return path


def test_transforms_of_the_program_map_a_collections_own_paths():
    recording = _Recording()
    fig, ax = plt.subplots()
    collection = mcoll.PolyCollection([[(0, 0), (1, 0), (1, 1)], [(2, 2), (3, 2), (3, 3)]],
                                      transform=recording + ax.transData)
    ax.add_collection(collection, autolim=False)
    own = collection.get_paths()
    fig.draw_without_rendering()
    assert recording.paths
    assert all(any(path is mine for mine in own) for path in recording.paths)
