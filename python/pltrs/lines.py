"""
2D lines with support for a variety of line styles, markers, colors, etc.
(matplotlib's ``lines``).

A `Line2D` wraps the native handle of a pltrs (Rust) line: its getters
read the line's properties from ``_native.lines.get`` and its setters
write them through ``_native.lines.update``, which validates them as
matplotlib does. The module-level dash helpers are matplotlib's.
"""

import copy
import functools
import logging

import numpy as np

from . import _docstring
from . import _api, _live_transforms, _mirrors, _unusable, _values, cbook, units
from ._artist_kwargs import native_artist_kwargs
from ._native import artist as _artist, lines as _lines
from ._program_classes import takes_from_program
from .artist import Artist, _check_alpha
from .cbook import ls_mapper, ls_mapper_r, STEP_LOOKUP_MAP  # noqa: F401
from .markers import (  # noqa: F401  (matplotlib re-exports these here)
    CARETLEFT, CARETRIGHT, CARETUP, CARETDOWN,
    CARETLEFTBASE, CARETRIGHTBASE, CARETUPBASE, CARETDOWNBASE,
    TICKLEFT, TICKRIGHT, TICKUP, TICKDOWN, MarkerStyle)
from .path import Path, drawn_path

_log = logging.getLogger(__name__)


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
    try:
        inds = _lines.markevery_indices(
            markevery, np.asarray(affine.transform(verts), dtype=float), diagonal)
    except ValueError:
        if not np.iterable(markevery) or isinstance(markevery, tuple):
            raise
        # matplotlib indexes the vertices with any other iterable, numpy
        # refusing what is no index.
        try:
            verts[markevery]
        except (ValueError, IndexError) as err:
            raise ValueError(
                f"markevery={markevery!r} is iterable but not a valid numpy "
                f"fancy index") from err
        raise
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
    from . import colors as mcolors
    for name in ("color", "gapcolor"):
        if kwargs.get(name) is not None:
            # As matplotlib's `set_color` and `set_gapcolor` check them.
            mcolors._check_color_like(color=kwargs[name])
    if "alpha" in kwargs:
        # As matplotlib's `set_alpha` checks it.
        _check_alpha(kwargs["alpha"])
    kwargs = native_artist_kwargs(kwargs)
    # A number as matplotlib's setters (its draw, for the edge width) read
    # it, `mew="1"` being 1; the line reads back what it was given.
    kwargs = {name: float(value)
              if name in ("linewidth", "markersize", "markeredgewidth") and value is not None
              else value
              for name, value in kwargs.items()}
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
        # A pattern fails where matplotlib's `set_linestyle` reads it.
        _get_dash_pattern(kwargs["linestyle"])
        offset, seq = kwargs["linestyle"]
        kwargs["linestyle"] = (offset, None if seq is None else tuple(seq))
    return kwargs


#: The colours a line reads back as the program gave them, as matplotlib
#: keeps the objects given (`._values`).
_GIVEN_COLORS = ("color", "gapcolor", "markeredgecolor", "markerfacecolor",
                 "markerfacecoloralt")
#: The numbers a line reads back as given.
_GIVEN_NUMBERS = ("markeredgewidth", "pickradius")


def _same_number(value, native):
    """Whether *value*, a number given as matplotlib takes it (a string of
    one too, which its draw reads with ``float``), is *native*."""
    try:
        return float(value) == native
    except (TypeError, ValueError):
        return False


def _remember_given(line, props):
    """Remember what *line* reads back of its properties *props*, given
    by any of their names (``mew``)."""
    _values.remember_all(line, cbook.normalize_kwargs(props, Line2D),
                         _GIVEN_COLORS + _GIVEN_NUMBERS)


@_docstring.interpd
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
@_mirrors.mirrorable
class Line2D(Artist):
    """
    A line - the line can have both a solid linestyle connecting all
    the vertices, and a marker at each vertex.  Additionally, the
    drawing of the solid line is influenced by the drawstyle, e.g., one
    can create "stepped" lines in various styles.
    """

    def set_transform(self, t):
        # docstring inherited: matplotlib's override has none.
        super().set_transform(t)

    #: The getters of a mirror that read what drawing its part changed
    #: (a tick's lines, which matplotlib's draw places).
    _refreshed_getters = ("get_xdata", "get_ydata")

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

    #: The index of the first vertex drawn, which matplotlib's `contains`
    #: adds to the indices it finds; pltrs (Rust) finds them in the whole
    #: line.
    ind_offset = 0

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

        Additional keyword arguments are `.Line2D` properties:

        %(Line2D:kwdoc)s

        See :meth:`set_linestyle` for a description of the line styles,
        :meth:`set_marker` for a description of the markers, and
        :meth:`set_drawstyle` for a description of the draw styles.

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
            "pickradius": pickradius, "drawstyle": drawstyle,
        }
        props = {k: v for k, v in props.items() if v is not None}
        if marker is not None:
            props["marker"] = marker
        if units._is_natively_supported(xdata) and units._is_natively_supported(ydata):
            x = cbook._to_unmasked_float_array(xdata).ravel()
            y = cbook._to_unmasked_float_array(ydata).ravel()
        else:
            # Values in units convert once the line joins an axes
            # (`_convert_units_for`), as matplotlib converts them when drawn.
            x = y = np.empty(0)
            self._unit_data = True
        self._native = _lines.new(
            x, y, _native_kwargs({**props, "pickradius": float(pickradius)}))
        super().__init__()
        _remember_given(self, props)
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
        _values.remember_all(self, kwargs, _GIVEN_COLORS)

    def _set_marker_color(self, name, color):
        """
        Set the marker colour *name*. matplotlib keeps what is no colour
        (``3``) as it is, failing to convert it when it draws the markers:
        the line keeps it (`._unusable`) while pltrs (Rust) keeps the colour
        it had.
        """
        if (color is None or cbook._str_lower_equal(color, "auto")
                or mcolors.is_color_like(color)):
            _unusable.drop(self, name)
            self._set_native(**{name: color})
            return
        _values.keep(self, name, color)
        _unusable.keep(self, name, color,
                       functools.partial(self._draw_marker_color, name),
                       measured=False)
        self.stale = True

    def _draw_marker_color(self, name, color):
        """Fail as matplotlib's draw converts the marker colour *name*,
        *color*: where the line draws markers (filled ones, for the face
        colours)."""
        if (_draws_marker(self.get_marker()) and self.get_markersize() > 0
                and (name == "markeredgecolor" or self.get_fillstyle() != "none")):
            mcolors.to_rgba(color, self.get_alpha())

    def _kept_order(self):
        """The order in which matplotlib's draw computes with the values
        the line keeps (see `._unusable`)."""
        return ["data", "dashes", "markevery", "markeredgecolor",
                "markerfacecolor", "markerfacecoloralt"]

    def _given_color(self, name, default=_values.NONE_GIVEN):
        """The colour property *name* as the program gave it (`._values`)."""
        return _values.given(self, name, self._get(name), default,
                             same=_values.same_color)

    def _marker_color(self, name, setting):
        """A marker colour: the line's colour where it is ``"auto"``."""
        if _values.is_kept(self, name):
            return self._given_color(name)
        if (cbook._str_lower_equal(self._get(setting), "auto")
                and _values.same_color(self._get(name), self._get("color"))):
            return self.get_color()
        return self._given_color(name)

    # -- data -------------------------------------------------------------

    def get_xdata(self, orig=True):
        """
        Return the xdata.

        If *orig* is *True*, return the original data, else the
        processed data.
        """
        if orig and "_xorig" in self.__dict__:
            return self._xorig
        return _lines.get_xdata(self._native)

    def get_ydata(self, orig=True):
        """
        Return the ydata.

        If *orig* is *True*, return the original data, else the
        processed data.
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
        _unusable.check(self, measuring=True)
        x, y = np.broadcast_arrays(self.get_xdata(orig=False),
                                   self.get_ydata(orig=False))
        return np.column_stack([x, y]).astype(float)

    # The numbers drawn, as matplotlib's ``recache`` keeps them (attributes
    # programs read: scikit-learn's tests read ``line._y``).
    _xy = property(get_xydata, doc="The *xy* data drawn, as a (N, 2) array.")
    _x = property(lambda self: self.get_xydata()[:, 0], doc="The x data drawn.")
    _y = property(lambda self: self.get_xydata()[:, 1], doc="The y data drawn.")

    def get_path(self):
        """Return the `~matplotlib.path.Path` associated with this line."""
        _unusable.check(self, measuring=True)
        return drawn_path(self, *_lines.get_path(self._native), (
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
        self._xorig, self._yorig = _copied(x), _copied(y)
        # Converted now, the numbers reach pltrs (Rust) when the next native
        # call is made. Data that converts to no numbers (strings on a
        # numeric axis) matplotlib keeps, failing to convert it as it
        # draws or measures the line, while pltrs (Rust) keeps the numbers
        # it had.
        try:
            numbers = self._numbers()
        except (TypeError, ValueError):
            _unusable.keep(self, "data", None, lambda _: self._numbers())
            self.stale = True
            return
        _unusable.drop(self, "data")
        _deferred_data[self._native] = numbers

    def _convert_units_for(self, axes):
        if self.__dict__.pop("_unit_data", False):
            _lines.set_data(self._native, _floats(axes.convert_xunits(self._xorig)),
                            _floats(axes.convert_yunits(self._yorig)))

    def recache_always(self):
        # Convert the original data through the units of the line's axes
        # again, and hand the numbers to pltrs (Rust).
        self.recache(always=True)

    def recache(self, always=False):
        # Convert the original data through the units of the line's axes again
        # when *always* is true, and hand the numbers to pltrs (Rust).
        #
        # pltrs (Rust) builds the line's path from its data whenever it is
        # drawn, and every setter of the data converts the numbers at once
        # (handing them over by the next native call), so without *always*
        # nothing is waiting to be converted.
        if not always or "_xorig" not in self.__dict__:
            return
        _lines.set_data(self._native, *self._numbers())

    def _numbers(self):
        """The original data converted through the units of the line's
        axes: the numbers pltrs (Rust) draws."""
        return (_floats(self.convert_xunits(self._xorig)),
                _floats(self.convert_yunits(self._yorig)))

    def set_xdata(self, x):
        """
        Set the data array for x.

        Parameters
        ----------
        x : 1D array

        See Also
        --------
        set_data
        set_ydata
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

        See Also
        --------
        set_data
        set_xdata
        """
        if not np.iterable(y):
            raise RuntimeError('y must be a sequence')
        self.set_data(self.get_xdata(orig=True), y)

    # -- style ------------------------------------------------------------

    def get_antialiased(self):
        """Return whether antialiased rendering is used."""
        return self._get("antialiased")

    def set_antialiased(self, b):
        """
        Set whether to use antialiased rendering.

        Parameters
        ----------
        b : bool
        """
        self._set_native(antialiased=bool(b))

    def get_color(self):
        """
        Return the line color.

        See also `~.Line2D.set_color`.
        """
        native = self._get("color")
        # A colour pltrs (Rust) took from the property cycle reads back as
        # the cycle holds it (matplotlib's tab10 holds triples).
        default = (_values.NONE_GIVEN if isinstance(native, str)
                   else _values.cycle_color(self.axes, native))
        return self._given_color("color", default)

    @property
    def _color(self):
        """The line color (matplotlib's attribute, which programs read
        directly)."""
        return self.get_color()

    def set_color(self, color):
        """
        Set the color of the line.

        Parameters
        ----------
        color : :mpltype:`color`
        """
        from . import colors as mcolors
        mcolors._check_color_like(color=color)
        self._set_native(color=color)

    def get_drawstyle(self):
        """
        Return the drawstyle.

        See also `~.Line2D.set_drawstyle`.
        """
        return self._get("drawstyle")

    def set_drawstyle(self, drawstyle):
        """
        Set the drawstyle of the plot.

        The drawstyle determines how the points are connected.

        Parameters
        ----------
        drawstyle : {'default', 'steps', 'steps-pre', 'steps-mid', \
'steps-post'}, default: 'default'
            For 'default', the points are connected with straight lines.

            The steps variants connect the points with step-like lines,
            i.e. horizontal lines with vertical steps. They differ in the
            location of the step:

            - 'steps-pre': The step is at the beginning of the line segment,
              i.e. the line will be at the y-value of point to the right.
            - 'steps-mid': The step is halfway between the points.
            - 'steps-post: The step is at the end of the line segment,
              i.e. the line will be at the y-value of the point to the left.
            - 'steps' is equal to 'steps-pre' and is maintained for
              backward-compatibility.

            For examples see :doc:`/gallery/lines_bars_and_markers/step_demo`.
        """
        if drawstyle is None:
            drawstyle = 'default'
        _api.check_in_list(self.drawStyles, drawstyle=drawstyle)
        self._set_native(drawstyle=drawstyle)

    def get_gapcolor(self):
        """
        Return the line gapcolor.

        See also `~.Line2D.set_gapcolor`.
        """
        return self._given_color("gapcolor")

    def set_gapcolor(self, gapcolor):
        """
        Set a color to fill the gaps in the dashed line style.

        .. note::

            Striped lines are created by drawing two interleaved dashed lines.
            There can be overlaps between those two, which may result in
            artifacts when using transparency.

            This functionality is experimental and may change.

        Parameters
        ----------
        gapcolor : :mpltype:`color` or None
            The color with which to fill the gaps. If None, the gaps are
            unfilled.
        """
        self._set_native(gapcolor=gapcolor)

    def get_linestyle(self):
        """
        Return the linestyle.

        See also `~.Line2D.set_linestyle`.
        """
        if _unusable.kept(self, "dashes", None) is not None:
            return '--'  # A custom dash pattern, kept (`_keeps_dashes`).
        return self._get("linestyle")

    def set_linestyle(self, ls):
        """
        Set the linestyle of the line.

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

              See also :meth:`set_dashes`.

            For examples see :doc:`/gallery/lines_bars_and_markers/linestyles`.
        """
        if isinstance(ls, str):
            if ls in [' ', '', 'none']:
                ls = 'None'
            _api.check_in_list([*self._lineStyles, *cbook.ls_mapper_r], ls=ls)
        else:
            # matplotlib makes the dash pattern of any other style.
            if self._keeps_dashes(_get_dash_pattern(ls)[1]):
                return
        _unusable.drop(self, "dashes")
        self._set_native(linestyle=ls)

    def _keeps_dashes(self, dashes):
        """Keep the dash list *dashes* if pltrs (Rust) cannot hold it (no
        positive value, a negative or non-finite one): matplotlib keeps it,
        and its graphics context refuses the negative and the all-zero
        ones as it draws the line (`_draw_dashes`). Whether it is kept."""
        if dashes is None or not len(dashes):
            return False
        values = np.asarray(dashes, dtype=float)
        if np.isfinite(values).all() and (values >= 0).all() and (values > 0).any():
            return False
        _unusable.keep(self, "dashes", values, self._draw_dashes, measured=False)
        self.stale = True
        return True

    def _draw_dashes(self, dashes):
        """Fail as matplotlib's graphics context takes the *dashes* the
        line is drawn with, where the line draws its line."""
        if len(self.get_xydata()) and self.get_linestyle() not in ('None', ' ', ''):
            if np.any(dashes < 0.0):
                raise ValueError("All values in the dash list must be non-negative")
            if dashes.size and not np.any(dashes > 0.0):
                raise ValueError("At least one value in the dash list must be positive")

    @property
    def _dash_pattern(self):
        """The ``(offset, dashes)`` the line is drawn with, scaled by its
        width as :rc:`lines.scale_dashes` says."""
        offset, dashes = _get_dash_pattern(self._get("dash_pattern"))
        return _scale_dashes(offset, None if dashes is None else tuple(dashes),
                             self.get_linewidth())

    def set_dashes(self, seq):
        """
        Set the dash sequence.

        The dash sequence is a sequence of floats of even length describing
        the length of dashes and spaces in points.

        For example, (5, 2, 1, 2) describes a sequence of 5 point and 1 point
        dashes separated by 2 point spaces.

        See also `~.Line2D.set_gapcolor`, which allows those spaces to be
        filled with a color.

        Parameters
        ----------
        seq : sequence of floats (on/off ink in points) or (None, None)
            If *seq* is empty or ``(None, None)``, the linestyle will be set
            to solid.
        """
        if seq == (None, None) or len(seq) == 0:
            self.set_linestyle('-')
        else:
            self.set_linestyle((0, seq))

    def is_dashed(self):
        """
        Return whether line has a dashed linestyle.

        A custom linestyle is assumed to be dashed, we do not inspect the
        ``onoffseq`` directly.

        See also `~.Line2D.set_linestyle`.
        """
        if _unusable.kept(self, "dashes", None) is not None:
            return True
        return _lines.is_dashed(self._native)

    def get_linewidth(self):
        """
        Return the linewidth in points.

        See also `~.Line2D.set_linewidth`.
        """
        return self._get("linewidth")

    def set_linewidth(self, w):
        """
        Set the line width in points.

        Parameters
        ----------
        w : float
            Line width, in points.
        """
        self._set_native(linewidth=float(w))

    def get_marker(self):
        """
        Return the line marker.

        See also `~.Line2D.set_marker`.
        """
        marker = self._get("marker")
        if isinstance(marker, tuple) and marker[:1] == ("path",):
            return Path(*marker[1])
        return marker

    @_docstring.interpd
    def set_marker(self, marker):
        """
        Set the line marker.

        Parameters
        ----------
        marker : marker style string, `~.path.Path` or `~.markers.MarkerStyle`
            See `~matplotlib.markers` for full description of possible
            arguments.
        """
        self._set_native(marker=marker, fillstyle=self.get_fillstyle())

    def get_fillstyle(self):
        """
        Return the marker fill style.

        See also `~.Line2D.set_fillstyle`.
        """
        return self._get("fillstyle")

    def set_fillstyle(self, fs):
        """
        Set the marker fill style.

        Parameters
        ----------
        fs : {'full', 'left', 'right', 'bottom', 'top', 'none'}
            Possible values:

            - 'full': Fill the whole marker with the *markerfacecolor*.
            - 'left', 'right', 'bottom', 'top': Fill the marker half at
              the given side with the *markerfacecolor*. The other
              half of the marker is filled with *markerfacecoloralt*.
            - 'none': No filling.

            For examples see :ref:`marker_fill_styles`.
        """
        # matplotlib's `MarkerStyle` checks it, :rc:`markers.fillstyle` for
        # None.
        if fs is None:
            fs = _rcParams()['markers.fillstyle']
        _api.check_in_list(MarkerStyle.fillstyles, fillstyle=fs)
        self._set_native(fillstyle=fs)

    def get_markersize(self):
        """
        Return the marker size in points.

        See also `~.Line2D.set_markersize`.
        """
        return self._get("markersize")

    def set_markersize(self, sz):
        """
        Set the marker size in points.

        Parameters
        ----------
        sz : float
             Marker size, in points.
        """
        self._set_native(markersize=float(sz))

    def get_markeredgewidth(self):
        """
        Return the marker edge width in points.

        See also `~.Line2D.set_markeredgewidth`.
        """
        return _values.given(self, "markeredgewidth", self._get("markeredgewidth"),
                             same=_same_number)

    def set_markeredgewidth(self, ew):
        """
        Set the marker edge width in points.

        Parameters
        ----------
        ew : float
             Marker edge width, in points.
        """
        if ew is None:
            ew = _rcParams()['lines.markeredgewidth']
        self._set_native(markeredgewidth=float(ew))
        _values.remember(self, "markeredgewidth", ew)

    def get_markeredgecolor(self):
        """
        Return the marker edge color.

        See also `~.Line2D.set_markeredgecolor`.
        """
        return self._marker_color("markeredgecolor", "markeredgecolor_setting")

    def set_markeredgecolor(self, ec):
        """
        Set the marker edge color.

        Parameters
        ----------
        ec : :mpltype:`color`
        """
        self._set_marker_color("markeredgecolor", ec)

    def get_markerfacecolor(self):
        """
        Return the marker face color.

        See also `~.Line2D.set_markerfacecolor`.
        """
        if self.get_fillstyle() == 'none':
            return 'none'
        return self._marker_color("markerfacecolor", "markerfacecolor_setting")

    def set_markerfacecolor(self, fc):
        """
        Set the marker face color.

        Parameters
        ----------
        fc : :mpltype:`color`
        """
        self._set_marker_color("markerfacecolor", fc)

    def get_markerfacecoloralt(self):
        """
        Return the alternate marker face color.

        See also `~.Line2D.set_markerfacecoloralt`.
        """
        if self.get_fillstyle() == 'none':
            return 'none'
        return self._marker_color("markerfacecoloralt", "markerfacecoloralt")

    def set_markerfacecoloralt(self, fc):
        """
        Set the alternate marker face color.

        Parameters
        ----------
        fc : :mpltype:`color`
        """
        self._set_marker_color("markerfacecoloralt", fc)

    def get_markevery(self):
        """
        Return the markevery setting for marker subsampling.

        See also `~.Line2D.set_markevery`.
        """
        if "_markevery" in self.__dict__:
            return self._markevery
        return self._get("markevery")

    def set_markevery(self, every):
        """
        Set the markevery property to subsample the plot when using markers.

        e.g., if ``every=5``, every 5-th marker will be plotted.

        Parameters
        ----------
        every : None or int or (int, int) or slice or list[int] or float or \
(float, float) or list[bool]
            Which markers to plot.

            - ``every=None``: every point will be plotted.
            - ``every=N``: every N-th marker will be plotted starting with
              marker 0.
            - ``every=(start, N)``: every N-th marker, starting at index
              *start*, will be plotted.
            - ``every=slice(start, end, N)``: every N-th marker, starting at
              index *start*, up to but not including index *end*, will be
              plotted.
            - ``every=[i, j, m, ...]``: only markers at the given indices
              will be plotted.
            - ``every=[True, False, True, ...]``: only positions that are True
              will be plotted. The list must have the same length as the data
              points.
            - ``every=0.1``, (i.e. a float): markers will be spaced at
              approximately equal visual distances along the line; the distance
              along the line between markers is determined by multiplying the
              display-coordinate distance of the Axes bounding-box diagonal
              by the value of *every*.
            - ``every=(0.5, 0.1)`` (i.e. a length-2 tuple of float): similar
              to ``every=0.1`` but the first marker will be offset along the
              line by 0.5 multiplied by the
              display-coordinate-diagonal-distance along the line.

            For examples see
            :doc:`/gallery/lines_bars_and_markers/markevery_demo`.

        Notes
        -----
        Setting *markevery* will still only draw markers at actual data points.
        While the float argument form aims for uniform visual spacing, it has
        to coerce from the ideal spacing to the nearest available data point.
        Depending on the number and distribution of data points, the result
        may still not look evenly spaced.

        When using a start offset to specify the first marker, the offset will
        be from the first data point which may be different from the first
        the visible data point if the plot is zoomed in.

        If zooming in on a plot when using float arguments then the actual
        data points that have markers will change because the distance between
        markers is always determined from the display-coordinates
        axes-bounding-box-diagonal regardless of the actual axes data limits.

        """
        try:
            self._set_native(markevery=every)
        except ValueError:
            # matplotlib keeps what it cannot sample markers with, failing
            # on it as it draws the markers (`_mark_every_path`).
            _unusable.keep(self, "markevery", every, self._draw_markevery,
                           measured=False)
        else:
            _unusable.drop(self, "markevery")
        self._markevery = every
        self.stale = True

    def _draw_markevery(self, every):
        """Fail as matplotlib's draw samples the markers with *every*: where
        the line draws markers."""
        if _draws_marker(self.get_marker()) and self.get_markersize() > 0:
            _mark_every_path(every, self.get_path(), self.get_transform(), self.axes)

    def get_pickradius(self):
        """
        Return the pick radius used for containment tests.

        See `.contains` for more details.
        """
        return _values.given(self, "pickradius", self._get("pickradius"), 5)

    def set_pickradius(self, pickradius):
        """
        Set the pick radius used for containment tests.

        See `.contains` for more details.

        Parameters
        ----------
        pickradius : float
            Pick radius, in points.
        """
        if not isinstance(pickradius, Real) or pickradius < 0:
            raise ValueError("pick radius should be a distance")
        self._set_native(pickradius=float(pickradius))
        _values.remember(self, "pickradius", pickradius)

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

    def contains(self, mouseevent):
        """
        Test whether *mouseevent* occurred on the line.

        An event is deemed to have occurred "on" the line if it is less
        than ``self.pickradius`` (default: 5 points) away from it.  Use
        `~.Line2D.get_pickradius` or `~.Line2D.set_pickradius` to get or set
        the pick radius.

        Parameters
        ----------
        mouseevent : `~matplotlib.backend_bases.MouseEvent`

        Returns
        -------
        contains : bool
            Whether any values are within the radius.
        details : dict
            A dictionary ``{'ind': pointlist}``, where *pointlist* is a
            list of points of the line that are within the pickradius around
            the event position.

            TODO: sort returned indices by distance
        """
        return super().contains(mouseevent)

    def _contains_details(self, ind):
        """The indices of the points and segments hit."""
        return dict(ind=ind)

    def _contains_within(self, mouseevent, radius):
        """`contains` as matplotlib's line answers it: no details for a
        line of another canvas or without points, and for a line in no
        figure a warning, the line then hit-tested in the coordinates its
        transform gives, its pick radius as pixels."""
        if self._different_canvas(mouseevent) or not len(self.get_xydata()):
            return False, {}
        if self.get_figure(root=True) is None:
            _log.warning('no figure set when check if mouse is on line')
        ind = _artist.contains(self._native, float(mouseevent.x),
                               float(mouseevent.y), radius)
        ind = np.asarray([] if ind is None else ind, dtype=int)
        return len(ind) > 0, self._contains_details(ind)

    def get_dash_capstyle(self):
        """
        Return the `.CapStyle` for dashed lines.

        See also `~.Line2D.set_dash_capstyle`.
        """
        return self._get("dash_capstyle")

    @_docstring.interpd
    def set_dash_capstyle(self, s):
        """
        How to draw the end caps if the line is `~Line2D.is_dashed`.

        The default capstyle is :rc:`lines.dash_capstyle`.

        Parameters
        ----------
        s : `.CapStyle` or %(CapStyle)s
        """
        self._set_native(dash_capstyle=_capstyle_name(s))

    def get_solid_capstyle(self):
        """
        Return the `.CapStyle` for solid lines.

        See also `~.Line2D.set_solid_capstyle`.
        """
        return self._get("solid_capstyle")

    @_docstring.interpd
    def set_solid_capstyle(self, s):
        """
        How to draw the end caps if the line is solid (not `~Line2D.is_dashed`)

        The default capstyle is :rc:`lines.solid_capstyle`.

        Parameters
        ----------
        s : `.CapStyle` or %(CapStyle)s
        """
        self._set_native(solid_capstyle=_capstyle_name(s))

    def get_dash_joinstyle(self):
        """
        Return the `.JoinStyle` for dashed lines.

        See also `~.Line2D.set_dash_joinstyle`.
        """
        return self._get("dash_joinstyle")

    @_docstring.interpd
    def set_dash_joinstyle(self, s):
        """
        How to join segments of the line if it `~Line2D.is_dashed`.

        The default joinstyle is :rc:`lines.dash_joinstyle`.

        Parameters
        ----------
        s : `.JoinStyle` or %(JoinStyle)s
        """
        self._set_native(dash_joinstyle=_joinstyle_name(s))

    def get_solid_joinstyle(self):
        """
        Return the `.JoinStyle` for solid lines.

        See also `~.Line2D.set_solid_joinstyle`.
        """
        return self._get("solid_joinstyle")

    @_docstring.interpd
    def set_solid_joinstyle(self, s):
        """
        How to join segments if the line is solid (not `~Line2D.is_dashed`).

        The default joinstyle is :rc:`lines.solid_joinstyle`.

        Parameters
        ----------
        s : `.JoinStyle` or %(JoinStyle)s
        """
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

    def get_window_extent(self, renderer=None):
        from .transforms import Bbox
        bbox = Bbox([[0, 0], [0, 0]])
        trans_data_to_xy = self.get_transform().transform
        bbox.update_from_data_xy(trans_data_to_xy(self.get_xydata()),
                                 ignore=True)
        # correct for marker size, if any
        if _draws_marker(self.get_marker()):
            ms = (self.get_markersize() / 72.0 * self.get_figure(root=True).dpi) * 0.5
            bbox = bbox.padded(ms)
        return bbox

    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)


def _floats(values):
    """``cbook._to_unmasked_float_array(values).ravel()``, which is
    *values* itself for a one-dimensional array of floats."""
    if type(values) is np.ndarray and values.ndim == 1 and values.dtype.char == "d":
        return values
    return cbook._to_unmasked_float_array(values).ravel()


def _push_deferred_data(waiting):
    """Hand pltrs (Rust) the numbers `Line2D.set_data` left for the next
    native call: the binding calls this with them, taken out of
    `_deferred_data`, before that call runs."""
    for native, (x, y) in waiting.items():
        _lines.set_data(native, x, y)


#: The numbers of each line whose data was set since the last native
#: call, by its native line (`_push_deferred_data` hands them over).
_deferred_data = _lines.defer_data({}, _push_deferred_data)


def _copied(values):
    """``copy.copy(values)``, sparing its type dispatch for numpy arrays."""
    return values.__copy__() if type(values) is np.ndarray else copy.copy(values)


def _draws_marker(marker):
    """``bool(MarkerStyle(marker))``, whether *marker* has a path: known
    without making the marker for the codes of `.MarkerStyle.markers`."""
    if type(marker) in (str, int) and marker in MarkerStyle.markers:
        return MarkerStyle.markers[marker] != 'nothing'
    return bool(MarkerStyle(marker))


def _capstyle_name(s):
    """The name of the cap style *s* (a `.CapStyle` or its name)."""
    from ._enums import CapStyle
    return CapStyle(s).name


def _joinstyle_name(s):
    """The name of the join style *s* (a `.JoinStyle` or its name)."""
    from ._enums import JoinStyle
    return JoinStyle(s).name


def _axline_geometry(xy1, xy2, slope):
    """
    The points and slope of an `AxLine` as pltrs (Rust) takes them, or
    None for those it cannot hold: points that are no pairs of numbers, a
    slope that is no number (matplotlib keeps them, and fails where it
    computes with them).
    """
    point1 = _axline_point(xy1)
    point2 = None if xy2 is None else _axline_point(xy2)
    if (point1 is None or (xy2 is not None and point2 is None)
            or not (slope is None or isinstance(slope, Real))):
        return None
    return point1, point2, None if slope is None else float(slope)


def _axline_point(xy):
    """The point *xy* of an `AxLine` as pltrs (Rust) takes it, or None for
    one that is no pair of numbers."""
    values = np.asarray(xy)
    if values.shape != (2,) or values.dtype.kind not in "biuf":
        return None
    return tuple(map(float, values))


class AxLine(Line2D):
    """
    A helper class that implements `~.Axes.axline`, by recomputing the artist
    transform at draw time.
    """
    # pltrs: pltrs (Rust) places the line in the view each time it is laid
    # out; `get_transform` is matplotlib's, the diagonal of the unit square
    # onto the part of the line the view limits show.

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    def __init__(self, xy1, xy2, slope, **kwargs):
        """
        Parameters
        ----------
        xy1 : (float, float)
            The first set of (x, y) coordinates for the line to pass through.
        xy2 : (float, float) or None
            The second set of (x, y) coordinates for the line to pass through.
            Both *xy2* and *slope* must be passed, but one of them must be None.
        slope : float or None
            The slope of the line. Both *xy2* and *slope* must be passed, but one of
            them must be None.
        """
        super().__init__([0, 1], [0, 1], **kwargs)
        if (xy2 is None and slope is None or
                xy2 is not None and slope is not None):
            raise TypeError(
                "Exactly one of 'xy2' and 'slope' must be given")
        geometry = _axline_geometry(xy1, xy2, slope)
        if geometry is None:
            # matplotlib keeps them and fails where its draw places the
            # line (`get_transform`): pltrs holds a placeholder and draws
            # the line through Python.
            from . import _python_drawing
            geometry = ((0.0, 0.0), None if xy2 is None else (1.0, 1.0),
                        None if slope is None else 0.0)
            _python_drawing.draw_through_python(self, False)
        _lines.make_axline(self._native, *geometry)
        self._remember_geometry(xy1, xy2, slope)
        if geometry != _axline_geometry(xy1, xy2, slope):
            for name, value in (("xy1", xy1), ("xy2", xy2), ("slope", slope)):
                _values.keep(self, name, value)

    def draw(self, renderer):
        # (pltrs (Rust) places the line in the view as it draws it, where
        # matplotlib's draw reads the transform first.)
        self.get_transform()
        super().draw(renderer)

    def get_transform(self):
        from .transforms import Bbox, BboxTransformTo
        ax = self.axes
        points_transform = (Artist.get_transform(self) - ax.transData
                            + ax.transScale)
        xy1, xy2, slope = self.get_xy1(), self.get_xy2(), self.get_slope()
        if xy2 is not None:
            # two points were given
            (x1, y1), (x2, y2) = points_transform.transform([xy1, xy2])
            dx = x2 - x1
            dy = y2 - y1
            if dx == 0:
                if dy == 0:
                    raise ValueError(
                        f"Cannot draw a line through two identical points "
                        f"(x={(x1, x2)}, y={(y1, y2)})")
                slope = np.inf
            else:
                slope = dy / dx
        else:
            # one point and a slope were given
            x1, y1 = points_transform.transform(xy1)
        (vxlo, vylo), (vxhi, vyhi) = ax.transScale.transform(ax.viewLim)
        # General case: find intersections with view limits in either
        # direction, and draw between the middle two points.
        if slope == 0:
            start = vxlo, y1
            stop = vxhi, y1
        elif np.isinf(slope):
            start = x1, vylo
            stop = x1, vyhi
        else:
            _, start, stop, _ = sorted([
                (vxlo, y1 + (vxlo - x1) * slope),
                (vxhi, y1 + (vxhi - x1) * slope),
                (x1 + (vylo - y1) / slope, vylo),
                (x1 + (vyhi - y1) / slope, vyhi),
            ])
        return (BboxTransformTo(Bbox([start, stop]))
                + ax.transLimits + ax.transAxes)

    def _remember_geometry(self, xy1, xy2, slope):
        """Keep the line's geometry as given (matplotlib keeps it), and its
        data, which matplotlib's `AxLine` is made with as
        ``Line2D([0, 1], [0, 1])``."""
        _values.remember_all(self, {"xy1": xy1, "xy2": xy2, "slope": slope},
                             ("xy1", "xy2", "slope"))
        self._xorig, self._yorig = [0, 1], [0, 1]

    def get_xy1(self):
        """Return the *xy1* value of the line."""
        return _values.given(self, "xy1", _lines.axline(self._native)[0])

    def get_xy2(self):
        """Return the *xy2* value of the line."""
        return _values.given(self, "xy2", _lines.axline(self._native)[1])

    def get_slope(self):
        """Return the *slope* value of the line."""
        return _values.given(self, "slope", _lines.axline(self._native)[2])

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
        self._hold_geometry("xy1", xy1, _axline_point(xy1))

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
        self._hold_geometry("xy2", xy2, _axline_point(xy2))

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
        if self.get_xy2() is not None:
            raise ValueError("Cannot set a 'slope' value while 'xy2' is set;"
                             " they differ but their functionalities overlap")
        self._hold_geometry("slope", slope,
                            float(slope) if isinstance(slope, Real) else None)

    def _hold_geometry(self, name, value, held):
        """
        Make *value* the line's *name* (``xy1``, ``xy2`` or ``slope``):
        handed to pltrs (Rust) as *held*, its numbers, else, None, kept as
        given (matplotlib keeps it unchecked), the line then drawn through
        Python, where matplotlib's draw fails on it (`get_transform`).
        """
        if held is None:
            from . import _python_drawing
            _values.keep(self, name, value)
            _python_drawing.draw_through_python(self, False)
        else:
            getattr(_lines, f"set_axline_{name}")(self._native, held)
            _values.remember(self, name, value)
        self.stale = True


class VertexSelector:
    """
    Manage the callbacks to maintain a list of selected vertices for `.Line2D`.
    Derived classes should override the `process_selected` method to do
    something with the picks.

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


# What matplotlib's module also has from its imports, which programs
# import from it.
from numbers import Integral, Number, Real  # noqa: E402, F401
from . import colors as mcolors  # noqa: E402, F401
from ._enums import CapStyle, JoinStyle  # noqa: E402, F401
from .artist import allow_rasterization  # noqa: E402, F401
from .transforms import Bbox, BboxTransformTo, TransformedPath  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
