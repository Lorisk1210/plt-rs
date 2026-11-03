"""
The image methods of `~.axes.Axes` (matplotlib's ``imshow`` and the
current-image bookkeeping of ``axes._base``), a mixin of the Axes class.

``imshow`` checks its arguments and sets up the colorizer as matplotlib
does (from matplotlib 3.11.2, under the Matplotlib License); pltrs (Rust)
makes the image (``_native.image``).
"""


from .. import (_api, _handles, _live_transforms, _values, cbook,
               colorizer as mcolorizer)
from .. import image as mimage
from .._native import image as _native_image
from .._preprocess import _preprocess_data
from ._artist_list import ArtistList


def _rc(key):
    """The rcParam *key* (imported when first needed)."""
    from .. import rcParams
    return rcParams[key]


class ImagingMixin:
    """The image methods of `~.axes.Axes`."""

    @_preprocess_data()
    def imshow(self, X, cmap=None, norm=None, *, aspect=None,
               interpolation=None, alpha=None,
               vmin=None, vmax=None, colorizer=None, origin=None, extent=None,
               interpolation_stage=None, filternorm=True, filterrad=4.0,
               resample=None, url=None, **kwargs):
        """
        Display data as an image, i.e., on a 2D regular raster.

        The input may either be actual RGB(A) data, or 2D scalar data, which
        will be rendered as a pseudocolor image. For displaying a grayscale
        image, set up the colormapping using the parameters
        ``cmap='gray', vmin=0, vmax=255``.

        See matplotlib's documentation of `matplotlib.axes.Axes.imshow`.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        if mimage._is_pil_image(X):
            X = mimage.pil_to_array(X)  # Needed e.g. to apply png palette.
        A = mimage._ImageBase._normalize_image_array(X)
        mcolorizer._ScalarMappable._check_exclusionary_keywords(
            colorizer, vmin=vmin, vmax=vmax)
        colorizer = mcolorizer._ScalarMappable._get_colorizer(
            cmap, norm, colorizer)
        if interpolation is not None:
            interpolation = interpolation.lower()
            _api.check_in_list(mimage.interpolations_names,
                               interpolation=interpolation)
        if interpolation_stage is not None:
            _api.check_in_list(['data', 'rgba', 'auto'], s=interpolation_stage)
        origin = _rc('image.origin') if origin is None else origin
        _api.check_in_list(["upper", "lower"], origin=origin)
        filterrad = float(filterrad)
        if filterrad <= 0:
            raise ValueError("The filter radius must be a positive number")
        colorizer._scale_norm(norm, vmin, vmax, A)
        spec = {
            'aspect': aspect,
            'interpolation': interpolation,
            'interpolation_stage': interpolation_stage,
            'origin': origin,
            'extent': None if extent is None else self._converted_extent(extent),
            'filternorm': bool(filternorm),
            'filterrad': filterrad,
            'resample': None if resample is None else bool(resample),
            # Which coordinates the image is in decides whether imshow sets
            # the aspect (the transform itself is set with the properties).
            'transform': _live_transforms.given_to_native(kwargs.get('transform')),
        }
        native = _native_image.imshow(self._native, mimage._native_data(A),
                                      colorizer.cmap, colorizer.norm, spec)
        im = _handles.wrap(native, mimage.AxesImage, self.figure)
        mimage._warn_identical_limits(self, im.get_extent())
        # The extent as matplotlib's imshow keeps it (``set_extent``).
        _values.remember(im, "extent", (list(im.get_extent()) if extent is None
                                        else self._extent_values(extent)))
        im._set_mapped_data(colorizer, A)
        im.set_alpha(alpha)
        im.set_url(url)
        im._internal_update(kwargs)
        self._sci(im)
        return im

    def get_images(self):
        r"""Return a list of `.AxesImage`\s contained by the Axes."""
        return cbook.silent_list('AxesImage', self.images)

    @property
    def images(self):
        """The `.AxesImage`\\s of the Axes, in the order they were added."""
        return ArtistList("images", lambda: _handles.wrap_all(
            _native_image.get_images(self._native), mimage.AxesImage,
            self.figure))

    def _sci(self, im):
        """
        Set the current image.

        This image will be the target of colormap functions like
        ``pyplot.viridis``, and other functions such as `~.pyplot.clim`.  The
        current image is an attribute of the current Axes.
        """
        _api.check_isinstance(mcolorizer.ColorizingArtist, im=im)
        self._current_image = im

    def _gci(self):
        """Helper for `~matplotlib.pyplot.gci`; do not use elsewhere."""
        return getattr(self, '_current_image', None)
