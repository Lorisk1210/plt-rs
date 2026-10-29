"""
Classes for the efficient drawing of large collections of objects that
share most properties, e.g., a large number of line segments or
polygons (matplotlib's ``collections``).

A collection wraps the native handle of a pltrs (Rust) collection.
Colours cross to pltrs (Rust) as matplotlib keeps them, RGBA arrays;
the face and edge colours read back are the ones drawn, after the
defaults, the colour mapping and alpha.
"""

import math

import numpy as np

from . import _api, cbook, colors as mcolors, lines as mlines
from ._enums import CapStyle, JoinStyle
from ._native import collections as _collections, held as _held
from .artist import Artist
from .path import Path


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from . import rcParams
    return rcParams


def _rgba(colors):
    """*colors* (one colour or a sequence) as an ``(N, 4)`` RGBA array."""
    return mcolors.to_rgba_array(colors)


def _edge_spec(colors):
    """Edge colours in native form: ``None``, ``'face'`` or RGBA rows."""
    if colors is None or cbook._str_equal(colors, 'face'):
        return colors
    return _rgba(colors)


def _linestyle_specs(ls):
    """*ls* (one line style or a sequence) as a list of native styles."""
    try:
        mlines._get_dash_pattern(ls)
        styles = [ls]
    except ValueError:
        styles = list(ls)
        for style in styles:
            mlines._get_dash_pattern(style)
    return [_native_style(style) for style in styles]


def _native_style(style):
    """A line style as the native side takes it (tuples of floats)."""
    if isinstance(style, tuple):
        offset, seq = style
        return (offset, None if seq is None else tuple(seq))
    return style


def _native_kwargs(kwargs):
    """
    Collection keyword arguments, whose aliases are resolved, in the form
    the native collection properties take.
    """
    native = {}
    for key, value in kwargs.items():
        if key in ("color", "facecolor"):
            native[key] = None if value is None else _rgba(value)
        elif key == "edgecolor":
            native[key] = _edge_spec(value)
        elif key == "hatchcolor":
            native[key] = value if cbook._str_equal(value, 'edge') else _rgba(value)
        elif key == "linewidth":
            native[key] = [float(w) for w in np.atleast_1d(value)]
        elif key == "linestyle":
            native[key] = _linestyle_specs(value)
        elif key == "antialiased":
            native[key] = [bool(a) for a in np.atleast_1d(value)]
        elif key == "alpha" and np.iterable(value):
            native[key] = [float(a) for a in value]
        else:
            native[key] = value
    return native


@_api.define_aliases({
    "antialiased": ["antialiaseds", "aa"],
    "edgecolor": ["edgecolors", "ec"],
    "facecolor": ["facecolors", "fc"],
    "linestyle": ["linestyles", "dashes", "ls"],
    "linewidth": ["linewidths", "lw"],
})
class Collection(Artist):
    """
    Base class for Collections. Must be subclassed to be usable.

    A Collection represents a sequence of `.Patch`\\es that can be drawn
    more efficiently together than individually.
    """

    #: Either a list of 3x3 arrays or an Nx3x3 array (representing N
    #: transforms), suitable for the `all_transforms` argument to
    #: `~matplotlib.backend_bases.RendererBase.draw_path_collection`.
    _transforms = np.empty((0, 3, 3))

    # Whether to draw an edge by default.  Set on a
    # subclass-by-subclass basis.
    _edge_default = False

    def __init__(self, *,
                 edgecolors=None,
                 facecolors=None,
                 hatchcolors=None,
                 linewidths=None,
                 linestyles='solid',
                 capstyle=None,
                 joinstyle=None,
                 antialiaseds=None,
                 offsets=None,
                 offset_transform=None,
                 norm=None,
                 cmap=None,
                 colorizer=None,
                 pickradius=5.0,
                 hatch=None,
                 urls=None,
                 zorder=1,
                 **kwargs
                 ):
        """
        Style the collection a subclass holds in ``self._native``: the
        collection is made before it is added to an Axes
        (`.Axes.add_collection` adds it).
        """
        for name, value in (("offset_transform", offset_transform),
                            ("norm", norm), ("cmap", cmap),
                            ("colorizer", colorizer)):
            if value is not None:
                raise NotImplementedError(f"pltrs: {type(self).__name__}({name}=...)")
        self._figure = None
        self.set_facecolor(facecolors)
        self.set_edgecolor(edgecolors)
        self.set_linewidth(linewidths)
        self.set_linestyle(linestyles)
        self.set_antialiased(antialiaseds)
        self.set_pickradius(pickradius)
        self.set_urls(urls)
        self.set_hatch(hatch)
        self.set_hatchcolor(hatchcolors)
        self.set_zorder(zorder)
        if capstyle:
            self.set_capstyle(capstyle)
        if joinstyle:
            self.set_joinstyle(joinstyle)
        if offsets is not None:
            self.set_offsets(offsets)
        self._internal_update(kwargs)

    def _get(self, name):
        """The native property *name*."""
        return _collections.get(self._native, name)

    def get_paths(self):
        """Return the paths of the items."""
        return [Path(vertices, codes)
                for vertices, codes in _collections.get_paths(self._native)]

    def set_paths(self, paths):
        """Set the paths of the items."""
        _collections.set_paths(self._native, list(paths))

    def get_offsets(self):
        """Return the offsets for the collection."""
        return _collections.get_offsets(self._native)

    def set_offsets(self, offsets):
        """
        Set the offsets for the collection.

        Parameters
        ----------
        offsets : (N, 2) or (2,) array-like
        """
        offsets = np.asanyarray(offsets)
        if offsets.shape == (2,):  # Broadcast (2,) -> (1, 2) but nothing else.
            offsets = offsets[None, :]
        _collections.set_offsets(
            self._native, np.column_stack(
                (np.asarray(offsets[:, 0], dtype=float),
                 np.asarray(offsets[:, 1], dtype=float))).reshape(-1, 2))

    def get_pickradius(self):
        """Return the pick radius used for containment tests."""
        return _collections.get_pickradius(self._native)

    def set_pickradius(self, pickradius):
        """Set the pick radius used for containment tests, in points."""
        if not isinstance(pickradius, (int, float)):
            raise ValueError(
                f"pickradius must be a real-valued number, not {pickradius!r}")
        _collections.set_pickradius(self._native, float(pickradius))

    pickradius = property(get_pickradius, set_pickradius)

    def get_urls(self):
        """Return a list of URLs, one for each element of the collection."""
        return _collections.get_urls(self._native)

    def set_urls(self, urls):
        """Set the URLs, one per element (None: no link)."""
        _collections.set_urls(self._native,
                              list(urls) if urls is not None else [None])

    def get_hatch(self):
        """Return the current hatching pattern."""
        return self._get("hatch")

    def set_hatch(self, hatch):
        """Set the hatching pattern (see `.hatch`)."""
        from . import hatch as mhatch
        mhatch._validate_hatch_pattern(hatch)
        _collections.set_hatch(self._native, hatch)

    def get_hatch_linewidth(self):
        """Return the hatch linewidth."""
        return self._get("hatch_linewidth")

    def set_hatch_linewidth(self, lw):
        """Set the hatch linewidth."""
        _collections.set_hatch_linewidth(self._native, float(lw))

    def get_hatchcolor(self):
        """Return the hatch colors."""
        return self._get("hatchcolor")

    def set_hatchcolor(self, c):
        """Set the hatchcolor(s): colors, 'edge' or None (:rc:`hatch.color`)."""
        if c is None:
            c = _rcParams()['hatch.color']
        _collections.set_hatchcolors(
            self._native, c if cbook._str_equal(c, 'edge') else _rgba(c))

    def get_capstyle(self):
        """Return the cap style for the collection (for all its elements)."""
        return self._get("capstyle")

    def set_capstyle(self, cs):
        """Set the `.CapStyle` for the collection (for all its elements)."""
        _collections.set_capstyle(self._native, CapStyle(cs).name)

    def get_joinstyle(self):
        """Return the join style for the collection (for all its elements)."""
        return self._get("joinstyle")

    def set_joinstyle(self, js):
        """Set the `.JoinStyle` for the collection (for all its elements)."""
        _collections.set_joinstyle(self._native, JoinStyle(js).name)

    def get_linewidth(self):
        """Return the line widths, broadcast against the line styles."""
        return self._lwls()[0]

    def set_linewidth(self, lw):
        """Set the linewidth(s) for the collection (None: the default)."""
        if lw is None:
            lw = self._get_default_linewidth()
        _collections.set_linewidths(
            self._native, [float(w) for w in np.atleast_1d(lw)])

    def get_linestyle(self):
        """Return the dash patterns, scaled by the line widths."""
        return self._lwls()[1]

    def set_linestyle(self, ls):
        """
        Set the linestyle(s) for the collection: a line style or a
        sequence of them.
        """
        try:
            specs = _linestyle_specs(ls)
        except ValueError as err:
            raise ValueError(f'Do not know how to convert {ls!r} to dashes') from err
        _collections.set_linestyles(self._native, specs)

    def _lwls(self):
        """The line widths and scaled dash patterns, broadcast as
        matplotlib's ``_bcast_lwls`` does."""
        linewidths = np.asarray(self._get("linewidth"), dtype=float)
        dashes = [mlines._get_dash_pattern(style) for style in self._get("linestyle")]
        if _rcParams()['_internal.classic_mode']:
            return linewidths, dashes
        if len(dashes) != len(linewidths):
            gcd = math.gcd(len(dashes), len(linewidths))
            dashes, linewidths = (list(dashes) * (len(linewidths) // gcd),
                                  list(linewidths) * (len(dashes) // gcd))
        return linewidths, [mlines._scale_dashes(o, d, lw)
                            for (o, d), lw in zip(dashes, linewidths)]

    def _get_default_linewidth(self):
        return _rcParams()['patch.linewidth']

    def get_antialiased(self):
        """Get the antialiasing state for rendering."""
        antialiased = self._get("antialiased")
        if antialiased is None:
            antialiased = [_rcParams()['patch.antialiased']]
        return np.array(antialiased, dtype=bool)

    def set_antialiased(self, aa):
        """Set the antialiasing state for rendering."""
        if aa is None:
            aa = _rcParams()['patch.antialiased']
        _collections.set_antialiaseds(
            self._native, [bool(a) for a in np.atleast_1d(aa)])

    def get_facecolor(self):
        """Return the face colors, as drawn."""
        return self._get("facecolor")

    def set_facecolor(self, c):
        """
        Set the facecolor(s) of the collection. *c* can be a color (all
        patches have same color), or a sequence of colors; None restores
        the default.
        """
        if isinstance(c, str) and c.lower() in ("none", "face"):
            c = c.lower()
        _collections.set_facecolors(self._native, None if c is None else _rgba(c))

    def get_edgecolor(self):
        """Return the edge colors, as drawn."""
        return self._get("edgecolor")

    def set_edgecolor(self, c):
        """
        Set the edgecolor(s) of the collection: a color, a sequence of
        colors, ``'face'`` (the face colors) or None (the default rule).
        """
        if isinstance(c, str) and c.lower() in ("none", "face"):
            c = c.lower()
        _collections.set_edgecolors(self._native, _edge_spec(c))

    def set_color(self, c):
        """Set both the edgecolor and the facecolor."""
        self.set_facecolor(c)
        self.set_edgecolor(c)

    def set_alpha(self, alpha):
        """
        Set the transparency of the collection: a float, an array of one
        per element, or None.
        """
        if np.iterable(alpha):
            alpha = np.asarray(alpha, dtype=float)
            if ((alpha < 0) | (alpha > 1)).any():
                raise ValueError("alpha must be between 0 and 1, inclusive")
            _collections.set_alphas(self._native, list(alpha))
        else:
            if alpha is not None and not isinstance(alpha, (int, float)):
                raise TypeError(
                    f'alpha must be numeric or None, not {type(alpha)}')
            super().set_alpha(alpha)

    def get_array(self):
        """Return the array of values that are mapped to colors."""
        array = _collections.get_array(self._native)
        return None if array is None else np.ma.masked_invalid(array)

    def set_array(self, A):
        """Set the value array from array-like *A* (None: no mapping)."""
        if A is None:
            _collections.set_array(self._native, None)
            return
        A = cbook.safe_masked_invalid(A, copy=True)
        if not np.can_cast(A.dtype, float, "same_kind"):
            raise TypeError(f"Image data of dtype {A.dtype} cannot be "
                            "converted to float")
        _collections.set_array(
            self._native, np.ma.filled(A.astype(float), np.nan).ravel())

    def get_clim(self):
        """Return the values (min, max) that are mapped to the colormap limits."""
        return _collections.get_clim(self._native)

    def set_clim(self, vmin=None, vmax=None):
        """Set the norm limits for image scaling."""
        if vmax is None:
            try:
                vmin, vmax = vmin
            except (TypeError, ValueError):
                pass
        _collections.set_clim(self._native, None if vmin is None else float(vmin),
                              None if vmax is None else float(vmax))


class _CollectionWithSizes(Collection):
    """Base class for collections that have an array of sizes."""

    _factor = 1.0

    def get_sizes(self):
        """
        Return the sizes ('areas') of the elements in the collection.

        Returns
        -------
        array
            The 'area' of each element.
        """
        sizes = _collections.get_sizes(self._native)
        return np.zeros(0) if sizes is None else sizes

    def set_sizes(self, sizes, dpi=72.0):
        """Set the sizes of each member of the collection."""
        if sizes is None:
            sizes = np.array([])
        _collections.set_sizes(self._native, np.atleast_1d(np.asarray(sizes, float)))


class PathCollection(_CollectionWithSizes):
    r"""
    A collection of `~.path.Path`\s, as created by e.g. `~.Axes.scatter`.
    """

    def __init__(self, paths, sizes=None, **kwargs):
        """
        Parameters
        ----------
        paths : list of `.path.Path`
            The paths that will make up the `.Collection`.
        sizes : array-like
            The factor by which to scale each drawn `~.path.Path`.
        """
        self._native = _held.path_collection(list(paths))
        super().__init__(**kwargs)
        self.set_sizes(sizes)


class PolyCollection(_CollectionWithSizes):
    """A collection of polygons (as `~.Axes.fill_between` and `~.Axes.hexbin`
    make)."""

    def __init__(self, verts, sizes=None, *, closed=True, **kwargs):
        """
        Parameters
        ----------
        verts : list of array-like
            The sequence of polygons [*verts0*, *verts1*, ...] where each
            element *verts_i* defines the vertices of polygon *i* as a 2D
            array-like of shape (M, 2).
        sizes : array-like, default: None
            Squared scaling factors for the polygons.
        closed : bool, default: True
            Whether the polygon should be closed by adding a CLOSEPOLY
            connection at the end.
        """
        self._native = _held.poly_collection(
            [np.asarray(v, float).reshape(-1, 2) for v in verts])
        super().__init__(**kwargs)
        if sizes is not None:
            self.set_sizes(sizes)
        if not closed:
            self.set_verts(verts, closed=False)

    def set_verts(self, verts, closed=True):
        """
        Set the vertices of the polygons.

        Parameters
        ----------
        verts : list of array-like
            The sequence of polygons [*verts0*, *verts1*, ...] where each
            element *verts_i* defines the vertices of polygon *i* as a 2D
            array-like of shape (M, 2).
        closed : bool, default: True
            Whether the polygon should be closed by adding a CLOSEPOLY
            connection at the end.
        """
        if not closed:
            _collections.set_segments(
                self._native, [np.asarray(v, float).reshape(-1, 2) for v in verts])
            return
        _collections.set_verts(
            self._native, [np.asarray(v, float).reshape(-1, 2) for v in verts])

    set_paths = set_verts


class FillBetweenPolyCollection(PolyCollection):
    """
    `.PolyCollection` that fills the area between two x- or y-curves.
    """


class RegularPolyCollection(_CollectionWithSizes):
    """A collection of n-sided regular polygons."""

    def __init__(self, numsides, *, rotation=0, sizes=(1,), **kwargs):
        """
        Parameters
        ----------
        numsides : int
            The number of sides of the polygon.
        rotation : float
            The rotation of the polygon in radians.
        sizes : tuple of float
            The area of the circle circumscribing the polygon in points^2.
        """
        self._native = _held.regular_poly_collection(
            int(numsides), float(rotation), np.atleast_1d(np.asarray(sizes, float)))
        super().__init__(**kwargs)

    def get_numsides(self):
        """Return the number of sides of the polygons."""
        return len(self.get_paths()[0].vertices) - 1


class StarPolygonCollection(RegularPolyCollection):
    """Draw a collection of regular stars with *numsides* points."""


class AsteriskPolygonCollection(RegularPolyCollection):
    """Draw a collection of regular asterisks with *numsides* points."""


class LineCollection(Collection):
    r"""
    Represents a sequence of `.Line2D`\s that should be drawn together.
    """

    _edge_default = True

    def __init__(self, segments,  # Can be None.
                 *,
                 zorder=2,        # Collection.zorder is 1
                 **kwargs
                 ):
        """
        Parameters
        ----------
        segments : list of (N, 2) array-like
            A sequence ``[line0, line1, ...]`` where each line is a (N, 2)-shape
            array-like containing points::
        **kwargs
            Forwarded to `.Collection`.
        """
        kwargs.setdefault('facecolors', 'none')
        self._native = _held.line_collection(
            [] if segments is None else
            [np.asarray(seg, float).reshape(-1, 2) for seg in segments])
        super().__init__(zorder=zorder, **kwargs)

    def set_segments(self, segments):
        """Set the segments: a sequence of (N, 2) arrays, one per line."""
        if segments is None:
            return
        _collections.set_segments(
            self._native, [np.asarray(seg, float).reshape(-1, 2) for seg in segments])

    set_verts = set_segments  # for compatibility with PolyCollection
    set_paths = set_segments

    def get_segments(self):
        """
        Returns
        -------
        list
            List of segments in the LineCollection. Each list item contains an
            array of vertices.
        """
        return [path.vertices for path in self.get_paths()]

    def _get_default_linewidth(self):
        return _rcParams()['lines.linewidth']

    def get_antialiased(self):
        antialiased = self._get("antialiased")
        if antialiased is None:
            antialiased = [_rcParams()['lines.antialiased']]
        return np.array(antialiased, dtype=bool)

    def set_color(self, c):
        """Set the edgecolor(s) of the LineCollection."""
        self.set_edgecolor(c)

    set_colors = set_color

    def get_color(self):
        """Return the edge colors of the lines."""
        return self.get_edgecolor()

    get_colors = get_color  # for compatibility with old versions


class EventCollection(LineCollection):
    """
    A collection of locations along a single axis at which an "event"
    occurred.
    """

    _edge_default = True

    def __init__(self,
                 positions,  # Cannot be None.
                 orientation='horizontal',
                 *,
                 lineoffset=0,
                 linelength=1,
                 linewidth=None,
                 color=None,
                 linestyle='solid',
                 antialiased=None,
                 **kwargs
                 ):
        """
        Parameters
        ----------
        positions : 1D array-like
            Each value is an event.
        orientation : {'horizontal', 'vertical'}, default: 'horizontal'
            The sequence of events is plotted along this direction.
        lineoffset : float, default: 0
            The offset of the center of the markers from the origin.
        linelength : float, default: 1
            The total height of the marker.
        """
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        positions = np.asarray(positions, dtype=float)
        if positions.ndim > 1:
            raise ValueError('positions must be one-dimensional')
        self._native = _held.event_collection(
            np.atleast_1d(positions), orientation == 'vertical',
            float(lineoffset), float(linelength))
        Collection.__init__(self, linewidths=linewidth, linestyles=linestyle,
                            antialiaseds=antialiased, facecolors='none',
                            zorder=2, **kwargs)
        if color is not None:
            self.set_color(color)

    def get_positions(self):
        """
        Return an array containing the floating-point values of the positions.
        """
        return list(_collections.event_positions(self._native))

    def set_positions(self, positions):
        """Set the positions of the events."""
        if positions is None:
            positions = []
        if np.ndim(positions) != 1:
            raise ValueError('positions must be one-dimensional')
        _collections.set_event_positions(
            self._native, np.sort(np.asarray(positions, dtype=float)))

    def add_positions(self, position):
        """Add one or more events at the specified positions."""
        if position is None or (hasattr(position, 'len') and
                                len(position) == 0):
            return
        positions = self.get_positions()
        positions = np.hstack([positions, np.asanyarray(position)])
        self.set_positions(positions)

    extend_positions = append_positions = add_positions

    def is_horizontal(self):
        """True if the eventcollection is horizontal, False if vertical."""
        return self.get_orientation() == 'horizontal'

    def get_orientation(self):
        """
        Return the orientation of the event line ('horizontal' or 'vertical').
        """
        return _collections.event_orientation(self._native)

    def switch_orientation(self):
        """
        Switch the orientation of the event line, either from vertical to
        horizontal or vice versus.
        """
        self.set_orientation('vertical' if self.is_horizontal() else 'horizontal')

    def set_orientation(self, orientation):
        """
        Set the orientation of the event line.

        Parameters
        ----------
        orientation : {'horizontal', 'vertical'}
        """
        _api.getitem_checked({"horizontal": True, "vertical": False},
                             orientation=orientation)
        _collections.set_event_orientation(self._native, orientation)

    def get_linelength(self):
        """Return the length of the lines used to mark each event."""
        return _collections.event_lines(self._native)[1]

    def set_linelength(self, linelength):
        """Set the length of the lines used to mark each event."""
        _collections.set_event_lines(self._native, None, float(linelength))

    def get_lineoffset(self):
        """Return the offset of the lines used to mark each event."""
        return _collections.event_lines(self._native)[0]

    def set_lineoffset(self, lineoffset):
        """Set the offset of the lines used to mark each event."""
        _collections.set_event_lines(self._native, float(lineoffset), None)

    def get_linewidth(self):
        """Get the width of the lines used to mark each event."""
        return super().get_linewidth()[0]

    def get_linewidths(self):
        """Get the widths of the lines used to mark each event."""
        return super().get_linewidth()

    def get_color(self):
        """Return the color of the lines used to mark each event."""
        return self.get_colors()[0]


class CircleCollection(_CollectionWithSizes):
    """A collection of circles, drawn using splines."""

    def __init__(self, sizes, **kwargs):
        """
        Parameters
        ----------
        sizes : float or array-like
            The area of each circle in points^2.
        """
        self._native = _held.circle_collection(
            np.atleast_1d(np.asarray(sizes, float)))
        super().__init__(**kwargs)


class EllipseCollection(Collection):
    """A collection of ellipses, drawn using splines."""

    def __init__(self, widths, heights, angles, *, units='points', **kwargs):
        """
        Parameters
        ----------
        widths : array-like
            The lengths of the first axes (e.g., major axis lengths).
        heights : array-like
            The lengths of second axes.
        angles : array-like
            The angles of the first axes, degrees CCW from the x-axis.
        units : {'points', 'inches', 'dots', 'width', 'height', 'x', 'y', 'xy'}
            The units in which majors and minors are given.
        """
        _api.check_in_list(['points', 'inches', 'dots', 'width', 'height',
                            'x', 'y', 'xy'], units=units)
        widths, heights, angles = np.broadcast_arrays(
            np.ravel(widths).astype(float), np.ravel(heights).astype(float),
            np.ravel(angles).astype(float))
        self._native = _held.ellipse_collection((widths, heights, angles), units)
        super().__init__(**kwargs)


class PatchCollection(Collection):
    """
    A generic collection of patches.

    PatchCollection draws faster than a large number of equivalent individual
    Patches. It also makes it easier to assign a colormap to a heterogeneous
    collection of patches.
    """

    def __init__(self, patches, *, match_original=False, **kwargs):
        """
        Parameters
        ----------
        patches : list of `.Patch`
            A sequence of Patch objects.  This list may include
            a heterogeneous assortment of different patch types.
        match_original : bool, default: False
            If True, use the colors and linewidths of the original
            patches.  If False, new colors may be assigned by
            providing the standard collection arguments, facecolor,
            edgecolor, linewidths, norm or cmap.
        """
        patches = list(patches)
        self._native = _held.patch_collection([p._native for p in patches],
                                              bool(match_original))
        if match_original:
            self._figure = None
            self._internal_update(kwargs)
        else:
            super().__init__(**kwargs)

    def set_paths(self, patches):
        """Set the paths from the outlines of *patches*."""
        _collections.set_paths(self._native, [
            p.get_path().transformed(p.get_patch_transform()) for p in patches])


class TriMesh(Collection):
    """
    Class for the efficient drawing of a triangular mesh using Gouraud
    shading.
    """


class QuadMesh(Collection):
    """
    Class for the efficient drawing of a quadrilateral mesh.
    """


class PolyQuadMesh(PolyCollection):
    """
    Class for drawing a quadrilateral mesh as individual Polygons (as
    `~.Axes.pcolor` makes).
    """
