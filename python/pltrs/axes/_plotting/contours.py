"""
Contours: `~.Axes.contour`, `~.Axes.contourf` and `~.Axes.clabel`.
"""

import math

import numpy as np

from ... import _docstring
from ... import (_api, _handles, _lazy_imports, _live_transforms, _tick_helpers,
                 _values, artist as martist, cbook, collections as mcoll, colors as mcolors)
from ... import contour as mcontour
from ..._native import contour as _contour, tri as _tri
from ..._preprocess import _preprocess_data
from ... import tri as mtri
from ._helpers import _make_axes_method
from .scatter import _mapping

_contour_arguments = _lazy_imports.module(globals(), "..._contour_arguments")
_contour_setters = _lazy_imports.module(globals(), "..._contour_setters")


def _linestyles(styles):
    """One line style or a sequence of them in native form (None stays None)."""
    if styles is None:
        return None
    single_dash = (isinstance(styles, tuple) and len(styles) == 2
                   and not isinstance(styles[0], (str, tuple)))
    if isinstance(styles, str) or single_dash:
        styles = [styles]
    return [mcoll._native_style(style) for style in styles]


def _contour_look(axes, kwargs):
    """
    The look of a contour set of *axes* popped from *kwargs*: ``(look, extras,
    attributes, colorizer, native_cmap)``, the native look and extras
    tuples, the attributes the set keeps (``_orig_linestyles``,
    ``negative_linestyles``), the Python `.Colorizer` of its colour mapping,
    and whether its colormap is the one pltrs (Rust) makes (no *cmap* is
    given: the default one, or the listed colormap of *colors*).
    """
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
    if logscale and norm is None:
        # matplotlib's set maps the levels of a `.LogLocator` through a
        # `.LogNorm` of its own, given the limits.
        kwargs["norm"] = mcolors.LogNorm(kwargs.pop("vmin", None), kwargs.pop("vmax", None))
    linewidths = kwargs.pop("linewidths", None)
    alpha = kwargs.pop("alpha", None)
    antialiased = kwargs.pop("antialiased", None)
    linestyles = kwargs.pop("linestyles", None)
    negative_linestyles = kwargs.pop("negative_linestyles", None)
    if negative_linestyles is None:
        from ... import rcParams
        negative_linestyles = rcParams['contour.negative_linestyle']
    colorizer = kwargs.pop("colorizer", None)
    made, mapping = _mapping(cmap, kwargs.pop("norm", None), kwargs.pop("vmin", None),
                             kwargs.pop("vmax", None), colorizer)
    if colors is not None and cmap is None and colorizer is None:
        # The colours given map the levels, not the default colormap.
        mapping = (None, *mapping[1:])
    look = (None if colors is None else
            # A colormap of no colours maps every level to transparent black.
            [tuple(c) for c in mcolors.to_rgba_array(colors)] or [(0., 0., 0., 0.)],
            mapping,
            None if alpha is None else float(alpha),
            _contour_arguments.held_linewidths(linewidths),
            _contour_arguments.held_linestyles(_linestyles, linestyles),
            _contour_arguments.held_linestyles(
                lambda styles: _linestyles(styles)[0],
                # matplotlib dashes negative levels when the colormap says
                # it is monochrome; pltrs (Rust) judges by the colours,
                # which a colormap of the program's may not follow
                # (ultraplot's of one colour is not).
                negative_linestyles if colors is not None or made.cmap.monochrome
                else "solid"))
    given_transform = kwargs.pop("transform", None)
    transform = _live_transforms.for_axes(
        _contour_arguments.held_transform(given_transform), axes)
    extras = (native_extend,
              [None if h is None else str(h) for h in hatches] if np.iterable(hatches)
              else [None],
              None if antialiased is None else bool(antialiased),
              _tick_helpers._description("locator", locator)
              if isinstance(locator, ticker.Locator) else None,
              _live_transforms.given_to_native(transform, holder=_live_transforms.root_figure(axes)),
              logscale)
    attributes = {"_given_linewidths": linewidths,
                  "_orig_linestyles": linestyles,
                  "negative_linestyles": negative_linestyles, "_extend": extend,
                  "colors": given_colors, "hatches": hatches,
                  "logscale": logscale, "locator": locator, "origin": None, "extent": None,
                  "nchunk": 0}
    if transform is not None:
        # The transform object itself, as matplotlib's set keeps it (its
        # ``transform_path_non_affine``: cartopy's cuts paths at a map's
        # edge, which its limits read).
        attributes["_given_transform"] = transform
    elif given_transform is not None:
        # No transform: kept as given, failing where the set is drawn.
        attributes["_given_transform"] = given_transform
    return look, extras, attributes, made, cmap is None


def _levels(levels):
    """Contour levels in native form: None, a count, or the values."""
    return None if levels is None else np.asarray(levels, dtype=float)


def _finish(cls, native, axes, look, kwargs, made=None, levels=None):
    """The *cls* contour set of *native* (or *made*, a contour set being
    made, around it), mapping its levels through the colorizer of *look*
    (`_contour_look`), its attributes and remaining *kwargs* set, its
    ``locator`` the one given or the one that chose its *levels* (given as
    a count, or none)."""
    if made is None:
        made = cls._from_contour(native, axes)
    else:
        _handles.adopt(made, _contour.get_collections(native)[0], axes.figure)
        made._take_contour(native, axes)
    contours = _style(made, look, kwargs)
    contours._keep_locator(contours.locator, levels)
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
        cmap = _from_native(contours._native_cmap())
        if attributes["colors"] is not None:
            cmap = _given_colormap(cmap, attributes["colors"])
        colorizer.cmap = cmap
    contours._attach_native_mapping(colorizer=colorizer)
    for name, value in attributes.items():
        setattr(contours, name, value)
    if (widths is not None and not contours.filled
            and _contour_arguments.held_linewidths(widths) is not None):
        # The widths of the levels as matplotlib's ``_process_linewidths``
        # gives them, cycled as given.
        levels = len(contours.levels)
        widths = (list(widths) * math.ceil(levels / len(widths)) if np.iterable(widths)
                  else [widths] * levels)[:levels]
        _values.remember(contours, "linewidth", np.atleast_1d(widths))
    contours.monochrome = contours.cmap.monochrome
    if not contours.filled:
        _contour_arguments.check_linestyles(contours, contours._orig_linestyles)
    _contour_arguments.keep_unheld(contours, contours.__dict__.get("_given_transform"),
                                   widths)
    # A class of the program's sees the setters matplotlib's constructor
    # calls, before and after the keywords it sets last.
    _contour_setters.replay_making(contours)
    contours._made_with(kwargs)
    _contour_setters.replay_adding(contours)
    return contours


def _given_colormap(cmap, colors):
    """
    *cmap*, the listed colormap pltrs (Rust) made of the contour colours
    *colors*, as matplotlib's contour set makes it: holding the colours as
    given, cycled to its size (the first left out when it is the colour
    under the lowest level), and its extremes.
    """
    sequence = [colors] if mcolors.is_color_like(colors) else colors
    first = 0 if cmap._rgba_under is None else 1
    return mcolors.ListedColormap(
        cbook._resize_sequence(sequence[first:], cmap.N),
        under=cmap._rgba_under, over=cmap._rgba_over)


class ContoursMixin:
    """`contour`, `contourf`, `tricontour`, `tricontourf` and `clabel`."""

    def _contour(self, filled, args, kwargs, made=None):
        """The `.QuadContourSet` of ``contour`` (or, *filled*, ``contourf``):
        a new one, or *made*, the set whose constructor contours."""
        fn = 'contourf' if filled else 'contour'
        # matplotlib's methods give the set ``filled`` themselves, whatever
        # the program gave (getdist passes it on).
        kwargs.pop('filled', None)
        # matplotlib's contour set takes the alpha first, checking it.
        martist._check_array_alpha(kwargs.get("alpha"))
        _contour_arguments.check_made(kwargs)
        origin = kwargs.pop("origin", None)
        extent = kwargs.pop("extent", None)
        if origin is not None:
            _api.check_in_list([None, 'lower', 'upper', 'image'], origin=origin)
        _contour_arguments.check_extent(extent)
        nchunk = kwargs.pop("nchunk", 0)
        chunks = _contour_arguments.held_chunks(nchunk)
        corner_mask, algorithm = kwargs.pop("corner_mask", None), kwargs.pop("algorithm", None)
        if args and isinstance(args[0], mcontour.QuadContourSet):
            # The field of that contour set is contoured again, at its
            # levels unless others are given (matplotlib's _process_args).
            generator = args[0]._generator
            settings = args[0]._algorithm, args[0]._corner_mask
            levels = kwargs.pop("levels", None)
            if levels is None:
                levels = args[0].levels
        else:
            settings = mcontour._generation_settings(algorithm, corner_mask)
            generation = (settings[1], chunks or 0, settings[0])
            if 2 < len(args) <= 4:
                x, y, *rest = args
                args = (*self._process_unit_info([("x", x), ("y", y)], kwargs), *rest)
            z, mask, x, y, levels = mcontour._contour_args(fn, args, kwargs)
            generator = mcontour._ContourGenerator(
                (z, mask, x, y,
                 (origin, None if extent is None else tuple(map(float, extent)))),
                generation)
            if chunks is None:
                generator.check_chunks(nchunk, args[2] if len(args) > 2 else args[0])
            _contour_arguments.check_grid_transform(kwargs.get("transform"), self)
        _contour_arguments.check_locator(kwargs.get("locator"), levels)
        look = _contour_look(self, kwargs)
        native = _contour.contour(self._native, bool(filled), *generator.field,
                                  _levels(levels), *look[:2])
        contours = _finish(mcontour.QuadContourSet, native, self, look, kwargs, made, levels)
        contours._generator = generator
        contours._algorithm, contours._corner_mask = settings
        if origin == 'image':
            from ... import rcParams
            origin = rcParams['image.origin']
        contours.origin, contours.extent = origin, extent
        contours.nchunk = nchunk
        return contours

    def _tricontour(self, filled, args, kwargs, made=None):
        """The `.TriContourSet` of ``tricontour`` (or, *filled*,
        ``tricontourf``): a new one, or *made*, the set whose constructor
        contours."""
        from ...tri import Triangulation, TriContourSet
        from ...tri._tricontour import _TriContourGenerator
        # matplotlib's contour set takes the alpha first, checking it.
        martist._check_array_alpha(kwargs.get("alpha"))
        _contour_arguments.check_made(kwargs)
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
        look = _contour_look(self, kwargs)
        native = _contour.tricontour(self._native, bool(filled), tri._native, z,
                                     _levels(levels), *look[:2])
        contours = _finish(TriContourSet, native, self, look, kwargs, made, levels)
        contours._generator = generator
        return contours

    # matplotlib's functions of the `.tri` module, as methods.
    tricontour = _make_axes_method(mtri.tricontour)
    tricontourf = _make_axes_method(mtri.tricontourf)
    tripcolor = _make_axes_method(mtri.tripcolor)
    triplot = _make_axes_method(mtri.triplot)

    @_preprocess_data()
    @_docstring.interpd
    def contour(self, *args, **kwargs):
        """
        Plot contour lines.

        Call signature::

            contour([X, Y,] Z, /, [levels], **kwargs)

        The arguments *X*, *Y*, *Z* are positional-only.
        %(contour_doc)s
        """
        contours = self._contour(False, args, kwargs)
        # matplotlib's set autoscales the view when it is made, then
        # ``contour`` asks for it again at the next draw.
        self._request_autoscale_view()
        return contours

    @_preprocess_data()
    @_docstring.interpd
    def contourf(self, *args, **kwargs):
        """
        Plot filled contours.

        Call signature::

            contourf([X, Y,] Z, /, [levels], **kwargs)

        The arguments *X*, *Y*, *Z* are positional-only.
        %(contour_doc)s
        """
        contours = self._contour(True, args, kwargs)
        # matplotlib's set autoscales the view when it is made, then
        # ``contourf`` asks for it again at the next draw.
        self._request_autoscale_view()
        return contours

    def clabel(self, CS, levels=None, **kwargs):
        """
        Label a contour plot.

        Adds labels to line contours in given `.ContourSet`.

        Parameters
        ----------
        CS : `.ContourSet` instance
            Line contours to label.

        levels : array-like, optional
            A list of level values, that should be labeled. The list must be
            a subset of ``CS.levels``. If not given, all levels are labeled.

        **kwargs
            All other parameters are documented in `~.ContourLabeler.clabel`.
        """
        return CS.clabel(levels, **kwargs)
