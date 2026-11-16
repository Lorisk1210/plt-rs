"""
What the fourth round on the generated families ``overrides`` and
``patches`` showed pltrs not asking, or asking otherwise than matplotlib
3.11, each with a small class of the kind libraries write: the renderers
a print cut to its tight box hands the program's code (the whole figure's
first, then the window's), norms deriving from `BoundaryNorm` or `NoNorm`
(ultraplot's ``DiscreteNorm``), the array a colour bar autoscales a norm
of the program's to, the transform a patch class' ``set_transform``
receives, a polar line's radial frame through a pickle, an anchored box
around a box class of the program's (plotnine's), image, patch and
collection classes giving their transform or paths, and what both layout
engines ask of an Axes class and where constrained layout fails; and the
polar branch of matplotlib's ``errorbar``, which an Axes class overriding
``vlines`` runs. Every expected value was recorded from matplotlib.
"""

import io
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import artist, colors, offsetbox, patches, projections


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_a_tight_print_hands_the_whole_figures_renderer_then_the_windows():
    seen = []

    def note(kind, renderer):
        size = np.round(renderer.get_canvas_width_height(), 1).tolist()
        seen.append((kind, type(renderer).__name__, *size))

    class Box(offsetbox.AnchoredOffsetbox):
        def get_offset(self, bbox, renderer):
            note("offset", renderer)
            return super().get_offset(bbox, renderer)

    class Probe(artist.Artist):
        def draw(self, renderer):
            note("draw", renderer)

    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 1])
    ax.add_artist(Box("upper left", child=offsetbox.TextArea("box")))
    fig.add_artist(Probe())
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert set(seen) == {("draw", "RendererAgg", 300, 200), ("offset", "RendererAgg", 300, 200),
                         ("draw", "RendererAgg", 298, 197), ("offset", "RendererAgg", 298, 197)}
    del seen[:]
    fig.savefig(io.BytesIO(), format="svg", bbox_inches="tight")
    assert set(seen) == {
        ("draw", "MixedModeRenderer", 216.0, 144.0), ("offset", "MixedModeRenderer", 216.0, 144.0),
        ("draw", "MixedModeRenderer", 214.6, 142.3), ("offset", "MixedModeRenderer", 214.6, 142.3)}


def test_a_tight_vector_print_makes_images_at_the_print_resolution():
    shapes = []

    class Cmap(colors.ListedColormap):
        def __call__(self, X, *args, **kwargs):
            shapes.append(np.shape(X))
            return super().__call__(X, *args, **kwargs)

    fig, ax = plt.subplots(figsize=(3, 2))
    ax.imshow(np.arange(12.).reshape(3, 4), cmap=Cmap(plt.cm.viridis.colors), extent=(0, 1, 0, 1))
    fig.savefig(io.BytesIO(), format="svg", bbox_inches="tight")
    assert shapes == [(154, 154), (154, 154)]


@pytest.mark.parametrize("base, kwargs, labels, limits", [
    (colors.BoundaryNorm, dict(boundaries=[-2, -1, 0, 1, 2, 4], ncolors=256),
     ["−2", "−1", "0", "1"], [-2.0, 4.0]),
    (colors.NoNorm, {}, ["0", "26", "52", "78"], [-0.5, 255.5]),
])
def test_a_norm_class_deriving_from_a_discrete_norm_maps_as_its_base(base, kwargs, labels, limits):
    z = np.outer(np.linspace(-2, 3, 4), np.linspace(0.2, 1.5, 3))
    drawn = []
    for cls in (base, type("Sub", (base,), {})):
        fig, ax = plt.subplots(figsize=(2, 2))
        image = ax.imshow(z, norm=cls(**kwargs))
        colorbar = fig.colorbar(image)
        fig.canvas.draw()
        assert [t.get_text() for t in colorbar.ax.get_yticklabels()][:4] == labels
        assert np.round(colorbar.ax.get_ylim(), 3).tolist() == limits
        drawn.append(np.asarray(fig.canvas.buffer_rgba()).copy())
    assert np.array_equal(*drawn)


def test_a_colorbar_autoscales_a_norm_of_the_program_to_the_array_as_held():
    shapes = []

    class Norm(colors.Normalize):
        def autoscale_None(self, A):
            shapes.append(np.shape(A))
            return super().autoscale_None(A)

    fig, ax = plt.subplots()
    ax.imshow(np.outer(np.arange(4.0), np.arange(3.0)), norm=Norm())
    del shapes[:]
    fig.colorbar(ax.images[0])
    assert shapes == [(4, 3), (4, 3), (256, 1)]


def test_a_patch_class_set_transform_receives_the_data_transform():
    received = []

    class Rect(patches.Rectangle):
        def set_transform(self, t):
            received.append(t)
            return super().set_transform(t)

    fig, ax = plt.subplots(figsize=(3, 2))
    rect = Rect((0.2, 0.2), 0.5, 0.3)
    ax.add_patch(rect)
    fig.canvas.draw()
    assert received == [ax.transData]
    assert np.round(rect.get_window_extent().bounds, 2).tolist() == [84.0, 52.8, 116.25, 46.2]


def test_a_polar_line_keeps_its_radial_frame_through_a_pickle():
    x = np.linspace(0, 2 * np.pi, 7)
    fig = plt.figure(figsize=(3, 2))
    ax = fig.add_subplot(projection="polar")
    line, = ax.plot(x, np.sin(x))
    fig.canvas.draw()
    ax.set_ylim(-1.06, 3.19)
    unpickled = pickle.loads(pickle.dumps(fig))
    unpickled.canvas.draw()
    fig.canvas.draw()
    kept = unpickled.axes[0].lines[0]
    assert np.round(kept.get_window_extent().bounds, 2).tolist() == [134.55, 95.96, 38.41, 33.26]
    assert np.round(line.get_window_extent().bounds, 2).tolist() == [134.55, 95.96, 38.41, 33.26]


def test_a_polar_errorbar_of_a_class_drawing_its_own_lines_rotates_its_caps():
    class Polar(projections.PolarAxes):
        def vlines(self, *args, **kwargs):
            return super().vlines(*args, **kwargs)

    fig = plt.figure(figsize=(3, 3))
    ax = fig.add_subplot(axes_class=Polar)
    ax.errorbar([0.5, 1.5], [1.0, 2.0], xerr=0.2, yerr=0.3, capsize=4)
    fig.canvas.draw()
    rotated = [line for line in ax.lines if len(line.get_xdata()) == 1]
    assert len(ax.lines) == 9 and len(rotated) == 8
    assert [np.round(line._marker.get_user_transform().get_matrix()[:2, :2], 3).tolist()
            for line in rotated][:3] == [[[0.955, -0.296], [0.296, 0.955]],
                                         [[0.267, -0.964], [0.964, 0.267]],
                                         [[0.765, -0.644], [0.644, 0.765]]]


def test_an_anchored_box_contains_what_its_program_child_draws():
    from pltrs.backend_bases import MouseEvent
    calls = []

    class Aux(offsetbox.AuxTransformBox):
        def get_bbox(self, renderer):
            calls.append(1)
            return super().get_bbox(renderer)

    fig, ax = plt.subplots(figsize=(3, 2))
    box = Aux(ax.transData)
    box.add_artist(patches.Rectangle((0, 0), 1, 0.5))
    anchored = offsetbox.AnchoredOffsetbox("upper right", child=box)
    ax.add_artist(anchored)
    fig.canvas.draw()
    del calls[:]
    assert anchored.contains(MouseEvent("button_press_event", fig.canvas, 200, 120))[0]
    extent = anchored.get_window_extent(fig.canvas.get_renderer())
    assert np.round(extent.bounds, 2).tolist() == [19.44, 80.94, 243.61, 88.11]
    assert len(calls) == 1


def test_an_image_class_composing_its_transform_composes_it_once():
    from pltrs import image, transforms

    class Shifted(image.AxesImage):
        def get_transform(self):
            return transforms.Affine2D().translate(10, 5) + super().get_transform()

    fig, ax = plt.subplots(figsize=(3, 2))
    shifted = Shifted(ax)
    shifted.set_data(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))
    shifted.set_extent(shifted.get_extent())
    ax.add_image(shifted)
    ax.set_xlim(-0.5, 5.5)
    ax.set_ylim(4.5, -0.5)
    fig.canvas.draw()
    assert np.round(shifted.get_window_extent().bounds, 2).tolist() == [425.0, -132.0, 232.5, 154.0]
    assert np.round(fig.get_tightbbox().bounds, 3).tolist() == [0.188, -1.32, 6.387, 3.08]


def _drawn(fig):
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def test_a_patch_class_giving_its_transform_draws_through_it_whole():
    class Circle(patches.Circle):
        def get_transform(self):
            return super().get_transform()

    drawn = []
    for cls in (patches.Circle, Circle):
        fig, ax = plt.subplots(figsize=(3, 2))
        circle = cls((0.5, 0.5), 0.2)
        ax.add_patch(circle)
        drawn.append(_drawn(fig))
        assert np.round(circle.get_window_extent().bounds, 2).tolist() == [107.25, 68.2, 93.0, 61.6]
    assert np.array_equal(*drawn)


def test_a_collection_class_giving_its_paths_is_drawn_with_them():
    from pltrs import collections, path

    class Round(collections.Collection):
        def get_paths(self):
            super().get_paths()
            return [path.Path.unit_circle()]

    drawn = []
    for cls, paths in ((collections.PathCollection, [path.Path.unit_circle()]),
                       (Round, [path.Path.unit_rectangle()])):
        fig, ax = plt.subplots(figsize=(3, 2))
        made = cls(paths, facecolors="C0") if cls is collections.PathCollection else cls(facecolors="C0")
        made.set_paths(paths)
        ax.add_collection(made)
        drawn.append(_drawn(fig))
    assert np.array_equal(*drawn)


@pytest.mark.parametrize("engine", ["constrained", "tight"])
def test_both_layout_engines_measure_a_class_as_matplotlibs_passes_do(engine):
    from pltrs import axes
    calls = []

    class Measured(axes.Axes):
        def get_tightbbox(self, *args, **kwargs):
            calls.append(("tight", kwargs.get("for_layout_only")))
            return super().get_tightbbox(*args, **kwargs)

        def _update_title_position(self, renderer):
            calls.append("title")
            return super()._update_title_position(renderer)

    fig = plt.figure(figsize=(3, 2))
    fig.add_subplot(axes_class=Measured).plot([0, 1])
    fig.canvas.draw()
    fig.set_layout_engine(engine)
    del calls[:]
    fig.canvas.draw()
    assert calls == [("tight", True), "title", ("tight", True), "title", "title"]


def test_constrained_layout_fails_on_a_grid_whose_first_ratio_is_zero():
    # matplotlib's LayoutGrid divides by the first ratio: a colour bar made
    # before the engine was set took its cell in a grid of heights 0, 1, 0.
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.imshow(np.arange(12.).reshape(3, 4))
    fig.colorbar(ax.images[0])
    fig.canvas.draw()
    fig.set_layout_engine("constrained")
    with pytest.raises(ZeroDivisionError, match="float division by zero"):
        fig.canvas.draw()
    with pytest.raises(ZeroDivisionError, match="float division by zero"):
        fig.savefig(io.BytesIO(), format="svg", bbox_inches="tight")
