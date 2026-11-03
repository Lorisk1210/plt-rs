"""
The artists of an axis (matplotlib's `.Tick`, `.XTick` and `.YTick`, and
the axis' ``label`` and ``offsetText``).

pltrs (Rust) draws an axis from its state, not from artists. The ticks of
an `.Axis` (``axis.majorTicks``, ``get_major_ticks()``,
``get_ticklabels()``), its label and its offset text are therefore
*mirrors*: loose `.Text` and `.Line2D` artists that ``_native.axis`` fills
from the axis when they are handed out, so that every getter works, and
reads back into the axis after any of their setters ran (``push``). A
property set on one tick stays that tick's own, as in matplotlib; a `Tick`
made with its constructor is an ordinary artist outside any axis.

Kept close to matplotlib 3.11.2's ``axis`` (Matplotlib License).
"""

import numpy as np

from . import _api, _artist_clipping, _live_transforms, _mirrors, _values, cbook
from ._native import axis as _axis
from .artist import Artist
from .lines import Line2D
from .text import Text
from .transforms import Bbox, IdentityTransform, blended_transform_factory


def _rc(key):
    from . import rcParams
    return rcParams[key]


def _val_or_rc(val, *keys):
    """*val*, or the first of the rcParams *keys* that is not None."""
    if val is not None:
        return val
    for key in keys:
        value = _rc(key)
        if value is not None:
            return value
    return None


class _AxisPart(_mirrors.Mirroring):
    """A part of an axis a mirror stands for: a tick's line or label, the
    axis label or the offset text, read back by *owner* (a `Tick` or an
    `_AxisText`)."""

    def __init__(self, owner):
        self.owner = owner

    def push(self, artist):
        self.owner._push(artist)

    def window_extent(self, artist, renderer=None, dpi=None):
        extent = self.owner._extent(artist)
        if extent is None:
            return Bbox.unit()
        x0, y0, x1, y1 = extent
        return Bbox([[x0, y0], [x1, y1]])

    def figure(self, artist, root=False):
        return self.owner.axis.get_figure(root=root)

    def transform(self, artist):
        # A transform given to the part is its own.
        if "_given_transform" in artist.__dict__:
            return _mirrors.OWN
        return self.owner._transform_of(artist)

    def transform_set(self, artist):
        # An axis gives every part of it its transform when it makes it.
        return True


class Tick(Artist):
    """
    Abstract base class for the axis ticks, grid lines and labels.

    Ticks mark a position on an Axis. They contain two lines as markers and
    two labels; one each for the bottom and top positions (in case of an
    `.XAxis`) or for the left and right positions (in case of a `.YAxis`).

    Attributes
    ----------
    tick1line : `~matplotlib.lines.Line2D`
        The left/bottom tick marker.
    tick2line : `~matplotlib.lines.Line2D`
        The right/top tick marker.
    gridline : `~matplotlib.lines.Line2D`
        The grid line associated with the label position.
    label1 : `~matplotlib.text.Text`
        The left/bottom tick label.
    label2 : `~matplotlib.text.Text`
        The right/top tick label.
    """

    def __init__(
        self, axes, loc, *,
        size=None,  # points
        width=None,
        color=None,
        tickdir=None,
        pad=None,
        labelsize=None,
        labelcolor=None,
        labelfontfamily=None,
        zorder=None,
        gridOn=None,  # defaults to axes.grid depending on axes.grid.which
        tick1On=True,
        tick2On=True,
        label1On=True,
        label2On=False,
        major=True,
        labelrotation=0,
        labelrotation_mode=None,
        grid_color=None,
        grid_linestyle=None,
        grid_linewidth=None,
        grid_alpha=None,
        **kwargs,  # Other Line2D kwargs applied to gridlines.
    ):
        """
        Parameters
        ----------
        loc
            The tick location in data coords.
        size
            The tick size in points.
        """
        from . import colors as mcolors
        super().__init__()
        if gridOn is None:
            which = _rc('axes.grid.which')
            if major and which in ('both', 'major'):
                gridOn = _rc('axes.grid')
            elif not major and which in ('both', 'minor'):
                gridOn = _rc('axes.grid')
            else:
                gridOn = False
        self.axes = axes
        self._loc = loc
        self._major = major
        name = self.__name__
        major_minor = "major" if major else "minor"
        self._size = _val_or_rc(size, f"{name}.{major_minor}.size")
        self._width = _val_or_rc(width, f"{name}.{major_minor}.width")
        self._base_pad = _val_or_rc(pad, f"{name}.{major_minor}.pad")
        color = _val_or_rc(color, f"{name}.color")
        labelcolor = _val_or_rc(labelcolor, f"{name}.labelcolor")
        if cbook._str_equal(labelcolor, 'inherit'):
            labelcolor = _rc(f"{name}.color")
        labelsize = _val_or_rc(labelsize, f"{name}.labelsize")
        self._set_labelrotation(labelrotation)
        if zorder is None:
            zorder = Line2D.zorder + 0.01 if major else Line2D.zorder
        self._zorder = zorder
        grid_color = _val_or_rc(grid_color, f"grid.{major_minor}.color",
                                "grid.color")
        grid_linestyle = _val_or_rc(grid_linestyle,
                                    f"grid.{major_minor}.linestyle",
                                    "grid.linestyle")
        grid_linewidth = _val_or_rc(grid_linewidth,
                                    f"grid.{major_minor}.linewidth",
                                    "grid.linewidth")
        if grid_alpha is None:
            if mcolors._has_alpha_channel(grid_color):
                rgba = mcolors.to_rgba(grid_color)
                grid_color, grid_alpha = rgba[:3], rgba[3]
            else:
                grid_alpha = _val_or_rc(None, f"grid.{major_minor}.alpha",
                                        "grid.alpha")
        grid_kw = {k[5:]: v for k, v in kwargs.items() if k != "rotation_mode"}
        self.tick1line = Line2D(
            [], [], color=color, linestyle="none", zorder=zorder,
            visible=tick1On, markeredgecolor=color, markersize=self._size,
            markeredgewidth=self._width)
        self.tick2line = Line2D(
            [], [], color=color, linestyle="none", zorder=zorder,
            visible=tick2On, markeredgecolor=color, markersize=self._size,
            markeredgewidth=self._width)
        self.gridline = Line2D(
            [], [], color=grid_color, alpha=grid_alpha, visible=gridOn,
            linestyle=grid_linestyle, linewidth=grid_linewidth, marker="",
            **grid_kw)
        self.label1 = Text(
            np.nan, np.nan, fontsize=labelsize, color=labelcolor,
            visible=label1On, fontfamily=labelfontfamily,
            rotation=self._labelrotation[1], rotation_mode=labelrotation_mode)
        self.label2 = Text(
            np.nan, np.nan, fontsize=labelsize, color=labelcolor,
            visible=label2On, fontfamily=labelfontfamily,
            rotation=self._labelrotation[1], rotation_mode=labelrotation_mode)
        self._apply_tickdir(tickdir)

    @classmethod
    def _of_axis(cls, axis, major, index):
        """The tick at *index* among the major (or minor) ticks of *axis*,
        its parts mirroring what pltrs (Rust) draws."""
        tick = cls.__new__(cls)
        Artist.__init__(tick)
        tick.axes = axis.axes
        tick._axis = axis
        tick._major = major
        tick._index = index
        tick._zorder = Line2D.zorder + 0.01 if major else Line2D.zorder
        tick.tick1line = Line2D([], [])
        tick.tick2line = Line2D([], [])
        tick.gridline = Line2D([], [])
        tick.label1 = Text(0, 0, '')
        tick.label2 = Text(0, 0, '')
        for part in tick.get_children():
            _mirrors.attach(part, _AxisPart(tick))
        tick._sync()
        return tick

    @property
    def axis(self):
        """The `.Axis` the tick belongs to, None for one made alone."""
        return self.__dict__.get("_axis")

    def get_figure(self, root=False):
        """Return the figure of the tick's Axes, which matplotlib's ticks
        join when they are made."""
        axes = self.axes
        return super().get_figure(root) if axes is None else axes.get_figure(root=root)

    def _sync(self):
        """Fill the parts of a tick of an axis from the axis."""
        axis = self.axis
        if axis is None:
            return
        parts = (self.label1, self.label2, self.tick1line, self.tick2line,
                 self.gridline)
        (self._loc, self._size, self._width, self._base_pad, self._tickdir,
         self._labelrotation) = _axis.sync_tick(
            axis.axes._native, axis.axis_name, not self._major, self._index,
            tuple(part._native for part in parts))
        # matplotlib's update_position gives the labels the tick's location
        # as the numpy float of the locator's array.
        coordinate = "y" if self.__name__ == "ytick" else "x"
        for label in (self.label1, self.label2):
            _values.remember(label, coordinate, np.float64(self._loc))

    def _push(self, part):
        """Read *part*, changed, back into the axis."""
        axis = self.axis
        args = (axis.axes._native, axis.axis_name, not self._major,
                self._index)
        if part is self.label1 or part is self.label2:
            _axis.push_tick_label(*args, part is self.label2, part._native)
        elif part is self.gridline:
            _axis.push_gridline(*args, part._native)
        else:
            _axis.push_tick_mark(*args, part is self.tick2line, part._native)

    def _transform_of(self, part):
        """
        matplotlib's transform of the part *part* of the tick: the axis
        transforms of its lines (``get_xaxis_transform('tick1')``...), the
        text transforms of its labels at the tick's pad
        (``get_xaxis_text1_transform``...), each kept while the pad stays.
        """
        axes = self.axes
        prefix = "y" if self.__name__ == "ytick" else "x"
        if part is self.label1 or part is self.label2:
            make = getattr(axes, f"get_{prefix}axis_text"
                                 f"{1 if part is self.label1 else 2}_transform")
            pad = self._pad
            return _live_transforms.kept(part, "transform", pad,
                                         lambda: make(pad)[0])
        which = ("tick1" if part is self.tick1line
                 else "tick2" if part is self.tick2line else "grid")
        return _live_transforms.kept(
            part, "transform", which,
            lambda: getattr(axes, f"get_{prefix}axis_transform")(which))

    def get_clip_box(self):
        """Return the clipbox: that of the Axes' patch for a tick of an
        axis, as matplotlib's Axes clip their axes (`.Axis.get_clip_box`)."""
        if "_clipbox" in self.__dict__ or self.axis is None:
            return super().get_clip_box()
        return _artist_clipping.patch_clip_box(self, self.axes)

    def _extent(self, part):
        """The display box of the label *part* as drawn, if it is."""
        if part is not self.label1 and part is not self.label2:
            return None
        axis = self.axis
        return _axis.tick_label_extent(axis.axes._native, axis.axis_name,
                                       not self._major, self._index,
                                       part is self.label2)

    def _set_labelrotation(self, labelrotation):
        if isinstance(labelrotation, str):
            mode = labelrotation
            angle = 0
        elif isinstance(labelrotation, (tuple, list)):
            mode, angle = labelrotation
        else:
            mode = 'default'
            angle = labelrotation
        _api.check_in_list(['auto', 'default'], labelrotation=mode)
        self._labelrotation = (mode, angle)

    @property
    def _pad(self):
        return self._base_pad + self.get_tick_padding()

    def _apply_tickdir(self, tickdir):
        """Set tick direction.  Valid values are 'out', 'in', 'inout'."""
        tickdir = _val_or_rc(tickdir, f'{self.__name__}.direction')
        _api.check_in_list(['in', 'out', 'inout'], tickdir=tickdir)
        self._tickdir = tickdir

    def get_tickdir(self):
        return self._tickdir

    def get_tick_padding(self):
        """Get the length of the tick outside of the Axes."""
        padding = {
            'in': 0.0,
            'inout': 0.5,
            'out': 1.0
        }
        return self._size * padding[self._tickdir]

    def get_children(self):
        children = [self.tick1line, self.tick2line,
                    self.gridline, self.label1, self.label2]
        return children

    def _gid_args(self):
        """The native axes, axis name, level and index of a tick of an
        axis."""
        axis = self.axis
        return axis.axes._native, axis.axis_name, not self._major, self._index

    def get_gid(self):
        """Return the group id of the tick."""
        if self.axis is None:
            return super().get_gid()
        return _axis.get_tick_gid(*self._gid_args())

    def set_gid(self, gid):
        """
        Set the (group) id of the tick: SVG output names the group holding
        the tick's lines and labels after it.

        Parameters
        ----------
        gid : str
        """
        if self.axis is None:
            super().set_gid(gid)
        else:
            _axis.set_tick_gid(*self._gid_args(),
                               None if gid is None else str(gid))

    def draw(self, renderer):
        """
        Draw the visible parts of a tick made alone, as matplotlib's does
        (how the projection of a program's class sees the parts its ticks
        draw). The ticks of an axis are drawn by pltrs (Rust) with their
        axis.
        """
        if self.get_visible() and self.axis is None:
            renderer.open_group(self.__name__, gid=self.get_gid())
            for artist in [self.gridline, self.tick1line, self.tick2line,
                           self.label1, self.label2]:
                artist.draw(renderer)
            renderer.close_group(self.__name__)
        self.stale = False

    def contains(self, mouseevent):
        """
        Test whether the mouse event occurred in the Tick marks.

        This function always returns false.  It is more useful to test if the
        axis as a whole contains the mouse rather than the set of tick marks.
        """
        return False, {}

    def set_pad(self, val):
        """
        Set the tick label pad in points

        Parameters
        ----------
        val : float
        """
        self._apply_params(pad=val)
        axis = self.axis
        if axis is not None:
            _axis.push_tick_pad(axis.axes._native, axis.axis_name,
                                not self._major, self._index, float(val))

    def get_pad(self):
        """Get the value of the tick label pad in points."""
        return self._base_pad

    def update_position(self, loc):
        """Set the location of tick in data coords with scalar *loc*."""
        raise NotImplementedError('Derived must override')

    def get_view_interval(self):
        """
        Return the view limits ``(min, max)`` of the axis the tick belongs to.
        """
        raise NotImplementedError('Derived must override')

    def get_loc(self):
        """Return the tick location (data coords) as a scalar."""
        return self._loc

    def set_url(self, url):
        """
        Set the url of label1 and label2.

        Parameters
        ----------
        url : str
        """
        super().set_url(url)
        self.label1.set_url(url)
        self.label2.set_url(url)

    def _apply_params(self, **kwargs):
        for name, target in [("gridOn", self.gridline),
                             ("tick1On", self.tick1line),
                             ("tick2On", self.tick2line),
                             ("label1On", self.label1),
                             ("label2On", self.label2)]:
            if name in kwargs:
                target.set_visible(kwargs.pop(name))
        if any(k in kwargs for k in ['size', 'width', 'pad', 'tickdir']):
            self._size = kwargs.pop('size', self._size)
            self._width = kwargs.pop('width', self._width)
            self._base_pad = kwargs.pop('pad', self._base_pad)
            self._apply_tickdir(kwargs.pop('tickdir', self._tickdir))
            for line in (self.tick1line, self.tick2line):
                line.set_markersize(self._size)
                line.set_markeredgewidth(self._width)
        tick_kw = {k: v for k, v in kwargs.items() if k in ['color', 'zorder']}
        if 'color' in kwargs:
            tick_kw['markeredgecolor'] = kwargs['color']
        self.tick1line.set(**tick_kw)
        self.tick2line.set(**tick_kw)
        for k, v in tick_kw.items():
            setattr(self, '_' + k, v)
        if 'labelrotation' in kwargs:
            self._set_labelrotation(kwargs.pop('labelrotation'))
            self.label1.set(rotation=self._labelrotation[1])
            self.label2.set(rotation=self._labelrotation[1])
        label_kw = {k[5:]: v for k, v in kwargs.items()
                    if k in ['labelsize', 'labelcolor', 'labelfontfamily',
                             'labelrotation_mode']}
        self.label1.set(**label_kw)
        self.label2.set(**label_kw)
        grid_kw = {k[5:]: v for k, v in kwargs.items()
                   if k.startswith('grid_')}
        self.gridline.set(**grid_kw)



class XTick(Tick):
    """
    Contains all the Artists needed to make an x tick - the tick line,
    the label text and the grid line
    """
    __name__ = 'xtick'

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # x in data coords, y in axes coords; a tick of no location yet
        # (``loc=None``) is at NaN.
        loc = np.nan if self._loc is None else self._loc
        self.tick1line.set_data([loc], [0])
        self.tick2line.set_data([loc], [1])
        self.gridline.set_data([loc, loc], [0, 1])
        self.label1.set(x=loc, y=0, verticalalignment="top",
                        horizontalalignment="center")
        self.label2.set(x=loc, y=1, verticalalignment="bottom",
                        horizontalalignment="center")

    def update_position(self, loc):
        """Set the location of tick in data coords with scalar *loc*."""
        self.tick1line.set_xdata((loc,))
        self.tick2line.set_xdata((loc,))
        self.gridline.set_xdata((loc, loc))
        self.label1.set_x(loc)
        self.label2.set_x(loc)
        self._loc = loc

    def get_view_interval(self):
        # docstring inherited
        return self.axes.viewLim.intervalx

    def _apply_tickdir(self, tickdir):
        # docstring inherited
        from . import lines as mlines
        super()._apply_tickdir(tickdir)
        mark1, mark2 = {
            'out': (mlines.TICKDOWN, mlines.TICKUP),
            'in': (mlines.TICKUP, mlines.TICKDOWN),
            'inout': ('|', '|'),
        }[self._tickdir]
        self.tick1line.set_marker(mark1)
        self.tick2line.set_marker(mark2)


class YTick(Tick):
    """
    Contains all the Artists needed to make a Y tick - the tick line,
    the label text and the grid line
    """
    __name__ = 'ytick'

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # x in axes coords, y in data coords; a tick of no location yet
        # (``loc=None``) is at NaN.
        loc = np.nan if self._loc is None else self._loc
        self.tick1line.set_data([0], [loc])
        self.tick2line.set_data([1], [loc])
        self.gridline.set_data([0, 1], [loc, loc])
        self.label1.set(x=0, y=loc, verticalalignment="center_baseline",
                        horizontalalignment="right")
        self.label2.set(x=1, y=loc, verticalalignment="center_baseline",
                        horizontalalignment="left")

    def update_position(self, loc):
        """Set the location of tick in data coords with scalar *loc*."""
        self.tick1line.set_ydata((loc,))
        self.tick2line.set_ydata((loc,))
        self.gridline.set_ydata((loc, loc))
        self.label1.set_y(loc)
        self.label2.set_y(loc)
        self._loc = loc

    def get_view_interval(self):
        # docstring inherited
        return self.axes.viewLim.intervaly

    def _apply_tickdir(self, tickdir):
        # docstring inherited
        from . import lines as mlines
        super()._apply_tickdir(tickdir)
        mark1, mark2 = {
            'out': (mlines.TICKLEFT, mlines.TICKRIGHT),
            'in': (mlines.TICKRIGHT, mlines.TICKLEFT),
            'inout': ('_', '_'),
        }[self._tickdir]
        self.tick1line.set_marker(mark1)
        self.tick2line.set_marker(mark2)


class _AxisText:
    """The mirror of a text of an axis (its label or its offset text)."""

    #: The native functions filling the mirror, reading it back and giving
    #: its extent.
    _sync_native = _push_native = _extent_native = None
    #: The position matplotlib's axes of each name give the text when they
    #: make it (``self.label.set(x=0.5, y=0)``...), by axis name.
    _START = {}

    def __init__(self, axis):
        self.axis = axis
        self.text = _mirrors.attach(
            Text(*self._START.get(axis.axis_name, (0, 0)), ''), _AxisPart(self))

    def _args(self):
        return self.axis.axes._native, self.axis.axis_name

    def _synced(self):
        """The mirror, filled from the axis."""
        type(self)._sync_native(*self._args(), self.text._native)
        return self.text

    def _push(self, text):
        type(self)._push_native(*self._args(), text._native)

    def _extent(self, text):
        return type(self)._extent_native(*self._args())

    def _transform_of(self, text):
        """matplotlib's transform of the text: x in the Axes' fractions and
        y in pixels for an x-axis (y, x for a y-axis label), as its axis
        makes it."""
        axes = self.axis.axes
        return _live_transforms.kept(
            text, "transform", None,
            lambda: blended_transform_factory(axes.transAxes, IdentityTransform()))


class AxisLabel(_AxisText):
    """The mirror of an axis label."""

    def _transform_of(self, text):
        # The transform `.Axis.set_label_coords` gave, else the default.
        given = self.axis.__dict__.get("_label_transform")
        if given is not None:
            return given
        if self.axis.axis_name != "y":
            return super()._transform_of(text)
        axes = self.axis.axes
        return _live_transforms.kept(
            text, "transform", None,
            lambda: blended_transform_factory(IdentityTransform(), axes.transAxes))

    _sync_native = _axis.sync_axis_label
    _push_native = _axis.push_axis_label
    _extent_native = _axis.axis_label_extent
    _START = {"x": (0.5, 0), "y": (0, 0.5)}


class OffsetText(_AxisText):
    """The mirror of the offset (or scale) text of an axis."""

    _sync_native = _axis.sync_offset_text
    _push_native = _axis.push_offset_text
    _extent_native = _axis.offset_text_extent
    _START = {"x": (1, 0), "y": (0, 0.5)}
