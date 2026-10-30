"""
The :mod:`.axis_artist` module implements custom artists to draw axis lines
(matplotlib's ``mpl_toolkits.axisartist.axis_artist``, from matplotlib
3.11.2, under the Matplotlib License).

An `AxisArtist` is a pltrs (Rust) axis artist of its axes, drawing its line,
ticks, tick labels and label: ``axis.major_ticks``, ``axis.major_ticklabels``,
``axis.label`` and ``axis.line`` act on those parts of it
(``_native.axisartist``). `Ticks` made on their own are tick marks an axes
draws once ``ax.add_artist(ticks)`` adds them.
"""

from pltrs import _api, _handles
import pltrs.artist as martist
from pltrs.collections import LineCollection
from pltrs._native import artist as _artist, axisartist as _axisartist


class _Part:
    """A part of an axis artist (its ticks, its tick labels, its label or
    its line), acting on the pltrs (Rust) axis artist."""

    def __init__(self, axis_artist, name):
        self._axis_artist = axis_artist
        self._name = name

    @property
    def _native(self):
        return self._axis_artist._native


class Ticks(martist.Artist):
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

    def __init__(self, ticksize, tick_out=False, *, axis=None, **kwargs):
        super().__init__()
        self._part = None
        self._ticksize = ticksize
        self.locs_angles_labels = []
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
        return ticks

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
        else:
            self._set_native("color", color)

    def set_markeredgewidth(self, width):
        """Set the width of the ticks in points."""
        width = None if width == "auto" else width
        if self._part is None:
            self._markeredgewidth = width
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

    def _add_to_axes(self, axes, kind, autolim=True):
        if self._part is not None:
            raise ValueError("the ticks of an axis artist are drawn by it")
        axis_name = "x" if self._axis is None else self._axis.axis_name
        native = _axisartist.add_tick_marks(
            axes._native, axis_name,
            [((float(x), float(y)), float(angle))
             for (x, y), angle in self._locs_angles],
            (float(self._ticksize), self._tick_direction, self._color,
             self._markeredgewidth))
        return _handles.adopt(self, native, axes.figure)


class LabelBase(martist.Artist):
    """
    A base class for `.AxisLabel` and `.TickLabels`: the texts of an
    axis artist, placed at a distance from their reference point.

    pltrs draws these texts as parts of an `AxisArtist`
    (``axis.label``, ``axis.major_ticklabels``).
    """

    def __init__(self, *args, **kwargs):
        super().__init__()
        self._part = None
        self._args = args
        self._internal_update(kwargs)

    def _add_to_axes(self, axes, kind, autolim=True):
        raise NotImplementedError(
            f"pltrs: add_artist of a {type(self).__name__} made on its own "
            "(pltrs draws axis texts as parts of an AxisArtist)")

    @classmethod
    def _of(cls, axis_artist, part):
        """The text part *part* of *axis_artist*."""
        label = cls.__new__(cls)
        label._part = _Part(axis_artist, part)
        return label

    def _set_native(self, key, value):
        _axisartist.set_label(self._part._native, self._part._name, key, value)

    def _get_native(self):
        return _axisartist.get_label(self._part._native, self._part._name)

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
        self._set_native("axis_direction", d)

    def get_axis_direction(self):
        """Return the side whose conventions the texts follow."""
        return self._get_native()[0]

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
        self._set_native("pad", float(pad))

    def get_pad(self):
        """Return the internal pad in points."""
        return self._get_native()[1]

    def set_visible(self, b):
        # docstring inherited
        self._set_native("visible", bool(b))

    def get_visible(self):
        # docstring inherited
        return self._get_native()[2]

    def set_color(self, color):
        """Set the color of the texts."""
        self._set_native("color", color)

    def set_rotation(self, s):
        """Set the rotation of the texts in degrees."""
        self._set_native("rotation", float(s))

    def get_rotation(self):
        """Return the rotation of the texts in degrees."""
        return self._get_native()[3]

    def set_ha(self, align):
        """Set the horizontal alignment of the texts."""
        self._set_native("ha", align)

    def set_va(self, align):
        """Set the vertical alignment of the texts."""
        self._set_native("va", align)

    set_horizontalalignment = set_ha
    set_verticalalignment = set_va

    def set_size(self, size):
        """Set the font size of the texts in points."""
        self._set_native("size", float(size))

    def get_size(self):
        """Return the font size of the texts in points."""
        return self._get_native()[4]

    set_fontsize = set_size
    get_fontsize = get_size


class AxisLabel(LabelBase):
    """
    Axis label of an `AxisArtist`.

    pltrs draws the label as part of its `AxisArtist` (``axis.label``).
    """

    def set_text(self, s):
        """Set the text of the label."""
        self._set_native("text", s)

    def get_text(self):
        """Return the text of the label."""
        return self._get_native()[5] or ""


class TickLabels(LabelBase):
    """
    Tick labels of an `AxisArtist`.

    pltrs draws the tick labels as part of their `AxisArtist`
    (``axis.major_ticklabels``).
    """

    def invert_axis_direction(self):
        """Turn the tick labels to the other side of the axis."""
        self._set_native("invert", True)


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

    def invert_ticklabel_direction(self):
        """Turn the tick labels to the other side of the axis line."""
        self.major_ticklabels.invert_axis_direction()

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
