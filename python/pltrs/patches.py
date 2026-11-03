"""
Patches are `.Artist`\\s with a face color and an edge color
(matplotlib's ``patches``).

A patch wraps the native handle of a pltrs (Rust) patch, typed by its
shape: the `Patch` base class reads and writes the properties every
patch has through ``_native.patches.get`` and ``_native.patches.update``,
and each subclass reads and writes its geometry through the native
functions of its shape.
"""

from numbers import Number

import numpy as np

from . import _api, _mirrors, _values, cbook, colors as mcolors, units
from ._enums import CapStyle, JoinStyle
from ._fancy_styles import FancyStylesMixin, _style_object
from ._native import patches as _patches
from ._patch_styles import (  # noqa: F401  (matplotlib.patches exports them)
    ArrowStyle, BoxStyle, ConnectionStyle, _native_spec, _Style)

cbook._set_module(__name__, ArrowStyle, BoxStyle, ConnectionStyle, _Style)
from ._program_classes import takes_from_program
from .artist import Artist
from .path import Path, drawn_path
from ._transform_trees import set_matrix
from .transforms import Affine2D, BboxTransformTo, IdentityTransform


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from . import rcParams
    return rcParams


def _number_text(value):
    """
    *value* as matplotlib's ``%s`` shows the number a patch was made with:
    pltrs (Rust) keeps floats, so an integral one shows as the integer it
    most likely was (None stays None).
    """
    if value is None:
        return "None"
    value = float(value)
    return str(int(value)) if value.is_integer() and abs(value) < 1e16 else str(value)


@_api.define_aliases({
    "antialiased": ["aa"],
    "edgecolor": ["ec"],
    "facecolor": ["fc"],
    "linestyle": ["ls"],
    "linewidth": ["lw"],
})
class Patch(Artist):
    """
    A patch is a 2D artist with a face color and an edge color.

    If any of *edgecolor*, *facecolor*, *linewidth*, or *antialiased*
    are *None*, they default to their rc params setting.
    """
    zorder = 1

    # Whether to draw an edge by default.  Set on a
    # subclass-by-subclass basis.
    _edge_default = False

    def __init__(self, *,
                 edgecolor=None,
                 facecolor=None,
                 color=None,
                 linewidth=None,
                 linestyle=None,
                 antialiased=None,
                 hatch=None,
                 fill=True,
                 capstyle=None,
                 joinstyle=None,
                 hatchcolor=None,
                 edgegapcolor=None,
                 **kwargs):
        """
        Style the patch a subclass holds in ``self._native``: the patch is
        outside any figure until an axes or a figure adds it
        (`~.Axes.add_patch`, `.Figure.add_artist`). A bare `Patch` (a legend
        proxy, say) has an empty path.
        """
        if "_native" not in self.__dict__:
            self._native = _patches.new_path_patch(Path(np.empty((0, 2))))
        super().__init__()
        if linestyle is None:
            linestyle = "solid"
        if capstyle is None:
            capstyle = CapStyle.butt
        if joinstyle is None:
            joinstyle = JoinStyle.miter
        self.set_fill(fill)
        if color is not None:
            if edgecolor is not None or facecolor is not None:
                _api.warn_external(
                    "Setting the 'color' property will override "
                    "the edgecolor or facecolor properties.")
            self.set_color(color)
        else:
            self.set_edgecolor(edgecolor)
            self.set_hatchcolor(hatchcolor)
            self.set_facecolor(facecolor)
        self.set_linestyle(linestyle)
        self.set_linewidth(linewidth)
        self.set_antialiased(antialiased)
        self.set_hatch(hatch)
        self.set_capstyle(capstyle)
        self.set_joinstyle(joinstyle)
        if edgegapcolor is not None:
            self.set_edgegapcolor(edgegapcolor)
        if len(kwargs):
            self._internal_update(kwargs)

    def _get(self, name):
        """The native property *name*."""
        return _patches.get(self._native, name)

    def get_edgegapcolor(self):
        """Return the edge gap color."""
        return self._get("edgegapcolor")

    def set_edgegapcolor(self, edgegapcolor):
        """Set a color to fill the gaps in the dashed edge style."""
        self._set_native(edgegapcolor=edgegapcolor)

    def get_verts(self):
        """
        Return a copy of the vertices used in this patch.

        If the patch contains Bézier curves, the curves will be interpolated by
        line segments.  To access the curves as curves, use `get_path`.
        """
        trans = self.get_transform()
        path = self.get_path()
        polygons = path.to_polygons(trans)
        if len(polygons):
            return polygons[0]
        return []

    def contains(self, mouseevent, radius=None):
        """
        Test whether the mouse event occurred in the patch.

        Parameters
        ----------
        mouseevent : `~matplotlib.backend_bases.MouseEvent`
            Where the user clicked.

        radius : float, optional
            Additional margin on the patch in target coordinates of
            `.Patch.get_transform`. If `None`, the picker if it is a number,
            else the edge's line width (0 for an invisible edge).

        Returns
        -------
        (bool, empty dict)
        """
        return self._contains_within(mouseevent,
                                     float(self._process_radius(radius)))

    def _set_native(self, **kwargs):
        """Set native properties by their resolved names, in order."""
        _patches.update(self._native, kwargs)

    def get_path(self):
        """Return the path of this patch."""
        return drawn_path(self, *_patches.get_path(self._native), (
            lambda: _patches.get_interpolation_steps(self._native),
            lambda steps: _patches.set_interpolation_steps(self._native, steps)))

    def get_patch_transform(self):
        """
        Return the `~.transforms.Transform` instance mapping patch
        coordinates to data coordinates.

        For example, one may define a patch of a circle which represents a
        radius of 5 by providing coordinates for a unit circle, and a
        transform which scales the coordinates (the patch coordinate) by 5.
        """
        return IdentityTransform()

    def _native_patch_transform(self):
        """The `.Affine2D` of the patch coordinates pltrs (Rust) has for
        the patch, made anew."""
        return Affine2D(_patches.get_patch_transform(self._native))

    def _kept_patch_transform(self):
        """The patch transform as the classes keeping one `.Affine2D`
        (matplotlib's ``_patch_transform``) give it: the same object, with
        pltrs (Rust)'s matrix."""
        transform = self.__dict__.get("_patch_transform")
        if transform is None:
            transform = self._patch_transform = Affine2D()
        set_matrix(transform, _patches.get_patch_transform(self._native))
        return transform

    def get_transform(self):
        """Return the `~.transforms.Transform` applied to the `Patch`."""
        return self.get_patch_transform() + Artist.get_transform(self)

    def get_data_transform(self):
        """
        Return the `~.transforms.Transform` mapping data coordinates to
        physical coordinates.
        """
        return Artist.get_transform(self)

    def get_window_extent(self, renderer=None):
        """Return the patch's box in display space: its path through its
        transform as it is (matplotlib's)."""
        return self.get_path().get_extents(self.get_transform())

    def get_extents(self):
        """
        Return the `Patch`'s axis-aligned extents as a `~.transforms.Bbox`.
        """
        return self.get_path().get_extents(self.get_transform())

    def update_from(self, other):
        """Copy properties from *other* to *self*."""
        super().update_from(other)
        _patches.update_style_from(self._native, other._native)
        if "_linestyle" in other.__dict__:
            self._linestyle = other._linestyle

    def _process_radius(self, radius):
        """The tolerance of a containment test: *radius*, else a number
        given as the picker, else the edge's line width (0 without an
        edge)."""
        if radius is not None:
            return radius
        picker = self.get_picker()
        if isinstance(picker, Number):
            return picker
        if self.get_edgecolor()[3] == 0:
            return 0
        return self.get_linewidth()

    def _pick_radius(self):
        return float(self._process_radius(None))

    def get_antialiased(self):
        """Return whether antialiasing is used for drawing."""
        antialiased = self._get("antialiased")
        return (_rcParams()['patch.antialiased'] if antialiased is None
                else antialiased)

    def set_antialiased(self, aa):
        """Set whether to use antialiased rendering (None: the rc default)."""
        self._set_native(antialiased=None if aa is None else bool(aa))

    def get_edgecolor(self):
        """Return the edge color."""
        return self._rgba_as_given("edgecolor", self.get_alpha())

    def set_edgecolor(self, color):
        """Set the patch edge color (None: the default rule)."""
        if color is not None:
            mcolors.to_rgba(color)
        self._set_native(edgecolor=color)
        _values.remember(self, "edgecolor", color)

    def get_facecolor(self):
        """Return the face color."""
        return self._rgba_as_given(
            "facecolor", self.get_alpha() if self.get_fill() else 0)

    def set_facecolor(self, color):
        """Set the patch face color (None: :rc:`patch.facecolor`)."""
        if color is not None:
            mcolors.to_rgba(color)
        self._set_native(facecolor=color)
        _values.remember(self, "facecolor", color)

    #: The colours the class gives its patches when none is given
    #: (matplotlib's constructor defaults: a `.table.Cell`'s).
    _default_colors = {}

    def _take_cycle_color(self, name):
        """Keep the colour *name* pltrs (Rust) took from the property cycle
        as the cycle holds it, as the plotting methods handing a patch the
        cycle's colour itself (`~.Axes.pie`, `~.Axes.stairs`) keep it."""
        color = _values.cycle_color(self.axes, self._get(name))
        if color is not _values.NONE_GIVEN:
            _values.remember(self, name, color)

    def _rgba_as_given(self, name, alpha):
        """
        The colour *name* as matplotlib keeps it, ``to_rgba(color, alpha)``
        of the colour given (:rc:`patch.facecolor` for a face given as None;
        the class' default for none), while pltrs (Rust) holds that colour;
        else pltrs (Rust)'s.
        """
        native = self._get(name)
        color = self.__dict__.get("_given_values", {}).get(name, _values.NONE_GIVEN)
        if color is None and name == "facecolor":
            color = _rcParams()["patch.facecolor"]
        if color is _values.NONE_GIVEN:
            color = self._default_colors.get(name, _values.NONE_GIVEN)
        if color is None or color is _values.NONE_GIVEN:
            return native
        try:
            rgba = mcolors.to_rgba(color, alpha)
        except ValueError:
            return native
        return rgba if rgba == tuple(native) else native

    def get_hatchcolor(self):
        """Return the hatch color."""
        # The edge colour, typed as matplotlib keeps it, when the hatch
        # takes it.
        native = self._get("hatchcolor")
        edge = self.get_edgecolor()
        return edge if tuple(edge) == tuple(native) else native

    def set_hatchcolor(self, color):
        """Set the patch hatch color: a color, 'edge' or None (the rc)."""
        if color is None:
            color = _rcParams()['hatch.color']
        if not (isinstance(color, str) and color == 'edge'):
            mcolors.to_rgba(color)
        self._set_native(hatchcolor=color)

    def set_color(self, c):
        """Set both the edgecolor and the facecolor."""
        self.set_edgecolor(c)
        self.set_hatchcolor(c)
        self.set_facecolor(c)

    def get_linewidth(self):
        """Return the line width in points."""
        return self._get("linewidth")

    def set_linewidth(self, w):
        """Set the patch linewidth in points (None: the rc default)."""
        self._set_native(linewidth=None if w is None else float(w))

    def get_linestyle(self):
        """Return the linestyle, as it was set."""
        if "_linestyle" in self.__dict__:
            return self._linestyle
        linestyle = self._get("linestyle")
        if isinstance(linestyle, tuple):
            # An (offset, on-off sequence) pattern, its sequence a tuple as
            # matplotlib's dash patterns are.
            offset, dashes = linestyle
            return offset, None if dashes is None else tuple(dashes)
        # A patch pltrs (Rust) made keeps the style matplotlib's constructor
        # sets when none is given.
        return 'solid' if linestyle == '-' else linestyle

    def set_linestyle(self, ls):
        """
        Set the patch linestyle: one of ``'-'``, ``'--'``, ``'-.'``,
        ``':'``, ``''`` (and their long names), or an ``(offset,
        onoffseq)`` tuple.
        """
        if ls is None:
            ls = "solid"
        if ls in [' ', '', 'none']:
            ls = 'None'
        self._linestyle = ls
        if isinstance(ls, tuple):
            offset, seq = ls
            ls = (offset, None if seq is None else tuple(seq))
        self._set_native(linestyle=ls)

    def _has_dashed_edge(self):
        """
        Return whether the patch edge has a dashed linestyle.

        A custom linestyle is assumed to be dashed, we do not inspect the
        ``onoffseq`` directly.
        """
        return self.get_linestyle() not in ('solid', '-')

    def get_fill(self):
        """Return whether the patch is filled."""
        return self._get("fill")

    def set_fill(self, b):
        """Set whether to fill the patch."""
        self._set_native(fill=bool(b))

    fill = property(get_fill, set_fill)

    def get_capstyle(self):
        """Return the capstyle."""
        return self._get("capstyle")

    def set_capstyle(self, s):
        """Set the `.CapStyle`."""
        self._set_native(capstyle=CapStyle(s).name)

    def get_joinstyle(self):
        """Return the joinstyle."""
        return self._get("joinstyle")

    def set_joinstyle(self, s):
        """Set the `.JoinStyle`."""
        self._set_native(joinstyle=JoinStyle(s).name)

    def get_hatch(self):
        """Return the hatching pattern."""
        return self._get("hatch")

    def set_hatch(self, hatch):
        """Set the hatching pattern (see `.hatch`)."""
        from . import hatch as mhatch
        mhatch._validate_hatch_pattern(hatch)
        self._set_native(hatch=hatch)

    def get_hatch_linewidth(self):
        """Return the hatch linewidth."""
        return _values.given(self, "hatch_linewidth", self._get("hatch_linewidth"))

    def set_hatch_linewidth(self, lw):
        """Set the hatch linewidth."""
        self._set_native(hatch_linewidth=float(lw))
        _values.remember(self, "hatch_linewidth", lw)

    def contains_point(self, point, radius=None):
        """
        Return whether the given point is inside the patch.

        Parameters
        ----------
        point : (float, float)
            The point (x, y) to check, in target coordinates of
            ``.Patch.get_transform()``. These are display coordinates for patches
            that are added to a figure or Axes.
        radius : float, optional
            Additional margin on the patch in target coordinates of
            `.Patch.get_transform`. See `.Path.contains_point` for further
            details.
        """
        radius = self._process_radius(radius)
        return self.get_path().contains_point(point, self.get_transform(),
                                              radius)

    def contains_points(self, points, radius=None):
        """
        Return whether the given points are inside the patch.

        Parameters
        ----------
        points : (N, 2) array
            The points to check, in target coordinates of
            ``self.get_transform()``. These are display coordinates for patches
            that are added to a figure or Axes. Columns contain x and y values.
        radius : float, optional
            Additional margin on the patch in target coordinates of
            `.Patch.get_transform`. See `.Path.contains_point` for further
            details.
        """
        radius = self._process_radius(radius)
        return self.get_path().contains_points(points, self.get_transform(),
                                               radius)


@_mirrors.mirrorable
class Rectangle(Patch):
    """
    A rectangle defined via an anchor point *xy* and its *width* and *height*.

    The rectangle extends from ``xy[0]`` to ``xy[0] + width`` in x-direction
    and from ``xy[1]`` to ``xy[1] + height`` in y-direction.
    """

    def __str__(self):
        pars = self.get_x(), self.get_y(), self.get_width(), self.get_height(), self.angle
        fmt = "Rectangle(xy=(%g, %g), width=%g, height=%g, angle=%g)"
        return fmt % pars

    def __init__(self, xy, width, height, *,
                 angle=0.0, rotation_point='xy', **kwargs):
        """
        Parameters
        ----------
        xy : (float, float)
            The anchor point.
        width : float
            Rectangle width.
        height : float
            Rectangle height.
        angle : float, default: 0
            Rotation in degrees anti-clockwise about the rotation point.
        rotation_point : {'xy', 'center', (number, number)}, default: 'xy'
            If ``'xy'``, rotate around the anchor point. If ``'center'``
            rotate around the center. If 2-tuple of number, rotate around
            this coordinate.
        """
        x0, y0 = xy
        if all(units._is_natively_supported(v) for v in (x0, y0, width, height)):
            self._native = _patches.new_rectangle(
                (float(x0), float(y0)), (float(width), float(height)),
                float(angle))
            _values.remember_all(self, {"x": x0, "y": y0, "width": width,
                                        "height": height, "angle": float(angle)},
                                 self._FIELDS)
        else:
            # Values in units convert once the rectangle joins an axes; the
            # far corner is summed now, which checks that the units add up,
            # as matplotlib's constructor does.
            self._unit_bounds = (x0, y0, x0 + width, y0 + height)
            self._native = _patches.new_rectangle((0., 0.), (0., 0.), float(angle))
        self.rotation_point = rotation_point
        super().__init__(**kwargs)

    def _convert_units_for(self, axes):
        bounds = self.__dict__.pop("_unit_bounds", None)
        if bounds is None:
            return
        x0, y0, x1, y1 = bounds
        x0, x1 = (float(axes.convert_xunits(x)) for x in (x0, x1))
        y0, y1 = (float(axes.convert_yunits(y)) for y in (y0, y1))
        self._set_fields(x=x0, y=y0, width=x1 - x0, height=y1 - y0)

    #: The names of `_fields`.
    _FIELDS = ("x", "y", "width", "height", "angle", "rotation_point")

    def _fields(self):
        """``(x, y, width, height, angle, rotation_point)``, as given
        (matplotlib keeps them as given)."""
        return _values.given_fields(self, self._FIELDS,
                                    _patches.rectangle(self._native))

    def _set_fields(self, x=None, y=None, width=None, height=None, angle=None,
                    rotation_point=None):
        given = (x, y, width, height, angle, rotation_point)
        current = self._fields()
        values = [current[i] if value is None else value
                  for i, value in enumerate(given)]
        _patches.set_rectangle(self._native, (float(values[0]), float(values[1])),
                               (float(values[2]), float(values[3])),
                               float(values[4]), values[5])
        _values.remember_all(self, {name: value for name, value in zip(self._FIELDS, given)
                                    if value is not None}, self._FIELDS)

    @property
    def rotation_point(self):
        """The rotation point of the patch."""
        return self._fields()[5]

    @rotation_point.setter
    def rotation_point(self, value):
        self._set_fields(rotation_point=value)

    @property
    def angle(self):
        """Rotation angle in degrees (anti-clockwise)."""
        return self._fields()[4]

    @angle.setter
    def angle(self, value):
        self._set_fields(angle=value)

    def get_x(self):
        """Return the left coordinate of the rectangle."""
        return self._fields()[0]

    def get_y(self):
        """Return the bottom coordinate of the rectangle."""
        return self._fields()[1]

    def get_xy(self):
        """Return the left and bottom coords of the rectangle as a tuple."""
        return self._fields()[:2]

    def get_corners(self):
        """
        Return the corners of the rectangle, moving anti-clockwise from
        (x0, y0).
        """
        return self.get_patch_transform().transform(
            [(0, 0), (1, 0), (1, 1), (0, 1)])

    def get_center(self):
        """Return the centre of the rectangle."""
        return self.get_patch_transform().transform((0.5, 0.5))

    def get_width(self):
        """Return the width of the rectangle."""
        return self._fields()[2]

    def get_height(self):
        """Return the height of the rectangle."""
        return self._fields()[3]

    def get_angle(self):
        """Get the rotation angle in degrees."""
        return self.angle

    def set_x(self, x):
        """Set the left coordinate of the rectangle."""
        self._set_fields(x=x)

    def set_y(self, y):
        """Set the bottom coordinate of the rectangle."""
        self._set_fields(y=y)

    def set_angle(self, angle):
        """Set the rotation angle in degrees."""
        self.angle = angle

    def set_xy(self, xy):
        """Set the left and bottom coordinates of the rectangle."""
        self._set_fields(x=xy[0], y=xy[1])

    def set_width(self, w):
        """Set the width of the rectangle."""
        self._set_fields(width=w)

    def set_height(self, h):
        """Set the height of the rectangle."""
        self._set_fields(height=h)

    def set_bounds(self, *args):
        """Set the bounds of the rectangle as *left*, *bottom*, *width*, *height*."""
        if len(args) == 1:
            l, b, w, h = args[0]
        else:
            l, b, w, h = args
        self._set_fields(l, b, w, h)

    def get_bbox(self):
        """Return the `.Bbox`."""
        from .transforms import Bbox
        return Bbox.from_extents(*self._convert_bounds())

    def _convert_bounds(self):
        x, y, width, height = self._fields()[:4]
        return x, y, x + width, y + height

    #: The correction of the rotation for unequal axis scales (matplotlib's;
    #: the selector widgets set it on their rectangles).
    _aspect_ratio_correction = 1.0

    def get_patch_transform(self):
        # Note: This cannot be called until after this has been added to
        # an Axes, otherwise unit conversion will fail. This makes it very
        # important to call the accessor method and not directly access the
        # transformation member variable.
        bbox = self.get_bbox()
        if self.rotation_point == 'center':
            width, height = bbox.x1 - bbox.x0, bbox.y1 - bbox.y0
            rotation_point = bbox.x0 + width / 2., bbox.y0 + height / 2.
        elif self.rotation_point == 'xy':
            rotation_point = bbox.x0, bbox.y0
        else:
            rotation_point = self.rotation_point
        return BboxTransformTo(bbox) \
            + Affine2D() \
            .translate(-rotation_point[0], -rotation_point[1]) \
            .scale(1, self._aspect_ratio_correction) \
            .rotate_deg(self.angle) \
            .scale(1, 1 / self._aspect_ratio_correction) \
            .translate(*rotation_point)

    xy = property(get_xy, set_xy)


class RegularPolygon(Patch):
    """A regular polygon patch."""

    get_patch_transform = Patch._kept_patch_transform

    def __str__(self):
        s = "RegularPolygon((%g, %g), %d, radius=%g, orientation=%g)"
        return s % (*self.xy, self.numvertices, self.radius, self.orientation)

    def __init__(self, xy, numVertices, *,
                 radius=5, orientation=0, **kwargs):
        """
        Parameters
        ----------
        xy : (float, float)
            The center position.
        numVertices : int
            The number of vertices.
        radius : float
            The distance from the center to each of the vertices.
        orientation : float
            The polygon is rotated by *orientation* radians.
        """
        self._native = _patches.new_regular_polygon(
            (float(xy[0]), float(xy[1])), int(numVertices), float(radius),
            float(orientation))
        _values.remember_all(self, {"xy": xy, "radius": radius,
                                    "orientation": orientation},
                             ("xy", "radius", "orientation"))
        super().__init__(**kwargs)

    def _fields(self):
        """``(x, y, numvertices, radius, orientation)``."""
        return _patches.regular_polygon(self._native)

    @property
    def xy(self):
        """The center of the polygon."""
        return _values.given(self, "xy", self._fields()[:2])

    @property
    def numvertices(self):
        """The number of vertices."""
        return self._fields()[2]

    @property
    def radius(self):
        """The distance from the center to each of the vertices."""
        return _values.given(self, "radius", self._fields()[3])

    @property
    def orientation(self):
        """The polygon's rotation, in radians."""
        return _values.given(self, "orientation", self._fields()[4])


class PathPatch(Patch):
    """A general polycurve path patch."""

    _edge_default = True

    def __str__(self):
        s = "PathPatch%d((%g, %g) ...)"
        return s % (len(self.get_path().vertices), *tuple(self.get_path().vertices[0]))

    def __init__(self, path, **kwargs):
        """
        *path* is a `.Path` object.
        """
        self._native = _patches.new_path_patch(path)
        super().__init__(**kwargs)

    def set_path(self, path):
        """Set the path of the patch."""
        _patches.set_path_shape(self._native, path)

    # matplotlib keeps the path as ``_path``, which subclasses read.
    _path = property(lambda self: self.get_path(), set_path)


class StepPatch(PathPatch):
    """
    A path patch describing a stepwise constant function.

    By default, the path is not closed and starts and stops at
    baseline value.
    """

    _edge_default = False

    def __init__(self, values, edges, *,
                 orientation='vertical', baseline=0, **kwargs):
        """
        Parameters
        ----------
        values : array-like
            The step heights.
        edges : array-like
            The edge positions, with ``len(edges) == len(vals) + 1``,
            between which the curve takes on vals values.
        orientation : {'vertical', 'horizontal'}, default: 'vertical'
            The direction of the steps.
        baseline : float, array-like or None, default: 0
            The bottom value of the bounding edges or when
            ``fill=True``, position of lower edge.
        """
        _api.check_in_list(('vertical', 'horizontal'), orientation=orientation)
        self._native = _patches.new_step_patch(
            np.asarray(values, dtype=float), np.asarray(edges, dtype=float),
            None if baseline is None else np.asarray(baseline, dtype=float),
            orientation == 'horizontal')
        # The native step patch holds its path: no `PathPatch.__init__`.
        Patch.__init__(self, **kwargs)

    def get_data(self):
        """Get `.StepPatch` values, edges and baseline as namedtuple."""
        from collections import namedtuple
        StairData = namedtuple('StairData', 'values edges baseline')
        return StairData(*_patches.step_patch(self._native))

    def set_data(self, values=None, edges=None, baseline=None):
        """
        Set `.StepPatch` values, edges and baseline.

        Parameters
        ----------
        values : 1D array-like or None
            Will not update values, if passing None
        edges : 1D array-like, optional
        baseline : float, 1D array-like or None
        """
        if values is None and edges is None and baseline is None:
            raise ValueError("Must set *values*, *edges* or *baseline*.")
        _patches.set_step_patch(
            self._native,
            None if values is None else np.asarray(values, dtype=float),
            None if edges is None else np.asarray(edges, dtype=float),
            (baseline is None,
             None if baseline is None else np.asarray(baseline, dtype=float)))


@_mirrors.mirrorable
class Polygon(Patch):
    """A general polygon patch."""

    def __str__(self):
        if len(self.get_path().vertices):
            s = "Polygon%d((%g, %g) ...)"
            return s % (len(self.get_path().vertices),
                        *self.get_path().vertices[0])
        else:
            return "Polygon0()"

    def __init__(self, xy, *, closed=True, **kwargs):
        """
        Parameters
        ----------
        xy : (N, 2) array
        closed : bool, default: True
            Whether the polygon is closed (i.e., has identical start and end
            points).
        """
        self._native = _patches.new_polygon(
            np.asarray(xy, dtype=float).reshape(-1, 2), bool(closed))
        super().__init__(**kwargs)

    def get_closed(self):
        """Return whether the polygon is closed."""
        return _patches.polygon(self._native)[1]

    def set_closed(self, closed):
        """
        Set whether the polygon is closed.

        Parameters
        ----------
        closed : bool
            True if the polygon is closed
        """
        if self.get_closed() == bool(closed):
            return
        _patches.set_polygon(self._native, self.get_xy(), bool(closed))

    def get_xy(self):
        """
        Get the vertices of the path.

        Returns
        -------
        (N, 2) array
            The coordinates of the vertices.
        """
        return _patches.polygon(self._native)[0]

    def set_xy(self, xy):
        """
        Set the vertices of the polygon.

        Parameters
        ----------
        xy : (N, 2) array-like
            The coordinates of the vertices.
        """
        xy = np.asarray(xy, dtype=float).reshape(-1, 2)
        _patches.set_polygon(self._native, xy, self.get_closed())

    xy = property(get_xy, set_xy,
                  doc='The vertices of the path as a (N, 2) array.')


@_mirrors.mirrorable
class Wedge(Patch):
    """Wedge shaped patch."""

    def __str__(self):
        pars = (self.center[0], self.center[1], self.r,
                self.theta1, self.theta2, self.width)
        fmt = "Wedge(center=(%g, %g), r=%g, theta1=%g, theta2=%g, width=%s)"
        return fmt % (*pars[:-1], _number_text(pars[-1]))

    def __init__(self, center, r, theta1, theta2, *, width=None, **kwargs):
        """
        A wedge centered at *x*, *y* center with radius *r* that
        sweeps *theta1* to *theta2* (in degrees).  If *width* is given,
        then a partial wedge is drawn from inner radius *r* - *width*
        to outer radius *r*.
        """
        self._native = _patches.new_wedge(
            (float(center[0]), float(center[1])), float(r),
            (float(theta1), float(theta2)), None if width is None else float(width))
        _values.remember_all(self, {"center": center, "r": r, "theta1": theta1,
                                    "theta2": theta2, "width": width}, self._GIVEN)
        super().__init__(**kwargs)

    #: The geometry matplotlib keeps as given.
    _GIVEN = ("center", "r", "theta1", "theta2", "width")

    def _fields(self):
        """``(x, y, r, theta1, theta2, width)``."""
        return _patches.wedge(self._native)

    def _set_fields(self, center=None, r=None, theta1=None, theta2=None,
                    width=None, *, clear_width=False):
        x, y, r0, t1, t2, w = self._fields()
        given = {"center": center, "r": r, "theta1": theta1, "theta2": theta2,
                 "width": width}
        center = (x, y) if center is None else center
        _patches.set_wedge(
            self._native, (float(center[0]), float(center[1])),
            float(r0 if r is None else r),
            (float(t1 if theta1 is None else theta1),
             float(t2 if theta2 is None else theta2)),
            None if clear_width else (w if width is None else float(width)))
        _values.remember_all(self, {k: v for k, v in given.items() if v is not None},
                             self._GIVEN)

    def _given(self, name, index):
        """Field *name* (at *index* of `_fields`) as given."""
        fields = self._fields()
        native = fields[:2] if name == "center" else fields[index]
        return _values.given(self, name, native)

    @property
    def center(self):
        """The center of the wedge."""
        return self._given("center", 0)

    @property
    def r(self):
        """The radius of the wedge."""
        return self._given("r", 2)

    @property
    def theta1(self):
        """The start angle in degrees."""
        return self._given("theta1", 3)

    @property
    def theta2(self):
        """The end angle in degrees."""
        return self._given("theta2", 4)

    @property
    def width(self):
        """The ring width, or None for a full wedge."""
        return self._given("width", 5)

    def set_center(self, center):
        """Set the center of the wedge."""
        self._set_fields(center=center)

    def set_radius(self, radius):
        """Set the radius of the wedge."""
        self._set_fields(r=radius)

    def set_theta1(self, theta1):
        """Set the start angle in degrees."""
        self._set_fields(theta1=theta1)

    def set_theta2(self, theta2):
        """Set the end angle in degrees."""
        self._set_fields(theta2=theta2)

    def set_width(self, width):
        """Set the ring width (None: a full wedge)."""
        self._set_fields(width=width, clear_width=width is None)


class Arrow(Patch):
    """An arrow patch."""

    get_patch_transform = Patch._kept_patch_transform

    def __str__(self):
        return "Arrow()"

    def __init__(self, x, y, dx, dy, *, width=1.0, **kwargs):
        """
        Draws an arrow from (*x*, *y*) to (*x* + *dx*, *y* + *dy*).
        The width of the arrow is scaled by *width*.
        """
        self._native = _patches.new_arrow((float(x), float(y)), (float(dx), float(dy)),
                                   float(width))
        super().__init__(**kwargs)

    def set_data(self, x=None, y=None, dx=None, dy=None, width=None):
        """
        Set `.Arrow` x, y, dx, dy and width.
        Values left as None will not be updated.

        Parameters
        ----------
        x, y : float or None, default: None
            The x and y coordinates of the arrow base.

        dx, dy : float or None, default: None
            The length of the arrow along x and y direction.

        width : float or None, default: None
            Width of full arrow tail.
        """
        given = (x, y, dx, dy, width)
        x, y, dx, dy, width = (float(current if new is None else new)
                               for new, current in
                               zip(given, _patches.arrow(self._native)))
        _patches.set_arrow(self._native, (x, y), (dx, dy), width)
        self.stale = True


class FancyArrow(Polygon):
    """
    Like Arrow, but lets you set head width and head height independently.
    """

    _edge_default = True

    def __str__(self):
        return "FancyArrow()"

    def __init__(self, x, y, dx, dy, *,
                 width=0.001, length_includes_head=False, head_width=None,
                 head_length=None, shape='full', overhang=0,
                 head_starts_at_zero=False, **kwargs):
        """
        Parameters
        ----------
        x, y : float
            The x and y coordinates of the arrow base.
        dx, dy : float
            The length of the arrow along x and y direction.
        width : float, default: 0.001
            Width of full arrow tail.
        length_includes_head : bool, default: False
            True if head is to be counted in calculating the length.
        head_width : float or None, default: 3*width
            Total width of the full arrow head.
        head_length : float or None, default: 1.5*head_width
            Length of arrow head.
        shape : {'full', 'left', 'right'}, default: 'full'
            Draw the left-half, right-half, or full arrow.
        overhang : float, default: 0
            Fraction that the arrow is swept back (0 overhang means
            triangular shape).
        head_starts_at_zero : bool, default: False
            If True, the head starts being drawn at coordinate 0
            instead of ending at coordinate 0.
        """
        if np.size(np.hypot(dx, dy)) == 0:
            # An arrow of empty data displays nothing: pltrs (Rust) gives
            # an arrow of non-finite coordinates no vertices.
            x = y = dx = dy = np.nan
        self._native = _patches.new_fancy_arrow(
            (float(x), float(y)), (float(dx), float(dy)), float(width),
            (None if head_width is None else float(head_width),
             None if head_length is None else float(head_length),
             shape, float(overhang), bool(length_includes_head),
             bool(head_starts_at_zero)))
        Patch.__init__(self, **kwargs)

    @property
    def verts(self):
        """The vertices of the arrow's polygon."""
        return _patches.fancy_arrow_verts(self._native)

    def get_xy(self):
        """The vertices of the arrow's polygon, closed (`.Polygon.get_xy`)."""
        return self.get_path().vertices

    xy = property(get_xy, Polygon.set_xy,
                  doc='The vertices of the path as a (N, 2) array.')

    def set_data(self, *, x=None, y=None, dx=None, dy=None, width=None,
                 head_width=None, head_length=None):
        """
        Set `.FancyArrow` x, y, dx, dy, width, head_with, and head_length.
        Values left as None will not be updated.

        Parameters
        ----------
        x, y : float or None, default: None
            The x and y coordinates of the arrow base.

        dx, dy : float or None, default: None
            The length of the arrow along x and y direction.

        width : float or None, default: None
            Width of full arrow tail.

        head_width : float or None, default: None
            Total width of the full arrow head.

        head_length : float or None, default: None
            Length of arrow head.
        """
        def given(value):
            return None if value is None else float(value)

        _patches.set_fancy_arrow(
            self._native, tuple(map(given, (x, y, dx, dy))),
            tuple(map(given, (width, head_width, head_length))))
        self.stale = True

    def get_xy(self):
        """Return the vertices of the arrow."""
        return self.get_path().vertices

    def get_closed(self):
        """An arrow is closed."""
        return True


class CirclePolygon(RegularPolygon):
    """A polygon-approximation of a circle patch."""

    def __str__(self):
        s = "CirclePolygon((%g, %g), radius=%g, resolution=%d)"
        return s % (*self.xy, self.radius, self.numvertices)

    def __init__(self, xy, radius=5, *,
                 resolution=20,  # the number of vertices
                 **kwargs):
        """
        Create a circle at *xy* = (*x*, *y*) with given *radius*.

        This circle is approximated by a regular polygon with *resolution*
        sides.  For a smoother circle drawn with splines, see `Circle`.
        """
        self._native = _patches.new_circle_polygon(
            (float(xy[0]), float(xy[1])), float(radius), int(resolution))
        _values.remember_all(self, {"xy": xy, "radius": radius, "orientation": 0},
                             ("xy", "radius", "orientation"))
        Patch.__init__(self, **kwargs)

    def _fields(self):
        return _patches.circle_polygon(self._native)


class Ellipse(Patch):
    """A scale-free ellipse."""

    #: As `Rectangle`'s.
    _aspect_ratio_correction = 1.0

    get_patch_transform = Patch._native_patch_transform

    def __str__(self):
        pars = (self._center[0], self._center[1],
                self.width, self.height, self.angle)
        fmt = "Ellipse(xy=(%s, %s), width=%s, height=%s, angle=%s)"
        return fmt % tuple(map(_number_text, pars))

    def __init__(self, xy, width, height, *, angle=0, **kwargs):
        """
        Parameters
        ----------
        xy : (float, float)
            xy coordinates of ellipse centre.
        width : float
            Total length (diameter) of horizontal axis.
        height : float
            Total length (diameter) of vertical axis.
        angle : float, default: 0
            Rotation in degrees anti-clockwise.
        """
        center, size = self._native_geometry(xy, width, height)
        self._native = _patches.new_ellipse(center, size, float(angle))
        _values.remember_all(self, {"center": xy, "width": width, "height": height,
                                    "angle": angle}, self._GIVEN)
        super().__init__(**kwargs)

    #: The geometry matplotlib keeps as given.
    _GIVEN = ("center", "width", "height", "angle")

    def _native_geometry(self, xy, width, height):
        """The centre and size of the ellipse as numbers; values in units
        (``basic_units``' ``TaggedValue``) convert once the ellipse joins an
        Axes, as matplotlib converts them when it draws."""
        values = (xy[0], xy[1], width, height)
        if all(units._is_natively_supported(v) for v in values):
            return (float(xy[0]), float(xy[1])), (float(width), float(height))
        self._unit_geometry = values
        return (0., 0.), (0., 0.)

    def _convert_units_for(self, axes):
        values = self.__dict__.pop("_unit_geometry", None)
        if values is None:
            return
        x, y, width, height = values
        # matplotlib converts the width and height as coordinates too.
        self._set_fields(center=(float(axes.convert_xunits(x)),
                                 float(axes.convert_yunits(y))),
                         width=float(axes.convert_xunits(width)),
                         height=float(axes.convert_yunits(height)))

    def _fields(self):
        """``(x, y, width, height, angle)``."""
        return _patches.ellipse(self._native)

    def _set_fields(self, center=None, width=None, height=None, angle=None):
        x, y, w, h, a = self._fields()
        given = {"center": center, "width": width, "height": height, "angle": angle}
        center = (x, y) if center is None else center
        _patches.set_ellipse(self._native, (float(center[0]), float(center[1])),
                             float(w if width is None else width),
                             float(h if height is None else height),
                             float(a if angle is None else angle))
        _values.remember_all(self, {k: v for k, v in given.items() if v is not None},
                             self._GIVEN)

    #: The attributes matplotlib's ``draw`` reads (its
    #: ``_recompute_transform``; the size it reads is the one the ellipse
    #: was made with), which a class of the program's may compute
    #: (`_push_computed`).
    _computed = ("_center", "angle")

    def _push_computed(self):
        """Hand pltrs (Rust) the geometry a class of the program's computes
        (``_center`` or ``angle`` as properties), as matplotlib's ``draw``
        reads it."""
        if any(takes_from_program(type(self), name) for name in self._computed):
            self._set_fields(center=self._center, angle=self.angle)

    @property
    def _center(self):
        return self.get_center()

    def set_center(self, xy):
        """Set the center of the ellipse."""
        self._set_fields(center=xy)

    def get_center(self):
        """Return the center of the ellipse."""
        return _values.given(self, "center", self._fields()[:2])

    center = property(get_center, set_center)

    def set_width(self, width):
        """Set the width of the ellipse."""
        self._set_fields(width=width)

    def get_width(self):
        """Return the width of the ellipse."""
        return _values.given(self, "width", self._fields()[2])

    width = property(get_width, set_width)

    def set_height(self, height):
        """Set the height of the ellipse."""
        self._set_fields(height=height)

    def get_height(self):
        """Return the height of the ellipse."""
        return _values.given(self, "height", self._fields()[3])

    height = property(get_height, set_height)

    def set_angle(self, angle):
        """Set the angle of the ellipse."""
        self._set_fields(angle=angle)

    def get_angle(self):
        """Return the angle of the ellipse."""
        return _values.given(self, "angle", self._fields()[4])

    angle = property(get_angle, set_angle)

    def get_corners(self):
        """
        Return the corners of the ellipse bounding box.

        The bounding box orientation is moving anti-clockwise from the
        lower left corner defined before rotation.
        """
        return self.get_patch_transform().transform(
            [(-1, -1), (1, -1), (1, 1), (-1, 1)])

    def get_vertices(self):
        """
        Return the vertices coordinates of the ellipse.

        The definition can be found `here <https://en.wikipedia.org/wiki/Ellipse>`_
        """
        if self.width < self.height:
            ret = self.get_patch_transform().transform([(0, 1), (0, -1)])
        else:
            ret = self.get_patch_transform().transform([(1, 0), (-1, 0)])
        return [tuple(x) for x in ret]

    def get_co_vertices(self):
        """
        Return the co-vertices coordinates of the ellipse.

        The definition can be found `here <https://en.wikipedia.org/wiki/Ellipse>`_
        """
        if self.width < self.height:
            ret = self.get_patch_transform().transform([(1, 0), (-1, 0)])
        else:
            ret = self.get_patch_transform().transform([(0, 1), (0, -1)])
        return [tuple(x) for x in ret]


class Annulus(Patch):
    """An elliptical annulus."""

    def __init__(self, xy, r, width, angle=0.0, **kwargs):
        """
        Parameters
        ----------
        xy : (float, float)
            xy coordinates of annulus centre.
        r : float or (float, float)
            The radius, or semi-axes: a scalar for a circular annulus, a
            pair ``(a, b)`` for the semi-major and -minor axes.
        width : float
            Width (thickness) of the annular ring.
        angle : float, default: 0
            Rotation angle in degrees (anti-clockwise from the positive
            x-axis).
        """
        self._native = _patches.new_annulus(
            (float(xy[0]), float(xy[1])), _annulus_radii(r), float(width),
            float(angle))
        _values.remember_all(self, {"center": xy, "width": width, "angle": angle},
                             ("center", "width", "angle"))
        super().__init__(**kwargs)

    def __str__(self):
        a, b = self.radii
        r = (_number_text(a) if a == b
             else f"({_number_text(a)}, {_number_text(b)})")
        return "Annulus(xy=(%s, %s), r=%s, width=%s, angle=%s)" % (
            *map(_number_text, self.center), r, _number_text(self.width),
            _number_text(self.angle))

    def _set(self, center=None, radii=None, width=None, angle=None):
        """Change the given parts of the annulus (see `_native.patches`)."""
        _patches.set_annulus(self._native, center, radii, width, angle)
        self.stale = True

    def set_center(self, xy):
        """
        Set the center of the annulus.

        Parameters
        ----------
        xy : (float, float)
        """
        self._set(center=(float(xy[0]), float(xy[1])))
        _values.remember(self, "center", xy)

    def get_center(self):
        """Return the center of the annulus."""
        return _values.given(self, "center", _patches.annulus(self._native)[0])

    center = property(get_center, set_center)

    def set_width(self, width):
        """
        Set the width (thickness) of the annulus ring.

        The width is measured inwards from the outer ellipse.

        Parameters
        ----------
        width : float
        """
        self._set(width=float(width))
        _values.remember(self, "width", width)

    def get_width(self):
        """Return the width (thickness) of the annulus ring."""
        return _values.given(self, "width", _patches.annulus(self._native)[2])

    width = property(get_width, set_width)

    def set_angle(self, angle):
        """
        Set the tilt angle of the annulus.

        Parameters
        ----------
        angle : float
        """
        self._set(angle=float(angle))
        _values.remember(self, "angle", angle)

    def get_angle(self):
        """Return the angle of the annulus."""
        return _values.given(self, "angle", _patches.annulus(self._native)[3])

    angle = property(get_angle, set_angle)

    def set_semimajor(self, a):
        """
        Set the semi-major axis *a* of the annulus.

        Parameters
        ----------
        a : float
        """
        self._set(radii=(float(a), self.b))

    def set_semiminor(self, b):
        """
        Set the semi-minor axis *b* of the annulus.

        Parameters
        ----------
        b : float
        """
        self._set(radii=(self.a, float(b)))

    def set_radii(self, r):
        """
        Set the semi-major (*a*) and semi-minor radii (*b*) of the annulus.

        Parameters
        ----------
        r : float or (float, float)
            The radius, or semi-axes:

            - If float: radius of the outer circle.
            - If two floats: semi-major and -minor axes of outer ellipse.
        """
        self._set(radii=_annulus_radii(r))

    def get_radii(self):
        """Return the semi-major and semi-minor radii of the annulus."""
        return _patches.annulus(self._native)[1]

    radii = property(get_radii, set_radii)

    a = property(lambda self: self.get_radii()[0], set_semimajor,
                 doc="The semi-major axis of the outer ellipse.")
    b = property(lambda self: self.get_radii()[1], set_semiminor,
                 doc="The semi-minor axis of the outer ellipse.")


def _annulus_radii(r):
    """The outer semi-axes ``(a, b)`` of an annulus of radius or radii *r*."""
    if np.shape(r) == (2,):
        a, b = r
        return float(a), float(b)
    if np.shape(r) == ():
        return float(r), float(r)
    raise ValueError("Parameter 'r' must be one or two floats.")


@_mirrors.mirrorable
class Circle(Ellipse):
    """A circle patch."""

    def __str__(self):
        pars = self.center[0], self.center[1], self.radius
        fmt = "Circle(xy=(%g, %g), radius=%g)"
        return fmt % pars

    def __init__(self, xy, radius=5, **kwargs):
        """
        Create a true circle at center *xy* = (*x*, *y*) with given *radius*.
        """
        self._native = _patches.new_circle((float(xy[0]), float(xy[1])), float(radius))
        # matplotlib's circle is an ellipse of that width and height.
        _values.remember_all(self, {"center": xy, "width": radius * 2,
                                    "height": radius * 2, "angle": 0}, self._GIVEN)
        Patch.__init__(self, **kwargs)

    def _fields(self):
        x, y, r = _patches.circle(self._native)
        return x, y, 2 * r, 2 * r, 0.0

    def _set_fields(self, center=None, width=None, height=None, angle=None):
        x, y, r = _patches.circle(self._native)
        given = {"center": center, "width": width, "height": width}
        center = (x, y) if center is None else center
        radius = r if width is None else width / 2
        _patches.set_circle(self._native, (float(center[0]), float(center[1])),
                            float(radius))
        _values.remember_all(self, {k: v for k, v in given.items() if v is not None},
                             self._GIVEN)

    def set_radius(self, radius):
        """Set the radius of the circle."""
        self._set_fields(width=2 * radius)

    def get_radius(self):
        """Return the radius of the circle."""
        return _patches.circle(self._native)[2]

    radius = property(get_radius, set_radius)


class Arc(Ellipse):
    """
    An elliptical arc, i.e. a segment of an ellipse.

    Due to internal optimizations, the arc cannot be filled.
    """

    def __str__(self):
        pars = (self.center[0], self.center[1], self.width,
                self.height, self.angle, self.theta1, self.theta2)
        fmt = ("Arc(xy=(%g, %g), width=%g, "
               "height=%g, angle=%g, theta1=%g, theta2=%g)")
        return fmt % pars

    def __init__(self, xy, width, height, *,
                 angle=0.0, theta1=0.0, theta2=360.0, **kwargs):
        """
        Parameters
        ----------
        xy : (float, float)
            The center of the ellipse.
        width : float
            The length of the horizontal axis.
        height : float
            The length of the vertical axis.
        angle : float
            Rotation of the ellipse in degrees (counterclockwise).
        theta1, theta2 : float, default: 0, 360
            Starting and ending angles of the arc in degrees.
        """
        fill = kwargs.setdefault('fill', False)
        if fill:
            raise ValueError("Arc objects cannot be filled")
        center, size = self._native_geometry(xy, width, height)
        self._native = _patches.new_arc(center, size, float(angle),
                                        (float(theta1), float(theta2)))
        _values.remember_all(self, {"center": xy, "width": width, "height": height,
                                    "angle": angle, "theta1": theta1, "theta2": theta2},
                             self._GIVEN + ("theta1", "theta2"))
        Patch.__init__(self, **kwargs)

    def _fields(self):
        (x, y, w, h), (angle, _, _) = _patches.arc(self._native)
        return x, y, w, h, angle

    def _set_fields(self, center=None, width=None, height=None, angle=None):
        x, y, w, h, a = self._fields()
        given = {"center": center, "width": width, "height": height, "angle": angle}
        center = (x, y) if center is None else center
        _patches.set_arc(self._native, (float(center[0]), float(center[1])),
                         float(w if width is None else width),
                         float(h if height is None else height),
                         float(a if angle is None else angle))
        _values.remember_all(self, {k: v for k, v in given.items() if v is not None},
                             self._GIVEN)

    @property
    def theta1(self):
        """The start angle of the arc in degrees."""
        return _values.given(self, "theta1", _patches.arc(self._native)[1][1])

    @property
    def theta2(self):
        """The end angle of the arc in degrees."""
        return _values.given(self, "theta2", _patches.arc(self._native)[1][2])

    _computed = Ellipse._computed + ("theta1", "theta2")

    def _push_computed(self):
        # docstring inherited
        super()._push_computed()
        if any(takes_from_program(type(self), name) for name in self._computed):
            _patches.set_arc_angles(self._native, float(self.theta1),
                                    float(self.theta2))


@_mirrors.mirrorable
class FancyBboxPatch(FancyStylesMixin, Patch):
    """
    A fancy box around a rectangle with lower left at *xy* = (*x*, *y*)
    with specified width and height.
    """

    _edge_default = True

    def __str__(self):
        s = self.__class__.__name__ + "((%g, %g), width=%g, height=%g)"
        return s % (self._x, self._y, self._width, self._height)

    def __init__(self, xy, width, height, boxstyle="round", *,
                 mutation_scale=1, mutation_aspect=1, **kwargs):
        """
        Parameters
        ----------
        xy : (float, float)
          The lower left corner of the box.
        width : float
            The width of the box.
        height : float
            The height of the box.
        boxstyle : str or `~matplotlib.patches.BoxStyle`
            The style of the fancy box.
        mutation_scale : float, default: 1
            Scaling factor applied to the attributes of the box style
            (e.g. pad or rounding_size).
        mutation_aspect : float, default: 1
            The height of the rectangle will be squeezed by this value before
            the mutation and the mutated box will be stretched by the inverse
            of it.
        """
        boxstyle = _style_object(boxstyle, BoxStyle)
        self._native = _patches.new_fancy_bbox(
            (float(xy[0]), float(xy[1])), (float(width), float(height)),
            _native_spec(boxstyle, BoxStyle),
            (float(mutation_scale),
             1. if mutation_aspect is None else float(mutation_aspect)))
        _values.remember_all(self, {"x": xy[0], "y": xy[1], "width": width,
                                    "height": height}, ("x", "y", "width", "height"))
        super().__init__(**kwargs)
        self.set_boxstyle(boxstyle)
        self._keep_mutation(mutation_scale, mutation_aspect)

    def set_boxstyle(self, boxstyle=None, **kwargs):
        """
        Set the box style, possibly with further attributes.

        Attributes from the previous box style are not reused.

        Without argument (or with ``boxstyle=None``), the available box styles
        are returned as a human-readable string.

        Parameters
        ----------
        boxstyle : str or `~matplotlib.patches.BoxStyle`
            The style of the box: either a `.BoxStyle` instance, or a string,
            which is the style name and optionally comma separated attributes
            (e.g. "Round,pad=0.2"). Such a string is used to construct a
            `.BoxStyle` object, as documented in that class.

        **kwargs
            Additional attributes for the box style.

        Examples
        --------
        ::

            set_boxstyle("Round,pad=0.2")
            set_boxstyle("round", pad=0.2)
        """
        if boxstyle is None:
            return BoxStyle.pprint_styles()
        self._set_style("_bbox_transmuter", _style_object(boxstyle, BoxStyle, **kwargs),
                        BoxStyle, _patches.set_boxstyle, _patches.get_boxstyle)

    def get_boxstyle(self):
        """Return the boxstyle object."""
        return self._get_style("_bbox_transmuter", BoxStyle, _patches.get_boxstyle)

    @property
    def _x(self):
        return _patches.fancy_bbox(self._native)[0]

    @property
    def _y(self):
        return _patches.fancy_bbox(self._native)[1]

    @property
    def _width(self):
        return _patches.fancy_bbox(self._native)[2]

    @property
    def _height(self):
        return _patches.fancy_bbox(self._native)[3]

    def _set_box(self, x=None, y=None, width=None, height=None):
        x0, y0, w0, h0 = _patches.fancy_bbox(self._native)
        _patches.set_fancy_bbox(
            self._native,
            (float(x0 if x is None else x), float(y0 if y is None else y)),
            float(w0 if width is None else width),
            float(h0 if height is None else height))
        given = {"x": x, "y": y, "width": width, "height": height}
        _values.remember_all(self, {k: v for k, v in given.items() if v is not None},
                             tuple(given))

    def get_x(self):
        """Return the left coord of the rectangle."""
        return _values.given(self, "x", self._x)

    def get_y(self):
        """Return the bottom coord of the rectangle."""
        return _values.given(self, "y", self._y)

    def get_width(self):
        """Return the width of the rectangle."""
        return _values.given(self, "width", self._width)

    def get_height(self):
        """Return the height of the rectangle."""
        return _values.given(self, "height", self._height)

    def set_x(self, x):
        """Set the left coord of the rectangle."""
        self._set_box(x=x)

    def set_y(self, y):
        """Set the bottom coord of the rectangle."""
        self._set_box(y=y)

    def set_width(self, w):
        """Set the rectangle width."""
        self._set_box(width=w)

    def set_height(self, h):
        """Set the rectangle height."""
        self._set_box(height=h)

    def set_bounds(self, *args):
        """Set the bounds of the rectangle: *left*, *bottom*, *width*, *height*."""
        if len(args) == 1:
            l, b, w, h = args[0]
        else:
            l, b, w, h = args
        self._set_box(l, b, w, h)

    def get_bbox(self):
        """Return the `.Bbox` of the rectangle."""
        from .transforms import Bbox
        return Bbox.from_bounds(self._x, self._y, self._width, self._height)


@_mirrors.mirrorable
class FancyArrowPatch(FancyStylesMixin, Patch):
    """
    A fancy arrow patch.

    It draws an arrow using the `ArrowStyle`. It is primarily used by the
    `~.axes.Axes.annotate` method.
    """

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    _edge_default = True
    #: The patches clipping the start and the end (none for an arrow made
    #: by pltrs (Rust), such as the arrow of an annotation).
    patchA = patchB = None

    def __str__(self):
        ends, path = _patches.arrow_route(self._native)
        if ends is not None:
            (x1, y1), (x2, y2) = ends
            return f"{type(self).__name__}(({x1:g}, {y1:g})->({x2:g}, {y2:g}))"
        return f"{type(self).__name__}({Path(*path)})"

    def __init__(self, posA=None, posB=None, *,
                 path=None, arrowstyle="simple", connectionstyle="arc3",
                 patchA=None, patchB=None, shrinkA=2, shrinkB=2,
                 mutation_scale=1, mutation_aspect=1, **kwargs):
        """
        **Defining the arrow position and path**

        There are two ways to define the arrow position and path:

        - **Start, end and connection**: give *posA* and *posB*, and a
          *connectionstyle* draws the path between them.
        - **Arrow path**: give *path*, a `.Path` in display coordinates.

        **Styling**

        The *arrowstyle* defines the styling of the arrow head, tail and
        shaft; *shrinkA* and *shrinkB* shorten the path at its ends, in
        points, and *mutation_scale* scales the style.
        """
        if posA is not None and posB is not None and path is None:
            if all(units._is_natively_supported(v) for v in (*posA, *posB)):
                ends = (tuple(map(float, posA)), tuple(map(float, posB)))
            else:
                # Positions in units convert once the arrow joins an axes.
                self._unit_positions = (posA, posB)
                ends = ((0., 0.), (0., 0.))
        elif posA is None and posB is None and path is not None:
            ends = None
        else:
            raise ValueError("Either posA and posB, or path need to provided")
        arrowstyle = _style_object(arrowstyle, ArrowStyle)
        connectionstyle = _style_object(connectionstyle, ConnectionStyle)
        self._native = _patches.new_fancy_arrow_patch(
            (ends, path),
            (_native_spec(arrowstyle, ArrowStyle),
             _native_spec(connectionstyle, ConnectionStyle),
             float(shrinkA), float(shrinkB), float(mutation_scale),
             1. if mutation_aspect is None else float(mutation_aspect)))
        kwargs.setdefault("joinstyle", JoinStyle.round)
        kwargs.setdefault("capstyle", CapStyle.round)
        super().__init__(**kwargs)
        self.patchA = patchA
        self.patchB = patchB
        self._link_patches()
        self.set_arrowstyle(arrowstyle)
        self._keep_mutation(mutation_scale, mutation_aspect)
        self.set_connectionstyle(connectionstyle)

    def set_connectionstyle(self, connectionstyle=None, **kwargs):
        """
        Set the connection style, possibly with further attributes.

        Attributes from the previous connection style are not reused.

        Without argument (or with ``connectionstyle=None``), the available box
        styles are returned as a human-readable string.

        Parameters
        ----------
        connectionstyle : str or `~matplotlib.patches.ConnectionStyle`
            The style of the connection: either a `.ConnectionStyle` instance,
            or a string, which is the style name and optionally comma separated
            attributes (e.g. "Arc,armA=30,rad=10"). Such a string is used to
            construct a `.ConnectionStyle` object, as documented in that class.

        **kwargs
            Additional attributes for the connection style.

        Examples
        --------
        ::

            set_connectionstyle("Arc,armA=30,rad=10")
            set_connectionstyle("arc", armA=30, rad=10)
        """
        if connectionstyle is None:
            return ConnectionStyle.pprint_styles()
        self._set_style(
            "_connector", _style_object(connectionstyle, ConnectionStyle, **kwargs),
            ConnectionStyle, _patches.set_connectionstyle,
            _patches.get_connectionstyle)

    def get_connectionstyle(self):
        """Return the `ConnectionStyle` used."""
        return self._get_style("_connector", ConnectionStyle,
                               _patches.get_connectionstyle)

    def set_arrowstyle(self, arrowstyle=None, **kwargs):
        """
        Set the arrow style, possibly with further attributes.

        Attributes from the previous arrow style are not reused.

        Without argument (or with ``arrowstyle=None``), the available box
        styles are returned as a human-readable string.

        Parameters
        ----------
        arrowstyle : str or `~matplotlib.patches.ArrowStyle`
            The style of the arrow: either a `.ArrowStyle` instance, or a
            string, which is the style name and optionally comma separated
            attributes (e.g. "Fancy,head_length=0.2"). Such a string is used to
            construct a `.ArrowStyle` object, as documented in that class.

        **kwargs
            Additional attributes for the arrow style.

        Examples
        --------
        ::

            set_arrowstyle("Fancy,head_length=0.2")
            set_arrowstyle("fancy", head_length=0.2)
        """
        if arrowstyle is None:
            return ArrowStyle.pprint_styles()
        self._set_style("_arrow_transmuter",
                        _style_object(arrowstyle, ArrowStyle, **kwargs),
                        ArrowStyle, _patches.set_arrowstyle, _patches.get_arrowstyle)

    def get_arrowstyle(self):
        """Return the arrowstyle object."""
        return self._get_style("_arrow_transmuter", ArrowStyle,
                               _patches.get_arrowstyle)

    def _link_patches(self):
        """
        Clip the ends to `patchA` and `patchB` as pltrs (Rust) draws them,
        which it can once they are in the arrow's figure: the arrow or
        patch added later links them when it is added.
        """
        patches = (self.patchA, self.patchB)
        linked = _patches.set_arrow_patches(
            self._native, *(None if patch is None else patch._native
                            for patch in patches))
        for patch, done in zip(patches, linked):
            if patch is None or done:
                continue
            for artist in (self, patch):
                waiting = artist.__dict__.setdefault("_on_adopt", [])
                if self._link_patches not in waiting:
                    waiting.append(self._link_patches)

    def set_patchA(self, patchA):
        """
        Set the tail patch.

        Parameters
        ----------
        patchA : `.patches.Patch`
        """
        self.patchA = patchA
        self._link_patches()
        self.stale = True

    def set_patchB(self, patchB):
        """
        Set the head patch.

        Parameters
        ----------
        patchB : `.patches.Patch`
        """
        self.patchB = patchB
        self._link_patches()
        self.stale = True

    def get_path(self):
        """Return the path of the arrow in the data coordinates."""
        path = Path(*_patches.display_path(self._native))
        return self.get_transform().inverted().transform_path(path)

    def _add_to_axes(self, axes, kind, autolim=True):
        added = super()._add_to_axes(axes, kind, autolim)
        if kind == "patch":
            # ``add_patch`` measures the arrow, which fails for a connection
            # that cannot be made, as matplotlib's ``_update_patch_limits``.
            self.get_path()
        return added

    def set_positions(self, posA, posB):
        """
        Set the start and end positions of the connecting path.

        Parameters
        ----------
        posA, posB : None, tuple
            (x, y) coordinates of arrow tail and arrow head respectively. If
            `None` use current value.
        """
        _patches.set_arrow_positions(
            self._native, *(None if pos is None else tuple(map(float, pos))
                            for pos in (posA, posB)))

    def _convert_units_for(self, axes):
        positions = self.__dict__.pop("_unit_positions", None)
        if positions is not None:
            self.set_positions(*((axes.convert_xunits(x), axes.convert_yunits(y))
                                 for x, y in positions))


def _connection_end(xy, coords, axes):
    """An end of a `ConnectionPatch` as ``_native.patches`` takes it."""
    from ._live_transforms import to_native
    from .text import _native_coords
    from .transforms import Transform
    if isinstance(coords, Transform):
        native_coords = ("transform", to_native(coords))
    else:
        native_coords = _native_coords(
            coords, None if axes is None else axes.figure)
    return ((float(xy[0]), float(xy[1])), native_coords,
            None if axes is None else axes._native)


def _connection_end_property(index, part, doc):
    """The property of part *part* (0: point, 1: coordinates, 2: axes) of
    end *index* of a `ConnectionPatch`."""
    def get(self):
        return self._ends()[index][part]

    def set(self, value):
        end = list(self._ends()[index])
        end[part] = value
        _patches.set_connection_end(self._native, bool(index),
                                    _connection_end(*end))
        self._given_ends[index] = end[1:]
        self.stale = True

    return property(get, set, doc=doc)


class ConnectionPatch(FancyArrowPatch):
    """A patch that connects two points (possibly in different Axes)."""

    def __str__(self):
        return "ConnectionPatch((%g, %g), (%g, %g))" % \
               (self.xy1[0], self.xy1[1], self.xy2[0], self.xy2[1])

    def __init__(self, xyA, xyB, coordsA, coordsB=None, *,
                 axesA=None, axesB=None,
                 arrowstyle="-",
                 connectionstyle="arc3",
                 patchA=None,
                 patchB=None,
                 shrinkA=0.,
                 shrinkB=0.,
                 mutation_scale=10.,
                 mutation_aspect=None,
                 clip_on=False,
                 **kwargs):
        """
        Connect point *xyA* in *coordsA* with point *xyB* in *coordsB*.

        *coordsA* and *coordsB* are the coordinate systems of annotations
        ('data', 'axes fraction', 'figure pixels', ...), a `.Transform`,
        or an `.Artist` whose extent's fractions they are; *coordsB*
        defaults to *coordsA*. *axesA* and *axesB* are the Axes of data and
        Axes coordinates (by default the Axes the patch is added to). The
        arrow is drawn as a `.FancyArrowPatch` with *arrowstyle*,
        *connectionstyle*, *shrinkA*, *shrinkB*, *mutation_scale* and
        *mutation_aspect*; it is not clipped unless *clip_on* is set.
        """
        if coordsB is None:
            coordsB = coordsA
        self._given_ends = [[coordsA, axesA], [coordsB, axesB]]
        self._native = _patches.new_connection_patch(
            _connection_end(xyA, coordsA, axesA),
            _connection_end(xyB, coordsB, axesB),
            (_native_spec(arrowstyle, ArrowStyle),
             _native_spec(connectionstyle, ConnectionStyle),
             float(shrinkA), float(shrinkB), float(mutation_scale),
             1.0 if mutation_aspect is None else float(mutation_aspect)))
        kwargs.setdefault("joinstyle", JoinStyle.round)
        kwargs.setdefault("capstyle", CapStyle.round)
        Patch.__init__(self, clip_on=clip_on, **kwargs)
        self.patchA = patchA
        self.patchB = patchB
        self._link_patches()

    def _ends(self):
        """The two ends, each as ``(xy, coords, axes)``."""
        return [(xy, *given) for xy, given in
                zip(_patches.connection_points(self._native), self._given_ends)]

    xy1 = _connection_end_property(0, 0, "The point A, in `coords1`.")
    xy2 = _connection_end_property(1, 0, "The point B, in `coords2`.")
    coords1 = _connection_end_property(0, 1, "The coordinates of `xy1`.")
    coords2 = _connection_end_property(1, 1, "The coordinates of `xy2`.")
    axesA = _connection_end_property(
        0, 2, "The Axes of `xy1` in data or axes coordinates.")
    axesB = _connection_end_property(
        1, 2, "The Axes of `xy2` in data or axes coordinates.")

    def set_annotation_clip(self, b):
        """
        Set the annotation's clipping behavior.

        Parameters
        ----------
        b : bool or None
            - True: The annotation will be clipped when ``self.xy`` is
              outside the Axes.
            - False: The annotation will always be drawn.
            - None: The annotation will be clipped when ``self.xy`` is
              outside the Axes and ``self.xycoords == "data"``.
        """
        _patches.set_connection_clip(self._native, None if b is None else bool(b))
        self.stale = True

    def get_annotation_clip(self):
        """
        Return the clipping behavior.

        See `.set_annotation_clip` for the meaning of the return value.
        """
        return _patches.get_connection_clip(self._native)


class Shadow(Patch):
    """
    A shadow of a patch: its outline, offset and painted in a darker shade
    of its face colour, half transparent, just below it.
    """

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    def __str__(self):
        return f"Shadow({self.patch})"

    def __init__(self, patch, ox, oy, *, shade=0.7, **kwargs):
        """
        Create a shadow of the given *patch*.

        By default, the shadow will have the same face color as the *patch*,
        but darkened. The darkness can be controlled by *shade*.

        Parameters
        ----------
        patch : `~matplotlib.patches.Patch`
            The patch to create the shadow for.
        ox, oy : float
            The shift of the shadow in data coordinates, scaled by a factor
            of dpi/72.
        shade : float, default: 0.7
            How the darkness of the shadow relates to the original color. If 1, the
            shadow is black, if 0, the shadow has the same color as the *patch*.
        **kwargs
            Properties of the shadow patch.
        """
        self.patch = patch
        self._ox, self._oy = ox, oy
        self._shadow_transform = Affine2D()
        self._native = _patches.new_shadow(
            patch._native, (float(ox), float(oy)), float(shade))
        Artist.__init__(self)
        # The patch's transform, as matplotlib's ``update_from`` copies it.
        if patch.is_transform_set():
            self._given_transform = Artist.get_transform(patch)
        self._internal_update(kwargs)

    def get_patch_transform(self):
        # The offset in points as the shadow is drawn, which matplotlib
        # sets when it draws the shadow (a shadow drawn is fresh, and stays
        # offset after that).
        figure = self.get_figure(root=True)
        if figure is not None and not self.stale:
            self._drawn = True
        if self.__dict__.get("_drawn"):
            scale = figure.dpi / 72
            self._shadow_transform.clear().translate(self._ox * scale,
                                                     self._oy * scale)
        return self.patch.get_patch_transform() + self._shadow_transform


def bbox_artist(artist, renderer, props=None, fill=True):
    """
    A debug function to draw a rectangle around the bounding
    box returned by an artist's `.Artist.get_window_extent`
    to test whether the artist is returning the correct bbox.

    *props* is a dict of rectangle props with the additional property
    'pad' that sets the padding around the bbox in points.
    """
    if props is None:
        props = {}
    props = props.copy()  # don't want to alter the pad externally
    pad = props.pop('pad', 4)
    pad = renderer.points_to_pixels(pad)
    bbox = artist.get_window_extent(renderer)
    r = Rectangle(
        xy=(bbox.x0 - pad / 2, bbox.y0 - pad / 2),
        width=bbox.width + pad, height=bbox.height + pad,
        fill=fill, transform=IdentityTransform(), clip_on=False)
    r.update(props)
    r.draw(renderer)


def draw_bbox(bbox, renderer, color='k', trans=None):
    """
    A debug function to draw a rectangle around the bounding
    box returned by an artist's `.Artist.get_window_extent`
    to test whether the artist is returning the correct bbox.
    """
    r = Rectangle(xy=bbox.p0, width=bbox.width, height=bbox.height,
                  edgecolor=color, fill=False, clip_on=False)
    if trans is not None:
        r.set_transform(trans)
    r.draw(renderer)
