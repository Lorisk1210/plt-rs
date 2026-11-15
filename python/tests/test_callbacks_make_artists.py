"""Code of the program's that pltrs (Rust) calls while it draws or lays
the figure out (a locator's ``tick_values`` or ``__call__``, a
formatter, a transform's ``transform_non_affine``, an artist's ``draw``)
makes an artist and measures it: a tick, a text, a line, a patch, which
the figure leaves loose, as matplotlib does (getdist's locators make a
tick to measure their labels). The expected values are matplotlib's."""

import pytest

import pltrs
from pltrs import ticker, transforms
from pltrs.lines import Line2D
from pltrs.patches import Rectangle
from pltrs.text import Text
import pltrs.pyplot as plt


@pytest.fixture(autouse=True)
def _matplotlib_rcparams():
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _make(axis, kind):
    """An artist of *kind* made for the figure of *axis*, then measured."""
    fig = axis.get_figure(root=True)
    if kind == "tick":
        made = axis._get_tick(major=True)
        made.label1.set_text("0.5")
    elif kind == "text":
        made = Text(0, 0, "made", figure=fig)
    elif kind == "axes-text":
        made = axis.axes.text(0.5, 0.5, "added")
    else:
        made = Line2D([0, 1], [0, 1]) if kind == "line" else Rectangle((0, 0), 1, 1)
        made.set_figure(fig)
        made.set_transform(axis.axes.transData)
    made.get_window_extent(fig.canvas.get_renderer())
    return made


def _locating(axis_of, kind, made):
    class Locating(ticker.MaxNLocator):
        def tick_values(self, vmin, vmax):
            made.append(_make(axis_of(), kind))
            return super().tick_values(vmin, vmax)
    return Locating()


KINDS = ["tick", "text", "axes-text", "line", "patch"]


@pytest.mark.parametrize("kind", KINDS)
@pytest.mark.parametrize("site", ["locator", "formatter", "transform", "draw"])
def test_a_callback_makes_and_measures_an_artist(site, kind):
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    made = []
    if site == "locator":
        ax.xaxis.set_major_locator(_locating(lambda: ax.xaxis, kind, made))
    elif site == "formatter":
        class Formatting(ticker.ScalarFormatter):
            def __call__(self, x, pos=None):
                if not made:
                    made.append(_make(ax.xaxis, kind))
                return super().__call__(x, pos)
        ax.xaxis.set_major_formatter(Formatting())
    elif site == "transform":
        class Shifting(transforms.Transform):
            input_dims = output_dims = 2

            def transform_non_affine(self, values):
                if not made:
                    made.append(_make(ax.xaxis, kind))
                return values
        line, = ax.plot([0, 1], [1, 0])
        line.set_transform(Shifting() + ax.transData)
    else:
        class Drawing(Line2D):
            def draw(self, renderer):
                if not made:
                    made.append(_make(ax.xaxis, kind))
                super().draw(renderer)
        ax.add_line(Drawing([0, 1], [0.5, 0.5]))
    fig.canvas.draw()
    fig.tight_layout()
    fig.canvas.draw()
    labels = [label.get_text() for label in ax.get_xticklabels()][:3]
    if site == "locator":
        # (Every tick location asked for makes another axes text.)
        assert labels == ["\N{MINUS SIGN}0.15", "0.00", "0.15"]
        assert len(ax.texts) == (13 if kind == "axes-text" else 0)
    else:
        assert labels == ["\N{MINUS SIGN}0.2", "0.0", "0.2"]
        assert len(ax.texts) == (1 if kind == "axes-text" else 0)
    expected = {"tick": "XTick", "line": "Line2D", "patch": "Rectangle"}.get(kind, "Text")
    assert type(made[0]).__name__ == expected


def _projected(projection):
    """A figure with axes of *projection* whose locator (on the axis a
    program would set one on) makes and measures a tick."""
    fig = plt.figure()
    ax = fig.add_subplot(projection=projection)
    ax.plot([0, 1], [0, 1], *([[0, 1]] if projection == "3d" else []))
    axis = ax.yaxis if projection == "polar" else ax.xaxis
    made = []
    axis.set_major_locator(_locating(lambda: axis, "tick", made))
    return fig, ax, made


@pytest.mark.parametrize("projection, ticks, labels, tick", [
    ("3d", [-0.15, 0.0, 0.15, 0.3], ["\N{MINUS SIGN}0.2", "0.0", "0.2"], "XTick"),
    ("polar", [0.0, 0.785, 1.571, 2.356], ["0.15", "0.30", "0.45"], "RadialTick"),
])
def test_a_locator_of_projected_axes_makes_a_tick(projection, ticks, labels, tick):
    fig, ax, made = _projected(projection)
    fig.canvas.draw()
    assert [round(float(v), 3) for v in ax.get_xticks()][:4] == ticks
    assert [label.get_text() for label in ax.get_yticklabels()][:3] == labels
    fig.get_tightbbox(fig.canvas.get_renderer())
    fig.tight_layout()
    assert {type(each).__name__ for each in made} == {tick}


@pytest.mark.parametrize("projection", ["3d", "polar"])
def test_a_plain_locator_subclass_ticks_projected_axes(projection):
    # (A subclass changing nothing, as a program's often starts.)
    class Plain(ticker.MaxNLocator):
        def tick_values(self, vmin, vmax):
            return super().tick_values(vmin, vmax)
    fig = plt.figure()
    ax = fig.add_subplot(projection=projection)
    (ax.yaxis if projection == "polar" else ax.xaxis).set_major_locator(Plain())
    fig.canvas.draw()
    fig.get_tightbbox(fig.canvas.get_renderer())


def test_a_tick_of_a_3d_axis_is_made_alone():
    ax = plt.figure().add_subplot(projection="3d")
    tick = ax.xaxis._get_tick(major=True)
    assert type(tick).__name__ == "XTick"
    assert tick.get_loc() == 0
