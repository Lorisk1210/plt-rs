"""
Geographic axes (matplotlib's ``projections.geo``): plt-rs axes in the
Aitoff, Hammer, Lambert or Mollweide projection, made by
``add_subplot(projection=...)``.
"""

from ..axes import Axes


class GeoAxes(Axes):
    """An abstract base class for geographic projections."""


class AitoffAxes(GeoAxes):
    """The Aitoff projection of longitudes and latitudes."""
    name = 'aitoff'


class HammerAxes(GeoAxes):
    """The Hammer projection of longitudes and latitudes."""
    name = 'hammer'


class MollweideAxes(GeoAxes):
    """The Mollweide projection of longitudes and latitudes."""
    name = 'mollweide'


class LambertAxes(GeoAxes):
    """The Lambert azimuthal equal-area projection of longitudes and
    latitudes."""
    name = 'lambert'
