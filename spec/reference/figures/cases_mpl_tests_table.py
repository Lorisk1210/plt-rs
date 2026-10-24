"""matplotlib's non-image tests from ``tests/test_table.py`` that only
check that a figure draws: ``test_non_square``, a table of two cell
colours in one column (``cellColours=["b", "r"]``: each string is a row)
below the axes of ``plt.table``. The other tests of the file are probes of
the ``mpl_table`` area or cases of ``mpl_misc``.
"""

from figures._mpl_tests import group_case

GROUP = "mpl_tests_table"
mpl_test = group_case(GROUP)


@mpl_test("non_square")
def _(fig):
    fig.add_subplot().table(cellColours=["b", "r"])
