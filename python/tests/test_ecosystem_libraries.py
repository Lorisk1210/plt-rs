"""
What xarray, networkx, scikit-learn, statsmodels, geopandas, plotnine and
astropy need of matplotlib that its own suite does not reach (``cargo
xtask ecosystem suite`` runs their suites), each answered as matplotlib
3.11 answers it. The expected values were recorded from matplotlib
3.11.2 running the same code.
"""

import os
import pathlib
import subprocess
import sys
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, colors, patches, units
from pltrs.axes import Axes
from pltrs.gridspec import GridSpecBase, SubplotParams
from pltrs.layout_engine import LayoutEngine
from pltrs.path import Path
from pltrs.transforms import Bbox, Transform

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _rounded(values, digits=4):
    """*values* (nested sequences of numbers) rounded, as lists."""
    if np.iterable(values):
        return [_rounded(v, digits) for v in values]
    return round(float(values), digits)


def test_pyplot_exposes_the_modules_it_imports():
    assert plt.matplotlib is sys.modules[pltrs.__name__]
    assert plt.np is np and plt.sys is sys
    assert plt.matplotlib.collections.QuadMesh is collections.QuadMesh
    assert plt.AxLine.__name__ == "AxLine"


def test_lines_keep_the_numbers_they_draw():
    (line,) = plt.plot([1, 2, 3], [4, 5, 6])
    assert line._x.tolist() == [1.0, 2.0, 3.0]
    assert line._y.tolist() == [4.0, 5.0, 6.0]
    assert line._xy.shape == (3, 2)


def test_arrows_keep_their_ends_and_drawing_resolution():
    fig, ax = plt.subplots()
    arrow = patches.FancyArrowPatch((0, 0), (1, 2))
    ax.add_patch(arrow)
    assert arrow._posA_posB == [(0, 0), (1, 2)]
    assert arrow._convert_xy_units((1, 2)) == (1, 2)
    assert arrow._dpi_cor == 1.0
    fig.canvas.draw()
    assert arrow._dpi_cor == pytest.approx(100 / 72)
    assert patches.FancyArrowPatch(path=Path([(0, 0), (1, 1)]))._posA_posB is None


def test_connection_styles_of_the_program_read_the_axes_transforms():
    fig, ax = plt.subplots()

    def loop(posA, posB, *args, **kwargs):
        x, y = ax.transData.inverted().transform(posA)
        return Path(ax.transData.transform([(x, y), (x + 0.5, y + 0.5), (x, y)]))

    arrow = ax.add_patch(patches.FancyArrowPatch((0.2, 0.2), (0.2, 0.2), arrowstyle="-",
                                                 connectionstyle=loop))
    fig.canvas.draw()
    assert arrow.get_path().vertices.shape[1] == 2


def test_colorbars_of_levels_scale_their_long_axis_by_function():
    fig, ax = plt.subplots()
    cb = fig.colorbar(ax.contourf(np.arange(12.0).reshape(3, 4), levels=[0, 2, 5, 11]))
    assert cb.ax.yaxis._scale.name == "function"
    assert _rounded(cb.ax.yaxis.get_transform().transform([2, 5])) == [0.3333, 0.6667]
    assert _rounded(cb._forward_boundaries(np.array([0.0, 11.0, -50.0]))) == [0.0, 1.0, -1.0]
    assert _rounded(cb._inverse_boundaries(np.array([0.5]))) == [3.5]
    assert len(fig.findobj()) > 10


def test_masked_line_widths_read_back_masked():
    lc = collections.LineCollection([[(0, 0), (1, 1)]] * 3, linestyles=["-", "--"])
    lc.set_linewidths(np.ma.masked_invalid([1.0, np.nan, 3.0]))
    widths = np.ma.asarray(lc.get_linewidths())
    assert widths.mask.tolist() == [False, True, False] * 2
    lc.set_linewidths(np.ma.masked_invalid([1.0, np.nan]))
    assert np.ma.getmaskarray(lc.get_linewidths()).tolist() == [False, True]


def test_contour_colours_and_names_stay_as_given():
    fig, ax = plt.subplots()
    z = np.arange(20.0).reshape(4, 5)
    cs = ax.contour(z, colors=["k", "r", "w", "b"])
    assert cs.cmap.colors == ["k", "r", "w", "b", "k", "r", "w", "b", "k"]
    assert cs.cmap.name == "unnamed"
    filled = ax.contourf(z, levels=[2, 5, 9], colors=["k", "r", "w", "b"], extend="both")
    assert filled.cmap.colors == ["r", "w"]
    assert filled.cmap._rgba_under == (0.0, 0.0, 0.0, 1.0)
    cmap = colors.ListedColormap(["k"])
    cmap.name = ["k"]
    assert ax.contour(z, levels=[4], cmap=cmap).levels.tolist() == [4.0]


def test_log_contours_warn_of_what_they_mask():
    fig, ax = plt.subplots()
    z = np.linspace(-1, 1, 150).reshape(10, 15)
    with pytest.warns(UserWarning, match="Log scale: values of z <= 0 have been masked"):
        ax.contour(z, norm=colors.LogNorm(0.1, 10))


def test_pyplot_plots_into_3d_axes():
    plt.figure().add_subplot(projection="3d")
    (line,) = plt.plot([1, 2], [1, 2])
    assert line.get_xdata().tolist() == [1, 2]


def test_surfaces_refuse_coordinates_that_are_not_numbers():
    ax = plt.figure().add_subplot(projection="3d")
    x, y = np.meshgrid(np.array(["d", "e"]), np.array(["a", "b", "c"]))
    with pytest.raises(TypeError, match="not supported for the input types"):
        ax.plot_surface(x, y, np.zeros((3, 2)))


class _Days:
    """A value in days, which `_DaysConverter` converts."""

    def __init__(self, days):
        self.days = days


class _DaysConverter(units.ConversionInterface):
    """Days as numbers."""

    @staticmethod
    def convert(value, unit, axis):
        return np.vectorize(lambda v: float(v.days), otypes=[float])(value)

    @staticmethod
    def axisinfo(unit, axis):
        return None

    @staticmethod
    def default_units(x, axis):
        return "days"


def test_meshes_take_the_units_of_converters_of_the_program():
    units.registry[_Days] = converter = _DaysConverter()
    try:
        fig, ax = plt.subplots()
        x = np.array([_Days(0), _Days(1), _Days(2)], dtype=object)
        ax.pcolormesh(x, [0, 1], np.zeros((1, 2)))
        assert ax.xaxis.get_converter() is converter
        assert ax.get_xlim() == (0.0, 2.0)
    finally:
        del units.registry[_Days]


def test_collections_keep_paths_given_as_none():
    assert collections.PathCollection(None).get_paths() is None


def test_lines_made_with_an_alias_take_it_over_the_property():
    fig, ax = plt.subplots()
    line = ax.axhline(1, linewidth=1, lw=5, color="g", c="r")
    assert (line.get_linewidth(), line.get_color()) == (5.0, "r")
    assert ax.axline((0, 0), (1, 1), ls=":", linestyle="--").get_linestyle() == ":"


def test_scatter_keeps_its_alphas():
    fig, ax = plt.subplots()
    alphas = ax.scatter([1, 2, 3], [1, 2, 3], alpha=np.array([0.7, 0.7, 0.2])).get_alpha()
    assert alphas.tolist() == [0.7, 0.7, 0.2]


def test_fill_between_takes_a_colour_per_polygon():
    fig, ax = plt.subplots()
    x = np.arange(5.0)
    fill = ax.fill_between(x, x, x + 1, facecolor=["#99999966"] * 5, edgecolor=["none"] * 5)
    assert _rounded(fill.get_facecolor()) == [[0.6, 0.6, 0.6, 0.4]] * 5
    plain = ax.fill_between(x, x, x + 1)
    assert _rounded(plain.get_facecolor()) == [[0.1216, 0.4667, 0.7059, 1.0]]


def test_texts_keep_their_string_and_layout():
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, "ab\nlong line", rotation=30, ha="center")
    assert text._text == "ab\nlong line"
    bbox, lines, frame = text._get_layout(fig.canvas.get_renderer())
    assert _rounded(bbox.extents, 3) == [-34.972, -4.727, 34.972, 55.421]
    assert [(line, _rounded(wad, 3), _rounded(xy, 3)) for line, wad, xy in lines] == [
        ("ab", [18.0, 12.39, 4.727], [-10.158, 24.941]),
        ("long line", [61.0, 12.39, 4.727], [-20.219, -0.633])]
    assert _rounded(frame, 3) == [[-17.855, -4.727], [61.0, 34.234]]
    text._text = "new"
    assert text.get_text() == "new"


def test_axes_patch_may_be_a_patch_of_the_program():
    fig, ax = plt.subplots()
    frame = patches.PathPatch(Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]),
                              transform=ax.transData, facecolor="0.9", edgecolor="white")
    ax.patch = frame
    assert ax.patch is frame
    assert ax._axisbelow == "line"
    image = ax.imshow(np.arange(4.0).reshape(2, 2), extent=(0, 1, 0, 1))
    assert image.get_clip_path() is not None
    fig.canvas.draw()
    assert _rounded(ax.patch.get_facecolor()) == [0.9, 0.9, 0.9, 1.0]


class _CountingAxes(Axes):
    """Axes counting the calls of its ``draw`` and ``get_tightbbox``, and
    with a ``get_transform`` of another signature (as astropy's WCSAxes)."""

    calls = None

    def draw(self, renderer):
        self.calls.append("draw")
        super().draw(renderer)

    def get_tightbbox(self, renderer=None, *args, **kwargs):
        self.calls.append("tightbbox")
        return super().get_tightbbox(renderer, *args, **kwargs)

    def get_transform(self, frame):
        raise AssertionError("not the artist's transform")


@pytest.mark.parametrize("make", ["add_subplot", "add_axes"])
def test_axes_classes_of_the_program_draw_and_measure_as_matplotlib(make):
    _CountingAxes.calls = calls = []
    fig = plt.figure()
    if make == "add_subplot":
        fig.add_subplot(axes_class=_CountingAxes)
    else:
        fig.add_axes(_CountingAxes(fig, [0.1, 0.1, 0.8, 0.8]))
    fig.canvas.draw()
    assert calls == ["draw"]
    if make == "add_subplot":
        # Measured by a layout engine only (Axes placed by hand are not).
        calls.clear()
        fig.set_layout_engine("tight")
        fig.canvas.draw()
        assert "tightbbox" in calls and calls[-1] == "draw"


def test_transforms_of_the_program_get_arrays_of_points():
    seen = []

    class Shift(Transform):
        input_dims = output_dims = 2
        is_separable = has_inverse = True

        # astropy's world transforms read the columns of their points.
        def transform(self, values):
            seen.append(np.ndim(values))
            return values.T.T + 1

        transform_non_affine = transform

        def inverted(self):
            return Unshift()

    class Unshift(Shift):
        def transform(self, values):
            return values.T.T - 1

        transform_non_affine = transform

    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1], transform=Shift() + ax.transData)
    fig.canvas.draw()
    assert seen and set(seen) == {2}


class _Grid(GridSpecBase):
    """A grid of the program's: its own subplot parameters."""

    def __init__(self, figure):
        self.figure = figure
        super().__init__(1, 2)

    def get_subplot_params(self, figure=None):
        return SubplotParams(left=0.1, bottom=0.2, right=0.9, top=0.8,
                             wspace=0.5, hspace=0)


class _Engine(LayoutEngine):
    """A layout engine of the program's (no base __init__, as plotnine's)."""

    _adjust_compatible = True
    _colorbar_gridspec = False

    def __init__(self):
        self.calls = 0

    def execute(self, fig):
        self.calls += 1
        for ax in fig.axes:
            ax._set_position(Bbox.from_bounds(0.3, 0.3, 0.4, 0.4))


def test_grids_and_layout_engines_of_the_program():
    fig = plt.figure()
    grid = _Grid(fig)
    assert _rounded(grid[0, 1].get_position(fig).bounds) == [0.58, 0.2, 0.32, 0.6]
    assert _rounded(grid.get_grid_positions(fig)) == [[0.2], [0.8], [0.1, 0.58], [0.42, 0.9]]
    ax = fig.add_subplot(grid[0, 0])
    assert _rounded(ax.get_position().bounds) == [0.1, 0.2, 0.32, 0.6]
    engine = _Engine()
    fig.set_layout_engine(engine)
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        fig.canvas.draw()
    assert engine.calls == 1
    assert _rounded(ax.get_position().bounds) == [0.3, 0.3, 0.4, 0.4]


def test_toolkits_pltrs_imports_are_attributes_of_the_alias():
    """As matplotlib's projections import ``mpl_toolkits.mplot3d``, which
    then is an attribute of ``mpl_toolkits`` (xarray reaches it so), with
    pltrs installed as matplotlib (``python/alias``). The pltrs imported
    is the one under test, the tree's or an installed wheel's."""
    alias = pathlib.Path(__file__).resolve().parents[1] / "alias"
    package_dir = pathlib.Path(pltrs.__file__).resolve().parents[1]
    code = ("import matplotlib, mpl_toolkits; before = hasattr(mpl_toolkits, 'mplot3d'); "
            "import matplotlib.pyplot; "
            "print(before, mpl_toolkits.mplot3d.axes3d.Axes3D.__name__)")
    env = {**os.environ, "PYTHONPATH": os.pathsep.join([str(alias), str(package_dir)])}
    out = subprocess.run([sys.executable, "-c", code], env=env, check=True,
                         capture_output=True, text=True).stdout
    assert out.split() == ["False", "Axes3D"]
