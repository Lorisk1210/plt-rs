"""Behaviour of Sankey diagrams, ported from matplotlib's
``lib/matplotlib/tests/test_sankey.py``.

Area ``sankey_behaviour``.

Ops
---
``sankey.diagram``  {options, subdiagrams} -> [{"flows", "angles", "tips",
    "text", "texts"}, ...]
    ``s = Sankey(ax=fig.add_subplot(), **options)``, then ``s.add(**sub)``
    for each subdiagram and ``s.finish()``: per diagram the flows, the
    arrow directions in quarter turns (``null`` for flows too small to
    draw), the tips, the trunk text and the flow labels; or the error for
    invalid settings. ``options`` holds ``gap``, ``radius``,
    ``head_angle``, ``tolerance``, ``unit`` and ``format`` (a printf
    template); a subdiagram holds ``flows``, ``labels``, ``orientations``
    (``-1``, ``0``, ``1``), ``trunklength``, ``pathlengths``, ``prior`` and
    ``connect``. Mirrors ``test_sankey``, ``test_label``,
    ``test_format_using_callable`` (with the template ``"%.3f"`` for the
    callable), ``test_sankey_errors``, ``test_sankey_add_errors`` and
    ``test_sankey2``.
"""

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.sankey import Sankey

from spec_support import ProbeSet

probes = ProbeSet("sankey_behaviour")


def _diagram(options, subdiagrams):
    fig = Figure()
    FigureCanvasAgg(fig)
    sankey = Sankey(ax=fig.add_subplot(), **options)
    for sub in subdiagrams:
        sankey.add(**sub)
    return [{"flows": [float(f) for f in d.flows],
             "angles": [None if a is None else int(a) for a in d.angles],
             "tips": np.asarray(d.tips, float).tolist(),
             "text": d.text.get_text(),
             "texts": [t.get_text() for t in d.texts]}
            for d in sankey.finish()]


FOO = {"flows": [0.25, -0.25, 0.5, -0.5], "labels": ["Foo"], "orientations": [-1]}
CASES = [
    ({}, [{}]),
    ({}, [{"flows": [0.25], "labels": ["First"], "orientations": [-1]}]),
    ({"format": "%.3f"}, [{"flows": [0.25], "labels": ["First"], "orientations": [-1]}]),
    ({"gap": -1}, [{}]),
    ({"gap": 1, "radius": 2}, [{}]),
    ({"head_angle": -1}, [{}]),
    ({"tolerance": -1}, [{}]),
    ({}, [{"flows": [1, -1], "orientations": [-1, 0, 1]}]),
    ({}, [{"flows": [1, -1], "labels": ["a", "b", "c"]}]),
    ({}, [{"flows": [0.2, -0.2]}, {"trunklength": -1}]),
    ({}, [{"flows": [0.2, -0.2]}, {"flows": [0.2, 0.3], "prior": 0}]),
    ({}, [{"flows": [0.2, -0.2]}, {"prior": -1}]),
    ({}, [{"flows": [0.2, -0.2]}, {"prior": 1}]),
    ({}, [{"flows": [0.2, -0.2]}, {"connect": [-1, 1], "prior": 0}]),
    ({}, [{"flows": [0.2, -0.2]}, {"connect": [2, 1], "prior": 0}]),
    ({}, [{"flows": [0.2, -0.2]}, {"connect": [1, 3], "prior": 0}]),
    ({}, [{"flows": [0.2, -0.2]}, {"connect": [1, 1], "prior": 0, "flows": [-0.2, 0.2],
                                   "orientations": [2]}]),
    ({}, [{"flows": [0.2, -0.2]}, {"connect": [1, 1], "prior": 0, "flows": [-0.2, 0.2],
                                   "pathlengths": [2]}]),
    ({"unit": "Bar"}, [FOO]),
    ({"unit": "Bar"}, [{**FOO, "flows": [0.25, -0.25, 0, 0.5, -0.5]}]),
    ({}, [{"flows": [0.2, -0.2]}, {"connect": [1, 0], "prior": 0, "flows": [0.2, -0.1, -0.1],
                                   "orientations": [0, 1, -1]}]),
]


def test_diagrams():
    for options, subdiagrams in CASES:
        probes.probe("sankey.diagram", _diagram, options=options, subdiagrams=subdiagrams)
