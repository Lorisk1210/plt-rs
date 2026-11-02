"""
Geographic axes (matplotlib's ``projections.geo``): pltrs (Rust) axes in the
Aitoff, Hammer, Lambert or Mollweide projection, made by
``add_subplot(projection=...)``, and the transforms of their projections
(``ax.transProjection``), which pltrs (Rust) computes. The transforms of
the axes' tick labels are matplotlib's trees of them, around the axes'
live transforms.
"""

import functools

import numpy as np

from .._native import transforms as _transforms
from ..axes import Axes
from ..path import Path
from ..ticker import Formatter
from ..transforms import Affine2D, Transform


class _GeoTransform(Transform):
    """
    The projection of longitudes and latitudes in radians onto the plane,
    or its inverse (``inverse``), by the projection ``projection`` names
    centred at ``_center``.
    """

    input_dims = output_dims = 2
    #: The name of the projection.
    projection = None
    #: Whether this is the inverse of the projection.
    inverse = False

    def __init__(self, resolution):
        """
        Create a new geographical transform.

        Resolution is the number of steps to interpolate between each input
        line segment to approximate its path in curved space.
        """
        super().__init__()
        self._resolution = resolution
        self._center = (0.0, 0.0)

    def __str__(self):
        return f"{type(self).__name__}({self._resolution})"

    def transform_non_affine(self, values):
        # docstring inherited
        values = np.asarray(values, dtype=float)
        compute = _transforms.geo_inverse if self.inverse else _transforms.geo_forward
        return compute(self.projection, self._center,
                       values.reshape(-1, 2)).reshape(values.shape)

    def transform_path_non_affine(self, path):
        # docstring inherited
        ipath = path.interpolated(self._resolution)
        return Path(self.transform(ipath.vertices), ipath.codes)

    def inverted(self):
        # docstring inherited
        inverse = self._inverse_class(self._resolution)
        inverse._center = self._center
        return inverse


def _transform_pair(projection, forward_doc, inverse_doc):
    """The classes of the projection *projection* and of its inverse."""
    forward = type(f"{projection.capitalize()}Transform", (_GeoTransform,),
                   {"__doc__": forward_doc, "projection": projection})
    inverse = type(f"Inverted{projection.capitalize()}Transform",
                   (_GeoTransform,),
                   {"__doc__": inverse_doc, "projection": projection,
                    "inverse": True, "_inverse_class": forward})
    forward._inverse_class = inverse
    return forward, inverse


class _LongitudeCap(Affine2D):
    """The stretch of the longitude gridlines from the axes' fractions to
    their latitude caps (matplotlib's ``GeoAxes._xaxis_pretransform``),
    read from the axes whenever it is used."""

    def __init__(self, axes):
        super().__init__()
        self._axes = axes

    def get_matrix(self):
        cap = np.deg2rad(_transforms.get_longitude_grid_ends(self._axes._native))
        return Affine2D().scale(1.0, cap * 2.0).translate(0.0, -cap).get_matrix()


class GeoAxes(Axes):
    """An abstract base class for geographic projections."""

    class ThetaFormatter(Formatter):
        """
        Used to format the theta tick labels.  Converts the native
        unit of radians into degrees and adds a degree symbol.
        """
        def __init__(self, round_to=1.0):
            self._round_to = round_to

        def __call__(self, x, pos=None):
            return _transforms.geo_theta_label(float(self._round_to), float(x))

    RESOLUTION = 75

    @property
    def transProjection(self):
        """The projection of longitudes and latitudes onto the plane."""
        return self._get_core_transform(self.RESOLUTION)

    def _get_affine_transform(self):
        return Affine2D(_transforms.geo_affine(self._native))

    @functools.cached_property
    def _yaxis_text_base(self):
        """The latitude labels' place, the gridlines stretched over all
        longitudes and spaced out vertically."""
        yaxis_stretch = Affine2D().scale(np.pi * 2, 1).translate(-np.pi, 0)
        yaxis_space = Affine2D().scale(1, 1.1)
        return (yaxis_stretch + self.transProjection
                + (yaxis_space + self._get_affine_transform() + self.transAxes))

    @functools.cached_property
    def _xaxis_text1_transform(self):
        return Affine2D().scale(1, 0) + self.transData + Affine2D().translate(0, 4)

    @functools.cached_property
    def _xaxis_text2_transform(self):
        return Affine2D().scale(1, 0) + self.transData + Affine2D().translate(0, -4)

    @functools.cached_property
    def _yaxis_text1_transform(self):
        return self._yaxis_text_base + Affine2D().translate(-8, 0)

    @functools.cached_property
    def _yaxis_text2_transform(self):
        return self._yaxis_text_base + Affine2D().translate(8, 0)

    def get_xaxis_transform(self, which='grid'):
        if which not in ('tick1', 'tick2', 'grid'):
            raise ValueError(
                "'which' must be one of 'tick1', 'tick2', or 'grid'")
        return self._xaxis_transform

    @functools.cached_property
    def _xaxis_transform(self):
        return _LongitudeCap(self) + self.transData

    def get_xaxis_text1_transform(self, pad):
        return self._xaxis_text1_transform, 'bottom', 'center'

    def get_xaxis_text2_transform(self, pad):
        return self._xaxis_text2_transform, 'top', 'center'

    def get_yaxis_text1_transform(self, pad):
        return self._yaxis_text1_transform, 'center', 'right'

    def get_yaxis_text2_transform(self, pad):
        return self._yaxis_text2_transform, 'center', 'left'

    def set_longitude_grid(self, degrees):
        """
        Set the number of degrees between each longitude grid.
        """
        _transforms.set_longitude_grid(self._native, float(degrees))

    def set_latitude_grid(self, degrees):
        """
        Set the number of degrees between each latitude grid.
        """
        _transforms.set_latitude_grid(self._native, float(degrees))

    def set_longitude_grid_ends(self, degrees):
        """
        Set the latitude(s) at which to stop drawing the longitude grids.
        """
        _transforms.set_longitude_grid_ends(self._native, float(degrees))


class AitoffAxes(GeoAxes):
    """The Aitoff projection of longitudes and latitudes."""
    name = 'aitoff'

    AitoffTransform, InvertedAitoffTransform = _transform_pair(
        'aitoff', "The base Aitoff transform.",
        "The inverse of the Aitoff transform (undefined: NaN everywhere).")

    def _get_core_transform(self, resolution):
        return self.AitoffTransform(resolution)


class HammerAxes(GeoAxes):
    """The Hammer projection of longitudes and latitudes."""
    name = 'hammer'

    HammerTransform, InvertedHammerTransform = _transform_pair(
        'hammer', "The base Hammer transform.",
        "The inverse of the Hammer transform.")

    def _get_core_transform(self, resolution):
        return self.HammerTransform(resolution)


class MollweideAxes(GeoAxes):
    """The Mollweide projection of longitudes and latitudes."""
    name = 'mollweide'

    MollweideTransform, InvertedMollweideTransform = _transform_pair(
        'mollweide', "The base Mollweide transform.",
        "The inverse of the Mollweide transform.")

    def _get_core_transform(self, resolution):
        return self.MollweideTransform(resolution)


class LambertAxes(GeoAxes):
    """The Lambert azimuthal equal-area projection of longitudes and
    latitudes."""
    name = 'lambert'

    _LambertBase, _InvertedLambertBase = _transform_pair(
        'lambert', "The base Lambert transform.",
        "The inverse of the Lambert transform.")

    class LambertTransform(_LambertBase):
        """The base Lambert transform."""

        def __init__(self, center_longitude, center_latitude, resolution):
            """
            Create a new Lambert transform.  Resolution is the number of
            steps to interpolate between each input line segment to
            approximate its path in curved Lambert space.
            """
            super().__init__(resolution)
            self._center = (float(center_longitude), float(center_latitude))

        def inverted(self):
            # docstring inherited
            return LambertAxes.InvertedLambertTransform(
                *self._center, self._resolution)

    class InvertedLambertTransform(_InvertedLambertBase):
        """The inverse of the Lambert transform."""

        def __init__(self, center_longitude, center_latitude, resolution):
            super().__init__(resolution)
            self._center = (float(center_longitude), float(center_latitude))

        def inverted(self):
            # docstring inherited
            return LambertAxes.LambertTransform(*self._center, self._resolution)

    def _get_core_transform(self, resolution):
        return self.LambertTransform(
            *_transforms.lambert_center(self._native), resolution)
