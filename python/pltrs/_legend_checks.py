"""
What matplotlib's `.Legend` constructor computes with its arguments
before pltrs (Rust) makes the legend, for the errors it raises there.

matplotlib's legend stores its paddings, spacings and modes as given and
computes with them as it builds its parts: the box it anchors to, the
patch of its frame, the packers of its entries (``_init_legend_box``),
its title. Its constructor fails where one of these computations does,
in that order; pltrs (Rust) holds numbers, so the legend computes as
matplotlib's does first (`check_anchor`, `check_frame`, `check_entries`,
`check_title`, `check_labelcolor`), then hands pltrs (Rust) what it
holds.
"""

from numbers import Real

import numpy as np

from . import cbook, colors, rcParams


def check_anchor(bbox, parent):
    """Make the box the legend anchors to, *bbox* (None for its parent's)
    in the coordinates of the transform given, as matplotlib's
    ``set_bbox_to_anchor`` makes it, for its errors."""
    from .transforms import BboxBase, Bbox
    if bbox is None or isinstance(bbox, BboxBase):
        return
    try:
        length = len(bbox)
    except TypeError as err:
        raise ValueError(f"Invalid bbox: {bbox}") from err
    if length == 2:
        bbox = [bbox[0], bbox[1], 0, 0]
    Bbox.from_bounds(*bbox)


def check_frame(facecolor, edgecolor, linewidth, framealpha):
    """
    The width of the legend's frame, checked with its colours and alpha
    as the `.FancyBboxPatch` of matplotlib's legend checks them (the edge
    colour, then the face colour, as colours; the width as a number, as
    it keeps it; the alpha as numeric or None); the rc defaults for None.
    """
    facecolor = rcParams["legend.facecolor"] if facecolor is None else facecolor
    if facecolor == 'inherit':
        facecolor = rcParams["axes.facecolor"]
    edgecolor = rcParams["legend.edgecolor"] if edgecolor is None else edgecolor
    if edgecolor == 'inherit':
        edgecolor = rcParams["axes.edgecolor"]
    if linewidth is None:
        linewidth = rcParams["legend.linewidth"]
    colors.to_rgba(edgecolor)
    colors.to_rgba(facecolor)
    width = float(rcParams['patch.linewidth'] if linewidth is None else linewidth)
    if framealpha is not None and not isinstance(framealpha, Real):
        raise TypeError(f'alpha must be numeric or None, not {type(framealpha)}')
    return width


def check_entries(legend, handles, handler_map, ncols):
    """
    Lay the legend's entries out as matplotlib's ``_init_legend_box`` does,
    for its errors: the size of each entry's handle box from the handle
    height and length (in font sizes), the markers of a line's handle
    scaled by the legend's ``markerscale``, the entries split into *ncols*
    columns, then the paddings and spacings between them.
    """
    fontsize = legend._fontsize
    0.35 * fontsize * (legend.handleheight - 0.7)  # noqa: B018
    count = 0
    for handle in handles:
        handler = legend.get_legend_handler(handler_map, handle)
        if handler is None:
            continue
        count += 1
        legend.handlelength * fontsize  # noqa: B018
        line = _scaled_marker_line(handler, handle)
        if line is not None and legend.markerscale != 1:
            float(line.get_markersize()) * legend.markerscale
    np.array_split(np.zeros((count, 2)), ncols)
    legend.handletextpad * fontsize  # noqa: B018
    legend.labelspacing * fontsize  # noqa: B018
    legend.columnspacing * fontsize  # noqa: B018
    legend.borderpad * fontsize  # noqa: B018


def _scaled_marker_line(handler, handle):
    """The line whose marker size the handle's legend line takes and
    matplotlib's *handler* scales by the legend's ``markerscale``: the
    handle of a line's handler, the data line of an errorbar's (None when
    it draws no line); None for a handler scaling no markers (a line
    collection's) or a handle that is no line."""
    from .legend_handler import (
        HandlerErrorbar, HandlerLine2D, HandlerLine2DCompound, HandlerLineCollection)
    from .lines import Line2D
    if (isinstance(handler, HandlerLineCollection)
            or not isinstance(handler, (HandlerLine2D, HandlerLine2DCompound))):
        return None
    if isinstance(handler, HandlerErrorbar):
        handle = handle[0]
    return handle if isinstance(handle, Line2D) else None


def check_title(title_fontsize, title_fontproperties):
    """Size the font of the legend's title as matplotlib's legend sizes
    it, for its errors."""
    from .font_manager import FontProperties
    if title_fontsize is not None and title_fontproperties is not None:
        raise ValueError(
            "title_fontsize and title_fontproperties can't be specified "
            "at the same time. Only use one of them. ")
    if title_fontsize is not None:
        FontProperties().set_size(title_fontsize)


def check_labelcolor(labelcolor):
    """Check *labelcolor* as matplotlib's legend colours its labels with
    it: a name of the handles' colours, ``'none'``, colours, else none."""
    labelcolor = rcParams['legend.labelcolor'] if labelcolor is None else labelcolor
    labelcolor = rcParams['text.color'] if labelcolor is None else labelcolor
    if isinstance(labelcolor, str) and labelcolor in _COLOR_GETTERS:
        return
    if cbook._str_equal(labelcolor, 'none'):
        return
    if np.iterable(labelcolor):
        colors.to_rgba_array(labelcolor)
        return
    raise ValueError(f"Invalid labelcolor: {labelcolor!r}")


#: The names of the colours of the handles labels can take (matplotlib's
#: ``color_getters``).
_COLOR_GETTERS = ('linecolor', 'markerfacecolor', 'mfc', 'markeredgecolor', 'mec')


def held_pad(value):
    """The pad to its parent's box pltrs (Rust) gives the legend: *value*,
    or the rc's for one that is no number (`keep_borderaxespad`)."""
    return value if value is None or isinstance(value, Real) else None


def keep_borderaxespad(legend, value):
    """Keep *value*, the legend's pad to its parent's box, when it is no
    number: matplotlib's legend computes with it in display units where it
    places itself (measured or drawn)."""
    if held_pad(value) is value:
        return
    from . import _unusable
    _unusable.keep(legend, "borderaxespad", value, _place_with)


def _place_with(pad):
    """Place a legend's box with *pad* as matplotlib's legend does
    (``_findoffset``), for its errors: one there makes the box ask its
    offset again without arguments (``OffsetBox.get_window_extent``),
    which the offset's signature refuses."""
    try:
        pad * 1.0
    except TypeError:
        from .offsetbox import OffsetBox
        OffsetBox.get_offset(None)


def native_mode(mode):
    """The mode pltrs (Rust) lays the legend out in: matplotlib's legend
    expands for ``'expand'`` alone, any other mode keeping its entries at
    their width."""
    return 'expand' if isinstance(mode, str) and mode == 'expand' else None
