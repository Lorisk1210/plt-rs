"""
Pseudocolour plots: `~.Axes.pcolor` and `~.Axes.pcolormesh`.
"""

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


def _coordinates_dtype(given, shape, shading):
    """
    The dtype of the coordinates of matplotlib's mesh, which keeps the
    grids its ``_pcolorargs`` makes: those of the *given* ``(X, Y)`` (the
    integers of `numpy.arange` without them), made floats where the cells
    are centred on them (shading 'nearest', or 'auto' with grids of the
    size of C, of *shape*).
    """
    from ... import rcParams
    shading = (shading or rcParams['pcolor.shading']).lower()
    if given is None:
        return np.dtype(float) if shading == 'nearest' else np.arange(1).dtype
    X, Y = (np.asarray(v) for v in given)
    dtype = np.result_type(X, Y)
    centred = (np.shape(X)[-1], np.shape(Y)[0]) == tuple(shape[1::-1])
    if shading == 'nearest' or shading == 'auto' and centred:
        return np.result_type(dtype, float)
    return dtype



def _first_variate(values):
    """The first variate of *values*, a structured array of several (other
    arrays as they are, which the mesh then refuses as matplotlib does)."""
    if values.dtype.names:
        return values[values.dtype.names[0]].astype(float)
    return values

class MeshMixin:
    """`pcolor` and `pcolormesh`."""

    def _pcolor(self, fn, polygons, args, shading, mapping, cls, kwargs,
                antialiased=None):
        """The collection of `pcolor` (*polygons*) or `pcolormesh`, mapping
        its values as *mapping* (`_mapping`'s pair) says; the mesh of
        `pcolormesh` is drawn *antialiased* or not."""
        if shading is not None:
            _api.check_in_list(['flat', 'nearest', 'gouraud', 'auto'],
                               shading=shading.lower())
            shading = shading.lower()
        made, native_mapping = mapping
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
        kwargs = cbook.normalize_kwargs(kwargs, cls)
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
        if antialiased is not None:
            # matplotlib's mesh draws with the antialiasing it is made with,
            # apart from the collection's own (`.QuadMesh.get_antialiased`).
            _collections.set_antialiaseds(collection._native, [bool(antialiased)])
        if variates is None:
            collection._attach_native_mapping(colorizer=made)
            if C.ndim == 2:
                collection._keep_given_array(args[-1])
        else:
            collection._map_variates_through(made, variates)
        if C.ndim == 3:
            # The RGB(A) colours are the mesh's array, as matplotlib's mesh
            # is made with them (which autoscales its norm to them).
            collection.set_array(args[-1])
        collection._internal_update(kwargs)
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
