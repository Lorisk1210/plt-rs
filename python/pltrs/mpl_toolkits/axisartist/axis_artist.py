"""
The :mod:`.axis_artist` module implements custom artists to draw axis lines
(matplotlib's ``mpl_toolkits.axisartist.axis_artist``, from matplotlib
3.11.2, under the Matplotlib License).

An `AxisArtist` is a pltrs (Rust) axis artist of its axes, drawing its line,
ticks, tick labels and label: ``axis.major_ticks``, ``axis.major_ticklabels``,
``axis.label`` and ``axis.line`` act on those parts of it
(``_native.axisartist``). `Ticks` made on their own are tick marks an axes
draws once ``ax.add_artist(ticks)`` adds them, and texts made on their own
(`LabelBase`, `AxisLabel`, `TickLabels`) likewise (`._own_texts`).
"""

from operator import methodcaller

from pltrs import _api, _handles
import pltrs.artist as martist
from pltrs.collections import LineCollection
from pltrs.transforms import Bbox
from pltrs._native import artist as _artist, axisartist as _axisartist
from ._own_texts import OwnTexts


class _Part:
    """A part of an axis artist (its ticks, its tick labels, its label or
    its line), acting on the pltrs (Rust) axis artist."""

    def __init__(self, axis_artist, name):
        self._axis_artist = axis_artist
        self._name = name

    @property
    def _native(self):
        return self._axis_artist._native


class AttributeCopier:
    def get_ref_artist(self):
        """
        Return the underlying artist that actually defines some properties
        (e.g., color) of this artist.
        """
        raise RuntimeError("get_ref_artist must overridden")

    def get_attribute_from_ref_artist(self, attr_name):
        getter = methodcaller("get_" + attr_name)
        prop = getter(super())
        return getter(self.get_ref_artist()) if prop == "auto" else prop


def _auto(value):
    """A colour or width as matplotlib's `AttributeCopier` sees it:
    ``"auto"`` for one that follows the reference artist (None)."""
    return "auto" if value is None else value


class _TickStyle:
    """The colour and width the ticks have of their own, ``"auto"`` when
    they follow the ticks of their axis (what matplotlib's `.Line2D`
    getters give `Ticks`)."""

    def _own_style(self):
        if self._part is None:
            return self._color, self._markeredgewidth
        return _axisartist.get_tick_style(self._part._native, self._part._name)

    def get_color(self):
        return _auto(self._own_style()[0])

    def get_markeredgecolor(self):
        return _auto(self._own_style()[0])

    def get_markeredgewidth(self):
        return _auto(self._own_style()[1])


class Ticks(AttributeCopier, _TickStyle, martist.Artist):
    """
    Ticks are derived from `.Line2D`, and note that ticks themselves
    are markers. Thus, you should use set_mec, set_mew, etc.

    To change the tick size (length), you need to use
    `set_ticksize`. To change the direction of the ticks (ticks are
    in opposite direction of ticklabels by default), use
    ``set_tick_out(False)``

    Ticks made by their constructor are drawn once an axes adds them
    (``ax.add_artist(ticks)``); the ticks of an `AxisArtist`
    (``axis.major_ticks``) are those it draws.
    """

    locs_angles_labels = _api.deprecated("3.11")(property(lambda self: []))

    def __init__(self, ticksize, tick_out=False, *, axis=None, **kwargs):
        super().__init__()
        self._part = None
        self._ticksize = ticksize
        self._locs_angles = []
        self._color = None
        self._markeredgewidth = None
        self._axis = axis
        self.set_tick_out(tick_out)
        self._internal_update(kwargs)

    @classmethod
    def _of(cls, axis_artist, level):
        """The *level* (``"major"`` or ``"minor"``) ticks of *axis_artist*."""
        ticks = cls.__new__(cls)
        ticks._part = _Part(axis_artist, level)
        ticks._axis = axis_artist.axis
        return ticks

    def get_ref_artist(self):
        # docstring inherited
        return self._axis.majorTicks[0].tick1line

    def get_color(self):
        return self.get_attribute_from_ref_artist("color")

    def get_markeredgecolor(self):
        return self.get_attribute_from_ref_artist("markeredgecolor")

    def get_markeredgewidth(self):
        return self.get_attribute_from_ref_artist("markeredgewidth")

    def _set_native(self, key, value):
        _axisartist.set_ticks(self._part._native, self._part._name, key, value)

    def _get_native(self):
        return _axisartist.get_ticks(self._part._native, self._part._name)

    def set_tick_out(self, b):
        """Set whether ticks are drawn inside or outside the axes."""
        self.set_tick_direction("out" if b else "in")

    def get_tick_out(self):
        """Return whether ticks are drawn inside or outside the axes."""
        return self.get_tick_direction() == "out"

    def set_tick_direction(self, direction):
        """Set the direction of the ticks: ``"in"``, ``"out"`` or
        ``"inout"``."""
        _api.check_in_list(["in", "out", "inout"], direction=direction)
        if self._part is None:
            self._tick_direction = direction
            self._push()
        else:
            self._set_native("tick_direction", direction)

    def get_tick_direction(self):
        """Return the direction of the ticks."""
        if self._part is None:
            return self._tick_direction
        return self._get_native()[1]

    def set_ticksize(self, ticksize):
        """Set length of the ticks in points."""
        if self._part is None:
            self._ticksize = ticksize
            self._push()
        else:
            self._set_native("ticksize", float(ticksize))

    def get_ticksize(self):
        """Return length of the ticks in points."""
        if self._part is None:
            return self._ticksize
        return self._get_native()[0]

    def set_color(self, color):
        """Set the color of the ticks."""
        color = None if color == "auto" else color
        if self._part is None:
            self._color = color
            self._push()
        else:
            self._set_native("color", color)

    def set_markeredgewidth(self, width):
        """Set the width of the ticks in points."""
        width = None if width == "auto" else width
        if self._part is None:
            self._markeredgewidth = width
            self._push()
        else:
            self._set_native("markeredgewidth", width)

    set_mew = set_markeredgewidth

    def set_visible(self, b):
        # docstring inherited
        if self._part is None:
            super().set_visible(b)
        else:
            self._set_native("visible", bool(b))

    def get_visible(self):
        # docstring inherited
        if self._part is None:
            return super().get_visible()
        return self._get_native()[2]

    def set_alpha(self, alpha):
        # docstring inherited
        if self._part is None:
            super().set_alpha(alpha)
        else:
            self._set_native("alpha", alpha)

    def set_locs_angles(self, locs_angles):
        """Set the locations (in data coordinates) and angles (in degrees)
        of the ticks."""
        self._locs_angles = locs_angles
        self._push()

    @property
    def locs_angles(self):
        """The locations and angles (in degrees) of the ticks: those set,
        or for the ticks of an `AxisArtist`, those it draws now."""
        if self._part is None:
            return self._locs_angles
        return [[loc, angle] for loc, angle in _axisartist.get_tick_locs_angles(
            self._part._native, self._part._name)]

    def _marks(self):
        """The native description of ticks made on their own: their axis,
        locations and angles, and style."""
        return ("x" if self._axis is None else self._axis.axis_name,
                [((float(x), float(y)), float(angle))
                 for (x, y), angle in self._locs_angles],
                (float(self._ticksize), self._tick_direction, self._color,
                 self._markeredgewidth))

    def _push(self):
        """Hand pltrs (Rust) ticks made on their own again, once an axes
        added them."""
        if self._figure is not None:
            _axisartist.set_tick_marks(self._native, *self._marks())

    def _add_to_axes(self, axes, kind, autolim=True):
        if self._part is not None:
            raise ValueError("the ticks of an axis artist are drawn by it")
        native = _axisartist.add_tick_marks(axes._native, *self._marks())
        return _handles.adopt(self, native, axes.figure)


class LabelBase(martist.Artist):
    """
    A base class for `.AxisLabel` and `.TickLabels`: the texts of an
    axis artist, placed at a distance from their reference point.

    The texts of an `AxisArtist` (``axis.label``, ``axis.major_ticklabels``)
    are those it draws. Texts made on their own are drawn once an axes adds
    them (``ax.add_artist(label)``): a `LabelBase` ``_offset_radius``
    pixels from its point towards ``_ref_angle`` degrees, turned by that
    direction.
    """

    locs_angles_labels = _api.deprecated("3.11")(property(lambda self: []))

    #: The kind of texts made on their own (see `._own_texts.OwnTexts`).
    _KIND = "label"
    #: Where the native getter of the texts of an axis artist puts each
    #: property.
    _GETTERS = {"axis_direction": 0, "pad": 1, "visible": 2, "rotation": 3,
                "size": 4, "text": 5}

    def __init__(self, *args, **kwargs):
        super().__init__()
        self._part = None
        self._own = OwnTexts(self._KIND, *args)
        self._internal_update(kwargs)

    def _add_to_axes(self, axes, kind, autolim=True):
        if self._part is not None:
            raise ValueError("the texts of an axis artist are drawn by it")
        native = _axisartist.add_axis_texts(axes._native,
                                            self._own.description())
        return _handles.adopt(self, native, axes.figure)

    @classmethod
    def _of(cls, axis_artist, part):
        """The text part *part* of *axis_artist*."""
        label = cls.__new__(cls)
        label._part = _Part(axis_artist, part)
        label._axis = axis_artist.axis
        return label

    def _set(self, key, value):
        """Set the property *key* of the texts: in the axis artist they are
        a part of, or kept, and handed to pltrs (Rust) again once an axes
        added them, for texts made on their own."""
        if self._part is not None:
            _axisartist.set_label(self._part._native, self._part._name, key,
                                  value)
            return
        self._own.props[key] = value
        self._push()

    def _push(self):
        """Hand pltrs (Rust) the texts made on their own again, once an
        axes added them."""
        if self._figure is not None:
            _axisartist.set_axis_texts(self._native, self._own.description())

    def _get(self, key):
        """Return the property *key* of the texts."""
        if self._part is not None:
            return _axisartist.get_label(
                self._part._native, self._part._name)[self._GETTERS[key]]
        return self._own.props[key]

    def _own_props(self):
        """The properties of texts made on their own."""
        if self._part is not None:
            raise AttributeError(
                "the texts of an axis artist are placed by it")
        return self._own.props

    @property
    def _ref_angle(self):
        """The direction from the point to the text, in degrees."""
        return self._own_props()["ref_angle"]

    @_ref_angle.setter
    def _ref_angle(self, angle):
        self._own_props()["ref_angle"] = angle
        self._push()

    @property
    def _offset_radius(self):
        """The distance from the point to the text, in pixels."""
        return self._own_props()["offset_radius"]

    @_offset_radius.setter
    def _offset_radius(self, radius):
        self._own_props()["offset_radius"] = radius
        self._push()

    def set_axis_direction(self, d):
        """
        Adjust the text angle and text alignment of the texts to the
        conventions of an axis on side *d*.

        Parameters
        ----------
        d : {"left", "bottom", "right", "top"}
        """
        _api.check_in_list(["left", "right", "bottom", "top"],
                           axis_direction=d)
        if self._part is not None:
            self._set("axis_direction", d)
        else:
            self._own.set_axis_direction(d)
            self._push()

    def get_axis_direction(self):
        """Return the side whose conventions the texts follow."""
        return self._get("axis_direction")

    def set_pad(self, pad):
        """
        Set the internal pad in points.

        The actual pad will be the sum of the internal pad and the
        external pad (the latter is set automatically by the `.AxisArtist`).

        Parameters
        ----------
        pad : float
            The internal pad in points.
        """
        self._set("pad", float(pad))

    def get_pad(self):
        """Return the internal pad in points."""
        return self._get("pad")

    def set_visible(self, b):
        # docstring inherited
        if self._part is None:
            super().set_visible(b)
        else:
            self._set("visible", bool(b))

    def get_visible(self):
        # docstring inherited
        if self._part is None:
            return super().get_visible()
        return self._get("visible")

    def set_color(self, color):
        """Set the color of the texts."""
        self._set("color", color)

    def get_color(self):
        """Return the color of the texts, ``"auto"`` for an axis artist's
        texts in its default color."""
        if self._part is None:
            return self._own.props["color"]
        return _auto(_axisartist.get_label_color(self._part._native,
                                                 self._part._name))

    def set_rotation(self, s):
        """Set the rotation of the texts in degrees."""
        self._set("rotation", float(s))

    def get_rotation(self):
        """Return the rotation of the texts in degrees."""
        return self._get("rotation")

    def set_ha(self, align):
        """Set the horizontal alignment of the texts."""
        self._set("ha", align)

    def set_va(self, align):
        """Set the vertical alignment of the texts."""
        self._set("va", align)

    set_horizontalalignment = set_ha
    set_verticalalignment = set_va

    def set_size(self, size):
        """Set the font size of the texts in points."""
        self._set("size", float(size))

    def get_size(self):
        """Return the font size of the texts in points."""
        return self._get("size")

    set_fontsize = set_size
    get_fontsize = get_size

    def set_text(self, s):
        """Set the text."""
        self._set("text", "" if s is None else str(s))

    def get_text(self):
        """Return the text."""
        return self._get("text") or ""


class AxisLabel(AttributeCopier, LabelBase):
    """
    Axis label of an `AxisArtist`, or made on its own: its pad away from
    its point, aligned and turned for the axis direction.
    """

    _KIND = "axislabel"

    def __init__(self, *args, axis_direction="bottom", axis=None, **kwargs):
        self._axis = axis
        super().__init__(*args, **kwargs)
        self.set_axis_direction(axis_direction)

    def get_ref_artist(self):
        # docstring inherited
        return self._axis.label

    _default_alignments = dict(left=("bottom", "center"),
                               right=("top", "center"),
                               bottom=("top", "center"),
                               top=("bottom", "center"))

    def set_default_alignment(self, d):
        """
        Set the default alignment. See `set_axis_direction` for details.

        Parameters
        ----------
        d : {"left", "bottom", "right", "top"}
        """
        va, ha = _api.getitem_checked(self._default_alignments, d=d)
        self.set_va(va)
        self.set_ha(ha)

    _default_angles = dict(left=180,
                           right=0,
                           bottom=0,
                           top=180)

    def set_default_angle(self, d):
        """
        Set the default angle. See `set_axis_direction` for details.

        Parameters
        ----------
        d : {"left", "bottom", "right", "top"}
        """
        self.set_rotation(_api.getitem_checked(self._default_angles, d=d))

    def get_color(self):
        return self.get_attribute_from_ref_artist("color")


class TickLabels(AxisLabel):
    """
    Tick labels of an `AxisArtist`, or made on its own: one text per
    location, angle and label of ``_locs_angles_labels``, each its pad
    beyond the room the alignment of the labels leaves.
    """

    _KIND = "ticklabels"

    def __init__(self, *, axis_direction="bottom", **kwargs):
        super().__init__(**kwargs)
        self.set_axis_direction(axis_direction)

    @property
    def _locs_angles_labels(self):
        """The locations (in data coordinates), angles (in degrees) and
        labels of the texts."""
        return self._own_props()["locs_angles_labels"]

    @_locs_angles_labels.setter
    def _locs_angles_labels(self, locs_angles_labels):
        self._own_props()["locs_angles_labels"] = list(locs_angles_labels)
        self._push()

    def set_locs_angles_labels(self, locs_angles_labels):
        """Set the locations, angles and labels of the texts."""
        self._locs_angles_labels = locs_angles_labels

    def set_axis_direction(self, label_direction):
        """
        Adjust the text angle and text alignment of ticklabels
        according to the Matplotlib convention.

        The *label_direction* must be one of [left, right, bottom, top].

        Parameters
        ----------
        label_direction : {"left", "bottom", "right", "top"}
        """
        super().set_axis_direction(label_direction)

    def get_ref_artist(self):
        # docstring inherited
        return self._axis.get_ticklabels()[0]

    def _level(self):
        """The tick level of an axis artist's tick labels."""
        return self._part._name.split("_")[0]

    def get_window_extents(self, renderer=None):
        """Return the display boxes of the (non-empty) tick labels."""
        if not self.get_visible():
            return []
        if self._part is None:
            if self._figure is None:
                return []
            extents = _axisartist.axis_text_extents(self._native)
        else:
            extents = _axisartist.ticklabel_extents(self._part._native,
                                                    self._level())
        return [Bbox.from_extents(*extent) for extent in extents]

    def get_texts_widths_heights_descents(self, renderer):
        """
        Return a list of ``(width, height, descent)`` tuples for ticklabels.

        Empty labels are left out.
        """
        if self._part is None:
            if self._figure is None:
                return []
            return _axisartist.axis_text_metrics(self._native)
        return _axisartist.ticklabel_metrics(self._part._native, self._level())

    def invert_axis_direction(self):
        """Turn the tick labels to the other side of the axis."""
        if self._part is not None:
            self._set("invert", True)
        else:
            opposite = {"left": "right", "right": "left",
                        "top": "bottom", "bottom": "top"}
            self.set_axis_direction(opposite[self.get_axis_direction()])


class _AxisLine(_Part):
    """The line of an axis artist (matplotlib's ``axis.line``)."""

    def set_color(self, color):
        """Set the color of the line."""
        _axisartist.set_line(self._native, "color", color)

    def set_linewidth(self, w):
        """Set the width of the line in points."""
        _axisartist.set_line(self._native, "linewidth", w)

    def set_visible(self, b):
        """Set whether the line is drawn."""
        _axisartist.set_line(self._native, "visible", bool(b))

    def get_visible(self):
        """Return whether the line is drawn."""
        return _axisartist.get_line_visible(self._native)

    set_lw = set_linewidth


class GridlinesCollection(LineCollection):
    """
    Collection of grid lines: the lines given to it. The gridlines of an
    axes drawn by axis artists are drawn by pltrs (Rust) through the axes'
    grid helper (``ax.grid()`` styles them).
    """

    def __init__(self, *args, which="major", axis="both", **kwargs):
        """
        Collection of grid lines.

        Parameters
        ----------
        which : {"major", "minor"}
            Which grid to consider.
        axis : {"both", "x", "y"}
            Which axis to consider.
        *args, **kwargs
            Passed to `.LineCollection`.
        """
        self._which = which
        self._axis = axis
        self._grid_helper = None
        super().__init__(*args, **kwargs)

    def set_which(self, which):
        """
        Select major or minor grid lines.

        Parameters
        ----------
        which : {"major", "minor"}
        """
        self._which = which

    def set_axis(self, axis):
        """
        Select axis.

        Parameters
        ----------
        axis : {"both", "x", "y"}
        """
        self._axis = axis

    def set_grid_helper(self, grid_helper):
        """
        Set grid helper.

        Parameters
        ----------
        grid_helper : `.GridHelperBase` subclass
        """
        self._grid_helper = grid_helper


class AxisArtist(martist.Artist):
    """
    An artist which draws axis (a line along which the n-th axes coord
    is constant) line, ticks, tick labels, and axis label.
    """

    zorder = 2.5

    @property
    def LABELPAD(self):
        return _axisartist.get_labelpad(self._native)

    @LABELPAD.setter
    def LABELPAD(self, v):
        _axisartist.set_labelpad(self._native, float(v))

    def __init__(self, axes, helper, offset=None, axis_direction="bottom",
                 **kwargs):
        """
        Parameters
        ----------
        axes : `mpl_toolkits.axisartist.axislines.Axes`
        helper : `~mpl_toolkits.axisartist.axislines.AxisArtistHelper`
        """
        native = helper._new_axis(axes, offset, axis_direction)
        # The axis artist is drawn once the axes adds it.
        _artist.remove(native)
        _handles.adopt(self, native, axes.figure)
        self._axes_of = axes
        self._init_parts(helper, axis_direction)
        self._internal_update(kwargs)

    @classmethod
    def _of_axes(cls, axes, native, helper, axis_direction):
        """The axis artist object around the native axis artist *native*
        of *axes*."""
        axis = _handles.wrap(native, cls, axes.figure)
        axis._axes_of = axes
        axis._init_parts(helper, axis_direction)
        return axis

    def _init_parts(self, helper, axis_direction):
        self._axis_artist_helper = helper
        # The axis whose ticks and label the parts follow by default.
        self.axis = (self._axes_of.yaxis if axis_direction in ["left", "right"]
                     else self._axes_of.xaxis)
        bind = getattr(helper, "_bind", None)
        if bind is not None:
            bind(self._native)
        self._axis_direction = axis_direction
        self.major_ticks = Ticks._of(self, "major")
        self.minor_ticks = Ticks._of(self, "minor")
        self.major_ticklabels = TickLabels._of(self, "major_ticklabels")
        self.minor_ticklabels = TickLabels._of(self, "minor_ticklabels")
        self.label = AxisLabel._of(self, "label")
        self.line = _AxisLine(self, "line")

    def _add_to_axes(self, axes, kind, autolim=True):
        native = _artist.adopt_into_axes(axes._native, self._native, "artist",
                                         False)
        return _handles.adopt(self, native, axes.figure)

    def set_axis_direction(self, axis_direction):
        """
        Adjust the direction, text angle, and text alignment of tick labels
        and axis labels following the Matplotlib convention for the rectangle
        axes.

        Parameters
        ----------
        axis_direction : {"left", "bottom", "right", "top"}
        """
        _api.check_in_list(["left", "right", "bottom", "top"],
                           axis_direction=axis_direction)
        self._axis_direction = axis_direction
        _axisartist.set_axis_direction(self._native, axis_direction)

    def set_ticklabel_direction(self, tick_direction):
        r"""
        Adjust the direction of the tick labels.

        Note that the *tick_direction*\s '+' and '-' are relative to the
        direction of the increasing coordinate.

        Parameters
        ----------
        tick_direction : {"+", "-"}
        """
        _api.getitem_checked({"+": 0, "-": 180}, tick_direction=tick_direction)
        _axisartist.set_ticklabel_direction(self._native, tick_direction)

    def invert_ticklabel_direction(self):
        """Turn the tick labels to the other side of the axis line."""
        self.major_ticklabels.invert_axis_direction()

    def set_axislabel_direction(self, label_direction):
        r"""
        Adjust the direction of the axis label.

        Note that the *label_direction*\s '+' and '-' are relative to the
        direction of the increasing coordinate.

        Parameters
        ----------
        label_direction : {"+", "-"}
        """
        _api.getitem_checked({"+": 0, "-": 180},
                             label_direction=label_direction)
        _axisartist.set_axislabel_direction(self._native, label_direction)

    def get_helper(self):
        """
        Return axis artist helper instance.
        """
        return self._axis_artist_helper

    def set_axisline_style(self, axisline_style=None, **kwargs):
        """
        Set the axisline style.

        The new style is completely defined by the passed attributes. Existing
        style attributes are forgotten.

        Parameters
        ----------
        axisline_style : str or None
            The line style, e.g. '->', optionally followed by a comma-separated
            list of attributes. Alternatively, the attributes can be provided
            as keywords.

            If *None* this returns a string containing the available styles.

        Examples
        --------
        The following two commands are equal:

        >>> set_axisline_style("->,size=1.5")
        >>> set_axisline_style("->", size=1.5)
        """
        from .axisline_style import AxislineStyle
        if axisline_style is None:
            return AxislineStyle.pprint_styles()
        if not isinstance(axisline_style, AxislineStyle._Base):
            axisline_style = AxislineStyle(axisline_style, **kwargs)
        _axisartist.set_axisline_style(self._native,
                                       axisline_style._native_text())
        self._axisline_style = axisline_style

    def get_axisline_style(self):
        """Return the current axisline style."""
        return self.__dict__.get("_axisline_style")

    def set_label(self, s):
        # docstring inherited
        self.label.set_text(s)

    def get_label(self):
        """Return the text of the axis label."""
        return self.label.get_text()

    def toggle(self, all=None, ticks=None, ticklabels=None, label=None):
        """
        Toggle visibility of ticks, ticklabels, and (axis) label.
        To turn all off, ::

          axis.toggle(all=False)

        To turn all off but ticks on ::

          axis.toggle(all=False, ticks=True)

        To turn all on but (axis) label off ::

          axis.toggle(all=True, label=False)

        """
        _axisartist.toggle(self._native, tuple(
            None if value is None else bool(value)
            for value in (all, ticks, ticklabels, label)))
