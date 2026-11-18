"""
Pseudocolour plots: `~.Axes.pcolor` and `~.Axes.pcolormesh`.
"""

import inspect

import numpy as np

from ... import _docstring
from ... import _api, _live_transforms, cbook, collections as mcoll
from ..._native import collections as _collections, plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import convert_units, wrap
from .scatter import _mapping


def _mesh_args(axes, fn, args, shading):
    """
    ``(C, (X, Y), dtype)`` of the positional arguments ``[X, Y,] C`` of
    *fn* on *axes* drawn with *shading*: the coordinates None when absent
    (numbers of the axes' units otherwise), C masked values as NaN (the
    RGB or RGBA colours ``pcolor`` and ``pcolormesh`` take for C come as
    an ``(M, N, 4)`` array), and the dtype of the coordinates matplotlib's
    mesh keeps (see `_coordinates_dtype`).
    """
    if len(args) == 1:
        C, = args
        coords = (None, None)
        given = None
    elif len(args) == 3:
        X, Y, C = args
        X, Y = convert_units(axes, "x", X), convert_units(axes, "y", Y)
        given = (X, Y)
        coords = tuple(np.asarray(np.ma.filled(np.ma.asarray(v, dtype=float), np.nan))
                       for v in (X, Y))
    else:
        raise _api.nargs_error(fn, takes="1 or 3", given=len(args))
    if given is not None:
        _check_mesh_shape(fn, *coords, np.shape(C), shading)
    dtype = _coordinates_dtype(given, np.shape(C), shading)
    if type(C) is np.ndarray and C.ndim == 2 and C.dtype == np.float64 and C.dtype.isnative:
        # What the masking below makes of plain floats, without its copies:
        # pltrs (Rust) copies the values it keeps.
        finite = np.isfinite(C)
        return (C if finite.all() else np.where(finite, C, np.nan)), coords, dtype
    C = cbook.safe_masked_invalid(C, copy=True)
    if C.ndim == 3 and C.shape[2] in (3, 4):
        C = np.ma.filled(C.astype(float), np.nan)
        if C.shape[2] == 3:
            C = np.concatenate([C, np.ones(C.shape[:2] + (1,))], axis=2)
        return C, coords, dtype
    if C.ndim != 2:
        raise TypeError(f"Dimensions of C must be 2D, not {C.ndim}D")
    return np.ma.filled(C.astype(float), np.nan), coords, dtype


def _rc_shading():
    """:rc:`pcolor.shading`."""
    from ... import rcParams
    return rcParams['pcolor.shading']


def _check_mesh_shape(fn, X, Y, shape, shading):
    """Fail as matplotlib's ``_pcolorargs`` fails for coordinates *X* and
    *Y* that do not fit each other, or the values of *shape* with
    *shading*."""
    nrows, ncols = shape[:2]
    Nx = X.shape[-1]
    Ny = Y.shape[0]
    if X.ndim != 2 or X.shape[0] == 1:
        X = X.reshape(1, Nx).repeat(Ny, axis=0)
    if Y.ndim != 2 or Y.shape[1] == 1:
        Y = Y.reshape(Ny, 1).repeat(Nx, axis=1)
    if X.shape != Y.shape:
        raise TypeError(f'Incompatible X, Y inputs to {fn}; see help({fn})')
    if shading in (None, 'auto'):
        shading = (shading or _rc_shading()).lower()
    if shading == 'auto':
        shading = 'nearest' if (ncols, nrows) == (Nx, Ny) else 'flat'
    if shading == 'flat':
        if (Nx, Ny) != (ncols + 1, nrows + 1):
            raise TypeError(f"Dimensions of C {shape} should be one smaller than X({Nx}) "
                            f"and Y({Ny}) while using shading='flat' see help({fn})")
    elif (Nx, Ny) != (ncols, nrows):
        raise TypeError('Dimensions of C %s are incompatible with X (%d) and/or Y (%d); '
                        'see help(%s)' % (shape, Nx, Ny, fn))


def _valid_shading(shading):
    """*shading*, or 'auto' for one matplotlib does not know, which it
    warns about."""
    valid = ['gouraud', 'nearest', 'flat', 'auto']
    try:
        _api.check_in_list(valid, shading=shading)
    except ValueError:
        _api.warn_external(f"shading value '{shading}' not in list of valid values "
                           f"{valid}. Setting shading='auto'.")
        return 'auto'
    return shading


def _refuse_gouraud_cells(shape):
    """Fail as matplotlib's `~.Axes.pcolor` fails with ``shading='gouraud'``
    once its coordinates are checked to be the size of the values of
    *shape* (as for 'nearest', see `_check_mesh_shape`): its mesh of flat
    cells refuses the values as one too many along each side."""
    ny, nx = shape[:2]
    h, w = ny - 1, nx - 1
    ok_shapes = [(h, w, 3), (h, w, 4), (h, w), (h * w,)]
    raise ValueError(f"For X ({nx}) and Y ({ny}) with flat shading, A should have "
                     f"shape {' or '.join(map(str, ok_shapes))}, not {shape}")


def _coordinates_dtype(given, shape, shading):
    """
    The dtype of the coordinates of matplotlib's mesh, which keeps the
    grids its ``_pcolorargs`` makes: those of the *given* ``(X, Y)`` (the
    integers of `numpy.arange` without them), made floats where the cells
    are centred on them (shading 'nearest', or 'auto' with grids of the
    size of C, of *shape*).
    """
    shading = (shading or _rc_shading()).lower()
    if given is None:
        return np.dtype(float) if shading == 'nearest' else np.arange(1).dtype
    X, Y = (np.asarray(v) for v in given)
    dtype = np.result_type(X, Y)
    centred = (np.shape(X)[-1], np.shape(Y)[0]) == tuple(shape[1::-1])
    if shading == 'nearest' or shading == 'auto' and centred:
        return np.result_type(dtype, float)
    return dtype



def _refuse_gouraud_polygons(args):
    """
    Raise what matplotlib's ``pcolor`` raises for Gouraud shading
    (``pcolor.shading`` set to it, say): its ``_pcolorargs`` makes the grid
    of the size of C, whose polygons take the values of a grid one smaller.
    """
    C = np.asanyarray(args[-1])
    nrows, ncols = C.shape[:2]
    width, height = ncols, nrows
    if len(args) == 3:
        width, height = np.shape(args[0])[-1], np.shape(args[1])[0]
        if (width, height) != (ncols, nrows):
            raise TypeError('Dimensions of C %s are incompatible with'
                            ' X (%d) and/or Y (%d); see help(%s)' % (
                                C.shape, width, height, 'pcolor'))
    h, w = height - 1, width - 1
    ok_shapes = [(h, w, 3), (h, w, 4), (h, w), (h * w,)]
    raise ValueError(
        f"For X ({width}) and Y ({height}) with flat shading, A should have "
        f"shape {' or '.join(map(str, ok_shapes))}, not {C.shape}")


def _first_variate(values):
    """The first variate of *values*, a structured array of several (other
    arrays as they are, which the mesh then refuses as matplotlib does)."""
    if values.dtype.names:
        return values[values.dtype.names[0]].astype(float)
    return values


def _set_in_order(kwargs, cls, polygons):
    """
    *kwargs* under the names of the properties they set, as matplotlib's
    collection sets them: its constructor's own parameters first
    (``linewidths``, ``edgecolors``...), then the rest in the order given
    (``_internal_update``), so that an alias set later replaces them
    (seaborn's ``heatmap`` passes ``linewidths`` and a program's
    ``linewidth``, which wins); ``pcolor`` takes ``linewidth`` and
    ``edgecolor`` over ``linewidths`` and ``edgecolors`` first.
    """
    kwargs = dict(kwargs)
    if polygons:
        for alias, name in (("linewidth", "linewidths"), ("edgecolor", "edgecolors")):
            if alias in kwargs:
                kwargs[name] = kwargs.pop(alias)
    own = _COLLECTION_PARAMETERS
    to_property = cls._alias_to_prop
    resolved = {}
    for key, value in [*((k, v) for k, v in kwargs.items() if k in own),
                       *((k, v) for k, v in kwargs.items() if k not in own)]:
        name = to_property.get(key, key)
        resolved.pop(name, None)
        resolved[name] = value
    return resolved


#: The parameters matplotlib's `.Collection` constructor sets before the
#: other keyword arguments.
_COLLECTION_PARAMETERS = frozenset(
    inspect.signature(mcoll.Collection.__init__).parameters) - {"self", "kwargs"}

class MeshMixin:
    """`pcolor` and `pcolormesh`."""

    def _pcolor(self, fn, polygons, args, shading, mapping, cls, kwargs,
                antialiased=None):
        """The collection of `pcolor` (*polygons*) or `pcolormesh`, mapping
        its values as *mapping* (`_mapping`'s pair) says; the mesh of
        `pcolormesh` is drawn *antialiased* or not."""
        if shading is not None:
            shading = _valid_shading(shading.lower())
        made, native_mapping = mapping
        if polygons and shading is None and _rc_shading().lower() == 'gouraud':
            # Gouraud shading from :rc:`pcolor.shading`; given explicitly,
            # it fails once the coordinates are checked (below).
            _refuse_gouraud_polygons(args)
        variates = None
        if made.norm.n_components > 1 and args and not polygons:
            # Values of several variates (a structured array, as matplotlib
            # 3.11's ``pcolormesh`` takes them): the mesh is made of the
            # first variate, then maps them all through the bivariate or
            # multivariate colorizer.
            variates = np.asarray(args[-1])
            args = (*args[:-1], _first_variate(variates))
            native_mapping = (None, None, None, None)
        C, coords, dtype = _mesh_args(self, fn, args, shading)
        if polygons and shading == 'gouraud':
            _refuse_gouraud_cells(np.shape(args[-1]))
        kwargs = _set_in_order(kwargs, cls, polygons)
        if (polygons and "antialiased" not in kwargs
                and not cbook._str_lower_equal(kwargs.get("edgecolor", "none"), "none")):
            # matplotlib's pcolor turns antialiasing off only for unstroked
            # cells; stroked ones take the collections' default.
            from ... import rcParams
            kwargs["antialiased"] = rcParams["patch.antialiased"]
        # The data limits cover the mesh in the coordinates it is drawn in.
        placement = (shading, _live_transforms.given_to_native(
            kwargs.get("transform"), self))
        if C.ndim == 3:
            native = _plotting.pcolor_rgba(self._native, polygons, C, coords, placement)
        else:
            native = _plotting.pcolor(self._native, polygons, C, coords, placement,
                                      native_mapping)
        collection = wrap(self, native, cls)
        collection._coordinates_dtype = dtype
        collection._coordinates_masked = len(args) == 3
        if antialiased is not None:
            # matplotlib's mesh draws with the antialiasing it is made with,
            # apart from the collection's own (`.QuadMesh.get_antialiased`).
            _collections.set_antialiaseds(collection._native, [bool(antialiased)])
        if variates is None:
            collection._attach_native_mapping(
                colorizer=made, given=args[-1] if C.ndim == 2 else None)
            if C.ndim == 2:
                collection._keep_given_array(args[-1])
        else:
            collection._map_variates_through(made, variates)
        if C.ndim == 3:
            # The RGB(A) colours are the mesh's array, as matplotlib's mesh
            # is made with them (which autoscales its norm to them).
            collection.set_array(args[-1])
        collection._made_with(kwargs)
        kept = collection._mapped if variates is None else None
        if kept is not None and kept.dtype.kind in "iub":
            # Integers (kept as given) index the colormap through an
            # identity norm.
            _collections.set_integer_values(collection._native)
        if not polygons:
            # matplotlib's QuadMesh constructor turns mouseover off after
            # the keyword arguments set theirs.
            collection.set_mouseover(False)
        return collection

    @_preprocess_data()
    @_docstring.interpd
    def pcolor(self, *args, shading=None, alpha=None, norm=None, cmap=None,
               vmin=None, vmax=None, colorizer=None, **kwargs):
        r"""
        Create a pseudocolor plot with a non-regular rectangular grid.

        Call signature::

            pcolor([X, Y,] C, /, **kwargs)

        *X* and *Y* can be used to specify the corners of the quadrilaterals.

        The arguments *X*, *Y*, *C* are positional-only.

        .. hint::

            ``pcolor()`` can be very slow for large arrays. In most
            cases you should use the similar but much faster
            `~.Axes.pcolormesh` instead. See
            :ref:`Differences between pcolor() and pcolormesh()
            <differences-pcolor-pcolormesh>` for a discussion of the
            differences.

        Parameters
        ----------
        C : 2D array-like
            The color-mapped values.  Color-mapping is controlled by *cmap*,
            *norm*, *vmin*, and *vmax*.

        X, Y : array-like, optional
            The coordinates of the corners of quadrilaterals of a pcolormesh::

                (X[i+1, j], Y[i+1, j])       (X[i+1, j+1], Y[i+1, j+1])
                                      ●╶───╴●
                                      │     │
                                      ●╶───╴●
                    (X[i, j], Y[i, j])       (X[i, j+1], Y[i, j+1])

            Note that the column index corresponds to the x-coordinate, and
            the row index corresponds to y. For details, see the
            :ref:`Notes <axes-pcolormesh-grid-orientation>` section below.

            If ``shading='flat'`` the dimensions of *X* and *Y* should be one
            greater than those of *C*, otherwise a TypeError is raised.  The
            quadrilateral is colored due to the value at ``C[i, j]``.

            If ``shading='nearest'``, the dimensions of *X* and *Y* should be
            the same as those of *C* (if not, a TypeError will be raised). The
            color ``C[i, j]`` will be centered on ``(X[i, j], Y[i, j])``.

            If *X* and/or *Y* are 1-D arrays or column vectors they will be
            expanded as needed into the appropriate 2D arrays, making a
            rectangular grid.

        shading : {'flat', 'nearest', 'auto'}, default: :rc:`pcolor.shading`
            The fill style for the quadrilateral. Possible values:

            - 'flat': A solid color is used for each quad. The color of the
              quad (i, j), (i+1, j), (i, j+1), (i+1, j+1) is given by
              ``C[i, j]``. The dimensions of *X* and *Y* should be
              one greater than those of *C*.
            - 'nearest': Each grid point will have a color centered on it,
              extending halfway between the adjacent grid centers.  The
              dimensions of *X* and *Y* must be the same as *C*.
            - 'auto': Choose 'flat' if dimensions of *X* and *Y* are one
              larger than *C*.  Choose 'nearest' if dimensions are the same.

            See :doc:`/gallery/images_contours_and_fields/pcolormesh_grids`
            for more description.

        %(cmap_doc)s

        %(norm_doc)s

        %(vmin_vmax_doc)s

        %(colorizer_doc)s

        edgecolors : {'none', None, 'face', color, color sequence}, optional
            The color of the edges. Defaults to 'none'. Possible values:

            - 'none' or '': No edge.
            - *None*: :rc:`patch.edgecolor` will be used. Note that currently
              :rc:`patch.force_edgecolor` has to be True for this to work.
            - 'face': Use the adjacent face color.
            - A color or sequence of colors will set the edge color.

            The singular form *edgecolor* works as an alias.

        alpha : float, default: None
            The alpha blending value of the face color, between 0 (transparent)
            and 1 (opaque). Note: The edgecolor is currently not affected by
            this.

        snap : bool, default: False
            Whether to snap the mesh to pixel boundaries.

        Returns
        -------
        `matplotlib.collections.PolyQuadMesh`

        Other Parameters
        ----------------
        antialiaseds : bool, default: False
            The default *antialiaseds* is False if the default
            *edgecolors*\ ="none" is used.  This eliminates artificial lines
            at patch boundaries, and works regardless of the value of alpha.
            If *edgecolors* is not "none", then the default *antialiaseds*
            is taken from :rc:`patch.antialiased`.
            Stroking the edges may be preferred if *alpha* is 1, but will
            cause artifacts otherwise.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Additionally, the following arguments are allowed. They are passed
            along to the `~matplotlib.collections.PolyQuadMesh` constructor:

        %(PolyCollection:kwdoc)s

        See Also
        --------
        pcolormesh : for an explanation of the differences between
            pcolor and pcolormesh.
        imshow : If *X* and *Y* are each equidistant, `~.Axes.imshow` can be a
            faster alternative.

        Notes
        -----
        **Masked arrays**

        *X*, *Y* and *C* may be masked arrays. If either ``C[i, j]``, or one
        of the vertices surrounding ``C[i, j]`` (*X* or *Y* at
        ``[i, j], [i+1, j], [i, j+1], [i+1, j+1]``) is masked, nothing is
        plotted.

        .. _axes-pcolor-grid-orientation:

        **Grid orientation**

        The grid orientation follows the standard matrix convention: An array
        *C* with shape (nrows, ncolumns) is plotted with the column number as
        *X* and the row number as *Y*.
        """
        if alpha is not None:
            kwargs["alpha"] = alpha
        return self._pcolor("pcolor", True, args, shading,
                            _mapping(cmap, norm, vmin, vmax, colorizer, variates=True),
                            mcoll.PolyQuadMesh, kwargs)

    @_preprocess_data()
    @_docstring.interpd
    def pcolormesh(self, *args, alpha=None, norm=None, cmap=None, vmin=None,
                   vmax=None, colorizer=None, shading=None, antialiased=False,
                   **kwargs):
        """
        Create a pseudocolor plot with a non-regular rectangular grid.

        Call signature::

            pcolormesh([X, Y,] C, /, **kwargs)

        *X* and *Y* can be used to specify the corners of the quadrilaterals.

        The arguments *X*, *Y*, *C* are positional-only.

        .. hint::

           `~.Axes.pcolormesh` is similar to `~.Axes.pcolor`. It is much faster
           and preferred in most cases. For a detailed discussion on the
           differences see :ref:`Differences between pcolor() and pcolormesh()
           <differences-pcolor-pcolormesh>`.

        Parameters
        ----------
        C : array-like
            The mesh data. Supported array shapes are:

            - (M, N) or M*N: a mesh with scalar data. The values are mapped to
              colors using normalization and a colormap. See parameters *norm*,
              *cmap*, *vmin*, *vmax*.
            - (M, N, 3): an image with RGB values (0-1 float or 0-255 int).
            - (M, N, 4): an image with RGBA values (0-1 float or 0-255 int),
              i.e. including transparency.

            The first two dimensions (M, N) define the rows and columns of
            the mesh data.

        X, Y : array-like, optional
            The coordinates of the corners of quadrilaterals of a pcolormesh::

                (X[i+1, j], Y[i+1, j])       (X[i+1, j+1], Y[i+1, j+1])
                                      ●╶───╴●
                                      │     │
                                      ●╶───╴●
                    (X[i, j], Y[i, j])       (X[i, j+1], Y[i, j+1])

            Note that the column index corresponds to the x-coordinate, and
            the row index corresponds to y. For details, see the
            :ref:`Notes <axes-pcolormesh-grid-orientation>` section below.

            If ``shading='flat'`` the dimensions of *X* and *Y* should be one
            greater than those of *C*, otherwise a TypeError is raised. The
            quadrilateral is colored due to the value at ``C[i, j]``.

            If ``shading='nearest'`` or ``'gouraud'``, the dimensions of *X*
            and *Y* should be the same as those of *C* (if not, a TypeError
            will be raised).  For ``'nearest'`` the color ``C[i, j]`` is
            centered on ``(X[i, j], Y[i, j])``.  For ``'gouraud'``, a smooth
            interpolation is carried out between the quadrilateral corners.

            If *X* and/or *Y* are 1-D arrays or column vectors they will be
            expanded as needed into the appropriate 2D arrays, making a
            rectangular grid.

        %(cmap_doc)s

        %(norm_doc)s

        %(vmin_vmax_doc)s

        %(colorizer_doc)s

        edgecolors : {'none', None, 'face', color, color sequence}, optional
            The color of the edges. Defaults to 'none'. Possible values:

            - 'none' or '': No edge.
            - *None*: :rc:`patch.edgecolor` will be used. Note that currently
              :rc:`patch.force_edgecolor` has to be True for this to work.
            - 'face': Use the adjacent face color.
            - A color or sequence of colors will set the edge color.

            The singular form *edgecolor* works as an alias.

        alpha : float, default: None
            The alpha blending value, between 0 (transparent) and 1 (opaque).

        shading : {'flat', 'nearest', 'gouraud', 'auto'}, optional
            The fill style for the quadrilateral; defaults to
            :rc:`pcolor.shading`. Possible values:

            - 'flat': A solid color is used for each quad. The color of the
              quad (i, j), (i+1, j), (i, j+1), (i+1, j+1) is given by
              ``C[i, j]``. The dimensions of *X* and *Y* should be
              one greater than those of *C*.
            - 'nearest': Each grid point will have a color centered on it,
              extending halfway between the adjacent grid centers.  The
              dimensions of *X* and *Y* must be the same as *C*.
            - 'gouraud': Each quad will be Gouraud shaded: The color of the
              corners (i', j') are given by ``C[i', j']``. The color values of
              the area in between is interpolated from the corner values.
              The dimensions of *X* and *Y* must be the same as *C*. When
              Gouraud shading is used, *edgecolors* is ignored.
            - 'auto': Choose 'flat' if dimensions of *X* and *Y* are one
              larger than *C*.  Choose 'nearest' if dimensions are the same.

            See :doc:`/gallery/images_contours_and_fields/pcolormesh_grids`
            for more description.

        snap : bool, default: :rc:`pcolormesh.snap`
            Whether to snap the mesh to pixel boundaries.

            .. versionchanged:: 3.4.0
               The default value changed from *False* to *True* to improve transparency
               handling. See :ref:`whats-new-3-4-0` for details.
        rasterized : bool, optional
            Rasterize the pcolormesh when drawing vector graphics.  This can
            speed up rendering and produce smaller files for large data sets.
            See also :doc:`/gallery/misc/rasterization_demo`.

        Returns
        -------
        `matplotlib.collections.QuadMesh`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Additionally, the following arguments are allowed. They are passed
            along to the `~matplotlib.collections.QuadMesh` constructor:

        %(QuadMesh:kwdoc)s

        See Also
        --------
        pcolor : An alternative implementation with slightly different
            features. For a detailed discussion on the differences see
            :ref:`Differences between pcolor() and pcolormesh()
            <differences-pcolor-pcolormesh>`.
        imshow : If *X* and *Y* are each equidistant, `~.Axes.imshow` can be a
            faster alternative.

        Notes
        -----
        **Masked arrays**

        *C* may be a masked array. If ``C[i, j]`` is masked, the corresponding
        quadrilateral will be transparent. Masking of *X* and *Y* is not
        supported. Use `~.Axes.pcolor` if you need this functionality.

        .. _axes-pcolormesh-grid-orientation:

        **Grid orientation**

        The grid orientation follows the standard matrix convention: An array
        *C* with shape (nrows, ncolumns) is plotted with the column number as
        *X* and the row number as *Y*.

        .. _differences-pcolor-pcolormesh:

        **Differences between pcolor() and pcolormesh()**

        Both methods are used to create a pseudocolor plot of a 2D array
        using quadrilaterals.

        The main difference lies in the created object and internal data
        handling:
        While `~.Axes.pcolor` returns a `.PolyQuadMesh`, `~.Axes.pcolormesh`
        returns a `.QuadMesh`. The latter is more specialized for the given
        purpose and thus is faster. It should almost always be preferred.

        There is also a slight difference in the handling of masked arrays.
        Both `~.Axes.pcolor` and `~.Axes.pcolormesh` support masked arrays
        for *C*. However, only `~.Axes.pcolor` supports masked arrays for *X*
        and *Y*. The reason lies in the internal handling of the masked values.
        `~.Axes.pcolor` leaves out the respective polygons from the
        PolyQuadMesh. `~.Axes.pcolormesh` sets the facecolor of the masked
        elements to transparent. You can see the difference when using
        edgecolors. While all edges are drawn irrespective of masking in a
        QuadMesh, the edge between two adjacent masked quadrilaterals in
        `~.Axes.pcolor` is not drawn as the corresponding polygons do not
        exist in the PolyQuadMesh. Because PolyQuadMesh draws each individual
        polygon, it also supports applying hatches and linestyles to the collection.

        Another difference is the support of Gouraud shading in
        `~.Axes.pcolormesh`, which is not available with `~.Axes.pcolor`.

        """
        if alpha is not None:
            kwargs["alpha"] = alpha
        return self._pcolor("pcolormesh", False, args, shading,
                            _mapping(cmap, norm, vmin, vmax, colorizer, variates=True),
                            mcoll.QuadMesh, kwargs, antialiased)
