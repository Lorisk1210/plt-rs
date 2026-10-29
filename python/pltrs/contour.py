"""
Classes to support contour plotting and labelling for the Axes class
(matplotlib's ``contour``).

pltrs (Rust) makes the contour sets: a `ContourSet` wraps the native
contour set (``self._contour``) and, as the `.Collection` it is in
matplotlib, the collection drawing its contours (``self._native``).
"""

from numbers import Integral

import numpy as np
from numpy import ma

from . import _api, _handles, collections as mcoll
from ._native import contour as _contour
from .path import Path


class ContourLabeler:
    """Mixin to provide labelling capability to `.ContourSet`."""

    def clabel(self, levels=None, *,
               fontsize=None, inline=True, inline_spacing=5, fmt=None,
               colors=None, use_clabeltext=False, manual=False,
               rightside_up=True, zorder=None):
        """
        Label a contour plot.

        Returns
        -------
        labels
            A list of `.Text` instances for the labels.
        """
        from .text import Text
        if fmt is not None and isinstance(fmt, dict):
            fmt = [(float(level), str(text)) for level, text in fmt.items()]
        elif fmt is not None and not isinstance(fmt, str) and not callable(fmt):
            raise NotImplementedError(f"pltrs: clabel(fmt={type(fmt).__name__})")
        if manual is True:
            raise NotImplementedError("pltrs: clabel(manual=True)")
        if isinstance(fontsize, str):
            from .font_manager import FontProperties
            fontsize = FontProperties(size=fontsize).get_size_in_points()
        if colors is not None:
            from .colors import to_rgba_array
            colors = [tuple(c) for c in to_rgba_array(colors)]
        texts = _contour.clabel(
            self._contour,
            (None if levels is None else [float(lev) for lev in levels],
             None if fontsize is None else float(fontsize), bool(inline),
             float(inline_spacing), colors,
             None if manual is False else [tuple(map(float, xy)) for xy in manual]),
            fmt, (bool(rightside_up), None if zorder is None else float(zorder),
                  bool(use_clabeltext)))
        return _handles.wrap_all(texts, Text, self.figure)

    @property
    def labelTexts(self):
        """The label `.Text` instances."""
        from .text import Text
        return _handles.wrap_all(_contour.label_texts(self._contour), Text,
                                 self.figure)


class ContourSet(ContourLabeler, mcoll.Collection):
    """
    Store a set of contour lines or filled regions.

    User-callable method: `~.Axes.clabel`
    """

    @classmethod
    def _from_contour(cls, contour, axes):
        """The object around the native contour set *contour* of *axes*."""
        collections = _contour.get_collections(contour)
        cs = _handles.wrap(collections[0], cls, axes.figure)
        cs._contour = contour
        return cs

    @property
    def levels(self):
        """The contour levels."""
        return _contour.get_levels(self._contour)

    @property
    def layers(self):
        """The values the levels or bands are coloured by."""
        return _contour.get_layers(self._contour)

    @property
    def filled(self):
        """Whether the contours are filled."""
        return _contour.get_state(self._contour)[0]

    @property
    def zmin(self):
        """The smallest value of the data."""
        return _contour.get_state(self._contour)[1]

    @property
    def zmax(self):
        """The largest value of the data."""
        return _contour.get_state(self._contour)[2]

    @property
    def extend(self):
        """Which ends the colors extend: 'neither', 'both', 'min' or 'max'."""
        return _contour.get_state(self._contour)[3]

    def get_paths(self):
        """Return the paths: one compound path per level or band."""
        return [Path(vertices, codes)
                for vertices, codes in _contour.get_paths(self._contour)]

    allsegs = property(lambda self: [
        [subp.vertices for subp in p._iter_connected_components()]
        for p in self.get_paths()])
    allkinds = property(lambda self: [
        [subp.codes for subp in p._iter_connected_components()]
        for p in self.get_paths()])

    def get_linewidth(self):
        """Return the line widths of the levels."""
        return np.asarray(_contour.get_linewidths(self._contour))

    get_linewidths = get_linewidth

    def set_linewidth(self, lw):
        """Set the line widths of the levels."""
        _contour.set_linewidths(self._contour, [float(w) for w in np.atleast_1d(lw)])

    set_linewidths = set_linewidth

    def get_edgecolor(self):
        """Return the colors of the levels (line contours)."""
        return _contour.get_colors(self._contour)

    get_edgecolors = get_edgecolor

    def get_clim(self):
        """Return the values mapped to the colormap limits."""
        return _contour.get_clim(self._contour)

    def set_clim(self, vmin=None, vmax=None):
        """Set the norm limits."""
        if vmax is None:
            try:
                vmin, vmax = vmin
            except (TypeError, ValueError):
                pass
        _contour.set_clim(self._contour, None if vmin is None else float(vmin),
                          None if vmax is None else float(vmax))

    def find_nearest_contour(self, x, y, indices=None, pixel=True):
        """
        Find the point in the unfilled contour plot that is closest to
        ``(x, y)``.

        Returns
        -------
        path : int
            The index of the path that is closest to ``(x, y)``.
        subpath : int
            The index within that closest path of the subpath that is
            closest to ``(x, y)``.
        index : int
            The index of the vertices within that subpath that are closest
            to ``(x, y)``.
        xmin, ymin : float
            The point in the contour plot that is closest to ``(x, y)``.
        d2 : float
            The squared distance from ``(xmin, ymin)`` to ``(x, y)``.
        """
        if indices is not None:
            indices = [int(i) for i in np.atleast_1d(indices)]
        nearest = _contour.find_nearest_contour(
            self._contour, (float(x), float(y)), indices, bool(pixel))
        if nearest is None:
            return (None, None, None, None, None, None)
        level, segment, index, (xmin, ymin), d2 = nearest
        return (level, segment, index, xmin, ymin, d2)

    def changed(self):
        """Call this whenever the mappable is changed to notify listeners."""

    def remove(self):
        """Remove the contour set, its collections and its labels."""
        _contour.remove(self._contour)
        _handles.forget(self)


class QuadContourSet(ContourSet):
    """
    Create and store a set of contour lines or filled regions.

    This class is typically not instantiated directly by the user but by
    `~.Axes.contour` and `~.Axes.contourf`.
    """


def _contour_args(fn, args, kwargs):
    """
    The field and levels of ``contour``/``contourf``'s positional
    arguments ``[X, Y,] Z [, levels]``, checked as matplotlib checks them:
    ``(z, mask, x, y, levels)``.
    """
    nargs = len(args)
    if 0 < nargs <= 2:
        z, *args = args
        z = ma.asarray(z)
        x = y = None
    elif 2 < nargs <= 4:
        x, y, z_orig, *args = args
        x = np.asarray(x, dtype=np.float64)
        y = np.asarray(y, dtype=np.float64)
        z = ma.asarray(z_orig)
        if z.ndim != 2:
            raise TypeError(f"Input z must be 2D, not {z.ndim}D")
        if z.shape[0] < 2 or z.shape[1] < 2:
            raise TypeError(f"Input z must be at least a (2, 2) shaped array, "
                            f"but has shape {z.shape}")
        Ny, Nx = z.shape
        if x.ndim != y.ndim:
            raise TypeError(f"Number of dimensions of x ({x.ndim}) and y "
                            f"({y.ndim}) do not match")
        if x.ndim == 1:
            if x.shape[0] != Nx:
                raise TypeError(f"Length of x ({x.shape[0]}) must match number of "
                                f"columns in z ({Nx})")
            if y.shape[0] != Ny:
                raise TypeError(f"Length of y ({y.shape[0]}) must match number of "
                                f"rows in z ({Ny})")
        elif x.ndim == 2:
            if x.shape != z.shape:
                raise TypeError(
                    f"Shapes of x {x.shape} and z {z.shape} do not match")
            if y.shape != z.shape:
                raise TypeError(
                    f"Shapes of y {y.shape} and z {z.shape} do not match")
        else:
            raise TypeError(f"Inputs x and y must be 1D or 2D, not {x.ndim}D")
    else:
        raise _api.nargs_error(fn, takes="from 1 to 4", given=nargs)
    if z.ndim != 2:
        raise TypeError(f"Input z must be 2D, not {z.ndim}D")
    if z.shape[0] < 2 or z.shape[1] < 2:
        raise TypeError(f"Input z must be at least a (2, 2) shaped array, "
                        f"but has shape {z.shape}")
    levels = kwargs.pop("levels", None)
    if levels is None and args:
        levels = args[0]
    if np.issubdtype(z.dtype, bool) and levels is None:
        levels = [0, .5, 1] if fn == "contourf" else [.5]
    z = ma.masked_invalid(z.astype(float), copy=False)
    if levels is not None and not isinstance(levels, Integral):
        levels = np.asarray(levels, np.float64)
        if fn == "contourf" and len(levels) < 2:
            raise ValueError("Filled contours require at least 2 levels.")
        if len(levels) > 1 and np.min(np.diff(levels)) <= 0.0:
            raise ValueError("Contour levels must be increasing")
    mask = ma.getmask(z)
    return (np.ma.filled(z, np.nan).astype(float),
            None if mask is ma.nomask or not mask.any() else np.asarray(mask, bool),
            x, y, levels)


__all__ = ["ContourLabeler", "ContourSet", "QuadContourSet"]
