"""
What matplotlib's `.colorbar.make_axes` and `.colorbar.make_axes_gridspec`
compute with their arguments before they make the colorbar's Axes, for
the errors they raise there.

matplotlib computes the colorbar's box and the parents' new places from
the fraction, shrink, pad and aspect as given, in Python, and fails where
that arithmetic does (a fraction given as a string), or where the Axes is
made in a box that is not finite, the parents already moved there. pltrs
(Rust) places the parents and makes the Axes with numbers: these
functions compute as matplotlib's do first, raising their errors with
their side effects, and return the settings pltrs (Rust) then places with.
"""

import numpy as np

from . import _api

#: The defaults of each location (matplotlib's
#: ``_normalize_location_orientation``).
_LOCATIONS = {
    "left": {"anchor": (1.0, 0.5), "panchor": (0.0, 0.5), "pad": 0.10},
    "right": {"anchor": (0.0, 0.5), "panchor": (1.0, 0.5), "pad": 0.05},
    "top": {"anchor": (0.5, 0.0), "panchor": (0.5, 1.0), "pad": 0.05},
    "bottom": {"anchor": (0.5, 1.0), "panchor": (0.5, 0.0), "pad": 0.15},
}


def _settings(location, orientation):
    """The location of the colorbar and its defaults, from *location* or
    *orientation*, checked as matplotlib's
    ``_normalize_location_orientation`` checks them."""
    from .colorbar import _get_orientation_from_location, _get_ticklocation_from_orientation
    if location is None:
        location = _get_ticklocation_from_orientation(orientation)
    settings = _api.getitem_checked(_LOCATIONS, location=location)
    if orientation is not None and orientation != _get_orientation_from_location(location):
        # Allow the user to pass both if they are consistent.
        raise TypeError("location and orientation are mutually exclusive")
    return location, settings


def check_make_axes(parents, location, orientation, fraction, shrink, aspect, kwargs):
    """
    Compute the colorbar's box and the new places of *parents* as
    matplotlib's `make_axes` does, for its errors; a box that is not
    finite fails where matplotlib makes the Axes in it, *parents* moved
    there as matplotlib moves them first.
    """
    from .transforms import Bbox, BboxTransform
    location, settings = _settings(location, orientation)
    anchor = kwargs.get('anchor', settings['anchor'])
    panchor = kwargs.get('panchor', settings['panchor'])
    fig = parents[0].get_figure()
    pad = kwargs.get('pad', 0.05 if fig.get_constrained_layout() else settings['pad'])
    if not all(fig is ax.get_figure() for ax in parents):
        raise ValueError('Unable to create a colorbar Axes as not all '
                         'parents share the same figure.')
    pb = parents_bbox = Bbox.union(
        [ax.get_position(original=True).frozen() for ax in parents])
    if location in ('left', 'right'):
        if location == 'left':
            pbcb, _, pb1 = pb.splitx(fraction, fraction + pad)
        else:
            pb1, _, pbcb = pb.splitx(1 - fraction - pad, 1 - fraction)
        pbcb = pbcb.shrunk(1.0, shrink).anchored(anchor, pbcb)
    else:
        if location == 'bottom':
            pbcb, _, pb1 = pb.splity(fraction, fraction + pad)
        else:
            pb1, _, pbcb = pb.splity(1 - fraction - pad, 1 - fraction)
        pbcb = pbcb.shrunk(shrink, 1.0).anchored(anchor, pbcb)
        1.0 / aspect  # noqa: B018  (the aspect as y's per x)
    if np.isfinite(pbcb).all():
        return
    shrinking_trans = BboxTransform(parents_bbox, pb1)
    for ax in parents:
        ax._set_position(Bbox(shrinking_trans.transform(ax.get_position(original=True))))
        if panchor is not False:
            ax.set_anchor(panchor)
    raise ValueError(f'all entries in rect must be finite not {pbcb}')


def check_make_axes_gridspec(parent, location, orientation, fraction, shrink,
                             aspect, kwargs):
    """Compute the ratios of the grid matplotlib's `make_axes_gridspec`
    divides the cell of *parent* into, for its errors."""
    location, settings = _settings(location, orientation)
    anchor = kwargs.get('anchor', settings['anchor'])
    pad = kwargs.get('pad', settings['pad'])
    2 * pad / (1 - pad)  # noqa: B018  (the space between the cells)
    parent.get_subplotspec().subgridspec  # noqa: B018
    if location in ('left', 'right'):
        [(1 - anchor[1]) * (1 - shrink), shrink, anchor[1] * (1 - shrink)]  # noqa: B018
        [fraction, 1 - fraction - pad]  # noqa: B018
    else:
        [anchor[0] * (1 - shrink), shrink, (1 - anchor[0]) * (1 - shrink)]  # noqa: B018
        [fraction, 1 - fraction - pad]  # noqa: B018
        1 / aspect  # noqa: B018


def check_colorbar(extend, ticks, locator, format, extendfrac, orientation,
                   ticklocation, location):
    """
    Check the arguments of a `.Colorbar` as matplotlib's constructor
    computes with them, in its order: the extensions, the ticks (fixed by
    a `.FixedLocator`), then, drawing the colorbar, the side of its ticks
    along its long axis, the lengths of its extensions and its formatter.
    """
    from . import ticker
    from .colorbar import _get_ticklocation_from_orientation
    if extend is not None:
        _api.getitem_checked(_INSIDE, extend=extend)
    if locator is None and ticks is not None:
        ticker.FixedLocator(ticks, nbins=len(ticks))
    if ticklocation == 'auto':
        ticklocation = (_get_ticklocation_from_orientation(orientation)
                        if location is None else location)
    sides = ['left', 'right'] if orientation == 'vertical' else ['top', 'bottom']
    _api.check_in_list(sides, position=ticklocation)
    if isinstance(extendfrac, str):
        _api.check_in_list(['auto'], extendfrac=extendfrac.lower())
    if not (format is None or isinstance(format, str) or callable(format)):
        _api.check_isinstance(ticker.Formatter, formatter=format)


#: The colours inside the extensions of each kind of extension
#: (matplotlib's ``Colorbar._inside``).
_INSIDE = {'neither': slice(0, None), 'both': slice(1, -1),
           'min': slice(1, None), 'max': slice(0, -1)}
