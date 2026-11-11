"""
Drawing defined in Python, done where pltrs (Rust) draws: artists of the
program's classes drawn through their ``draw``, subclasses of pltrs's
artists drawing themselves with ``super().draw``, path effects of the
program's classes, the renderers each format hands the program, and
renderers of the program's. The expected values were recorded from
matplotlib 3.11.2 running the same code.
"""

import importlib
import importlib.abc
import importlib.util
import io
import pathlib
import pickle
import sys

import numpy as np
import pytest

import pltrs
import pltrs.patheffects as pe
import pltrs.pyplot as plt
from pltrs.artist import Artist
from pltrs.backend_bases import RendererBase
from pltrs.backends.backend_mixed import MixedModeRenderer
from pltrs.lines import Line2D
from pltrs.image import BboxImage
from pltrs.offsetbox import AnchoredOffsetbox, AuxTransformBox
from pltrs.patches import Arc, PathPatch
from pltrs.path import Path
from pltrs.text import Text, TextPath
from pltrs.transforms import Affine2D, Bbox, IdentityTransform

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


class Cross(Artist):
    """A red diagonal in display pixels, whatever its visibility."""

    def draw(self, renderer):
        gc = renderer.new_gc()
        gc.set_foreground("red")
        gc.set_linewidth(3)
        renderer.draw_path(gc, Path([[10, 10], [200, 200]]),
                           IdentityTransform())
        gc.restore()


def _pixels(fig, fmt="png", **kwargs):
    """The figure saved as PNG, as an (H, W, 4) array of fractions."""
    buf = io.BytesIO()
    fig.savefig(buf, format=fmt, **kwargs)
    buf.seek(0)
    return plt.imread(buf)


def _red(img):
    """How many pixels of *img* are red."""
    return int(((img[..., 0] > 0.8) & (img[..., 1] < 0.3)
                & (img[..., 2] < 0.3)).sum())


def test_artists_of_the_program_draw_through_their_draw():
    fig = plt.figure(figsize=(6.4, 4.8), dpi=100)
    fig.add_subplot()
    cross = Cross()
    fig.add_artist(cross)
    assert _red(_pixels(fig)) == 333
    cross.set_visible(False)  # matplotlib calls `draw` whatever it is
    assert _red(_pixels(fig)) == 333
    cross.remove()
    assert _red(_pixels(fig)) == 0


class LabelledLine(Line2D):
    """matplotlib's "artist within an artist" example: a line drawing its
    label at its end."""

    def __init__(self, *args, **kwargs):
        self.text = Text(0, 0, '')
        super().__init__(*args, **kwargs)
        self.text.set_text(self.get_label())

    def set_figure(self, figure):
        self.text.set_figure(figure)
        super().set_figure(figure)

    @Line2D.axes.setter
    def axes(self, new_axes):
        self.text.axes = new_axes
        Line2D.axes.fset(self, new_axes)

    def set_transform(self, transform):
        self.text.set_transform(transform + Affine2D().translate(2, 2))
        super().set_transform(transform)

    def set_data(self, x, y):
        if len(x):
            self.text.set_position((x[-1], y[-1]))
        super().set_data(x, y)

    def draw(self, renderer):
        super().draw(renderer)
        self.text.draw(renderer)


def test_subclasses_draw_themselves_with_super_draw():
    fig, ax = plt.subplots()
    line = LabelledLine([0, 0.5, 0.8], [0.2, 0.9, 0.3], color="red",
                        label="end")
    line.text.set_color("red")
    ax.add_line(line)
    img = _pixels(fig)
    assert _red(img) == 1157
    assert line.text.get_window_extent().bounds == pytest.approx(
        (478.8, 162.3434, 27.0, 14.3366), abs=1e-4)
    assert line.text.get_transform().transform((0.8, 0.3)) == pytest.approx(
        ax.transData.transform((0.8, 0.3)) + 2)


class Angle(Arc):
    """An arc whose angles and centre are computed when it is drawn, in
    display pixels (matplotlib's ``AngleAnnotation``, cut down)."""

    def __init__(self, ax, xy, p1, p2):
        self.ax, self._xydata, self.vec1, self.vec2 = ax, xy, p1, p2
        super().__init__(xy, 80, 80, theta1=self.theta1, theta2=self.theta2,
                         color="red", lw=2)
        self.set_transform(IdentityTransform())
        ax.add_patch(self)

    def _direction(self, vec):
        x, y = self.ax.transData.transform(vec) - self._center
        return np.rad2deg(np.arctan2(y, x))

    _center = property(lambda self: self.ax.transData.transform(self._xydata),
                       lambda self, xy: None)
    theta1 = property(lambda self: self._direction(self.vec1), lambda self, a: None)
    theta2 = property(lambda self: self._direction(self.vec2), lambda self, a: None)


def test_geometry_a_subclass_computes_is_drawn():
    fig, ax = plt.subplots()
    ax.set(xlim=(0, 10), ylim=(0, 10))
    Angle(ax, (5, 5), (9, 5), (5, 9))
    assert _red(_pixels(fig)) == 151


class Stretched(pltrs.image.AxesImage):
    """An image filling a box, its rows made when it is drawn (the ribbon
    box example's): drawing changes the artist."""

    zorder = 1

    def __init__(self, ax, bbox):
        super().__init__(ax, extent=(0, 1, 0, 1), cmap="Reds")
        self._bbox = bbox
        self.set_transform(pltrs.transforms.BboxTransformTo(bbox))

    def draw(self, renderer):
        rows = int(self._bbox.height / self._bbox.width * 10)
        if self.get_array() is None or self.get_array().shape[0] != rows:
            self.set_array(np.linspace(0, 1, rows * 10).reshape(rows, 10))
        super().draw(renderer)


def test_drawing_may_change_the_figure_and_reads_it_laid_out():
    from pltrs.transforms import Bbox, TransformedBbox
    fig, ax = plt.subplots()
    box = TransformedBbox(Bbox.from_extents(1, 0, 2, 8), ax.transData)
    image = Stretched(ax, box)
    ax.add_artist(image)
    ax.set(xlim=(0, 4), ylim=(0, 10))
    img = _pixels(fig)
    assert image.get_array().shape == (23, 10)
    assert image.get_zorder() == 1
    assert _red(img) == 5880


class Ghost(pe.AbstractPathEffect):
    """Draws the path again, offset, in a pale colour, then the path."""

    def __init__(self, offset=(3, -3), color="0.7"):
        super().__init__(offset)
        self._color = color

    def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
        gc0 = renderer.new_gc()
        gc0.copy_properties(gc)
        gc0.set_foreground(self._color)
        face = None if rgbFace is None else pltrs.colors.to_rgb(self._color)
        renderer.draw_path(gc0, tpath, affine + self._offset_transform(renderer),
                           face)
        gc0.restore()
        renderer.draw_path(gc, tpath, affine, rgbFace)


class Thick(pe.Stroke):
    """A stroke three times as wide, which widens the graphics context the
    effects after it receive (matplotlib hands them all the same one)."""

    def draw_path(self, renderer, gc, tpath, affine, rgbFace):
        gc.set_linewidth(gc.get_linewidth() * 3)
        super().draw_path(renderer, gc, tpath, affine, rgbFace)


def test_path_effects_of_the_program_draw_their_paths():
    fig, axs = plt.subplots(1, 3, figsize=(6, 2))
    axs[0].plot([0, 1, 2], [0, 1, 0], lw=3, path_effects=[Ghost()])
    axs[1].bar([0, 1], [1, 2], color="red",
               path_effects=[Ghost((2, 2), "pink")])
    axs[2].plot([0, 1], [0, 1], "o-", color="red",
                path_effects=[Thick(foreground="r"), pe.Normal()])
    img = _pixels(fig)
    pale = (np.abs(img[..., :3] - 0.7) < 0.02).all(axis=2).sum()
    pink = (np.abs(img[..., :3] - pltrs.colors.to_rgb("pink")) < 0.02).all(
        axis=2).sum()
    assert (int(pale), int(pink), _red(img)) == (794, 673, 13680)
    for fmt in ["pdf", "svg", "ps"]:
        fig.savefig(io.BytesIO(), format=fmt)


def test_each_format_hands_the_program_matplotlibs_renderer():
    seen = []

    class Probe(Artist):
        def draw(self, renderer):
            inner = getattr(renderer, "_renderer", renderer)
            seen.append((type(renderer).__name__, type(inner).__name__,
                         renderer.points_to_pixels(10),
                         tuple(map(float, renderer.get_canvas_width_height())),
                         renderer.flipy(), renderer.option_scale_image(),
                         renderer.get_image_magnification(),
                         float(self.get_figure(root=True).dpi)))

    fig = plt.figure(figsize=(6.4, 4.8), dpi=100)
    fig.add_artist(Probe())
    for fmt in ["png", "pdf", "svg", "ps", "pgf"]:
        fig.savefig(io.BytesIO(), format=fmt, dpi=150)
    assert seen[0] == ('RendererAgg', 'RendererAgg', 10 * 150 / 72,
                       (960.0, 720.0), True, False, 1.0, 150.0)
    vector = [(name, inner, flipy) for name, inner, _, _, flipy, *_ in seen[1:]]
    assert vector == [('MixedModeRenderer', 'RendererPdf', False),
                      ('MixedModeRenderer', 'RendererSVG', True),
                      ('MixedModeRenderer', 'RendererPS', False),
                      ('MixedModeRenderer', 'RendererPgf', False)]
    assert seen[1][3] == pytest.approx((460.8, 345.6))
    assert seen[1][2:] == (10, seen[1][3], False, True, 150 / 72, 72.0)
    assert seen[4][2] == pytest.approx(10 * 150 / 72)


class Recorder(RendererBase):
    """A renderer of the program's, recording what it is asked to draw."""

    def __init__(self):
        super().__init__()
        self.calls = []

    def get_canvas_width_height(self):
        return 640, 480

    def points_to_pixels(self, points):
        return points * 100 / 72

    def draw_path(self, gc, path, transform, rgbFace=None):
        self.calls.append(("path", len(path.vertices), tuple(gc.get_rgb()),
                           gc.get_linewidth(),
                           None if rgbFace is None else tuple(rgbFace)))

    def draw_image(self, gc, x, y, im, transform=None):
        self.calls.append(("image", im.shape))


def test_artists_draw_through_renderers_of_the_program():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1, 2], [0, 1, 0], "r-", lw=2)
    fig.canvas.draw()
    recorder = Recorder()
    line.draw(recorder)
    assert recorder.calls == [("path", 3, (1.0, 0.0, 0.0, 1.0), 2.0, None)]
    recorder = Recorder()
    fig.draw(recorder)
    assert len(recorder.calls) == 37
    line.set_rasterized(True)
    recorder = Recorder()
    fig.draw(MixedModeRenderer(fig, 6.4, 4.8, 100, recorder,
                               bbox_inches_restore=None))
    assert [c for c in recorder.calls if c[0] == "image"] == [
        ("image", (340, 456, 4))]


class Shadows(Artist):
    """Draws artists again, together, through an image filter (the agg
    filter example's ``FilteredArtistList``)."""

    def __init__(self, artists, filter):
        super().__init__()
        self._artists = artists
        self._filter = filter

    def draw(self, renderer):
        renderer.start_rasterizing()
        renderer.start_filter()
        for a in self._artists:
            a.draw(renderer)
        renderer.stop_filter(self._filter)
        renderer.stop_rasterizing()


def _offset_grey(image, dpi):
    """The image, grey and half transparent, 6 pixels lower."""
    shadow = np.zeros_like(image)
    shadow[..., 3] = image[..., 3] * 0.5
    return shadow, 0, -6


def test_drawing_of_the_program_filters_and_rasterizes():
    fig, ax = plt.subplots()
    bars = ax.bar([0, 1], [1, 2], color="red")
    shadows = Shadows(list(bars), _offset_grey)
    ax.add_artist(shadows)
    shadows.set_zorder(0.5)
    img = _pixels(fig)
    grey = ((np.abs(img[..., 0] - img[..., 1]) < 0.01)
            & (img[..., 0] > 0.3) & (img[..., 0] < 0.6)).sum()
    assert (int(grey), _red(img)) == (2824, 105600)
    pdf = io.BytesIO()
    fig.savefig(pdf, format="pdf")
    assert b"/Subtype /Image" in pdf.getvalue()


def test_figures_with_artists_of_the_program_pickle():
    fig = plt.figure(figsize=(6.4, 4.8), dpi=100)
    fig.add_artist(Cross())
    back = pickle.loads(pickle.dumps(fig))
    assert type(back.artists[0]) is Cross
    assert _red(_pixels(back)) == 948


class PathClippedImagePatch(PathPatch):
    """matplotlib's ``demo_text_path`` example's patch filled with an image:
    its draw draws an image clipped to its path, then itself."""

    def __init__(self, path, bbox_image, **kwargs):
        super().__init__(path, **kwargs)
        self.bbox_image = BboxImage(self.get_window_extent, norm=None,
                                    origin=None)
        self.bbox_image.set_data(bbox_image)

    def set_facecolor(self, color):
        super().set_facecolor("none")

    def draw(self, renderer=None):
        self.bbox_image.set_clip_path(self._path, self.get_transform())
        self.bbox_image.draw(renderer)
        super().draw(renderer)


def test_drawing_may_draw_artists_the_figure_does_not_hold():
    fig, ax = plt.subplots()
    image = np.zeros((10, 10, 3))
    image[..., 0] = 1
    patch = PathClippedImagePatch(TextPath((0, 0), "!?", size=150), image,
                                  ec="k")
    box = AuxTransformBox(IdentityTransform())
    box.add_artist(patch)
    ax.add_artist(AnchoredOffsetbox(loc="upper left", child=box, frameon=True,
                                    borderpad=0.2))
    assert _red(_pixels(fig)) == 2727


#: Examples of matplotlib's gallery drawing through Python, as the spec's
#: figure cases run them (matplotlib code, imported under pltrs), with the
#: tolerance of the spec's pixel comparisons to the images matplotlib drew.
_GALLERY = [
    ("cases_gallery_misc", "gallery_misc/demo_agg_filter"),
    ("cases_gallery_misc", "gallery_misc/demo_ribbon_box"),
    ("cases_gallery_misc", "gallery_misc/patheffect_demo"),
    ("cases_gallery_misc", "gallery_misc/patheffect_demo_custom"),
    ("cases_gallery_text", "gallery_text/angle_annotation_1"),
    ("cases_gallery_text", "gallery_text/angle_annotation_2"),
    ("cases_gallery_text", "gallery_text/line_with_text"),
]
_TOLERANCE = 1.0
_SPEC = pathlib.Path(__file__).resolve().parents[2] / "spec" / "reference"


class _Alias(importlib.abc.Loader):
    """Loads a name of matplotlib's as pltrs's module *real*."""

    def __init__(self, real):
        self.real = real
        self.real_spec = None

    def create_module(self, spec):
        module = importlib.import_module(self.real)
        self.real_spec = module.__spec__
        return module

    def exec_module(self, module):
        """The module is executed already; it gets its own ``__spec__``
        back, which importing it under the alias replaced."""
        module.__spec__ = self.real_spec


class _AsMatplotlib(importlib.abc.MetaPathFinder):
    """Imports ``matplotlib`` and ``mpl_toolkits`` as pltrs's modules (the
    spec's cases are matplotlib code)."""

    def find_spec(self, name, path, target=None):
        top = name.partition(".")[0]
        if top == "mpl_toolkits":
            return importlib.util.spec_from_loader(name, _Alias(f"pltrs.{name}"))
        if top == "matplotlib":
            real = "pltrs" + name[len("matplotlib"):]
            return importlib.util.spec_from_loader(name, _Alias(real))
        return None


@pytest.fixture(scope="module")
def spec_cases():
    """The spec's figure cases of `_GALLERY`'s modules, by name."""
    finder = _AsMatplotlib()
    paths = [str(_SPEC), str(_SPEC / "figures")]
    sys.meta_path.insert(0, finder)
    sys.path[:0] = paths
    before = set(sys.modules)
    try:
        for module in sorted({module for module, _ in _GALLERY}):
            importlib.import_module(module)
        yield importlib.import_module("spec_support.figcases")._CASES
    finally:
        sys.meta_path.remove(finder)
        del sys.path[:len(paths)]
        for name in set(sys.modules) - before:
            del sys.modules[name]


@pytest.mark.parametrize(("module", "name"), _GALLERY,
                         ids=[name for _, name in _GALLERY])
def test_gallery_examples_draw_as_matplotlib(spec_cases, module, name):
    from PIL import Image
    from pltrs.testing import setup
    setup()
    pltrs.rcParams["text.hinting"] = "none"
    case = spec_cases[name]
    png, _ = case.render()
    got = np.asarray(Image.open(io.BytesIO(png)).convert("RGB"), dtype=float)
    # matplotlib's image on this platform (see `spec_support.golden`).
    golden = sys.modules["spec_support.golden"].golden_path(case.png_golden)
    want = np.asarray(Image.open(golden).convert("RGB"), dtype=float)
    assert got.shape == want.shape
    assert np.sqrt(np.mean((got - want) ** 2)) <= _TOLERANCE


class Dot(Artist):
    """A dot at display (100, 100) that tells whether it holds a point."""

    def draw(self, renderer):
        gc = renderer.new_gc()
        renderer.draw_path(gc, Path.unit_circle(),
                           Affine2D().scale(10).translate(100, 100), (1, 0, 0))
        gc.restore()

    def contains(self, mouseevent):
        hit = (mouseevent.x - 100) ** 2 + (mouseevent.y - 100) ** 2 < 100
        return hit, {"where": "dot"}


def test_artists_of_the_program_are_picked_through_their_contains():
    from pltrs.backend_bases import MouseEvent
    fig = plt.figure()
    dot = Dot()
    dot.set_picker(True)
    fig.add_artist(dot)
    picked = []
    fig.canvas.mpl_connect(
        "pick_event", lambda event: picked.append((event.artist is dot,
                                                   event.where)))
    fig.canvas.draw()
    for x, y in [(100, 100), (300, 300), (105, 95)]:
        MouseEvent("button_press_event", fig.canvas, x, y, 1)._process()
    assert picked == [(True, "dot"), (True, "dot")]


class Badge(Artist):
    """A blue box past the top right corner of the figure, which its class
    measures."""

    def draw(self, renderer):
        gc = renderer.new_gc()
        renderer.draw_path(gc, Path.unit_rectangle(),
                           Affine2D().scale(60, 30).translate(600, 460),
                           (0, 0, 1))
        gc.restore()

    def get_window_extent(self, renderer=None):
        return Bbox.from_bounds(600, 460, 60, 30)


def test_layouts_and_tight_boxes_take_the_extents_classes_measure():
    fig = plt.figure(figsize=(6.4, 4.8), dpi=100)
    fig.add_subplot()
    fig.add_artist(Badge())
    assert fig.get_tightbbox().bounds == pytest.approx(
        (0.482778, 0.291889, 6.117222, 4.608111), abs=1e-6)
    assert _pixels(fig, bbox_inches="tight").shape == (480, 631, 4)
