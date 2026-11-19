"""
Differences the impact report found in what plotting calls make and what
their artists report (task DIFFX-A): contour sets of a `LogLocator`
mapping through a `LogNorm`, a stem plot with NaN among its locations,
the path of a step line once its markers were drawn, the labels the
Axes gives its children, the baselines of ``acorr`` and ``xcorr``, and
the integer values of a hexbin binned by ``bins``; then (the follow-up)
the limits given with the norm of a contour set, set on that norm, and
the values the masks of a scatter hide. Every expected value was
recorded from matplotlib 3.11.2.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import ticker
from pltrs.colors import LogNorm, Normalize
from pltrs.contour import ContourSet
from pltrs.lines import Line2D


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X, Y = np.meshgrid(np.linspace(-3, 3, 50), np.linspace(-2, 2, 40))
Z = np.exp(-X**2 - Y**2) * 50 + 0.01

#: The colours the viridis `LogNorm` of the levels 0.01 to 100 gives 0.05,
#: 1 and 50.
LOG_COLOURS = [[0.265145, 0.232956, 0.516599, 1.0], [0.127568, 0.566949, 0.550556, 1.0],
               [0.804182, 0.882046, 0.114965, 1.0]]


def _contours(ax, method, **kwargs):
    if method.startswith("tri"):
        return getattr(ax, method)(X.ravel(), Y.ravel(), Z.ravel(), **kwargs)
    return getattr(ax, method)(X, Y, Z, **kwargs)


@pytest.mark.parametrize("method", ["contour", "contourf", "tricontour", "tricontourf"])
def test_a_log_locator_maps_the_levels_through_a_log_norm(method):
    fig, ax = plt.subplots()
    cs = _contours(ax, method, locator=ticker.LogLocator())
    assert type(cs.norm).__name__ == "LogNorm"
    assert cs.get_clim() == (0.01, 100.0)
    assert cs.norm(1.0) == 0.5
    assert list(cs.levels) == [0.01, 0.1, 1.0, 10.0, 100.0]
    np.testing.assert_allclose(cs.to_rgba(np.array([0.05, 1.0, 50.0])), LOG_COLOURS,
                               atol=1e-6)


@pytest.mark.parametrize("method", ["contour", "contourf", "tricontour", "tricontourf"])
def test_the_norm_of_a_log_locator_colours_other_artists_and_the_colorbar(method):
    fig, ax = plt.subplots()
    cs = _contours(ax, method, locator=ticker.LogLocator())
    points = ax.scatter([0, 1], [0, 1], c=[50, 1], norm=cs.norm, cmap=cs.cmap)
    fig.canvas.draw()
    np.testing.assert_allclose(points.get_facecolor(), [LOG_COLOURS[2], LOG_COLOURS[1]],
                               atol=1e-6)
    colorbar = fig.colorbar(cs)
    fig.canvas.draw()
    assert colorbar.ax.get_yscale() == "log"
    assert list(colorbar.get_ticks()) == [0.01, 0.1, 1.0, 10.0, 100.0]
    assert colorbar.ax.get_ylim() == (0.01, 100.0)


def test_the_limits_given_with_a_log_locator_are_the_log_norms():
    fig, ax = plt.subplots()
    cs = ax.contourf(X, Y, Z, locator=ticker.LogLocator(), vmin=0.1)
    assert type(cs.norm).__name__ == "LogNorm"
    assert cs.get_clim() == (0.1, 100.0)
    cs = ax.contourf(X, Y, Z, locator=ticker.LogLocator(), vmin=0.1, vmax=1000)
    assert cs.get_clim() == (0.1, 1000.0)
    assert list(cs.levels) == [0.01, 0.1, 1.0, 10.0, 100.0]
    image = ax.imshow(Z, norm=cs.norm, cmap=cs.cmap)
    np.testing.assert_allclose(
        image.to_rgba(np.array([0.1, 1.0, 10.0])),
        [[0.267004, 0.004874, 0.329415, 1.0], [0.229739, 0.322361, 0.545706, 1.0],
         [0.127568, 0.566949, 0.550556, 1.0]], atol=1e-6)


@pytest.mark.parametrize("orientation", ["vertical", "horizontal"])
def test_a_nan_location_leaves_the_stem_baseline_undrawn(orientation):
    fig, ax = plt.subplots()
    stems = ax.stem([0, np.nan, 2], [1, 2, 3], orientation=orientation)
    along, across = stems.baseline.get_xdata(), stems.baseline.get_ydata()
    if orientation == "horizontal":
        along, across = across, along
    assert np.isnan(along).all()
    assert list(across) == [0, 0]
    limits = (ax.get_xlim(), ax.get_ylim())
    if orientation == "horizontal":
        limits = limits[::-1]
    assert limits == ((-0.1, 2.1), (-0.15000000000000002, 3.15))


def _vertices(line):
    return line.get_path().vertices.tolist()


MID_STEPS = [[0.0, 0.0], [0.5, 0.0], [0.5, 1.0], [1.5, 1.0], [1.5, 4.0], [2.0, 4.0]]


def test_a_step_line_drawn_with_markers_has_its_points_as_path():
    fig, ax = plt.subplots()
    line, = ax.step([0, 1, 2], [0, 1, 4], "o-", where="mid")
    assert _vertices(line) == MID_STEPS
    fig.canvas.draw()
    drawn = line.get_path()
    assert drawn.vertices.tolist() == [[0.0, 0.0], [1.0, 1.0], [2.0, 4.0]]
    fig.canvas.draw()
    assert line.get_path() is not drawn, "each draw makes the path anew"
    line.set_ydata([1, 2, 3])
    assert _vertices(line) == [[0.0, 1.0], [0.5, 1.0], [0.5, 2.0], [1.5, 2.0], [1.5, 3.0],
                               [2.0, 3.0]]
    fig.canvas.draw()
    line.set_drawstyle("steps-post")
    assert _vertices(line) == [[0.0, 1.0], [1.0, 1.0], [1.0, 2.0], [2.0, 2.0], [2.0, 3.0]]
    fig.canvas.draw()
    line.set_marker("None")
    assert _vertices(line) == [[0.0, 1.0], [1.0, 2.0], [2.0, 3.0]], "until recached"
    line.recache()
    assert _vertices(line) == [[0.0, 1.0], [1.0, 1.0], [1.0, 2.0], [2.0, 2.0], [2.0, 3.0]]


@pytest.mark.parametrize("marker, size", [(None, 6.0), ("o", 0.0)])
def test_a_step_line_drawing_no_markers_keeps_its_steps(marker, size):
    fig, ax = plt.subplots()
    line = ax.add_line(Line2D([0, 1, 2], [0, 1, 4], drawstyle="steps-mid", marker=marker,
                              markersize=size))
    fig.canvas.draw()
    assert _vertices(line) == MID_STEPS


def _labels(ax):
    return [(type(child).__name__, child.get_label()) for child in ax._children]


def test_the_blocks_of_a_colorbar_drawn_anew_count_the_children_alone():
    fig, (ax, cax) = plt.subplots(1, 2)
    image = ax.imshow(np.arange(4.).reshape(2, 2))
    fig.colorbar(image, cax=cax)
    image.set_cmap("plasma")
    assert _labels(cax) == [("LineCollection", "_child0"), ("QuadMesh", "_child1")]
    fig.colorbar(ax.imshow(np.arange(4.).reshape(2, 2)), cax=cax)
    assert _labels(cax)[2:] == [("LineCollection", "_child2"), ("QuadMesh", "_child3")]
    image.set_clim(0, 5)
    assert _labels(cax) == [("LineCollection", "_child0"), ("LineCollection", "_child2"),
                            ("QuadMesh", "_child3"), ("QuadMesh", "_child3")]


@pytest.mark.parametrize("label", [None, ""])
def test_artists_made_with_no_label_are_named_as_children(label):
    fig, ax = plt.subplots()
    made = [ax.scatter([1], [2], label=label), ax.scatter([1], [2], c=[3], label=label),
            ax.plot([1], [2], label=label)[0], ax.vlines([1], 0, 1, label=label),
            ax.fill_between([1, 2], [0, 1], label=label), ax.imshow([[1]], label=label),
            ax.pcolormesh([[1]], label=label)]
    assert [artist.get_label() for artist in made] == [f"_child{n}" for n in range(7)]
    assert ax.text(0, 0, "x", label=label).get_label() == label


@pytest.mark.parametrize("method, args", [("acorr", ()), ("xcorr", (np.arange(40.0),))])
def test_the_baseline_of_a_correlation_holds_axhlines_lists(method, args):
    fig, ax = plt.subplots()
    *_, baseline = getattr(ax, method)(np.sin(np.arange(40) * 0.7) * 3, *args, maxlags=5)
    assert baseline.get_xdata() == [0, 1]
    assert baseline.get_ydata() == [0, 0]
    assert baseline.get_data() == ([0, 1], [0, 0])


HEX_X = np.linspace(0.1, 10, 300)
HEX_Y = np.sin(HEX_X * 3) * 5 + 6


@pytest.mark.parametrize("bins", [4, [0, 5, 10, 20]])
def test_a_hexbin_binned_by_bins_maps_integers(bins):
    fig, ax = plt.subplots()
    hexbin = ax.hexbin(HEX_X, HEX_Y, gridsize=5, bins=bins)
    values = hexbin.get_array()
    assert values.dtype == np.int64
    assert values[:5].tolist() == [0, 1, 3, 3, 2]
    clim = hexbin.get_clim()
    assert clim == (0, 3) and all(type(limit) is int for limit in clim)
    fig.canvas.draw()
    assert hexbin.get_clim() == (0, 3)


def test_the_limits_given_to_a_binned_hexbin_are_floats():
    fig, ax = plt.subplots()
    clim = ax.hexbin(HEX_X, HEX_Y, gridsize=5, bins=4, vmin=1).get_clim()
    assert clim == (1.0, 3)
    assert [type(limit) for limit in clim] == [float, int]
    counts = ax.hexbin(HEX_X, HEX_Y, gridsize=5)
    assert counts.get_array().dtype == np.float64
    assert counts.get_clim() == (0.0, 16.0)


GX, GY = np.meshgrid(np.linspace(-3, 3, 30), np.linspace(-2, 2, 20))
GZ = np.exp(-GX**2 - GY**2) * 50 + 0.01
#: The upper limit a norm takes from the data: GZ's largest value as this
#: numpy's ``exp`` computes it (48.9326816927841 under numpy 2,
#: 48.93268169278411 under numpy 1.26, where matplotlib takes the same).
GZ_MAX = float(GZ.max())

#: Per norm and limits given: the norm's limits (the lower one scaled to
#: the image drawn with the norm before, where none is given), the set's
#: levels, and the colours the image gives 1 and 20 once the set is made.
GIVEN_LIMITS = {
    "LogNorm vmin": (LogNorm, {"vmin": 0.1}, (0.1, GZ_MAX),
                     [0.01, 0.1, 1.0, 10.0, 100.0],
                     [[0.174274, 0.445044, 0.557792, 1.0], [0.626579, 0.854645, 0.223353, 1.0]]),
    "Normalize vmin vmax": (Normalize, {"vmin": 1, "vmax": 40}, (1.0, 40.0),
                            [0.0, 8.0, 16.0, 24.0, 32.0, 40.0, 48.0, 56.0],
                            [[0.267004, 0.004874, 0.329415, 1.0],
                             [0.132444, 0.552216, 0.553018, 1.0]]),
    "LogNorm vmax": (LogNorm, {"vmax": 30}, (0.010113016470349052, 30.0),
                     [0.01, 0.1, 1.0, 10.0, 100.0], None),
    "Normalize(0, 10) vmax": (lambda: Normalize(0, 10), {"vmax": 60}, (0.0, 60.0),
                              [0.0, 8.0, 16.0, 24.0, 32.0, 40.0, 48.0, 56.0],
                              [[0.272594, 0.025563, 0.353093, 1.0],
                               [0.190631, 0.407061, 0.556089, 1.0]]),
}


def _grid_contours(ax, method, **kwargs):
    if method.startswith("tri"):
        return getattr(ax, method)(GX.ravel(), GY.ravel(), GZ.ravel(), **kwargs)
    return getattr(ax, method)(GX, GY, GZ, **kwargs)


@pytest.mark.parametrize("method", ["contour", "contourf", "tricontour", "tricontourf"])
@pytest.mark.parametrize("case", sorted(GIVEN_LIMITS))
def test_the_limits_given_with_a_norm_are_set_on_that_norm(case, method):
    make, limits, clim, levels, shared = GIVEN_LIMITS[case]
    fig, (ax, other) = plt.subplots(1, 2)
    norm = make()
    image = other.imshow(GZ, norm=norm)
    fig.canvas.draw()
    cs = _grid_contours(ax, method, norm=norm, **limits)
    assert cs.norm is norm
    assert (norm.vmin, norm.vmax) == clim
    assert cs.get_clim() == clim
    assert list(cs.levels) == levels
    assert image.get_clim() == clim, "the image sharing the norm follows it"
    if shared is not None:
        np.testing.assert_allclose(image.to_rgba(np.array([1.0, 20.0])), shared, atol=1e-6)


def test_the_bands_and_colorbar_of_limits_given_with_a_norm():
    fig, ax = plt.subplots()
    cs = ax.contourf(GX, GY, GZ, norm=Normalize(), vmin=1, vmax=40)
    fig.canvas.draw()
    np.testing.assert_allclose(
        cs.get_facecolor()[:3],
        [[0.283091, 0.110553, 0.431554, 1.0], [0.214298, 0.355619, 0.551184, 1.0],
         [0.132444, 0.552216, 0.553018, 1.0]], atol=1e-6)
    colorbar = fig.colorbar(cs)
    fig.canvas.draw()
    assert colorbar.ax.get_ylim() == (0.0, 56.0)
    assert list(colorbar.get_ticks()) == [0.0, 8.0, 16.0, 24.0, 32.0, 40.0, 48.0, 56.0]
    fig, ax = plt.subplots()
    cs = ax.contourf(GX, GY, GZ, norm=LogNorm(), vmin=0.1)
    colorbar = fig.colorbar(cs)
    fig.canvas.draw()
    assert colorbar.ax.get_yscale() == "log"
    assert colorbar.ax.get_ylim() == (0.01, 100.0)


@pytest.mark.parametrize("method", ["contour", "tricontour"])
def test_labels_of_lines_given_limits_with_their_norm(method):
    fig, ax = plt.subplots()
    cs = _grid_contours(ax, method, norm=Normalize(), vmin=5, levels=[0, 10, 20, 30, 40, 50])
    assert cs.get_clim() == (5.0, 50.0)
    labels = ax.clabel(cs, fontsize=6)
    assert [text.get_text() for text in labels] == ["10", "20", "30", "40"]
    assert list(cs.labelCValues) == [10.0, 20.0, 30.0, 40.0]


def test_a_contour_set_made_directly_sets_the_limits_on_its_norm():
    fig, ax = plt.subplots()
    norm = Normalize()
    cs = ContourSet(ax, [0, 1, 2], [[[[0, 0], [1, 0], [1, 1]]], [[[1, 1], [2, 1], [2, 2]]]],
                    filled=True, norm=norm, vmin=-1, vmax=4)
    assert cs.norm is norm and cs.get_clim() == (-1.0, 4.0)
    np.testing.assert_allclose(cs.get_facecolor(), [[0.2068, 0.3718, 0.5531, 1.0],
                                                    [0.1276, 0.5669, 0.5506, 1.0]], atol=1e-4)


def test_limits_given_with_a_colorizer_are_refused():
    from pltrs.colorizer import Colorizer
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="cannot be used simultaneously"):
        ax.contourf(np.arange(12.0).reshape(3, 4), colorizer=Colorizer(), vmin=1)


def _masked(values):
    return np.ma.getdata(values).tolist(), np.ma.getmaskarray(values).tolist()


#: Per scatter: its offsets and array, data and mask, before and after a
#: draw.
MASKED_SCATTERS = {
    "masked x": (
        lambda ax: ax.scatter(np.ma.masked_array([1, 2, 3], [0, 1, 0]), [1, 2, 3], c=[1, 2, 3]),
        ([[1.0, 1.0], [2.0, 2.0], [3.0, 3.0]], [[False, False], [True, True], [False, False]]),
        ([1.0, 2.0, 3.0], [False, True, False])),
    "masked y": (
        lambda ax: ax.scatter([1.0, 2, 3], np.ma.masked_array([1.5, 2.5, 3.5], [1, 0, 0]),
                              c=[1.0, 5, 3]),
        ([[1.0, 1.5], [2.0, 2.5], [3.0, 3.5]], [[True, True], [False, False], [False, False]]),
        ([1.0, 5.0, 3.0], [True, False, False])),
    "masked c": (
        lambda ax: ax.scatter([1, 2, 3], [1, 2, 3], c=np.ma.masked_array([1, 2, 3], [0, 0, 1])),
        ([[1.0, 1.0], [2.0, 2.0], [3.0, 3.0]], [[False, False], [False, False], [True, True]]),
        ([1.0, 2.0, 3.0], [False, False, True])),
    "masked sizes, given colours": (
        lambda ax: ax.scatter([1, 2, 3], [1, 2, 3], s=np.ma.masked_array([10, 20, 30], [0, 1, 0]),
                              color="r"),
        ([[1.0, 1.0], [2.0, 2.0], [3.0, 3.0]], [[False, False], [True, True], [False, False]]),
        None),
    "NaN x": (
        lambda ax: ax.scatter([1, np.nan, 3], [1, 2, 3]),
        ([[1.0, 1.0], [np.nan, 2.0], [3.0, 3.0]], [[False, False], [True, True], [False, False]]),
        None),
}


@pytest.mark.parametrize("case", sorted(MASKED_SCATTERS))
def test_a_scatter_keeps_the_values_its_masks_hide(case):
    make, offsets, array = MASKED_SCATTERS[case]
    fig, ax = plt.subplots()
    points = make(ax)
    held = points.get_offsets()
    np.testing.assert_equal(_masked(held), offsets)
    fig.canvas.draw()
    assert points.get_offsets() is held
    np.testing.assert_equal(_masked(held), offsets)
    if array is None:
        assert points.get_array() is None
    else:
        assert _masked(points.get_array()) == array
    assert ax.get_xlim() == {"masked c": (0.95, 2.05), "masked y": (1.95, 3.05)}.get(
        case, (0.9, 3.1))


def test_a_masked_offset_assigned_counts_in_the_limits_only_unmasked():
    fig, ax = plt.subplots()
    points = ax.scatter([1, 2, 3], [1, 2, 3], c=np.ma.masked_array([1, 2, 3], [0, 0, 1]))
    fig.canvas.draw()
    offsets = points.get_offsets()
    offsets[2, 1] = 4.5
    fig.canvas.draw()
    assert _masked(points.get_offsets()) == (
        [[1.0, 1.0], [2.0, 2.0], [3.0, 4.5]], [[False, False], [False, False], [True, False]])
    assert points.get_datalim(ax.transData).bounds == (1.0, 1.0, 1.0, 1.0)
