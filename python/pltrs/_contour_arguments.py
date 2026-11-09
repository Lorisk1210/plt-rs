"""
What matplotlib's contour sets do with the arguments they are made with,
for those pltrs (Rust) does not hold.

matplotlib's ``ContourSet.__init__`` keeps most of its arguments and
computes with them as it makes the set (the line styles of its levels,
the levels its locator ticks, the transform it maps the grid with) or as
it draws it (line widths, hatches, a transform that is no transform),
failing there with the error Python's or numpy's arithmetic raises. The
contour sets hand pltrs (Rust) what it holds (`held_linewidths`,
`held_linestyles`, `held_transform`) and run matplotlib's checks for the
rest where matplotlib computes with them. Ported from matplotlib 3.11.2's
``contour.py`` under the Matplotlib License.
"""

from numbers import Integral

import numpy as np

from . import _api, _live_transforms, _unusable, lines as mlines
from .colorizer import Colorizer


def check_made(kwargs):
    """Fail as matplotlib's constructor does with the keywords *kwargs*
    before it contours: ``color`` (no keyword of a contour set), then a
    colorizer that is no `.Colorizer`."""
    if "color" in kwargs:
        raise _api.kwarg_error("ContourSet.__init__", "color")
    colorizer = kwargs.get("colorizer")
    if colorizer:
        _api.check_isinstance(Colorizer, colorizer=colorizer)


def check_axes(ax, quad):
    """Fail as matplotlib's contour set made for *ax*, which is no Axes,
    fails: a `.QuadContourSet` (*quad*) reads its data transform where it
    contours the grid, a set marks it stale (a false one is not asked),
    then adds itself to it."""
    from .axes import Axes
    if isinstance(ax, Axes):
        return
    if quad:
        ax.transData  # noqa: B018
    if ax:
        ax.stale = True
    ax.add_collection  # noqa: B018


def check_extent(extent):
    """Refuse an *extent* that is no four values, as matplotlib does."""
    if extent is not None and len(extent) != 4:
        raise ValueError(
            "If given, 'extent' must be None or (x0, x1, y0, y1)")


def check_grid_transform(transform, axes):
    """Fail as matplotlib's `.QuadContourSet` fails to map its grid with a
    *transform* that is no transform (nor gives one for *axes*)."""
    if held_transform(transform) is None and transform is not None:
        if transform != axes.transData:
            transform.contains_branch_separately  # noqa: B018


def check_locator(locator, levels):
    """Fail as matplotlib's set fails to tick levels (none given, or their
    number) with a *locator* that is no locator."""
    from .ticker import Locator
    if (locator is not None and not isinstance(locator, Locator)
            and (levels is None or isinstance(levels, Integral))):
        locator.tick_values  # noqa: B018


def held_chunks(nchunk):
    """The number of chunks pltrs (Rust) contours with for *nchunk*: a
    whole number of none or more (None for no chunking), else None, which
    contourpy refuses (`._ContourGenerator.check_chunks`)."""
    if nchunk is None:
        return 0
    return int(nchunk) if isinstance(nchunk, Integral) and nchunk >= 0 else None


def held_transform(transform):
    """*transform* as a contour set holds it: a transform, or an object
    giving one for its Axes (`._live_transforms.gives_transform`); None
    for anything else, which matplotlib keeps and fails on as it draws."""
    from .transforms import Transform
    if isinstance(transform, Transform) or _live_transforms.gives_transform(transform):
        return transform
    return None


def held_linewidths(linewidths):
    """The line widths pltrs (Rust) draws the levels with: numbers, else
    None (its default), which the set keeps (`keep_linewidths`)."""
    if linewidths is None:
        return None
    try:
        return [float(width) for width in np.atleast_1d(linewidths)]
    except (TypeError, ValueError):
        return None


def held_linestyles(convert, styles):
    """*styles* (a line style or a sequence of them) converted by *convert*
    as pltrs (Rust) takes them, if each is a line style matplotlib knows,
    else None (pltrs (Rust)'s default): matplotlib checks them as it makes
    the set (`check_linestyles`)."""
    try:
        held = convert(styles)
        for style in held if isinstance(held, list) else [held]:
            if style is not None:
                mlines._get_dash_pattern(style)
        return held
    except (TypeError, ValueError, LookupError):
        return None


def check_linestyles(contours, linestyles):
    """The line styles of the levels of *contours* as matplotlib's
    ``_process_linestyles`` gives them, set as its ``set`` sets them
    (``_get_dash_patterns``), failing as they fail."""
    Nlev = len(contours.levels)
    if linestyles is None:
        tlinestyles = ['solid'] * Nlev
        if contours.monochrome:
            eps = - (contours.zmax - contours.zmin) * 1e-15
            for i, lev in enumerate(contours.levels):
                if lev < eps:
                    tlinestyles[i] = contours.negative_linestyles
    else:
        if isinstance(linestyles, str):
            tlinestyles = [linestyles] * Nlev
        elif np.iterable(linestyles):
            tlinestyles = list(linestyles)
            if len(tlinestyles) < Nlev:
                nreps = int(np.ceil(Nlev / len(linestyles)))
                tlinestyles = tlinestyles * nreps
            if len(tlinestyles) > Nlev:
                tlinestyles = tlinestyles[:Nlev]
        else:
            raise ValueError("Unrecognized type for linestyles kwarg")
    mlines._get_dash_patterns(tlinestyles)


def keep_unheld(contours, given_transform, linewidths):
    """Have *contours* keep what pltrs (Rust) does not hold of what it was
    made with, failing where matplotlib's draw fails on it: a transform
    that is no transform, hatches that are no sequence (a filled set
    hatches its bands), line widths that are no numbers (the collection
    keeps them)."""
    if given_transform is not None and held_transform(given_transform) is None:
        _unusable.keep(contours, "transform", given_transform,
                       lambda transform: transform.is_affine)
    if contours.filled and not np.iterable(contours.hatches):
        _unusable.keep(contours, "hatches", contours.hatches, iter, measured=False)
    if (linewidths is not None and held_linewidths(linewidths) is None
            and not contours.filled):
        levels = len(contours.levels)
        contours.set_linewidth(
            (list(linewidths) * int(np.ceil(levels / len(linewidths)))
             if np.iterable(linewidths) else [linewidths] * levels)[:levels])
