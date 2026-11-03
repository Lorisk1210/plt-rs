"""
pltrs's own smoke tests (``cargo xtask python test``): the vertical slice
end to end, quickly. matplotlib's suite (``cargo xtask mpl-suite``) is
the real measure; these catch a broken build or binding in seconds.
"""

import io
import os
import subprocess
import sys
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import ticker
from pltrs.testing.compare import compare_images
from pltrs.testing.decorators import check_figures_equal
from pltrs.testing.exceptions import ImageComparisonFailure


@pytest.fixture(autouse=True)
def close_figures():
    """Start and end every test without figures, with matplotlib's default
    rcParams (whatever the look in force) during the test."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _look_params(look):
    """``axes.titlelocation`` and ``axes.grid`` as a fresh interpreter with
    ``PLTRS_LOOK=look`` (or without the variable, for None) starts."""
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("PLTRS_")}
    if look is not None:
        env["PLTRS_LOOK"] = look
    code = ("import pltrs; "
            "print(pltrs.rcParams['axes.titlelocation'], "
            "pltrs.rcParams['axes.grid'], "
            "pltrs.rcParamsDefault['axes.titlelocation'])")
    result = subprocess.run([sys.executable, "-c", code], env=env,
                            capture_output=True, text=True, check=True)
    return result.stdout.split()


def test_the_modern_look_is_the_default_and_matplotlibs_one_switch_away():
    assert _look_params(None) == ["left", "True", "center"]
    assert _look_params("matplotlib") == ["center", "False", "center"]


def test_pyplot_loads_what_matplotlibs_pyplot_loads():
    """Importing pyplot leaves Pillow's image modules and the 3-D toolkit
    loaded, as matplotlib's does: programs reach ``PIL.Image`` after a bare
    ``import PIL`` and ``mpl_toolkits.mplot3d`` after ``import
    mpl_toolkits``."""
    code = ("import sys, pltrs.pyplot; "
            "print(*(name in sys.modules for name in ("
            "'PIL.Image', 'PIL.PngImagePlugin', "
            "'pltrs.mpl_toolkits.mplot3d')))")
    result = subprocess.run([sys.executable, "-c", code],
                            capture_output=True, text=True, check=True)
    assert result.stdout.split() == ["True", "True", "True"]


def test_pyplot_slice_draws_and_saves_every_format(tmp_path):
    fig, ax = plt.subplots()
    line, = plt.plot([1, 2, 3], [4, 5, 6], "r--", lw=3, label="line")
    plt.scatter([1, 2], [3, 4], s=20, c="g", label="points")
    bars = plt.bar([1, 2], [3, 4], label="bars")
    plt.title("Title")
    plt.xlabel("x")
    plt.ylabel("y")
    legend = plt.legend()
    assert plt.gcf() is fig and plt.gca() is ax
    assert ax.get_title() == "Title" and ax.get_xlabel() == "x"
    assert [t.get_text() for t in legend.get_texts()] == ["line", "points", "bars"]
    assert line.get_linewidth() == 3 and line.get_linestyle() == "--"
    assert bars.patches[1].get_height() == 4
    for fmt, magic in [("png", b"\x89PNG"), ("svg", b"<?xml"), ("pdf", b"%PDF")]:
        path = tmp_path / f"figure.{fmt}"
        plt.savefig(path)
        assert path.read_bytes().startswith(magic)
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png")
    assert buffer.getvalue().startswith(b"\x89PNG")


def test_artists_keep_their_identity_and_attributes():
    fig = plt.figure()
    ax = fig.add_subplot(111)
    line, = ax.plot(np.arange(5.0))
    line.note = "kept"
    assert ax.lines[0] is line and ax.lines[0].note == "kept"
    assert fig.axes == [ax] and line.figure is fig
    np.testing.assert_array_equal(line.get_xdata(), [0, 1, 2, 3, 4])
    line.set(color="tab:green", linewidth=2)
    assert line.get_color() == "tab:green" and line.get_lw() == 2


def test_figure_numbers_and_closing():
    first = plt.figure()
    second = plt.figure("labelled")
    assert plt.get_fignums() == [1, 2]
    assert plt.figure(1) is first and plt.gcf() is first
    plt.close("labelled")
    assert plt.get_fignums() == [1] and second.number == 2
    plt.close("all")
    assert plt.get_fignums() == []


def test_rcparams_validate_through_pltrs():
    pltrs.rcParams["lines.linewidth"] = 4
    assert pltrs.rcParams["lines.linewidth"] == 4.0
    with pytest.raises(ValueError, match="lines.linewidth"):
        pltrs.rcParams["lines.linewidth"] = "wide"
    with pytest.raises(KeyError, match="not a valid value for rcParam"):
        pltrs.rcParams["no.such.key"] = 1
    with pltrs.rc_context({"lines.linewidth": 7}):
        assert pltrs.rcParams["lines.linewidth"] == 7.0
    assert pltrs.rcParams["lines.linewidth"] == 4.0
    assert "pltrs.ink" not in pltrs.rcParams
    standalone = pltrs.RcParams({"figure.figsize": (3.5, 42)})
    assert dict(standalone) == {"figure.figsize": [3.5, 42.0]}


def test_python_formatters_run_inside_draws():
    seen = []

    def label(x, pos):
        seen.append(pos)
        return f"<{x:g}>"

    fig, ax = plt.subplots()
    ax.plot([0, 1])
    ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
    fig.savefig(io.BytesIO(), format="png")
    assert seen and all(isinstance(pos, int) for pos in seen)


def test_an_exception_in_a_formatter_reaches_the_caller():
    def broken(x, pos):
        raise ZeroDivisionError("from the formatter")

    fig, ax = plt.subplots()
    ax.yaxis.set_major_formatter(broken)
    with pytest.raises(ZeroDivisionError, match="from the formatter"):
        fig.savefig(io.BytesIO(), format="png")
    ax.yaxis.set_major_formatter(ticker.NullFormatter())
    fig.savefig(io.BytesIO(), format="png")


def test_native_errors_are_matplotlibs_exceptions():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="same first dimension"):
        ax.plot([1, 2], [1, 2, 3])
    with pytest.raises(AttributeError, match="unexpected keyword argument 'bogus'"):
        ax.plot([1, 2], bogus=1)
    with pytest.raises(TypeError, match="expected a native"):
        pltrs._native.lines.get(ax._native, "linewidth")
    with pytest.raises(ValueError):
        fig.savefig(io.BytesIO(), format="nonsense")


@check_figures_equal()
def test_check_figures_equal_compares_through_pltrs(fig_test, fig_ref):
    fig_test.subplots().plot([1, 3, 5])
    fig_ref.subplots().plot([0, 1, 2], [1, 3, 5])


def test_compare_images_reports_differences(tmp_path):
    for name, values in [("a", [1, 2, 3]), ("b", [3, 2, 1])]:
        fig, ax = plt.subplots()
        ax.plot(values)
        fig.savefig(tmp_path / f"{name}.png")
    assert compare_images(tmp_path / "a.png", tmp_path / "a.png", 0) is None
    result = compare_images(tmp_path / "a.png", tmp_path / "b.png", 0,
                            in_decorator=True)
    assert result["rms"] > 0 and result["diff"].endswith("-failed-diff.png")
    small = plt.figure(figsize=(2, 2))
    small.savefig(tmp_path / "small.png")
    with pytest.raises(ImageComparisonFailure, match="sizes do not match"):
        compare_images(tmp_path / "a.png", tmp_path / "small.png", 0)


def test_handles_serve_every_thread():
    import threading
    fig, ax = plt.subplots()
    ax.set_title("made here")
    titles = []
    thread = threading.Thread(target=lambda: titles.append(ax.get_title()))
    thread.start()
    thread.join()
    assert titles == ["made here"]


def test_style_use_applies_bundled_sheets():
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        pltrs.style.use("classic")
    assert pltrs.rcParams["lines.linewidth"] == 1.0


def test_backends_switch_and_canvases_draw_print_and_blit(tmp_path):
    from pltrs.backends.backend_agg import FigureCanvasAgg
    from pltrs.backends.backend_pdf import PdfPages
    plt.switch_backend("agg")
    fig, ax = plt.subplots(figsize=(2, 1), dpi=50)
    ax.plot([0, 1], [0, 1])
    assert type(fig.canvas) is FigureCanvasAgg
    draws = []
    fig.canvas.mpl_connect("draw_event", draws.append)
    fig.canvas.draw()
    assert np.asarray(fig.canvas.buffer_rgba()).shape == (50, 100, 4)
    assert len(fig.canvas.tostring_argb()) == 50 * 100 * 4
    corner = type("Box", (), {"extents": (0, 0, 10, 10)})
    region = fig.canvas.copy_from_bbox(corner)
    assert region.get_extents() == (0, 40, 10, 50)
    fig.canvas.restore_region(region)
    raw = io.BytesIO()
    fig.savefig(raw, format="rgba", dpi=10)
    assert len(raw.getvalue()) == 20 * 10 * 4 and len(draws) == 2
    with PdfPages(tmp_path / "pages.pdf") as pdf:
        pdf.savefig(fig)
        pdf.savefig(fig)
        assert pdf.get_pagecount() == 2
    assert (tmp_path / "pages.pdf").read_bytes().startswith(b"%PDF")
    plt.switch_backend("pdf")
    assert type(plt.figure().canvas).__name__ == "FigureCanvasPdf"
    plt.switch_backend("agg")


def test_animations_save_through_the_writers(tmp_path):
    from pltrs import animation
    fig, ax = plt.subplots(figsize=(1, 1), dpi=20)
    line, = ax.plot([], [])

    def update(frame):
        line.set_data([0, 1], [0, frame])
        return line,

    anim = animation.FuncAnimation(fig, update, frames=3)
    anim.save(tmp_path / "movie.gif", writer="pillow")
    assert (tmp_path / "movie.gif").read_bytes().startswith(b"GIF8")
    assert "<script" in anim.to_jshtml()


def test_artists_made_before_they_are_added():
    from pltrs.artist import Artist
    from pltrs.lines import Line2D
    fig, ax = plt.subplots()
    line = Line2D([0, 2], [1, 3], linewidth=3)
    assert line.figure is None and line.axes is None
    assert ax.add_line(line) is line
    assert line.figure is fig and line.axes is ax and list(ax.lines) == [line]
    assert line.get_linewidth() == 3 and line.get_label() == "_child0"
    from pltrs.collections import LineCollection
    from pltrs.patches import Rectangle
    rect = Rectangle((0, 0), 4, 2, facecolor="red")
    assert ax.add_patch(rect) is rect and list(ax.patches) == [rect]
    assert rect.get_facecolor() == (1, 0, 0, 1) and rect.axes is ax
    segments = LineCollection([[(0, 0), (5, 5)]])
    assert ax.add_collection(segments) is segments
    assert list(ax.collections) == [segments] and ax.get_xlim()[1] >= 5
    with pytest.raises(ValueError, match="Can not reset the Axes"):
        fig.add_subplot(212).add_line(line)
    with pytest.raises(RuntimeError, match="more than one figure"):
        plt.figure().add_subplot().add_line(line)
    assert line.get_transform() is ax.transData
    assert ax.transAxes.transform((1, 1)) == pytest.approx(ax.bbox.p1)
    with pytest.raises(NotImplementedError, match="cannot remove artist"):
        Artist().remove()


def test_grids_and_legends_keep_matplotlibs_api():
    fig = plt.figure()
    gs = fig.add_gridspec(2, 2)
    ax = fig.add_subplot(gs[0, :])
    assert ax.get_subplotspec() == gs[0, :] and ax.get_gridspec() is gs
    ax.plot([0, 1], label="a")
    legend = ax.legend(loc="upper left")
    assert [t.get_text() for t in legend.get_texts()] == ["a"]
    assert legend._loc == 2 and ax.get_legend() is legend


def test_norms_of_other_classes_map_images_through_python():
    from pltrs import colors

    class Halving(colors.Normalize):
        def __call__(self, value, clip=None):
            return np.ma.asarray(value) / 2

    fig, ax = plt.subplots()
    data = np.array([[0.2, 1.6]])
    im = ax.imshow(data, norm=Halving(0, 1), cmap="gray",
                   interpolation="nearest")
    fig.canvas.draw()
    pixels = im.make_image(None)[0]
    expected = (im.cmap(data[0] / 2, bytes=True))
    assert (pixels[0, 0] == expected[0]).all()
    assert (pixels[0, -1] == expected[1]).all()


def test_formatters_see_every_tick_and_give_the_offset():
    formatter = ticker.ScalarFormatter(useOffset=True)
    fig, ax = plt.subplots()
    ax.plot([1e5 + 1, 1e5 + 2])
    ax.yaxis.set_major_formatter(formatter)
    fig.canvas.draw()
    assert formatter.get_offset() != ""


def test_wide_floats_warn_when_drawn():
    fig, ax = plt.subplots()
    ax.imshow(np.zeros((3, 3), dtype=np.longdouble))
    if np.dtype(np.longdouble).itemsize > 8:
        with pytest.warns(UserWarning, match="Casting input data from"):
            fig.canvas.draw()


def test_replaced_text_measurement_and_drawing_take_effect(monkeypatch):
    from pltrs import text
    from pltrs.patches import Rectangle

    def measure(renderer, s, fontprop, ismath, dpi):
        return 10.0 * len(s), fontprop.get_size(), 0.0

    def draw(self, renderer):
        bbox = self.get_window_extent()
        Rectangle(bbox.p0, bbox.width, bbox.height, facecolor="red",
                  edgecolor="none").draw(renderer)

    fig = plt.figure(figsize=(2, 1), dpi=100)
    label = fig.text(0.1, 0.5, "abcd", fontsize=20)
    measured = label.get_window_extent().width
    monkeypatch.setattr(text, "_get_text_metrics_with_cache", measure)
    monkeypatch.setattr(text.Text, "draw", draw)
    assert label.get_window_extent().width == 40.0
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())
    assert (pixels[45, 30] == [255, 0, 0, 255]).all(), "a box, not glyphs"
    monkeypatch.undo()
    assert label.get_window_extent().width == measured


def test_tight_layout_warns_as_matplotlib_does():
    fig, axs = plt.subplots(1, 2)
    fig.add_axes([0.9, 0.1, 0.02, 0.8])
    with pytest.warns(UserWarning, match="not compatible with tight_layout"):
        fig.tight_layout()
    fig, ax = plt.subplots(layout="constrained")
    with pytest.warns(UserWarning, match="layout has changed to tight"):
        fig.tight_layout()


def test_tick_helpers_keep_their_identity_and_drive_the_ticks():
    fig, ax = plt.subplots()
    ax.set_xlim(0, 1)
    assert ax.xaxis.get_major_locator() is ax.xaxis.major.locator
    assert isinstance(ax.xaxis.get_major_formatter(), ticker.ScalarFormatter)
    locator = ticker.MultipleLocator(0.5)
    ax.xaxis.set_major_locator(locator)
    assert ax.xaxis.get_major_locator() is locator
    assert not ax.xaxis.isDefault_majloc
    # One tick beyond each end of the view, as matplotlib's locator gives.
    np.testing.assert_allclose(ax.get_xticks(), np.arange(-0.5, 1.6, 0.5))
    locator.set_params(base=0.25)
    np.testing.assert_allclose(ax.get_xticks(), np.arange(-0.25, 1.3, 0.25))


def test_ticks_and_axis_texts_mirror_the_axis():
    fig, ax = plt.subplots()
    ax.set_xticks([0, 0.5, 1])
    assert [label.get_text() for label in ax.get_xticklabels()] == [
        "0.0", "0.5", "1.0"]
    ax.tick_params(labelsize=20, labelcolor="red")
    tick = ax.xaxis.get_major_ticks()[1]
    assert (tick.label1.get_fontsize(), tick.label1.get_color()) == (20, "red")
    tick.label1.set_rotation(30)
    assert ax.xaxis.get_major_ticks()[1].label1.get_rotation() == 30
    assert ax.xaxis.get_major_ticks()[0].label1.get_rotation() == 0
    ax.set_ylabel("height", loc="top")
    assert ax.yaxis.label.get_text() == "height"
    assert ax.yaxis.label.get_horizontalalignment() == "right"


def test_projection_classes_of_the_program_draw_through_their_transforms():
    from pltrs import axis as maxis, transforms
    from pltrs.axes import Axes
    from pltrs.projections import register_projection

    class WideXAxis(maxis.XAxis):
        def get_view_interval(self):
            return -1.0, self.axes.viewLim.intervalx[1]

    class LeaningAxes(Axes):
        name = "pltrs-smoke-leaning"

        def _init_axis(self):
            self.xaxis = WideXAxis(self)
            self.yaxis = maxis.YAxis(self)

        def _set_lim_and_transforms(self):
            super()._set_lim_and_transforms()
            self.transData = (self.transScale + (
                self.transLimits + transforms.Affine2D().skew_deg(45, 0))
                + self.transAxes)

    register_projection(LeaningAxes)
    fig = plt.figure(figsize=(4, 4), dpi=100)
    ax = fig.add_axes([0, 0, 1, 1], projection="pltrs-smoke-leaning")
    assert isinstance(ax, LeaningAxes) and ax.name == "pltrs-smoke-leaning"
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.plot([0, 0], [0, 1], color="k")
    np.testing.assert_allclose(ax.transData.transform((0, 1)), (400, 400))
    assert min(ax.get_xticks()) < 0, "ticked over the axis' own interval"
    pixels = _pixels(fig)
    assert pixels[200, 200, 0] < 128 and pixels[200, 50, 0] == 255, "the line leans"


def test_a_single_tick_takes_a_pad_of_its_own():
    fig, ax = plt.subplots()
    ax.set_xticks([0, 0.5, 1])
    fig.canvas.draw()
    labels = ax.xaxis.get_major_ticks()
    before = [tick.label1.get_window_extent().y1 for tick in labels]
    labels[1].set_pad(labels[1].get_pad() + 10)
    assert ax.xaxis.get_major_ticks()[1].get_pad() == labels[0].get_pad() + 10
    fig.canvas.draw()
    after = [tick.label1.get_window_extent().y1 for tick in labels]
    assert after[1] == pytest.approx(before[1] - 10 / 72 * fig.dpi)
    assert (after[0], after[2]) == (before[0], before[2])

def _pixels(fig):
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


class _Halving(pltrs.colors.Normalize):
    """A norm of a class of the program's (pickled by reference)."""

    def __call__(self, value, clip=None):
        return np.ma.asarray(value) / 2


def test_figures_pickle_with_their_python_objects():
    import pickle

    fig = plt.figure()
    ax = fig.add_subplot(1, 2, 1)
    polar = fig.add_subplot(1, 2, 2, projection="polar")
    polar.plot([0, 1, 2], [1, 2, 3])
    im = ax.imshow([[0.2, 1.6]], norm=_Halving(0, 1), cmap="gray")
    ax.yaxis.set_major_formatter(ticker.ScalarFormatter(useOffset=True))
    ax.xaxis.set_major_formatter(ticker.FuncFormatter("{:.1f}".format))
    fig.colorbar(im, ax=ax)
    back = pickle.loads(pickle.dumps(fig))
    assert type(back.axes[0].images[0].norm) is _Halving
    assert (_pixels(back) == _pixels(fig)).all()


def test_loose_artists_and_native_values_pickle():
    import copy
    import pickle
    from pltrs.lines import Line2D

    line = pickle.loads(pickle.dumps(Line2D([0, 1], [2, 3], color="r")))
    assert line.figure is None and list(line.get_ydata()) == [2, 3]
    fig, ax = plt.subplots()
    ax.plot([0, 1])
    twin = copy.deepcopy(fig)
    assert twin.axes[0] is not ax and twin.axes[0].lines[0].get_xydata().shape == (2, 2)
    transform = pickle.loads(pickle.dumps(ax.transData))
    assert (transform.transform([[0, 0]]) == ax.transData.transform([[0, 0]])).all()


def test_closures_of_rust_code_refuse_to_pickle_as_lambdas_do():
    import pickle

    fig, ax = plt.subplots()
    ax.xaxis.set_major_formatter(lambda x, pos: str(x))
    with pytest.raises((pickle.PicklingError, AttributeError, TypeError)):
        pickle.dumps(fig)


def test_date_and_category_axes_pickle_with_their_helpers():
    import datetime
    import gc
    import pickle

    fig, (dates, fruit) = plt.subplots(1, 2)
    dates.plot([datetime.datetime(2020, 1, d) for d in range(1, 10)], range(9))
    fruit.bar(["apple", "pear", "fig"], [1, 3, 2])
    expected = _pixels(fig)
    back = pickle.loads(pickle.dumps(fig))
    gc.collect()  # the helpers stay alive with the figure read back
    assert (_pixels(back) == expected).all()
    assert [t.get_text() for t in back.axes[1].get_xticklabels()] == [
        "apple", "pear", "fig"]


def test_units_convert_before_reaching_pltrs():
    import datetime

    from pltrs import units

    class Miles:
        """A distance, which its converter plots in miles or feet."""

        def __init__(self, value):
            self.value = value

    class MilesConverter(units.ConversionInterface):
        @staticmethod
        def convert(value, unit, axis):
            factor = 5280 if unit == "feet" else 1
            if np.iterable(value):
                return [v.value * factor for v in value]
            return value.value * factor

        @staticmethod
        def default_units(value, axis):
            return "miles"

        @staticmethod
        def axisinfo(unit, axis):
            return units.AxisInfo(label=unit)

    units.registry[Miles] = MilesConverter()
    try:
        fig, ax = plt.subplots()
        line, = ax.plot([Miles(1), Miles(2)], [1, 2])
        assert ax.xaxis.get_units() == "miles" and ax.get_xlabel() == "miles"
        ax.xaxis.set_units("feet")
        assert list(line.get_xdata(orig=False)) == [5280, 10560]
        assert ax.get_xlabel() == "feet"
    finally:
        del units.registry[Miles]
    fig, (dates, fruit) = plt.subplots(1, 2)
    dates.axvline(datetime.datetime(2020, 1, 2))
    dates.set_xlim(datetime.datetime(2020, 1, 1), datetime.datetime(2020, 1, 3))
    assert dates.get_xlim() == (18262.0, 18264.0)
    fruit.bar(["a", "b"], [1, 2])
    fig.canvas.draw()
    assert [t.get_text() for t in fruit.get_xticklabels()] == ["a", "b"]


def test_navigation_sees_the_figure_as_matplotlib_leaves_it():
    from pltrs.backend_bases import MouseButton, MouseEvent, NavigationToolbar2
    fig, ax = plt.subplots()
    ax.plot(np.arange(10))
    ax.set_aspect("equal")
    # Before a draw, transData maps the view of a fresh axes, (0, 1), onto
    # the position the aspect has not adjusted yet.
    x0, y0, width, height = ax.bbox.bounds
    assert ax.transData.transform((0.5, 0.5)) == pytest.approx(
        (x0 + width / 2, y0 + height / 2))
    start = ax.transData.transform((0.5, 0.5)).astype(int)
    stop = ax.transData.transform((0.2, 0.5)).astype(int)
    left = MouseButton.LEFT
    events = [
        MouseEvent("button_press_event", fig.canvas, *start, button=left),
        MouseEvent("motion_notify_event", fig.canvas, *stop, button=left,
                   buttons={left}),
        MouseEvent("button_release_event", fig.canvas, *stop, button=left)]
    toolbar = NavigationToolbar2(fig.canvas)
    toolbar.pan()
    toolbar.press_pan(events[0])
    toolbar.drag_pan(events[1])
    toolbar.release_pan(events[2])
    assert ax.get_xlim()[0] > 3, "the pan moves the laid out axes"
    seen = []
    fig.add_axobserver(seen.append)
    fig.canvas.grab_mouse(ax)
    ax.remove()
    assert seen == [fig] and fig.canvas.mouse_grabber is None


def test_blitting_draws_single_artists_onto_the_canvas():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    fig.canvas.draw()
    background = fig.canvas.copy_from_bbox(ax.bbox)
    line, = ax.plot([0, 1], [0, 1], color="red", lw=5, animated=True)
    before = np.asarray(fig.canvas.buffer_rgba()).copy()
    ax.draw_artist(line)
    assert (np.asarray(fig.canvas.buffer_rgba()) != before).any()
    fig.canvas.restore_region(background)
    assert (np.asarray(fig.canvas.buffer_rgba()) == before).all()


def test_pillow_writes_the_formats_matplotlib_writes_with_it():
    from PIL import Image
    fig = plt.figure(figsize=(1, 1), dpi=20)
    gif = io.BytesIO()
    fig.savefig(gif, format="gif")
    assert Image.open(gif).mode == "P"
    tiff = io.BytesIO()
    fig.savefig(tiff, format="tiff", pil_kwargs={"description": "smoke"})
    assert Image.open(tiff).tag_v2[270] == "smoke"


def test_the_main_thread_runs_other_threads_calls_in_its_event_loop():
    import threading
    from pltrs.backend_bases import MouseEvent
    fig, ax = plt.subplots()
    xy = ax.transData.transform((0.25, 0.5))

    def press():
        MouseEvent("button_press_event", fig.canvas, *xy, 1)._process()

    threading.Timer(0.05, press).start()
    (clicked,) = fig.ginput(timeout=10)
    assert clicked == pytest.approx((0.25, 0.5))
    assert not ax.lines, "the clicks shown are removed"


def test_constrained_layout_returns_the_grids_it_solved():
    from pltrs import _layoutgrid
    fig, axs = plt.subplots(1, 2, layout="constrained")
    axs[0].set_title("title")
    grids = fig.get_layout_engine().execute(fig)
    assert grids["hasgrids"] and isinstance(grids[fig], _layoutgrid.LayoutGrid)
    spec = grids[axs[0].get_gridspec()]
    assert spec.name.startswith(grids[fig].name + ".gridspec")
    assert spec.get_inner_bbox(0, 0).bounds == pytest.approx(
        axs[0].get_position().bounds)
    assert spec.get_outer_bbox(0, 0).x1 == spec.get_outer_bbox(0, 1).x0
    _layoutgrid.plot_children(fig)
    assert len(fig.artists) == 6 * 3, "six boxes per cell of each grid"


def test_bbox_images_fill_the_box_their_callable_gives():
    from pltrs.image import BboxImage
    from pltrs.transforms import Bbox
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, "backed")
    behind = BboxImage(text.get_window_extent)
    behind.set_data(np.arange(4.0).reshape(2, 2))
    ax.add_artist(behind)
    corner = BboxImage(lambda renderer: Bbox.from_bounds(
        10, 10, renderer.points_to_pixels(36), 30))
    corner.set_data(np.arange(4.0).reshape(2, 2))
    ax.add_artist(corner)
    fig.canvas.draw()
    assert behind.get_window_extent().bounds == text.get_window_extent().bounds
    assert corner.get_window_extent().bounds == pytest.approx(
        (10, 10, fig.dpi / 2, 30))


def test_generated_set_methods_list_the_subclass_properties():
    # Values recorded from matplotlib 3.11.2.
    import inspect
    from pltrs.artist import Artist
    from pltrs.lines import Line2D

    class MyArtist(Artist):
        def set_myparam1(self, val):
            """Set the parameter."""
            self.myparam1 = val

    class MyLine(Line2D):
        def set_myparam2(self, val):
            self.myparam2 = val

    parameters = list(inspect.signature(MyArtist.set).parameters)
    assert parameters[:6] == ["self", "agg_filter", "alpha", "animated", "clip_box",
                              "clip_on"]
    assert len(parameters) == 21
    assert "myparam1" in MyArtist.set.__doc__
    assert MyArtist.set.__qualname__ == f"{MyArtist.__qualname__}.set"
    assert MyArtist.set is MyArtist.set
    assert "myparam2" in MyLine.set.__doc__ and "myparam1" not in MyLine.set.__doc__
    assert (set(inspect.signature(MyLine.set).parameters)
            - set(inspect.signature(Line2D.set).parameters)) == {"myparam2"}
    artist = MyArtist()
    artist.set(myparam1=3)
    assert artist.myparam1 == 3


def test_dropping_the_text_caches_draws_the_same():
    from pltrs import _native

    def drawn():
        fig, ax = plt.subplots()
        ax.set_title(r"$\sum_{n=1}^{5} \frac{x^n}{n!}$ and plain text")
        ax.text(0.5, 0.5, "middle")
        buffer = io.BytesIO()
        fig.savefig(buffer, format="png", metadata={"Software": None})
        plt.close(fig)
        return buffer.getvalue()

    first = drawn()
    _native.text_overrides.text_engine_clear_caches()
    assert drawn() == first


def _link_or_skip(link, target):
    """Make *link* a symbolic link to the directory *target*, or skip the
    test where the system makes none (Windows without the privilege, Wine,
    which reports links it does not make)."""
    try:
        link.symlink_to(target, target_is_directory=True)
    except OSError:
        pytest.skip("this system makes no symbolic links")
    if not link.is_symlink():
        pytest.skip("this system makes no symbolic links")


def test_shared_data_links_lead_to_this_package(tmp_path):
    # The data directory is shared by every installation of pltrs: a link
    # to one since removed, or to another installation, is pointed here.
    from pathlib import Path
    shipped = Path(pltrs.__file__).with_name("mpl-data")
    gone = tmp_path / "removed-install"
    _link_or_skip(tmp_path / "sample_data", gone / "sample_data")
    other = tmp_path / "other-install" / "images"
    other.mkdir(parents=True)
    _link_or_skip(tmp_path / "images", other)
    pltrs._reach_shipped_data(str(tmp_path))
    for name in ("sample_data", "images"):
        assert (tmp_path / name).resolve() == (shipped / name).resolve()
    assert (tmp_path / "sample_data" / "goog.npz").is_file()


def _refuse_link(link, target, target_is_directory=False):
    raise OSError(1314, "A required privilege is not held by the client")


def _pretend_link(link, target, target_is_directory=False):
    pass


@pytest.mark.parametrize("make_link", [_refuse_link, _pretend_link],
                         ids=["windows-without-privilege", "wine"])
def test_shipped_data_is_copied_where_no_link_is_made(tmp_path, monkeypatch, make_link):
    from pathlib import Path
    monkeypatch.setattr(Path, "symlink_to", make_link)
    pltrs._reach_shipped_data(str(tmp_path))
    assert (tmp_path / "sample_data" / "goog.npz").is_file()
    assert not (tmp_path / "sample_data").is_symlink()
    assert any((tmp_path / "images").iterdir())
