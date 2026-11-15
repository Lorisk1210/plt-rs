"""Figures and artists go once the program forgets them, as in matplotlib
(the causes the leak harness, ``python -m leaks``, found): a removed
artist, a cleared or replaced one, a class of the program's, and the
objects of the program's that pltrs (Rust) calls; a figure kept for a
long time keeps no more than it shows. Each test also holds under
matplotlib (the checks on pltrs (Rust)'s counts apart)."""

import gc
import io
import pickle
import weakref

import numpy as np
import pytest

import pltrs
from pltrs import patheffects, scale, ticker, transforms
import pltrs.pyplot as plt


@pytest.fixture(autouse=True)
def _matplotlib_rcparams():
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _native_counts():
    """pltrs (Rust)'s counts of what it keeps (none under matplotlib)."""
    diagnostics = getattr(getattr(pltrs, "_native", None), "diagnostics", None)
    gc.collect()
    return diagnostics.live_counts() if diagnostics is not None else None


def _gone(reference):
    gc.collect()
    return reference() is None


def _dropped(make):
    """A weak reference to the figure *make* makes, once it was drawn,
    saved, closed and dropped."""
    fig = make()
    fig.savefig(io.BytesIO(), format="png")
    plt.close(fig)
    return weakref.ref(fig)


def test_a_text_whose_font_properties_were_read_keeps_no_figure():
    # ECO6's leak, now a scene of the harness too.
    def make():
        fig, ax = plt.subplots()
        ax.text(0.5, 0.5, "text").get_fontproperties()
        return fig
    assert _gone(_dropped(make))


def test_an_image_an_axes_made_and_removed_goes():
    # ECO7's leak: `Axes.imshow` made its image the current one (pyplot's
    # `imshow` does, in matplotlib too, which then keeps it).
    fig, ax = plt.subplots()
    image = ax.imshow(np.eye(3))
    image.remove()
    reference = weakref.ref(image)
    del image
    fig.canvas.draw()
    assert _gone(reference)


def test_removed_and_cleared_artists_go_while_their_figure_lives():
    fig, ax = plt.subplots()
    fig.canvas.draw()
    # (The counts are the process's: those of the figures other tests left.)
    before = _native_counts()
    references, kept = [], []
    for i in range(300):
        line, = ax.plot([0, 1], [i, 0])
        legend = ax.legend([f"{i}"])
        fig.canvas.draw()
        if i < 60:
            references += [weakref.ref(line), weakref.ref(legend)]
        if i % 2:
            line.remove()
        else:
            ax.clear()
        del line, legend
        counts = _native_counts() if i % 20 == 19 else None
        if counts is not None:
            kept.append(counts["removed_artists"] - before["removed_artists"])
    gc.collect()
    assert sum(reference() is not None for reference in references) == 0
    # The removed artists no handle reaches are let go at the next draw,
    # the objects of removed artists swept from their figure now and then:
    # bounded, rather than six more for every iteration.
    assert not kept or max(kept) < 400


def test_a_cleared_figure_keeps_none_of_its_former_axes():
    fig = plt.figure()
    references = []
    for i in range(50):
        fig.clf()
        ax = fig.add_subplot()
        ax.plot([0, 1], [1, i])
        fig.canvas.draw()
        references.append(weakref.ref(ax))
    del ax
    gc.collect()
    assert sum(reference() is not None for reference in references) <= 1


def test_children_of_a_removed_axes_stay_its_own_while_it_is_held():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])
    line.mark = "kept"
    ax.remove()
    del line
    for _ in range(80):
        fig.add_subplot().remove()
    gc.collect()
    assert ax.lines[0].mark == "kept"


def test_replaced_colorbars_go():
    fig, ax = plt.subplots()
    image = ax.imshow(np.eye(3))
    references = []
    for _ in range(40):
        colorbar = fig.colorbar(image)
        fig.canvas.draw()
        references.append(weakref.ref(colorbar))
        colorbar.remove()
        del colorbar
    gc.collect()
    assert sum(reference() is not None for reference in references[:20]) == 0


def test_classes_of_the_program_made_per_figure_go():
    def make():
        fig, ax = plt.subplots()

        class Drawn(type(ax.plot([0, 1])[0])):
            def draw(self, renderer):
                # Reaches the figure, as a class defined in a function
                # that makes it does.
                assert fig is not None
                super().draw(renderer)

        ax.add_line(Drawn([0, 1], [1, 0]))
        return fig
    assert _gone(_dropped(make))


class _KeepingScale(scale.LinearScale):
    """A scale keeping its axis, as matplotlib's scales were made to."""

    name = "robust2-keeping"

    def __init__(self, axis):
        super().__init__(axis)
        self.kept = axis


scale.register_scale(_KeepingScale)


class _Keeping(transforms.Transform):
    """A transform keeping its Axes, as projections' transforms do."""

    input_dims = output_dims = 2

    def __init__(self, ax):
        super().__init__()
        self.ax = ax

    def transform_non_affine(self, values):
        return values


class _Effect(patheffects.AbstractPathEffect):
    """A path effect keeping its Axes."""

    def __init__(self, ax):
        super().__init__()
        self.ax = ax

    def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
        renderer.draw_path(gc, tpath, affine, rgbFace)


def test_scales_transforms_and_effects_of_the_program_keeping_their_figure_go():
    def make():
        fig, ax = plt.subplots()
        ax.set_xscale("robust2-keeping")
        line, = ax.plot([1, 2, 3], [1, 2, 1])
        line.set_transform(_Keeping(ax) + ax.transData)
        line.set_path_effects([_Effect(ax)])
        ax.xaxis.set_major_formatter(ticker.FuncFormatter(lambda v, p: f"{v}{len(fig.axes)}"))
        line.set_agg_filter(lambda image, dpi: (image, 0, 0) if fig else None)
        return fig
    assert _gone(_dropped(make))


def test_held_objects_of_the_program_still_draw_and_pickle():
    fig, ax = plt.subplots()
    ax.set_xscale("robust2-keeping")
    line, = ax.plot([1, 2, 3], [1, 2, 1])
    line.set_transform(_Keeping(ax) + ax.transData)
    line.set_path_effects([_Effect(ax)])
    fig.canvas.draw()
    restored = pickle.loads(pickle.dumps(fig))
    restored.canvas.draw()
    assert restored.axes[0].get_xscale() == "robust2-keeping"
    assert isinstance(restored.axes[0].lines[0].get_path_effects()[0], _Effect)


# What the fuzzer's programs on the new paths met (``fuzz.runner.FAILED_SEEDS``).

def test_a_locator_changing_the_other_axis_while_widening_limits():
    # Seed 1019: the figure was refused to the locator during the draw.
    from pltrs import ticker
    fig, ax = plt.subplots(figsize=(3, 2))
    entered = []

    class Widening(ticker.MaxNLocator):
        def nonsingular(self, *args, **kwargs):
            if not entered:
                entered.append(1)
                try:
                    ax.set_ylim(-1)
                finally:
                    entered.pop()
            return super().nonsingular(*args, **kwargs)

    x = np.linspace(0, 2 * np.pi, 7)
    ax.plot(x, np.sin(x))
    ax.xaxis.set_major_locator(Widening(4))
    fig.canvas.draw()
    assert ax.get_xlim() == pytest.approx((-0.3141592653589793, 6.5973445725385655))
    assert ax.get_ylim() == pytest.approx((-1.0, 1.0))


def test_a_locator_measuring_the_figure_while_limits_are_set():
    # Seed 100015: refused while `set_xlim` validated the limits.
    from pltrs import dates
    fig, ax = plt.subplots(figsize=(3, 2))
    entered = []

    class Measuring(dates.DayLocator):
        def nonsingular(self, *args, **kwargs):
            if not entered:
                entered.append(1)
                try:
                    fig.get_tightbbox()
                finally:
                    entered.pop()
            return super().nonsingular(*args, **kwargs)

    days = np.arange("2020-01-01", "2020-01-08", dtype="datetime64[D]")
    ax.plot(days, [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
    ax.xaxis.set_major_locator(Measuring())
    ax.set_xlim(days[1], days[5])
    fig.canvas.draw()
    assert ax.get_xlim() == pytest.approx((18263.0, 18267.0))
    assert ax.get_ylim() == pytest.approx((0.6, 9.4))


def test_a_norm_drawing_its_figure_while_its_colorbar_is_ticked():
    # Seed 101239: a Rust panic (the colorbar's scale borrowed its norm).
    from pltrs import colors
    fig, ax = plt.subplots(figsize=(3, 2))
    entered = []

    class Drawing(colors.Normalize):
        def __call__(self, *args, **kwargs):
            if not entered:
                entered.append(1)
                try:
                    fig.canvas.draw()
                finally:
                    entered.pop()
            return super().__call__(*args, **kwargs)

    image = ax.imshow(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)), norm=Drawing())
    colorbar = fig.colorbar(image)
    fig.canvas.draw()
    assert ax.get_xlim() == pytest.approx((-0.5, 5.5))
    assert colorbar.ax.get_ylim() == pytest.approx((-2.0, 2.0))
