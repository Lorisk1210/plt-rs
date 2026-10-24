"""matplotlib's non-image tests from ``tests/test_text.py`` and
``tests/test_mathtext.py`` that only check that a figure draws:
``test_mathtext_cmr10_minus_sign``, tick labels written as mathtext in
``cmr10``, which has no minus sign of its own (matplotlib used to warn
about the missing glyph). The other tests of these files are probes of the
``mpl_text`` area or cases of ``mpl_text``.
"""

from figures._mpl_tests import group_case

GROUP = "mpl_tests_text"
mpl_test = group_case(GROUP)


@mpl_test("mathtext_cmr10_minus_sign",
          rc={"font.family": "cmr10", "axes.formatter.use_mathtext": True})
def _(fig):
    fig.subplots().plot(range(-1, 1), range(-1, 1))
