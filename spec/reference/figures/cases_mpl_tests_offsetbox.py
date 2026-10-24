"""matplotlib's non-image tests from ``tests/test_offsetbox.py`` that only
check that a figure draws:

* ``offsetbox_loc_codes``: an ``AnchoredOffsetbox`` of an empty 100 x 100
  ``DrawingArea`` at each of the ten location names, framed
  (``test_offsetbox_loc_codes``).
* ``expand_with_tight_layout``: a two-column legend expanded to the width
  of the axes, then ``tight_layout`` (``test_expand_with_tight_layout``,
  where the crash used to happen).

The other tests of the file are probes of the ``offsetbox_behaviour`` and
``mpl_offsetbox`` areas or cases of ``mpl_misc``.
"""

from matplotlib.offsetbox import AnchoredOffsetbox, DrawingArea

from figures._mpl_tests import group_case

GROUP = "mpl_tests_offsetbox"
mpl_test = group_case(GROUP)

#: The location names of ``test_offsetbox_loc_codes``.
LOCATIONS = ["upper right", "upper left", "lower left", "lower right", "right",
             "center left", "center right", "lower center", "upper center", "center"]


@mpl_test("offsetbox_loc_codes")
def _(fig):
    ax = fig.subplots()
    da = DrawingArea(100, 100)
    for loc in LOCATIONS:
        ax.add_artist(AnchoredOffsetbox(loc=loc, child=da))


@mpl_test("expand_with_tight_layout")
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2], label="series 1")
    ax.plot([2, 1], label="series 2")
    ax.legend(ncols=2, mode="expand")
    fig.tight_layout()
