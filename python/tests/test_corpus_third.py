"""
What the third round of corpora (``cargo xtask diff --corpus``: the books'
and courses' notebooks, the galleries of scikit-learn, scikit-image,
statsmodels and NetworkX, astroML's figures) found pltrs doing otherwise
than matplotlib 3.11. Every expected value was recorded from matplotlib.
"""

import io
import os
import pathlib
import subprocess
import sys
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.font_manager import FontProperties
from pltrs.offsetbox import AnnotationBbox, TextArea
from pltrs.patches import Ellipse
from pltrs.transforms import Affine2D

# pltrs' own dependencies leave pandas out (a wheel's check installs none).
pd = pytest.importorskip("pandas")

PYTHON_DIR = pathlib.Path(__file__).resolve().parents[1]


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _warnings_of(work):
    """The messages of the warnings *work* issues."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        work()
    return [str(w.message) for w in caught]


def test_a_matplotlibrc_in_the_working_directory_applies(tmp_path):
    (tmp_path / "matplotlibrc").write_text("font.size: 18.0\nlines.linewidth: 3.5\n")
    env = {**os.environ, "PYTHONPATH": str(PYTHON_DIR), "PLTRS_LOOK": "matplotlib",
           "MPLCONFIGDIR": str(tmp_path / "config")}
    env.pop("PLTRSRC", None)
    env.pop("PLTRSCONFIGDIR", None)
    done = subprocess.run(
        [sys.executable, "-c", "import pltrs; print(pltrs.rcParams['font.size'], "
         "pltrs.rcParams['lines.linewidth'], pltrs.matplotlib_fname())"],
        cwd=tmp_path, env=env, capture_output=True, text=True, check=True)
    assert done.stdout.split() == ["18.0", "3.5", "matplotlibrc"]


def test_figure_legends_and_figure_pixels_follow_the_saved_window():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    ax.plot([0, 1], [0, 1])
    fig.legend([plt.Line2D([], [], marker="s", color="red", ls="", markersize=12)], ["x"],
               loc="center right")
    ax.annotate("", (60, 20), xycoords="figure pixels", xytext=(60, 60),
                textcoords="figure pixels", arrowprops=dict(arrowstyle="-", color="blue", lw=4))
    buf = io.BytesIO()
    fig.savefig(buf, format="png", bbox_inches="tight")
    buf.seek(0)
    img = plt.imread(buf)
    assert img.shape[:2] == (137, 196)
    red = np.nonzero((img[..., 0] > 0.9) & (img[..., 1] < 0.1) & (img[..., 2] < 0.1))
    blue = np.nonzero((img[..., 2] > 0.9) & (img[..., 0] < 0.1) & (img[..., 1] < 0.1))
    assert (red[1].min(), red[1].max(), red[0].min(), red[0].max()) == (171, 177, 65, 71)
    assert (blue[1].min(), blue[1].max(), blue[0].min(), blue[0].max()) == (59, 60, 78, 115)


def test_tight_boxes_take_the_spines_in():
    plt.figure(figsize=(14, 9))
    ax = plt.subplot(3, 4, 9)
    ax.set_ylim(float.fromhex("-0x1.fcd150fec1ce2p-1"), float.fromhex("0x1.7693271a28682p+1"))
    ax.set_xticks(())
    ax.set_yticks(())
    assert [float(v).hex() for v in ax.get_tightbbox().extents] == [
        "0x1.5e00000000000p+7", "0x1.8bffffffffffep+6",
        "0x1.9ade9bd37a6f6p+8", "0x1.2ed2d2d2d2d2ep+8"]


def test_pyplot_checks_the_arguments_before_making_a_figure():
    with pytest.raises(TypeError) as raised:
        plt.boxplot([1, 2, 3], labels=["a"])
    assert str(raised.value) == (
        "boxplot() got an unexpected keyword argument 'labels'. Did you mean 'label'?")
    with pytest.raises(TypeError, match=r"^hist\(\) missing 1 required positional argument: 'x'$"):
        plt.hist()
    assert plt.get_fignums() == []


def test_transforms_scale_and_translate_by_numpy_arithmetic():
    with pytest.raises(ValueError, match="setting an array element with a sequence"):
        Affine2D().scale(np.matrix([[2.0]]))
    with pytest.raises(ValueError, match="setting an array element with a sequence"):
        Affine2D().translate(np.array([2.0]), 1)
    assert Affine2D().scale(np.array(2.0)).get_matrix().tolist() == [
        [2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]]
    assert Affine2D().scale(np.float32(1.5), 2).get_matrix()[0, 0] == 1.5


def test_an_ellipse_keeps_an_array_size_until_it_is_placed():
    fig, ax = plt.subplots()
    ellipse = Ellipse((0, 0), np.matrix([[0.5]]), 1, angle=3.0)
    assert type(ellipse.width) is np.matrix
    with pytest.raises(ValueError, match="setting an array element with a sequence"):
        ax.add_patch(ellipse)


def test_a_colour_after_an_edge_colour_colours_the_edges():
    fig, ax = plt.subplots()
    x = np.linspace(0, 1, 5)
    after = ax.fill_between(x, x, x + 1, edgecolor="none", color="lightgray", alpha=0.5)
    np.testing.assert_allclose(after.get_edgecolor(), [[0.82745098, 0.82745098, 0.82745098, 0.5]])
    before = ax.fill_between(x, x, x + 1, color="lightgray", edgecolor="none")
    assert before.get_edgecolor().shape == (0, 4)


def test_negative_marker_areas_warn_at_each_scaling():
    fig, ax = plt.subplots()
    made = _warnings_of(lambda: ax.scatter([0, 1], [0, 1], s=[-10, 20]))
    drawn = _warnings_of(fig.canvas.draw)
    saved = _warnings_of(lambda: fig.savefig(io.BytesIO(), format="png", bbox_inches="tight"))
    assert (len(made), len(drawn), len(saved)) == (1, 1, 2)
    assert set(made + drawn + saved) == {"invalid value encountered in sqrt"}


def test_a_wireframe_given_one_stride_samples_the_other_direction_fully():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    x, y = np.meshgrid(np.arange(-10, 10, 0.5), np.arange(-10, 10, 0.5))
    wireframe = ax.plot_wireframe(x, y, x * y, rstride=10)
    surface = ax.plot_surface(x, y, x * y, rstride=10)
    fig.canvas.draw()
    assert (len(wireframe.get_segments()), len(surface.get_paths())) == (45, 16)


def test_complex_histograms_warn_as_numpy_bins_them():
    def plot():
        n, bins, _ = plt.hist(np.array([1 + 1e-9j, 0.5 + 0j, 1.5 - 1e-9j]), bins=4)
        assert bins.tolist() == [0.5, 0.75, 1.0, 1.25, 1.5]
        assert n.tolist() == [1.0, 0.0, 1.0, 1.0]
    messages = _warnings_of(plot)
    assert messages == ["Casting complex values to real discards the imaginary part"] * 3


def test_an_axis_keeps_the_converter_it_took():
    # pandas plots through matplotlib by name: pltrs as matplotlib (its
    # alias), as the programs that plot with pandas have it.
    program = """
import warnings
import numpy as np, pandas as pd
import matplotlib.pyplot as plt
idx = pd.date_range("1959-01-01", "1962-07-01", freq="QS")
series = pd.Series(np.arange(len(idx), dtype=float), index=idx)
series.plot(ax=plt.subplots()[1])
fig, ax = plt.subplots()
ax.plot(idx, np.arange(len(idx)))
print(type(ax.xaxis.get_converter()).__name__)
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    series.plot(ax=ax, color="k")
print([str(w.message) for w in caught])
print(type(ax.xaxis.get_converter()).__name__, ax.xaxis.get_units())
"""
    env = {**os.environ, "PLTRS_LOOK": "matplotlib", "MPLBACKEND": "agg",
           "PYTHONPATH": os.pathsep.join([str(PYTHON_DIR), str(PYTHON_DIR / "alias")])}
    done = subprocess.run([sys.executable, "-c", program], env=env, capture_output=True,
                          text=True, check=True)
    assert done.stdout.splitlines() == [
        "_SwitchableDateConverter",
        "['This axis already has a converter set and is updating to a potentially "
        "incompatible converter']",
        "DatetimeConverter None",
    ]


def test_annotation_boxes_convert_dates():
    fig, ax = plt.subplots()
    ax.scatter([pd.Timestamp("2020-01-01"), pd.Timestamp("2020-03-01")], [1, 2])
    box = AnnotationBbox(TextArea("hello"), (pd.Timestamp("2020-02-01"), 1.5), frameon=False)
    ax.add_artist(box)
    fig.canvas.draw()
    assert box.xy == (pd.Timestamp("2020-02-01"), 1.5)


def test_measuring_raises_what_keeps_a_text_from_being_laid_out():
    fig, ax = plt.subplots()
    missing = FontProperties(fname="/nonexistent/FiraSans.ttf")
    text = ax.text(0, 0, "x", fontproperties=missing)
    with pytest.raises(FileNotFoundError,
                       match=r"^\[Errno 2\] No such file or directory: '/nonexistent/FiraSans.ttf'$"):
        text.get_window_extent()
    text.remove()
    ax.add_artist(AnnotationBbox(TextArea("y", textprops={"fontproperties": missing}), (0.5, 0.5)))
    with pytest.raises(FileNotFoundError):
        fig.canvas.draw()
    fig, ax = plt.subplots()
    title = ax.set_title(r"$\stackrel{2}{3}$")
    with pytest.raises(ValueError, match=r"Unknown symbol: \\stackrel"):
        title.get_window_extent()


@pytest.mark.parametrize("kept, frame", [(False, [255, 127, 127, 255]), (True, [255, 63, 63, 255])])
def test_an_axes_legend_kept_as_an_artist_stays_and_draws_twice(kept, frame):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([0, 1], label="a")
    legend = ax.legend(loc="center", facecolor="red", framealpha=0.5, edgecolor="none")
    if kept:
        ax.add_artist(legend)
    assert ax.get_legend() is legend
    fig.canvas.draw()
    bbox = legend.get_frame().get_window_extent()
    image = np.asarray(fig.canvas.buffer_rgba())
    assert image[image.shape[0] - int(bbox.y0) - 4, int(bbox.x0) + 3].tolist() == frame


def test_collections_resolve_default_colours_again_when_first_drawn():
    with plt.style.context("classic"):
        fig, ax = plt.subplots()
        dots = ax.scatter([0, 1], [0, 1], s=100)
    assert dots.get_edgecolor().tolist() == [[0.0, 0.0, 0.0, 1.0]]
    fig.canvas.draw()
    assert dots.get_edgecolor().tolist() == []
    assert dots.get_facecolor().tolist() == [[0.0, 0.0, 1.0, 1.0]]


def test_a_first_tight_save_places_axis_artist_labels():
    from pltrs.mpl_toolkits import axisartist
    from PIL import Image
    fig = plt.figure()
    ax = axisartist.Axes(fig, [0.1, 0.1, 0.8, 0.8])
    fig.add_axes(ax)
    ax.set_xlabel("X Axis")
    ax.set_ylabel("Y Axis")
    buf = io.BytesIO()
    fig.savefig(buf, format="png", bbox_inches="tight")
    assert Image.open(buf).size == (596, 451)


def test_tight_layouts_and_boxes_raise_what_keeps_a_text_from_being_measured():
    fig, ax = plt.subplots()
    ax.set_title(r"$\stackrel{2}{3}$")
    before = ax.get_position().bounds
    for measure in (fig.tight_layout, ax.get_tightbbox, fig.get_tightbbox):
        with pytest.raises(ValueError, match=r"Unknown symbol: \\stackrel, found '\\'"):
            measure()
    assert ax.get_position().bounds == before


def test_a_tight_window_is_measured_in_inches_as_matplotlib_maps_it():
    # The window's corner lands the top spine on a half pixel, whose last
    # bit decides the row it is snapped to.
    pltrs.rcParams["figure.dpi"] = 75
    pltrs.rcParams["figure.figsize"] = [6, 3.5]
    labels = ["None", "GED", "High school diploma", "Associate's degree",
              "Bachelor's degree", "Master's degree", "PhD", "Professional degree"]
    plt.bar(range(8), [0.098, 0.13, 0.393, 0.085, 0.19, 0.078, 0.007, 0.013])
    plt.xticks(range(8), labels, rotation=30, ha="right")
    plt.gca().set(ylabel="PMF")
    plt.tight_layout()
    buf = io.BytesIO()
    plt.gcf().savefig(buf, format="png", bbox_inches="tight")
    buf.seek(0)
    image = plt.imread(buf)
    assert image.shape[:2] == (255, 436)
    assert [row for row in range(4, 12) if image[row, 100, 0] < 0.4] == [8]


def test_an_empty_annotation_leaves_its_arrow_out_of_tight_boxes():
    fig, ax = plt.subplots(figsize=(4, 4))
    arrow = dict(arrowstyle="-", color="pink", linewidth=2)
    across = ax.annotate("", xy=(0, -1.3), xycoords="axes fraction",
                         xytext=(0, 1.3), arrowprops=arrow)
    ax.annotate("", xy=(-1.3, 0), xycoords="axes fraction", xytext=(1.3, 0),
                arrowprops=arrow)
    renderer = fig.canvas.get_renderer()
    assert np.allclose(across.get_window_extent(renderer).extents,
                       [50, -354.4, 50, 444.4], atol=0.01)
    assert np.allclose(ax.get_tightbbox(renderer).extents,
                       [18.28, 20.39, 371.0, 357.28], atol=0.01)
    buf = io.BytesIO()
    fig.savefig(buf, format="png", bbox_inches="tight")
    buf.seek(0)
    assert plt.imread(buf).shape[:2] == (356, 372)


def test_a_parasite_keeps_the_position_its_host_had_and_is_measured_there():
    from pltrs.mpl_toolkits.axisartist import floating_axes
    from pltrs.projections import PolarAxes

    fig = plt.figure(figsize=(8, 4))
    polar = PolarAxes.PolarTransform()
    helper = floating_axes.GridHelperCurveLinear(
        polar, extremes=(2 * np.pi, 0, 5, 0))
    host = fig.add_subplot(
        floating_axes.FloatingSubplot(fig, 111, grid_helper=helper))
    par = host.get_aux_axes(polar)
    made_at = (0.125, 0.11, 0.775, 0.77)
    assert np.allclose(host.get_position().bounds, (0.32, 0.11, 0.385, 0.77))
    assert np.allclose(par.get_position(original=True).bounds, made_at)
    assert np.allclose(par.get_position().bounds, made_at)
    renderer = fig.canvas.get_renderer()
    assert np.allclose(par.get_tightbbox(renderer).extents, [100, 44, 720, 352])
    assert np.allclose(host.get_tightbbox(renderer).extents,
                       [100, 22.63, 720, 373.3], atol=0.01)
    buf = io.BytesIO()
    fig.savefig(buf, format="png", bbox_inches="tight")
    buf.seek(0)
    assert plt.imread(buf).shape[:2] == (370, 640)
    assert np.allclose(host.get_position().bounds, (0.32, 0.11, 0.385, 0.77))
    assert np.allclose(par.get_position().bounds, made_at)
    par.set_position([0.1, 0.2, 0.3, 0.4])
    assert np.allclose(par.get_tightbbox(renderer).extents, [80, 80, 320, 240])
    assert np.allclose(host.get_tightbbox(renderer).extents,
                       [80, 22.63, 585.04, 373.3], atol=0.01)


def test_a_marker_on_a_half_pixel_lands_on_the_row_agg_composes():
    # The offset is 112.50000000000003 pixels up; composed with the flip
    # into the device as Agg composes it, it truncates to row 138.
    fig = plt.figure(figsize=(5, 2.5))
    fig.subplots_adjust(left=0.1, right=0.95, bottom=0.15, top=0.9)
    ax = fig.add_subplot(121)
    ax.scatter([158.0], [317.0], c="k", s=9)
    ax.set(xlim=(58, 218), ylim=(173, 533))
    buf = io.BytesIO()
    fig.savefig(buf, format="png")
    buf.seek(0)
    image = plt.imread(buf)
    dark = np.nonzero(image[100:180, 150:190, 0] < 0.5)[0] + 100
    assert sorted({int(row) for row in dark}) == [136, 137, 138, 139, 140]


def test_built_in_colormaps_keep_the_segment_data_matplotlib_writes():
    jet = pltrs.colormaps["jet"]._segmentdata
    assert list(jet) == ["red", "green", "blue"]
    assert jet["red"][:2] == ((0.0, 0, 0), (0.35, 0, 0))
    assert [type(v) for v in jet["red"][0]] == [float, int, int]
    assert pltrs.colormaps["jet_r"]._segmentdata["blue"][:2] == [
        (0.0, 0, 0), (0.35, 0, 0)]
    assert type(pltrs.colormaps["coolwarm"]._segmentdata["red"]) is list
    blues = pltrs.colormaps["Blues"]._segmentdata
    assert list(blues) == ["red", "green", "blue", "alpha"]
    assert isinstance(blues["red"], np.ndarray) and blues["red"].shape == (9, 3)
    grey = pltrs.colormaps["grey"]._segmentdata["red"]
    assert grey == ((0.0, 0, 0), (1.0, 1, 1)) and type(grey[0][1]) is int
