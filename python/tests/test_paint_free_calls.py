"""
The program's code a draw runs to map values (a scale's functions, a norm,
a transform, the text measurer) finds the figure free: it changes it at
once, may make changes returning what they changed, reads them back, and
the rest of the draw sees them. Setting tick labels places the ticks of
their level only, and a failed `set_alpha` leaves the colours the getters
report as they were. The expected values are real matplotlib 3.11's for
the same programs.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors, text as mtext, transforms


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _pixels(fig):
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def _scaled(forward):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([1, 2, 3], [1, 4, 9])
    ax.set_yscale("function", functions=(forward, lambda x: x**2))
    return fig, ax


def _retitling(ax, state, title):
    """A scale's forward that sets the title of `ax` once `state` arms it."""
    def forward(x):
        if state.get("armed"):
            state["armed"] = False
            made = ax.set_title(title)
            state["seen"] = (type(made).__name__, made.get_text(), ax.get_title())
        return np.sqrt(np.abs(x))
    return forward


@pytest.mark.parametrize("draws_before", [0, 1])
def test_a_scale_function_retitles_the_axes_during_the_draw(draws_before):
    state = {}
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.plot([1, 2, 3], [1, 4, 9])
    ax.set_yscale("function",
                  functions=(_retitling(ax, state, "from forward"),
                             lambda x: x**2))
    for _ in range(draws_before):
        fig.canvas.draw()
    state["armed"] = True
    drawn = _pixels(fig)
    assert state["seen"] == ("Text", "from forward", "from forward")
    assert (fig.stale, ax.get_title()) == (False, "from forward")
    made, made_ax = _scaled(lambda x: np.sqrt(np.abs(x)))
    made_ax.set_title("from forward")
    assert np.array_equal(drawn, _pixels(made))


class _LabellingNorm(colors.Normalize):
    """A norm of the program's labelling the x axis on its first call."""

    def __init__(self, ax, seen):
        super().__init__(0, 10)
        self.ax, self.seen = ax, seen

    def __call__(self, value, clip=None):
        if not self.seen:
            label = self.ax.set_xlabel("from the norm")
            self.seen.append((type(label).__name__, self.ax.get_xlabel()))
        return super().__call__(value, clip)


def _image(norm, xlabel=None):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.imshow(np.arange(12.).reshape(3, 4), norm=norm(ax))
    if xlabel is not None:
        ax.set_xlabel(xlabel)
    return fig, ax


def test_a_norm_labels_the_axes_while_its_image_is_drawn():
    seen = []
    fig, ax = _image(lambda ax: _LabellingNorm(ax, seen))
    drawn = _pixels(fig)
    assert seen == [("Text", "from the norm")]
    assert (ax.get_xlabel(), fig.stale) == ("from the norm", False)
    made, _ = _image(lambda ax: colors.Normalize(0, 10), "from the norm")
    assert np.array_equal(drawn, _pixels(made))


class _Halving(transforms.Transform):
    """A transform of the program's labelling the y axis when it maps."""

    input_dims = output_dims = 2
    is_separable = False
    has_inverse = False

    def __init__(self, ax, seen):
        super().__init__()
        self.ax, self.seen = ax, seen

    def transform_non_affine(self, values):
        if not self.seen:
            label = self.ax.set_ylabel("from the transform")
            self.seen.append((type(label).__name__, self.ax.get_ylabel()))
        return np.asarray(values) * 0.5


def test_a_transform_labels_the_axes_while_it_places_a_text():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    seen = []
    ax.text(0.2, 0.4, "moved", transform=_Halving(ax, seen) + ax.transAxes)
    drawn = _pixels(fig)
    assert seen == [("Text", "from the transform")]
    assert ax.get_ylabel() == "from the transform"
    made, made_ax = plt.subplots(figsize=(3, 2), dpi=50)
    made_ax.text(0.1, 0.2, "moved", transform=made_ax.transAxes)
    made_ax.set_ylabel("from the transform")
    assert np.array_equal(drawn, _pixels(made))


def test_a_colorbar_norm_retitles_the_colorbar_during_the_draw():
    state = {"armed": False}
    fig, ax = plt.subplots()

    def forward(x):
        if state["armed"]:
            state["armed"] = False
            title = colorbar.ax.set_title("cb")
            state["seen"] = (type(title).__name__, colorbar.ax.get_title())
        return np.sqrt(np.abs(x))

    norm = colors.FuncNorm((forward, lambda x: x**2), vmin=0, vmax=20)
    colorbar = fig.colorbar(ax.pcolormesh(np.arange(20.).reshape(4, 5), norm=norm))
    fig.canvas.draw()
    for _ in range(2):
        state["armed"] = True
        fig.canvas.draw()
        assert state.pop("seen") == ("Text", "cb")
        assert (colorbar.ax.get_title(), fig.stale) == ("cb", False)


class _FadingNorm(colors.Normalize):
    """A norm of the program's fading the image it maps, once."""

    def __init__(self):
        super().__init__()
        self.image = None

    def __call__(self, value, clip=None):
        if self.image is not None:
            image, self.image = self.image, None
            image.set_alpha(0.5)
        return super().__call__(value, clip)


def test_a_norm_changes_the_image_it_maps():
    fig, ax = plt.subplots()
    norm = _FadingNorm()
    image = ax.imshow(np.arange(12.).reshape(3, 4) + 3, norm=norm)
    norm.image = image
    fig.canvas.draw()
    assert (norm.vmin, norm.vmax, image.get_clim()) == (3.0, 14.0, (3.0, 14.0))
    # Changed while it was drawn, it ends the draw fresh.
    assert (image.get_alpha(), image.stale) == (0.5, False)


def _measured(ylabel=None):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
    ax.text(0.5, 0.5, "measured")
    if ylabel is not None:
        ax.set_ylabel(ylabel)
    return fig, ax


def test_the_text_measurer_labels_the_axes_while_a_text_is_drawn(monkeypatch):
    fig, ax = _measured()
    seen = []
    original = mtext._get_text_metrics_with_cache

    def measuring(renderer, text, fontprop, ismath, dpi):
        if text == "measured" and not seen:
            label = ax.set_ylabel("from the measurer")
            seen.append((type(label).__name__, ax.get_ylabel()))
        return original(renderer, text, fontprop, ismath, dpi)

    monkeypatch.setattr(mtext, "_get_text_metrics_with_cache", measuring)
    drawn = _pixels(fig)
    monkeypatch.undo()
    assert seen == [("Text", "from the measurer")]
    assert (ax.get_ylabel(), fig.stale) == ("from the measurer", False)
    made, _ = _measured("from the measurer")
    assert np.array_equal(drawn, _pixels(made))


def _rtol_warnings(call):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        result = call()
    return sum("rtol" in str(w.message) for w in caught), len(result)


@pytest.mark.skipif(tuple(map(int, np.__version__.split(".")[:2])) < (2, 3),
                    reason="numpy warns of a NaN rtol from 2.3 on, as matplotlib meets it")
@pytest.mark.parametrize("secondary, expected", [
    (False, [(0, 8), (1, 8), (1, 8)]),
    (True, [(0, 3), (1, 3), (1, 13)]),
])
def test_set_ticklabels_places_the_ticks_of_its_level_only(secondary, expected):
    # Logit limits outside the scale's domain: matplotlib's tolerance for
    # minor ticks on major ones is NaN, which numpy warns about wherever
    # the minor ticks are placed.
    fig, ax = plt.subplots()
    target = (ax.secondary_xaxis("top", functions=(lambda v: 2 * v,
                                                   lambda v: v / 2))
              if secondary else ax)
    target.set_ylim((-1, 1))
    target.set_yscale("logit")
    assert [
        _rtol_warnings(lambda: target.set_yticklabels([])),
        _rtol_warnings(target.get_yticklabels),
        _rtol_warnings(lambda: target.yaxis.set_ticklabels([], minor=True)),
    ] == expected


def _rounded(rgba):
    return np.round(rgba, 3).tolist()


def test_a_failed_set_alpha_leaves_the_reported_colours():
    fig, ax = plt.subplots()
    dots = ax.scatter([1, 2, 3], [1, 2, 3], c=["r", "g", "b"],
                      edgecolors=["k", "y", "c"])
    with pytest.raises(ValueError):
        dots.set_alpha([0.5, 0.25])
    assert dots.get_alpha().tolist() == [0.5, 0.25]
    faces = [[1.0, 0.0, 0.0, 1.0], [0.0, 0.5, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]]
    assert _rounded(dots.get_facecolor()) == faces
    assert _rounded(dots.get_edgecolor()) == [
        [0.0, 0.0, 0.0, 1.0], [0.75, 0.75, 0.0, 1.0], [0.0, 0.75, 0.75, 1.0]]
    with pytest.raises(ValueError):
        fig.canvas.draw()
    assert _rounded(dots.get_facecolor()) == faces
    # A side set again takes the alphas.
    dots.set_edgecolor("k")
    assert _rounded(dots.get_facecolor()) == faces
    assert _rounded(dots.get_edgecolor()) == [
        [0.0, 0.0, 0.0, 0.5], [0.0, 0.0, 0.0, 0.25]]
    dots.set_facecolor(["r", "g"])
    assert _rounded(dots.get_facecolor()) == [
        [1.0, 0.0, 0.0, 0.5], [0.0, 0.5, 0.0, 0.25]]
    # matplotlib compares a single alpha with the array it had.
    with pytest.raises(ValueError, match="truth value of an array"):
        dots.set_alpha(0.5)
