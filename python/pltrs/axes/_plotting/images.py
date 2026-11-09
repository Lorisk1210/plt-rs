"""
Matrices drawn as images: `~.Axes.pcolorfast`, `~.Axes.spy` and
`~.Axes.matshow`.
"""

import numpy as np

from ... import _docstring
from ... import _api, _values, cbook, collections as mcoll, colors as mcolors
from ... import image as mimage, lines as mlines
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from .._aspect import _checked_aspect
from ._helpers import wrap
from .scatter import _mapping


def _fast_coords(args, nr, nc):
    """The ``(X, Y)`` of `~.Axes.pcolorfast`'s positional arguments (None
    without), checked as matplotlib checks them."""
    if len(args) == 1:
        return None
    if len(args) != 3:
        raise _api.nargs_error('pcolorfast', '1 or 3', len(args))
    x = np.asarray(args[0])
    y = np.asarray(args[1])
    if x.ndim == 1 and y.ndim == 1:
        if not (x.size == 2 and y.size == 2):
            if x.size != nc + 1:
                raise ValueError(
                    f"Length of X ({x.size}) must be one larger than the "
                    f"number of columns in C ({nc})")
            if y.size != nr + 1:
                raise ValueError(
                    f"Length of Y ({y.size}) must be one larger than the "
                    f"number of rows in C ({nr})"
                )
    elif not (x.ndim == 2 and y.ndim == 2):
        raise TypeError(
            f"When 3 positional parameters are passed to pcolorfast, the first "
            f"two (X and Y) must be both 1D or both 2D; the given X was "
            f"{x.ndim}D and the given Y was {y.ndim}D")
    return x.astype(float), y.astype(float)


class ImagesMixin:
    """`pcolorfast`, `spy` and `matshow`."""

    @_preprocess_data()
    @_docstring.interpd
    def pcolorfast(self, *args, alpha=None, norm=None, cmap=None, vmin=None,
                   vmax=None, colorizer=None, **kwargs):
        """
        Create a pseudocolor plot with a non-regular rectangular grid.

        Call signature::

            ax.pcolorfast([X, Y], C, /, **kwargs)

        The arguments *X*, *Y*, *C* are positional-only.

        This method is similar to `~.Axes.pcolor` and `~.Axes.pcolormesh`.
        It's designed to provide the fastest pcolor-type plotting with the
        Agg backend. To achieve this, it uses different algorithms internally
        depending on the complexity of the input grid (regular rectangular,
        non-regular rectangular or arbitrary quadrilateral).

        .. warning::

            This method is experimental. Compared to `~.Axes.pcolor` or
            `~.Axes.pcolormesh` it has some limitations:

            - It supports only flat shading (no outlines)
            - It lacks support for log scaling of the axes.
            - It does not have a pyplot wrapper.

        Parameters
        ----------
        C : array-like
            The image data. Supported array shapes are:

            - (M, N): an image with scalar data.  Color-mapping is controlled
              by *cmap*, *norm*, *vmin*, and *vmax*.
            - (M, N, 3): an image with RGB values (0-1 float or 0-255 int).
            - (M, N, 4): an image with RGBA values (0-1 float or 0-255 int),
              i.e. including transparency.

            The first two dimensions (M, N) define the rows and columns of
            the image.

            This parameter can only be passed positionally.

        X, Y : tuple or array-like, default: ``(0, N)``, ``(0, M)``
            *X* and *Y* are used to specify the coordinates of the
            quadrilaterals. There are different ways to do this:

            - Use tuples ``X=(xmin, xmax)`` and ``Y=(ymin, ymax)`` to define
              a *uniform rectangular grid*.

              The tuples define the outer edges of the grid. All individual
              quadrilaterals will be of the same size. This is the fastest
              version.

            - Use 1D arrays *X*, *Y* to specify a *non-uniform rectangular
              grid*.

              In this case *X* and *Y* have to be monotonic 1D arrays of length
              *N+1* and *M+1*, specifying the x and y boundaries of the cells.

              The speed is intermediate. Note: The grid is checked, and if
              found to be uniform the fast version is used.

            - Use 2D arrays *X*, *Y* if you need an *arbitrary quadrilateral
              grid* (i.e. if the quadrilaterals are not rectangular).

              In this case *X* and *Y* are 2D arrays with shape (M + 1, N + 1),
              specifying the x and y coordinates of the corners of the colored
              quadrilaterals.

              This is the most general, but the slowest to render.  It may
              produce faster and more compact output using ps, pdf, and
              svg backends, however.

            These arguments can only be passed positionally.

        %(cmap_doc)s

            This parameter is ignored if *C* is RGB(A).

        %(norm_doc)s

            This parameter is ignored if *C* is RGB(A).

        %(vmin_vmax_doc)s

            This parameter is ignored if *C* is RGB(A).

        %(colorizer_doc)s

            This parameter is ignored if *C* is RGB(A).

        alpha : float, default: None
            The alpha blending value, between 0 (transparent) and 1 (opaque).

        snap : bool, default: False
            Whether to snap the mesh to pixel boundaries.

        Returns
        -------
        `.AxesImage` or `.PcolorImage` or `.QuadMesh`
            The return type depends on the type of grid:

            - `.AxesImage` for a regular rectangular grid.
            - `.PcolorImage` for a non-regular rectangular grid.
            - `.QuadMesh` for a non-rectangular grid.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Supported additional parameters depend on the type of grid.
            See return types of *image* for further description.
        """
        C = args[-1]
        nr, nc = np.shape(C)[:2]
        coords = _fast_coords(args, nr, nc)
        if coords is not None and coords[0].ndim == 2 and np.ndim(C) not in {2, 3}:
            raise ValueError("C must be 2D or 3D")
        made, mapping = _mapping(cmap, norm, vmin, vmax, colorizer, variates=True)
        # matplotlib makes the artist with the alpha given, which checks it,
        # then gives it the data.
        mimage._checked_alpha(alpha)
        A = mimage._ImageBase._normalize_image_array(C)
        if A.ndim == 2:
            made._scale_norm(norm, vmin, vmax, A)
        image, pcolor_image, mesh = _plotting.pcolorfast(
            self._native, coords, mimage._native_data(A), mapping,
            None if alpha is None else float(alpha))
        if mesh is not None:
            ret = wrap(self, mesh, mcoll.QuadMesh)
            ret._attach_native_mapping(colorizer=made)
        elif image is not None:
            ret = wrap(self, image, mimage.AxesImage)
            ret._set_mapped_data(made, A)
        else:
            ret = wrap(self, pcolor_image, mimage.PcolorImage)
            ret._set_mapped_data(made, A)
        ret._internal_update(kwargs)
        return ret

    @_api.make_keyword_only("3.10", "precision")
    @_docstring.interpd
    def spy(self, Z, precision=0, marker=None, markersize=None,
            aspect='equal', origin="upper", **kwargs):
        """
        Plot the sparsity pattern of a 2D array.

        This visualizes the non-zero values of the array.

        Two plotting styles are available: image and marker. Both
        are available for full arrays, but only the marker style
        works for `scipy.sparse.spmatrix` instances.

        **Image style**

        If *marker* and *markersize* are *None*, `~.Axes.imshow` is used. Any
        extra remaining keyword arguments are passed to this method.

        **Marker style**

        If *Z* is a `scipy.sparse.spmatrix` or *marker* or *markersize* are
        *None*, a `.Line2D` object will be returned with the value of marker
        determining the marker type, and any remaining keyword arguments
        passed to `~.Axes.plot`.

        Parameters
        ----------
        Z : (M, N) array-like
            The array to be plotted.

        precision : float or 'present', default: 0
            If *precision* is 0, any non-zero value will be plotted. Otherwise,
            values of :math:`|Z| > precision` will be plotted.

            For `scipy.sparse.spmatrix` instances, you can also
            pass 'present'. In this case any value present in the array
            will be plotted, even if it is identically zero.

        aspect : {'equal', 'auto', None} or float, default: 'equal'
            The aspect ratio of the Axes.  This parameter is particularly
            relevant for images since it determines whether data pixels are
            square.

            This parameter is a shortcut for explicitly calling
            `.Axes.set_aspect`. See there for further details.

            - 'equal': Ensures an aspect ratio of 1. Pixels will be square.
            - 'auto': The Axes is kept fixed and the aspect is adjusted so
              that the data fit in the Axes. In general, this will result in
              non-square pixels.
            - *None*: Use :rc:`image.aspect`.

        origin : {'upper', 'lower'}, default: :rc:`image.origin`
            Place the [0, 0] index of the array in the upper left or lower left
            corner of the Axes. The convention 'upper' is typically used for
            matrices and images.

        Returns
        -------
        `~matplotlib.image.AxesImage` or `.Line2D`
            The return type depends on the plotting style (see above).

        Other Parameters
        ----------------
        **kwargs
            The supported additional parameters depend on the plotting style.

            For the image style, you can pass the following additional
            parameters of `~.Axes.imshow`:

            - *cmap*
            - *alpha*
            - *url*
            - any `.Artist` properties (passed on to the `.AxesImage`)

            For the marker style, you can pass any `.Line2D` property except
            for *linestyle*:

            %(Line2D:kwdoc)s
        """
        if marker is None and markersize is None and hasattr(Z, 'tocoo'):
            marker = 's'
        _api.check_in_list(["upper", "lower"], origin=origin)
        image = marker is None and markersize is None
        if image:
            if 'interpolation' in kwargs:
                raise _api.kwarg_error("spy", "interpolation")
            cmap, norm = kwargs.pop('cmap', None), kwargs.pop('norm', None)
            vmin, vmax = kwargs.pop('vmin', None), kwargs.pop('vmax', None)
            made, mapping = _mapping(cmap, norm, vmin, vmax, kwargs.pop('colorizer', None))
            if cmap is None:
                made.cmap = mcolors.ListedColormap(['w', 'k'], name='binary')
            if norm is None and vmin is None and vmax is None:
                made.norm = mcolors.NoNorm()
            line = mlines._native_kwargs({})
        else:
            if 'linestyle' in kwargs:
                raise _api.kwarg_error("spy", "linestyle")
            mapping = (None, None, None, None)
            line = mlines._native_kwargs(cbook.normalize_kwargs(kwargs, mlines.Line2D))
        if hasattr(Z, 'toarray'):
            Z = Z.toarray()
        if aspect is not None and not cbook._str_equal(aspect, 'equal'):
            # As matplotlib's `set_aspect` takes it.
            aspect = _checked_aspect(aspect)
        image, line = _plotting.spy(
            self._native, np.asarray(Z, dtype=float), float(precision), mapping,
            (None if marker is None else mlines._marker_spec(marker),
             None if markersize is None else float(markersize), aspect, origin, line))
        if image is not None:
            ret = wrap(self, image, mimage.AxesImage)
            # matplotlib's spy draws through imshow, which keeps the extent
            # it sets as a list.
            _values.remember(ret, "extent", list(ret.get_extent()))
            # matplotlib's imshow keeps the mask as a masked array, and
            # scales the norm to it.
            ret._set_mapped_data(made, cbook.safe_masked_invalid(
                np.abs(np.asarray(Z)) > precision, copy=True))
            ret.autoscale_None()
            ret._internal_update(kwargs)
            return ret
        return wrap(self, line, mlines.Line2D)

    def matshow(self, Z, **kwargs):
        """
        Plot the values of a 2D matrix or array as color-coded image.

        The matrix will be shown the way it would be printed, with the first
        row at the top.  Row and column numbering is zero-based.

        Parameters
        ----------
        Z : (M, N) array-like
            The matrix to be displayed.

        Returns
        -------
        `~matplotlib.image.AxesImage`

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.axes.Axes.imshow` arguments

        See Also
        --------
        imshow : More general function to plot data on a 2D regular raster.

        Notes
        -----
        This is just a convenience function wrapping `.imshow` to set useful
        defaults for displaying a matrix. In particular:

        - Set ``origin='upper'``.
        - Set ``interpolation='nearest'``.
        - Set ``aspect='equal'``.
        - Ticks are placed to the left and above.
        - Ticks are formatted to show integer indices.

        """
        Z = np.asanyarray(Z)
        kw = {'origin': 'upper',
              'interpolation': 'nearest',
              'aspect': 'equal',          # (already the imshow default)
              **kwargs}
        im = self.imshow(Z, **kw)
        _plotting.style_matrix_axes(self._native)
        return im
