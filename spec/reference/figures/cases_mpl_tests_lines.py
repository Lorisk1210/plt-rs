"""matplotlib's non-image tests from ``tests/test_lines.py`` that only
check that a figure draws: ``test_linestyle_variants``, lines of
``range(10)`` in every named line style (short and long names) and with
the offset dash patterns ``(0, None)``, ``(0, ())`` and ``(0, [])``, which
draw solid (gh-22930); matplotlib writes the empty dash list of the last
two into SVG, PDF and PGF output. The other tests of the file are probes
of the ``line_behaviour``, ``markers`` and ``mpl_lines`` areas.
"""

from figures._mpl_tests import group_case

GROUP = "mpl_tests_lines"
mpl_test = group_case(GROUP)


@mpl_test("linestyle_variants")
def _(fig):
    ax = fig.subplots()
    for ls in ["-", "solid", "--", "dashed", "-.", "dashdot", ":", "dotted",
               (0, None), (0, ()), (0, [])]:
        ax.plot(range(10), linestyle=ls)
