"""
Patches are `.Artist`\\s with a face color and an edge color
(matplotlib's ``patches``).

A patch wraps the native handle of a pltrs (Rust) patch, typed by its
shape: the `Patch` base class reads and writes the properties every
patch has through ``_native.patches.get`` and ``_native.patches.update``,
and each subclass reads and writes its geometry through the native
functions of its shape.
"""

import inspect
import math
from numbers import Number, Real

import numpy as np

from . import _docstring
from . import (_api, _kept_geometry, _mirrors, _owned_paths, _unusable, _values, cbook,
               colors as mcolors, units)
from ._enums import CapStyle, JoinStyle
from ._fancy_styles import FancyStylesMixin, _style_object, native_aspect
from ._native import artist as _artist, patches as _patches
from ._patch_styles import (  # noqa: F401  (matplotlib.patches exports them)
    ArrowStyle, BoxStyle, ConnectionStyle, _native_spec, _Style, calls_program)

cbook._set_module(__name__, ArrowStyle, BoxStyle, ConnectionStyle, _Style)
from ._program_classes import takes_from_program
from .artist import Artist, allow_rasterization
from .lines import _get_dash_pattern
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


def _kept_linestyle(ls):
    """The linestyle *ls* as matplotlib's `Patch.set_linestyle` keeps it
    (``_linestyle``), which `Patch.get_linestyle` returns."""
    if ls is None:
        return "solid"
    if ls in [' ', '', 'none']:
        return 'None'
    return ls


@_docstring.interpd
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

    def set_alpha(self, alpha):
        # docstring inherited: matplotlib's override has none.
        super().set_alpha(alpha)
    zorder = 1

    # Whether to draw an edge by default.  Set on a
    # subclass-by-subclass basis.
    _edge_default = False

    # The attributes matplotlib keeps in the instance that the class serves
    # as properties here: they outlive a change of the instance's class
    # (``art3d.patch_2d_to_3d``), as matplotlib's do.
    _instance_attributes = ()

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
        The following kwarg properties are supported

        %(Patch:kwdoc)s
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
        """
        Return the edge gap color.

        .. versionadded:: 3.11

        See also `~.Patch.set_edgegapcolor`.
        """
        return _values.given(self, "edgegapcolor", self._get("edgegapcolor"),
                             same=_values.same_color)

    def set_edgegapcolor(self, edgegapcolor):
        """
        Set a color to fill the gaps in the dashed edge style.

        .. versionadded:: 3.11

        .. note::

            Striped edges are created by drawing two interleaved dashed lines.
            There can be overlaps between those two, which may result in
            artifacts when using transparency.

            This functionality is experimental and may change.

        Parameters
        ----------
        edgegapcolor : :mpltype:`color` or None
            The color with which to fill the gaps. If None, the gaps are
            unfilled.
        """
        if edgegapcolor is not None:
            edgegapcolor = mcolors.to_rgba(edgegapcolor, self.get_alpha())
        self._set_native(edgegapcolor=edgegapcolor)
        _values.remember(self, "edgegapcolor", edgegapcolor)

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
            `.Patch.get_transform`. See `.Path.contains_point` for further
            details.

            If `None`, the default value depends on the state of the object:

            - If `.Artist.get_picker` is a number, the default
              is that value.  This is so that picking works as expected.
            - Otherwise if the edge color has a non-zero alpha, the default
              is half of the linewidth.  This is so that all the colored
              pixels are "in" the patch.
            - Finally, if the edge has 0 alpha, the default is 0.  This is
              so that patches without a stroked edge do not have points
              outside of the filled region report as "in" due to an
              invisible edge.


        Returns
        -------
        (bool, empty dict)
        """
        if self._different_canvas(mouseevent):
            return False, {}
        radius = float(self._process_radius(radius))
        # matplotlib hit-tests the event's position with the patch's path
        # (also for a patch in no figure, in the coordinates of its
        # transform).
        mouseevent.x, mouseevent.y  # noqa: B018
        return self._contains_within(mouseevent, radius)

    def _set_native(self, **kwargs):
        """Set native properties by their resolved names, in order."""
        _patches.update(self._native, kwargs)

    def get_path(self):
        """Return the path of this patch."""
        raise NotImplementedError('Derived must override')

    def _native_path(self):
        """The path pltrs (Rust) holds for the patch, which the classes
        deriving from `Patch` return (as matplotlib's keep their own)."""
        return drawn_path(self, *_patches.get_path(self._native), (
            lambda: _patches.get_interpolation_steps(self._native),
            lambda steps: _patches.set_interpolation_steps(self._native, steps)))

    def get_patch_transform(self):
        """
        Return the `~.transforms.Transform` instance mapping patch coordinates
        to data coordinates.

        For example, one may define a patch of a circle which represents a
        radius of 5 by providing coordinates for a unit circle, and a
        transform which scales the coordinates (the patch coordinate) by 5.
        """
        return IdentityTransform()

    def _kept_patch_transform(self):
        # The patch transform as the classes keeping one `.Affine2D`
        # (matplotlib's ``_patch_transform``) give it: the same object, with
        # pltrs (Rust)'s matrix.
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
        return self.get_path().get_extents(self.get_transform())

    def get_extents(self):
        """
        Return the `Patch`'s axis-aligned extents as a `~.transforms.Bbox`.
        """
        return self.get_path().get_extents(self.get_transform())

    def _convert_xy_units(self, xy):
        """Convert x and y units for a tuple (x, y)."""
        return self.convert_xunits(xy[0]), self.convert_yunits(xy[1])

    def update_from(self, other):
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
        return _values.given_flag(
            self, "antialiased",
            _rcParams()['patch.antialiased'] if antialiased is None else antialiased)

    def set_antialiased(self, aa):
        """
        Set whether to use antialiased rendering.

        Parameters
        ----------
        aa : bool or None
        """
        self._set_native(antialiased=None if aa is None else bool(aa))
        # matplotlib keeps the object given (``1``, ``'False'``).
        _values.remember(self, "antialiased", aa)

    def get_edgecolor(self):
        """Return the edge color."""
        return self._rgba_as_given("edgecolor", self.get_alpha())

    def set_edgecolor(self, color):
        """
        Set the patch edge color.

        Parameters
        ----------
        color : :mpltype:`color` or None
        """
        if color is not None:
            mcolors.to_rgba(color)
        self._set_native(edgecolor=color)
        _values.remember(self, "edgecolor", color)

    def get_facecolor(self):
        """Return the face color."""
        fill = self.get_fill()
        rgba = self._rgba_as_given("facecolor", self.get_alpha() if fill else 0)
        given = self.__dict__.get("_given_values", {}).get("facecolor")
        if not fill and not cbook._str_lower_equal(given, "none"):
            # matplotlib's ``to_rgba(color, 0)``: the alpha is the int given.
            return (*rgba[:3], 0)
        return rgba

    def set_facecolor(self, color):
        """
        Set the patch face color.

        Parameters
        ----------
        color : :mpltype:`color` or None
        """
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
        # The colour given, or the edge colour when the hatch takes it,
        # typed as matplotlib keeps them (``to_rgba`` of the colour given).
        native = self._get("hatchcolor")
        given = self.__dict__.get("_given_values", {}).get("hatchcolor")
        if given is not None and not cbook._str_equal(given, 'edge'):
            typed = mcolors.to_rgba(given, self.get_alpha())
            if tuple(typed) == tuple(native):
                return typed
        edge = self.get_edgecolor()
        return edge if tuple(edge) == tuple(native) else native

    def set_hatchcolor(self, color):
        """
        Set the patch hatch color.

        Parameters
        ----------
        color : :mpltype:`color` or 'edge' or None
        """
        if color is None:
            color = _rcParams()['hatch.color']
        if not (isinstance(color, str) and color == 'edge'):
            mcolors.to_rgba(color)
        self._set_native(hatchcolor=color)
        _values.remember(self, "hatchcolor", color)

    def set_color(self, c):
        """
        Set both the edgecolor and the facecolor.

        Parameters
        ----------
        c : :mpltype:`color`

        See Also
        --------
        Patch.set_facecolor, Patch.set_edgecolor
            For setting the edge or face color individually.
        """
        self.set_edgecolor(c)
        self.set_hatchcolor(c)
        self.set_facecolor(c)

    def get_linewidth(self):
        """Return the line width in points."""
        return self._get("linewidth")

    def set_linewidth(self, w):
        """
        Set the patch linewidth in points.

        Parameters
        ----------
        w : float or None
        """
        self._set_native(linewidth=None if w is None else float(w))

    def get_linestyle(self):
        """Return the linestyle."""
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
        Set the patch linestyle.

        Parameters
        ----------
        ls : {'-', '--', '-.', ':', '', ...} or (offset, on-off-seq)
            Possible values:

            - A string:

              =======================================================  ================
              linestyle                                                description
              =======================================================  ================
              ``'-'`` or ``'solid'``                                   solid line
              ``'--'`` or ``'dashed'``                                 dashed line
              ``'-.'`` or ``'dashdot'``                                dash-dotted line
              ``':'`` or ``'dotted'``                                  dotted line
              ``''`` or ``'none'`` (discouraged: ``'None'``, ``' '``)  draw nothing
              =======================================================  ================

            - A tuple describing the start position and lengths of dashes and spaces:

                  (offset, onoffseq)

              where

              - *offset* is a float specifying the offset (in points); i.e. how much
                is the dash pattern shifted.
              - *onoffseq* is a sequence of on and off ink in points. There can be
                arbitrary many pairs of on and off values.

              Example: The tuple ``(0, (10, 5, 1, 5))`` means that the pattern starts
              at the beginning of the line. It draws a 10 point long dash,
              then a 5 point long space, then a 1 point long dash, followed by a 5 point
              long space, and then the pattern repeats.

            For examples see :doc:`/gallery/lines_bars_and_markers/linestyles`.
        """
        ls = self._linestyle = _kept_linestyle(ls)
        # The dash pattern matplotlib makes of it, refusing what it cannot.
        _get_dash_pattern(ls)
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
        """
        Set whether to fill the patch.

        Parameters
        ----------
        b : bool
        """
        self._set_native(fill=bool(b))

    fill = property(get_fill, set_fill)

    def get_capstyle(self):
        """Return the capstyle."""
        return self._get("capstyle")

    @_docstring.interpd
    def set_capstyle(self, s):
        """
        Set the `.CapStyle`.

        The default capstyle is 'round' for `.FancyArrowPatch` and 'butt' for
        all other patches.

        Parameters
        ----------
        s : `.CapStyle` or %(CapStyle)s
        """
        self._set_native(capstyle=CapStyle(s).name)

    def get_joinstyle(self):
        """Return the joinstyle."""
        return self._get("joinstyle")

    @_docstring.interpd
    def set_joinstyle(self, s):
        """
        Set the `.JoinStyle`.

        The default joinstyle is 'round' for `.FancyArrowPatch` and 'miter' for
        all other patches.

        Parameters
        ----------
        s : `.JoinStyle` or %(JoinStyle)s
        """
        self._set_native(joinstyle=JoinStyle(s).name)

    def get_hatch(self):
        """Return the hatching pattern."""
        return _values.given(self, "hatch", self._get("hatch"))

    def set_hatch(self, hatch):
        r"""
        Set the hatching pattern.

        *hatch* can be one of::

          /   - diagonal hatching
          \   - back diagonal
          |   - vertical
          -   - horizontal
          +   - crossed
          x   - crossed diagonal
          o   - small circle
          O   - large circle
          .   - dots
          *   - stars

        Letters can be combined, in which case all the specified
        hatchings are done.  If same letter repeats, it increases the
        density of hatching of that pattern.

        In regular (non-raw) Python strings, backslashes must be doubled:
        ``'\\\\'`` and ``r'\\'`` are both a double back-diagonal hatch.

        Parameters
        ----------
        hatch : {'/', '\\', '|', '-', '+', 'x', 'o', 'O', '.', '*'}
        """
        from . import hatch as mhatch
        mhatch._validate_hatch_pattern(hatch)
        self._set_native(hatch=hatch)
        _values.remember(self, "hatch", hatch)

    def get_hatch_linewidth(self):
        """Return the hatch linewidth."""
        return _values.given(self, "hatch_linewidth", self._get("hatch_linewidth"))

    def set_hatch_linewidth(self, lw):
        """Set the hatch linewidth."""
        if not isinstance(lw, Real):
            # Kept unchecked, as matplotlib keeps it until it draws.
            _values.keep(self, "hatch_linewidth", lw)
            return
        native = float(lw)
        self._set_native(hatch_linewidth=native)
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

            If `None`, the default value depends on the state of the object:

            - If `.Artist.get_picker` is a number, the default
              is that value.  This is so that picking works as expected.
            - Otherwise if the edge color has a non-zero alpha, the default
              is half of the linewidth.  This is so that all the colored
              pixels are "in" the patch.
            - Finally, if the edge has 0 alpha, the default is 0.  This is
              so that patches without a stroked edge do not have points
              outside of the filled region report as "in" due to an
              invisible edge.

        Returns
        -------
        bool

        Notes
        -----
        The proper use of this method depends on the transform of the patch.
        Isolated patches do not have a transform. In this case, the patch
        creation coordinates and the point coordinates match. The following
        example checks that the center of a circle is within the circle

        >>> center = 0, 0
        >>> c = Circle(center, radius=1)
        >>> c.contains_point(center)
        True

        The convention of checking against the transformed patch stems from
        the fact that this method is predominantly used to check if display
        coordinates (e.g. from mouse events) are within the patch. If you want
        to do the above check with data coordinates, you have to properly
        transform them first:

        >>> center = 0, 0
        >>> c = Circle(center, radius=3)
        >>> plt.gca().add_patch(c)
        >>> transformed_interior_point = c.get_data_transform().transform((0, 2))
        >>> c.contains_point(transformed_interior_point)
        True

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

            If `None`, the default value depends on the state of the object:

            - If `.Artist.get_picker` is a number, the default
              is that value.  This is so that picking works as expected.
            - Otherwise if the edge color has a non-zero alpha, the default
              is half of the linewidth.  This is so that all the colored
              pixels are "in" the patch.
            - Finally, if the edge has 0 alpha, the default is 0.  This is
              so that patches without a stroked edge do not have points
              outside of the filled region report as "in" due to an
              invisible edge.

        Returns
        -------
        length-N bool array

        Notes
        -----
        The proper use of this method depends on the transform of the patch.
        See the notes on `.Patch.contains_point`.
        """
        radius = self._process_radius(radius)
        return self.get_path().contains_points(points, self.get_transform(),
                                               radius)

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)


@_mirrors.mirrorable
class Rectangle(Patch):
    """
    A rectangle defined via an anchor point *xy* and its *width* and *height*.

    The rectangle extends from ``xy[0]`` to ``xy[0] + width`` in x-direction
    and from ``xy[1]`` to ``xy[1] + height`` in y-direction. ::

      :                +------------------+
      :                |                  |
      :              height               |
      :                |                  |
      :               (xy)---- width -----+

    One may picture *xy* as the bottom left corner, but which corner *xy* is
    actually depends on the direction of the axis and the sign of *width*
    and *height*; e.g. *xy* would be the bottom right corner if the x-axis
    was inverted or if *width* was negative.
    """

    _instance_attributes = ("angle",)

    def __str__(self):
        pars = self.get_x(), self.get_y(), self.get_width(), self.get_height(), self.angle
        fmt = "Rectangle(xy=(%g, %g), width=%g, height=%g, angle=%g)"
        return fmt % pars

    @_docstring.interpd
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
            If ``'xy'``, rotate around the anchor point. If ``'center'`` rotate
            around the center. If 2-tuple of number, rotate around this
            coordinate.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.patches.Patch` properties
            %(Patch:kwdoc)s

        See Also
        --------
        FancyBboxPatch : A rectangle with a fancy box style, e.g. rounded corners.
        """
        x0, y0 = xy[0], xy[1]
        self._native = _patches.new_rectangle((0., 0.), (0., 0.), 0.)
        if any(map(_kept_geometry.in_units, (x0, y0, width, height))):
            # Values in units convert once the rectangle joins an axes; the
            # far corner is summed now, which checks that the units add up,
            # as matplotlib's constructor does.
            self._unit_bounds = (x0, y0, x0 + width, y0 + height)
            self._hold_fields(angle=float(angle))
        else:
            self._hold_fields(x=x0, y=y0, width=width, height=height, angle=float(angle))
        self.rotation_point = rotation_point
        super().__init__(**kwargs)
        if "_unit_bounds" not in self.__dict__:
            self._converted_bounds()  # Validate the inputs, as matplotlib does.
        self._watch_geometry()

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

    def get_path(self):
        """Return the vertices of the rectangle."""
        def steps():
            shared = _patches.get_shared_rectangle_steps()
            return (_patches.get_interpolation_steps(self._native)
                    if shared is None else shared)
        vertices, codes = _patches.get_path(self._native)
        unit = Path.unit_rectangle()
        if (np.array_equal(vertices, unit.vertices)
                and np.array_equal(codes, unit.codes)):
            # The very object, as matplotlib's rectangles share it.
            unit._steps_of = (steps, _patches.set_shared_rectangle_steps)
            return unit
        return drawn_path(self, vertices, codes, (
            steps, _patches.set_shared_rectangle_steps))

    def _fields(self):
        """``(x, y, width, height, angle, rotation_point)``, as given
        (matplotlib keeps them as given)."""
        return _values.given_fields(self, self._FIELDS,
                                    _patches.rectangle(self._native))

    def _set_fields(self, x=_values.NONE_GIVEN, y=_values.NONE_GIVEN,
                    width=_values.NONE_GIVEN, height=_values.NONE_GIVEN,
                    angle=_values.NONE_GIVEN, rotation_point=_values.NONE_GIVEN):
        """Change the fields of the geometry given (by name or in the
        order of `_FIELDS`), kept as given (see `._kept_geometry`)."""
        given = zip(self._FIELDS, (x, y, width, height, angle, rotation_point))
        self._hold_fields(**{name: value for name, value in given
                             if value is not _values.NONE_GIVEN})
        self._watch_geometry()

    def _hold_fields(self, **given):
        """Hand pltrs (Rust) the fields *given* it can hold (see
        `._kept_geometry.hold`)."""
        held = dict(zip(self._FIELDS, _patches.rectangle(self._native)))
        for name, value in given.items():
            if name == "rotation_point":
                # Checked by pltrs (Rust) as matplotlib's setter checks it.
                held[name] = value
                _values.remember(self, name, value)
            else:
                held[name] = _kept_geometry.hold(self, name, value, _kept_geometry.real,
                                                 held[name])
        _patches.set_rectangle(self._native, (held["x"], held["y"]),
                               (held["width"], held["height"]), held["angle"],
                               held["rotation_point"])

    def _watch_geometry(self):
        """Have the rectangle recompute its transform where matplotlib's
        does while its geometry is one it fails or warns on."""
        _kept_geometry.watch(self, self._FIELDS, _patches.rectangle(self._native)[:5],
                             self.get_patch_transform)

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
        """
        Set the rotation angle in degrees.

        The rotation is performed anti-clockwise around *xy*.
        """
        self.angle = angle

    def set_xy(self, xy):
        """
        Set the left and bottom coordinates of the rectangle.

        Parameters
        ----------
        xy : (float, float)
        """
        x, y = xy
        self._set_fields(x=x, y=y)

    def set_width(self, w):
        """Set the width of the rectangle."""
        self._set_fields(width=w)

    def set_height(self, h):
        """Set the height of the rectangle."""
        self._set_fields(height=h)

    def set_bounds(self, *args):
        """
        Set the bounds of the rectangle as *left*, *bottom*, *width*, *height*.

        The values may be passed as separate parameters or as a tuple::

            set_bounds(left, bottom, width, height)
            set_bounds((left, bottom, width, height))

        .. ACCEPTS: (left, bottom, width, height)
        """
        if len(args) == 1:
            l, b, w, h = args[0]
        else:
            l, b, w, h = args
        self._set_fields(l, b, w, h)

    def get_bbox(self):
        """Return the `.Bbox`."""
        from .transforms import Bbox
        return Bbox.from_extents(*self._converted_bounds())

    def _converted_bounds(self):
        """Convert bounds of the rectangle (matplotlib's
        ``Rectangle._convert_units``)."""
        x, y, width, height = self._fields()[:4]
        x0 = self.convert_xunits(x)
        y0 = self.convert_yunits(y)
        x1 = self.convert_xunits(x + width)
        y1 = self.convert_yunits(y + height)
        return x0, y0, x1, y1

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

    _instance_attributes = ("xy", "numvertices", "orientation", "radius")

    def get_patch_transform(self):
        # docstring inherited
        _unusable.check(self, measuring=True)
        return self._kept_patch_transform()

    def _add_to_axes(self, axes, kind, autolim=True):
        if kind == "patch":
            # matplotlib's add_patch reads the patch transform first, for
            # the data limits.
            _unusable.check(self, measuring=True)
        return super()._add_to_axes(axes, kind, autolim)

    def __str__(self):
        s = "RegularPolygon((%g, %g), %d, radius=%g, orientation=%g)"
        return s % (*self.xy, self.numvertices, self.radius, self.orientation)

    @_docstring.interpd
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
            The polygon rotation angle (in radians).

        **kwargs
            `Patch` properties:

            %(Patch:kwdoc)s
        """
        usable = isinstance(orientation, Real)
        self._native = _patches.new_regular_polygon(
            (float(xy[0]), float(xy[1])), int(numVertices), float(radius),
            float(orientation) if usable else 0.)
        _values.remember_all(self, {"xy": xy, "radius": radius,
                                    "orientation": orientation},
                             ("xy", "radius", "orientation"))
        super().__init__(**kwargs)
        if not usable:
            # matplotlib keeps it, and fails to rotate the polygon by it
            # (``Affine2D.rotate``, through ``math.cos``).
            _unusable.keep(self, "orientation", orientation, math.cos)

    def get_path(self):
        # docstring inherited
        return self._native_path()

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
        return _unusable.kept(
            self, "orientation", _values.given(self, "orientation", self._fields()[4]))


class PathPatch(Patch):
    """A general polycurve path patch."""

    _edge_default = True

    def __str__(self):
        s = "PathPatch%d((%g, %g) ...)"
        return s % (len(self.get_path().vertices), *tuple(self.get_path().vertices[0]))

    @_docstring.interpd
    def __init__(self, path, **kwargs):
        """
        *path* is a `.Path` object.

        Valid keyword arguments are:

        %(Patch:kwdoc)s
        """
        self._native = _patches.new_path_patch(path)
        super().__init__(**kwargs)
        self._keep_path(path)

    def get_path(self):
        # docstring inherited
        if "_given_path" in self.__dict__:
            return self.__dict__["_given_path"]
        return self._native_path()

    def set_path(self, path):
        if not isinstance(path, Path):
            # matplotlib keeps it, and fails to draw its vertices.
            self.__dict__["_given_path"] = path
            _unusable.keep(self, "path", path, _path_vertices, measured=False)
            return
        _patches.set_path_shape(self._native, path)
        _unusable.drop(self, "path")
        self._keep_path(path)

    def _keep_path(self, path):
        """Keep *path*, which `get_path` gives, as matplotlib's patch keeps
        and draws the very path it is given: what the program changes in
        it reaches pltrs (Rust) (`._owned_paths`)."""
        self.__dict__["_given_path"] = path
        _owned_paths.hand_out([path], self._push_given_path)

    def _push_given_path(self):
        """Hand pltrs (Rust) the path given again, which the program
        changed."""
        self.set_path(self.__dict__["_given_path"])

    # matplotlib keeps the path as ``_path``, which subclasses read.
    _path = property(lambda self: self.get_path(), set_path)


class StepPatch(PathPatch):
    """
    A path patch describing a stepwise constant function.

    By default, the path is not closed and starts and stops at
    baseline value.
    """

    _instance_attributes = ("orientation",)

    _edge_default = False

    @_docstring.interpd
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
            The direction of the steps. Vertical means that *values* are
            along the y-axis, and edges are along the x-axis.

        baseline : float, array-like or None, default: 0
            The bottom value of the bounding edges or when
            ``fill=True``, position of lower edge. If *fill* is
            True or an array is passed to *baseline*, a closed
            path is drawn.

        **kwargs
            `Patch` properties:

            %(Patch:kwdoc)s
        """
        self._keep_data(values, edges, baseline)
        self._check_data(edges, values, baseline)
        self._native = _patches.new_step_patch(
            np.asarray(values, dtype=float), np.asarray(edges, dtype=float),
            None if baseline is None else np.asarray(baseline, dtype=float),
            orientation != 'vertical')
        self._given_orientation = orientation
        # The native step patch holds its path: no `PathPatch.__init__`.
        Patch.__init__(self, **kwargs)

    @staticmethod
    def _check_data(edges, values, baseline=None):
        """Raise matplotlib's errors for steps of *values* between *edges*
        above *baseline* it cannot draw (``_update_path``): NaN edges, a
        count of edges that is not one more than that of the values, values
        or a baseline that are no numbers or do not broadcast together."""
        edges, values = np.asarray(edges), np.asarray(values)
        if np.isnan(np.sum(edges)):
            raise ValueError('Nan values in "edges" are disallowed')
        if edges.size - 1 != values.size:
            raise ValueError('Size mismatch between "values" and "edges". '
                             "Expected `len(values) + 1 == len(edges)`, but "
                             f"`len(values) = {values.size}` and "
                             f"`len(edges) = {edges.size}`.")
        # matplotlib's mask of the steps it leaves out.
        nan_mask = np.isnan(values)
        if baseline is not None:
            nan_mask |= np.isnan(baseline)

    @property
    def orientation(self):
        """The direction of the steps: 'vertical', or anything else for
        horizontal steps, as it was given."""
        return self.__dict__.get("_given_orientation", (
            'horizontal' if _patches.step_patch_horizontal(self._native) else 'vertical'))

    def get_data(self):
        """Get `.StepPatch` values, edges and baseline as namedtuple."""
        from collections import namedtuple
        StairData = namedtuple('StairData', 'values edges baseline')
        return StairData(*self.__dict__.get("_given_data",
                                            _patches.step_patch(self._native)))

    def _keep_data(self, values, edges, baseline):
        """Keep the *values*, *edges* and *baseline* as matplotlib's patch
        keeps them, arrays of what was given (which `get_data` gives back
        even when it could not draw them)."""
        self._given_data = (np.asarray(values), np.asarray(edges),
                            None if baseline is None else np.asarray(baseline))

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
        # matplotlib keeps them as arrays before it checks them (which
        # leaves the steps drawn as they were).
        old_values, old_edges, old_baseline = self.get_data()
        self._keep_data(old_values if values is None else values,
                        old_edges if edges is None else edges,
                        old_baseline if baseline is None else baseline)
        values, edges, baseline = self._given_data
        self._check_data(edges, values, baseline)
        # matplotlib counts the values and edges by their size, whatever
        # their shape.
        _patches.set_step_patch(
            self._native,
            None if values is None else np.ravel(np.asarray(values, dtype=float)),
            None if edges is None else np.ravel(np.asarray(edges, dtype=float)),
            (baseline is None,
             None if baseline is None else np.asarray(baseline, dtype=float)))

    def set_path(self, path):
        """Set the path of the patch, drawn until the data are set again."""
        if not isinstance(path, Path):
            # matplotlib keeps it, and fails to draw its vertices.
            self.__dict__["_given_path"] = path
            _unusable.keep(self, "path", path, _path_vertices, measured=False)
            return
        self.__dict__.pop("_given_path", None)
        _unusable.drop(self, "path")
        _patches.set_step_path(self._native, path)


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

    @_docstring.interpd
    def __init__(self, xy, *, closed=True, **kwargs):
        """
        Parameters
        ----------
        xy : (N, 2) array

        closed : bool, default: True
            Whether the polygon is closed (i.e., has identical start and end
            points).

        **kwargs
            %(Patch:kwdoc)s
        """
        xy = np.asarray(xy)
        _, _ = xy.shape  # (N, 2): matplotlib's set_xy unpacks it, failing otherwise.
        self._native = _patches.new_polygon(xy.astype(float), bool(closed))
        super().__init__(**kwargs)

    def get_path(self):
        """Get the `.Path` of the polygon."""
        return self._native_path()

    def _outline(self):
        """The vertices and the closedness of the polygon."""
        return _patches.polygon(self._native)

    def _set_outline(self, xy, closed):
        _patches.set_polygon(self._native, xy, closed)

    def get_closed(self):
        """Return whether the polygon is closed."""
        return self._outline()[1]

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
        self._set_outline(self.get_xy(), bool(closed))
        self.stale = True

    def get_xy(self):
        """
        Get the vertices of the path.

        Returns
        -------
        (N, 2) array
            The coordinates of the vertices.
        """
        return self._outline()[0]

    def set_xy(self, xy):
        """
        Set the vertices of the polygon.

        Parameters
        ----------
        xy : (N, 2) array-like
            The coordinates of the vertices.

        Notes
        -----
        Unlike `.Path`, we do not ignore the last input vertex. If the
        polygon is meant to be closed, and the last point of the polygon is not
        equal to the first, we assume that the user has not explicitly passed a
        ``CLOSEPOLY`` vertex, and add it ourselves.
        """
        xy = np.asarray(xy)
        _, _ = xy.shape  # (N, 2): matplotlib unpacks it, failing otherwise.
        self._set_outline(xy.astype(float), self.get_closed())
        self.stale = True

    xy = property(get_xy, set_xy,
                  doc='The vertices of the path as a (N, 2) array.')


@_mirrors.mirrorable
class Wedge(Patch):
    """Wedge shaped patch."""

    _instance_attributes = ("center", "r", "theta1", "theta2", "width")

    def __str__(self):
        pars = (self.center[0], self.center[1], self.r,
                self.theta1, self.theta2, self.width)
        fmt = "Wedge(center=(%g, %g), r=%g, theta1=%g, theta2=%g, width=%s)"
        return fmt % (*pars[:-1], _number_text(pars[-1]))

    @_docstring.interpd
    def __init__(self, center, r, theta1, theta2, *, width=None, **kwargs):
        """
        A wedge centered at *x*, *y* center with radius *r* that
        sweeps *theta1* to *theta2* (in degrees).  If *width* is given,
        then a partial wedge is drawn from inner radius *r* - *width*
        to outer radius *r*.

        Valid keyword arguments are:

        %(Patch:kwdoc)s
        """
        self._native = _patches.new_wedge((0., 0.), 1., (0., 360.), None)
        super().__init__(**kwargs)
        self._hold_fields(center=center, r=r, theta1=theta1, theta2=theta2, width=width)
        if self._watch_geometry():
            # matplotlib's constructor computes the path, kept until it
            # changes.
            self._recompute_path()
            _unusable.drop(self, "geometry")

    #: The geometry matplotlib keeps as given, and how pltrs (Rust) holds
    #: each field (see `._kept_geometry`).
    _FIELDS = {"center": lambda center: _wedge_center(center), "r": _kept_geometry.real,
               "theta1": _kept_geometry.real, "theta2": _kept_geometry.real,
               "width": lambda width: None if width is None else _kept_geometry.real(width)}

    # A mirror (the background of polar Axes) reads its geometry as the
    # last draw fitted it.
    _refreshed_getters = ("_fields", "get_path")

    def get_path(self):
        # docstring inherited
        _unusable.check(self, measuring=True)
        return self._native_path()

    def _fields(self):
        """``(x, y, r, theta1, theta2, width)``."""
        return _patches.wedge(self._native)

    def _set_fields(self, **given):
        """Change the fields of the geometry *given* (``center``, ``r``,
        ``theta1``, ``theta2``, ``width``), kept as given (see
        `._kept_geometry`)."""
        self._hold_fields(**given)
        self._watch_geometry()
        self.stale = True

    def _hold_fields(self, **given):
        """Hand pltrs (Rust) the fields *given* it can hold (see
        `._kept_geometry.hold`)."""
        x, y, r, theta1, theta2, width = self._fields()
        held = {"center": (x, y), "r": r, "theta1": theta1, "theta2": theta2,
                "width": width}
        for name, value in given.items():
            held[name] = _kept_geometry.hold(self, name, value, self._FIELDS[name], held[name])
        _patches.set_wedge(self._native, held["center"], held["r"],
                           (held["theta1"], held["theta2"]), held["width"])

    def _watch_geometry(self):
        """Have the wedge recompute its path where matplotlib's does while
        its geometry is one it fails or warns on (a ring of no radius
        divides by it); whether it does."""
        x, y, r, theta1, theta2, width = self._fields()
        return _kept_geometry.watch(
            self, self._FIELDS, (x, y, r, theta1, theta2, 0.0 if width is None else width),
            self._recompute_path, suspect=r == 0 and self.width is not None, cached=True)

    def _recompute_path(self):
        """The path matplotlib's constructor, draws and measures compute
        from the geometry as given (``_recompute_path``), raising and
        warning as they do."""
        # Inner and outer rings are connected unless the annulus is complete
        if abs((self.theta2 - self.theta1) - 360) <= 1e-12:
            theta1, theta2 = 0, 360
            connector = Path.MOVETO
        else:
            theta1, theta2 = self.theta1, self.theta2
            connector = Path.LINETO
        # Form the outer ring
        arc = Path.arc(theta1, theta2)
        if self.width is not None:
            # Partial annulus needs to draw the outer ring
            # followed by a reversed and scaled inner ring
            v1 = arc.vertices
            v2 = arc.vertices[::-1] * (self.r - self.width) / self.r
            v = np.concatenate([v1, v2, [(0, 0)]])
            c = [*arc.codes, connector, *arc.codes[1:], Path.CLOSEPOLY]
        else:
            # Wedge doesn't need an inner ring
            v = np.concatenate([arc.vertices, [(0, 0), (0, 0)]])
            c = [*arc.codes, connector, Path.CLOSEPOLY]
        # Shift and scale the wedge to the final location.
        return Path(v * self.r + self.center, c)

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
        self._set_fields(center=center)

    def set_radius(self, radius):
        self._set_fields(r=radius)

    def set_theta1(self, theta1):
        self._set_fields(theta1=theta1)

    def set_theta2(self, theta2):
        self._set_fields(theta2=theta2)

    def set_width(self, width):
        self._set_fields(width=width)


def _wedge_center(center):
    """The centre of a wedge, which matplotlib adds to its vertices, as
    pltrs (Rust) holds it: a pair, or one number (alone or in a sequence)
    for both coordinates (see `._kept_geometry`)."""
    match np.shape(center):
        case () | (1,):
            x = _kept_geometry.real(np.ravel(center)[0])
            return x, x
        case (2,):
            return _kept_geometry.point(center)
    raise ValueError(f"{center!r} is no point")


class Arrow(Patch):
    """An arrow patch."""

    def __str__(self):
        return "Arrow()"

    @_docstring.interpd
    def __init__(self, x, y, dx, dy, *, width=1.0, **kwargs):
        """
        Draws an arrow from (*x*, *y*) to (*x* + *dx*, *y* + *dy*).
        The width of the arrow is scaled by *width*.

        Parameters
        ----------
        x : float
            x coordinate of the arrow tail.
        y : float
            y coordinate of the arrow tail.
        dx : float
            Arrow length in the x direction.
        dy : float
            Arrow length in the y direction.
        width : float, default: 1
            Scale factor for the width of the arrow. With a default value of 1,
            the tail width is 0.2 and head width is 0.6.
        **kwargs
            Keyword arguments control the `Patch` properties:

            %(Patch:kwdoc)s

        See Also
        --------
        FancyArrow
            Patch that allows independent control of the head and tail
            properties.
        """
        self._native = _patches.new_arrow((float(x), float(y)), (float(dx), float(dy)),
                                   float(width))
        super().__init__(**kwargs)

    def get_path(self):
        # docstring inherited
        return self._native_path()

    def get_patch_transform(self):
        # docstring inherited
        return self._kept_patch_transform()

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
        if any(np.ndim(value) for value in given if value is not None):
            # matplotlib builds the patch transform with them, putting each
            # into an element of its matrix (``Affine2D.scale``,
            # ``translate``), which numpy refuses for an array.
            raise ValueError("setting an array element with a sequence.")
        x, y, dx, dy, width = (float(current if new is None else new)
                               for new, current in
                               zip(given, _patches.arrow(self._native)))
        _patches.set_arrow(self._native, (x, y), (dx, dy), width)
        self.stale = True


class FancyArrow(Polygon):
    """
    Like Arrow, but lets you set head width and head height independently.
    """

    _instance_attributes = ("verts",)

    _edge_default = True

    def __str__(self):
        return "FancyArrow()"

    @_docstring.interpd
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
            triangular shape). Can be negative or greater than one.

        head_starts_at_zero : bool, default: False
            If True, the head starts being drawn at coordinate 0
            instead of ending at coordinate 0.

        **kwargs
            `.Patch` properties:

            %(Patch:kwdoc)s
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

    def _outline(self):
        return _patches.fancy_arrow_outline(self._native)

    def _set_outline(self, xy, closed):
        _patches.set_fancy_arrow_outline(self._native, xy, closed)

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


class CirclePolygon(RegularPolygon):
    """A polygon-approximation of a circle patch."""

    def __str__(self):
        s = "CirclePolygon((%g, %g), radius=%g, resolution=%d)"
        return s % (*self.xy, self.radius, self.numvertices)

    @_docstring.interpd
    def __init__(self, xy, radius=5, *,
                 resolution=20,  # the number of vertices
                 **kwargs):
        """
        Create a circle at *xy* = (*x*, *y*) with given *radius*.

        This circle is approximated by a regular polygon with *resolution*
        sides.  For a smoother circle drawn with splines, see `Circle`.

        Valid keyword arguments are:

        %(Patch:kwdoc)s
        """
        self._native = _patches.new_circle_polygon(
            (float(xy[0]), float(xy[1])), float(radius), int(resolution))
        _values.remember_all(self, {"xy": xy, "radius": radius, "orientation": 0},
                             ("xy", "radius", "orientation"))
        Patch.__init__(self, **kwargs)

    def _fields(self):
        return _patches.circle_polygon(self._native)


#: How pltrs (Rust) holds each field of an ellipse's geometry (see
#: `._kept_geometry`).
_ELLIPSE_FIELDS = {"center": _kept_geometry.point, "width": _kept_geometry.real,
                   "height": _kept_geometry.real, "angle": _kept_geometry.real}


class Ellipse(Patch):
    """A scale-free ellipse."""

    #: As `Rectangle`'s.
    _aspect_ratio_correction = 1.0

    def get_patch_transform(self):
        # docstring inherited; the `.Affine2D` of the patch coordinates
        # pltrs (Rust) has for the ellipse, made anew, as matplotlib's is
        # recomputed (failing for a geometry it cannot compute with).
        _unusable.check(self, measuring=True)
        return Affine2D(_patches.get_patch_transform(self._native))

    def __str__(self):
        pars = (self._center[0], self._center[1],
                self.width, self.height, self.angle)
        fmt = "Ellipse(xy=(%s, %s), width=%s, height=%s, angle=%s)"
        return fmt % tuple(map(_number_text, pars))

    @_docstring.interpd
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

        Notes
        -----
        Valid keyword arguments are:

        %(Patch:kwdoc)s
        """
        self._native = _patches.new_ellipse((0., 0.), (0., 0.), 0.)
        self._make_geometry(xy, width, height, angle)
        super().__init__(**kwargs)
        self._set_center_as_made(xy)
        self._watch_geometry()

    #: The geometry matplotlib keeps as given.
    _GIVEN = ("center", "width", "height", "angle")

    def get_path(self):
        """Return the path of the ellipse."""
        return self._native_path()

    def _computes(self):
        """Whether the class is one of the program's computing what
        matplotlib's ``_recompute_transform`` and ``draw`` read
        (`_computed`)."""
        return any(takes_from_program(type(self), name) for name in self._computed)

    def _set_center_as_made(self, xy):
        """Set ``_center`` as matplotlib's ``Ellipse.__init__`` does, through
        the setter of a class of the program's computing it."""
        if takes_from_program(type(self), "_center"):
            self._center = xy

    def _recomputed(self):
        """The centre and angle matplotlib's ``_recompute_transform`` reads
        (the centre once for each coordinate), which a class of the
        program's may compute."""
        center = (self.convert_xunits(self._center[0]),
                  self.convert_yunits(self._center[1]))
        return center, self.angle

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited; ``add_patch`` measures the patch through its
        # transform (``_update_patch_limits``), which matplotlib recomputes
        # from the geometry a class of the program's computes, failing on
        # a geometry it cannot compute with (a size no number, as a 1x1
        # `numpy.matrix`).
        if kind == "patch" and self._computes():
            center, angle = self._recomputed()
            self._set_fields(center=center, angle=angle)
        if kind == "patch":
            _unusable.check(self, measuring=True)
        return super()._add_to_axes(axes, kind, autolim)

    def _make_geometry(self, xy, width, height, angle):
        """Give the ellipse made its geometry, as given (see
        `._kept_geometry`); values in units (``basic_units``'
        ``TaggedValue``) convert once the ellipse joins an Axes, as
        matplotlib converts them when it draws."""
        try:
            values = (xy[0], xy[1], width, height)
        except (TypeError, LookupError):
            values = ()
        if any(map(_kept_geometry.in_units, values)):
            self._unit_geometry = values
            _values.remember_all(self, {"center": xy, "width": width, "height": height},
                                 self._GIVEN)
            self._hold_fields(angle=angle)
        else:
            self._hold_fields(center=xy, width=width, height=height, angle=angle)

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

    def _set_native_fields(self, center, width, height, angle):
        """Have pltrs (Rust) hold the geometry, in numbers."""
        _patches.set_ellipse(self._native, center, width, height, angle)

    def _set_fields(self, **given):
        """Change the fields of the geometry *given* (``center``,
        ``width``, ``height``, ``angle``), kept as given (see
        `._kept_geometry`)."""
        self._hold_fields(**given)
        self._watch_geometry()

    def _hold_fields(self, **given):
        """Hand pltrs (Rust) the fields *given* it can hold (see
        `._kept_geometry.hold`)."""
        x, y, w, h, a = self._fields()
        held = {"center": (x, y), "width": w, "height": h, "angle": a}
        for name, value in given.items():
            held[name] = _kept_geometry.hold(self, name, value, _ELLIPSE_FIELDS[name],
                                             held[name])
        self._set_native_fields(**held)

    def _watch_geometry(self):
        """Have the ellipse recompute its transform where matplotlib's
        does while its geometry is one it fails or warns on."""
        _kept_geometry.watch(self, self._GIVEN, self._fields(), self._recompute_transform)

    def _recompute_transform(self):
        """The patch transform matplotlib's draws and measures compute
        from the geometry as given (``_recompute_transform``), raising and
        warning as they do."""
        center = (self.convert_xunits(self._center[0]),
                  self.convert_yunits(self._center[1]))
        width = self.convert_xunits(self.width)
        height = self.convert_yunits(self.height)
        return Affine2D() \
            .scale(width * 0.5, height * 0.5 * self._aspect_ratio_correction) \
            .rotate_deg(self.angle) \
            .scale(1, 1 / self._aspect_ratio_correction) \
            .translate(*center)

    #: The attributes matplotlib's ``draw`` reads (its
    #: ``_recompute_transform``; the size it reads is the one the ellipse
    #: was made with), which a class of the program's may compute
    #: (`_push_computed`).
    _computed = ("_center", "angle")

    def _push_computed(self):
        """Hand pltrs (Rust) the geometry a class of the program's computes
        (``_center`` or ``angle`` as properties), as matplotlib's ``draw``
        reads it (``Patch.draw`` recomputes the transform once)."""
        if self._computes():
            center, angle = self._recomputed()
            self._set_fields(center=center, angle=angle)

    @property
    def _center(self):
        return self.get_center()

    def set_center(self, xy):
        """
        Set the center of the ellipse.

        Parameters
        ----------
        xy : (float, float)
        """
        self._set_fields(center=xy)

    def get_center(self):
        """Return the center of the ellipse."""
        return _values.given(self, "center", self._fields()[:2])

    center = property(get_center, set_center)

    def set_width(self, width):
        """
        Set the width of the ellipse.

        Parameters
        ----------
        width : float
        """
        self._set_fields(width=width)

    def get_width(self):
        """
        Return the width of the ellipse.
        """
        return _values.given(self, "width", self._fields()[2])

    width = property(get_width, set_width)

    def set_height(self, height):
        """
        Set the height of the ellipse.

        Parameters
        ----------
        height : float
        """
        self._set_fields(height=height)

    def get_height(self):
        """Return the height of the ellipse."""
        return _values.given(self, "height", self._fields()[3])

    height = property(get_height, set_height)

    def set_angle(self, angle):
        """
        Set the angle of the ellipse.

        Parameters
        ----------
        angle : float
        """
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

        .. versionadded:: 3.8
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

        .. versionadded:: 3.8
        """
        if self.width < self.height:
            ret = self.get_patch_transform().transform([(1, 0), (-1, 0)])
        else:
            ret = self.get_patch_transform().transform([(0, 1), (0, -1)])
        return [tuple(x) for x in ret]


class Annulus(Patch):
    """
    An elliptical annulus.
    """

    _instance_attributes = ("a", "b")

    @_docstring.interpd
    def __init__(self, xy, r, width, angle=0.0, **kwargs):
        """
        Parameters
        ----------
        xy : (float, float)
            xy coordinates of annulus centre.
        r : float or (float, float)
            The radius, or semi-axes:

            - If float: radius of the outer circle.
            - If two floats: semi-major and -minor axes of outer ellipse.
        width : float
            Width (thickness) of the annular ring. The width is measured inward
            from the outer ellipse so that for the inner ellipse the semi-axes
            are given by ``r - width``. *width* must be less than or equal to
            the semi-minor axis.
        angle : float, default: 0
            Rotation angle in degrees (anti-clockwise from the positive
            x-axis). Ignored for circular annuli (i.e., if *r* is a scalar).
        **kwargs
            Keyword arguments control the `Patch` properties:

            %(Patch:kwdoc)s
        """
        self._native = _patches.new_annulus((0., 0.), (1., 1.), 0., 0.)
        super().__init__(**kwargs)
        # matplotlib's constructor sets them one by one.
        self.set_radii(r)
        self.set_center(xy)
        self.set_width(width)
        self.set_angle(angle)

    def get_path(self):
        # docstring inherited
        _unusable.check(self, measuring=True)
        return self._native_path()

    def __str__(self):
        a, b = self.radii
        r = (_number_text(a) if a == b
             else f"({_number_text(a)}, {_number_text(b)})")
        return "Annulus(xy=(%s, %s), r=%s, width=%s, angle=%s)" % (
            *map(_number_text, self.center), r, _number_text(self.width),
            _number_text(self.angle))

    #: The geometry matplotlib keeps as given, and how pltrs (Rust) holds
    #: each field (see `._kept_geometry`).
    _FIELDS = {"center": _kept_geometry.point, "a": _kept_geometry.real,
               "b": _kept_geometry.real, "width": _kept_geometry.real,
               "angle": _kept_geometry.real}

    def _set(self, **given):
        """Change the fields of the annulus *given*, kept as given (see
        `._kept_geometry`)."""
        center, (a, b), width, angle = _patches.annulus(self._native)
        held = {"center": center, "a": a, "b": b, "width": width, "angle": angle}
        for name, value in given.items():
            held[name] = _kept_geometry.hold(self, name, value, self._FIELDS[name], held[name])
        # pltrs (Rust) is handed the fields given that it holds.
        changed = {name for name in given if not _values.is_kept(self, name)}
        _patches.set_annulus(
            self._native, held["center"] if "center" in changed else None,
            (held["a"], held["b"]) if changed & {"a", "b"} else None,
            held["width"] if "width" in changed else None,
            held["angle"] if "angle" in changed else None)
        _kept_geometry.watch(self, self._FIELDS, (*held["center"], held["a"], held["b"],
                                                  held["width"], held["angle"]),
                             self._recompute_path, cached=True)
        self.stale = True

    def _transform_verts(self, verts, a, b):
        return Affine2D() \
            .scale(*self._convert_xy_units((a, b))) \
            .rotate_deg(self.angle) \
            .translate(*self._convert_xy_units(self.center)) \
            .transform(verts)

    def _recompute_path(self):
        """The path matplotlib's draws and measures compute from the
        geometry as given (``_recompute_path``), raising and warning as
        they do."""
        arc = Path.arc(0, 360)
        a, b, w = self.a, self.b, self.width
        v1 = self._transform_verts(arc.vertices, a, b)
        v2 = self._transform_verts(arc.vertices[::-1], a - w, b - w)
        v = np.vstack([v1, v2, v1[0, :], (0, 0)])
        c = np.hstack([arc.codes, Path.MOVETO,
                       arc.codes[1:], Path.MOVETO,
                       Path.CLOSEPOLY])
        return Path(v, c)

    def set_center(self, xy):
        """
        Set the center of the annulus.

        Parameters
        ----------
        xy : (float, float)
        """
        self._set(center=xy)

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
        if width > min(self.a, self.b):
            raise ValueError(
                'Width of annulus must be less than or equal to semi-minor axis')
        self._set(width=width)

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
        self._set(angle=angle)

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
        self._set(a=float(a))

    def set_semiminor(self, b):
        """
        Set the semi-minor axis *b* of the annulus.

        Parameters
        ----------
        b : float
        """
        self._set(b=float(b))

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
        if np.shape(r) == (2,):
            a, b = r
        elif np.shape(r) == ():
            a = b = float(r)
        else:
            raise ValueError("Parameter 'r' must be one or two floats.")
        self._set(a=a, b=b)

    def get_radii(self):
        """Return the semi-major and semi-minor radii of the annulus."""
        a, b = _patches.annulus(self._native)[1]
        return _values.given(self, "a", a), _values.given(self, "b", b)

    radii = property(get_radii, set_radii)

    a = property(lambda self: self.get_radii()[0], set_semimajor,
                 doc="The semi-major axis of the outer ellipse.")
    b = property(lambda self: self.get_radii()[1], set_semiminor,
                 doc="The semi-minor axis of the outer ellipse.")


@_mirrors.mirrorable
class Circle(Ellipse):
    """
    A circle patch.
    """

    def __str__(self):
        pars = self.center[0], self.center[1], self.radius
        fmt = "Circle(xy=(%g, %g), radius=%g)"
        return fmt % pars

    @_docstring.interpd
    def __init__(self, xy, radius=5, **kwargs):
        """
        Create a true circle at center *xy* = (*x*, *y*) with given *radius*.

        Unlike `CirclePolygon` which is a polygonal approximation, this uses
        Bezier splines and is much closer to a scale-free circle.

        Valid keyword arguments are:

        %(Patch:kwdoc)s
        """
        # matplotlib's circle is an ellipse of that width and height.
        size = radius * 2
        self._native = _patches.new_circle((0., 0.), 0.)
        self._hold_fields(center=xy, width=size, height=size, angle=0)
        Patch.__init__(self, **kwargs)
        self._watch_geometry()

    # The ellipse the circle is: its width, height and angle change apart,
    # as matplotlib's `Ellipse` setters change them.

    def _fields(self):
        return _patches.circle(self._native)

    def _set_native_fields(self, center, width, height, angle):
        _patches.set_circle(self._native, center, width, height, angle)

    def set_radius(self, radius):
        """
        Set the radius of the circle.

        Parameters
        ----------
        radius : float
        """
        self._set_fields(width=2 * radius, height=2 * radius)

    def get_radius(self):
        """Return the radius of the circle."""
        return self.width / 2.

    radius = property(get_radius, set_radius)


class Arc(Ellipse):
    """
    An elliptical arc, i.e. a segment of an ellipse.

    Due to internal optimizations, the arc cannot be filled.
    """

    _instance_attributes = ("theta1", "theta2")

    def __str__(self):
        pars = (self.center[0], self.center[1], self.width,
                self.height, self.angle, self.theta1, self.theta2)
        fmt = ("Arc(xy=(%g, %g), width=%g, "
               "height=%g, angle=%g, theta1=%g, theta2=%g)")
        return fmt % pars

    @_docstring.interpd
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
            Starting and ending angles of the arc in degrees. These values
            are relative to *angle*, e.g. if *angle* = 45 and *theta1* = 90
            the absolute starting angle is 135.
            Default *theta1* = 0, *theta2* = 360, i.e. a complete ellipse.
            The arc is drawn in the counterclockwise direction.
            Angles greater than or equal to 360, or smaller than 0, are
            represented by an equivalent angle in the range [0, 360), by
            taking the input value mod 360.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.patches.Patch` properties
            Most `.Patch` properties are supported as keyword arguments,
            except *fill* and *facecolor* because filling is not supported.

        %(Patch:kwdoc)s
        """
        fill = kwargs.setdefault('fill', False)
        if fill:
            raise ValueError("Arc objects cannot be filled")
        self._native = _patches.new_arc((0., 0.), (0., 0.), 0.,
                                        (float(theta1), float(theta2)))
        _values.remember_all(self, {"theta1": theta1, "theta2": theta2},
                             ("theta1", "theta2"))
        self._make_geometry(xy, width, height, angle)
        Patch.__init__(self, **kwargs)
        self._set_center_as_made(xy)
        self._watch_geometry()
        if self._computes():
            # matplotlib's ``__init__`` sets the angles, then stretches them.
            for name, theta in (("theta1", theta1), ("theta2", theta2)):
                if takes_from_program(type(self), name):
                    setattr(self, name, theta)
            self._given_angles()
        self.get_path()  # The path of the arc as made.

    def get_path(self):
        """
        Return the path of the arc: that of its angles stretched for its
        width and height when it was last drawn, or else made, as
        matplotlib's ``draw`` computes it (pltrs (Rust) draws the arc of
        its geometry of then).
        """
        draws = _artist.draw_count(self._native)
        if self.__dict__.get("_path_draws") != draws:
            self._path_draws = draws
            self._arc_path = super().get_path()
        return self._arc_path

    def _fields(self):
        (x, y, w, h), (angle, _, _) = _patches.arc(self._native)
        return x, y, w, h, angle

    def _set_native_fields(self, center, width, height, angle):
        _patches.set_arc(self._native, center, width, height, angle)

    @property
    def theta1(self):
        """The start angle of the arc in degrees."""
        return _values.given(self, "theta1", _patches.arc(self._native)[1][1])

    @property
    def theta2(self):
        """The end angle of the arc in degrees."""
        return _values.given(self, "theta2", _patches.arc(self._native)[1][2])

    _computed = Ellipse._computed + ("theta1", "theta2", "width", "height")

    def _given_angles(self):
        """
        The angles of the arc as given, read as matplotlib's
        ``_theta_stretch`` reads them (a class of the program's may compute
        them, and the width and height): the width and height, then, when
        they differ, whether the angles make a full turn, which matplotlib
        does not stretch, then the angles. pltrs (Rust) stretches the
        angles for the arc's own width and height.
        """
        width = self.convert_xunits(self.width)
        height = self.convert_yunits(self.height)
        if width != height:
            self._full_turn()
        return self.theta1, self.theta2

    def _full_turn(self):
        """Whether the angles make a full turn."""
        return self.theta1 != self.theta2 and self.theta1 % 360 == self.theta2 % 360

    def _push_computed(self):
        # docstring inherited; matplotlib's ``Arc.draw`` recomputes the
        # transform, stretches the angles, then ``Patch.draw`` recomputes
        # the transform again.
        if self._computes():
            self._recomputed()
            theta1, theta2 = self._given_angles()
            center, angle = self._recomputed()
            self._set_fields(center=center, angle=angle)
            _patches.set_arc_angles(self._native, float(theta1), float(theta2))

    @allow_rasterization
    def draw(self, renderer):
        """
        Draw the arc to the given *renderer*.

        Notes
        -----
        Ellipses are normally drawn using an approximation that uses
        eight cubic Bezier splines.  The error of this approximation
        is 1.89818e-6, according to this unverified source:

          Lancaster, Don.  *Approximating a Circle or an Ellipse Using
          Four Bezier Cubic Splines.*

          https://www.tinaja.com/glib/ellipse4.pdf

        There is a use case where very large ellipses must be drawn
        with very high accuracy, and it is too expensive to render the
        entire ellipse with enough segments (either splines or line
        segments).  Therefore, in the case where either radius of the
        ellipse is large enough that the error of the spline
        approximation will be visible (greater than one pixel offset
        from the ideal), a different technique is used.

        In that case, only the visible parts of the ellipse are drawn,
        with each visible arc using a fixed number of spline segments
        (8).  The algorithm proceeds as follows:

        1. The points where the ellipse intersects the axes (or figure)
           bounding box are located.  (This is done by performing an inverse
           transformation on the bbox such that it is relative to the unit
           circle -- this makes the intersection calculation much easier than
           doing rotated ellipse intersection directly.)

           This uses the "line intersecting a circle" algorithm from:

               Vince, John.  *Geometry for Computer Graphics: Formulae,
               Examples & Proofs.*  London: Springer-Verlag, 2005.

        2. The angles of each of the intersection points are calculated.

        3. Proceeding counterclockwise starting in the positive
           x-direction, each of the visible arc-segments between the
           pairs of vertices are drawn using the Bezier arc
           approximation technique implemented in `.Path.arc`.
        """
        # pltrs (Rust) draws the visible parts of a large arc as matplotlib
        # does.
        super().draw(renderer)


@_mirrors.mirrorable
class FancyBboxPatch(FancyStylesMixin, Patch):
    """
    A fancy box around a rectangle with lower left at *xy* = (*x*, *y*)
    with specified width and height.

    `.FancyBboxPatch` is similar to `.Rectangle`, but it draws a fancy box
    around the rectangle. The transformation of the rectangle box to the
    fancy box is delegated to the style classes defined in `.BoxStyle`.
    """

    _edge_default = True

    def __str__(self):
        s = self.__class__.__name__ + "((%g, %g), width=%g, height=%g)"
        return s % (self._x, self._y, self._width, self._height)

    @_docstring.interpd
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
            The style of the fancy box. This can either be a `.BoxStyle`
            instance or a string of the style name and optionally comma
            separated attributes (e.g. "Round, pad=0.2"). This string is
            passed to `.BoxStyle` to construct a `.BoxStyle` object. See
            there for a full documentation.

            The following box styles are available:

            %(BoxStyle:table)s

        mutation_scale : float, default: 1
            Scaling factor applied to the attributes of the box style
            (e.g. pad or rounding_size).

        mutation_aspect : float, default: 1
            The height of the rectangle will be squeezed by this value before
            the mutation and the mutated box will be stretched by the inverse
            of it. For example, this allows different horizontal and vertical
            padding.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.patches.Patch` properties

        %(Patch:kwdoc)s
        """
        boxstyle = _style_object(boxstyle, BoxStyle)
        self._native = _patches.new_fancy_bbox(
            (float(xy[0]), float(xy[1])), (float(width), float(height)),
            _native_spec(boxstyle, BoxStyle),
            (float(mutation_scale),
             native_aspect(mutation_aspect)))
        _values.remember_all(self, {"x": xy[0], "y": xy[1], "width": width,
                                    "height": height}, ("x", "y", "width", "height"))
        super().__init__(**kwargs)
        self.set_boxstyle(boxstyle)
        self._keep_mutation(mutation_scale, mutation_aspect)

    @_docstring.interpd
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

            The following box styles are available:

            %(BoxStyle:table_and_accepts)s

        **kwargs
            Additional attributes for the box style. See the table above for
            supported parameters.

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

    def get_path(self):
        """Return the mutated path of the rectangle."""
        _unusable.check(self, measuring=True)
        return Path(*_patches.get_path(self._native))

    @staticmethod
    def _use_mutation_aspect(aspect):
        """What matplotlib's box style computes first with the mutation
        *aspect*: it squeezes the height, ``height / aspect``."""
        return 1. / aspect

    def _use_mutation_scale(self, scale):
        """What matplotlib's box style computes first with the mutation
        *scale*: its padding, ``mutation_size * pad``."""
        return scale * getattr(self.get_boxstyle(), "pad", 1.)

    def get_boxstyle(self):
        """Return the boxstyle object."""
        return self._get_style("_bbox_transmuter", BoxStyle, _patches.get_boxstyle)

    def get_mutation_scale(self):
        """Return the mutation scale."""
        return self._kept_mutation_scale()

    def _use_mutation_scale(self, scale):
        """What matplotlib's box style computes first with the mutation
        *scale*: the pad, ``mutation_size * self.pad``."""
        return scale * getattr(self.get_boxstyle(), "pad", 1.)

    def get_mutation_aspect(self):
        """Return the aspect ratio of the bbox mutation."""
        return self._kept_mutation_aspect()

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
        """
        Set the left coord of the rectangle.

        Parameters
        ----------
        x : float
        """
        self._set_box(x=x)

    def set_y(self, y):
        """
        Set the bottom coord of the rectangle.

        Parameters
        ----------
        y : float
        """
        self._set_box(y=y)

    def set_width(self, w):
        """
        Set the rectangle width.

        Parameters
        ----------
        w : float
        """
        self._set_box(width=w)

    def set_height(self, h):
        """
        Set the rectangle height.

        Parameters
        ----------
        h : float
        """
        self._set_box(height=h)

    def set_bounds(self, *args):
        """
        Set the bounds of the rectangle.

        Call signatures::

            set_bounds(left, bottom, width, height)
            set_bounds((left, bottom, width, height))

        Parameters
        ----------
        left, bottom : float
            The coordinates of the bottom left corner of the rectangle.
        width, height : float
            The width/height of the rectangle.
        """
        if len(args) == 1:
            l, b, w, h = args[0]
        else:
            l, b, w, h = args
        self._set_box(l, b, w, h)

    def get_bbox(self):
        """Return the `.Bbox`."""
        from .transforms import Bbox
        return Bbox.from_bounds(self._x, self._y, self._width, self._height)


@_mirrors.mirrorable
class FancyArrowPatch(FancyStylesMixin, Patch):
    """
    A fancy arrow patch.

    It draws an arrow using the `ArrowStyle`. It is primarily used by the
    `~.axes.Axes.annotate` method.  For most purposes, use the annotate method for
    drawing arrows.

    The head and tail positions are fixed at the specified start and end points
    of the arrow, but the size and shape (in display coordinates) of the arrow
    does not change when the axis is moved or zoomed.
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

    def _shrink(index, name):
        """The property *name*: how far the arrow is shrunk at its start
        (*index* 0) or end, in points, as the program gave it."""
        def get(self):
            return _values.given(self, name, _patches.get_shrink(self._native)[index])

        def set(self, points):
            _patches.set_shrink(self._native, *((float(points), None) if index == 0
                                                else (None, float(points))))
            _values.remember(self, name, points)
            self.stale = True
        side = "start" if index == 0 else "end"
        return property(get, set, doc=f"How far the arrow's {side} is shrunk, in points.")

    shrinkA = _shrink(0, "shrinkA")
    shrinkB = _shrink(1, "shrinkB")
    del _shrink

    @_docstring.interpd
    def __init__(self, posA=None, posB=None, *,
                 path=None, arrowstyle="simple", connectionstyle="arc3",
                 patchA=None, patchB=None, shrinkA=2, shrinkB=2,
                 mutation_scale=1, mutation_aspect=1, **kwargs):
        """
        **Defining the arrow position and path**

        There are two ways to define the arrow position and path:

        - **Start, end and connection**:
          The typical approach is to define the start and end points of the
          arrow using *posA* and *posB*. The curve between these two can
          further be configured using *connectionstyle*.

          If given, the arrow curve is clipped by *patchA* and *patchB*,
          allowing it to start/end at the border of these patches.
          Additionally, the arrow curve can be shortened by *shrinkA* and *shrinkB*
          to create a margin between start/end (after possible clipping) and the
          drawn arrow.

        - **path**: Alternatively if *path* is provided, an arrow is drawn along
          this Path. In this case, *connectionstyle*, *patchA*, *patchB*,
          *shrinkA*, and *shrinkB* are ignored.

        **Styling**

        The *arrowstyle* defines the styling of the arrow head, tail and shaft.
        The resulting arrows can be styled further by setting the `.Patch`
        properties such as *linewidth*, *color*, *facecolor*, *edgecolor*
        etc. via keyword arguments.

        Parameters
        ----------
        posA, posB : (float, float), optional
            (x, y) coordinates of start and end point of the arrow.
            The actually drawn start and end positions may be modified
            through *patchA*, *patchB*, *shrinkA*, and *shrinkB*.

            *posA*, *posB* are exclusive of *path*.

        path : `~matplotlib.path.Path`, optional
            If provided, an arrow is drawn along this path and *patchA*,
            *patchB*, *shrinkA*, and *shrinkB* are ignored.

            *path* is exclusive of *posA*, *posB*.

        arrowstyle : str or `.ArrowStyle`, default: 'simple'
            The styling of arrow head, tail and shaft. This can be

            - `.ArrowStyle` or one of its subclasses
            - The shorthand string name (e.g. "->") as given in the table below,
              optionally containing a comma-separated list of style parameters,
              e.g. "->, head_length=10, head_width=5".

            The style parameters are scaled by *mutation_scale*.

            The following arrow styles are available. See also
            :doc:`/gallery/text_labels_and_annotations/fancyarrow_demo`.

            %(ArrowStyle:table)s

            Only the styles ``<|-``, ``-|>``, ``<|-|>`` ``simple``, ``fancy``
            and ``wedge`` contain closed paths and can be filled.

        connectionstyle : str or `.ConnectionStyle` or None, optional, \
default: 'arc3'
            `.ConnectionStyle` with which *posA* and *posB* are connected.
            This can be

            - `.ConnectionStyle` or one of its subclasses
            - The shorthand string name as given in the table below, e.g. "arc3".

            %(ConnectionStyle:table)s

            Ignored if *path* is provided.

        patchA, patchB : `~matplotlib.patches.Patch`, default: None
            Optional Patches at *posA* and *posB*, respectively. If given,
            the arrow path is clipped by these patches such that head and tail
            are at the border of the patches.

            Ignored if *path* is provided.

        shrinkA, shrinkB : float, default: 2
            Shorten the arrow path at *posA* and *posB* by this amount in points.
            This allows to add a margin between the intended start/end points and
            the arrow.

            Ignored if *path* is provided.

        mutation_scale : float, default: 1
            Value with which attributes of *arrowstyle* (e.g., *head_length*)
            will be scaled.

        mutation_aspect : None or float, default: None
            The height of the rectangle will be squeezed by this value before
            the mutation and the mutated box will be stretched by the inverse
            of it.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.patches.Patch` properties, optional
            Here is a list of available `.Patch` properties:

        %(Patch:kwdoc)s

            In contrast to other patches, the default ``capstyle`` and
            ``joinstyle`` for `FancyArrowPatch` are set to ``"round"``.
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
        # A style that cannot be called the arrow keeps (`set_arrowstyle`),
        # drawn with the default meanwhile.
        self._native = _patches.new_fancy_arrow_patch(
            (ends, path),
            (_native_spec(arrowstyle if callable(arrowstyle) else "simple", ArrowStyle),
             _native_spec(connectionstyle if callable(connectionstyle) else "arc3",
                          ConnectionStyle),
             float(shrinkA), float(shrinkB), float(mutation_scale),
             native_aspect(mutation_aspect)))
        kwargs.setdefault("joinstyle", JoinStyle.round)
        kwargs.setdefault("capstyle", CapStyle.round)
        super().__init__(**kwargs)
        _values.remember(self, "shrinkA", shrinkA)
        _values.remember(self, "shrinkB", shrinkB)
        self.patchA = patchA
        self.patchB = patchB
        self._link_patches()
        self.set_arrowstyle(arrowstyle)
        self._keep_mutation(mutation_scale, mutation_aspect)
        self.set_connectionstyle(connectionstyle)

    @_docstring.interpd
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

            The following connection styles are available:

            %(ConnectionStyle:table_and_accepts)s

        **kwargs
            Additional attributes for the connection style. See the table above
            for supported parameters.

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

    @_docstring.interpd
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

            The following arrow styles are available:

            %(ArrowStyle:table_and_accepts)s

        **kwargs
            Additional attributes for the arrow style. See the table above for
            supported parameters.

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

    def get_mutation_scale(self):
        """
        Return the mutation scale.

        Returns
        -------
        scalar
        """
        return self._kept_mutation_scale()

    def _use_mutation_scale(self, scale):
        """What matplotlib's arrow computes first with the mutation
        *scale*: its size in pixels, ``scale * dpi_cor``."""
        return scale * self._dpi_cor

    def get_mutation_aspect(self):
        """Return the aspect ratio of the bbox mutation."""
        return self._kept_mutation_aspect()

    def _link_patches(self):
        """
        Clip the ends to `patchA` and `patchB` as pltrs (Rust) draws them,
        which it can once they are in the arrow's figure: the arrow or
        patch added later links them when it is added. An end clipped by
        what is no patch the arrow keeps, as matplotlib's does, and fails
        to draw where matplotlib's asks it whether it contains the end
        (any true value); a false one clips nothing.
        """
        patches = tuple(patch if isinstance(patch, Patch) else None
                        for patch in (self.patchA, self.patchB))
        for name, given in (("patchA", self.patchA), ("patchB", self.patchB)):
            if given is not None and not isinstance(given, Patch) and given:
                _unusable.keep(self, name, given, _contains_end)
            else:
                _unusable.drop(self, name)
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

    @staticmethod
    def _use_mutation_aspect(aspect):
        """What matplotlib's arrow style computes first with the mutation
        *aspect*: it squeezes the path, ``vertices / [1, aspect]``."""
        return np.zeros((1, 2)) / [1, aspect]

    @staticmethod
    def _use_mutation_scale(scale):
        """What matplotlib's arrow computes first with the mutation *scale*:
        the scale in pixels, ``mutation_scale * dpi_cor``."""
        return scale * 1.

    def _place_ends(self):
        """Fail as matplotlib's patch fails to place its ends where it
        measures them: nothing to place for an arrow of given points."""

    def get_path(self):
        """Return the path of the arrow in the data coordinates."""
        _unusable.check(self, measuring=True)
        self._place_ends()
        # The arrow of an annotation is drawn by the annotation, which
        # places it (`._annotation_arrow`).
        arrays = _mirrors.ask(self, "display_path")
        if arrays is _mirrors.OWN:
            arrays = _patches.display_path(self._native)
        return self.get_transform().inverted().transform_path(Path(*arrays))

    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    def _add_to_axes(self, axes, kind, autolim=True):
        if (calls_program(self.get_connectionstyle())
                and "_transform_trees" not in axes.__dict__):
            # A connection style of the program's may read the Axes'
            # transforms (networkx's self loops read ``transData``) where
            # pltrs (Rust) calls it, measuring or drawing the arrow, when
            # they can no longer be built: they are built now.
            axes._build_transform_trees()
        added = super()._add_to_axes(axes, kind, autolim)
        if kind == "patch":
            # ``add_patch`` measures the arrow, which fails for a connection
            # that cannot be made, as matplotlib's ``_update_patch_limits``
            # (with the view as it is, not autoscaled first).
            _patches.check_measurable(self._native)
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
        positions = self._posA_posB
        for index, pos in enumerate((posA, posB)):
            if pos is not None:
                positions[index] = pos
        try:
            ends = [tuple(map(float, pos)) for pos in positions]
        except (TypeError, ValueError):
            ends = None
        if ends is None or any(len(end) != 2 for end in ends):
            # matplotlib keeps them, and fails where it converts them and
            # maps them to the display.
            _unusable.keep(self, "positions", positions, self._place_ends_as_drawn)
            return
        _unusable.drop(self, "positions")
        _patches.set_arrow_positions(self._native, *ends)
        # Read back as given, as matplotlib keeps them.
        self._given_positions = positions

    def _kept_order(self):
        """The order matplotlib's ``_get_path_in_displaycoord`` computes with
        the values the arrow keeps: the ends, the connection (clipped by the
        patches), the mutation scale, then the arrow style (squeezed by the
        mutation aspect)."""
        return ["positions", "_connector", "patchA", "patchB", "mutation_scale",
                "_arrow_transmuter", "mutation_aspect"]

    def _place_ends_as_drawn(self, positions):
        """Fail as matplotlib's arrow fails to place its ends *positions*:
        converted with the units of its Axes, then mapped by its
        transform."""
        ends = [(self.convert_xunits(pos[0]), self.convert_yunits(pos[1]))
                for pos in positions]
        self.get_transform().transform(ends)

    def _convert_units_for(self, axes):
        positions = self.__dict__.pop("_unit_positions", None)
        if positions is not None:
            self.set_positions(*((axes.convert_xunits(x), axes.convert_yunits(y))
                                 for x, y in positions))

    @property
    def _posA_posB(self):
        """The start and end of the arrow, ``[posA, posB]`` (None for an
        arrow along a path), as matplotlib keeps them: those in units until
        the arrow's Axes converts them."""
        kept = _unusable.kept(self, "positions", None)
        if kept is not None:
            return list(kept)
        positions = self.__dict__.get("_unit_positions")
        if positions is not None:
            return list(positions)
        ends, _ = _patches.arrow_route(self._native)
        if ends is None:
            return None
        given = self.__dict__.get("_given_positions")
        if given is not None and [tuple(map(float, pos)) for pos in given] == list(ends):
            return list(given)
        return [tuple(end) for end in ends]

    @property
    def _dpi_cor(self):
        """The pixels of a point the arrow was last drawn with (1 until it is
        drawn), which scales its shrinking and mutation: matplotlib's draw
        sets it to ``renderer.points_to_pixels(1)``."""
        figure = self.get_figure(root=True)
        if figure is None or not _artist.draw_count(self._native):
            return 1.0
        return figure.dpi / 72


#: The names of the coordinate systems of a `ConnectionPatch` (those of
#: matplotlib's ``ConnectionPatch._get_xy``).
_CONNECTION_COORDS = frozenset([
    "figure points", "axes points", "figure fraction", "subfigure fraction",
    "axes fraction", "data", "offset points", "polar", "figure pixels",
    "subfigure pixels", "axes pixels"])


#: What matplotlib's ``ConnectionPatch._get_xy`` reads first to place a
#: point in each coordinate system: an attribute of the patch's figure or
#: of the Axes of the point.
_CONNECTION_NEEDS = {
    "figure points": ("figure", "dpi"), "axes points": ("figure", "dpi"),
    "figure fraction": ("figure", "transFigure"),
    "subfigure fraction": ("figure", "transSubfigure"),
    "axes fraction": ("axes", "transAxes"), "data": ("axes", "transData"),
    "polar": ("axes", "transData"), "figure pixels": ("figure", "figbbox"),
    "subfigure pixels": ("figure", "bbox"), "axes pixels": ("axes", "bbox")}


def _place_ends_decoupled(patch):
    """Fail as matplotlib's `ConnectionPatch` fails to place its ends once
    it has no figure and no Axes (removed), where their coordinates need
    them: on the attribute of None it reads first."""
    for _, coords, axes in patch._ends():
        need = _CONNECTION_NEEDS.get(coords) if isinstance(coords, str) else None
        if need is not None:
            owner = (patch.get_figure(root=False) if need[0] == "figure"
                     else axes if axes is not None else patch.axes)
            getattr(owner, need[1])


def _path_vertices(path):
    """Fail as matplotlib's patch fails to draw *path*, which is no
    `.Path`: it reads its vertices."""
    path.vertices


def _contains_end(patch):
    """Fail as matplotlib's connection styles fail to clip an arrow's end
    by *patch*, what is no patch: they ask it whether it contains the
    end."""
    patch.contains(SimpleNamespace(x=0., y=0.))


def _refuse_connection_coords(coords):
    """Fail as matplotlib's `ConnectionPatch` fails to place a point in
    *coords*, no coordinate system it knows."""
    raise ValueError(f"{coords} is not a valid coordinate transformation")


def _usable_coords(coords):
    """*coords*, or ``'data'`` for a name of no coordinate system, which
    pltrs (Rust) cannot hold (matplotlib keeps it, and fails to place the
    point with it, see `._unusable`)."""
    if isinstance(coords, str) and coords not in _CONNECTION_COORDS:
        return "data"
    return coords


def _connection_end(xy, coords, axes):
    """An end of a `ConnectionPatch` as ``_native.patches`` takes it."""
    coords = _usable_coords(coords)
    from ._live_transforms import to_native
    from .text import _native_coords
    from .transforms import Transform
    if isinstance(coords, Transform):
        native_coords = ("transform", to_native(coords))
    else:
        native_coords = _native_coords(
            coords, None if axes is None else axes.figure)
    return (_connection_point(xy, coords, axes), native_coords,
            None if axes is None else axes._native)


def _connection_point(xy, coords, axes):
    """The point *xy* of a `ConnectionPatch` end in *coords*, in
    ``'data'`` coordinates converted with the units of *axes* (dates,
    categories), as matplotlib's ``_get_xy`` converts it."""
    x, y = xy[0], xy[1]
    if axes is not None and isinstance(coords, str) and coords == "data":
        x, y = axes.convert_xunits(x), axes.convert_yunits(y)
    return float(x), float(y)


def _connection_end_property(index, part, doc):
    """The property of part *part* (0: point, 1: coordinates, 2: axes) of
    end *index* of a `ConnectionPatch`."""
    def get(self):
        return self._ends()[index][part]

    def set(self, value):
        ends = [list(end) for end in self._ends()]
        ends[index][part] = value
        _patches.set_connection_end(self._native, bool(index),
                                    _connection_end(*ends[index]))
        self._given_points = [end[0] for end in ends]
        self._given_ends[index] = ends[index][1:]
        self._keep_unusable_coords()
        self.stale = True

    return property(get, set, doc=doc)


class ConnectionPatch(FancyArrowPatch):
    """A patch that connects two points (possibly in different Axes)."""

    _instance_attributes = ("xy1", "xy2", "coords1", "coords2")

    def __str__(self):
        return "ConnectionPatch((%g, %g), (%g, %g))" % \
               (self.xy1[0], self.xy1[1], self.xy2[0], self.xy2[1])

    @_docstring.interpd
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

        Valid keys are

        ===============  ======================================================
        Key              Description
        ===============  ======================================================
        arrowstyle       the arrow style
        connectionstyle  the connection style
        relpos           default is (0.5, 0.5)
        patchA           default is bounding box of the text
        patchB           default is None
        shrinkA          default is 2 points
        shrinkB          default is 2 points
        mutation_scale   default is text size (in points)
        mutation_aspect  default is 1.
        ?                any key for `matplotlib.patches.PathPatch`
        ===============  ======================================================

        *coordsA* and *coordsB* are strings that indicate the
        coordinates of *xyA* and *xyB*.

        ==================== ==================================================
        Property             Description
        ==================== ==================================================
        'figure points'      points from the lower left corner of the figure
        'figure pixels'      pixels from the lower left corner of the figure
        'figure fraction'    0, 0 is lower left of figure and 1, 1 is upper
                             right
        'subfigure points'   points from the lower left corner of the subfigure
        'subfigure pixels'   pixels from the lower left corner of the subfigure
        'subfigure fraction' fraction of the subfigure, 0, 0 is lower left.
        'axes points'        points from lower left corner of the Axes
        'axes pixels'        pixels from lower left corner of the Axes
        'axes fraction'      0, 0 is lower left of Axes and 1, 1 is upper right
        'data'               use the coordinate system of the object being
                             annotated (default)
        'offset points'      offset (in points) from the *xy* value
        'polar'              you can specify *theta*, *r* for the annotation,
                             even in cartesian plots.  Note that if you are
                             using a polar Axes, you do not need to specify
                             polar for the coordinate system since that is the
                             native "data" coordinate system.
        ==================== ==================================================

        Alternatively they can be set to any valid
        `~matplotlib.transforms.Transform`.

        Note that 'subfigure pixels' and 'figure pixels' are the same
        for the parent figure, so users who want code that is usable in
        a subfigure can use 'subfigure pixels'.

        .. note::

           Using `ConnectionPatch` across two `~.axes.Axes` instances
           is not directly compatible with :ref:`constrained layout
           <constrainedlayout_guide>`. Add the artist
           directly to the `.Figure` instead of adding it to a specific Axes,
           or exclude it from the layout using ``con.set_in_layout(False)``.

           .. code-block:: default

              fig, ax = plt.subplots(1, 2, constrained_layout=True)
              con = ConnectionPatch(..., axesA=ax[0], axesB=ax[1])
              fig.add_artist(con)

        """
        if coordsB is None:
            coordsB = coordsA
        self._given_points = [xyA, xyB]
        self._given_ends = [[coordsA, axesA], [coordsB, axesB]]
        self._native = _patches.new_connection_patch(
            _connection_end(xyA, coordsA, axesA),
            _connection_end(xyB, coordsB, axesB),
            (_native_spec(arrowstyle, ArrowStyle),
             _native_spec(connectionstyle, ConnectionStyle),
             float(shrinkA), float(shrinkB), float(mutation_scale),
             native_aspect(mutation_aspect)))
        kwargs.setdefault("joinstyle", JoinStyle.round)
        kwargs.setdefault("capstyle", CapStyle.round)
        Patch.__init__(self, clip_on=clip_on, **kwargs)
        _values.remember(self, "shrinkA", shrinkA)
        _values.remember(self, "shrinkB", shrinkB)
        self._keep_mutation(mutation_scale, mutation_aspect)
        self.patchA = patchA
        self.patchB = patchB
        self._link_patches()
        self._keep_unusable_coords()

    def _place_ends(self):
        # docstring inherited
        if self.__dict__.get("_decoupled"):
            # Removed from its figure and Axes, the patch is measured as
            # matplotlib's places its ends without them.
            _place_ends_decoupled(self)

    def _keep_unusable_coords(self):
        """Keep the coordinates of each end that pltrs (Rust) cannot hold,
        as matplotlib keeps them until it places the points (see
        `._unusable`)."""
        for index, (coords, _) in enumerate(self._given_ends):
            name = f"coords{index + 1}"
            if _usable_coords(coords) is coords:
                _unusable.drop(self, name)
            else:
                _unusable.keep(self, name, coords, _refuse_connection_coords)

    def _ends(self):
        """The two ends, each as ``(xy, coords, axes)``, with the points as
        given (dates, categories), as matplotlib keeps them, or as pltrs
        (Rust) placed them (an inset indicator's connectors)."""
        points = self.__dict__.get("_given_points")
        if points is None:
            points = _patches.connection_points(self._native)
        return [(xy, *given) for xy, given in zip(points, self._given_ends)]

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

    def draw(self, renderer):
        # (pltrs (Rust) leaves out a connection whose ends the annotation
        # clip hides, as matplotlib's ``_check_xy``.)
        super().draw(renderer)


class Shadow(Patch):

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    def __str__(self):
        return f"Shadow({self.patch})"

    @_docstring.interpd
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

            .. versionadded:: 3.8

        **kwargs
            Properties of the shadow patch. Supported keys are:

            %(Patch:kwdoc)s
        """
        self._shadowing(patch, ox, oy)
        self._native = _patches.new_shadow(
            patch._native, (float(ox), float(oy)), float(shade))
        Artist.__init__(self)
        # The patch's transform, as matplotlib's ``update_from`` copies it.
        if patch.is_transform_set():
            self._given_transform = Artist.get_transform(patch)
        # Just below the patch, as matplotlib's numpy float.
        _values.remember(self, "zorder", np.nextafter(patch.get_zorder(), -np.inf))
        self._internal_update(kwargs)

    def _shadowing(self, patch, ox, oy):
        """Make the object the shadow of *patch* offset by (*ox*, *oy*)
        points (also for the shadow of a wedge pltrs (Rust) made for
        ``pie``)."""
        self.patch = patch
        self._ox, self._oy = ox, oy
        self._shadow_transform = Affine2D()

    def get_path(self):
        return self.patch.get_path()

    def get_patch_transform(self):
        # The offset in points as the shadow is drawn, which matplotlib
        # sets when it draws the shadow (a shadow drawn is fresh, and stays
        # offset after that, also once removed from its figure).
        figure = self._home_figure(root=True)
        if figure is not None and not self.stale:
            self._drawn = True
        if self.__dict__.get("_drawn"):
            scale = figure.dpi / 72
            self._shadow_transform.clear().translate(self._ox * scale,
                                                     self._oy * scale)
        return self.patch.get_patch_transform() + self._shadow_transform

    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)


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


_docstring.interpd.register(
    FancyArrow="\n".join(
        (inspect.getdoc(FancyArrow.__init__) or "").splitlines()[2:]))


# What matplotlib's module also has from its imports, which programs
# import from it.
import functools  # noqa: E402, F401
import textwrap  # noqa: E402, F401
from collections import namedtuple  # noqa: E402, F401
from types import SimpleNamespace  # noqa: E402, F401
from . import artist, colors, hatch as mhatch, lines as mlines, transforms  # noqa: E402, F401
from .bezier import (  # noqa: E402, F401
    get_cos_sin, get_intersection, get_parallels, inside_circle, make_wedged_bezier2,
    NonIntersectingPathException, split_bezier_intersecting_with_closedpath,
    split_path_inout)
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
