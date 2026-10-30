"""
The image module supports basic image loading, rescaling and display
operations (matplotlib's ``image``).

The image artists wrap the native handles of the images pltrs (Rust)
draws: an image keeps its data and its colorizer, as matplotlib's do, and
hands pltrs (Rust) a copy of them whenever they change; pltrs (Rust)
resamples and colour-maps the image as matplotlib's ``_make_image`` does.
Images are read and written with Pillow, as matplotlib does (``imread``,
``imsave``, ``pil_to_array`` and the validation of image arrays are
matplotlib's, from matplotlib 3.11.2, under the Matplotlib License).
"""

import logging
import os
from pathlib import Path

import numpy as np
import PIL.Image
import PIL.PngImagePlugin

from . import _api, _image, cbook, colorizer as mcolorizer, colors as mcolors
# For user convenience, the names from _image are also imported into
# the image namespace
from ._image import *  # noqa: F401, F403
from ._native import image as _native_image
from .transforms import Affine2D

_log = logging.getLogger(__name__)

# map interpolation strings to module constants
_interpd_ = {
    'auto': _image.NEAREST,  # this will use nearest or Hanning...
    'none': _image.NEAREST,  # fall back to nearest when not supported
    'nearest': _image.NEAREST,
    'bilinear': _image.BILINEAR,
    'bicubic': _image.BICUBIC,
    'spline16': _image.SPLINE16,
    'spline36': _image.SPLINE36,
    'hanning': _image.HANNING,
    'hamming': _image.HAMMING,
    'hermite': _image.HERMITE,
    'kaiser': _image.KAISER,
    'quadric': _image.QUADRIC,
    'catrom': _image.CATROM,
    'gaussian': _image.GAUSSIAN,
    'bessel': _image.BESSEL,
    'mitchell': _image.MITCHELL,
    'sinc': _image.SINC,
    'lanczos': _image.LANCZOS,
    'blackman': _image.BLACKMAN,
    'antialiased': _image.NEAREST,  # this will use nearest or Hanning...
}

interpolations_names = set(_interpd_)


def _rc(key):
    """The rcParam *key* (imported when first needed)."""
    from . import rcParams
    return rcParams[key]


def _val_or_rc(val, key):
    """*val*, or the rcParam *key* when *val* is None."""
    return _rc(key) if val is None else val


def _native_data(A):
    """
    The image array *A* (as `_ImageBase._normalize_image_array` leaves it)
    as pltrs (Rust) takes it: ``(kind, values, mask)``.
    """
    if A.ndim == 3:
        depth = 'rgb' if A.shape[-1] == 3 else 'rgba'
        if A.dtype == np.uint8:
            return depth + '8', np.ma.getdata(A), None
        if A.dtype == np.float32:
            # matplotlib resamples single-precision colours in single
            # precision.
            return depth + '32', np.ma.getdata(A), None
        return depth, np.ma.getdata(A).astype(float), None
    values = np.ma.getdata(A)
    mask = np.ma.getmaskarray(A) if np.ma.is_masked(A) else None
    if values.dtype == bool:
        kind = 'boolean'
    elif np.issubdtype(values.dtype, np.integer):
        kind = 'integer'
    elif values.dtype.itemsize <= 4:
        kind = 'single'
    elif values.dtype.itemsize <= 8:
        kind = 'double'
    else:
        # Wider floats (float128) are resampled in double precision, as
        # matplotlib resamples them, with its warning when drawn.
        kind, values = values.dtype.name, values.astype(np.float64)
    return kind, values, mask


def _warn_identical_limits(axes, extent):
    """
    Warn, as matplotlib's ``set_xlim`` and ``set_ylim`` do, when the image
    *extent* gives an axis of *axes* that follows its data identical
    limits (which pltrs (Rust) expands).
    """
    left, right, bottom, top = extent
    follows = _native_image.autoscale_on(axes._native)
    for name, on, low, high in (('x', follows[0], left, right),
                                ('y', follows[1], bottom, top)):
        if on and low == high:
            _api.warn_external(
                f"Attempting to set identical low and high {name}lims "
                f"makes transformation singular; automatically expanding.")


def _checked_alpha(alpha):
    """
    The opacity of an image as matplotlib accepts it (its
    ``Artist._set_alpha_for_array`` and ``_ImageBase.set_alpha``): None, a
    float or a 2D array, within [0, 1].
    """
    if isinstance(alpha, str):
        raise TypeError("alpha must be numeric or None, not a string")
    if np.iterable(alpha):
        alpha = np.asarray(alpha, dtype=float)
        if not (0 <= alpha.min() and alpha.max() <= 1):
            raise ValueError('alpha must be between 0 and 1, inclusive, '
                             f'but min is {alpha.min()}, max is {alpha.max()}')
    elif alpha is not None and not 0 <= alpha <= 1:
        raise ValueError(f'alpha ({alpha}) is outside 0-1 range')
    if np.ndim(alpha) not in (0, 2):
        raise TypeError('alpha must be a float, two-dimensional '
                        'array, or None')
    return alpha


class _ImageBase(mcolorizer.ColorizingArtist):
    """
    Base class for images.

    *interpolation* and *cmap* default to their rc settings.

    *cmap* is a `.colors.Colormap` instance.
    *norm* is a `.colors.Normalize` instance to map luminance to 0-1.

    *extent* is a ``(left, right, bottom, top)`` tuple in data coordinates, for
    making image plots registered with data plots; the default is to label the
    pixel centers with the zero-based row and column indices.

    Additional kwargs are `.Artist` properties.
    """

    zorder = 0

    def _set_mapped_data(self, colorizer, A):
        """Attach the image pltrs (Rust) made to its *colorizer* and its
        data *A*."""
        self._attach_colorizer(colorizer)
        self._A = A

    def changed(self):
        """
        Call this whenever the mappable is changed so observers can update.
        """
        _native_image.set_mapping(self._native, self.cmap, self.norm)
        super().changed()

    def get_size(self):
        """Return the size of the image as tuple (numrows, numcols)."""
        return self.get_shape()[:2]

    def get_shape(self):
        """
        Return the shape of the image as tuple (numrows, numcols, channels).
        """
        if self._A is None:
            raise RuntimeError('You must first set the image array')

        return self._A.shape

    def set_alpha(self, alpha):
        """
        Set the alpha value used for blending - not supported on all backends.

        Parameters
        ----------
        alpha : float or 2D array-like or None
        """
        alpha = _checked_alpha(alpha)
        self._alpha_array = alpha if np.ndim(alpha) == 2 else None
        _native_image.set_image_alpha(self._native, alpha)

    def get_alpha(self):
        """Return the alpha value used for blending: a float, a 2D array,
        or None."""
        alpha = getattr(self, '_alpha_array', None)
        return super().get_alpha() if alpha is None else alpha

    def _get_scalar_alpha(self):
        """
        Get a scalar alpha value to be applied to the artist as a whole.

        If the alpha value is a matrix, the method returns 1.0 because pixels
        have individual alpha values (see `~._ImageBase._make_image` for
        details). If the alpha value is a scalar, the method returns said value
        to be applied to the artist as a whole because pixels do not have
        individual alpha values.
        """
        alpha = self.get_alpha()
        return 1.0 if alpha is None or np.ndim(alpha) > 0 else alpha

    @staticmethod
    def _normalize_image_array(A):
        """
        Check validity of image-like input *A* and normalize it to a format suitable for
        Image subclasses.
        """
        A = cbook.safe_masked_invalid(A, copy=True)
        if A.dtype != np.uint8 and not np.can_cast(A.dtype, float, "same_kind"):
            raise TypeError(f"Image data of dtype {A.dtype} cannot be "
                            f"converted to float")
        if A.ndim == 3 and A.shape[-1] == 1:
            A = A.squeeze(-1)  # If just (M, N, 1), assume scalar and apply colormap.
        if not (A.ndim == 2 or A.ndim == 3 and A.shape[-1] in [3, 4]):
            raise TypeError(f"Invalid shape {A.shape} for image data")
        if A.ndim == 3:
            # If the input data has values outside the valid range (after
            # normalisation), we issue a warning and then clip X to the bounds
            # - otherwise casting wraps extreme values, hiding outliers and
            # making reliable interpretation impossible.
            high = 255 if np.issubdtype(A.dtype, np.integer) else 1
            if A.min() < 0 or high < A.max():
                _log.warning(
                    'Clipping input data to the valid range for imshow with '
                    'RGB data ([0..1] for floats or [0..255] for integers). '
                    'Got range [%s..%s].',
                    A.min(), A.max()
                )
                A = np.clip(A, 0, high)
            # Cast unsupported integer types to uint8
            if A.dtype != np.uint8 and np.issubdtype(A.dtype, np.integer):
                A = A.astype(np.uint8)
        return A

    def set_data(self, A):
        """
        Set the image array.

        Note that this function does *not* update the normalization used.

        Parameters
        ----------
        A : array-like or `PIL.Image.Image`
        """
        if isinstance(A, PIL.Image.Image):
            A = pil_to_array(A)  # Needed e.g. to apply png palette.
        self._A = self._normalize_image_array(A)
        _native_image.set_data(self._native, _native_data(self._A))

    def set_array(self, A):
        """
        Retained for backwards compatibility - use set_data instead.

        Parameters
        ----------
        A : array-like
        """
        # This also needs to be here to override the inherited
        # cm.ScalarMappable.set_array method so it is not invoked by mistake.
        self.set_data(A)

    def get_interpolation(self):
        """
        Return the interpolation method the image uses when resizing.

        One of 'auto', 'antialiased', 'nearest', 'bilinear', 'bicubic',
        'spline16', 'spline36', 'hanning', 'hamming', 'hermite', 'kaiser',
        'quadric', 'catrom', 'gaussian', 'bessel', 'mitchell', 'sinc', 'lanczos',
        or 'none'.
        """
        return _native_image.get_interpolation(self._native)

    def set_interpolation(self, s):
        """
        Set the interpolation method the image uses when resizing.

        If None, use :rc:`image.interpolation`. If 'none', the image is
        shown as is without interpolating. 'none' is only supported in
        agg, ps and pdf backends and will fall back to 'nearest' mode
        for other backends.

        Parameters
        ----------
        s : {'auto', 'nearest', 'bilinear', 'bicubic', 'spline16', \
'spline36', 'hanning', 'hamming', 'hermite', 'kaiser', 'quadric', 'catrom', \
'gaussian', 'bessel', 'mitchell', 'sinc', 'lanczos', 'none'} or None
        """
        s = _val_or_rc(s, 'image.interpolation').lower()
        _api.check_in_list(interpolations_names, interpolation=s)
        _native_image.set_interpolation(self._native, s)

    def get_interpolation_stage(self):
        """
        Return when interpolation happens during the transform to RGBA.

        One of 'data', 'rgba', 'auto'.
        """
        return _native_image.get_interpolation_stage(self._native)

    def set_interpolation_stage(self, s):
        """
        Set when interpolation happens during the transform to RGBA.

        Parameters
        ----------
        s : {'data', 'rgba', 'auto'}, default: :rc:`image.interpolation_stage`
            Whether to apply resampling interpolation in data or RGBA space.
            If 'auto', 'rgba' is used if the upsampling rate is less than 3,
            otherwise 'data' is used.
        """
        s = _val_or_rc(s, 'image.interpolation_stage')
        _api.check_in_list(['data', 'rgba', 'auto'], s=s)
        _native_image.set_interpolation_stage(self._native, s)

    def set_resample(self, v):
        """
        Set whether image resampling is used.

        Parameters
        ----------
        v : bool, default: :rc:`image.resample`
        """
        v = _val_or_rc(v, 'image.resample')
        _native_image.set_resample(self._native, bool(v))

    def get_resample(self):
        """Return whether image resampling is used."""
        return _native_image.get_resample(self._native)

    def set_filternorm(self, filternorm):
        """
        Set whether the resize filter normalizes the weights.

        See help for `~.Axes.imshow`.

        Parameters
        ----------
        filternorm : bool
        """
        _native_image.set_filternorm(self._native, bool(filternorm))

    def get_filternorm(self):
        """Return whether the resize filter normalizes the weights."""
        return _native_image.get_filternorm(self._native)

    def set_filterrad(self, filterrad):
        """
        Set the resize filter radius only applicable to some
        interpolation schemes -- see help for imshow

        Parameters
        ----------
        filterrad : positive float
        """
        r = float(filterrad)
        if r <= 0:
            raise ValueError("The filter radius must be a positive number")
        _native_image.set_filterrad(self._native, r)

    def get_filterrad(self):
        """Return the filterrad setting."""
        return _native_image.get_filterrad(self._native)


class AxesImage(_ImageBase):
    """
    An image attached to an Axes.

    pltrs makes them with `~.Axes.imshow`.
    """

    @property
    def origin(self):
        """Where the [0, 0] index of the array is: 'upper' or 'lower'."""
        return _native_image.get_origin(self._native)

    def get_extent(self):
        """Return the image extent as tuple (left, right, bottom, top)."""
        return _native_image.get_extent(self._native)

    def get_cursor_data(self, event):
        """
        Return the image value at the event position or *None* if the event is
        outside the image.

        See Also
        --------
        matplotlib.artist.Artist.get_cursor_data
        """
        if self._figure is None or event.x is None or event.y is None:
            return None
        element = _native_image.cursor_element(
            self._native, float(event.x), float(event.y))
        return None if element is None else self.get_array()[element]

    def set_extent(self, extent, **kwargs):
        """
        Set the image extent.

        Parameters
        ----------
        extent : 4-tuple of float
            The position and size of the image as tuple
            ``(left, right, bottom, top)`` in data coordinates.

        Notes
        -----
        This updates `.Axes.dataLim`, and, if autoscaling, sets `.Axes.viewLim`
        to tightly fit the image, regardless of `~.Axes.dataLim`.
        """
        extent = self.axes._converted_extent(extent, kwargs)
        if kwargs:
            raise _api.kwarg_error("set_extent", kwargs)
        _warn_identical_limits(self.axes, extent)
        _native_image.set_extent(self._native, extent)

    def make_image(self, renderer, magnification=1.0, unsampled=False):
        """
        Normalize, rescale, and colormap this image's data for rendering using
        *renderer*, with the given *magnification*.

        If *unsampled* is True, the image will not be scaled, but an
        appropriate affine transformation will be returned instead.

        Returns
        -------
        image : (M, N, 4) `numpy.uint8` array
            The RGBA image, resampled unless *unsampled* is True.
        x, y : float
            The upper left corner where the image should be drawn, in pixel
            space.
        trans : `~matplotlib.transforms.Affine2D`
            The affine transformation from image to pixel space (None, in
            pltrs, for an image on non-affine axes).
        """
        if unsampled:
            raise NotImplementedError("pltrs: make_image(unsampled=True)")
        if self._A is None:
            raise RuntimeError('You must first set the image array')
        if self._A.size == 0:
            raise RuntimeError("_make_image must get a non-empty image. "
                               "Your Artist's draw method must filter before "
                               "this method is called.")
        made = _native_image.make_image(self._native, float(magnification))
        if made is None:
            return None, 0, 0, None
        pixels, x, y, matrix = made
        return pixels, x, y, None if matrix is None else Affine2D(matrix)


def imread(fname, format=None):
    """
    Read an image from a file into an array.

    Parameters
    ----------
    fname : str or file-like
        The image file to read: a filename or a file-like object opened in
        read-binary mode.
    format : str, optional
        The image file format assumed for reading the data.  The image is
        loaded as a PNG file if *format* is set to "png", if *fname* is a path
        or opened file with a ".png" extension, or if it is a URL.  In all
        other cases, *format* is ignored and the format is auto-detected by
        `PIL.Image.open`.

    Returns
    -------
    `numpy.array`
        The image data. The returned array has shape

        - (M, N) for grayscale images.
        - (M, N, 3) for RGB images.
        - (M, N, 4) for RGBA images.

        PNG images are returned as float arrays (0-1).  All other formats are
        returned as int arrays, with a bit depth determined by the file's
        contents.
    """
    # hide imports to speed initial import on systems with slow linkers
    from urllib import parse
    if format is None:
        if isinstance(fname, str):
            parsed = parse.urlparse(fname)
            # If the string is a URL (Windows paths appear as if they have a
            # length-1 scheme), assume png.
            if len(parsed.scheme) > 1:
                ext = 'png'
            else:
                ext = Path(fname).suffix.lower()[1:]
        elif hasattr(fname, 'geturl'):  # Returned by urlopen().
            ext = 'png'
        elif hasattr(fname, 'name'):
            ext = Path(fname.name).suffix.lower()[1:]
        else:
            ext = 'png'
    else:
        ext = format
    img_open = (
        PIL.PngImagePlugin.PngImageFile if ext == 'png' else PIL.Image.open)
    if isinstance(fname, str) and len(parse.urlparse(fname).scheme) > 1:
        # Pillow doesn't handle URLs directly.
        raise ValueError(
            "Please open the URL for reading and pass the "
            "result to Pillow, e.g. with "
            "``np.array(PIL.Image.open(urllib.request.urlopen(url)))``."
            )
    with img_open(fname) as image:
        return (_pil_png_to_float_array(image)
                if isinstance(image, PIL.PngImagePlugin.PngImageFile) else
                pil_to_array(image))


def imsave(fname, arr, vmin=None, vmax=None, cmap=None, format=None,
           origin=None, dpi=100, *, metadata=None, pil_kwargs=None):
    """
    Colormap and save an array as an image file.

    RGB(A) images are passed through.  Single channel images will be
    colormapped according to *cmap* and *norm*.

    .. note::

       If you want to save a single channel image as gray scale please use an
       image I/O library (such as pillow, tifffile, or imageio) directly.

    Parameters
    ----------
    fname : str or path-like or file-like
        A path or a file-like object to store the image in.
        If *format* is not set, then the output format is inferred from the
        extension of *fname*, if any, and from :rc:`savefig.format` otherwise.
        If *format* is set, it determines the output format.
    arr : array-like
        The image data. Accepts NumPy arrays or sequences
        (e.g., lists or tuples). The shape can be one of
        MxN (luminance), MxNx3 (RGB) or MxNx4 (RGBA).
    vmin, vmax : float, optional
        *vmin* and *vmax* set the color scaling for the image by fixing the
        values that map to the colormap color limits. If either *vmin*
        or *vmax* is None, that limit is determined from the *arr*
        min/max value.
    cmap : str or `~matplotlib.colors.Colormap`, default: :rc:`image.cmap`
        A Colormap instance or registered colormap name. The colormap
        maps scalar data to colors. It is ignored for RGB(A) data.
    format : str, optional
        The file format, e.g. 'png', 'pdf', 'svg', ...  The behavior when this
        is unset is documented under *fname*.
    origin : {'upper', 'lower'}, default: :rc:`image.origin`
        Indicates whether the ``(0, 0)`` index of the array is in the upper
        left or lower left corner of the Axes.
    dpi : float
        The DPI to store in the metadata of the file.  This does not affect the
        resolution of the output image.  Depending on file format, this may be
        rounded to the nearest integer.
    metadata : dict, optional
        Metadata in the image file.  The supported keys depend on the output
        format, see the documentation of the respective backends for more
        information.
        Currently only supported for "png", "pdf", "ps", "eps", and "svg".
    pil_kwargs : dict, optional
        Keyword arguments passed to `PIL.Image.Image.save`.  If the 'pnginfo'
        key is present, it completely overrides *metadata*, including the
        default 'Software' key.
    """
    # Normalizing input (e.g., list or tuples) to NumPy array if needed
    arr = np.asanyarray(arr)

    if isinstance(fname, os.PathLike):
        fname = os.fspath(fname)
    if format is None:
        format = (Path(fname).suffix[1:] if isinstance(fname, str)
                  else _rc("savefig.format")).lower()
    vector = format in ["pdf", "ps", "eps", "svg"]
    if vector:
        # Vector formats that are not handled by PIL.
        if pil_kwargs is not None:
            raise ValueError(
                f"Cannot use 'pil_kwargs' when saving to {format}")
        if metadata is not None:
            raise NotImplementedError(
                f"pltrs: imsave(format={format!r}, metadata=...)")
    # Don't bother creating an image; this avoids rounding errors on the
    # size when dividing and then multiplying by dpi.
    origin = _val_or_rc(origin, "image.origin")
    _api.check_in_list(('upper', 'lower'), origin=origin)
    if origin == "lower":
        arr = arr[::-1]
    if (isinstance(arr, memoryview) and arr.format == "B"
            and arr.ndim == 3 and arr.shape[-1] == 4):
        # Such an ``arr`` would also be handled fine by sm.to_rgba below
        # (after casting with asarray), but it is useful to special-case it
        # because that's what backend_agg passes, and can be in fact used
        # as is, saving a few operations.
        rgba = arr
    else:
        sm = mcolorizer.Colorizer(cmap=cmap)
        sm.set_clim(vmin, vmax)
        rgba = sm.to_rgba(arr, bytes=True)
    if vector:
        # matplotlib draws the image with figimage on a frameless,
        # transparent figure of its size; so does pltrs (Rust).
        document = _native_image.imsave_vector(
            np.asarray(rgba, dtype=np.uint8), format, float(dpi))
        if hasattr(fname, "write"):
            fname.write(document)
        else:
            with open(fname, "wb") as file:
                file.write(document)
        return
    if pil_kwargs is None:
        pil_kwargs = {}
    else:
        # we modify this below, so make a copy (don't modify caller's dict)
        pil_kwargs = pil_kwargs.copy()
    pil_shape = (rgba.shape[1], rgba.shape[0])
    rgba = np.require(rgba, requirements='C')
    image = PIL.Image.frombuffer(
        "RGBA", pil_shape, rgba, "raw", "RGBA", 0, 1)
    if format == "png":
        # Only use the metadata kwarg if pnginfo is not set, because the
        # semantics of duplicate keys in pnginfo is unclear.
        if "pnginfo" in pil_kwargs:
            if metadata:
                _api.warn_external("'metadata' is overridden by the "
                                   "'pnginfo' entry in 'pil_kwargs'.")
        else:
            from . import __version__
            metadata = {
                "Software": (f"Matplotlib version{__version__}, "
                             f"https://matplotlib.org/"),
                **(metadata if metadata is not None else {}),
            }
            pil_kwargs["pnginfo"] = pnginfo = PIL.PngImagePlugin.PngInfo()
            for k, v in metadata.items():
                if v is not None:
                    pnginfo.add_text(k, v)
    elif metadata is not None:
        raise ValueError(f"metadata not supported for format {format!r}")
    if format in ["jpg", "jpeg"]:
        format = "jpeg"  # Pillow doesn't recognize "jpg".
        facecolor = _rc("savefig.facecolor")
        if cbook._str_equal(facecolor, "auto"):
            facecolor = _rc("figure.facecolor")
        color = tuple(int(x * 255) for x in mcolors.to_rgb(facecolor))
        background = PIL.Image.new("RGB", pil_shape, color)
        background.paste(image, image)
        image = background
    pil_kwargs.setdefault("format", format)
    pil_kwargs.setdefault("dpi", (dpi, dpi))
    image.save(fname, **pil_kwargs)


def pil_to_array(pilImage):
    """
    Load a `PIL image`_ and return it as a numpy int array.

    .. _PIL image: https://pillow.readthedocs.io/en/latest/reference/Image.html

    Returns
    -------
    numpy.array

        The array shape depends on the image type:

        - (M, N) for grayscale images.
        - (M, N, 3) for RGB images.
        - (M, N, 4) for RGBA images.
    """
    if pilImage.mode in ['RGBA', 'RGBX', 'RGB', 'L']:
        # return MxNx4 RGBA, MxNx3 RBA, or MxN luminance array
        return np.asarray(pilImage)
    elif pilImage.mode.startswith('I;16'):
        # return MxN luminance array of uint16
        raw = pilImage.tobytes('raw', pilImage.mode)
        if pilImage.mode.endswith('B'):
            x = np.frombuffer(raw, '>u2')
        else:
            x = np.frombuffer(raw, '<u2')
        return x.reshape(pilImage.size[::-1]).astype('=u2')
    else:  # try to convert to an rgba image
        try:
            pilImage = pilImage.convert('RGBA')
        except ValueError as err:
            raise RuntimeError('Unknown image mode') from err
        return np.asarray(pilImage)  # return MxNx4 RGBA array


def _pil_png_to_float_array(pil_png):
    """Convert a PIL `PNGImageFile` to a 0-1 float array."""
    # Unlike pil_to_array this converts to 0-1 float32s for backcompat with the
    # old libpng-based loader.
    # The supported rawmodes are from PIL.PngImagePlugin._MODES.  When
    # mode == "RGB(A)", the 16-bit raw data has already been coarsened to 8-bit
    # by Pillow.
    mode = pil_png.mode
    rawmode = pil_png.png.im_rawmode
    if rawmode == "1":  # Grayscale.
        return np.asarray(pil_png, np.float32)
    if rawmode == "L;2":  # Grayscale.
        return np.divide(pil_png, 2**2 - 1, dtype=np.float32)
    if rawmode == "L;4":  # Grayscale.
        return np.divide(pil_png, 2**4 - 1, dtype=np.float32)
    if rawmode == "L":  # Grayscale.
        return np.divide(pil_png, 2**8 - 1, dtype=np.float32)
    if rawmode == "I;16B":  # Grayscale.
        return np.divide(pil_png, 2**16 - 1, dtype=np.float32)
    if mode == "RGB":  # RGB.
        return np.divide(pil_png, 2**8 - 1, dtype=np.float32)
    if mode == "P":  # Palette.
        return np.divide(pil_png.convert("RGBA"), 2**8 - 1, dtype=np.float32)
    if mode == "LA":  # Grayscale + alpha.
        return np.divide(pil_png.convert("RGBA"), 2**8 - 1, dtype=np.float32)
    if mode == "RGBA":  # RGBA.
        return np.divide(pil_png, 2**8 - 1, dtype=np.float32)
    raise ValueError(f"Unknown PIL rawmode: {rawmode}")


class NonUniformImage(AxesImage):
    """An image with pixels on a rectilinear, non-uniform grid."""


class PcolorImage(AxesImage):
    """
    Make a pcolor-style plot with an irregular rectangular grid (as
    `~.Axes.pcolorfast` makes).
    """


class FigureImage(_ImageBase):
    """An image attached to a figure (as `~.Figure.figimage` makes)."""


class BboxImage(_ImageBase):
    """The Image class whose size is determined by the given bbox."""

    @classmethod
    def _loose(cls, A, *, cmap=None, norm=None, interpolation=None,
               origin=None, filternorm=True, filterrad=4.0, resample=False,
               **kwargs):
        """
        The image of the array *A* outside any figure, until the offset box
        holding it joins one (the image of an `.OffsetImage`).
        """
        A = cls._normalize_image_array(A)
        colorizer = mcolorizer._ScalarMappable._get_colorizer(cmap, norm, None)
        image = cls.__new__(cls)
        image._native = _native_image.new_bbox_image(
            _native_data(A), colorizer.cmap, colorizer.norm, {
                'interpolation': interpolation, 'interpolation_stage': None,
                'origin': origin,
                'filternorm': bool(filternorm), 'filterrad': float(filterrad),
                'resample': bool(resample)})
        mcolorizer.artist.Artist.__init__(image)
        image._set_mapped_data(colorizer, A)
        image._internal_update(kwargs)
        return image
