"""
The :mod:`.axis_artist` module implements custom artists to draw axis lines
(matplotlib's ``mpl_toolkits.axisartist.axis_artist``, from matplotlib
3.11.2, under the Matplotlib License).

An `AxisArtist` is a pltrs (Rust) axis artist of its axes, which draws its
line, ticks, tick labels and label. Its parts are matplotlib's: `Ticks` are
`.Line2D`\\s and `LabelBase`, `AxisLabel` and `TickLabels` are
`.Text`\\s, holding every property as matplotlib's do. They are mirrors
(``pltrs._mirrors``) of what pltrs (Rust) draws: each of their setters
hands the part of the axis artist what it draws with
(``_native.axisartist.restyle_ticks``, ``restyle_label``). Ticks and texts
made by their constructors and added to an axes (``ax.add_artist(ticks)``)
are drawn by pltrs (Rust) the same way, as tick marks
(``add_tick_marks``) and axis texts (``add_axis_texts``) that their
setters describe again.
"""

from operator import methodcaller

import numpy as np

from pltrs import _api, _handles, _mirrors, cbook, rcParams
import pltrs.artist as martist
import pltrs.colors as mcolors
import pltrs.text as mtext
from pltrs.collections import LineCollection
from pltrs.font_manager import FontManager
from pltrs.lines import Line2D
from pltrs.path import Path
from pltrs.transforms import Affine2D, Bbox, ScaledTranslation
from pltrs._native import (
    artist as _artist, axisartist as _axisartist, text as _text)


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


class _Drawn(_mirrors.Mirroring):
    """
    What pltrs (Rust) draws for a `Ticks` or a text of this module: the
    part *name* (``"major"``, ``"minor"``, ``"major_ticklabels"``,
    ``"minor_ticklabels"`` or ``"label"``) of the axis artist
    *axis_artist*, or, without one, the tick marks or texts *native* that
    *axes* added. The part is in the figure of *figure_of* (matplotlib
    gives the ticks of an axis artist none).
    """

    def __init__(self, native, *, figure_of, axis_artist=None, name=None):
        self.native = native
        self.figure_of = figure_of
        self.axis_artist = axis_artist
        self.name = name
        self._transform = None

    def push(self, artist):
        artist._restyle(self)

    def figure(self, artist, root=False):
        if self.figure_of is None:
            return _mirrors.OWN
        return self.figure_of.get_figure(root=root)

    def transform(self, artist):
        # A part of an axis artist is placed by the axis artist's helper,
        # unless the program gave it a transform of its own.
        if self.axis_artist is None or "_given_transform" in artist.__dict__:
            return _mirrors.OWN
        if self._transform is None:
            self._transform = artist._part_transform(self.axis_artist)
        return self._transform

    def transform_set(self, artist):
        return True if self.axis_artist is not None else _mirrors.OWN

    def window_extent(self, artist, renderer=None, dpi=None):
        return artist._drawn_extent(self)

    def refresh(self, artist):
        artist._refresh(self)

    def drawn_since(self, artist, key):
        """Whether the axis artist was drawn since *artist* last asked
        under *key* (an attribute of *artist* keeping the count)."""
        count = _artist.draw_count(self.native)
        if count == artist.__dict__.get(key, 0):
            return False
        artist.__dict__[key] = count
        return True


class _DrawnOnItsOwn:
    """
    The behaviour shared by `Ticks` and `LabelBase` added to an axes on
    their own: pltrs (Rust) draws them as an artist of their own (a
    subclass ``_add_native(axes)``\\s it), which their setters describe
    again, while the object keeps its properties on the `.Line2D` or
    `.Text` it is.
    """

    def _push(self):
        """Hand what pltrs (Rust) draws for the object its state again."""
        mirror = _mirrors.mirroring(self)
        if mirror is not None:
            mirror.push(self)

    def _add_to_axes(self, axes, kind, autolim=True):
        if isinstance(_mirrors.mirroring(self), _Drawn):
            raise ValueError("Can not reset the Axes. You are probably trying "
                             "to reuse an artist in more than one Axes which "
                             "is not supported")
        if not self.is_transform_set():
            self.set_transform(axes.transData)
        native = self._add_native(axes)
        figure = axes.get_figure(root=True)
        figure._artists[_artist.id(native)] = self
        self._axes = axes
        _mirrors.attach(self, _Drawn(native, figure_of=axes))
        self._sync_drawn(native)
        return self

    def _sync_drawn(self, native):
        """Give the artist *native* drawing the object the visibility,
        opacity and z-order of the object."""
        _artist.set_visible(native, self.get_visible())
        _artist.set_alpha(native, self.get_alpha())
        _artist.set_zorder(native, float(self.get_zorder()))

    def remove(self):
        # docstring inherited
        mirror = _mirrors.mirroring(self)
        if not isinstance(mirror, _Drawn) or mirror.axis_artist is not None:
            return super().remove()
        _artist.remove(mirror.native)
        figure = mirror.figure_of.get_figure(root=True)
        figure._artists.pop(_artist.id(mirror.native), None)
        del self._mirror
        self._axes = None
        return None


class _AutoStyle:
    """
    The colour and width `Ticks` leave ``"auto"`` (following their
    reference artist), which a `.Line2D` of pltrs cannot hold: what
    matplotlib's `.Line2D` getters give `Ticks` before the
    `AttributeCopier` asks the reference artist.
    """

    def set_color(self, color):
        # docstring inherited
        # Unlike the base Line2D.set_color, this also supports "auto".
        self._auto_color = cbook._str_equal(color, "auto")
        if self._auto_color:
            self.stale = True
        else:
            Line2D.set_color(self, color)

    def get_color(self):
        return "auto" if self._auto_color else Line2D.get_color(self)

    @property
    def _color(self):
        return self.get_color()

    def set_markeredgewidth(self, ew):
        # docstring inherited
        self._auto_width = cbook._str_equal(ew, "auto")
        if self._auto_width:
            self.stale = True
        else:
            Line2D.set_markeredgewidth(self, ew)

    def get_markeredgewidth(self):
        return "auto" if self._auto_width else Line2D.get_markeredgewidth(self)

    def _follows_color(self, setting):
        """Whether the marker colour *setting* is the line colour, while
        that is ``"auto"``."""
        return self._auto_color and cbook._str_lower_equal(
            self._get(setting), "auto")

    def get_markeredgecolor(self):
        if self._follows_color("markeredgecolor_setting"):
            return "auto"
        return Line2D.get_markeredgecolor(self)

    def get_markerfacecolor(self):
        if (self._follows_color("markerfacecolor_setting")
                and self.get_fillstyle() != "none"):
            return "auto"
        return Line2D.get_markerfacecolor(self)


@_mirrors.pushing
class Ticks(AttributeCopier, _AutoStyle, _DrawnOnItsOwn, Line2D):
    """
    Ticks are derived from `.Line2D`, and note that ticks themselves
    are markers. Thus, you should use set_mec, set_mew, etc.

    To change the tick size (length), use `set_ticksize`.
    To change the direction of the ticks, use ``set_tick_direction("in")`` (or
    "out", or "inout").
    """

    locs_angles_labels = _api.deprecated("3.11")(property(lambda self: []))

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    @_api.delete_parameter("3.11", "tick_out", alternative="tick_direction")
    def __init__(self, ticksize, tick_out=False, *, axis=None, **kwargs):
        self._ticksize = ticksize
        self.set_tick_out(tick_out)

        self._axis = axis
        if self._axis is not None:
            if "color" not in kwargs:
                kwargs["color"] = "auto"
            if "mew" not in kwargs and "markeredgewidth" not in kwargs:
                kwargs["markeredgewidth"] = "auto"
        # pltrs (Rust) lines hold no "auto": the object keeps it.
        self._auto_color = cbook._str_equal(kwargs.get("color"), "auto")
        self._auto_width = cbook._str_equal(kwargs.get("markeredgewidth"), "auto")
        for name, auto in (("color", self._auto_color),
                           ("markeredgewidth", self._auto_width)):
            if auto:
                del kwargs[name]

        Line2D.__init__(self, [0.], [0.], **kwargs)
        self.set_snap(True)

    def get_ref_artist(self):
        # docstring inherited
        return self._axis.majorTicks[0].tick1line

    def set_color(self, color):
        # docstring inherited
        # Unlike the base Line2D.set_color, this also supports "auto".
        if not cbook._str_equal(color, "auto"):
            mcolors._check_color_like(color=color)
        _AutoStyle.set_color(self, color)

    def get_color(self):
        return self.get_attribute_from_ref_artist("color")

    def get_markeredgecolor(self):
        return self.get_attribute_from_ref_artist("markeredgecolor")

    def get_markeredgewidth(self):
        return self.get_attribute_from_ref_artist("markeredgewidth")

    def set_tick_direction(self, direction):
        _api.check_in_list(["in", "out", "inout"], direction=direction)
        self._tick_dir = direction

    def get_tick_direction(self):
        return self._tick_dir

    def set_tick_out(self, b):
        """
        Set whether ticks are drawn inside or outside the axes.

        .. admonition:: Discouraged
           Consider using the more general method `.set_tick_direction` instead.
        """
        self._tick_dir = "out" if b else "in"

    @_api.deprecated("3.11", alternative="get_tick_direction")
    def get_tick_out(self):
        """Return whether ticks are drawn inside or outside the axes."""
        if self._tick_dir == "in":
            return False
        elif self._tick_dir == "out":
            return True
        else:
            raise ValueError(
                f"Tick direction ({self._tick_dir!r}) not supported by get_tick_out, "
                f"use get_tick_direction instead")

    def set_ticksize(self, ticksize):
        """Set length of the ticks in points."""
        self._ticksize = ticksize

    def get_ticksize(self):
        """Return length of the ticks in points."""
        return self._ticksize

    def set_locs_angles(self, locs_angles):
        self.locs_angles = locs_angles

    @property
    def locs_angles(self):
        """The locations and angles (in degrees) of the ticks: those set,
        or for the ticks of an `AxisArtist`, those its last draw placed
        (matplotlib's ``_update_ticks``) when it was drawn since."""
        mirror = _mirrors.mirroring(self)
        if (isinstance(mirror, _Drawn) and mirror.axis_artist is not None
                and mirror.drawn_since(self, "_locs_angles_at")):
            self.__dict__["_locs_angles"] = mirror.axis_artist._tick_info(
                mirror.name)[0]
        try:
            return self.__dict__["_locs_angles"]
        except KeyError:
            raise AttributeError(
                f"{type(self).__name__!r} object has no attribute "
                f"'locs_angles'") from None

    @locs_angles.setter
    def locs_angles(self, locs_angles):
        self.__dict__["_locs_angles"] = locs_angles
        self._push()

    _tick_paths = {
        "in": Path([[0, 0], [+1, 0]]),
        "inout": Path([[-1/2, 0.], [+1/2, 0]]),
        "out": Path([[-1, 0], [0, 0]]),
    }

    def draw(self, renderer):
        if not self.get_visible():
            return

        gc = renderer.new_gc()
        edgecolor = mcolors.to_rgba(self.get_markeredgecolor())
        gc.set_foreground(edgecolor, isRGBA=True)
        gc.set_linewidth(self.get_markeredgewidth())
        gc.set_alpha(self.get_alpha())

        tickvert_path = self._tick_paths[self._tick_dir]
        path_trans = self.get_transform()
        marker_transform = (Affine2D()
                            .scale(renderer.points_to_pixels(self._ticksize)))

        for loc, angle in self.locs_angles:
            locs = path_trans.transform_non_affine(np.array([loc]))
            if self.axes and not self.axes.viewLim.contains(*locs[0]):
                continue
            renderer.draw_markers(
                gc, tickvert_path,
                marker_transform + Affine2D().rotate_deg(angle),
                Path(locs), path_trans.get_affine())

        gc.restore()

    # -- what pltrs (Rust) draws ------------------------------------------

    def _part_transform(self, axis_artist):
        """The transform matplotlib gives the ticks of *axis_artist*."""
        return (axis_artist._axis_artist_helper.get_tick_transform(axis_artist._axes_of)
                + axis_artist.offset_transform)

    def _own_style(self):
        """The colour and width the marks are drawn with, None for those
        following the reference artist."""
        color = _AutoStyle.get_markeredgecolor(self)
        width = _AutoStyle.get_markeredgewidth(self)
        return (None if cbook._str_equal(color, "auto") else color,
                None if cbook._str_equal(width, "auto") else float(width))

    def _marks(self):
        """The native description of ticks drawn on their own: their axis,
        locations and angles, and style."""
        color, width = self._own_style()
        return ("x" if self._axis is None else self._axis.axis_name,
                [((float(x), float(y)), float(angle))
                 for (x, y), angle in self.__dict__.get("_locs_angles", [])],
                (float(self._ticksize), self._tick_dir, color, width))

    def _restyle(self, drawn):
        if drawn.axis_artist is None:
            _axisartist.set_tick_marks(drawn.native, *self._marks())
            self._sync_drawn(drawn.native)
            return
        color, width = self._own_style()
        _axisartist.restyle_ticks(drawn.native, drawn.name, (
            float(self._ticksize), self._tick_dir, bool(self.get_visible()),
            color, width, self.get_alpha()))

    def _add_native(self, axes):
        return _axisartist.add_tick_marks(axes._native, *self._marks())

    def _drawn_extent(self, drawn):
        return _mirrors.OWN

    def _refresh(self, drawn):
        pass


def _look_of(text):
    """How pltrs (Rust) typesets *text* beyond its size, colour, angle and
    alignment: its font properties, the alignment of its lines (None: as
    the text), its line spacing (None: normal) and its rotation mode."""
    return (FontManager._props(text.get_fontproperties()),
            _text.get_multialignment(text._native),
            _text.get_linespacing(text._native),
            text.get_rotation_mode())


@_mirrors.pushing
class LabelBase(_DrawnOnItsOwn, mtext.Text):
    """
    A base class for `.AxisLabel` and `.TickLabels`. The position and
    angle of the text are calculated by the offset_ref_angle,
    text_ref_angle, and offset_radius attributes.
    """

    locs_angles_labels = _api.deprecated("3.11")(property(lambda self: []))

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    #: The kind of texts pltrs (Rust) draws for the class on its own.
    _KIND = "label"

    def __init__(self, *args, **kwargs):
        self._ref_angle = 0
        self._offset_radius = 0.

        super().__init__(*args, **kwargs)

        self.set_rotation_mode("anchor")
        self._text_follow_ref_angle = True

    @property
    def _ref_angle(self):
        return self.__dict__["_ref_angle_value"]

    @_ref_angle.setter
    def _ref_angle(self, angle):
        self.__dict__["_ref_angle_value"] = angle
        self._push()

    @property
    def _offset_radius(self):
        return self.__dict__["_offset_radius_value"]

    @_offset_radius.setter
    def _offset_radius(self, radius):
        self.__dict__["_offset_radius_value"] = radius
        self._push()

    @property
    def _text_ref_angle(self):
        if self._text_follow_ref_angle:
            return self._ref_angle + 90
        else:
            return 0

    @property
    def _offset_ref_angle(self):
        return self._ref_angle

    _get_opposite_direction = {"left": "right",
                               "right": "left",
                               "top": "bottom",
                               "bottom": "top"}.__getitem__

    def draw(self, renderer):
        if not self.get_visible():
            return

        # save original and adjust some properties
        with _mirrors.detached(self):
            tr = self.get_transform()
            angle_orig = self.get_rotation()
            color_orig = mtext.Text.get_color(self)
            theta = np.deg2rad(self._offset_ref_angle)
            dd = self._offset_radius
            dx, dy = dd * np.cos(theta), dd * np.sin(theta)

            self.set_transform(tr + Affine2D().translate(dx, dy))
            self.set_rotation(self._text_ref_angle + angle_orig)
            # matplotlib's `Text.draw` asks `get_color`, which resolves
            # "auto"; pltrs (Rust) draws the colour the text holds.
            mtext.Text.set_color(self, self.get_color())
            super().draw(renderer)
            # restore original properties
            self.set_transform(tr)
            self.set_rotation(angle_orig)
            mtext.Text.set_color(self, color_orig)

    def get_window_extent(self, renderer=None):
        extent = self._drawn_extent(_mirrors.mirroring(self))
        if extent is not _mirrors.OWN:
            return extent
        if renderer is None:
            renderer = self.get_figure(root=True)._get_renderer()

        # save original and adjust some properties
        with _mirrors.detached(self):
            tr = self.get_transform()
            angle_orig = self.get_rotation()
            theta = np.deg2rad(self._offset_ref_angle)
            dd = self._offset_radius
            dx, dy = dd * np.cos(theta), dd * np.sin(theta)

            self.set_transform(tr + Affine2D().translate(dx, dy))
            self.set_rotation(self._text_ref_angle + angle_orig)
            bbox = super().get_window_extent(renderer).frozen()
            # restore original properties
            self.set_transform(tr)
            self.set_rotation(angle_orig)

        return bbox

    # -- what pltrs (Rust) draws ------------------------------------------

    def _part_transform(self, axis_artist):
        """The transform matplotlib gives the text of *axis_artist*."""
        return (axis_artist._axis_artist_helper.get_tick_transform(axis_artist._axes_of)
                + axis_artist.offset_transform)

    def _resolved_color(self):
        """The colour the texts are drawn in: the reference artist's for
        ``"auto"``, the default text colour without one."""
        try:
            color = self.get_color()
        except AttributeError:
            color = "auto"
        return rcParams["text.color"] if cbook._str_equal(color, "auto") else color

    def _pad_points(self):
        """The pad pltrs (Rust) leaves, in points (only `AxisLabel`\\s have
        one)."""
        return 0.

    def _texts(self):
        """The texts drawn: their reference points, angles and strings."""
        x, y = self.get_position()
        return [((float(x), float(y)), float(self._ref_angle), self.get_text())]

    def _description(self):
        """The native description of the texts drawn on their own."""
        return (self._KIND, float(self._offset_radius), self._texts(), (
            float(self._pad_points()), self._resolved_color(),
            float(self.get_fontsize()), _look_of(self),
            float(self.get_rotation()), (self.get_va(), self.get_ha()),
            self.__dict__.get("_axis_direction", "bottom")))

    def _restyle(self, drawn):
        if drawn.axis_artist is None:
            _axisartist.set_axis_texts(drawn.native, self._description())
            self._sync_drawn(drawn.native)
            return
        color = mtext.Text.get_color(self)
        _axisartist.restyle_label(drawn.native, drawn.name, (
            (bool(self.get_visible()), float(self._pad_points()),
             None if cbook._str_equal(color, "auto") else color),
            (float(self.get_fontsize()), _look_of(self), float(self.get_rotation())),
            (self.get_va(), self.get_ha(), self._axis_direction),
            self._part_text()))

    def _part_text(self):
        """The text an axis artist's part draws (only its label has one)."""
        return None

    def _add_native(self, axes):
        return _axisartist.add_axis_texts(axes._native, self._description())

    def _drawn_extent(self, drawn):
        """Where pltrs (Rust) draws the text (`OWN` for a text it does not
        draw)."""
        if not isinstance(drawn, _Drawn):
            return _mirrors.OWN
        if drawn.axis_artist is None:
            extents = _axisartist.axis_text_extents(drawn.native)
        else:
            extents = self._part_extents(drawn)
        return (Bbox.from_extents(*extents[-1]) if extents
                else Bbox([[0, 0], [0, 0]]))

    def _part_extents(self, drawn):
        """The extents of what the part *drawn* of an axis artist draws."""
        extent = _axisartist.label_extent(drawn.native)
        return [] if extent is None else [extent]

    def _refresh(self, drawn):
        pass


@_mirrors.pushing
class AxisLabel(AttributeCopier, LabelBase):
    """
    Axis label. Derived from `.Text`. The position of the text is updated
    in the fly, so changing text position has no effect. Otherwise, the
    properties can be changed as a normal `.Text`.

    To change the pad between tick labels and axis label, use `set_pad`.
    """

    _KIND = "axislabel"

    def __init__(self, *args, axis_direction="bottom", axis=None, **kwargs):
        self._axis = axis
        self._pad = 5
        self._external_pad = 0  # in pixels
        LabelBase.__init__(self, *args, **kwargs)
        self.set_axis_direction(axis_direction)

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
        self._pad = pad

    def get_pad(self):
        """
        Return the internal pad in points.

        See `.set_pad` for more details.
        """
        return self._pad

    def get_ref_artist(self):
        # docstring inherited
        return self._axis.label

    def get_text(self):
        # docstring inherited
        t = super().get_text()
        if t == "__from_axes__":
            return self._axis.label.get_text()
        return t

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

    def set_axis_direction(self, d):
        """
        Adjust the text angle and text alignment of axis label
        according to the matplotlib convention.

        =====================    ========== ========= ========== ==========
        Property                 left       bottom    right      top
        =====================    ========== ========= ========== ==========
        axislabel angle          180        0         0          180
        axislabel va             center     top       center     bottom
        axislabel ha             right      center    right      center
        =====================    ========== ========= ========== ==========

        Note that the text angles are actually relative to (90 + angle
        of the direction to the ticklabel), which gives 0 for bottom
        axis.

        Parameters
        ----------
        d : {"left", "bottom", "right", "top"}
        """
        self.set_default_alignment(d)
        self.set_default_angle(d)
        # The side pltrs (Rust) measures the label on.
        self._axis_direction = d

    def get_color(self):
        return self.get_attribute_from_ref_artist("color")

    def draw(self, renderer):
        if not self.get_visible():
            return

        self._offset_radius = \
            self._external_pad + renderer.points_to_pixels(self.get_pad())

        super().draw(renderer)

    def get_window_extent(self, renderer=None):
        if renderer is None and self._drawn_extent(
                _mirrors.mirroring(self)) is _mirrors.OWN:
            renderer = self.get_figure(root=True)._get_renderer()
        if not self.get_visible():
            return

        if renderer is not None:
            r = self._external_pad + renderer.points_to_pixels(self.get_pad())
            self._offset_radius = r

        bb = super().get_window_extent(renderer)

        return bb

    # -- what pltrs (Rust) draws ------------------------------------------

    def _part_transform(self, axis_artist):
        return (axis_artist._axis_artist_helper.get_axislabel_transform(axis_artist._axes_of)
                + axis_artist.offset_transform)

    def _pad_points(self):
        return self.get_pad()

    def _part_text(self):
        text = mtext.Text.get_text(self)
        return None if text == "__from_axes__" else text

    def _refresh(self, drawn):
        # matplotlib's draw places the label where the helper says.
        if (drawn.axis_artist is None or not self.get_visible()
                or not drawn.drawn_since(self, "_placed_at")):
            return
        axis_artist = drawn.axis_artist
        xy, _ = axis_artist._axis_artist_helper.get_axislabel_pos_angle(
            axis_artist._axes_of)
        if xy is not None:
            with _mirrors.detached(self):
                self.set(x=xy[0], y=xy[1])


@_mirrors.pushing
class TickLabels(AxisLabel):  # mtext.Text
    """
    Tick labels. While derived from `.Text`, this single artist draws all
    ticklabels. As in `.AxisLabel`, the position of the text is updated
    in the fly, so changing text position has no effect. Otherwise,
    the properties can be changed as a normal `.Text`. Unlike the
    ticklabels of the mainline Matplotlib, properties of a single
    ticklabel alone cannot be modified.

    To change the pad between ticks and ticklabels, use `~.AxisLabel.set_pad`.
    """

    _KIND = "ticklabels"

    def __init__(self, *, axis_direction="bottom", **kwargs):
        super().__init__(**kwargs)
        self.set_axis_direction(axis_direction)
        self._axislabel_pad = 0

    def get_ref_artist(self):
        # docstring inherited
        return self._axis.get_ticklabels()[0]

    def set_axis_direction(self, label_direction):
        """
        Adjust the text angle and text alignment of ticklabels
        according to the Matplotlib convention.

        The *label_direction* must be one of [left, right, bottom, top].

        =====================    ========== ========= ========== ==========
        Property                 left       bottom    right      top
        =====================    ========== ========= ========== ==========
        ticklabel angle          90         0         -90        180
        ticklabel va             center     baseline  center     baseline
        ticklabel ha             right      center    right      center
        =====================    ========== ========= ========== ==========

        Note that the text angles are actually relative to (90 + angle
        of the direction to the ticklabel), which gives 0 for bottom
        axis.

        Parameters
        ----------
        label_direction : {"left", "bottom", "right", "top"}

        """
        self.set_default_alignment(label_direction)
        self.set_default_angle(label_direction)
        self._axis_direction = label_direction

    def invert_axis_direction(self):
        label_direction = self._get_opposite_direction(self._axis_direction)
        self.set_axis_direction(label_direction)

    def _get_ticklabels_offsets(self, renderer, label_direction):
        """
        Calculate the ticklabel offsets from the tick and their total heights.

        The offset only takes account the offset due to the vertical alignment
        of the ticklabels: if axis direction is bottom and va is 'top', it will
        return 0; if va is 'baseline', it will return (height-descent).
        """
        whd_list = self.get_texts_widths_heights_descents(renderer)

        if not whd_list:
            return 0, 0

        r = 0
        va, ha = self.get_va(), self.get_ha()

        if label_direction == "left":
            pad = max(w for w, h, d in whd_list)
            if ha == "left":
                r = pad
            elif ha == "center":
                r = .5 * pad
        elif label_direction == "right":
            pad = max(w for w, h, d in whd_list)
            if ha == "right":
                r = pad
            elif ha == "center":
                r = .5 * pad
        elif label_direction == "bottom":
            pad = max(h for w, h, d in whd_list)
            if va == "bottom":
                r = pad
            elif va == "center":
                r = .5 * pad
            elif va == "baseline":
                max_ascent = max(h - d for w, h, d in whd_list)
                max_descent = max(d for w, h, d in whd_list)
                r = max_ascent
                pad = max_ascent + max_descent
        elif label_direction == "top":
            pad = max(h for w, h, d in whd_list)
            if va == "top":
                r = pad
            elif va == "center":
                r = .5 * pad
            elif va == "baseline":
                max_ascent = max(h - d for w, h, d in whd_list)
                max_descent = max(d for w, h, d in whd_list)
                r = max_descent
                pad = max_ascent + max_descent

        # r : offset
        # pad : total height of the ticklabels. This will be used to
        # calculate the pad for the axislabel.
        return r, pad

    _default_alignments = dict(left=("center", "right"),
                               right=("center", "left"),
                               bottom=("baseline", "center"),
                               top=("baseline", "center"))

    _default_angles = dict(left=90,
                           right=-90,
                           bottom=0,
                           top=180)

    def draw(self, renderer):
        if not self.get_visible():
            self._axislabel_pad = self._external_pad
            return

        r, total_width = self._get_ticklabels_offsets(renderer,
                                                      self._axis_direction)

        pad = self._external_pad + renderer.points_to_pixels(self.get_pad())
        self._offset_radius = r + pad

        for (x, y), a, l in self._locs_angles_labels:
            if not l.strip():
                continue
            self._ref_angle = a
            self.set_x(x)
            self.set_y(y)
            self.set_text(l)
            LabelBase.draw(self, renderer)

        # the value saved will be used to draw axislabel.
        self._axislabel_pad = total_width + pad

    def set_locs_angles_labels(self, locs_angles_labels):
        self._locs_angles_labels = locs_angles_labels

    @property
    def _locs_angles_labels(self):
        """The locations, angles and labels of the texts: those set, or for
        the tick labels of an `AxisArtist`, those its last draw placed when
        it was drawn since."""
        mirror = _mirrors.mirroring(self)
        if isinstance(mirror, _Drawn):
            self._refresh_labels(mirror)
        try:
            return self.__dict__["_labels"]
        except KeyError:
            raise AttributeError(
                f"{type(self).__name__!r} object has no attribute "
                f"'_locs_angles_labels'") from None

    @_locs_angles_labels.setter
    def _locs_angles_labels(self, locs_angles_labels):
        self.__dict__["_labels"] = locs_angles_labels
        self.__dict__["_labels_given"] = True
        self._push()

    def _refresh_labels(self, drawn):
        """Take the labels the last draw of the axis artist placed, when it
        was drawn since (matplotlib's draw hands its tick labels them)."""
        if drawn.axis_artist is not None and drawn.drawn_since(self, "_labels_at"):
            self.__dict__["_labels"] = self._drawn_labels(drawn)
            self.__dict__["_labels_given"] = False

    def _measures_drawn(self, drawn):
        """Whether the texts measured are those pltrs (Rust) draws: unless
        the program gave the tick labels of an axis artist no labels since
        it was last drawn, which matplotlib measures (until its next draw
        places the drawn ones)."""
        if drawn.axis_artist is None:
            return True
        self._refresh_labels(drawn)
        return not (self.__dict__.get("_labels_given")
                    and not self.__dict__["_labels"])

    def get_window_extents(self, renderer=None):
        mirror = _mirrors.mirroring(self)
        if not isinstance(mirror, _Drawn) and renderer is None:
            renderer = self.get_figure(root=True)._get_renderer()

        if not self.get_visible():
            self._axislabel_pad = self._external_pad
            return []

        if isinstance(mirror, _Drawn):
            if not self._measures_drawn(mirror):
                return []
            extents = (_axisartist.axis_text_extents(mirror.native)
                       if mirror.axis_artist is None
                       else self._part_extents(mirror))
            return [Bbox.from_extents(*extent) for extent in extents]

        bboxes = []

        r, total_width = self._get_ticklabels_offsets(renderer,
                                                      self._axis_direction)

        pad = self._external_pad + renderer.points_to_pixels(self.get_pad())
        self._offset_radius = r + pad

        for (x, y), a, l in self._locs_angles_labels:
            self._ref_angle = a
            self.set_x(x)
            self.set_y(y)
            self.set_text(l)
            bb = LabelBase.get_window_extent(self, renderer)
            bboxes.append(bb)

        # the value saved will be used to draw axislabel.
        self._axislabel_pad = total_width + pad

        return bboxes

    def get_texts_widths_heights_descents(self, renderer):
        """
        Return a list of ``(width, height, descent)`` tuples for ticklabels.

        Empty labels are left out.
        """
        mirror = _mirrors.mirroring(self)
        if isinstance(mirror, _Drawn):
            if not self._measures_drawn(mirror):
                return []
            if mirror.axis_artist is None:
                return _axisartist.axis_text_metrics(mirror.native)
            return _axisartist.ticklabel_metrics(mirror.native, self._level(mirror))
        whd_list = []
        for _loc, _angle, label in self._locs_angles_labels:
            if not label.strip():
                continue
            clean_line, ismath = self._preprocess_math(label)
            whd = mtext._get_text_metrics_with_cache(
                renderer, clean_line, self._fontproperties, ismath=ismath,
                dpi=self.get_figure(root=True).dpi)
            whd_list.append(whd)
        return whd_list

    # -- what pltrs (Rust) draws ------------------------------------------

    def _part_transform(self, axis_artist):
        return (axis_artist._axis_artist_helper.get_tick_transform(axis_artist._axes_of)
                + axis_artist.offset_transform)

    @staticmethod
    def _level(drawn):
        """The tick level of the tick labels *drawn* of an axis artist."""
        return drawn.name.split("_")[0]

    def _drawn_labels(self, drawn):
        """The tick labels the axis artist places now."""
        return drawn.axis_artist._tick_info(self._level(drawn))[1]

    def _texts(self):
        return [((float(x), float(y)), float(a), str(label)) for (x, y), a, label
                in self.__dict__.get("_labels", [])]

    def _part_text(self):
        return None

    def _part_extents(self, drawn):
        return _axisartist.ticklabel_extents(drawn.native, self._level(drawn))

    def _refresh(self, drawn):
        # matplotlib's draw leaves the text and position of the last
        # (non-empty) label it drew.
        if (drawn.axis_artist is None or not self.get_visible()
                or not drawn.drawn_since(self, "_placed_at")):
            return
        drawn_labels = [label for label in self._drawn_labels(drawn)
                        if label[2].strip()]
        if drawn_labels:
            (x, y), _, label = drawn_labels[-1]
            with _mirrors.detached(self):
                mtext.Text.set_x(self, x)
                mtext.Text.set_y(self, y)
                mtext.Text.set_text(self, label)


class _AxisLine:
    """The line of an axis artist (matplotlib's ``axis.line``)."""

    def __init__(self, axis_artist):
        self._axis_artist = axis_artist

    @property
    def _native(self):
        return self._axis_artist._native

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

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

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
        return self.label.get_pad()

    @LABELPAD.setter
    def LABELPAD(self, v):
        self.label.set_pad(v)

    def __init__(self, axes, helper, offset=None, axis_direction="bottom",
                 **kwargs):
        """
        Parameters
        ----------
        axes : `mpl_toolkits.axisartist.axislines.Axes`
        helper : `~mpl_toolkits.axisartist.axislines.AxisArtistHelper`
        """
        if offset is None:
            offset = (0, 0)
        # Unpacked as matplotlib's `ScaledTranslation(*offset, ...)` does.
        self._offset = (*offset,)
        native = helper._new_axis(axes, offset, axis_direction)
        # What matplotlib's `_init_line` computes (a curvilinear line).
        _axisartist.init_line(native)
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
        axis._offset = (0, 0)
        axis._axes_of = axes
        axis._init_parts(helper, axis_direction)
        return axis

    def _init_parts(self, helper, axis_direction):
        """Make the parts, as matplotlib's ``__init__`` makes them, for
        pltrs (Rust) to draw."""
        self._axis_artist_helper = helper
        bind = getattr(helper, "_bind", None)
        if bind is not None:
            bind(self._native)
        axes = self._axes_of
        # The axis whose ticks and label the parts follow by default.
        self.axis = axes.yaxis if axis_direction in ["left", "right"] else axes.xaxis
        self._axisline_style = None
        self._axis_direction = axis_direction
        self.line = _AxisLine(self)
        self._visible_parts = _axisartist.parts_visible(self._native)
        self._init_ticks()
        self._init_label()
        self.set_axis_direction(axis_direction)

    @property
    def offset_transform(self):
        """The transform moving the parts by the artist's offset, in
        points."""
        transform = self.__dict__.get("_offset_transform")
        if transform is None:
            transform = self._offset_transform = ScaledTranslation(
                *self._offset,
                Affine2D().scale(1 / 72)  # points to inches.
                + self._axes_of.get_figure(root=False).dpi_scale_trans)
        return transform

    @offset_transform.setter
    def offset_transform(self, transform):
        self._offset_transform = transform

    #: The parts in the order ``_native.axisartist.parts_visible`` says
    #: whether they are shown.
    _PARTS = ("major", "minor", "major_ticklabels", "minor_ticklabels", "label")

    def _attach(self, part, name):
        """Make *part* the mirror of the part *name* pltrs (Rust) draws,
        shown as pltrs (Rust) made it (an axes hides some), and hand it
        the part's state."""
        if not self._visible_parts[self._PARTS.index(name)]:
            part.set_visible(False)
        drawn = _Drawn(self._native, axis_artist=self, name=name,
                       figure_of=None if isinstance(part, Ticks) else self._axes_of)
        _mirrors.attach(part, drawn)
        part._restyle(drawn)
        return part

    def _init_ticks(self):
        axis_name = self.axis.axis_name
        self.major_ticks = self._attach(Ticks(
            ticksize=rcParams[f"{axis_name}tick.major.size"],
            tick_direction=rcParams[f"{axis_name}tick.direction"],
            axis=self.axis), "major")
        self.minor_ticks = self._attach(Ticks(
            ticksize=rcParams[f"{axis_name}tick.minor.size"],
            tick_direction=rcParams[f"{axis_name}tick.direction"],
            axis=self.axis), "minor")

        size = rcParams[f"{axis_name}tick.labelsize"]
        self.major_ticklabels = self._attach(TickLabels(
            axis=self.axis,
            axis_direction=self._axis_direction,
            fontsize=size,
            pad=rcParams[f"{axis_name}tick.major.pad"],
        ), "major_ticklabels")
        self.minor_ticklabels = self._attach(TickLabels(
            axis=self.axis,
            axis_direction=self._axis_direction,
            fontsize=size,
            pad=rcParams[f"{axis_name}tick.minor.pad"],
        ), "minor_ticklabels")

    def _init_label(self):
        label = AxisLabel(
            0, 0, "__from_axes__",
            color="auto",
            fontsize=rcParams['axes.labelsize'],
            fontweight=rcParams['axes.labelweight'],
            axis=self.axis,
            axis_direction=self._axis_direction,
        )
        label.set_pad(5)
        self.label = self._attach(label, "label")

    def _add_to_axes(self, axes, kind, autolim=True):
        native = _artist.adopt_into_axes(axes._native, self._native, "artist",
                                         False)
        self._added = True
        return _handles.adopt(self, native, axes.figure)

    @property
    def axes(self):
        """The `~.axes.Axes` the axis artist draws the axis of."""
        return self._axes_of

    @axes.setter
    def axes(self, new_axes):
        if new_axes is not None and new_axes is not self._axes_of:
            raise ValueError("Can not reset the Axes. You are probably trying "
                             "to reuse an artist in more than one Axes which "
                             "is not supported")

    def get_figure(self, root=False):
        # docstring inherited
        # As matplotlib's, an axis artist is in no figure until an axes adds
        # it with `add_artist`; those of an `axislines.Axes` never are.
        if not self.__dict__.get("_added"):
            return None
        return super().get_figure(root=root)

    # axis direction

    def set_axis_direction(self, axis_direction):
        """
        Adjust the direction, text angle, and text alignment of tick labels
        and axis labels following the Matplotlib convention for the rectangle
        axes.

        The *axis_direction* must be one of [left, right, bottom, top].

        =====================    ========== ========= ========== ==========
        Property                 left       bottom    right      top
        =====================    ========== ========= ========== ==========
        ticklabel direction      "-"        "+"       "+"        "-"
        axislabel direction      "-"        "+"       "+"        "-"
        ticklabel angle          90         0         -90        180
        ticklabel va             center     baseline  center     baseline
        ticklabel ha             right      center    right      center
        axislabel angle          180        0         0          180
        axislabel va             center     top       center     bottom
        axislabel ha             right      center    right      center
        =====================    ========== ========= ========== ==========

        Note that the direction "+" and "-" are relative to the direction of
        the increasing coordinate. Also, the text angles are actually
        relative to (90 + angle of the direction to the ticklabel),
        which gives 0 for bottom axis.

        Parameters
        ----------
        axis_direction : {"left", "bottom", "right", "top"}
        """
        self.major_ticklabels.set_axis_direction(axis_direction)
        self.label.set_axis_direction(axis_direction)
        self._axis_direction = axis_direction
        if axis_direction in ["left", "top"]:
            self.set_ticklabel_direction("-")
            self.set_axislabel_direction("-")
        else:
            self.set_ticklabel_direction("+")
            self.set_axislabel_direction("+")

    @property
    def _ticklabel_add_angle(self):
        return _ADD_ANGLES[_axisartist.get_label_directions(self._native)[0]]

    @_ticklabel_add_angle.setter
    def _ticklabel_add_angle(self, angle):
        _axisartist.set_ticklabel_direction(self._native, _DIRECTIONS[angle % 360])

    @property
    def _axislabel_add_angle(self):
        return _ADD_ANGLES[_axisartist.get_label_directions(self._native)[1]]

    @_axislabel_add_angle.setter
    def _axislabel_add_angle(self, angle):
        _axisartist.set_axislabel_direction(self._native, _DIRECTIONS[angle % 360])

    def set_ticklabel_direction(self, tick_direction):
        r"""
        Adjust the direction of the tick labels.

        Note that the *tick_direction*\s '+' and '-' are relative to the
        direction of the increasing coordinate.

        Parameters
        ----------
        tick_direction : {"+", "-"}
        """
        self._ticklabel_add_angle = _api.getitem_checked(
            _ADD_ANGLES, tick_direction=tick_direction)

    def invert_ticklabel_direction(self):
        self._ticklabel_add_angle = (self._ticklabel_add_angle + 180) % 360
        self.major_ticklabels.invert_axis_direction()
        self.minor_ticklabels.invert_axis_direction()

    def set_axislabel_direction(self, label_direction):
        r"""
        Adjust the direction of the axis label.

        Note that the *label_direction*\s '+' and '-' are relative to the
        direction of the increasing coordinate.

        Parameters
        ----------
        label_direction : {"+", "-"}
        """
        self._axislabel_add_angle = _api.getitem_checked(
            _ADD_ANGLES, label_direction=label_direction)

    def get_transform(self):
        return self._axes_of.transAxes + self.offset_transform

    def _get_tick_info(self, tick_iter):
        """
        Return a pair of:

        - list of locs and angles for ticks
        - list of locs, angles and labels for ticklabels.
        """
        ticks_loc_angle = []
        ticklabels_loc_angle_label = []

        ticklabel_add_angle = self._ticklabel_add_angle

        for loc, angle_normal, angle_tangent, label in tick_iter:
            angle_label = angle_tangent - 90 + ticklabel_add_angle
            angle_tick = (angle_normal
                          if 90 <= (angle_label - angle_normal) % 360 <= 270
                          else angle_normal + 180)
            ticks_loc_angle.append([loc, angle_tick])
            ticklabels_loc_angle_label.append([loc, angle_label, label])

        return ticks_loc_angle, ticklabels_loc_angle_label

    def _tick_info(self, level):
        """`_get_tick_info` of the ticks of *level* (``"major"`` or
        ``"minor"``) the helper places now, as matplotlib's
        ``_update_ticks`` hands them to the parts when it draws."""
        major, minor = self._axis_artist_helper.get_tick_iterators(self._axes_of)
        return self._get_tick_info(major if level == "major" else minor)

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
        return self._axisline_style

    def set_label(self, s):
        # docstring inherited
        self.label.set_text(s)

    def get_tightbbox(self, renderer=None):
        """Return the box of what the axis artist draws (its tick labels,
        label, offset text and line), None when it is hidden or draws
        nothing, as matplotlib's (pltrs (Rust) lays the parts out)."""
        if not self.get_visible():
            return None
        if renderer is None:
            # As matplotlib's, which asks its figure for a renderer.
            self.get_figure(root=True)._get_renderer()
        extent = _axisartist.tight_bbox(self._native)
        return None if extent is None else Bbox.from_extents(*extent)

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
        if all:
            _ticks, _ticklabels, _label = True, True, True
        elif all is not None:
            _ticks, _ticklabels, _label = False, False, False
        else:
            _ticks, _ticklabels, _label = None, None, None

        if ticks is not None:
            _ticks = ticks
        if ticklabels is not None:
            _ticklabels = ticklabels
        if label is not None:
            _label = label

        if _ticks is not None:
            self.major_ticks.set_visible(_ticks)
            self.minor_ticks.set_visible(_ticks)
        if _ticklabels is not None:
            self.major_ticklabels.set_visible(_ticklabels)
            self.minor_ticklabels.set_visible(_ticklabels)
        if _label is not None:
            self.label.set_visible(_label)


#: matplotlib's angles for the tick label and label directions.
_ADD_ANGLES = {"+": 0, "-": 180}
#: The direction of each angle.
_DIRECTIONS = {0: "+", 180: "-"}
