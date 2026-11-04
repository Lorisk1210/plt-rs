"""
What pandas' plotting tests and seaborn's suite (``cargo xtask ecosystem
suite``) need of matplotlib beyond its own suite, each answered as
matplotlib 3.11 answers it. The expected values were recorded from
matplotlib 3.11.2 running the same code.
"""

import datetime
import struct
import sys
import warnings
import zoneinfo

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import collections, colors, dates, lines, markers, scale, ticker
from pltrs.path import Path

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def test_box_parts_given_none_take_the_line_defaults():
    fig, ax = plt.subplots()
    parts = ax.boxplot([[1, 2, 3, 10]], flierprops={
        "markersize": None, "markeredgewidth": None, "linestyle": None})
    flier = parts["fliers"][0]
    assert (flier.get_markersize(), flier.get_markeredgewidth(),
            flier.get_linestyle()) == (6.0, 1.0, "-")


def test_tick_helpers_ask_stand_in_axes_only_what_they_have():
    class ViewOnly:
        def get_view_interval(self):
            return (0, 10)

    formatter = ticker.ScalarFormatter()
    formatter.set_useOffset(False)
    formatter.set_scientific(False)
    formatter.axis = ViewOnly()
    assert formatter.format_ticks([0, 5, 10]) == ["0", "5", "10"]

    class PseudoAxis:
        def get_view_interval(self):
            return (0, 10)

        def get_data_interval(self):
            return (None, None)

        def get_tick_space(self):
            return 5

        def get_minpos(self):
            return 0

        def get_scale(self):
            return scale.LinearScale(None)

        def get_majorticklocs(self):
            raise AssertionError("only minor locators ask for the major ticks")

    locator = ticker.MaxNLocator(5)
    locator.set_axis(PseudoAxis())
    assert locator().tolist() == [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]


def test_legend_handles_read_back_what_their_handles_were_given():
    fig, ax = plt.subplots()
    handle = lines.Line2D([], [], color=(0.1, 0.2, 0.3), markeredgewidth=2)
    drawn = ax.legend([handle], ["a"]).get_lines()[0]
    assert drawn.get_color() == (0.1, 0.2, 0.3)
    assert drawn.get_markeredgewidth() == 2 and type(drawn.get_markeredgewidth()) is int


def test_a_legend_of_a_removed_legends_handles():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label="a")
    ax.scatter([1], [2], label="b")
    old = ax.legend()
    handles = old.legend_handles
    old.remove()
    new = ax.legend(handles, ["A", "B"])
    assert [t.get_text() for t in new.get_texts()] == ["A", "B"]
    assert [type(h).__name__ for h in new.legend_handles] == ["Line2D", "PathCollection"]
    assert new.legend_handles[0].get_color() == (
        0.12156862745098039, 0.4666666666666667, 0.7058823529411765)


def test_a_cleared_axes_forgets_its_legend_in_the_figure():
    fig, ax = plt.subplots()
    ax.plot([1], label="x")
    legend = ax.legend(title="T")
    ax.clear()
    assert legend.figure is fig and legend.get_title().get_text() == "T"
    assert ax.get_legend() is None


def test_alpha_of_one_element_is_kept_as_given():
    rgba = colors.to_rgba("r", np.array([0.5]))
    assert rgba[:3] == (1.0, 0.0, 0.0) and isinstance(rgba[3], np.ndarray)
    with pytest.raises(ValueError, match="truth value of an array"):
        colors.to_rgba("r", np.array([0.5, 0.2]))


def test_a_program_scale_keeps_the_tick_helpers_it_does_not_set():
    class CategoryScale(scale.LinearScale):
        def set_default_locators_and_formatters(self, axis):
            pass

    fig, ax = plt.subplots()
    ax.xaxis.update_units(np.array(["a", "b", "c"], dtype=object))
    ax.set_xscale(CategoryScale(None))
    assert type(ax.xaxis.get_major_locator()).__name__ == "StrCategoryLocator"
    assert ax.xaxis.isDefault_majloc


def test_category_names_need_the_units_update_units_gives():
    fig, ax = plt.subplots()
    with pytest.raises(Exception) as caught:
        ax.xaxis.convert_units(["a"])
    assert type(caught.value).__name__ == "ConversionError"
    assert str(caught.value.__cause__).startswith("Missing category information")


def test_text_on_a_date_axis_converts_as_dates():
    fig, ax = plt.subplots()
    ax.plot(np.array(["2020-01-01"], dtype="datetime64[D]"), [1])
    assert ax.xaxis.convert_units(["2003-02-28"]).tolist() == [12111.0]
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        assert type(ax.xaxis.converter).__name__ == "_SwitchableDateConverter"
    assert caught[0].category.__name__ == "MatplotlibDeprecationWarning"


def test_plotted_lines_keep_their_original_data():
    fig, ax = plt.subplots()
    (ints,) = ax.plot([1, 2, 3])
    (masked,) = ax.plot(np.arange(3), np.ma.masked_invalid([1, np.nan, 3]))
    assert ints.get_ydata().dtype == np.int64 and ints.get_ydata().tolist() == [1, 2, 3]
    assert masked.get_ydata().mask.tolist() == [False, True, False]
    assert np.array_equal(masked.get_ydata(orig=False), [1.0, np.nan, 3.0], equal_nan=True)


def test_axes_name_the_axes_they_share_with():
    fig, (left, right) = plt.subplots(1, 2, sharey=True)
    assert right._sharey is left
    assert left._sharey is None and right._sharex is None


def test_axes_sharing_an_axis_share_its_units_not_its_converter():
    fig, (top, bottom) = plt.subplots(2, sharex=True)
    top.plot(np.array(["2020-01-01", "2020-01-03"], dtype="datetime64[D]"), [1, 2])
    twin = top.twinx()
    assert bottom.xaxis.get_converter() is None and not bottom.xaxis.have_units()
    assert twin.xaxis.get_converter() is None
    assert type(top.xaxis.get_converter()).__name__ == "_SwitchableDateConverter"
    fig, (top, bottom) = plt.subplots(2, sharex=True)
    top.plot(["x", "y"], [1, 2])
    assert bottom.xaxis.get_converter() is None and bottom.xaxis.have_units()
    assert bottom.xaxis.units is top.xaxis.units


def test_meshes_convert_to_paths_and_triangles():
    coordinates = np.stack(np.meshgrid(np.arange(3.0), np.arange(2.0)), axis=-1)
    paths = collections.QuadMesh._convert_mesh_to_paths(coordinates)
    assert len(paths) == 2
    assert paths[1].vertices.tolist() == [[1, 0], [2, 0], [2, 1], [1, 1], [1, 0]]
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(np.arange(3.0), np.arange(2.0),
                         np.array([[0.0, 1, 2], [3, 4, 5]]), shading="gouraud")
    mesh.update_scalarmappable()
    triangles, colours = mesh._convert_mesh_to_triangles(coordinates)
    assert triangles.shape == (8, 3, 2) and colours.shape == (8, 3, 4)
    assert triangles[0].tolist() == [[0.0, 0.0], [1.0, 0.0], [0.5, 0.5]]
    assert np.round(colours[0], 6).tolist() == [
        [0.267004, 0.004874, 0.329415, 1.0], [0.253935, 0.265254, 0.529983, 1.0],
        [0.283284, 0.437552, 0.42381, 1.0]]


def test_a_mesh_keeps_the_values_it_was_given():
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(np.ma.masked_less_equal(np.array([[0.0, 1, 2], [3, 0, 5]]), 0))
    values = mesh.get_array()
    assert values.data.tolist() == [[0.0, 1.0, 2.0], [3.0, 0.0, 5.0]]
    assert values.mask.tolist() == [[True, False, False], [False, True, False]]
    assert ax.pcolormesh(np.arange(6).reshape(2, 3)).get_array().dtype == np.int64


def test_log_transforms_compute_as_numpy_does():
    x = np.abs(np.random.default_rng(0).normal(size=1000)) + 1e-3
    transform = scale.LogTransform(10)
    assert np.array_equal(transform.transform_non_affine(x), np.log10(x))
    assert transform.transform_non_affine(np.array([0.0, 100.0])).tolist() == [-1000.0, 2.0]
    inverse = scale.InvertedLogTransform(10).transform_non_affine(np.array([2.0]))
    assert inverse.tolist() == [100.00000000000004]


def test_built_in_markers_share_their_paths():
    style = markers.MarkerStyle
    assert style("o").get_path() is style("o").get_path()
    assert style((5, 1, 0)).get_path() is style((5, 1, 0)).get_path()
    vertices = [(0, 0), (1, 1), (0, 1)]
    assert style(vertices).get_path() is not style(vertices).get_path()
    path = Path([(0, 0), (1, 0), (1, 1)])
    assert style(path).get_path() is path


def test_colour_names_follow_the_name_mapping():
    try:
        colors._colors_full_map["b"] = (0.25, 0.5, 0.75)
        assert colors.to_rgba("b") == (0.25, 0.5, 0.75, 1)
        fig, ax = plt.subplots()
        ax.plot([1, 2], color="b")
        fig.canvas.draw()
        del colors._colors_full_map["b"]
        with pytest.raises(ValueError, match="Invalid RGBA argument: 'b'"):
            colors.to_rgba("b")
    finally:
        colors._colors_full_map["b"] = colors.BASE_COLORS["b"]
    assert colors.to_rgba("b") == (0, 0, 1, 1)


def test_private_alpha_and_face_colours_read_as_in_matplotlib():
    fig, ax = plt.subplots()
    points = ax.scatter([1], [2], color=[[0.3, 0.8, 0.5, 0.5]])
    assert points._alpha is None and points._facecolors[0, 3] == 0.5
    assert ax.plot([1])[0]._alpha is None


def _slim_pacific():
    """A slim ``TZif`` file, its data in the version 2 part only (as the
    ``tzdata`` package's are): ``America/Los_Angeles``' transitions of
    2020, then the rules of its ``TZ`` string."""
    def header(types, chars, times):
        out = b"TZif2" + bytes(15)
        for count in (0, 0, 0, times, types, chars):
            out += struct.pack(">l", count)
        return out

    data = header(1, 1, 0) + bytes(7) + header(2, 8, 2)
    for instant in (1_583_661_600, 1_604_221_200):
        data += struct.pack(">q", instant)
    data += (bytes([1, 0]) + struct.pack(">lBB", -8 * 3600, 0, 0)
             + struct.pack(">lBB", -7 * 3600, 1, 4))
    return data + b"PST\0PDT\0\nPST8PDT,M3.2.0,M11.1.0\n"


def test_zones_read_as_zoneinfo_reads_them(tmp_path, monkeypatch):
    package = tmp_path / "tzdata" / "zoneinfo" / "Test"
    package.mkdir(parents=True)
    for directory in (tmp_path / "tzdata", tmp_path / "tzdata" / "zoneinfo", package):
        (directory / "__init__.py").write_text("")
    (package / "Slim").write_bytes(_slim_pacific())
    monkeypatch.syspath_prepend(str(tmp_path))
    for name in [m for m in sys.modules if m == "tzdata" or m.startswith("tzdata.")]:
        monkeypatch.delitem(sys.modules, name)
    tz = zoneinfo.ZoneInfo.no_cache("Test/Slim")
    spring = dates.HourLocator(byhour=[1, 2, 3], tz=tz).tick_values(
        datetime.datetime(2030, 3, 9, tzinfo=tz), datetime.datetime(2030, 3, 11, tzinfo=tz))
    assert spring.tolist() == [21982.375, 21982.416666666668, 21982.458333333332,
                               21983.375, 21983.416666666668, 21983.416666666668]
    autumn = dates.HourLocator(byhour=[0, 1, 2], tz=tz).tick_values(
        datetime.datetime(2030, 11, 2, tzinfo=tz), datetime.datetime(2030, 11, 4, tzinfo=tz))
    assert autumn.tolist() == [22220.291666666668, 22220.333333333332, 22220.375,
                               22221.291666666668, 22221.333333333332, 22221.416666666668,
                               22222.333333333332]
