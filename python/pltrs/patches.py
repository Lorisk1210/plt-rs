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

from . import _api, colors as mcolors, units
from ._enums import CapStyle, JoinStyle
from ._native import patches as _patches
from ._patch_styles import (  # noqa: F401  (matplotlib.patches exports them)
    ArrowStyle, BoxStyle, ConnectionStyle, _native_spec, _Style)
from .artist import Artist
from .path import Path, _DrawnPath
from .transforms import Affine2D


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from . import rcParams
    return rcParams


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
        Return a copy of the vertices used in this patch, in the patch's
        data coordinates (the patch transform applied).
        """
        polygons = self.get_path().to_polygons(self.get_patch_transform())
        if len(polygons):
            return polygons[0]
        return []

    def _set_native(self, **kwargs):
        """Set native properties by their resolved names, in order."""
        _patches.update(self._native, kwargs)

    def get_path(self):
        """Return the path of this patch."""
        return _DrawnPath(*_patches.get_path(self._native), (
            lambda: _patches.get_interpolation_steps(self._native),
            lambda steps: _patches.set_interpolation_steps(self._native, steps)))

    def get_patch_transform(self):
        """
        Return the `~.transforms.Transform` instance mapping patch
        coordinates to data coordinates.
        """
        return Affine2D(_patches.get_patch_transform(self._native))

    def get_transform(self):
        """Return the `~.transforms.Transform` applied to the `Patch`."""
        return self.get_patch_transform() + Artist.get_transform(self)

    def get_data_transform(self):
        """
        Return the `~.transforms.Transform` mapping data coordinates to
        physical coordinates.
        """
        return Artist.get_transform(self)

    def get_extents(self):
        """
        Return the `Patch`'s axis-aligned extents as a `~.transforms.Bbox`.
        """
        return self.get_path().get_extents(self.get_transform())

    def update_from(self, other):
        """Copy properties from *other* to *self*."""
        _patches.update_style_from(self._native, other._native)
        self.set_visible(other.get_visible())
        self.set_alpha(other.get_alpha())
        self.set_clip_on(other.get_clip_on())
        self.set_label(other.get_label())
        if other.is_transform_set():
            self.set_transform(other.get_data_transform())
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
        return self._get("edgecolor")

    def set_edgecolor(self, color):
        """Set the patch edge color (None: the default rule)."""
        if color is not None:
            mcolors.to_rgba(color)
        self._set_native(edgecolor=color)

    def get_facecolor(self):
        """Return the face color."""
        return self._get("facecolor")

    def set_facecolor(self, color):
        """Set the patch face color (None: :rc:`patch.facecolor`)."""
        if color is not None:
            mcolors.to_rgba(color)
        self._set_native(facecolor=color)

    def get_hatchcolor(self):
        """Return the hatch color."""
        return self._get("hatchcolor")

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
        return self._get("linestyle")

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
        return self._get("hatch_linewidth")

    def set_hatch_linewidth(self, lw):
        """Set the hatch linewidth."""
        self._set_native(hatch_linewidth=float(lw))

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

    def _fields(self):
        """``(x, y, width, height, angle, rotation_point)``."""
        return _patches.rectangle(self._native)

    def _set_fields(self, x=None, y=None, width=None, height=None, angle=None,
                    rotation_point=None):
        current = self._fields()
        values = [current[i] if value is None else value for i, value in
                  enumerate((x, y, width, height, angle, rotation_point))]
        _patches.set_rectangle(self._native, (float(values[0]), float(values[1])),
                               (float(values[2]), float(values[3])),
                               float(values[4]), values[5])

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

    xy = property(get_xy, set_xy)


class RegularPolygon(Patch):
    """A regular polygon patch."""

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
        super().__init__(**kwargs)

    @property
    def xy(self):
        """The center of the polygon."""
        return _patches.regular_polygon(self._native)[:2]

    @property
    def numvertices(self):
        """The number of vertices."""
        return _patches.regular_polygon(self._native)[2]

    @property
    def radius(self):
        """The distance from the center to each of the vertices."""
        return _patches.regular_polygon(self._native)[3]

    @property
    def orientation(self):
        """The polygon's rotation, in radians."""
        return _patches.regular_polygon(self._native)[4]


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
        super().__init__(**kwargs)

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


class Wedge(Patch):
    """Wedge shaped patch."""

    def __str__(self):
        pars = (self.center[0], self.center[1], self.r,
                self.theta1, self.theta2, self.width)
        fmt = "Wedge(center=(%g, %g), r=%g, theta1=%g, theta2=%g, width=%s)"
        return fmt % pars

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
        super().__init__(**kwargs)

    def _fields(self):
        """``(x, y, r, theta1, theta2, width)``."""
        return _patches.wedge(self._native)

    def _set_fields(self, center=None, r=None, theta1=None, theta2=None,
                    width=None, *, clear_width=False):
        x, y, r0, t1, t2, w = self._fields()
        center = (x, y) if center is None else center
        _patches.set_wedge(
            self._native, (float(center[0]), float(center[1])),
            float(r0 if r is None else r),
            (float(t1 if theta1 is None else theta1),
             float(t2 if theta2 is None else theta2)),
            None if clear_width else (w if width is None else float(width)))

    @property
    def center(self):
        """The center of the wedge."""
        return self._fields()[:2]

    @property
    def r(self):
        """The radius of the wedge."""
        return self._fields()[2]

    @property
    def theta1(self):
        """The start angle in degrees."""
        return self._fields()[3]

    @property
    def theta2(self):
        """The end angle in degrees."""
        return self._fields()[4]

    @property
    def width(self):
        """The ring width, or None for a full wedge."""
        return self._fields()[5]

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
        self._native = _patches.new_fancy_arrow(
            (float(x), float(y)), (float(dx), float(dy)), float(width),
            (None if head_width is None else float(head_width),
             None if head_length is None else float(head_length),
             shape, float(overhang), bool(length_includes_head),
             bool(head_starts_at_zero)))
        Patch.__init__(self, **kwargs)

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
        Patch.__init__(self, **kwargs)


class Ellipse(Patch):
    """A scale-free ellipse."""

    def __str__(self):
        pars = (self._center[0], self._center[1],
                self.width, self.height, self.angle)
        fmt = "Ellipse(xy=(%s, %s), width=%s, height=%s, angle=%s)"
        return fmt % pars

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
        self._native = _patches.new_ellipse(
            (float(xy[0]), float(xy[1])), (float(width), float(height)),
            float(angle))
        super().__init__(**kwargs)

    def _fields(self):
        """``(x, y, width, height, angle)``."""
        return _patches.ellipse(self._native)

    def _set_fields(self, center=None, width=None, height=None, angle=None):
        x, y, w, h, a = self._fields()
        center = (x, y) if center is None else center
        _patches.set_ellipse(self._native, (float(center[0]), float(center[1])),
                             float(w if width is None else width),
                             float(h if height is None else height),
                             float(a if angle is None else angle))

    @property
    def _center(self):
        return self.get_center()

    def set_center(self, xy):
        """Set the center of the ellipse."""
        self._set_fields(center=xy)

    def get_center(self):
        """Return the center of the ellipse."""
        return self._fields()[:2]

    center = property(get_center, set_center)

    def set_width(self, width):
        """Set the width of the ellipse."""
        self._set_fields(width=width)

    def get_width(self):
        """Return the width of the ellipse."""
        return self._fields()[2]

    width = property(get_width, set_width)

    def set_height(self, height):
        """Set the height of the ellipse."""
        self._set_fields(height=height)

    def get_height(self):
        """Return the height of the ellipse."""
        return self._fields()[3]

    height = property(get_height, set_height)

    def set_angle(self, angle):
        """Set the angle of the ellipse."""
        self._set_fields(angle=angle)

    def get_angle(self):
        """Return the angle of the ellipse."""
        return self._fields()[4]

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
        a, b = (r, r) if np.isscalar(r) else r
        self._native = _patches.new_annulus(
            (float(xy[0]), float(xy[1])), (float(a), float(b)), float(width),
            float(angle))
        super().__init__(**kwargs)


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
        Patch.__init__(self, **kwargs)

    def _fields(self):
        x, y, r = _patches.circle(self._native)
        return x, y, 2 * r, 2 * r, 0.0

    def _set_fields(self, center=None, width=None, height=None, angle=None):
        x, y, r = _patches.circle(self._native)
        center = (x, y) if center is None else center
        radius = r if width is None else width / 2
        _patches.set_circle(self._native, (float(center[0]), float(center[1])),
                            float(radius))

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
        self._native = _patches.new_arc(
            (float(xy[0]), float(xy[1])), (float(width), float(height)),
            float(angle), (float(theta1), float(theta2)))
        Patch.__init__(self, **kwargs)

    def _fields(self):
        (x, y, w, h), (angle, _, _) = _patches.arc(self._native)
        return x, y, w, h, angle

    def _set_fields(self, center=None, width=None, height=None, angle=None):
        x, y, w, h, a = self._fields()
        center = (x, y) if center is None else center
        _patches.set_arc(self._native, (float(center[0]), float(center[1])),
                         float(w if width is None else width),
                         float(h if height is None else height),
                         float(a if angle is None else angle))

    @property
    def theta1(self):
        """The start angle of the arc in degrees."""
        return _patches.arc(self._native)[1][1]

    @property
    def theta2(self):
        """The end angle of the arc in degrees."""
        return _patches.arc(self._native)[1][2]


class FancyBboxPatch(Patch):
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
        self._native = _patches.new_fancy_bbox(
            (float(xy[0]), float(xy[1])), (float(width), float(height)),
            _native_spec(boxstyle, BoxStyle),
            (float(mutation_scale), float(mutation_aspect)))
        super().__init__(**kwargs)

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

    def get_x(self):
        """Return the left coord of the rectangle."""
        return self._x

    def get_y(self):
        """Return the bottom coord of the rectangle."""
        return self._y

    def get_width(self):
        """Return the width of the rectangle."""
        return self._width

    def get_height(self):
        """Return the height of the rectangle."""
        return self._height

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


class FancyArrowPatch(Patch):
    """
    A fancy arrow patch.

    It draws an arrow using the `ArrowStyle`. It is primarily used by the
    `~.axes.Axes.annotate` method.
    """

    _edge_default = True

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
        if patchA is not None or patchB is not None:
            raise NotImplementedError("pltrs: FancyArrowPatch(patchA=..., patchB=...)")
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
        self._native = _patches.new_fancy_arrow_patch(
            (ends, path),
            (_native_spec(arrowstyle, ArrowStyle),
             _native_spec(connectionstyle, ConnectionStyle),
             float(shrinkA), float(shrinkB), float(mutation_scale),
             float(mutation_aspect)))
        kwargs.setdefault("joinstyle", JoinStyle.round)
        kwargs.setdefault("capstyle", CapStyle.round)
        super().__init__(**kwargs)

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


class ConnectionPatch(FancyArrowPatch):
    """A patch that connects two points (possibly in different Axes)."""


class Shadow(Patch):
    """A shadow of a patch."""
