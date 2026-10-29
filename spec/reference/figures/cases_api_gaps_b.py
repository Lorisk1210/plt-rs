"""Figures drawn through the matplotlib API that pltrs gained in its
axes and artist classes: axes locators, the colour mapping, hatch and
offsets of collections set after they are made, new image data and
resampling settings, and date axes without dates. (Its custom legend
handlers draw the last figures of the legend guide, in
``cases_gallery_guides_axes``.)
"""

import matplotlib.transforms as mtransforms
import numpy as np

from spec_support import case

G = "api_gaps_b"


def api(name, **options):
    """Registers a case of this group."""
    return case(f"{G}/{name}", **options)


# --- axes, collections, images and dates -----------------------------------------


@api("axes_locator")
def _(fig):
    fixed = fig.add_subplot(1, 2, 1)
    fixed.plot([0, 1, 2], [0, 1, 0])
    fixed.set_axes_locator(lambda ax, renderer: mtransforms.Bbox.from_bounds(0.1, 0.5, 0.3, 0.4))

    def lower_half(ax, renderer):
        x0, y0, width, height = ax.get_position(original=True).bounds
        return mtransforms.Bbox.from_bounds(x0, y0, width, height / 2)

    halved = fig.add_subplot(1, 2, 2)
    halved.plot([0, 1, 2], [2, 0, 1])
    halved.set_axes_locator(lower_half)
    halved.set_aspect("equal")


@api("collection_setters")
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 1, 6)
    squares = ax.scatter(x, x ** 2, s=400, marker="s")
    squares.set_hatch("//")
    squares.set_array(np.linspace(0, 10, 6))
    squares.set_hatch_linewidth(3)
    band = ax.fill_between(x, -0.5, x / 2 - 0.5, facecolor="none", edgecolor="k")
    band.set_hatch("x")
    band.set_hatch_linewidth(0.3)
    corner = ax.scatter([0.0], [0.0], s=200, color="k")
    corner.set_offset_transform(ax.transAxes)
    corner.set_offsets([(0.1, 0.9)])


@api("image_set_data")
def _(fig):
    left, right = fig.subplots(1, 2)
    first = left.imshow(np.arange(12.0).reshape(3, 4))
    first.set_data(np.arange(8.0).reshape(4, 2) ** 2)
    sinc = right.imshow(np.arange(16.0).reshape(4, 4) % 3, interpolation="sinc")
    sinc.set_filterrad(1.5)
    sinc.set_filternorm(False)


@api("date_axes")
def _(fig):
    dated, empty = fig.subplots(2, 1)
    dated.plot([19000.0, 19003.0, 19010.0], [0.0, 2.0, 1.0])
    dated.xaxis_date()
    empty.yaxis_date()
