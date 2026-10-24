"""matplotlib's own tests of widgets (``tests/test_widgets.py``) that draw a
figure and only check that drawing or saving it works: one case per figure.

Every test runs in the test suite's default style, ``classic`` plus
``_classic_test_patch``, saved at 100 dpi.

* ``save_blitted_widget_as_pdf``: default and styled radio and check buttons
  on a 5 x 2 in grid of axes with width ratios 1 and 2, the upper ones
  titled (the test saves it as a PDF after an Agg draw).
* ``parent_axes_removal``: radio and check buttons whose axes were removed,
  leaving an empty 8 x 6 in figure (the test saves it as raw RGBA).
"""

import matplotlib.style
from matplotlib import widgets

from spec_support import case

GROUP = "mpl_tests_widgets"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def classic(name, *, figsize=(8, 6)):
    """Register ``build`` as ``mpl_tests_widgets/<name>`` in the test suite's
    default style."""
    return case(f"{GROUP}/{name}", figsize=figsize, dpi=100, rc=CLASSIC)


@classic("save_blitted_widget_as_pdf", figsize=(5, 2))
def _(fig):
    ax = fig.subplots(nrows=2, ncols=2, width_ratios=[1, 2])
    widgets.RadioButtons(ax[0, 0], ["Apples", "Oranges"])
    widgets.RadioButtons(
        ax[0, 1], ["Apples", "Oranges"],
        label_props={"color": ["red", "orange"], "fontsize": [16, 20]},
        radio_props={"edgecolor": ["red", "orange"], "facecolor": ["mistyrose", "peachpuff"]})
    widgets.CheckButtons(ax[1, 0], ["Apples", "Oranges"], actives=[True, True])
    widgets.CheckButtons(
        ax[1, 1], ["Apples", "Oranges"], actives=[True, True],
        label_props={"color": ["red", "orange"], "fontsize": [16, 20]},
        frame_props={"edgecolor": ["red", "orange"], "facecolor": ["mistyrose", "peachpuff"]},
        check_props={"color": ["darkred", "darkorange"]})
    ax[0, 0].set_title("Default")
    ax[0, 1].set_title("Stylized")


@classic("parent_axes_removal")
def _(fig):
    ax_radio, ax_checks = fig.subplots(1, 2)
    widgets.RadioButtons(ax_radio, ["1", "2"], 0)
    widgets.CheckButtons(ax_checks, ["1", "2"], [True, False])
    ax_checks.remove()
    ax_radio.remove()
