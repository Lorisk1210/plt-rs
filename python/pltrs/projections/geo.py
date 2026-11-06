"""
Geographic axes (matplotlib's ``projections.geo``): pltrs (Rust) axes in the
Aitoff, Hammer, Lambert or Mollweide projection, made by
``add_subplot(projection=...)``, and the transforms of their projections
(``ax.transProjection``), which pltrs (Rust) computes. Their transform
trees are matplotlib's (``GeoAxes._set_lim_and_transforms``, from
matplotlib 3.11.2, under the Matplotlib License), over pltrs (Rust)'s
leaves (`.._transform_trees`).
"""

import numpy as np

from .. import _api
from .._native import transforms as _transforms
from .._transform_trees import TreeAttribute, set_matrix
from ..axes import Axes
from ..path import Path
from ..ticker import Formatter
from ..transforms import Affine2D, BboxTransformTo, Transform


# The projection of longitudes and latitudes in radians onto the plane, or
# its inverse (``inverse``), by the projection ``projection`` names centred
# at ``_center``. No docstring: the inverse transforms show `.Transform`'s,
# as matplotlib's do.
class _GeoTransform(Transform):

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


def _transform_pair(projection, forward_doc):
    """The classes of the projection *projection* (documented by
    *forward_doc*) and of its inverse, named as the classes matplotlib nests
    in the projection's axes."""
    axes = f"{projection.capitalize()}Axes"

    def make(name, attributes):
        cls = type(name, (_GeoTransform,), attributes)
        cls.__qualname__ = f"{axes}.{name}"
        return cls

    forward = make(f"{projection.capitalize()}Transform",
                   {"__doc__": forward_doc, "projection": projection})
    inverse = make(f"Inverted{projection.capitalize()}Transform",
                   {"__doc__": None, "projection": projection,
                    "inverse": True, "_inverse_class": forward})
    forward._inverse_class = inverse
    return forward, inverse


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

    def _gen_axes_patch(self):
        # docstring inherited
        from ..patches import Circle
        return Circle((0.5, 0.5), 0.5)

    def clear(self):
        # docstring inherited; pltrs (Rust) clears a geographic Axes to its
        # grids, ticks and limits, as matplotlib's does.
        super().clear()

    def set_yscale(self, *args, **kwargs):
        if args[0] != 'linear':
            raise NotImplementedError

    set_xscale = set_yscale

    def set_xlim(self, *args, **kwargs):
        """Not supported. Please consider using Cartopy."""
        raise TypeError("Changing axes limits of a geographic projection is "
                        "not supported.  Please consider using Cartopy.")

    set_ylim = set_xlim
    set_xbound = set_xlim
    set_ybound = set_ylim

    def invert_xaxis(self):
        """Not supported. Please consider using Cartopy."""
        raise TypeError("Changing axes limits of a geographic projection is "
                        "not supported.  Please consider using Cartopy.")

    invert_yaxis = invert_xaxis

    def format_coord(self, lon, lat):
        """Return a format string formatting the coordinate."""
        lon, lat = np.rad2deg([lon, lat])
        ns = 'N' if lat >= 0.0 else 'S'
        ew = 'E' if lon >= 0.0 else 'W'
        return ('%f\N{DEGREE SIGN}%s, %f\N{DEGREE SIGN}%s'
                % (abs(lat), ns, abs(lon), ew))

    transProjection = TreeAttribute(
        "The projection of longitudes and latitudes onto the plane.")
    transAffine = TreeAttribute()
    _xaxis_pretransform = TreeAttribute()
    _xaxis_text1_transform = TreeAttribute()
    _xaxis_text2_transform = TreeAttribute()
    _yaxis_text1_transform = TreeAttribute()
    _yaxis_text2_transform = TreeAttribute()

    def _set_lim_and_transforms(self):
        # A (possibly non-linear) projection on the (already scaled) data
        self.transProjection = self._get_core_transform(self.RESOLUTION)

        self.transAffine = self._get_affine_transform()

        self.transAxes = BboxTransformTo(self.bbox)

        # The complete data transformation stack -- from data all the
        # way to display coordinates
        self.transData = \
            self.transProjection + \
            self.transAffine + \
            self.transAxes

        # This is the transform for longitude ticks (its cap is pltrs
        # (Rust)'s, given by `_apply_projection_leaves`).
        self._xaxis_pretransform = \
            Affine2D() \
            .scale(1, np.pi) \
            .translate(0, -np.pi / 2)
        self._xaxis_transform = \
            self._xaxis_pretransform + \
            self.transData
        self._xaxis_text1_transform = \
            Affine2D().scale(1, 0) + \
            self.transData + \
            Affine2D().translate(0, 4)
        self._xaxis_text2_transform = \
            Affine2D().scale(1, 0) + \
            self.transData + \
            Affine2D().translate(0, -4)

        # This is the transform for latitude ticks.
        yaxis_stretch = Affine2D().scale(np.pi * 2, 1).translate(-np.pi, 0)
        yaxis_space = Affine2D().scale(1, 1.1)
        self._yaxis_transform = \
            yaxis_stretch + \
            self.transData
        yaxis_text_base = \
            yaxis_stretch + \
            self.transProjection + \
            (yaxis_space +
             self.transAffine +
             self.transAxes)
        self._yaxis_text1_transform = \
            yaxis_text_base + \
            Affine2D().translate(-8, 0)
        self._yaxis_text2_transform = \
            yaxis_text_base + \
            Affine2D().translate(8, 0)

    def _apply_projection_leaves(self, leaves):
        """Give the longitude gridlines pltrs (Rust)'s latitude cap."""
        _, cap = leaves
        set_matrix(self._xaxis_pretransform, Affine2D().scale(1.0, cap * 2.0)
                   .translate(0.0, -cap).get_matrix())

    def _get_affine_transform(self):
        return Affine2D(_transforms.geo_affine(self._native))

    def get_xaxis_transform(self, which='grid'):
        _api.check_in_list(['tick1', 'tick2', 'grid'], which=which)
        return self._xaxis_transform

    def get_xaxis_text1_transform(self, pad):
        return self._xaxis_text1_transform, 'bottom', 'center'

    def get_xaxis_text2_transform(self, pad):
        return self._xaxis_text2_transform, 'top', 'center'

    def get_yaxis_transform(self, which='grid'):
        _api.check_in_list(['tick1', 'tick2', 'grid'], which=which)
        return self._yaxis_transform

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

    def get_data_ratio(self):
        """Return the aspect ratio of the data itself."""
        return 1.0

    ### Interactive panning

    def can_zoom(self):
        """
        Return whether this Axes supports the zoom box button functionality.

        This Axes object does not support interactive zoom box.
        """
        return False

    def can_pan(self):
        """
        Return whether this Axes supports the pan/zoom button functionality.

        This Axes object does not support interactive pan/zoom.
        """
        return False

    def start_pan(self, x, y, button):
        pass

    def end_pan(self):
        pass

    def drag_pan(self, button, key, x, y):
        pass


class AitoffAxes(GeoAxes):
    name = 'aitoff'

    def __init__(self, *args, **kwargs):
        # matplotlib's signature; pltrs (Rust) sets the axes' aspect and
        # limits up.
        super().__init__(*args, **kwargs)

    AitoffTransform, InvertedAitoffTransform = _transform_pair(
        'aitoff', "The base Aitoff transform.")

    def _get_core_transform(self, resolution):
        return self.AitoffTransform(resolution)


class HammerAxes(GeoAxes):
    name = 'hammer'

    def __init__(self, *args, **kwargs):
        # matplotlib's signature; pltrs (Rust) sets the axes' aspect and
        # limits up.
        super().__init__(*args, **kwargs)

    HammerTransform, InvertedHammerTransform = _transform_pair(
        'hammer', "The base Hammer transform.")

    def _get_core_transform(self, resolution):
        return self.HammerTransform(resolution)


class MollweideAxes(GeoAxes):
    name = 'mollweide'

    def __init__(self, *args, **kwargs):
        # matplotlib's signature; pltrs (Rust) sets the axes' aspect and
        # limits up.
        super().__init__(*args, **kwargs)

    MollweideTransform, InvertedMollweideTransform = _transform_pair(
        'mollweide', "The base Mollweide transform.")

    def _get_core_transform(self, resolution):
        return self.MollweideTransform(resolution)


class LambertAxes(GeoAxes):
    name = 'lambert'

    _LambertBase, _InvertedLambertBase = _transform_pair(
        'lambert', "The base Lambert transform.")

    class LambertTransform(_LambertBase):
        """The base Lambert transform."""

        def __init__(self, center_longitude, center_latitude, resolution):
            """
            Create a new Lambert transform.  Resolution is the number of steps
            to interpolate between each input line segment to approximate its
            path in curved Lambert space.
            """
            super().__init__(resolution)
            self._center = (float(center_longitude), float(center_latitude))

        def inverted(self):
            # docstring inherited
            return LambertAxes.InvertedLambertTransform(
                *self._center, self._resolution)

    class InvertedLambertTransform(_InvertedLambertBase):
        def __init__(self, center_longitude, center_latitude, resolution):
            super().__init__(resolution)
            self._center = (float(center_longitude), float(center_latitude))

        def inverted(self):
            # docstring inherited
            return LambertAxes.LambertTransform(*self._center, self._resolution)

    def __init__(self, *args, center_longitude=0, center_latitude=0, **kwargs):
        super().__init__(*args, **kwargs)
        _transforms.set_lambert_center(
            self._native, (float(center_longitude), float(center_latitude)))

    @classmethod
    def _make_native(cls, figure, kwargs, *, spec=None, rect=None):
        """The native axes ``add_subplot`` and ``add_axes`` make: by the
        constructor, which takes the centre among *kwargs*."""
        return cls._constructed_native(figure, kwargs, spec=spec, rect=rect)

    def clear(self):
        # docstring inherited; pltrs (Rust) hides the latitude labels.
        super().clear()

    def _get_core_transform(self, resolution):
        return self.LambertTransform(
            *_transforms.lambert_center(self._native), resolution)


# What matplotlib's module also has from its imports, which programs
# import from it.
from .. import axis as maxis, spines as mspines  # noqa: E402, F401
from ..patches import Circle  # noqa: E402, F401
from ..ticker import FixedLocator, NullFormatter, NullLocator  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
