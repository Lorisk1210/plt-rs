"""
2D lines with support for a variety of line styles, markers, colors, etc.
(matplotlib's ``lines``).

A `Line2D` wraps the native handle of a pltrs (Rust) line: its getters
read the line's properties from ``_native.lines.get`` and its setters
write them through ``_native.lines.update``, which validates them as
matplotlib does. The module-level dash helpers are matplotlib's.
"""

import numpy as np

from . import _api, _live_transforms, cbook
from ._native import lines as _lines
from .artist import Artist
from .cbook import ls_mapper
from .markers import (  # noqa: F401  (matplotlib re-exports these here)
    CARETLEFT, CARETRIGHT, CARETUP, CARETDOWN,
    CARETLEFTBASE, CARETRIGHTBASE, CARETUPBASE, CARETDOWNBASE,
    TICKLEFT, TICKRIGHT, TICKUP, TICKDOWN, MarkerStyle)
from .path import Path


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


def _marker_spec(marker, fillstyle=None):
    """
    The native form of a marker: the `.MarkerStyle` spec of *marker*, its
    fill style, its user transform's matrix and its cap and join styles.
    """
    style = MarkerStyle(marker, fillstyle)
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
    to the caller.
    """
    kwargs = dict(kwargs)
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

    def __str__(self):
        label = self.get_label()
        if label:
            return f"Line2D({label})"
        x, y = self.get_xdata(), self.get_ydata()
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

        If *orig* is *True*, return the original data, else the
        processed data.
        """
        return _lines.get_xdata(self._native)

    def get_ydata(self, orig=True):
        """
        Return the ydata.

        If *orig* is *True*, return the original data, else the
        processed data.
        """
        return _lines.get_ydata(self._native)

    def get_data(self, orig=True):
        """
        Return the line data as an ``(xdata, ydata)`` pair.

        If *orig* is *True*, return the original data.
        """
        return self.get_xdata(orig=orig), self.get_ydata(orig=orig)

    def get_xydata(self):
        """Return the *xy* data as a (N, 2) array."""
        x, y = np.broadcast_arrays(self.get_xdata(), self.get_ydata())
        return np.column_stack([x, y]).astype(float)

    def get_path(self):
        """Return the `~matplotlib.path.Path` associated with this line."""
        return Path(*_lines.get_path(self._native))

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
        _lines.set_data(self._native, cbook._to_unmasked_float_array(x).ravel(),
                        cbook._to_unmasked_float_array(y).ravel())

    def set_xdata(self, x):
        """
        Set the data array for x.

        Parameters
        ----------
        x : 1D array
        """
        if not np.iterable(x):
            raise RuntimeError('x must be a sequence')
        self.set_data(x, self.get_ydata())

    def set_ydata(self, y):
        """
        Set the data array for y.

        Parameters
        ----------
        y : 1D array
        """
        if not np.iterable(y):
            raise RuntimeError('y must be a sequence')
        self.set_data(self.get_xdata(), y)

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

    def set_color(self, color):
        """Set the color of the line."""
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
        """Return the markevery setting for marker subsampling."""
        return self._get("markevery")

    def set_markevery(self, every):
        """
        Set the markevery property to subsample the plot when using
        markers: None, an int, a ``(start, step)`` pair, a slice, a list of
        indices or of bools, a float or a pair of floats.
        """
        self._set_native(markevery=every)

    def get_pickradius(self):
        """Return the pick radius used for containment tests."""
        return self._get("pickradius")

    def set_pickradius(self, pickradius):
        """Set the pick radius used for containment tests, in points."""
        if not isinstance(pickradius, (int, float)) or pickradius < 0:
            raise ValueError("pick radius should be a distance")
        self._set_native(pickradius=float(pickradius))

    pickradius = property(get_pickradius, set_pickradius)

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


def _capstyle_name(s):
    """The name of the cap style *s* (a `.CapStyle` or its name)."""
    from ._enums import CapStyle
    return CapStyle(s).name


def _joinstyle_name(s):
    """The name of the join style *s* (a `.JoinStyle` or its name)."""
    from ._enums import JoinStyle
    return JoinStyle(s).name
