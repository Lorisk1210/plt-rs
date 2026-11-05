"""
Contours: `~.Axes.contour`, `~.Axes.contourf` and `~.Axes.clabel`.
"""

import math

import numpy as np

from ... import (_api, _handles, _live_transforms, _tick_helpers, _values, artist as martist,
                 collections as mcoll, colors as mcolors)
from ... import contour as mcontour
from ..._native import contour as _contour, tri as _tri
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


def _contour_look(kwargs):
    """
    The look of a contour set popped from *kwargs*: ``(look, extras,
    attributes, colorizer, native_cmap)``, the native look and extras
    tuples, the attributes the set keeps (``_orig_linestyles``,
    ``negative_linestyles``), the Python `.Colorizer` of its colour mapping,
    and whether its colormap is the one pltrs (Rust) makes (no *cmap* is
    given: the default one, or the listed colormap of *colors*).
    """
    if "color" in kwargs:
        raise _api.kwarg_error("ContourSet.__init__", "color")
    from ... import ticker
    locator = kwargs.pop("locator", None)
    given_colors = colors = kwargs.pop("colors", None)
    cmap = kwargs.pop("cmap", None)
    if colors is not None and cmap is not None:
        raise ValueError('Either colors or cmap must be None')
    if colors is not None and mcolors.is_color_like(colors):
        colors = [colors]  # one colour, 'none' included
    # matplotlib keeps any *extend* and extends the ends it names.
    extend = kwargs.pop("extend", "neither")
    native_extend = extend if extend in ('neither', 'both', 'min', 'max') else 'neither'
    hatches = kwargs.pop("hatches", (None,))
    norm = kwargs.get("norm")
    if kwargs.get("colorizer") is not None:
        norm = kwargs["colorizer"].norm
    logscale = (isinstance(norm, mcolors.LogNorm)
                or isinstance(locator, ticker.LogLocator))
    linewidths = kwargs.pop("linewidths", None)
    alpha = kwargs.pop("alpha", None)
    antialiased = kwargs.pop("antialiased", None)
    linestyles = kwargs.pop("linestyles", None)
    negative_linestyles = kwargs.pop("negative_linestyles", None)
    if negative_linestyles is None:
        from ... import rcParams
        negative_linestyles = rcParams['contour.negative_linestyle']
    made, mapping = _mapping(cmap, kwargs.pop("norm", None), kwargs.pop("vmin", None),
                             kwargs.pop("vmax", None), kwargs.pop("colorizer", None))
    look = (None if colors is None else
            [tuple(c) for c in mcolors.to_rgba_array(colors)],
            mapping,
            None if alpha is None else float(alpha),
            None if linewidths is None else
            [float(w) for w in np.atleast_1d(linewidths)],
            _linestyles(linestyles), _linestyles(negative_linestyles)[0])
    extras = (native_extend, [None if h is None else str(h) for h in hatches],
              None if antialiased is None else bool(antialiased),
              None if locator is None
              else _tick_helpers._description("locator", locator),
              _live_transforms.given_to_native(kwargs.pop("transform", None)))
    attributes = {"_given_linewidths": linewidths,
                  "_orig_linestyles": linestyles,
                  "negative_linestyles": negative_linestyles, "_extend": extend,
                  "colors": given_colors, "hatches": hatches,
                  "logscale": logscale, "origin": None, "extent": None,
                  "nchunk": 0}
    return look, extras, attributes, made, cmap is None


def _levels(levels):
    """Contour levels in native form: None, a count, or the values."""
    return None if levels is None else np.asarray(levels, dtype=float)


def _finish(cls, native, axes, look, kwargs, made=None):
    """The *cls* contour set of *native* (or *made*, a contour set being
    made, around it), mapping its levels through the colorizer of *look*
    (`_contour_look`), its attributes and remaining *kwargs* set."""
    if made is None:
        made = cls._from_contour(native, axes)
    else:
        _handles.adopt(made, _contour.get_collections(native)[0], axes.figure)
        made._take_contour(native, axes)
    contours = _style(made, look, kwargs)
    # matplotlib's contour sets autoscale the view when made
    # (``autoscale_view(tight=True)``), where pltrs (Rust) waits.
    axes._unstale_viewLim()
    return contours


def _style(contours, look, kwargs):
    """Map the levels of *contours* through the colorizer of *look*
    (`_contour_look`) and set its attributes and remaining *kwargs*;
    return it."""
    from ...cm import _from_native
    _, _, attributes, colorizer, native_cmap = look
    attributes = dict(attributes)
    widths = attributes.pop("_given_linewidths")
    if native_cmap:
        colorizer.cmap = _from_native(contours._native_cmap())
    contours._attach_native_mapping(colorizer=colorizer)
    for name, value in attributes.items():
        setattr(contours, name, value)
    if widths is not None and not contours.filled:
        # The widths of the levels as matplotlib's ``_process_linewidths``
        # gives them, cycled as given.
        levels = len(contours.levels)
        widths = (list(widths) * math.ceil(levels / len(widths)) if np.iterable(widths)
                  else [widths] * levels)[:levels]
        _values.remember(contours, "linewidth", np.atleast_1d(widths))
    contours.monochrome = contours.cmap.monochrome
    contours._internal_update(kwargs)
    return contours


class ContoursMixin:
    """`contour`, `contourf`, `tricontour`, `tricontourf` and `clabel`."""

    def _contour(self, filled, args, kwargs, made=None):
        """The `.QuadContourSet` of ``contour`` (or, *filled*, ``contourf``):
        a new one, or *made*, the set whose constructor contours."""
        fn = 'contourf' if filled else 'contour'
        # matplotlib's contour set takes the alpha first, checking it.
        martist._check_array_alpha(kwargs.get("alpha"))
        origin = kwargs.pop("origin", None)
        extent = kwargs.pop("extent", None)
        if origin is not None:
            _api.check_in_list([None, 'lower', 'upper', 'image'], origin=origin)
        generation = (kwargs.pop("corner_mask", None), int(kwargs.pop("nchunk", 0)),
                      kwargs.pop("algorithm", None))
        if args and isinstance(args[0], mcontour.QuadContourSet):
            # The field of that contour set is contoured again, at its
            # levels unless others are given (matplotlib's _process_args).
            generator = args[0]._generator
            levels = kwargs.pop("levels", None)
            if levels is None:
                levels = args[0].levels
        else:
            if 2 < len(args) <= 4:
                x, y, *rest = args
                args = (*self._process_unit_info([("x", x), ("y", y)], kwargs), *rest)
            z, mask, x, y, levels = mcontour._contour_args(fn, args, kwargs)
            generator = mcontour._ContourGenerator(
                (z, mask, x, y,
                 (origin, None if extent is None else tuple(map(float, extent)))),
                generation)
        look = _contour_look(kwargs)
        native = _contour.contour(self._native, bool(filled), *generator.field,
                                  _levels(levels), *look[:2])
        contours = _finish(mcontour.QuadContourSet, native, self, look, kwargs, made)
        contours._generator = generator
        if origin == 'image':
            from ... import rcParams
            origin = rcParams['image.origin']
        contours.origin, contours.extent = origin, extent
        contours.nchunk = generation[1]
        return contours

    def _tricontour(self, filled, args, kwargs):
        """The `.TriContourSet` of ``tricontour`` (or, *filled*,
        ``tricontourf``)."""
        from ...tri import Triangulation, TriContourSet
        from ...tri._tricontour import _TriContourGenerator
        levels = kwargs.pop("levels", None)
        if args and isinstance(args[0], TriContourSet):
            # The field of that contour set is contoured again, at its
            # levels unless others are given (matplotlib's _process_args).
            generator = args[0]._generator
            if levels is None:
                levels = args[0].levels
        else:
            tri, args, kwargs = Triangulation.get_from_args_and_kwargs(*args, **kwargs)
            z, *args = args
            z = np.ma.asarray(z)
            if z.shape != tri.x.shape:
                raise ValueError('z array must have same length as triangulation x'
                                 ' and y arrays')
            # np.unique(tri.get_masked_triangles()), found natively.
            z_check = z[_tri.used_points(tri._native)]
            if np.ma.is_masked(z_check):
                raise ValueError('z must not contain masked points within the '
                                 'triangulation')
            if not np.isfinite(z_check).all():
                raise ValueError('z array must not contain non-finite values '
                                 'within the triangulation')
            if levels is None and args:
                levels = args[0]
            if levels is None and np.issubdtype(z.dtype, bool):
                levels = [0, .5, 1] if filled else [.5]
            generator = _TriContourGenerator(
                tri, np.ma.filled(z.astype(float), np.nan))
        tri, z = generator.field
        look = _contour_look(kwargs)
        native = _contour.tricontour(self._native, bool(filled), tri._native, z,
                                     _levels(levels), *look[:2])
        contours = _finish(TriContourSet, native, self, look, kwargs)
        contours._generator = generator
        return contours

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

    def tripcolor(self, *args, alpha=1.0, norm=None, cmap=None, vmin=None,
                  vmax=None, shading='flat', facecolors=None, **kwargs):
        """
        Create a pseudocolor plot of an unstructured triangular grid.

        Returns
        -------
        `~matplotlib.collections.PolyCollection` or `~matplotlib.collections.TriMesh`
        """
        from ...tri import tripcolor
        return tripcolor(self, *args, alpha=alpha, norm=norm, cmap=cmap,
                         vmin=vmin, vmax=vmax, shading=shading,
                         facecolors=facecolors, **kwargs)

    @_preprocess_data()
    def contour(self, *args, **kwargs):
        """
        Plot contour lines.

        Call signature::

            contour([X, Y,] Z, /, [levels], **kwargs)

        Returns
        -------
        `~matplotlib.contour.QuadContourSet`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
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

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
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
