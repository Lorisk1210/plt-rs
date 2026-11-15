"""Objects of the program's given to plotting methods (a transform, path
effects, a norm, a colormap, a filter) are the objects the artist gives
back, and go with their figure even when they refer back to it; the
keywords every artist takes are taken by every plotting method. The
expected values are matplotlib's."""

import gc
import io
import pickle
import weakref

import numpy as np
import pytest

import pltrs
from pltrs import cm, colors, patheffects, transforms
import pltrs.pyplot as plt


@pytest.fixture(autouse=True)
def _matplotlib_rcparams():
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class Keeping(transforms.Transform):
    """A transform keeping its Axes."""
    input_dims = output_dims = 2

    def __init__(self, ax):
        super().__init__()
        self.ax = ax

    def transform_non_affine(self, values):
        return values


class Effect(patheffects.AbstractPathEffect):
    """A path effect keeping its Axes."""

    def __init__(self, ax):
        super().__init__()
        self.ax = ax

    def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
        renderer.draw_path(gc, tpath, affine, rgbFace)


class Norm(colors.Normalize):
    """A norm keeping its Axes."""

    def __init__(self, ax=None):
        super().__init__(0, 3)
        self.ax = ax


class Map(colors.ListedColormap):
    """A colormap mapping in Python, keeping its Axes."""

    def __init__(self, ax=None):
        super().__init__(["red", "blue", "green"])
        self.ax = ax

    def __call__(self, X, alpha=None, bytes=False):
        return super().__call__(X, alpha, bytes)


x = np.linspace(0, 1, 5)
image = np.arange(16.0).reshape(4, 4)

#: (keyword, the object for an Axes, how the artist gives it back)
GIVEN = {
    "transform": (lambda ax: Keeping(ax) + ax.transData, lambda a: a.get_transform()),
    "path_effects": (lambda ax: [Effect(ax)], lambda a: a.get_path_effects()),
    "norm": (lambda ax: Norm(ax), lambda a: a.norm),
    "cmap": (lambda ax: Map(ax), lambda a: a.get_cmap()),
}

#: (method, keyword): how each method makes its artist with the keyword.
MADE = {
    ("plot", "transform"): lambda ax, kw: ax.plot(x, x, **kw)[0],
    ("plot", "path_effects"): lambda ax, kw: ax.plot(x, x, **kw)[0],
    ("step", "path_effects"): lambda ax, kw: ax.step(x, x, **kw)[0],
    ("errorbar", "transform"): lambda ax, kw: ax.errorbar(x, x, yerr=0.1, **kw).lines[0],
    ("errorbar", "path_effects"): lambda ax, kw: ax.errorbar(x, x, yerr=0.1, **kw).lines[0],
    ("contourf", "transform"): lambda ax, kw: ax.contourf(image, **kw),
    ("scatter", "norm"): lambda ax, kw: ax.scatter(x, x, c=x, **kw),
    ("imshow", "norm"): lambda ax, kw: ax.imshow(image, **kw),
    ("imshow", "cmap"): lambda ax, kw: ax.imshow(image, **kw),
    ("pcolormesh", "norm"): lambda ax, kw: ax.pcolormesh(image, **kw),
    ("hexbin", "norm"): lambda ax, kw: ax.hexbin(x, x, gridsize=3, **kw),
    ("quiver", "norm"): lambda ax, kw: ax.quiver(x, x, x, x, **kw),
    ("quiver", "cmap"): lambda ax, kw: ax.quiver(x, x, x, x, **kw),
}


@pytest.mark.parametrize("method, keyword", MADE)
def test_a_given_object_comes_back_and_goes_with_its_figure(method, keyword):
    make, read = GIVEN[keyword]
    fig, ax = plt.subplots()
    given = make(ax)
    artist = MADE[method, keyword](ax, {keyword: given})
    assert read(artist) is given
    fig.canvas.draw()
    figure = weakref.ref(fig)
    plt.close(fig)
    del fig, ax, given, artist
    gc.collect()
    assert figure() is None


@pytest.mark.parametrize("setting", ["set_norm", "colorizer", "set_cmap"])
def test_a_norm_or_colormap_set_later_goes_with_its_figure(setting):
    fig, ax = plt.subplots()
    made = ax.imshow(np.eye(3))
    if setting == "set_norm":
        made.set_norm(Norm(ax))
    elif setting == "colorizer":
        made.colorizer.norm = Norm(ax)
    else:
        made.set_cmap(Map(ax))
    fig.canvas.draw()
    figure = weakref.ref(fig)
    plt.close(fig)
    del fig, ax, made
    gc.collect()
    assert figure() is None


def test_norms_and_colormaps_held_by_nothing_else_still_draw():
    # A mappable made for a colorbar alone, and an image pickled and read
    # back: their norms and colormaps live as long as what uses them.
    fig, ax = plt.subplots()
    fig.colorbar(cm.ScalarMappable(norm=Norm(), cmap=Map()), ax=ax)
    other, bx = plt.subplots()
    bx.imshow(image, norm=Norm(bx), cmap=Map(bx))
    back = pickle.loads(pickle.dumps(other))
    gc.collect()
    for figure in (fig, other, back):
        figure.savefig(io.BytesIO(), format="png")
    assert type(back.axes[0].images[0].norm).__name__ == "Norm"


def _filter(image, dpi):
    return image, 0, 0


#: The keywords every artist takes, with a value and how it reads back.
ARTIST_KEYWORDS = {
    "agg_filter": (_filter, lambda a: a.get_agg_filter() is _filter),
    "snap": (True, lambda a: a.get_snap()),
    "mouseover": (True, lambda a: a.get_mouseover()),
    "in_layout": (False, lambda a: a.get_in_layout()),
    "gid": ("g", lambda a: a.get_gid()),
    "url": ("u", lambda a: a.get_url()),
    "rasterized": (True, lambda a: a.get_rasterized()),
}

LINE_METHODS = {
    "plot": lambda ax, kw: ax.plot(x, x, **kw)[0],
    "step": lambda ax, kw: ax.step(x, x, **kw)[0],
    "axhline": lambda ax, kw: ax.axhline(0.5, **kw),
    "axline": lambda ax, kw: ax.axline((0, 0), (1, 1), **kw),
    "errorbar": lambda ax, kw: ax.errorbar(x, x, yerr=0.1, **kw).lines[0],
    "semilogy": lambda ax, kw: ax.semilogy(x, x + 1, **kw)[0],
}


@pytest.mark.parametrize("keyword", ARTIST_KEYWORDS)
@pytest.mark.parametrize("method", LINE_METHODS)
def test_line_making_methods_take_every_artists_keywords(method, keyword):
    if (method, keyword) == ("axline", "rasterized"):
        pytest.skip("matplotlib's AxLine fails making itself rasterized")
    value, read = ARTIST_KEYWORDS[keyword]
    fig, ax = plt.subplots()
    line = LINE_METHODS[method](ax, {keyword: value})
    assert read(line) == (True if keyword == "agg_filter" else value)
    fig.canvas.draw()


def test_a_line_keyword_no_line_takes_fails_before_the_line_is_made():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match="Line2D.set.. got an unexpected keyword "
                                             "argument 'nonsense'"):
        ax.plot(x, x, nonsense=1)
    assert ax.get_lines() == []


@pytest.mark.parametrize("make", [
    lambda ax: ax.pcolormesh(image, mouseover=True),
    lambda ax: ax.hist2d(x, x, bins=3, mouseover=True)[3],
])
def test_a_quad_mesh_turns_mouseover_off_after_its_keywords(make):
    fig, ax = plt.subplots()
    assert make(ax).get_mouseover() is False


def test_an_errorbar_data_line_takes_the_clip_path():
    fig, ax = plt.subplots()
    clip = pltrs.patches.Circle((0.5, 0.5), 0.3, transform=ax.transAxes)
    line = ax.errorbar(x, x, yerr=0.1, clip_path=clip).lines[0]
    assert type(line.get_clip_path()).__name__ == "TransformedPatchPath"
