"""
Contours: `~.Axes.contour`, `~.Axes.contourf` and `~.Axes.clabel`.
"""

import numpy as np

from ... import _api, collections as mcoll, colors as mcolors, contour as mcontour
from ..._native import contour as _contour
from ..._preprocess import _preprocess_data
from .scatter import _mapping


def _linestyles(styles):
    """One line style or a sequence of them in native form (None stays None)."""
    if styles is None:
        return None
    single_dash = (isinstance(styles, tuple) and len(styles) == 2
                   and not isinstance(styles[0], (str, tuple)))
    if isinstance(styles, str) or single_dash:
        styles = [styles]
    return [mcoll._native_style(style) for style in styles]


def _contour_look(fn, kwargs):
    """
    The look of a contour set popped from *kwargs*: ``(look, extras,
    attributes)``, the native look and extras tuples and the attributes the
    set keeps (``linestyles``, ``negative_linestyles``).
    """
    for name in ("locator", "transform", "colorizer", "xunits", "yunits"):
        if kwargs.get(name) is not None:
            raise NotImplementedError(f"pltrs: {fn}({name}=...)")
    colors = kwargs.pop("colors", None)
    cmap = kwargs.pop("cmap", None)
    if colors is not None and cmap is not None:
        raise ValueError('Either colors or cmap must be None')
    extend = kwargs.pop("extend", "neither")
    _api.check_in_list(['neither', 'both', 'min', 'max'], extend=extend)
    hatches = kwargs.pop("hatches", [None])
    linewidths = kwargs.pop("linewidths", None)
    alpha = kwargs.pop("alpha", None)
    antialiased = kwargs.pop("antialiased", None)
    linestyles = kwargs.pop("linestyles", None)
    negative_linestyles = kwargs.pop("negative_linestyles", None)
    if negative_linestyles is None:
        from ... import rcParams
        negative_linestyles = rcParams['contour.negative_linestyle']
    look = (None if colors is None else
            [tuple(c) for c in mcolors.to_rgba_array(colors)],
            _mapping(cmap, kwargs.pop("norm", None), kwargs.pop("vmin", None),
                     kwargs.pop("vmax", None)),
            None if alpha is None else float(alpha),
            None if linewidths is None else
            [float(w) for w in np.atleast_1d(linewidths)],
            _linestyles(linestyles), _linestyles(negative_linestyles)[0])
    extras = (extend, [None if h is None else str(h) for h in hatches],
              None if antialiased is None else bool(antialiased))
    attributes = {"linestyles": linestyles,
                  "negative_linestyles": negative_linestyles}
    return look, extras, attributes


def _levels(levels):
    """Contour levels in native form: None, a count, or the values."""
    return None if levels is None else np.asarray(levels, dtype=float)


def _finish(cls, native, axes, attributes, kwargs):
    """The *cls* contour set of *native*, its *attributes* and remaining
    *kwargs* set."""
    contours = cls._from_contour(native, axes)
    for name, value in attributes.items():
        setattr(contours, name, value)
    contours._internal_update(kwargs)
    return contours


class ContoursMixin:
    """`contour`, `contourf`, `tricontour`, `tricontourf` and `clabel`."""

    def _contour(self, filled, args, kwargs):
        """The `.QuadContourSet` of ``contour`` (or, *filled*, ``contourf``)."""
        fn = 'contourf' if filled else 'contour'
        z, mask, x, y, levels = mcontour._contour_args(fn, args, kwargs)
        origin = kwargs.pop("origin", None)
        extent = kwargs.pop("extent", None)
        if origin is not None:
            _api.check_in_list([None, 'lower', 'upper', 'image'], origin=origin)
        generation = (kwargs.pop("corner_mask", None), int(kwargs.pop("nchunk", 0)),
                      kwargs.pop("algorithm", None))
        look, extras, attributes = _contour_look(fn, kwargs)
        native = _contour.contour(
            self._native, bool(filled),
            (z, mask, x, y,
             (origin, None if extent is None else tuple(map(float, extent)))),
            generation, _levels(levels), look, extras)
        return _finish(mcontour.QuadContourSet, native, self, attributes, kwargs)

    def _tricontour(self, filled, args, kwargs):
        """The `.TriContourSet` of ``tricontour`` (or, *filled*,
        ``tricontourf``)."""
        from ...tri import Triangulation, TriContourSet
        fn = 'tricontourf' if filled else 'tricontour'
        tri, args, kwargs = Triangulation.get_from_args_and_kwargs(*args, **kwargs)
        z, *args = args
        z = np.ma.asarray(z)
        if z.shape != tri.x.shape:
            raise ValueError('z array must have same length as triangulation x'
                             ' and y arrays')
        z_check = z[np.unique(tri.get_masked_triangles())]
        if np.ma.is_masked(z_check):
            raise ValueError('z must not contain masked points within the '
                             'triangulation')
        if not np.isfinite(z_check).all():
            raise ValueError('z array must not contain non-finite values '
                             'within the triangulation')
        levels = kwargs.pop("levels", None)
        if levels is None and args:
            levels = args[0]
        look, extras, attributes = _contour_look(fn, kwargs)
        native = _contour.tricontour(
            self._native, bool(filled), tri._native,
            np.ma.filled(z.astype(float), np.nan), _levels(levels), look, extras)
        return _finish(TriContourSet, native, self, attributes, kwargs)

    def tricontour(self, *args, **kwargs):
        """
        Draw contour lines on an unstructured triangular grid.

        Returns
        -------
        `~matplotlib.tri.TriContourSet`
        """
        return self._tricontour(False, args, kwargs)

    def tricontourf(self, *args, **kwargs):
        """
        Draw contour regions on an unstructured triangular grid.

        Returns
        -------
        `~matplotlib.tri.TriContourSet`
        """
        return self._tricontour(True, args, kwargs)

    def triplot(self, *args, **kwargs):
        """
        Draw an unstructured triangular grid as lines and/or markers.

        Returns
        -------
        lines : `~matplotlib.lines.Line2D`
            The drawn triangles edges.
        markers : `~matplotlib.lines.Line2D`
            The drawn marker nodes.
        """
        from ...tri import triplot
        return triplot(self, *args, **kwargs)

    @_preprocess_data()
    def contour(self, *args, **kwargs):
        """
        Plot contour lines.

        Call signature::

            contour([X, Y,] Z, /, [levels], **kwargs)

        Returns
        -------
        `~matplotlib.contour.QuadContourSet`
        """
        return self._contour(False, args, kwargs)

    @_preprocess_data()
    def contourf(self, *args, **kwargs):
        """
        Plot filled contours.

        Call signature::

            contourf([X, Y,] Z, /, [levels], **kwargs)

        Returns
        -------
        `~matplotlib.contour.QuadContourSet`
        """
        return self._contour(True, args, kwargs)

    def clabel(self, CS, levels=None, **kwargs):
        """
        Label a contour plot.

        Adds labels to line contours in given `.ContourSet`.

        Returns
        -------
        labels
            A list of `.Text` instances for the labels.
        """
        return CS.clabel(levels, **kwargs)
