"""
Drawing triangular grids as lines and markers (matplotlib's
``tri._triplot``, kept close to the original).
"""

import numpy as np

from .. import cbook, lines as mlines
from .._native import plotting as _plotting
from ._triangulation import Triangulation


def triplot(ax, *args, **kwargs):
    """
    Draw an unstructured triangular grid as lines and/or markers.

    Call signatures::

      triplot(triangulation, ...)
      triplot(x, y, [triangles], *, [mask=mask], ...)

    The triangular grid can be specified either by passing a `.Triangulation`
    object as the first parameter, or by passing the points *x*, *y* and
    optionally the *triangles* and a *mask*. If neither of *triangulation* or
    *triangles* are given, the triangulation is calculated on the fly.

    Returns
    -------
    lines : `~matplotlib.lines.Line2D`
        The drawn triangles edges.
    markers : `~matplotlib.lines.Line2D`
        The drawn marker nodes.
    """
    tri, args, kwargs = Triangulation.get_from_args_and_kwargs(*args, **kwargs)
    x, y, edges = (tri.x, tri.y, tri.edges)

    # Decode plot format string, e.g., 'ro-'
    fmt = args[0] if args else ""
    linestyle, marker, color = _plotting.format_parts(fmt)

    # Insert plot format string into a copy of kwargs (kwargs values prevail).
    kw = cbook.normalize_kwargs(kwargs, mlines.Line2D)
    for key, val in zip(('linestyle', 'marker', 'color'),
                        (linestyle, marker, color)):
        if val is not None:
            kw.setdefault(key, val)

    # Draw lines without markers: nan values separate the edges in one line.
    linestyle = kw['linestyle']
    kw_lines = {
        **kw,
        'marker': 'None',  # No marker to draw.
        'zorder': kw.get('zorder', 1),  # Path default zorder is used.
    }
    if linestyle not in [None, 'None', '', ' ']:
        tri_lines_x = np.insert(x[edges], 2, np.nan, axis=1)
        tri_lines_y = np.insert(y[edges], 2, np.nan, axis=1)
        tri_lines = ax.plot(tri_lines_x.ravel(), tri_lines_y.ravel(),
                            **kw_lines)
    else:
        tri_lines = ax.plot([], [], **kw_lines)

    # Draw markers separately.
    marker = kw['marker']
    kw_markers = {
        **kw,
        'linestyle': 'None',  # No line to draw.
    }
    kw_markers.pop('label', None)
    if marker not in [None, 'None', '', ' ']:
        tri_markers = ax.plot(x, y, **kw_markers)
    else:
        tri_markers = ax.plot([], [], **kw_markers)

    return tri_lines + tri_markers
