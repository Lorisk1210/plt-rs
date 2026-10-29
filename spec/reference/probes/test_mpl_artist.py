"""matplotlib's own non-image tests of ``Artist``
(``matplotlib/tests/test_artist.py``) that the ``artist_behaviour`` area
leaves out.

Area ``mpl_artist``. Every op runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``).

Ops
---
``Artist.get_figure``  {} -> whether each figure ``test_get_figure`` asks
    for is the expected one, for a figure holding the subfigure ``sfig1``
    holding the subfigure ``sfig2`` holding an axes and its line: the
    figure is its own root and parent figure, the axes and the line are
    in ``sfig2`` (``root=False``) and in the figure (``root=True``),
    ``sfig2`` is in ``sfig1`` and in the figure, and ``sfig1`` is in the
    figure either way.

Left out, besides the tests the ``artist_behaviour`` area lists: the
parts of ``test_get_figure`` about artists outside any figure (a pltrs
artist always belongs to one), about the deprecated ``get_figure()``
without ``root`` and about the ``figure`` attribute, which matplotlib
makes the parent for axes but the root for subfigures (pltrs's
``figure()`` is always the root, ``parent_figure()`` the parent).
"""

import matplotlib.pyplot as plt

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_artist")
probe = prober(probes)


def _get_figure():
    fig = plt.figure()
    sfig1 = fig.subfigures()
    sfig2 = sfig1.subfigures()
    ax = sfig2.subplots()
    line, = ax.plot([0, 1])
    return {
        "figure_root": fig.get_figure(root=True) is fig,
        "figure_parent": fig.get_figure(root=False) is fig,
        "axes_root": ax.get_figure(root=True) is fig,
        "axes_parent": ax.get_figure(root=False) is sfig2,
        "line_root": line.get_figure(root=True) is fig,
        "line_parent": line.get_figure(root=False) is sfig2,
        "sfig2_root": sfig2.get_figure(root=True) is fig,
        "sfig2_parent": sfig2.get_figure(root=False) is sfig1,
        "sfig1_root": sfig1.get_figure(root=True) is fig,
        "sfig1_parent": sfig1.get_figure(root=False) is fig,
    }


def test_get_figure():
    assert all(probe("Artist.get_figure", _get_figure).values())
