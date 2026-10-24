"""matplotlib's own non-image tests of saving with ``bbox_inches="tight"``
(``matplotlib/tests/test_bbox_tight.py``).

Area ``mpl_bbox_tight``. Every op runs in the style matplotlib's test
fixture applies (``classic`` plus ``_classic_test_patch``) unless its
``style`` is ``"mpl20"``; figures come from ``plt.subplots()`` (or
``plt.figure``) in that style.

Ops
---
``Figure.savefig``  {scene} -> ``[height, width]`` of the PNG saved with
    ``bbox_inches="tight"``:

    * ``layout_constrained`` (``mpl20``)  an equal-aspect axes laid out by
      constrained layout with ``h_pad=0.5``, without tick labels (what
      ``remove_text`` leaves), saved with ``pad_inches="layout"``: the
      padding is the layout's (``test_bbox_inches_tight_layout_constrained``,
      whose image the ``mpl_figure`` group cannot save, as
      ``pad_inches="layout"`` has no rcParam);
    * ``layout_notconstrained``  a plain axes saved with
      ``pad_inches="layout"``, which falls back to ``savefig.pad_inches``
      (``test_bbox_inches_tight_layout_notconstrained``);
    * ``non_finite_annotation``  an empty annotation at ``(0, nan)`` on an
      axes turned off (``test_only_on_non_finite_bbox``);
    * ``pcolorfast``  ``pcolorfast(arange(4).reshape(2, 2))`` seen over
      ``ylim=(0, .1)``: the clipped-out image does not count, so the
      picture is wider than tall (``test_tight_pcolorfast``).
``Figure.savefig.noop``  {} -> ``{shape, opaque, white}`` for
    ``test_noop_tight_bbox``: a frameless 10 x 7 pixel figure filled by an
    axes turned off showing a rasterized ``imshow`` of ``arange(70)``, saved
    as a PDF with ``bbox_inches="tight", pad_inches=0``, then as a PNG the
    same way: the PNG's ``[height, width]``, whether every pixel is opaque
    and whether every pixel is white.

The image tests are cases of ``mpl_figure``.
"""

import io

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.ticker import NullFormatter

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_bbox_tight")
probe = prober(probes)


def _png_shape(fig, **savefig):
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png", bbox_inches="tight", **savefig)
    buffer.seek(0)
    return list(plt.imread(buffer).shape[:2])


def _savefig(scene):
    if scene == "layout_constrained":
        fig, ax = plt.subplots(layout="constrained")
        fig.get_layout_engine().set(h_pad=0.5)
        ax.set_aspect("equal")
        for axis in (ax.xaxis, ax.yaxis):
            axis.set_major_formatter(NullFormatter())
            axis.set_minor_formatter(NullFormatter())
        return _png_shape(fig, pad_inches="layout")
    if scene == "layout_notconstrained":
        fig, ax = plt.subplots()
        return _png_shape(fig, pad_inches="layout")
    if scene == "non_finite_annotation":
        fig, ax = plt.subplots()
        ax.annotate("", xy=(0, float("nan")))
        ax.set_axis_off()
        return _png_shape(fig)
    if scene == "pcolorfast":
        fig, ax = plt.subplots()
        ax.pcolorfast(np.arange(4).reshape((2, 2)))
        ax.set(ylim=(0, .1))
        return _png_shape(fig)
    raise ValueError(scene)


def _noop():
    fig = plt.figure(frameon=False, dpi=100, figsize=(10 / 100, 7 / 100))
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set_axis_off()
    ax.xaxis.set_visible(False)
    ax.yaxis.set_visible(False)
    ax.imshow(np.arange(70).reshape(7, 10), rasterized=True)
    fig.savefig(io.BytesIO(), bbox_inches="tight", pad_inches=0, format="pdf")
    buffer = io.BytesIO()
    fig.savefig(buffer, bbox_inches="tight", pad_inches=0)
    buffer.seek(0)
    image = np.asarray(plt.imread(buffer) * 255).round().astype(np.uint8)
    return {"shape": list(image.shape[:2]), "opaque": bool((image[:, :, 3] == 255).all()),
            "white": bool((image[:, :, :3] == 255).all())}


def test_savefig():
    probe("Figure.savefig", _savefig, style="mpl20", scene="layout_constrained")
    for scene in ["layout_notconstrained", "non_finite_annotation"]:
        probe("Figure.savefig", _savefig, scene=scene)
    height, width = probe("Figure.savefig", _savefig, scene="pcolorfast")
    assert width > height


def test_noop_tight_bbox():
    res = probe("Figure.savefig.noop", _noop)
    assert res == {"shape": [7, 10], "opaque": True, "white": False}
