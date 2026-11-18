"""
Units and dates as matplotlib handles them (task UNIT1): error bars in
`timedelta`, a program's subclass of the category converter, `set_units`
with a `UnitData`, masked dates, the Line2D checks a unit type sees, and
patches, texts and annotations given values in units (converted when they
join an Axes and again when its units change, read back as given). Every
expected value was recorded from matplotlib 3.11; the file passes under
matplotlib through the alias of ``python/tests/leaks/alias``.
"""

import datetime
import uuid

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import category, dates as mdates, patches as mpatches, text as mtext
from pltrs import units as munits

DAY = datetime.timedelta(days=1)
D0 = datetime.datetime(2024, 5, 1)


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams and registry, whatever ran before."""
    saved = dict(munits.registry)
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    munits.registry.clear()
    munits.registry.update(saved)
    plt.close("all")


class Q:
    """A length in metres, converted to the axis unit (``"m"``, ``"km"``)."""

    def __init__(self, m):
        self.m = m

    def __add__(self, other):
        return Q(self.m + (other.m if isinstance(other, Q) else other))

    __radd__ = __add__

    def __sub__(self, other):
        return Q(self.m - (other.m if isinstance(other, Q) else other))

    def __rsub__(self, other):
        return Q((other.m if isinstance(other, Q) else other) - self.m)

    def __mul__(self, factor):
        return Q(self.m * factor)

    __rmul__ = __mul__

    def __float__(self):
        return float(self.m)


class QConverter(munits.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        factor = 1000.0 if unit == "km" else 1.0
        if isinstance(value, Q):
            return value.m / factor
        if np.iterable(value):
            return np.array([v.m / factor if isinstance(v, Q) else v for v in value])
        return value

    @staticmethod
    def default_units(x, axis):
        return "m"


@pytest.fixture
def quantity_axes():
    """An Axes whose x axis converts `Q` lengths, in metres, 0 to 10 km."""
    munits.registry[Q] = QConverter()
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    ax.plot([Q(0), Q(10000)], [0, 10])
    return fig, ax


def extent(artist, fig):
    return np.round(artist.get_window_extent(fig.canvas.get_renderer()).bounds, 1).tolist()


def test_errorbar_timedelta_ends_as_matplotlib_adds_them():
    # ``date + timedelta(hours=-12)`` is the day before; matplotlib adds
    # the negated error, so the lower end of the first bar is June 30.
    days = [datetime.date(2024, 7, d) for d in (1, 4, 7, 10)]
    fig, ax = plt.subplots()
    container = ax.errorbar(days, [2, 3, 2, 1], xerr=datetime.timedelta(hours=12), fmt="k.")
    assert ax.dataLim.intervalx.tolist() == [19904.0, 19914.0]
    assert container.lines[2][0].get_segments()[0][:, 0].tolist() == [19904.0, 19905.0]


def test_program_subclass_of_category_converter_converts():
    class Times10(category.StrCategoryConverter):
        @staticmethod
        def convert(value, unit, axis):
            return np.asarray(category.StrCategoryConverter.convert(value, unit, axis)) * 10

    munits.registry[str] = Times10()
    fig, ax = plt.subplots()
    ax.bar(["a", "b", "c"], [1, 2, 3])
    fig.canvas.draw()
    assert type(ax.xaxis.get_converter()) is Times10
    assert np.round(ax.get_xlim(), 2).tolist() == [-1.44, 21.44]


def test_uuid_converter_shape_of_pandas():
    class UuidConverter(category.StrCategoryConverter):
        @staticmethod
        def convert(value, unit, axis):
            if np.iterable(value) and all(isinstance(u, uuid.UUID) for u in value):
                value = [u.bytes for u in value]
            return category.StrCategoryConverter.convert(value, unit, axis)

        @staticmethod
        def default_units(data, axis):
            if np.iterable(data) and all(isinstance(u, uuid.UUID) for u in data):
                data = [u.bytes for u in data]
            return category.StrCategoryConverter.default_units(data, axis)

    munits.registry[uuid.UUID] = UuidConverter()
    fig, ax = plt.subplots()
    ax.scatter([uuid.UUID(int=k) for k in (3, 1, 2)], [1, 2, 3])
    assert np.round(ax.get_xlim(), 2).tolist() == [-0.1, 2.1]
    assert len(ax.xaxis.get_units()._mapping) == 3


def test_set_units_unitdata_replaces_the_categories():
    fig, ax = plt.subplots()
    line, = ax.plot(["a", "b", "c"], [1, 2, 3])
    units = category.UnitData(["c", "b", "a"])
    ax.xaxis.set_units(units)
    fig.canvas.draw()
    assert [t.get_text() for t in ax.get_xticklabels()] == ["c", "b", "a"]
    assert line.get_xdata(orig=False).tolist() == [2.0, 1.0, 0.0]
    assert ax.xaxis.get_units() is units


def test_unitdata_changed_in_place_and_shared():
    fig, (a, b) = plt.subplots(1, 2, sharex=True)
    a.plot(["a", "b"], [1, 2])
    units = category.UnitData(["b", "a"])
    b.xaxis.set_units(units)
    assert a.xaxis.get_units() is units
    units.update(["n"])
    a.plot(["m"], [3])
    fig.canvas.draw()
    assert [t.get_text() for t in a.get_xticklabels()] == ["b", "a", "n", "m"]
    assert a.lines[1].get_xdata(orig=False).tolist() == [3.0]


def test_unitdata_of_bytes():
    fig, ax = plt.subplots()
    ax.plot([b"a", b"b"], [1, 2])
    ax.xaxis.set_units(category.UnitData([b"b", b"a"]))
    fig.canvas.draw()
    assert dict(ax.xaxis.get_units()._mapping) == {b"b": 0, b"a": 1}


def test_masked_dates_plot_and_date2num():
    masked = np.ma.masked_array([datetime.datetime(2024, 1, k) for k in (1, 2, 3)],
                                mask=[0, 1, 0])
    fig, ax = plt.subplots()
    line, = ax.plot(masked, [1, 2, 3])
    assert np.isnan(line.get_xdata(orig=False)[1])
    numbers = mdates.date2num(masked)
    # Under the mask, matplotlib's masked arithmetic leaves seconds since
    # 1970, not days.
    assert np.ma.getdata(numbers).tolist() == [19723.0, 1704153600.0, 19725.0]
    assert np.ma.getmaskarray(numbers).tolist() == [False, True, False]


def test_bar_of_masked_dates_fails_as_matplotlib():
    masked = np.ma.masked_array([datetime.date(2024, 1, k) for k in (1, 2, 3)],
                                mask=[0, 1, 0])
    fig, ax = plt.subplots()
    bars = ax.bar(masked, [1, 2, 3])
    assert np.round([bar.get_x() for bar in bars], 1).tolist() == [19722.6, 1704153599.6,
                                                                    19724.6]
    with pytest.raises(ValueError, match=r"^Date ordinal .* converts to .* \(using epoch "
                       r"1970-01-01T00:00:00\), but Matplotlib dates must be between "
                       r"year 0001 and 9999\.$"):
        fig.canvas.draw()


def test_stairs_take_masked_data():
    edges = np.ma.masked_array([1.0, 2.0, 3.0], mask=[0, 1, 0])
    fig, ax = plt.subplots()
    steps = ax.stairs([1, 2], edges)
    assert steps.get_data().edges.tolist() == [1.0, 2.0, 3.0]


def test_line_data_checked_as_sequences():
    class Values:
        """Numbers in a unit (no ``values`` attribute, which matplotlib
        would unpack)."""
        count = 0

        def __init__(self, numbers):
            self.numbers = np.asarray(numbers, float)

        shape = property(lambda self: self.numbers.shape)
        ndim = property(lambda self: self.numbers.ndim)

        def __len__(self):
            return len(self.numbers)

        def __getitem__(self, index):
            return Values(self.numbers[index])

        def __iter__(self):
            Values.count += 1
            return (Values(number) for number in self.numbers)

    class Converter(munits.ConversionInterface):
        @staticmethod
        def convert(value, unit, axis):
            return value.numbers if isinstance(value, Values) else value

    munits.registry[Values] = Converter()
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], Values([1, 2, 3]))
    fig.canvas.draw()
    # `Line2D.__init__` and `set_ydata` check that the data is a sequence,
    # and the conversion asks twice whether it is natively supported.
    assert Values.count == 4


def test_line_data_that_is_no_sequence_is_refused():
    class Scalar:
        """Numbers in a unit that refuse to be iterated."""

        def __init__(self, numbers):
            self.numbers = np.asarray(numbers, float)

        shape = property(lambda self: self.numbers.shape)
        ndim = property(lambda self: self.numbers.ndim)

        def __len__(self):
            return len(self.numbers)

        def __getitem__(self, index):
            return Scalar(self.numbers[index])

        def __iter__(self):
            raise TypeError("no iteration")

    class Converter(munits.ConversionInterface):
        @staticmethod
        def convert(value, unit, axis):
            return value.numbers if isinstance(value, Scalar) else value

    munits.registry[Scalar] = Converter()
    fig, ax = plt.subplots()
    with pytest.raises(RuntimeError, match="^ydata must be a sequence$"):
        ax.plot([1, 2, 3], Scalar([1, 2, 3]))


def test_rectangle_in_units_reads_back_as_given():
    fig, ax = plt.subplots()
    ax.plot([D0, D0 + 10 * DAY], [0, 10])
    rect = ax.add_patch(mpatches.Rectangle((D0 + 2 * DAY, 1), 2 * DAY, 3))
    fig.canvas.draw()
    assert rect.get_x() == D0 + 2 * DAY
    assert rect.get_width() == 2 * DAY
    assert type(rect.get_y()) is int
    assert rect.get_bbox().bounds == (19846.0, 1.0, 2.0, 3.0)


def test_patches_follow_a_change_of_units(quantity_axes):
    fig, ax = quantity_axes
    rect = ax.add_patch(mpatches.Rectangle((Q(2000), 1), Q(1500), 3))
    ellipse = ax.add_patch(mpatches.Ellipse((Q(2000), 2), Q(1500), 2))
    text = ax.add_artist(mtext.Text(Q(2000), 2, "t"))
    fig.canvas.draw()
    ax.xaxis.set_units("km")
    fig.canvas.draw()
    assert rect.get_bbox().bounds == (2.0, 1.0, 1.5, 3.0)
    assert extent(rect, fig) == [60.2, 32.2, 21.1, 31.5]
    assert extent(ellipse, fig) == [49.7, 32.2, 21.1, 21.0]
    assert text.get_unitless_position() == (2.0, 2.0)
    assert ax.dataLim.intervalx.tolist() == [0.0, 10.0]


def test_limits_in_units_give_each_axis_its_units_after_an_annotation_in_them():
    # The gallery's units/annotate_with_units: the annotation converts
    # again when the x axis takes its units, which leaves the y axis, not
    # yet asked by matplotlib, to take its own from its limits.
    munits.registry[Q] = QConverter()
    fig, ax = plt.subplots()
    note = ax.annotate("note", (Q(500), Q(1500)))
    ax.set_xlim(Q(0), Q(4000))
    assert (ax.xaxis.get_units(), ax.yaxis.get_units()) == ("m", None)
    assert not ax.yaxis.have_units()
    ax.set_ylim(Q(0), Q(4000))
    assert (ax.xaxis.get_units(), ax.yaxis.get_units()) == ("m", "m")
    assert (ax.get_xlim(), ax.get_ylim()) == ((0.0, 4000.0), (0.0, 4000.0))
    assert note.get_unitless_position() == (500.0, 1500.0)
    ax.yaxis.set_units("km")
    assert note.get_unitless_position() == (500.0, 1.5)


def test_circle_and_annulus_at_a_date():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    circle = ax.add_patch(mpatches.Circle((D0 + 2 * DAY, 2), 1))
    annulus = ax.add_patch(mpatches.Annulus((D0 + 2 * DAY, 2), 1, 0.5))
    assert ax.dataLim.intervalx.tolist() == [19845.0, 19847.0]
    assert circle.get_center() == (D0 + 2 * DAY, 2)
    assert annulus.get_center()[0] == D0 + 2 * DAY


def test_patches_computing_with_a_quantity_float_it(quantity_axes):
    fig, ax = quantity_axes
    wedge = ax.add_patch(mpatches.Wedge((Q(2000), 2), 1, 0, 90))
    polygon = ax.add_patch(mpatches.RegularPolygon((Q(2000), 2), 5, radius=1))
    box = ax.add_patch(mpatches.FancyBboxPatch((Q(2000), 1), Q(1500), 2))
    fig.canvas.draw()
    assert extent(wedge, fig) == [60.2, 42.8, 0.0, 10.5]
    assert extent(polygon, fig) == [60.2, 34.3, 0.0, 19.0]
    assert extent(box, fig) == [60.2, 29.1, 21.1, 27.3]


def test_arc_in_units_fails_as_made():
    with pytest.raises(AttributeError, match="arctan2"):
        mpatches.Arc((D0, 2), 2 * DAY, 2, theta1=0, theta2=180)


def test_patches_in_units_read_back_before_an_axes():
    rect = mpatches.Rectangle((D0, 1), 2 * DAY, 3)
    rect.set_y(5)
    assert (rect.get_x(), rect.get_y(), rect.get_width()) == (D0, 5, 2 * DAY)
    assert mpatches.Ellipse((D0, 2), 2 * DAY, 2).get_center() == (D0, 2)
    assert mpatches.FancyArrowPatch((D0, 1), (D0, 3))._posA_posB == [(D0, 1), (D0, 3)]
