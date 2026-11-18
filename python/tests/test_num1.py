"""
The types of the numbers a program gets back and hands on are matplotlib's
(task NUM1): its own ints and numpy scalars back from the getters, values
computed in the input's dtype, and numpy scalars and arrays of the dtype
matplotlib holds handed to the program's callbacks.

The sweep of `number_types` records 3,505 rows (getters of given values,
every getter of drawn artists, float properties set and read, values
matplotlib computes, keys of returned containers, arguments of the
program's callbacks) and holds each to matplotlib's recorded description
(``number_types/reference.json``); the rows still to differ are listed,
each with its reason, in ``number_types/expected.json``, and a row that
stops differing fails until it is taken off that list. The tests after it
pin single behaviours. Every expected value was recorded from matplotlib
3.11; the file passes under matplotlib through the alias of
``python/tests/leaks/alias``.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors, patches, ticker, transforms
from pltrs.path import Path

from number_types import runner

#: Whether the tests run on matplotlib itself (through the alias), where
#: no row may differ.
ON_MATPLOTLIB = pltrs.__name__ == "matplotlib"

REFERENCE = runner.recorded()
EXPECTED = {} if ON_MATPLOTLIB else runner.expected()


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


@pytest.mark.skipif(
    np.lib.NumpyVersion(np.__version__) < "2.0.0",
    reason="matplotlib's recorded types are numpy 2's: with numpy 1.26 matplotlib 3.11.2 "
           "itself fails 7 of these scenes (np.bool_ for np.bool, float32 kept)")
@pytest.mark.parametrize("scene", sorted(runner.SCENES))
def test_number_types_follow_matplotlib(scene):
    ours = runner.run([scene])
    theirs = {row: value for row, value in REFERENCE.items() if row in ours}
    assert theirs, f"no row of {scene} was recorded"
    differing = runner.differing(ours, theirs)
    expected = {row: [entry["matplotlib"], entry["pltrs"]]
                for row, entry in EXPECTED.items() if row in theirs}
    unexpected = {row: pair for row, pair in differing.items() if expected.get(row) != pair}
    assert not unexpected, (
        "rows differing from matplotlib otherwise than expected.json says "
        f"(row: [matplotlib, pltrs]): {unexpected}")
    done = sorted(set(expected) - set(differing))
    assert not done, (
        f"rows that no longer differ from matplotlib: {done}; take them off "
        "number_types/expected.json (python -m number_types --expect)")


def test_every_row_expected_to_differ_has_a_reason():
    assert all(entry["reason"] for entry in EXPECTED.values())
    assert len(EXPECTED) <= 11


def test_a_locator_hands_its_view_limits_as_numpy_floats():
    seen = []

    class Locator(ticker.MaxNLocator):
        def view_limits(self, vmin, vmax):
            seen.append((type(vmin), type(vmax)))
            return super().view_limits(vmin, vmax)

    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], [1, 4, 9])
    ax.xaxis.set_major_locator(Locator())
    fig.canvas.draw()
    assert seen and set(seen) == {(np.float64, np.float64)}


def test_labelling_functions_get_numpy_scalars_of_the_data():
    seen = []
    fig, ax = plt.subplots()
    ax.bar_label(ax.bar([1, 2], [3, 4]), fmt=lambda v: seen.append(type(v)) or "")
    ax.pie(np.float32([1, 2]), autopct=lambda p: seen.append(type(p)) or "")
    ax.clabel(ax.contour(np.arange(9.0).reshape(3, 3), levels=[2, 4]),
              fmt=lambda v: seen.append(type(v)) or "")
    fig.colorbar(ax.imshow(np.arange(4.0).reshape(2, 2)),
                 format=lambda x, pos: seen.append(type(x)) or "")
    fig.canvas.draw()
    assert seen[:4] == [np.int64, np.int64, np.float32, np.float32]
    assert set(seen[4:]) == {np.float64}


def test_a_norm_of_the_program_gets_the_data_in_matplotlibs_dtype():
    seen = []

    class Norm(colors.Normalize):
        def __call__(self, value, clip=None):
            seen.append(np.ma.asarray(value).dtype)
            return super().__call__(value, clip)

    fig, ax = plt.subplots(figsize=(1, 1), dpi=20)
    ax.imshow(np.arange(4, dtype=np.int16).reshape(2, 2), norm=Norm())
    ax.pcolormesh(np.arange(4, dtype=np.int16).reshape(2, 2), norm=Norm())
    fig.canvas.draw()
    # The image resampled in float32, the mesh's own array as given.
    assert set(seen) == {np.dtype(np.float32), np.dtype(np.int16)}


def test_values_computed_from_ints_stay_ints():
    fig, ax = plt.subplots()
    boxes = ax.boxplot([[1, 2, 3, 4, 10]])
    assert boxes["fliers"][0].get_ydata().dtype == np.int64
    assert boxes["caps"][0].get_ydata().dtype == np.int64
    image = ax.imshow(np.arange(4).reshape(2, 2), norm=colors.BoundaryNorm([0, 1, 2, 4], 256))
    bar = fig.colorbar(image)
    fig.canvas.draw()
    assert bar.get_ticks().dtype == np.int64
    assert ax.pcolorfast((0, 8), (3, 6), np.arange(6).reshape(2, 3)).get_extent() == (0, 8, 3, 6)
    assert type(ax.pcolorfast((0, 8), (3, 6), np.ones((2, 3))).get_extent()[0]) is np.int64


def test_ints_the_program_gave_read_back_as_ints():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2], [3, 4], alpha=1)
    points = ax.scatter([1, 2], [3, 4], alpha=1)
    ax.tick_params(axis="x", width=2, pad=6)
    fig.canvas.draw()
    tick = ax.xaxis.get_major_ticks()[0]
    assert [type(value) for value in (line.get_alpha(), points.get_alpha(), tick.get_pad(),
                                      tick.tick1line.get_markeredgewidth())] == [int] * 4
    assert ax.legend(["x"], loc=(0, 0))._loc == (0, 0)


def test_a_formatter_keeps_its_ticks_after_another_locator_ran():
    fig, ax = plt.subplots()
    ax.plot([0, 10], [1, 100])
    ax.xaxis.set_major_locator(ticker.MaxNLocator(5))
    fig.canvas.draw()
    minor = ticker.AutoMinorLocator(2)
    ax.xaxis.set_minor_locator(minor)
    minor()
    formatter = ax.xaxis.get_major_formatter()
    assert list(formatter.locs) == [-2.5, 0, 2.5, 5, 7.5, 10, 12.5]
    assert formatter(2) == "2.0"


def test_helpers_reading_their_axis_fail_without_one():
    for call in (lambda: ticker.MaxNLocator()(), lambda: ticker.LogLocator().nonsingular(1, 10),
                 lambda: ticker.LogFormatter()(10), lambda: ticker.PercentFormatter()(10)):
        with pytest.raises(AttributeError, match="'NoneType' object has no attribute"):
            call()
    assert ticker.LogLocator().nonsingular(-np.inf, np.inf) == (1, 10)
    assert ticker.LogFormatter()(0) == "0"


def test_a_transform_writing_into_its_argument_changes_nothing_pltrs_holds():
    class Doubling(transforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            assert values.flags.writeable
            values *= 2
            return values

        def inverted(self):
            return transforms.Affine2D().scale(0.5)

    fig, ax = plt.subplots(figsize=(2, 2), dpi=20)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    line, = ax.plot([1, 2], [1, 2], transform=Doubling() + ax.transData)
    fig.canvas.draw()
    first = np.asarray(fig.canvas.buffer_rgba()).copy()
    fig.canvas.draw()
    if ON_MATPLOTLIB:
        # matplotlib hands the line's own vertices, which the write changes
        # (draw by draw): a write into the argument succeeds there too.
        return
    assert line.get_xydata().tolist() == [[1, 1], [2, 2]]
    assert (np.asarray(fig.canvas.buffer_rgba()) == first).all()


def test_a_connection_style_of_the_program_gets_arrays():
    seen = []

    class Straight(patches.ConnectionStyle._Base):
        def connect(self, posA, posB):
            seen.append((type(posA), np.shape(posA), type(posB)))
            return Path([posA, posB])

    fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
    ax.add_patch(patches.FancyArrowPatch((0.1, 0.1), (0.9, 0.9), arrowstyle="->",
                                         connectionstyle=Straight()))
    fig.canvas.draw()
    assert seen and set(seen) == {(np.ndarray, (2,), np.ndarray)}
