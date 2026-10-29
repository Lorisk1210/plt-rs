"""``TextPath``, after matplotlib's own ``tests/test_textpath.py``.

Area ``mpl_textpath``. The upstream test checks that copies of a
``TextPath`` share (``copy``) or own (``deepcopy``) equal vertices: Python
object identity, which pltrs' value types make moot (a cloned path holds
equal vertices of its own). The probes record the path itself, for the
text the test uses and a few more, under the reference rcParams with only
matplotlib's bundled fonts installed.

Ops
---
``textpath.TextPath``  {xy, s, size?} -> ``{"vertices", "codes"}`` of
    ``TextPath(xy, s, size=size)`` (``size`` null: the font size, 10 pt).
"""

import pytest
from matplotlib.textpath import TextPath

from spec_support import ProbeSet
from spec_support.fonts import bundled_fonts_only

probes = ProbeSet("mpl_textpath")


@pytest.fixture(scope="module", autouse=True)
def _bundled_fonts_only():
    with bundled_fonts_only():
        yield


def _text_path(xy, s, size=None):
    path = TextPath(tuple(xy), s, size=size)
    return {"vertices": path.vertices, "codes": path.codes}


def test_copy():
    probes.probe("textpath.TextPath", _text_path, tol=1e-12, xy=[0, 0], s=".")
    probes.probe("textpath.TextPath", _text_path, tol=1e-12, xy=[1.5, -2], s="!?", size=150)
    probes.probe("textpath.TextPath", _text_path, tol=1e-12, xy=[0, 0], s="Ag", size=20)
