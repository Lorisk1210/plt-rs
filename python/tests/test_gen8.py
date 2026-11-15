"""
What the third round on the generated families ``overrides`` and
``patches`` showed pltrs not asking, or asking otherwise than matplotlib
3.11, each with a small class of the kind libraries write: a spine of the
program's put in the place of an Axes' spine (the ticks keep the spine
they were made on), the getters of a `Text` class of the program's that
matplotlib's draw and measures read (ultraplot's ``CurvedText``), the
plotting methods matplotlib makes what it plots with through other
plotting methods (``errorbar`` through ``vlines``, ultraplot's sticky
bars) and the setters more constructors call; and two of matplotlib's
tests that fail one after the other in one process (an inset made once a
program set a stale callback). Every expected value was recorded from
matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.axes as maxes
import pltrs.pyplot as plt
from pltrs import image, patches, spines, text as mtext


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _tight_height(fig):
    fig.canvas.draw()
    return round(float(fig.get_tightbbox(fig.canvas.get_renderer()).height), 2)


def test_ticks_keep_the_spine_they_were_made_on():
    class Outward(spines.Spine):
        def _ensure_position_is_set(self):
            if self._position is None:
                self._position = ("outward", 10.0)
                self.set_position(self._position)

    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 1], [0, 1])
    plain = _tight_height(fig)
    ax.spines["bottom"] = Outward.linear_spine(ax, "bottom")
    # The spine moved out; the ticks, made on the spine it replaced, stay.
    assert _tight_height(fig) == plain
    assert ax.spines["bottom"].get_position() == ("outward", 10.0)


def test_a_text_is_measured_and_drawn_as_its_class_answers():
    class Answered(mtext.Text):
        def get_text(self):
            return ""

    fig = plt.figure(figsize=(3, 3))
    ax = fig.add_subplot(projection="polar")
    text = Answered(0.39, 1.7, " ", transform=ax.transData)
    ax.add_artist(text)
    text.set_clip_on(False)
    fig.canvas.draw()
    bounds = ax.get_tightbbox(fig.canvas.get_renderer()).bounds
    assert np.round(bounds, 1).tolist() == [1.8, 6.6, 294.9, 283.8]


def test_a_text_is_wrapped_as_its_class_answers():
    class Wrapped(mtext.Text):
        def get_wrap(self):
            return True

    fig, ax = plt.subplots(figsize=(3, 2))
    text = Wrapped(0.5, 0.5, "a fairly long text to wrap around", transform=ax.transAxes)
    ax.add_artist(text)
    fig.canvas.draw()
    extent = text.get_window_extent(fig.canvas.get_renderer())
    assert np.round(extent.bounds, 1).tolist() == [153.8, 94.3, 132.8, 34.2]


class _StickyBars(maxes.Axes):
    """ultraplot's way: sticky edges at the ends of the bars it draws."""

    def vlines(self, *args, **kwargs):
        lines = super().vlines(*args, **kwargs)
        lines.sticky_edges.y.extend([0.75, 2.25])
        return lines


def test_errorbar_draws_its_bars_through_the_class_vlines():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=_StickyBars)
    container = ax.errorbar([0.0], [1.5], yerr=0.75, fmt="none")
    assert list(container.lines[2][0].sticky_edges.y) == [0.75, 2.25]
    assert ax.get_ylim() == (0.75, 2.25)


def test_composed_plotting_methods_go_through_the_class_methods():
    calls = []

    class Recording(maxes.Axes):
        def plot(self, *args, **kwargs):
            calls.append("plot")
            return super().plot(*args, **kwargs)

        def fill(self, *args, **kwargs):
            calls.append("fill")
            return super().fill(*args, **kwargs)

        def fill_between(self, *args, **kwargs):
            calls.append("fill_between")
            return super().fill_between(*args, **kwargs)

    fig = plt.figure()
    ax = fig.add_subplot(axes_class=Recording)
    ax.bxp([dict(med=1, q1=0.5, q3=1.5, whislo=0, whishi=2, fliers=[3], label="a")])
    assert calls.count("plot") == 7
    assert ax.get_xlim() == (0.5, 1.5)
    del calls[:]
    _, _, steps = ax.hist([1.0, 2.0, 2.5, 3.0], histtype="step")
    assert calls == ["fill"]
    assert not steps[0].get_fill()
    del calls[:]
    ax.stackplot([0, 1, 2], [1, 2, 1], [1, 1, 1])
    assert calls == ["fill_between", "fill_between"]


def test_more_constructors_call_the_class_setters():
    calls = []

    class Shade(patches.Shadow):
        def set_alpha(self, alpha):
            calls.append(("alpha", alpha))
            super().set_alpha(alpha)

    class Placed(image.FigureImage):
        def set_figure(self, fig):
            calls.append("figure")
            super().set_figure(fig)

    fig, ax = plt.subplots()
    shade = Shade(patches.Circle((0.5, 0.5), 0.2), 0.02, -0.02)
    assert calls == [("alpha", 0.5)]
    assert shade.get_alpha() == 0.5
    del calls[:]
    Placed(fig)
    assert calls == ["figure"]


def test_a_figure_printed_cut_to_its_tight_box_has_the_box_size_meanwhile():
    import io

    from pltrs.artist import Artist
    seen = []

    class Probe(Artist):
        """mpl_scatter_density's way: bins sized by the figure's size."""

        def draw(self, renderer):
            figure = self.get_figure(root=True)
            seen.append((np.round(figure.get_size_inches(), 3).tolist(),
                         np.round(figure.bbox_inches.bounds, 3).tolist()))

    fig, ax = plt.subplots(figsize=(4, 3))
    ax.plot([0, 1])
    fig.add_artist(Probe())
    fig.savefig(io.BytesIO(), bbox_inches="tight")
    assert seen[-1] == ([3.617, 2.746], [0.0, 0.0, 3.617, 2.746])
    assert fig.get_size_inches().tolist() == [4.0, 3.0]


def test_an_inset_is_located_after_a_program_set_a_stale_callback():
    # matplotlib's test_backend_qt.py::test_canvas_reinit, then its
    # test_bbox_tight.py::test_bbox_inches_inset_rasterized: once a stale
    # callback of the program's ran, the locator, stale while it is made,
    # is asked for its Axes before its constructor ends.
    import io

    from pltrs.mpl_toolkits.axes_grid1.inset_locator import inset_axes
    seen = []
    fig, ax = plt.subplots()
    fig.stale_callback = lambda figure, stale: seen.append(stale)
    fig.stale = True
    assert seen == [True]
    # A (sub)figure made alone has no callback, also while it is made.
    from pltrs.figure import FigureBase
    assert FigureBase(alpha=0.5).stale_callback is None

    fig, ax = plt.subplots()
    im = ax.imshow(np.arange(100).reshape(10, 10))
    inset = inset_axes(ax, width="10%", height="30%", loc="upper left",
                       bbox_to_anchor=(0.045, 0.0, 1, 1), bbox_transform=ax.transAxes)
    fig.colorbar(im, cax=inset, orientation="horizontal")
    fig.savefig(io.BytesIO(), format="pdf", bbox_inches="tight")
    assert inset.get_position().bounds == pytest.approx(
        (0.2606, 0.6345, 0.0578, 0.231), abs=1e-4)
