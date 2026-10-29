"""matplotlib's own tests of ``matplotlib.container``
(``matplotlib/tests/test_container.py``): removing stem, error bar and pie
containers, and the geometry a bar container reports.

Area ``mpl_container``. Every op draws on ``Figure().add_subplot()``.
Artist counts are recorded as ``[lines, collections, patches, texts,
containers]``, the lengths of the axes' artist lists.

Ops
---
``StemContainer.remove``  {locs, heads} -> the counts after
    ``stem(locs, heads)`` and after removing its container
    (``test_stem_remove``).
``ErrorbarContainer.remove``  {xerr?, yerr?, fmt?} -> the counts after
    ``errorbar([1], [1], ...)`` and after removing its container
    (``test_errorbar_remove``).
``BarContainer``  {orientation, positions, values, base} ->
    ``{position_centers, bottoms, tops}`` of ``bar(positions, values,
    bottom=base)`` or ``barh(positions, values, left=base)``
    (``test_barcontainer_position_centers__bottoms__tops``).
``PieContainer.remove``  {values, labels, autopct, added} -> the counts
    after ``pie(values, labels=labels, autopct=autopct)``, after
    ``pie_label(pie, added)`` and after removing the pie
    (``test_piecontainer_remove``).
``Axes.pie``  {values, labels, autopct, labeldistance} -> the lengths of
    the wedges, labels and percentage labels unpacked from the pie
    (``test_piecontainer_unpack_backcompat``).

Left out: ``test_nonstring_label``, which checks that a number is accepted
as a legend label (pltrs labels are strings), and the
``errorbar([1], [1], fmt="none")`` case of ``test_errorbar_remove``: that
container holds no artist at all, and pltrs containers find the axes
recording them through their artists, so its entry stays in
``ax.containers``.
"""

from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mpl_container")


def _counts(ax):
    return [len(ax.lines), len(ax.collections), len(ax.patches), len(ax.texts),
            len(ax.containers)]


def _stem_remove(locs, heads):
    ax = Figure().add_subplot()
    stems = ax.stem(locs, heads)
    before = _counts(ax)
    stems.remove()
    return [before, _counts(ax)]


def _errorbar_remove(**options):
    ax = Figure().add_subplot()
    bars = ax.errorbar([1], [1], **options)
    before = _counts(ax)
    bars.remove()
    return [before, _counts(ax)]


def _bar_container(orientation, positions, values, base):
    ax = Figure().add_subplot()
    if orientation == "vertical":
        bars = ax.bar(positions, values, bottom=base)
    else:
        bars = ax.barh(positions, values, left=base)
    return {"position_centers": bars.position_centers, "bottoms": bars.bottoms,
            "tops": bars.tops}


def _pie_remove(values, labels, autopct, added):
    ax = Figure().add_subplot()
    pie = ax.pie(values, labels=labels, autopct=autopct)
    drawn = _counts(ax)
    ax.pie_label(pie, added)
    labelled = _counts(ax)
    pie.remove()
    return [drawn, labelled, _counts(ax)]


def _pie_unpack(values, labels, autopct, labeldistance):
    ax = Figure().add_subplot()
    wedges, texts, autotexts = ax.pie(values, labels=labels, autopct=autopct,
                                      labeldistance=labeldistance)
    return [len(wedges), len(texts), len(autotexts)]


def test_stem_remove():
    probes.probe("StemContainer.remove", _stem_remove, locs=[1, 2], heads=[1, 2])


def test_errorbar_remove():
    for options in [{}, {"xerr": 1}, {"yerr": 2}, {"xerr": [2], "yerr": 2},
                    {"xerr": 1, "fmt": "none"}]:
        probes.probe("ErrorbarContainer.remove", _errorbar_remove, **options)


def test_barcontainer_position_centers__bottoms__tops():
    for orientation in ["vertical", "horizontal"]:
        probes.probe("BarContainer", _bar_container, orientation=orientation,
                     positions=[1, 2, 4], values=[2, 3, 4], base=[1, 5, 3])


def test_piecontainer_remove():
    probes.probe("PieContainer.remove", _pie_remove, values=[2, 3], labels=["foo", "bar"],
                 autopct="%1.0f%%", added=["baz", "qux"])


def test_piecontainer_unpack_backcompat():
    probes.probe("Axes.pie", _pie_unpack, values=[2, 3], labels=["foo", "bar"],
                 autopct="%1.0f%%", labeldistance=None)
