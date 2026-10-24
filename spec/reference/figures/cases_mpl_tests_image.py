"""matplotlib's own tests of images (``tests/test_image.py``) that draw and
then check pixels or the saved document, rather than compare with a
baseline image: one case per figure they check.

Every test runs in the test suite's default style, ``classic`` plus
``_classic_test_patch``, on classic's 8 x 6 in figure saved at 100 dpi
unless it sizes its figure.

* ``image_edges``: an image filling a 1 in figure saved with a green face
  colour (``savefig(facecolor=(0, 1, 0))``, the case's
  ``savefig.facecolor``); its edges must not show the green.
* ``unclipped``: an unclipped image reaching beyond the axes fills the
  whole figure with black.
* ``respects_bbox_before`` / ``respects_bbox_after``: an image of the lower
  axes clipped to the box of the upper one only (``set_clip_path(None)``,
  ``set_clip_box(axs[0].bbox)``), invisible until the lower axes is panned
  to show it in the upper box.
* ``image_preserve_size2``: a 7 in figure saved at 1 dpi keeps the 7 x 7
  identity matrix pixel for pixel.
* ``minimized_rasterized``: two colorbars whose rasterised images the SVG
  must keep as wide as the colorbars (``xtask vectors`` compares it).
* ``rgba_stage_demultiplies_resampled_alpha``: flat RGBA images of alpha 0,
  0.25, 0.5, 0.75 and 1 resampled in the ``rgba`` stage, one per axes;
  their colours must come back from the premultiplied resampling unchanged.

Deviations from the tests: ``np.random`` draws come from
``spec_support.rng`` with the tests' seeds, and the five parameters of
``test_rgba_stage_demultiplies_resampled_alpha`` share one figure.
"""

import matplotlib.style
import numpy as np

from spec_support import case, rng

GROUP = "mpl_tests_image"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def classic(name, *, figsize=(8, 6), dpi=100, rc=None):
    """Register ``build`` as ``mpl_tests_image/<name>`` in the test suite's
    default style."""
    return case(f"{GROUP}/{name}", figsize=figsize, dpi=dpi, rc={**CLASSIC, **(rc or {})})


@classic("image_edges", figsize=(1, 1), rc={"savefig.facecolor": "#00ff00"})
def _(fig):
    ax = fig.add_axes((0, 0, 1, 1), frameon=False)
    data = np.tile(np.arange(12), 15).reshape(20, 9)
    ax.imshow(data, origin='upper', extent=[-10, 10, -10, 10], interpolation='none',
              cmap='gray')
    ax.set_xlim(-2, 2)
    ax.set_ylim(-2, 2)
    ax.set_xticks([])
    ax.set_yticks([])


@classic("unclipped")
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    ax.imshow([[0, 0], [0, 0]], aspect="auto", extent=(-10, 10, -10, 10), cmap='gray',
              clip_on=False)
    ax.set(xlim=(0, 1), ylim=(0, 1))


def _respects_bbox(fig, panned):
    axs = fig.subplots(2)
    for ax in axs:
        ax.set_axis_off()
    im = axs[1].imshow([[0, 1], [2, 3]], aspect="auto", extent=(0, 1, 0, 1))
    im.set_clip_path(None)
    im.set_clip_box(axs[0].bbox)
    if panned:
        axs[1].set(ylim=(-1, 0))


@classic("respects_bbox_before")
def _(fig):
    _respects_bbox(fig, panned=False)


@classic("respects_bbox_after")
def _(fig):
    _respects_bbox(fig, panned=True)


@classic("image_preserve_size2", figsize=(7, 7), dpi=1)
def _(fig):
    fig.set_frameon(False)
    ax = fig.add_axes((0.0, 0.0, 1.0, 1.0))
    ax.set_axis_off()
    ax.imshow(np.identity(7, float), interpolation='nearest', origin='lower', aspect='auto')


@classic("minimized_rasterized")
def _(fig):
    data = rng(0).uniform(100).reshape(10, 10)
    ax = fig.subplots(1, 2)
    for axes in ax:
        fig.colorbar(axes.pcolormesh(data), ax=axes)


@classic("rgba_stage_demultiplies_resampled_alpha")
def _(fig):
    for ax, alpha in zip(fig.subplots(1, 5), [0, 0.25, 0.5, 0.75, 1]):
        data = np.empty((4, 4, 4))
        data[..., :3] = [0.2, 0.4, 0.6]
        data[..., 3] = alpha
        ax.imshow(data, interpolation='nearest', interpolation_stage='rgba')
