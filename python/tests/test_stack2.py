"""Code of the program's that native code calls back enters its figure
once, as under matplotlib: it reads it, changes it, measures it or draws
it, and gets matplotlib's answers (the census of `spec/diff`'s ``reentry``
programs; the values recorded under matplotlib 3.11)."""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import transforms, units

@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


#: What a callback does in its figure once, and what it gives back.
OPERATIONS = {
    "get_xlim": lambda fig, ax: ax.get_xlim(),
    "set_xlim": lambda fig, ax: ax.set_xlim(0, 10),
    "set_title": lambda fig, ax: ax.set_title("reentered").get_text(),
    "add_patch": lambda fig, ax: type(ax.add_patch(pltrs.patches.Rectangle((0, 0), 1, 1))).__name__,
    "tightbbox": lambda fig, ax: tuple(np.round(fig.get_tightbbox().bounds, 4)),
    "axes_tightbbox": lambda fig, ax: tuple(np.round(
        ax.get_tightbbox(fig.canvas.get_renderer()).bounds, 2)),
    "savefig": lambda fig, ax: fig.savefig(io.BytesIO()),
}


def _shifting(fig, ax, operation, kept):
    """A transform of the program's whose first mapping runs *operation*
    in the figure, keeping what it gave in *kept*."""

    class Shifting(transforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            # Entered once: a draw the operation makes maps through the
            # transform again.
            if "entered" not in kept:
                kept["entered"] = True
                kept["value"] = OPERATIONS[operation](fig, ax)
            return values

    return Shifting()


#: matplotlib 3.11's values: what the operation gave inside a transform
#: mapping a line's data limits as `plot` adds the line, then the limits
#: and the title of the Axes once it is drawn.
WHILE_PLOTTING = {
    "get_xlim": ((-0.1, 2.1), (-0.1, 2.1), ""),
    "set_xlim": ((0.0, 10.0), (0.0, 10.0), ""),
    "set_title": ("reentered", (-0.1, 2.1), "reentered"),
    "add_patch": ("Rectangle", (-0.1, 2.1), ""),
    "tightbbox": ((0.3928, 0.2919, 5.3672, 3.9321), (-0.1, 2.1), ""),
    "axes_tightbbox": ((39.28, 29.19, 536.72, 393.21), (-0.1, 2.1), ""),
    "savefig": (None, (-0.1, 2.1), ""),
}


@pytest.mark.parametrize("operation", WHILE_PLOTTING)
def test_a_transform_mapping_limits_as_a_line_is_added_enters_the_figure(operation):
    # pltrs measured the line's limits with the figure borrowed: every
    # change, measure and save raised "figure in use".
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    kept = {}
    ax.plot([1, 2], [3, 4], transform=_shifting(fig, ax, operation, kept) + ax.transData)
    fig.canvas.draw()
    value, xlim, title = WHILE_PLOTTING[operation]
    assert kept["value"] == pytest.approx(value) if isinstance(value, tuple) else kept["value"] == value
    assert ax.get_xlim() == pytest.approx(xlim)
    assert ax.get_title() == title


#: matplotlib's values for the same inside `relim`.
WHILE_RELIMITING = {
    "get_xlim": (-0.1, 2.1),
    "set_xlim": (0.0, 10.0),
    "set_title": "reentered",
    "savefig": None,
}


@pytest.mark.parametrize("operation", WHILE_RELIMITING)
def test_a_transform_mapping_limits_in_relim_enters_the_figure(operation):
    # pltrs' relim held the figure for change: even reading it failed.
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    line, = ax.plot([1, 2], [3, 4])
    kept = {}
    line.set_transform(_shifting(fig, ax, operation, kept) + ax.transData)
    ax.relim()
    value = WHILE_RELIMITING[operation]
    assert kept["value"] == pytest.approx(value) if isinstance(value, tuple) else kept["value"] == value


def test_a_converter_giving_an_array_of_one_sets_limits():
    # matplotlib squeezes what a converter gives for a limit.
    class Thing:
        def __init__(self, value):
            self.value = value

    class Converting(units.ConversionInterface):
        @staticmethod
        def convert(value, unit, axis):
            return np.asarray([thing.value for thing in np.atleast_1d(value)], dtype=float)

    units.registry[Thing] = Converting()
    try:
        fig, ax = plt.subplots()
        ax.plot([Thing(1), Thing(2)], [1, 2])
        assert ax.set_xlim(Thing(0), Thing(5)) == (0.0, 5.0)
        assert ax.get_xlim() == (0.0, 5.0)
    finally:
        del units.registry[Thing]


def test_measuring_places_an_axes_by_its_locator_first():
    # matplotlib's Axes.get_tightbbox calls the locator before measuring,
    # also before the first draw.
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], [1, 4, 9])
    calls = []
    ax.set_axes_locator(lambda axes, renderer: (
        calls.append(renderer is not None), transforms.Bbox.from_bounds(0.1, 0.1, 0.8, 0.8))[1])
    bounds = fig.get_tightbbox().bounds
    assert calls == [True]
    assert bounds == pytest.approx((0.4527, 0.2439, 5.3073, 4.0761), abs=1e-4)
    assert ax.get_position().bounds == pytest.approx((0.1, 0.1, 0.8, 0.8))


def test_a_child_added_while_relim_measures_counts():
    # matplotlib's relim loops over the list of children, which sees one
    # added meanwhile (here by a transform of the program's).
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    line, = ax.plot([1, 2], [3, 4])
    added = []

    class Adding(transforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            if not added:
                added.append(ax.add_patch(pltrs.patches.Rectangle((5, 0), 1, 1)))
            return values

    line.set_transform(Adding() + ax.transData)
    ax.relim()
    ax.autoscale_view()
    assert ax.get_xlim() == pytest.approx((-0.3, 6.3))


def test_a_collection_is_a_child_when_its_limits_are_measured():
    # matplotlib's add_collection adds the collection, then measures it.
    fig, ax = plt.subplots()
    seen = []

    class Seeing(transforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            if not seen:
                seen.append([type(child).__name__ for child in ax.get_children()][:2])
            return values

    ax.scatter([1, 2], [3, 4], transform=Seeing() + ax.transData)
    assert seen == [["PathCollection", "Spine"]]
