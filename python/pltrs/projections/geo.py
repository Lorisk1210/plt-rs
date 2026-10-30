"""
Geographic axes (matplotlib's ``projections.geo``): pltrs (Rust) axes in the
Aitoff, Hammer, Lambert or Mollweide projection, made by
``add_subplot(projection=...)``, and the transforms of their projections
(``ax.transProjection``), which pltrs (Rust) computes.
"""

import numpy as np

from .._native import transforms as _transforms
from ..axes import Axes
from ..path import Path
from ..transforms import Transform


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


class GeoAxes(Axes):
    """An abstract base class for geographic projections."""

    RESOLUTION = 75

    @property
    def transProjection(self):
        """The projection of longitudes and latitudes onto the plane."""
        return self._get_core_transform(self.RESOLUTION)


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
