"""
The images drawn in a figure's own pixels (matplotlib's
``Figure.figimage`` and ``Figure.images``, and whether they are
composited, ``Figure.suppressComposite``): pltrs (Rust) draws them
unresampled, colour-mapped through the colorizer the image keeps. The
argument handling of ``figimage`` is matplotlib 3.11.2's, under the
Matplotlib License.
"""

import numpy as np

from . import _docstring
from . import _handles, colorizer as mcolorizer, image as mimage
from ._native import backends as _backends, image as _native_image


class FigureImagesMixin:
    """`figimage`, ``images`` and ``suppressComposite`` of `.Figure`."""

    @_docstring.interpd
    def figimage(self, X, xo=0, yo=0, alpha=None, norm=None, cmap=None,
                 vmin=None, vmax=None, origin=None, resize=False, *,
                 colorizer=None, **kwargs):
        """
        Add a non-resampled image to the figure.

        The image is attached to the lower or upper left corner depending on
        *origin*.

        Parameters
        ----------
        X
            The image data. This is an array of one of the following shapes:

            - (M, N): an image with scalar data.  Color-mapping is controlled
              by *cmap*, *norm*, *vmin*, and *vmax*.
            - (M, N, 3): an image with RGB values (0-1 float or 0-255 int).
            - (M, N, 4): an image with RGBA values (0-1 float or 0-255 int),
              i.e. including transparency.

        xo, yo : int
            The *x*/*y* image offset in pixels.

        alpha : None or float
            The alpha blending value.

        %(cmap_doc)s

            This parameter is ignored if *X* is RGB(A).

        %(norm_doc)s

            This parameter is ignored if *X* is RGB(A).

        %(vmin_vmax_doc)s

            This parameter is ignored if *X* is RGB(A).

        origin : {'upper', 'lower'}, default: :rc:`image.origin`
            Indicates where the [0, 0] index of the array is in the upper left
            or lower left corner of the Axes.

        resize : bool
            If *True*, resize the figure to match the given image size.

        %(colorizer_doc)s

            This parameter is ignored if *X* is RGB(A).

        Returns
        -------
        `matplotlib.image.FigureImage`

        Other Parameters
        ----------------
        **kwargs
            Additional kwargs are `.Artist` kwargs passed on to `.FigureImage`.

        Notes
        -----
        figimage complements the Axes image (`~matplotlib.axes.Axes.imshow`)
        which will be resampled to fit the current Axes.  If you want
        a resampled image to fill the entire figure, you can define an
        `~matplotlib.axes.Axes` with extent [0, 0, 1, 1].

        Examples
        --------
        ::

            f = plt.figure()
            nx = int(f.get_figwidth() * f.dpi)
            ny = int(f.get_figheight() * f.dpi)
            data = np.random.random((ny, nx))
            f.figimage(data)
            plt.show()
        """
        A = mimage._ImageBase._normalize_image_array(np.asanyarray(X))
        mcolorizer._ScalarMappable._check_exclusionary_keywords(
            colorizer, vmin=vmin, vmax=vmax)
        colorizer = mcolorizer._ScalarMappable._get_colorizer(
            cmap, norm, colorizer)
        if norm is None:
            # As matplotlib's figimage: the norm scales when first drawn.
            colorizer.set_clim(vmin, vmax)
        native = _native_image.figimage(
            self._native, mimage._native_data(A), colorizer.cmap,
            colorizer.norm, (float(xo), float(yo)), origin, bool(resize))
        im = _handles.wrap(native, mimage.FigureImage, self.figure)
        im._keep_figimage_settings(xo, yo)
        im._set_mapped_data(colorizer, A)
        im.set_alpha(alpha)
        im._internal_update(kwargs)
        return im

    @property
    def images(self):
        """The `.FigureImage`\\s of the figure, in the order they were
        added."""
        return _handles.wrap_all(_native_image.figure_images(self._native),
                                 mimage.FigureImage, self.figure)

    @property
    def suppressComposite(self):
        """
        For multiple images, the figure will make composite images
        depending on the renderer option_image_nocomposite function. If
        *suppressComposite* is a boolean, this will override the renderer;
        True also keeps consecutive rasterized artists in images of their
        own.
        """
        return _backends.get_suppress_composite(self._native)

    @suppressComposite.setter
    def suppressComposite(self, suppress):
        _backends.set_suppress_composite(
            self._native, None if suppress is None else bool(suppress))
