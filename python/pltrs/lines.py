"""
2D lines with support for a variety of line styles, markers, colors, etc.
(matplotlib's ``lines``).

A `Line2D` wraps the native handle of a pltrs (Rust) line: its getters
read the line's properties from ``_native.lines.get`` and its setters
write them through ``_native.lines.update``, which validates them as
matplotlib does. The module-level dash helpers are matplotlib's.
"""

import copy

import numpy as np

from . import _api, _live_transforms, cbook
from ._artist_kwargs import native_artist_kwargs
from ._native import lines as _lines
from ._program_classes import takes_from_program
from .artist import Artist
from .cbook import ls_mapper, ls_mapper_r, STEP_LOOKUP_MAP  # noqa: F401
from .markers import (  # noqa: F401  (matplotlib re-exports these here)
    CARETLEFT, CARETRIGHT, CARETUP, CARETDOWN,
    CARETLEFTBASE, CARETRIGHTBASE, CARETUPBASE, CARETDOWNBASE,
    TICKLEFT, TICKRIGHT, TICKUP, TICKDOWN, MarkerStyle)
from .path import Path, _DrawnPath


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from . import rcParams
    return rcParams


def _get_dash_pattern(style):
    """Convert linestyle to dash pattern."""
    # go from short hand -> full strings
    if isinstance(style, str):
        style = ls_mapper.get(style, style)
    # un-dashed styles
    if style in ['solid', 'None', 'none', '', ' ']:
        offset = 0
        dashes = None
    # dashed styles
    elif style in ['dashed', 'dashdot', 'dotted']:
        offset = 0
        dashes = tuple(_rcParams()[f'lines.{style}_pattern'])
    #
    elif isinstance(style, tuple):
        offset, dashes = style
        if offset is None:
            raise ValueError(f'Unrecognized linestyle: {style!r}')
    else:
        raise ValueError(f'Unrecognized linestyle: {style!r}')

    # normalize offset to be positive and shorter than the dash cycle
    if dashes is not None:
        dsum = sum(dashes)
        if dsum:
            offset %= dsum

    return offset, dashes


def _get_dash_patterns(styles):
    """Convert linestyle or sequence of linestyles to list of dash patterns."""
    try:
        patterns = [_get_dash_pattern(styles)]
    except ValueError:
        try:
            patterns = [_get_dash_pattern(x) for x in styles]
        except ValueError as err:
            emsg = f'Do not know how to convert {styles!r} to dashes'
            raise ValueError(emsg) from err

    return patterns


def _get_inverse_dash_pattern(offset, dashes):
    """Return the inverse of the given dash pattern, for filling the gaps."""
    # Define the inverse pattern by moving the last gap to the start of the
    # sequence.
    gaps = dashes[-1:] + dashes[:-1]
    # Set the offset so that this new first segment is skipped
    # (see backend_bases.GraphicsContextBase.set_dashes for offset definition).
    offset_gaps = offset + dashes[-1]

    return offset_gaps, gaps


def _scale_dashes(offset, dashes, lw):
    if not _rcParams()['lines.scale_dashes']:
        return offset, dashes
    scaled_offset = offset * lw
    scaled_dashes = ([x * lw if x is not None else None for x in dashes]
                     if dashes is not None else None)
    return scaled_offset, scaled_dashes


def segment_hits(cx, cy, x, y, radius):
    """
    Return the indices of the segments in the polyline with coordinates (*cx*,
    *cy*) that are within a distance *radius* of the point (*x*, *y*).
    """
    # Process single points specially
    if len(x) <= 1:
        res, = np.nonzero((cx - x) ** 2 + (cy - y) ** 2 <= radius ** 2)
        return res

    # We need to lop the last element off a lot.
    xr, yr = x[:-1], y[:-1]

    # Only look at line segments whose nearest point to C on the line
    # lies within the segment.
    dx, dy = x[1:] - xr, y[1:] - yr
    Lnorm_sq = dx ** 2 + dy ** 2  # Possibly want to eliminate Lnorm==0
    u = ((cx - xr) * dx + (cy - yr) * dy) / Lnorm_sq
    candidates = (u >= 0) & (u <= 1)

    # Note that there is a little area near one side of each point
    # which will be near neither segment, and another which will
    # be near both, depending on the angle of the lines.  The
    # following radius test eliminates these ambiguities.
    point_hits = (cx - x) ** 2 + (cy - y) ** 2 <= radius ** 2
    candidates = candidates & ~(point_hits[:-1] | point_hits[1:])

    # For those candidates which remain, determine how far they lie away
    # from the line.
    px, py = xr + u * dx, yr + u * dy
    line_hits = (cx - px) ** 2 + (cy - py) ** 2 <= radius ** 2
    line_hits = line_hits & candidates
    points, = point_hits.ravel().nonzero()
    lines, = line_hits.ravel().nonzero()
    return np.concatenate((points, lines))


def _mark_every_path(markevery, tpath, affine, ax):
    """
    Helper function that sorts out how to deal the input
    `markevery` and returns the points where markers should be drawn.

    Takes in the `markevery` value and the line path and returns the
    sub-sampled path; pltrs (Rust) selects the vertices.
    """
    codes, verts = tpath.codes, tpath.vertices
    diagonal = None
    if ax is not None:
        (x0, y0), (x1, y1) = ax.transAxes.transform([[0, 0], [1, 1]])
        diagonal = float(np.hypot(x1 - x0, y1 - y0))
    inds = _lines.markevery_indices(
        markevery, np.asarray(affine.transform(verts), dtype=float), diagonal)
    return Path(verts[inds], None if codes is None else codes[inds])


def _marker_spec(marker, fillstyle=None):
    """
    The native form of a marker: the `.MarkerStyle` spec of *marker*, its
    fill style, its user transform's matrix and its cap and join styles.
    """
    # A MarkerStyle without a fill style to impose is read as it is: a
    # copy of it would hold the same.
    style = (marker if isinstance(marker, MarkerStyle) and fillstyle is None
             else MarkerStyle(marker, fillstyle))
    transform = style.get_user_transform()
    return (style._spec if style._spec is not None else ("code", "None"),
            style.get_fillstyle(),
            None if transform is None else transform.get_matrix(),
            None if style._user_capstyle is None else style._user_capstyle.name,
            None if style._user_joinstyle is None
            else style._user_joinstyle.name)


def _native_kwargs(kwargs):
    """
    *kwargs*, whose aliases are resolved, in the form the native line
    properties take: markers as `_marker_spec`, ``dashes`` as a line style,
    a transform as its native transform, ``data``/``xdata``/``ydata`` left
    to the caller, the properties of every artist as
    `._artist_kwargs.native_artist_kwargs` makes them.
    """
    kwargs = native_artist_kwargs(kwargs)
    if "transform" in kwargs:
        kwargs["transform"] = _live_transforms.to_native(kwargs["transform"])
    fillstyle = kwargs.get("fillstyle")
    if "marker" in kwargs:
        kwargs["marker"] = _marker_spec(kwargs["marker"], fillstyle)
    if "dashes" in kwargs:
        seq = kwargs.pop("dashes")
        if seq == (None, None) or len(seq) == 0:
            kwargs["linestyle"] = "-"
        else:
            kwargs["linestyle"] = (0, tuple(seq))
    if isinstance(kwargs.get("linestyle"), tuple):
        offset, seq = kwargs["linestyle"]
        kwargs["linestyle"] = (offset, None if seq is None else tuple(seq))
    return kwargs


@_api.define_aliases({
    "antialiased": ["aa"],
    "color": ["c"],
    "drawstyle": ["ds"],
    "linestyle": ["ls"],
    "linewidth": ["lw"],
    "markeredgecolor": ["mec"],
    "markeredgewidth": ["mew"],
    "markerfacecolor": ["mfc"],
    "markerfacecoloralt": ["mfcalt"],
    "markersize": ["ms"],
})
class Line2D(Artist):
    """
    A line - the line can have both a solid linestyle connecting all
    the vertices, and a marker at each vertex.  Additionally, the
    drawing of the solid line is influenced by the drawstyle, e.g., one
    can create "stepped" lines in various styles.
    """

    lineStyles = _lineStyles = {  # hidden names deprecated
        '-':    '_draw_solid',
        '--':   '_draw_dashed',
        '-.':   '_draw_dash_dot',
        ':':    '_draw_dotted',
        'None': '_draw_nothing',
        ' ':    '_draw_nothing',
        '':     '_draw_nothing',
    }

    _drawStyles_l = {
        'default':    '_draw_lines',
        'steps-mid':  '_draw_steps_mid',
        'steps-pre':  '_draw_steps_pre',
        'steps-post': '_draw_steps_post',
    }

    _drawStyles_s = {
        'steps': '_draw_steps_pre',
    }

    # drawStyles should now be deprecated.
    drawStyles = {**_drawStyles_l, **_drawStyles_s}
    # Need a list ordered with long names first:
    drawStyleKeys = [*_drawStyles_l, *_drawStyles_s]

    # Referenced here to maintain API.  These are defined in
    # MarkerStyle
    markers = MarkerStyle.markers
    filled_markers = MarkerStyle.filled_markers
    fillStyles = MarkerStyle.fillstyles

    zorder = 2

    #: The number of points above which pltrs (Rust) draws only the part
    #: of a line with sorted x in the view (``pltrs::lines``' subslicing).
    _subslice_optim_min_size = 1000

    def __str__(self):
        label = self.get_label()
        if label:
            return f"Line2D({label})"
        x, y = self.get_xdata(orig=False), self.get_ydata(orig=False)
        if len(x) > 3:
            return "Line2D(({:g},{:g}),({:g},{:g}),...,({:g},{:g}))".format(
                x[0], y[0], x[1], y[1], x[-1], y[-1])
        return "Line2D(%s)" % ",".join(
            map("({:g},{:g})".format, x, y))

    def __init__(self, xdata, ydata, *,
                 linewidth=None,  # all Nones default to rc
                 linestyle=None,
                 color=None,
                 gapcolor=None,
                 marker=None,
                 markersize=None,
                 markeredgewidth=None,
                 markeredgecolor=None,
                 markerfacecolor=None,
                 markerfacecoloralt='none',
                 fillstyle=None,
                 antialiased=None,
                 dash_capstyle=None,
                 solid_capstyle=None,
                 dash_joinstyle=None,
                 solid_joinstyle=None,
                 pickradius=5,
                 drawstyle=None,
                 markevery=None,
                 **kwargs
                 ):
        """
        Create a `.Line2D` instance with *x* and *y* data in sequences of
        *xdata*, *ydata*.

        The line is outside any figure until an axes or a figure adds it
        (`~.Axes.add_line`, `.Figure.add_artist`).

        Additional keyword arguments are `.Line2D` properties.
        """
        if not np.iterable(xdata):
            raise RuntimeError('xdata must be a sequence')
        if not np.iterable(ydata):
            raise RuntimeError('ydata must be a sequence')
        props = {
            "linewidth": linewidth, "linestyle": linestyle, "color": color,
            "gapcolor": gapcolor, "markersize": markersize,
            "markeredgewidth": markeredgewidth,
            "markeredgecolor": markeredgecolor,
            "markerfacecolor": markerfacecolor,
            "markerfacecoloralt": markerfacecoloralt,
            "fillstyle": fillstyle, "antialiased": antialiased,
            "dash_capstyle": None if dash_capstyle is None
            else _capstyle_name(dash_capstyle),
            "solid_capstyle": None if solid_capstyle is None
            else _capstyle_name(solid_capstyle),
            "dash_joinstyle": None if dash_joinstyle is None
            else _joinstyle_name(dash_joinstyle),
            "solid_joinstyle": None if solid_joinstyle is None
            else _joinstyle_name(solid_joinstyle),
            "pickradius": float(pickradius), "drawstyle": drawstyle,
        }
        props = {k: v for k, v in props.items() if v is not None}
        if marker is not None:
            props["marker"] = marker
        self._native = _lines.new(
            cbook._to_unmasked_float_array(xdata).ravel(),
            cbook._to_unmasked_float_array(ydata).ravel(), _native_kwargs(props))
        super().__init__()
        self._xorig, self._yorig = copy.copy(xdata), copy.copy(ydata)
        # matplotlib's __init__ sets the data through `set_data`, which a
        # class of the program's may override.
        if takes_from_program(type(self), "set_data"):
            self.set_data(xdata, ydata)
        if markevery is not None:
            self.set_markevery(markevery)
        self._internal_update(kwargs)

    def _get(self, name):
        """The native property *name*."""
        return _lines.get(self._native, name)

    def _set_native(self, **kwargs):
        """Set native properties by their resolved names."""
        _lines.update(self._native, _native_kwargs(kwargs))

    # -- data -------------------------------------------------------------

    def get_xdata(self, orig=True):
        """
        Return the xdata.

        If *orig* is *True*, return the original data (as it was given to
        the line), else the processed data.
        """
        if orig and "_xorig" in self.__dict__:
            return self._xorig
        return _lines.get_xdata(self._native)

    def get_ydata(self, orig=True):
        """
        Return the ydata.

        If *orig* is *True*, return the original data (as it was given to
        the line), else the processed data.
        """
        if orig and "_yorig" in self.__dict__:
            return self._yorig
        return _lines.get_ydata(self._native)

    def get_data(self, orig=True):
        """
        Return the line data as an ``(xdata, ydata)`` pair.

        If *orig* is *True*, return the original data.
        """
        return self.get_xdata(orig=orig), self.get_ydata(orig=orig)

    def get_xydata(self):
        """Return the *xy* data as a (N, 2) array."""
        x, y = np.broadcast_arrays(self.get_xdata(orig=False),
                                   self.get_ydata(orig=False))
        return np.column_stack([x, y]).astype(float)

    def get_path(self):
        """Return the `~matplotlib.path.Path` associated with this line."""
        return _DrawnPath(*_lines.get_path(self._native), (
            lambda: _lines.get_interpolation_steps(self._native),
            lambda steps: _lines.set_interpolation_steps(self._native, steps)))

    def set_data(self, *args):
        """
        Set the x and y data.

        Parameters
        ----------
        *args : (2, N) array or two 1D arrays

        See Also
        --------
        set_xdata
        set_ydata
        """
        if len(args) == 1:
            (x, y), = args
        else:
            x, y = args
        if not np.iterable(x):
            raise RuntimeError('x must be a sequence')
        if not np.iterable(y):
            raise RuntimeError('y must be a sequence')
        self._xorig, self._yorig = copy.copy(x), copy.copy(y)
        self.recache_always()

    def recache_always(self):
        """Convert the original data through the units of the line's axes
        again, and hand the numbers to pltrs (Rust)."""
        self.recache(always=True)

    def recache(self, always=False):
        """
        Convert the original data through the units of the line's axes
        again when *always* is true, and hand the numbers to pltrs (Rust).

        pltrs (Rust) builds the line's path from its data whenever it is
        drawn, and every setter of the data hands the numbers over at once,
        so without *always* nothing is waiting to be converted.
        """
        if not always or "_xorig" not in self.__dict__:
            return
        _lines.set_data(
            self._native,
            cbook._to_unmasked_float_array(self.convert_xunits(self._xorig)).ravel(),
            cbook._to_unmasked_float_array(self.convert_yunits(self._yorig)).ravel())

    def set_xdata(self, x):
        """
        Set the data array for x.

        Parameters
        ----------
        x : 1D array
        """
        if not np.iterable(x):
            raise RuntimeError('x must be a sequence')
        self.set_data(x, self.get_ydata(orig=True))

    def set_ydata(self, y):
        """
        Set the data array for y.

        Parameters
        ----------
        y : 1D array
        """
        if not np.iterable(y):
            raise RuntimeError('y must be a sequence')
        self.set_data(self.get_xdata(orig=True), y)

    # -- style ------------------------------------------------------------

    def get_antialiased(self):
        """Return whether antialiased rendering is used."""
        return self._get("antialiased")

    def set_antialiased(self, b):
        """Set whether to use antialiased rendering."""
        self._set_native(antialiased=bool(b))

    def get_color(self):
        """Return the line color."""
        return self._get("color")

    @property
    def _color(self):
        """The line color (matplotlib's attribute, which programs read
        directly)."""
        return self.get_color()

    def set_color(self, color):
        """Set the color of the line."""
        from . import colors as mcolors
        mcolors._check_color_like(color=color)
        self._set_native(color=color)

    def get_drawstyle(self):
        """Return the drawstyle."""
        return self._get("drawstyle")

    def set_drawstyle(self, drawstyle):
        """
        Set the drawstyle of the plot: 'default', 'steps', 'steps-pre',
        'steps-mid' or 'steps-post'.
        """
        if drawstyle is None:
            drawstyle = 'default'
        _api.check_in_list(self.drawStyles, drawstyle=drawstyle)
        self._set_native(drawstyle=drawstyle)

    def get_gapcolor(self):
        """Return the line gapcolor."""
        return self._get("gapcolor")

    def set_gapcolor(self, gapcolor):
        """Set a color to fill the gaps in the dashed line style."""
        self._set_native(gapcolor=gapcolor)

    def get_linestyle(self):
        """Return the linestyle."""
        return self._get("linestyle")

    def set_linestyle(self, ls):
        """
        Set the linestyle of the line: one of ``'-'``, ``'--'``, ``'-.'``,
        ``':'``, ``''`` (and their long names), or an ``(offset,
        onoffseq)`` tuple.
        """
        if isinstance(ls, str):
            if ls in [' ', '', 'none']:
                ls = 'None'
            _api.check_in_list([*self._lineStyles, *cbook.ls_mapper_r], ls=ls)
        self._set_native(linestyle=ls)

    @property
    def _dash_pattern(self):
        """The ``(offset, dashes)`` the line is drawn with, scaled by its
        width as :rc:`lines.scale_dashes` says."""
        offset, dashes = _get_dash_pattern(self._get("dash_pattern"))
        return _scale_dashes(offset, None if dashes is None else tuple(dashes),
                             self.get_linewidth())

    def set_dashes(self, seq):
        """
        Set the dash sequence: on/off ink in points; an empty sequence or
        ``(None, None)`` makes the line solid.
        """
        self._set_native(dashes=seq)

    def is_dashed(self):
        """Return whether line has a dashed linestyle."""
        return _lines.is_dashed(self._native)

    def get_linewidth(self):
        """Return the linewidth in points."""
        return self._get("linewidth")

    def set_linewidth(self, w):
        """Set the line width in points."""
        self._set_native(linewidth=float(w))

    def get_marker(self):
        """Return the line marker."""
        marker = self._get("marker")
        if isinstance(marker, tuple) and marker[:1] == ("path",):
            return Path(*marker[1])
        return marker

    def set_marker(self, marker):
        """Set the line marker."""
        self._set_native(marker=marker, fillstyle=self.get_fillstyle())

    def get_fillstyle(self):
        """Return the marker fill style."""
        return self._get("fillstyle")

    def set_fillstyle(self, fs):
        """
        Set the marker fill style: 'full', 'left', 'right', 'bottom',
        'top' or 'none'.
        """
        _api.check_in_list(MarkerStyle.fillstyles, fs=fs)
        self._set_native(fillstyle=fs)

    def get_markersize(self):
        """Return the marker size in points."""
        return self._get("markersize")

    def set_markersize(self, sz):
        """Set the marker size in points."""
        if sz is None:
            sz = _rcParams()['lines.markersize']
        self._set_native(markersize=float(sz))

    def get_markeredgewidth(self):
        """Return the marker edge width in points."""
        return self._get("markeredgewidth")

    def set_markeredgewidth(self, ew):
        """Set the marker edge width in points."""
        if ew is None:
            ew = _rcParams()['lines.markeredgewidth']
        self._set_native(markeredgewidth=float(ew))

    def get_markeredgecolor(self):
        """Return the marker edge color."""
        return self._get("markeredgecolor")

    def set_markeredgecolor(self, ec):
        """Set the marker edge color."""
        self._set_native(markeredgecolor=ec)

    def get_markerfacecolor(self):
        """Return the marker face color."""
        return self._get("markerfacecolor")

    def set_markerfacecolor(self, fc):
        """Set the marker face color."""
        self._set_native(markerfacecolor=fc)

    def get_markerfacecoloralt(self):
        """Return the alternate marker face color."""
        return self._get("markerfacecoloralt")

    def set_markerfacecoloralt(self, fc):
        """Set the alternate marker face color."""
        self._set_native(markerfacecoloralt=fc)

    def get_markevery(self):
        """Return the markevery setting for marker subsampling, as it was set."""
        if "_markevery" in self.__dict__:
            return self._markevery
        return self._get("markevery")

    def set_markevery(self, every):
        """
        Set the markevery property to subsample the plot when using
        markers: None, an int, a ``(start, step)`` pair, a slice, a list of
        indices or of bools, a float or a pair of floats.
        """
        self._set_native(markevery=every)
        self._markevery = every

    def get_pickradius(self):
        """Return the pick radius used for containment tests."""
        return self._get("pickradius")

    def set_pickradius(self, pickradius):
        """Set the pick radius used for containment tests, in points."""
        if not isinstance(pickradius, (int, float)) or pickradius < 0:
            raise ValueError("pick radius should be a distance")
        self._set_native(pickradius=float(pickradius))

    pickradius = property(get_pickradius, set_pickradius)

    def set_picker(self, p):
        """
        Set the event picker details for the line.

        Parameters
        ----------
        p : float or callable[[Artist, Event], tuple[bool, dict]]
            If a float, it is used as the pick radius in points.
        """
        if not callable(p):
            self.set_pickradius(p)
        super().set_picker(p)

    def _contains_details(self, ind):
        """The indices of the points and segments hit."""
        return dict(ind=ind)

    def get_dash_capstyle(self):
        """Return the `.CapStyle` for dashed lines."""
        return self._get("dash_capstyle")

    def set_dash_capstyle(self, s):
        """How to draw the end caps if the line is `~Line2D.is_dashed`."""
        self._set_native(dash_capstyle=_capstyle_name(s))

    def get_solid_capstyle(self):
        """Return the `.CapStyle` for solid lines."""
        return self._get("solid_capstyle")

    def set_solid_capstyle(self, s):
        """How to draw the end caps if the line is solid (not dashed)."""
        self._set_native(solid_capstyle=_capstyle_name(s))

    def get_dash_joinstyle(self):
        """Return the `.JoinStyle` for dashed lines."""
        return self._get("dash_joinstyle")

    def set_dash_joinstyle(self, s):
        """How to join segments of the line if it `~Line2D.is_dashed`."""
        self._set_native(dash_joinstyle=_joinstyle_name(s))

    def get_solid_joinstyle(self):
        """Return the `.JoinStyle` for solid lines."""
        return self._get("solid_joinstyle")

    def set_solid_joinstyle(self, s):
        """How to join segments if the line is solid (not dashed)."""
        self._set_native(solid_joinstyle=_joinstyle_name(s))

    def update_from(self, other):
        """Copy properties from *other* to self."""
        super().update_from(other)
        self._set_native(
            linestyle=other._get("dash_pattern"),
            linewidth=other.get_linewidth(), color=other.get_color(),
            gapcolor=other.get_gapcolor(), markersize=other.get_markersize(),
            markerfacecolor=other._get("markerfacecolor_setting"),
            markerfacecoloralt=other.get_markerfacecoloralt(),
            markeredgecolor=other._get("markeredgecolor_setting"),
            markeredgewidth=other.get_markeredgewidth(),
            dash_capstyle=other.get_dash_capstyle(),
            dash_joinstyle=other.get_dash_joinstyle(),
            solid_capstyle=other.get_solid_capstyle(),
            solid_joinstyle=other.get_solid_joinstyle(),
            marker=other.get_marker(), fillstyle=other.get_fillstyle(),
            drawstyle=other.get_drawstyle())

    def get_bbox(self):
        """Get the bounding box of this line."""
        from .transforms import Bbox
        bbox = Bbox([[0, 0], [0, 0]])
        bbox.update_from_data_xy(self.get_xydata())
        return bbox


def _capstyle_name(s):
    """The name of the cap style *s* (a `.CapStyle` or its name)."""
    from ._enums import CapStyle
    return CapStyle(s).name


def _joinstyle_name(s):
    """The name of the join style *s* (a `.JoinStyle` or its name)."""
    from ._enums import JoinStyle
    return JoinStyle(s).name


class AxLine(Line2D):
    """
    A helper class that implements `~.Axes.axline`, by recomputing the artist
    transform at draw time.

    pltrs: pltrs (Rust) places the line in the view each time it is laid
    out; `get_transform` maps the diagonal of the unit square onto the part
    of the line the view shows now.
    """

    def get_transform(self):
        # docstring inherited
        from .transforms import Bbox, BboxTransformTo
        segment = _lines.axline_segment(self._native)
        if segment is None:
            return super().get_transform()
        return BboxTransformTo(Bbox(segment))

    def get_xy1(self):
        """Return the *xy1* value of the line."""
        return _lines.axline(self._native)[0]

    def get_xy2(self):
        """Return the *xy2* value of the line."""
        return _lines.axline(self._native)[1]

    def get_slope(self):
        """Return the *slope* value of the line."""
        return _lines.axline(self._native)[2]

    def set_xy1(self, *args, **kwargs):
        """
        Set the *xy1* value of the line.

        Parameters
        ----------
        xy1 : tuple[float, float]
            Points for the line to pass through.
        """
        params = _api.select_matching_signature([
            lambda self, x, y: locals(), lambda self, xy1: locals(),
        ], self, *args, **kwargs)
        if "x" in params:
            _api.warn_deprecated("3.10", message=(
                "Passing x and y separately to AxLine.set_xy1 is deprecated since "
                "%(since)s; pass them as a single tuple instead."))
            xy1 = params["x"], params["y"]
        else:
            xy1 = params["xy1"]
        _lines.set_axline_xy1(self._native, tuple(map(float, xy1)))
        self.stale = True

    def set_xy2(self, *args, **kwargs):
        """
        Set the *xy2* value of the line.

        .. note::

            You can only set *xy2* if the line was created using the *xy2*
            parameter. If the line was created using *slope*, please use
            `~.AxLine.set_slope`.

        Parameters
        ----------
        xy2 : tuple[float, float]
            Points for the line to pass through.
        """
        if self.get_slope() is not None:
            raise ValueError("Cannot set an 'xy2' value while 'slope' is set;"
                             " they differ but their functionalities overlap")
        params = _api.select_matching_signature([
            lambda self, x, y: locals(), lambda self, xy2: locals(),
        ], self, *args, **kwargs)
        if "x" in params:
            _api.warn_deprecated("3.10", message=(
                "Passing x and y separately to AxLine.set_xy2 is deprecated since "
                "%(since)s; pass them as a single tuple instead."))
            xy2 = params["x"], params["y"]
        else:
            xy2 = params["xy2"]
        _lines.set_axline_xy2(self._native, tuple(map(float, xy2)))
        self.stale = True

    def set_slope(self, slope):
        """
        Set the *slope* value of the line.

        .. note::

            You can only set *slope* if the line was created using the *slope*
            parameter. If the line was created using *xy2*, please use
            `~.AxLine.set_xy2`.

        Parameters
        ----------
        slope : float
            The slope of the line.
        """
        _lines.set_axline_slope(self._native, float(slope))
        self.stale = True


class VertexSelector:
    """
    Manage the callbacks to maintain a list of selected vertices for `.Line2D`.
    Derived classes should override the `process_selected` method to do
    something with the picks (matplotlib's class, kept as it is).

    Here is an example which highlights the selected verts with red circles::

        import numpy as np
        import matplotlib.pyplot as plt
        import matplotlib.lines as lines

        class HighlightSelected(lines.VertexSelector):
            def __init__(self, line, fmt='ro', **kwargs):
                super().__init__(line)
                self.markers, = self.axes.plot([], [], fmt, **kwargs)

            def process_selected(self, ind, xs, ys):
                self.markers.set_data(xs, ys)
                self.canvas.draw()

        fig, ax = plt.subplots()
        x, y = np.random.rand(2, 30)
        line, = ax.plot(x, y, 'bs-', picker=5)

        selector = HighlightSelected(line)
        plt.show()
    """

    def __init__(self, line):
        """
        Parameters
        ----------
        line : `~matplotlib.lines.Line2D`
            The line must already have been added to an `~.axes.Axes` and must
            have its picker property set.
        """
        if line.axes is None:
            raise RuntimeError('You must first add the line to the Axes')
        if line.get_picker() is None:
            raise RuntimeError('You must first set the picker property '
                               'of the line')
        self.axes = line.axes
        self.line = line
        self.cid = self.canvas.callbacks._connect_picklable(
            'pick_event', self.onpick)
        self.ind = set()

    canvas = property(lambda self: self.axes.get_figure(root=True).canvas)

    def process_selected(self, ind, xs, ys):
        """
        Default "do nothing" implementation of the `process_selected` method.

        Parameters
        ----------
        ind : list of int
            The indices of the selected vertices.
        xs, ys : array-like
            The coordinates of the selected vertices.
        """

    def onpick(self, event):
        """When the line is picked, update the set of selected indices."""
        if event.artist is not self.line:
            return
        self.ind ^= set(event.ind)
        ind = sorted(self.ind)
        xdata, ydata = self.line.get_data()
        self.process_selected(ind, xdata[ind], ydata[ind])


lineStyles = Line2D._lineStyles
lineMarkers = MarkerStyle.markers
drawStyles = Line2D.drawStyles
fillStyles = MarkerStyle.fillstyles
