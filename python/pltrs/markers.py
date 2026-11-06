r"""
Functions to handle markers; used by the marker functionality of
`~matplotlib.axes.Axes.plot`, `~matplotlib.axes.Axes.scatter`, and
`~matplotlib.axes.Axes.errorbar` (matplotlib's ``markers``).

A `MarkerStyle` keeps matplotlib's attributes; pltrs (Rust) resolves the marker
into the paths and transforms it draws (``pltrs::markers``).

Markers are given as one of matplotlib's codes (``"o"``, ``"s"``,
`CARETLEFT`...), a mathtext string (``"$\alpha$"``), a list of vertices,
a `~matplotlib.path.Path`, or a regular polygon ``(numsides, style,
angle)``, with style 0 (polygon), 1 (star) or 2 (asterisk).
"""

import copy

from collections.abc import Hashable, Sized

import numpy as np

from . import _api
from ._enums import JoinStyle, CapStyle
from ._native import markers as _markers
from .path import Path
from .transforms import Affine2D

# special-purpose marker identifiers:
(TICKLEFT, TICKRIGHT, TICKUP, TICKDOWN,
 CARETLEFT, CARETRIGHT, CARETUP, CARETDOWN,
 CARETLEFTBASE, CARETRIGHTBASE, CARETUPBASE, CARETDOWNBASE) = range(12)

_empty_path = Path(np.empty((0, 2)))


def _rc(key):
    """The rcParam *key* (imported when first needed)."""
    from . import rcParams
    return rcParams[key]


#: The kinds of marker spec whose paths are matplotlib's built-in ones.
_BUILTIN_KINDS = ("code", "polygon")

#: The built-in marker paths, by their vertices and codes.
_BUILTIN_PATHS = {}


def _builtin_path(vertices, codes):
    """
    The `.Path` of *vertices* and *codes*, one object for every marker of
    those, as matplotlib's markers share the paths of the built-in shapes
    (`.Path.unit_circle`, the class attributes of `MarkerStyle`).
    """
    vertices = np.asarray(vertices)
    key = (vertices.shape, vertices.tobytes(),
           None if codes is None else np.asarray(codes).tobytes())
    path = _BUILTIN_PATHS.get(key)
    if path is None:
        path = _BUILTIN_PATHS[key] = Path(vertices, codes)
    return path


class MarkerStyle:
    """
    A class representing marker types.

    Instances are immutable. If you need to change anything, create a new
    instance.

    Attributes
    ----------
    markers : dict
        All known markers.
    filled_markers : tuple
        All known filled markers. This is a subset of *markers*.
    fillstyles : tuple
        The supported fillstyles.
    """

    markers = {
        '.': 'point',
        ',': 'pixel',
        'o': 'circle',
        'v': 'triangle_down',
        '^': 'triangle_up',
        '<': 'triangle_left',
        '>': 'triangle_right',
        '1': 'tri_down',
        '2': 'tri_up',
        '3': 'tri_left',
        '4': 'tri_right',
        '8': 'octagon',
        's': 'square',
        'p': 'pentagon',
        '*': 'star',
        'h': 'hexagon1',
        'H': 'hexagon2',
        '+': 'plus',
        'x': 'x',
        'D': 'diamond',
        'd': 'thin_diamond',
        '|': 'vline',
        '_': 'hline',
        'P': 'plus_filled',
        'X': 'x_filled',
        TICKLEFT: 'tickleft',
        TICKRIGHT: 'tickright',
        TICKUP: 'tickup',
        TICKDOWN: 'tickdown',
        CARETLEFT: 'caretleft',
        CARETRIGHT: 'caretright',
        CARETUP: 'caretup',
        CARETDOWN: 'caretdown',
        CARETLEFTBASE: 'caretleftbase',
        CARETRIGHTBASE: 'caretrightbase',
        CARETUPBASE: 'caretupbase',
        CARETDOWNBASE: 'caretdownbase',
        "None": 'nothing',
        "none": 'nothing',
        ' ': 'nothing',
        '': 'nothing'
    }

    # Just used for informational purposes.  is_filled()
    # is calculated in the _set_* functions.
    filled_markers = (
        '.', 'o', 'v', '^', '<', '>', '8', 's', 'p', '*', 'h', 'H', 'D', 'd',
        'P', 'X')

    fillstyles = ('full', 'left', 'right', 'bottom', 'top', 'none')
    _half_fillstyles = ('left', 'right', 'bottom', 'top')

    def __init__(self, marker,
                 fillstyle=None, transform=None, capstyle=None, joinstyle=None):
        """
        Parameters
        ----------
        marker : str, array-like, Path, MarkerStyle
            - Another instance of `MarkerStyle` copies the details of that *marker*.
            - For other possible marker values, see the module docstring
              `matplotlib.markers`.

        fillstyle : str, default: :rc:`markers.fillstyle`
            One of 'full', 'left', 'right', 'bottom', 'top', 'none'.

        transform : `~matplotlib.transforms.Transform`, optional
            Transform that will be combined with the native transform of the
            marker.

        capstyle : `.CapStyle` or %(CapStyle)s, optional
            Cap style that will override the default cap style of the marker.

        joinstyle : `.JoinStyle` or %(JoinStyle)s, optional
            Join style that will override the default join style of the marker.
        """
        self._spec = None
        self._user_transform = transform
        self._user_capstyle = CapStyle(capstyle) if capstyle is not None else None
        self._user_joinstyle = JoinStyle(joinstyle) if joinstyle is not None else None
        self._set_fillstyle(fillstyle)
        self._set_marker(marker)

    def _recache(self):
        """Resolve the marker into its paths and transforms (by pltrs (Rust))."""
        if self._spec is None:
            return
        ((path, mtx), alt, (self._snap_threshold, joinstyle, capstyle,
                            self._filled)) = _markers.resolve(
            self._spec, self._fillstyle,
            None if self._user_capstyle is None else self._user_capstyle.name,
            None if self._user_joinstyle is None else self._user_joinstyle.name)
        kind, given = self._spec
        if kind == "path":
            self._path = given
        else:
            self._path = (_builtin_path if kind in _BUILTIN_KINDS else Path)(*path)
        self._transform = Affine2D(mtx)
        if alt is None:
            self._alt_path = None
            self._alt_transform = None
        else:
            self._alt_path = _builtin_path(*alt[0])
            self._alt_transform = Affine2D(alt[1])
        self._joinstyle = JoinStyle(joinstyle)
        self._capstyle = CapStyle(capstyle)

    def __bool__(self):
        return bool(len(self._path.vertices))

    def is_filled(self):
        return self._filled

    def get_fillstyle(self):
        return self._fillstyle

    def _set_fillstyle(self, fillstyle):
        """
        Set the fillstyle.

        Parameters
        ----------
        fillstyle : {'full', 'left', 'right', 'bottom', 'top', 'none'}
            The part of the marker surface that is colored with
            markerfacecolor.
        """
        if fillstyle is None:
            fillstyle = _rc('markers.fillstyle')
        _api.check_in_list(self.fillstyles, fillstyle=fillstyle)
        self._fillstyle = fillstyle

    def get_joinstyle(self):
        return self._joinstyle.name

    def get_capstyle(self):
        return self._capstyle.name

    def get_marker(self):
        return self._marker

    def _set_marker(self, marker):
        """
        Set the marker.

        Parameters
        ----------
        marker : str, array-like, Path, MarkerStyle
            - Another instance of `MarkerStyle` copies the details of that *marker*.
            - For other possible marker values see the module docstring
              `matplotlib.markers`.
        """
        if isinstance(marker, str) and _markers.is_math_text(marker):
            self._spec = ("math", marker)
        elif isinstance(marker, Hashable) and marker in self.markers:
            self._spec = ("code", marker)
        elif (isinstance(marker, np.ndarray) and marker.ndim == 2 and
                marker.shape[1] == 2):
            self._spec = ("vertices", np.asarray(marker, float))
        elif isinstance(marker, Path):
            self._spec = ("path", marker)
        elif (isinstance(marker, Sized) and len(marker) in (2, 3) and
                marker[1] in (0, 1, 2)):
            numsides, rotation = (marker[0], 0.0) if len(marker) == 2 else (
                marker[0], marker[2])
            self._spec = ("polygon", (int(numsides), int(marker[1]),
                                      float(rotation)))
        elif isinstance(marker, MarkerStyle):
            self.__dict__ = copy.deepcopy(marker.__dict__)
        else:
            try:
                vertices = Path(marker).vertices
            except ValueError as err:
                raise ValueError(
                    f'Unrecognized marker style {marker!r}') from err
            self._spec = ("vertices", vertices)
        if not isinstance(marker, MarkerStyle):
            self._marker = marker
            self._recache()

    def get_path(self):
        """
        Return a `.Path` for the primary part of the marker.

        For unfilled markers this is the whole marker, for filled markers,
        this is the area to be drawn with *markerfacecolor*.
        """
        return self._path

    def get_transform(self):
        """
        Return the transform to be applied to the `.Path` from
        `MarkerStyle.get_path()`.
        """
        if self._user_transform is None:
            return self._transform.frozen()
        else:
            return (self._transform + self._user_transform).frozen()

    def get_alt_path(self):
        """
        Return a `.Path` for the alternate part of the marker.

        For unfilled markers, this is *None*; for filled markers, this is the
        area to be drawn with *markerfacecoloralt*.
        """
        return self._alt_path

    def get_alt_transform(self):
        """
        Return the transform to be applied to the `.Path` from
        `MarkerStyle.get_alt_path()`.
        """
        if self._user_transform is None:
            return self._alt_transform.frozen()
        else:
            return (self._alt_transform + self._user_transform).frozen()

    def get_snap_threshold(self):
        return self._snap_threshold

    def get_user_transform(self):
        """Return user supplied part of marker transform."""
        if self._user_transform is not None:
            return self._user_transform.frozen()

    def transformed(self, transform):
        """
        Return a new version of this marker with the transform applied.

        Parameters
        ----------
        transform : `~matplotlib.transforms.Affine2D`
            Transform will be combined with current user supplied transform.
        """
        new_marker = MarkerStyle(self)
        if new_marker._user_transform is not None:
            new_marker._user_transform += transform
        else:
            new_marker._user_transform = transform
        return new_marker

    def rotated(self, *, deg=None, rad=None):
        """
        Return a new version of this marker rotated by specified angle.

        Parameters
        ----------
        deg : float, optional
            Rotation angle in degrees.

        rad : float, optional
            Rotation angle in radians.

        .. note:: You must specify exactly one of deg or rad.
        """
        if deg is None and rad is None:
            raise ValueError('One of deg or rad is required')
        if deg is not None and rad is not None:
            raise ValueError('Only one of deg and rad can be supplied')
        new_marker = MarkerStyle(self)
        if new_marker._user_transform is None:
            new_marker._user_transform = Affine2D()

        if deg is not None:
            new_marker._user_transform.rotate_deg(deg)
        if rad is not None:
            new_marker._user_transform.rotate(rad)

        return new_marker

    def scaled(self, sx, sy=None):
        """
        Return new marker scaled by specified scale factors.

        If *sy* is not given, the same scale is applied in both the *x*- and
        *y*-directions.

        Parameters
        ----------
        sx : float
            *X*-direction scaling factor.
        sy : float, optional
            *Y*-direction scaling factor.
        """
        if sy is None:
            sy = sx

        new_marker = MarkerStyle(self)
        _transform = new_marker._user_transform or Affine2D()
        new_marker._user_transform = _transform.scale(sx, sy)
        return new_marker


# What matplotlib's module also has from its imports, which programs
# import from it.
from . import cbook  # noqa: E402, F401
from .transforms import IdentityTransform  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
