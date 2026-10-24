"""matplotlib's non-image tests from ``tests/test_lines.py`` that only
check that a figure draws: ``test_linestyle_variants``, lines of
``range(10)`` in every named line style (short and long names) and with
the offset dash pattern ``(0, None)``, which draws solid (gh-22930). The
other tests of the file are probes of the ``line_behaviour``, ``markers``
and ``mpl_lines`` areas.

The test's ``(0, ())`` and ``(0, [])`` spellings are left out: matplotlib
writes their empty dash list into SVG (``stroke-dasharray:``) and PGF
output, where it changes nothing, while plt-rs's renderers take an empty
dash pattern for a solid line and write none. The pixels are the same, and
plt-rs draws those spellings exactly as ``(0, None)``.
"""

from figures._mpl_tests import group_case

GROUP = "mpl_tests_lines"
mpl_test = group_case(GROUP)


@mpl_test("linestyle_variants")
def _(fig):
    ax = fig.subplots()
    for ls in ["-", "solid", "--", "dashed", "-.", "dashdot", ":", "dotted", (0, None)]:
        ax.plot(range(10), linestyle=ls)
