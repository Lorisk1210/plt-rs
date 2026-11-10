"""
What the seventh round of corpora (``cargo xtask diff --corpus``) found
pltrs doing otherwise than matplotlib 3.11, reduced to matplotlib's API:
a projection class setting itself up in ``set_figure`` (mpltern), the
checks of colour sequences (UpSetPlot), ``Axes._add_text`` (mplhep), a
legend kept with ``add_artist`` (pyCirclize), TeX's preamble read when TeX
runs (SciencePlots), and what CORP6 left: a 3-D axes' own limit
callbacks, the program's styles derived from matplotlib's, contours
normalising through the program's norm at each draw. Every expected value
was recorded from matplotlib.
"""

import collections
import io
import shutil
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
import pltrs.transforms as mtransforms
from pltrs.axes import Axes
from pltrs.colors import Normalize, to_rgba_array
from pltrs.lines import Line2D
from pltrs.patches import ArrowStyle, BoxStyle, ConnectionStyle, FancyArrowPatch
from pltrs.projections import register_projection
from pltrs.text import Text


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _OuterBoxAxes(Axes):
    """A projection that makes, in ``set_figure``, a box its
    ``_set_lim_and_transforms`` reads (as mpltern's ``TernaryAxes`` does)."""

    name = "corp7_outer_box"

    def set_figure(self, fig):
        self.outer_lim = mtransforms.Bbox.unit()
        super().set_figure(fig)

    def _set_lim_and_transforms(self):
        super()._set_lim_and_transforms()
        self.outer_box = mtransforms.TransformedBbox(self.outer_lim, self.transScale)


register_projection(_OuterBoxAxes)


def test_a_projection_sets_itself_up_in_set_figure():
    fig = plt.figure()
    ax = fig.add_subplot(projection="corp7_outer_box")
    assert isinstance(ax, _OuterBoxAxes)
    assert ax.outer_box.bounds == (0.0, 0.0, 1.0, 1.0)
    fig.canvas.draw()


def test_a_sequence_of_another_length_is_refused_before_its_elements_convert():
    colors = np.ma.masked_array(np.array([np.nan] * 5, dtype=object), mask=[0, 0, 1, 0, 0])
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        with pytest.raises(ValueError, match="Invalid RGBA argument: nan"):
            to_rgba_array(colors)
    with pytest.raises(ValueError, match="RGBA sequence should have length 3 or 4"):
        pltrs.colors.to_rgba(np.array([0.1, 0.2]))


def test_add_text_adds_an_unclipped_text():
    fig, ax = plt.subplots()
    text = Text(1.2, 0.5, "outside")
    assert ax._add_text(text) is text
    assert text.axes is ax and text in ax.texts
    assert text.get_clip_box() is None and text.get_clip_path() is None
    assert text.get_transform() == ax.transData
    with pytest.raises(TypeError):
        ax._add_text(Line2D([], []))


def test_a_legend_kept_with_add_artist_is_clipped_to_the_axes_patch():
    fig = plt.figure(figsize=(4, 4), dpi=50)
    ax = fig.add_subplot(projection="polar")
    first = ax.legend(handles=[Line2D([], [], label="first")],
                      bbox_to_anchor=(1.2, 1.0), loc="upper left")
    ax.add_artist(first)
    ax.legend(handles=[Line2D([], [], label="second")], loc="center")
    assert type(first.get_clip_path()).__name__ == "TransformedPatchPath"
    assert first._fully_clipped_to_axes()
    assert first not in ax.get_default_bbox_extra_artists()
    renderer = fig.canvas.get_renderer()
    assert ax.get_tightbbox(renderer).width < 200
    fig2, ax2 = plt.subplots()
    kept = ax2.legend(handles=[Line2D([], [], label="kept")])
    ax2.add_artist(kept)
    assert kept._fully_clipped_to_axes()


@pytest.mark.skipif(not shutil.which("latex"), reason="needs LaTeX")
def test_tex_reads_the_preamble_when_it_runs():
    with plt.rc_context({"text.usetex": True,
                         "text.latex.preamble": r"\usepackage{nosuchpackagecorp7}"}):
        fig, ax = plt.subplots(figsize=(2, 2))
        ax.plot([0, 1], label="a")
        ax.legend()
        with pytest.raises(RuntimeError, match="latex was not able to process"):
            fig.savefig(io.BytesIO(), format="png")
    fig.savefig(io.BytesIO(), format="png")


def _record_limits(ax, seen, tag=""):
    for name in "xyz":
        ax.callbacks.connect(
            f"{name}lim_changed",
            lambda a, name=name: seen.append(
                (tag + name, *(round(float(v), 3) for v in getattr(a, f"get_{name}lim")()))))


def test_3d_axes_call_their_own_limit_callbacks():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    seen = []
    _record_limits(ax, seen)
    ax.plot([0, 1, 2], [0, 2, 4], [1, 3, 5])
    assert seen == [("x", -0.146, 2.146), ("y", -0.292, 4.292), ("z", 0.917, 5.083)]
    seen.clear()
    ax.set_xlim(-1, 3)
    ax.set_zlim(0, 10)
    assert seen == [("x", -1.0, 3.0), ("z", 0.0, 10.0)]
    seen.clear()
    ax.autoscale()
    assert seen == []
    ax.scatter([5], [5], [5])
    # The pending autoscaling first, then the scatter's.
    assert seen == [("x", -0.146, 2.146), ("y", -0.292, 4.292), ("z", 0.917, 5.083),
                    ("x", -0.365, 5.365), ("y", -0.365, 5.365), ("z", 0.708, 5.292)]
    seen.clear()
    ax.invert_zaxis()
    assert seen == [("z", 5.292, 0.708)]
    seen.clear()
    ax.cla()
    ax.set_xlim(0, 1)
    assert seen == []


def test_3d_axes_sharing_an_axis_call_each_others_callbacks():
    fig = plt.figure()
    a = fig.add_subplot(121, projection="3d")
    b = fig.add_subplot(122, projection="3d", sharez=a)
    seen = []
    _record_limits(a, seen, "a.")
    _record_limits(b, seen, "b.")
    a.set_zlim(-1, 5)
    assert seen == [("a.z", -1.0, 5.0), ("b.z", -1.0, 5.0)]
    seen.clear()
    a.cla()
    assert seen == [("b.z", 0.0, 1.0), ("b.z", -0.021, 1.021)]


_calls = collections.Counter()


class _Arc3(ConnectionStyle.Arc3):
    def connect(self, posA, posB):
        _calls["connect"] += 1
        return super().connect(posA, posB)


class _Fancy(ArrowStyle.Fancy):
    def transmute(self, path, mutation_size, linewidth):
        _calls["transmute"] += 1
        return super().transmute(path, mutation_size, linewidth)


class _Round(BoxStyle.Round):
    def __call__(self, x0, y0, width, height, mutation_size):
        _calls["box"] += 1
        return super().__call__(x0, y0, width, height, mutation_size)


@pytest.mark.parametrize("tight", [False, True])
def test_styles_derived_from_matplotlib_s_are_called_as_drawn(tight):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.add_patch(FancyArrowPatch((0.1, 0.1), (0.8, 0.7), connectionstyle=_Arc3(rad=0.3),
                                 arrowstyle=_Fancy(), mutation_scale=20))
    ax.text(0.5, 0.5, "box", bbox=dict(boxstyle=_Round(pad=0.2)))
    _calls.clear()
    if tight:
        # Drawn once before, cut to the tight box.
        fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    else:
        fig.canvas.draw()
    draws = 2 if tight else 1
    assert _calls == {"connect": draws, "transmute": draws, "box": draws}


class _Recording(Normalize):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.calls = 0

    def __call__(self, value, clip=None):
        self.calls += 1
        return super().__call__(value, clip)


@pytest.mark.parametrize("filled", [False, True])
def test_contours_call_the_program_s_norm_at_each_draw(filled):
    fig, ax = plt.subplots()
    norm = _Recording()
    data = np.arange(12.).reshape(3, 4) / 11
    (ax.contourf if filled else ax.contour)(data, norm=norm)
    for _ in range(2):
        before = norm.calls
        fig.canvas.draw()
        assert norm.calls == before + 1


def test_a_closed_figure_s_renderer_is_the_one_saving_would_use():
    # adjustText falls back on it for figures Jupyter closed already.
    from pltrs.backend_bases import _get_renderer
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, "hello")
    plt.close(fig)
    assert not hasattr(fig.canvas, "get_renderer")
    renderer = _get_renderer(fig)
    assert type(renderer).__name__ == "RendererAgg"
    assert (renderer.width, renderer.height, renderer.dpi) == (640, 480, 100.0)
    assert [round(v, 2) for v in text.get_window_extent(renderer).bounds] == [328.0, 234.26, 35.0, 14.34]
