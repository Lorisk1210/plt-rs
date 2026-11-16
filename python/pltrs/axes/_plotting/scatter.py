"""
`~.Axes.scatter`: markers of varying size and color.
"""

import numpy as np

from ... import _docstring
from ... import (_api, _live_transforms, _values, cbook, collections as mcoll,
                colors as mcolors, lines as mlines)
from ... import colorizer as mcolorizer
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...artist import _check_array_alpha
from ...markers import MarkerStyle
from ...transforms import Transform
from ._helpers import floats, wrap


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


def _mapping(cmap, norm, vmin, vmax, colorizer=None, variates=False):
    """
    The colour mapping of a plotting method: ``(colorizer, native)``, the
    Python `.Colorizer` the artist keeps and the mapping pltrs (Rust) is
    given, ``(cmap, norm, vmin, vmax)``: the colormap (None for
    :rc:`image.cmap`), the norm (an instance, a scale name or None) and
    the limits. A bivariate or multivariate colormap is refused unless
    the method maps several *variates* (``pcolormesh``, or a method that
    refuses their values itself).
    """
    mcolorizer._ScalarMappable._check_exclusionary_keywords(
        colorizer, vmin=vmin, vmax=vmax)
    made = mcolorizer._ScalarMappable._get_colorizer(cmap, norm, colorizer)
    if made.norm.n_components > 1 and not variates:
        # matplotlib's method sets the colormap on the colorizer of one
        # variate its artist was made with, which refuses it.
        mcolorizer.Colorizer().cmap = cmap
    if norm is not None and not isinstance(norm, str) and (
            vmin is not None or vmax is not None):
        raise ValueError(
            "Passing a Normalize instance simultaneously with "
            "vmin/vmax is not supported.  Please pass vmin/vmax "
            "directly to the norm when creating it.")
    if colorizer is not None:
        cmap, norm = made.cmap, made.norm
    elif cmap is not None or not _rc_cmap_found_natively():
        cmap = made.cmap
    return made, (cmap, norm, None if vmin is None else float(vmin),
                  None if vmax is None else float(vmax))


def _rc_cmap_found_natively():
    """Whether pltrs (Rust) finds :rc:`image.cmap` by its name: the registry
    ``matplotlib.colormaps`` is the package's, which it mirrors, not one a
    program put in its place (ultraplot's colormap database), whose
    colormap is then handed to it as resolved."""
    return getattr(mcolorizer._mpl().colormaps, "_native_mirror", False)


class ScatterMixin:
    """`scatter`."""

    @staticmethod
    def _parse_scatter_color_args(c, edgecolors, kwargs, xsize,
                                  get_next_color_func):
        """
        Helper function to process color related arguments of `.Axes.scatter`.

        Argument precedence for facecolors:

        - c (if not None)
        - kwargs['facecolor']
        - kwargs['facecolors']
        - kwargs['color'] (==kwcolor)
        - 'b' if in classic mode else the result of ``get_next_color_func()``

        Argument precedence for edgecolors:

        - kwargs['edgecolor']
        - edgecolors (is an explicit kw argument in scatter())
        - kwargs['color'] (==kwcolor)
        - 'face' if not in classic mode else None

        Parameters
        ----------
        c : :mpltype:`color` or array-like or list of :mpltype:`color` or None
            See argument description of `.Axes.scatter`.
        edgecolors : :mpltype:`color` or sequence of color or {'face', 'none'} or None
            See argument description of `.Axes.scatter`.
        kwargs : dict
            Additional kwargs. If these keys exist, we pop and process them:
            'facecolors', 'facecolor', 'edgecolor', 'color'
            Note: The dict is modified by this function.
        xsize : int
            The size of the x and y arrays passed to `.Axes.scatter`.
        get_next_color_func : callable
            A callable that returns a color. This color is used as facecolor
            if no other color is provided.

            Note, that this is a function rather than a fixed color value to
            support conditional evaluation of the next color.  As of the
            current implementation obtaining the next color from the
            property cycle advances the cycle. This must only happen if we
            actually use the color, which will only be decided within this
            method.

        Returns
        -------
        c
            The input *c* if it was not *None*, else a color derived from the
            other inputs or defaults.
        colors : array(N, 4) or None
            The facecolors as RGBA values, or *None* if a colormap is used.
        edgecolors
            The edgecolor.

        """
        facecolors = kwargs.pop('facecolors', None)
        facecolors = kwargs.pop('facecolor', facecolors)
        edgecolors = kwargs.pop('edgecolor', edgecolors)

        kwcolor = kwargs.pop('color', None)

        if kwcolor is not None and c is not None:
            raise ValueError("Supply a 'c' argument or a 'color'"
                             " kwarg but not both; they differ but"
                             " their functionalities overlap.")

        if kwcolor is not None:
            try:
                mcolors.to_rgba_array(kwcolor)
            except ValueError as err:
                raise ValueError(
                    "'color' kwarg must be a color or sequence of color "
                    "specs.  For a sequence of values to be color-mapped, use "
                    "the 'c' argument instead.") from err
            if edgecolors is None:
                edgecolors = kwcolor
            if facecolors is None:
                facecolors = kwcolor

        if edgecolors is None and not _rcParams()['_internal.classic_mode']:
            edgecolors = _rcParams()['scatter.edgecolors']

        # Raise a warning if both `c` and `facecolor` are set (issue #24404).
        if c is not None and facecolors is not None:
            _api.warn_external(
                "You passed both c and facecolor/facecolors for the markers. "
                "c has precedence over facecolor/facecolors. "
                "This behavior may change in the future."
            )

        c_was_none = c is None
        if c is None:
            c = (facecolors if facecolors is not None
                 else "b" if _rcParams()['_internal.classic_mode']
                 else get_next_color_func())
        c_is_string_or_strings = (
            isinstance(c, str)
            or (np.iterable(c) and len(c) > 0
                and isinstance(cbook._safe_first_finite(c), str)))

        def invalid_shape_exception(csize, xsize):
            return ValueError(
                f"'c' argument has {csize} elements, which is inconsistent "
                f"with 'x' and 'y' with size {xsize}.")

        c_is_mapped = False  # Unless proven otherwise below.
        valid_shape = True  # Unless proven otherwise below.
        if not c_was_none and kwcolor is None and not c_is_string_or_strings:
            try:  # First, does 'c' look suitable for value-mapping?
                c = np.asanyarray(c, dtype=float)
            except ValueError:
                pass  # Failed to convert to float array; must be color specs.
            else:
                # handle the documented special case of a 2D array with 1
                # row which as RGB(A) to broadcast.
                if c.shape == (1, 4) or c.shape == (1, 3):
                    c_is_mapped = False
                    if c.size != xsize:
                        valid_shape = False
                # If c can be either mapped values or an RGB(A) color, prefer
                # the former if shapes match, the latter otherwise.
                elif c.size == xsize:
                    c = c.ravel()
                    c_is_mapped = True
                else:  # Wrong size; it must not be intended for mapping.
                    if c.shape in ((3,), (4,)):
                        _api.warn_external(
                            "*c* argument looks like a single numeric RGB or "
                            "RGBA sequence, which should be avoided as value-"
                            "mapping will have precedence in case its length "
                            "matches with *x* & *y*.  Please use the *color* "
                            "keyword-argument or provide a 2D array "
                            "with a single row if you intend to specify "
                            "the same RGB or RGBA value for all points.")
                    valid_shape = False
        if not c_is_mapped:
            try:  # Is 'c' acceptable as PathCollection facecolors?
                colors = mcolors.to_rgba_array(c)
            except (TypeError, ValueError) as err:
                if "RGBA values should be within 0-1 range" in str(err):
                    raise
                else:
                    if not valid_shape:
                        raise invalid_shape_exception(c.size, xsize) from err
                    # Both the mapping *and* the RGBA conversion failed: pretty
                    # severe failure => one may appreciate a verbose feedback.
                    raise ValueError(
                        f"'c' argument must be a color, a sequence of colors, "
                        f"or a sequence of numbers, not {c!r}") from err
            else:
                if len(colors) not in (0, 1, xsize):
                    # NB: remember that a single color is also acceptable.
                    # Besides *colors* will be an empty array if c == 'none'.
                    raise invalid_shape_exception(len(colors), xsize)
        else:
            colors = None  # use cmap, norm after collection is created
        return c, colors, edgecolors


    @_api.make_keyword_only("3.10", "marker")
    @_preprocess_data(replace_names=["x", "y", "s", "linewidths",
                                     "edgecolors", "c", "facecolor",
                                     "facecolors", "color"],
                      label_namer="y")
    @_docstring.interpd
    def scatter(self, x, y, s=None, c=None, marker=None, cmap=None, norm=None,
                vmin=None, vmax=None, alpha=None, linewidths=None, *,
                edgecolors=None, colorizer=None, plotnonfinite=False, **kwargs):
        """
        A scatter plot of *y* vs. *x* with varying marker size and/or color.

        Parameters
        ----------
        x, y : float or array-like, shape (n, )
            The data positions.

        s : float or array-like, shape (n, ), optional
            The marker size in points**2 (typographic points are 1/72 in.).
            Default is ``rcParams['lines.markersize'] ** 2``.

            The linewidth and edgecolor can visually interact with the marker
            size, and can lead to artifacts if the marker size is smaller than
            the linewidth.

            If the linewidth is greater than 0 and the edgecolor is anything
            but *'none'*, then the effective size of the marker will be
            increased by half the linewidth because the stroke will be centered
            on the edge of the shape.

            To eliminate the marker edge either set *linewidth=0* or
            *edgecolor='none'*.

        c : array-like or list of :mpltype:`color` or :mpltype:`color`, optional
            The marker colors. Possible values:

            - A scalar or sequence of n numbers to be mapped to colors using
              *cmap* and *norm*.
            - A 2D array in which the rows are RGB or RGBA.
            - A sequence of colors of length n.
            - A single color format string.

            Note that *c* should not be a single numeric RGB or RGBA sequence
            because that is indistinguishable from an array of values to be
            colormapped. If you want to specify the same RGB or RGBA value for
            all points, use a 2D array with a single row.  Otherwise,
            value-matching will have precedence in case of a size matching with
            *x* and *y*.

            If you wish to specify a single color for all points
            prefer the *color* keyword argument.

            Defaults to `None`. In that case the marker color is determined
            by the value of *color*, *facecolor* or *facecolors*. In case
            those are not specified or `None`, the marker color is determined
            by the next color of the ``Axes``' current "shape and fill" color
            cycle. This cycle defaults to :rc:`axes.prop_cycle`.

        marker : `~.markers.MarkerStyle`, default: :rc:`scatter.marker`
            The marker style. *marker* can be either an instance of the class
            or the text shorthand for a particular marker.
            See :mod:`matplotlib.markers` for more information about marker
            styles.

        %(cmap_doc)s

            This parameter is ignored if *c* is RGB(A).

        %(norm_doc)s

            This parameter is ignored if *c* is RGB(A).

        %(vmin_vmax_doc)s

            This parameter is ignored if *c* is RGB(A).

        alpha : float, default: None
            The alpha blending value, between 0 (transparent) and 1 (opaque).

        linewidths : float or array-like, default: :rc:`lines.linewidth`
            The linewidth of the marker edges. Note: The default *edgecolors*
            is 'face'. You may want to change this as well.

        edgecolors : {'face', 'none', *None*} or :mpltype:`color` or list of \
:mpltype:`color`, default: :rc:`scatter.edgecolors`
            The edge color of the marker. Possible values:

            - 'face': The edge color will always be the same as the face color.
            - 'none': No patch boundary will be drawn.
            - A color or sequence of colors.

            For non-filled markers, *edgecolors* is ignored. Instead, the color
            is determined like with 'face', i.e. from *c*, *colors*, or
            *facecolors*.

        %(colorizer_doc)s

            This parameter is ignored if *c* is RGB(A).

        plotnonfinite : bool, default: False
            Whether to plot points with nonfinite *c* (i.e. ``inf``, ``-inf``
            or ``nan``). If ``True`` the points are drawn with the *bad*
            colormap color (see `.Colormap.set_bad`).

        Returns
        -------
        `~matplotlib.collections.PathCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        **kwargs : `~matplotlib.collections.PathCollection` properties
            %(PathCollection:kwdoc)s

        See Also
        --------
        plot : To plot scatter plots when markers are identical in size and
            color.

        Notes
        -----
        * The `.plot` function will be faster for scatterplots where markers
          don't vary in size or color.

        * Any or all of *x*, *y*, *s*, and *c* may be masked arrays, in which
          case all masks will be combined and only unmasked points will be
          plotted.

        * Fundamentally, scatter works with 1D arrays; *x*, *y*, *s*, and *c*
          may be input as N-D arrays, but within scatter they will be
          flattened. The exception is *c*, which will be flattened only if its
          size matches the size of *x* and *y*.

        """
        if edgecolors is not None:
            kwargs.update({'edgecolors': edgecolors})
        if linewidths is not None:
            kwargs.update({'linewidths': linewidths})
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.Collection)
        linewidths = kwargs.pop('linewidth', None)
        edgecolors = kwargs.pop('edgecolor', None)
        offset_transform = kwargs.pop('transform', None)
        x, y = self._process_unit_info([("x", x), ("y", y)], kwargs)
        x = np.ma.ravel(cbook._unpack_to_numpy(x))
        y = np.ma.ravel(cbook._unpack_to_numpy(y))
        if x.size != y.size:
            raise ValueError("x and y must be the same size")
        if s is None:
            s = (20 if _rcParams()['_internal.classic_mode'] else
                 _rcParams()['lines.markersize'] ** 2.0)
        s = np.ma.ravel(s)
        if not (np.issubdtype(s.dtype, np.floating)
                or np.issubdtype(s.dtype, np.integer)):
            raise ValueError(f"s must be float, but has type {s.dtype}")
        if len(s) not in (1, x.size):
            raise ValueError(f"s (size {len(s)}) cannot be broadcast "
                             f"to match x and y (size {len(x)})")
        orig_edgecolor = edgecolors
        c, colors, edgecolors = self._parse_scatter_color_args(
            c, edgecolors, kwargs, x.size,
            get_next_color_func=lambda: _plotting.next_patch_color(self._native))
        # As matplotlib's: a point masked or not finite in one argument is
        # masked in all of them, its value left out of the colour scaling.
        if plotnonfinite and colors is None:
            c = np.ma.masked_invalid(c)
            x, y, s, edgecolors, linewidths = cbook._combine_masks(
                x, y, s, edgecolors, linewidths)
        else:
            x, y, s, c, colors, edgecolors, linewidths = cbook._combine_masks(
                x, y, s, c, colors, edgecolors, linewidths)
        if (x.size in (3, 4) and np.ma.is_masked(edgecolors)
                and not np.ma.is_masked(orig_edgecolor)):
            edgecolors = edgecolors.data
        marker = marker if marker is not None else _rcParams()['scatter.marker']
        marker_obj = marker if isinstance(marker, MarkerStyle) else MarkerStyle(marker)
        if cbook._str_equal(marker_obj.get_marker(), ","):
            _api.warn_external(
                "The pixel maker ',' is not supported on scatter(); using "
                "a finite-sized square instead, which is not necessarily 1 pixel in "
                "size. Use the square marker 's' instead to suppress this warning."
            )
        if not marker_obj.is_filled() and orig_edgecolor is not None:
            _api.warn_external(
                f"You passed an edgecolor/edgecolors ({orig_edgecolor!r}) "
                f"for an unfilled marker ({marker!r}).  Matplotlib is "
                "ignoring the edgecolor in favor of the facecolor.  This "
                "behavior may change in the future."
            )
        made = None
        if colors is None:
            made, mapping = _mapping(cmap, norm, vmin, vmax, colorizer)
            faces = (None, (np.ma.filled(np.ma.asarray(c, dtype=float), np.nan).ravel(),
                            mapping))
        else:
            extra_kwargs = {
                'cmap': cmap, 'norm': norm, 'vmin': vmin, 'vmax': vmax
            }
            extra_keys = [k for k, v in extra_kwargs.items() if v is not None]
            if any(extra_keys):
                keys_str = ", ".join(f"'{k}'" for k in extra_keys)
                _api.warn_external(
                    "No data for colormapping provided via 'c'. "
                    f"Parameters {keys_str} will be ignored")
            faces = (np.asarray(colors, dtype=float).reshape(-1, 4), None)
        if edgecolors is not None and not cbook._str_equal(edgecolors, 'face'):
            edgecolors = mcolors.to_rgba_array(edgecolors)
        given_linewidths = None
        # The widths as the collection's `set_linewidth` keeps them (an
        # integer width reads back as one).
        widths_as_given = None if linewidths is None else np.atleast_1d(linewidths)
        if linewidths is not None:
            try:
                linewidths = [float(lw) if lw is not None
                              else _rcParams()['lines.linewidth']
                              for lw in np.ma.getdata(np.atleast_1d(linewidths))]
            except (TypeError, ValueError):
                # Widths that are no numbers: the collection keeps them
                # (its `set_linewidth`), as matplotlib's does.
                given_linewidths, linewidths = linewidths, None
        if alpha is not None:
            # As the collection's `set_alpha` checks it.
            _check_array_alpha(alpha)
        alphas = None
        if np.iterable(alpha):
            alphas, alpha = [float(a) for a in alpha], None
        # The collection's `set_sizes` (numpy warns of a negative area).
        mcoll._scale_of(s)
        native = _plotting.scatter(
            self._native, (floats(x), floats(y)),
            np.ma.filled(s.astype(float), np.nan), faces,
            mlines._marker_spec(marker_obj), (edgecolors, linewidths),
            (None if alpha is None else float(alpha), alphas),
            (bool(plotnonfinite), _live_transforms.given_to_native(offset_transform, self)))
        collection = wrap(self, native, mcoll.PathCollection)
        if (isinstance(offset_transform, Transform)
                and getattr(offset_transform, "_live", None) is None):
            # Kept as given, as matplotlib's collection keeps it, and asked
            # where matplotlib asks it if the program's code computes it.
            collection._offset_transform = offset_transform
            _live_transforms.ask_when_drawn(collection)
        # matplotlib's scatter gives its marker path as a tuple, its sizes as
        # given, and its face colours as an array (a face, even if clear).
        collection._paths_container = tuple
        # (The marker's path transformed, whose codes are the marker's.)
        collection._paths_read_only = "codes"
        _values.remember(collection, "sizes", np.asarray(s))
        collection._face_fill = True
        if alphas is not None:
            # The array of alphas comes back, as `set_alpha` keeps it.
            collection._array_alpha = np.asarray(alphas)
        if made is not None:
            collection._attach_native_mapping(colorizer=made)
        if given_linewidths is not None or (widths_as_given is not None
                                            and not len(widths_as_given)):
            # Widths no number reads, or none, which renderers fail on.
            collection.set_linewidth(widths_as_given if given_linewidths is None
                                     else given_linewidths)
        elif widths_as_given is not None:
            _values.remember(collection, "linewidth", widths_as_given)
        if (isinstance(offset_transform, _live_transforms.Transform)
                and getattr(offset_transform, "_live", None) is None):
            # The collection keeps the transform given, as matplotlib's
            # constructor sets it, and follows it where pltrs (Rust) took
            # parts of it by their matrix.
            collection._offset_transform = offset_transform
            _live_transforms.watch_given(collection, offset_transform, "offset")
        collection._made_with(kwargs)
        return collection
