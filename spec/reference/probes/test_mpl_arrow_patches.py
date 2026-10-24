"""matplotlib's own non-image tests of arrow patches
(``matplotlib/tests/test_arrow_patches.py``).

Area ``mpl_arrow_patches``. Every op runs in the style matplotlib's test
fixture applies (``classic`` plus ``_classic_test_patch``).

Ops
---
``FancyArrowPatch``  {angleA, angleB} -> ``true`` once a
    ``FancyArrowPatch((.2, .2), (.5, .5))`` with the connection style
    ``Angle3(angleA, angleB)`` is added to an axes and the figure is
    drawn, or the error when the two directions are parallel and the
    connection has no corner (``test_invalid_intersection``: 20° and 200°
    are, 20° and 199.9° are not). matplotlib raises when the patch is
    added, plt-rs when it is drawn.

Every other test of the file draws an image: they are cases of
``mpl_rendering``.
"""

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_arrow_patches")
probe = prober(probes)


def _fancy_arrow(angleA, angleB):
    fig, ax = plt.subplots()
    style = mpatches.ConnectionStyle.Angle3(angleA=angleA, angleB=angleB)
    ax.add_patch(mpatches.FancyArrowPatch((.2, .2), (.5, .5), connectionstyle=style))
    fig.canvas.draw()
    return True


def test_invalid_intersection():
    assert isinstance(probe("FancyArrowPatch", _fancy_arrow, angleA=20, angleB=200), ValueError)
    assert probe("FancyArrowPatch", _fancy_arrow, angleA=20, angleB=199.9) is True
