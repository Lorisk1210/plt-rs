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

import functools
from numbers import Real

import numpy as np

from . import (_api, _artist_clipping, _artist_effects, _live_transforms, _mirrors,
               _unusable, _values, cbook)
from ._native import artist as _artist, axis as _axis
from .artist import Artist, allow_rasterization
from .lines import Line2D
from .text import Text, _check_metrics_renderer
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


def _axis_of(owner):
    """The `.Axis` of the owner of an axis' part (a `Tick` or an
    `_AxisText`), None for a tick made alone."""
    return owner._tick_axis if isinstance(owner, Tick) else owner.axis


class _AxisPart(_mirrors.Mirroring):
    """A part of an axis a mirror stands for: a tick's line or label, the
    axis label or the offset text, read back by *owner* (a `Tick` or an
    `_AxisText`)."""

    def __init__(self, owner):
        self.owner = owner

    def push(self, artist):
        self.owner._push(artist)

    def window_extent(self, artist, renderer=None, dpi=None):
        if "_given_transform" in artist.__dict__ or isinstance(artist, Line2D):
            # A part given a transform of its own is drawn through it; a
            # tick's line holds matplotlib's data and transform, from
            # which `.Line2D` measures itself as matplotlib's does.
            return _mirrors.OWN
        if renderer is not None and artist.get_visible() and artist.get_text() != '':
            # matplotlib's `.Text.get_window_extent` lays the text out with
            # the renderer given.
            _check_metrics_renderer(renderer)
        extent = self.owner._extent(artist)
        if extent is None:
            # matplotlib's empty text measures as nothing at its position.
            return _mirrors.OWN if artist.get_visible() and artist.get_text() == '' else Bbox.unit()
        x0, y0, x1, y1 = extent
        return Bbox([[x0, y0], [x1, y1]])

    def tight_box(self, artist, renderer=None):
        # A label measures as matplotlib's `.Text.get_tightbbox`: as nothing
        # while it is hidden or empty.
        if not isinstance(artist, Line2D) and (
                not artist.get_visible() or artist.get_text() == ""):
            return Bbox.null()
        return super().tight_box(artist, renderer)

    def contains(self, artist, mouseevent):
        # A label (the axis', its offset text, a tick's) holds the pointer
        # within the box it was drawn in, as matplotlib's `.Text.contains`
        # tests; a line part tests itself.
        if isinstance(artist, Line2D):
            return _mirrors.OWN
        extent = self.owner._extent(artist)
        if extent is None or not artist.get_visible():
            return False, {}
        x0, y0, x1, y1 = extent
        return x0 <= mouseevent.x <= x1 and y0 <= mouseevent.y <= y1, {}

    def refresh(self, artist):
        self.owner._refresh(artist)

    def figure(self, artist, root=False):
        return _axis_of(self.owner).get_figure(root=root)

    def transform(self, artist):
        # A transform given to the part is its own.
        if "_given_transform" in artist.__dict__:
            return _mirrors.OWN
        return self.owner._transform_of(artist)

    def transform_set(self, artist):
        # An axis gives every part of it its transform when it makes it.
        return True

    def clip_box(self, artist):
        # matplotlib's Axes clip the gridlines of their axes to their patch.
        return self._patch_clip(artist, _artist_clipping.patch_clip_box)

    def clip_path(self, artist):
        return self._patch_clip(artist, _artist_clipping.patch_clip_path)

    def _patch_clip(self, artist, clip):
        """*clip* (`._artist_clipping.patch_clip_box` or ``patch_clip_path``)
        of the Axes' patch for the gridline of a tick of an axis; `OWN` for
        the other parts."""
        owner = self.owner
        if getattr(owner, "gridline", None) is not artist or _axis_of(owner) is None:
            return _mirrors.OWN
        return clip(artist, owner.axes)


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

    Notes
    -----
    Ticks are managed though the autoscaling / view limit mechanism and therefore
    need very careful handling when explicitly accessed, see :ref:`axes-tick-objects`.

    The tick lines are implemented as `~matplotlib.lines.Line2D` instances using the
    `~.matplotlib.markers` TICKLEFT, TICKRIGHT, TICKDOWN, TICKUP. Therefore,
    properties are controlled via the marker, e.g. for the color
    `.Line2D.set_markeredgecolor` and not `.Line2D.set_color`.
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
        # matplotlib's constructor gives the tick the figure of *axes*.
        axes.get_figure(root=False)
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
        for artist in [self.tick1line, self.tick2line, self.gridline,
                       self.label1, self.label2]:
            self._set_artist_props(artist)
        if loc is not None:
            # A tick made at no location (``loc=None``) is where its class
            # makes it, as matplotlib's ``__init__`` replaces what
            # ``update_position(None)`` left.
            self.update_position(loc)

    def _set_artist_props(self, a):
        a.set_figure(self.get_figure(root=False))

    def _take_axes_transforms(self, name):
        """Give the parts of a tick made alone the transforms matplotlib's
        tick of the *name* axis (``'x'`` or ``'y'``) takes of its Axes when
        it is made: those of its lines (``get_xaxis_transform``) and, with
        their alignments, of its labels (`_get_text1_transform`, which a
        tick class of the program's overrides)."""
        transform = getattr(self.axes, f"get_{name}axis_transform")
        self.tick1line.set_transform(transform("tick1"))
        self.tick2line.set_transform(transform("tick2"))
        self.gridline.set_transform(transform("grid"))
        for label, (trans, va, ha) in ((self.label1, self._get_text1_transform()),
                                       (self.label2, self._get_text2_transform())):
            label.set(verticalalignment=va, horizontalalignment=ha, transform=trans)

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
        tick._drawn = None
        for part in tick.get_children():
            _mirrors.attach(part, _AxisPart(tick))
        tick._sync()
        return tick

    @property
    def _tick_axis(self):
        """The `.Axis` the tick belongs to, None for one made alone."""
        return self.__dict__.get("_axis")

    def _get_figure(self, root):
        """Return the figure of the tick's Axes, which matplotlib's ticks
        join when they are made."""
        axes = self.axes
        if axes is None:
            return super()._get_figure(root)
        if self._tick_axis is None:
            # A tick made by its constructor keeps the figure it joined
            # then, also once its Axes left it, as matplotlib's does.
            return axes._home_figure(root)
        return axes.get_figure(root=root)

    def _sync(self):
        """Fill the parts of a tick of an axis from the axis."""
        axis = self._tick_axis
        if axis is None:
            return
        parts = (self.label1, self.label2, self.tick1line, self.tick2line,
                 self.gridline)
        (loc, self._size, self._width, self._base_pad, self._tickdir,
         self._labelrotation) = _axis.sync_tick(
            axis.axes._native, axis.axis_name, not self._major, self._index,
            tuple(part._native for part in parts))
        # matplotlib's update_position gives the tick and its labels its
        # location as the numpy float of the locator's array; a tick never
        # placed sits at the int 0 it was made at.
        self._loc = 0 if loc is None else np.float64(loc)
        coordinate = "y" if self.__name__ == "ytick" else "x"
        for label in (self.label1, self.label2):
            _values.remember(label, coordinate, self._loc)
        if axis.axes.name != "3d":
            self._place_lines(None if loc is None else self._loc)
        self._drawn = _artist.draw_count(axis.axes._native)

    def _place_lines(self, loc):
        """
        Give the lines of a tick of an axis the data matplotlib's tick
        holds: the data its class makes them with (``_made_data``), until
        ``update_position`` places the tick at *loc* (None for a tick never
        placed), which becomes ``(loc,)`` along the axis. Their native
        lines keep what pltrs (Rust) draws. The ticks of a class of the
        program's deriving from neither `XTick` nor `YTick` keep the lines
        their class makes.
        """
        made_data = getattr(type(self), "_made_data", None)
        if made_data is None:
            return
        for line, made in zip((self.tick1line, self.tick2line, self.gridline),
                              made_data):
            data = [list(values) for values in made]
            if loc is not None:
                data[self._along] = (loc,)
            line._xorig, line._yorig = data

    def _refresh(self, part):
        """Give the parts of a tick of an axis drawn since they were
        filled what matplotlib's draw gives them (see `_follow_draw`)."""
        self._follow_draw()

    def _follow_draw(self):
        """
        Give a tick of an axis drawn since it was filled what matplotlib's
        draw gives it (``_update_ticks``): its location along the axis,
        its labels that location and the formatter's text and its lines
        that location, all else they have staying.
        """
        axis = self._tick_axis
        if axis is None:
            return
        drawn = _artist.draw_count(axis.axes._native)
        if drawn == self._drawn:
            return
        self._drawn = drawn
        _unusable.drop(self, "loc")
        located = _axis.drawn_tick_label(axis.axes._native, axis.axis_name,
                                         not self._major, self._index)
        if located is None:
            # A tick of a 3-D axis, whose draw places its parts anew.
            self._sync()
            return
        loc, text = located
        self._loc = 0 if loc is None else np.float64(loc)
        coordinate = "y" if self.__name__ == "ytick" else "x"
        for label in (self.label1, self.label2):
            with _mirrors.detached(label):
                label.set(text=text, **{coordinate: self._loc})
        self._place_lines(None if loc is None else self._loc)

    def _retire(self):
        """
        Leave a tick of an axis as matplotlib's `.Axis.clear` and
        ``reset_ticks`` leave the tick objects they drop: no longer of its
        axis, its parts artists of their own that keep what they had (the
        text and the location of the last draw), placed by the transforms
        the axis gave them, in the figure of its Axes.
        """
        axis = self._tick_axis
        if axis is None:
            return
        self._follow_draw()
        figure = axis.get_figure(root=False)
        from . import spines
        for part in self.get_children():
            # The transform the tick was made with: taking it now asks
            # nothing of the spines (`.Spine._ensure_position_is_set`).
            with spines._transforms_read():
                transform = self._transform_of(part)
            del part._mirror
            part.set_transform(transform)
            if figure is not None:
                part.set_figure(figure)
        self._axis = None

    def _push(self, part):
        """Read *part*, changed, back into the axis."""
        axis = self._tick_axis
        args = (axis.axes._native, axis.axis_name, not self._major,
                self._index)
        if part is self.label1 or part is self.label2:
            _axis.push_tick_label(*args, part is self.label2, part._native)
            if "_path_effects" in part.__dict__:
                # Those the label was given, in place of the rcParam's.
                _axis.push_tick_label_effects(*args, part is self.label2, part._native)
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

    def set_clip_path(self, path, transform=None):
        # docstring inherited
        super().set_clip_path(path, transform)

    def get_clip_box(self):
        """Return the clipbox: that of the Axes' patch for a tick of an
        axis, as matplotlib's Axes clip their axes (`.Axis.get_clip_box`)."""
        if "_clipbox" in self.__dict__ or self._tick_axis is None:
            return super().get_clip_box()
        return _artist_clipping.patch_clip_box(self, self.axes)

    def get_clip_path(self):
        """Return the clip path: that of the Axes' patch for a tick of an
        axis, as matplotlib's Axes clip their axes (`.Axis.get_clip_path`)."""
        if "_clippath" in self.__dict__ or self._tick_axis is None:
            return super().get_clip_path()
        return _artist_clipping.patch_clip_path(self, self.axes)

    def _extent(self, part):
        """The display box of the label *part* as drawn, if it is."""
        if part is not self.label1 and part is not self.label2:
            return None
        axis = self._tick_axis
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
        axis = self._tick_axis
        return axis.axes._native, axis.axis_name, not self._major, self._index

    def get_gid(self):
        """Return the group id of the tick."""
        if self._tick_axis is None:
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
        if self._tick_axis is None:
            super().set_gid(gid)
        else:
            _axis.set_tick_gid(*self._gid_args(),
                               None if gid is None else str(gid))

    def _drawn_by_its_axis(self):
        """Whether pltrs (Rust) draws the tick with the 2-D axis it is a
        tick of."""
        axis = self._tick_axis
        return axis is not None and axis.axes.name != "3d"

    def set_visible(self, b):
        # docstring inherited
        super().set_visible(b)
        if self._drawn_by_its_axis():
            # matplotlib's `Tick.draw` draws nothing of a hidden tick.
            _axis.set_tick_visible(*self._gid_args(), bool(self.get_visible()))

    def set_agg_filter(self, filter_func):
        # docstring inherited
        super().set_agg_filter(filter_func)
        if self._drawn_by_its_axis():
            # matplotlib's `allow_rasterization` filters the tick's draw.
            _axis.set_tick_agg_filter(
                *self._gid_args(), None if filter_func is None else
                functools.partial(_artist_effects.filtered, filter_func))

    @allow_rasterization
    def draw(self, renderer):
        # Draw the visible parts of a tick made alone, as matplotlib's does
        # (how the projection of a program's class sees the parts its ticks
        # draw). The ticks of an axis are drawn by pltrs (Rust) with their
        # axis.
        if self.get_visible():
            _unusable.check(self)
        if self.get_visible() and self._tick_axis is None:
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
        axis = self._tick_axis
        if axis is not None:
            _axis.push_tick_pad(axis.axes._native, axis.axis_name,
                                not self._major, self._index, float(val))

    def get_pad(self):
        """Get the value of the tick label pad in points."""
        return self._base_pad

    def update_position(self, loc):
        """Set the location of tick in data coords with scalar *loc*."""
        raise NotImplementedError('Derived must override')

    def _get_text1_transform(self):
        raise NotImplementedError('Derived must override')

    def _get_text2_transform(self):
        raise NotImplementedError('Derived must override')

    def _keeps_location(self, loc):
        """
        Whether the location *loc* is no number, which the tick keeps as
        matplotlib's does, its parts staying where they are: drawing the
        tick then fails as matplotlib's lines fail to read it, until its
        axis places it again (see `._unusable`). Otherwise the tick takes
        it, as `update_position` places it.
        """
        if isinstance(loc, Real):
            _unusable.drop(self, "loc")
            return False
        _unusable.keep(self, "loc", loc, float, drawn=False)
        self._loc = loc
        return True

    def get_view_interval(self):
        """
        Return the view limits ``(min, max)`` of the axis the tick belongs to.
        """
        raise NotImplementedError('Derived must override')

    def get_loc(self):
        """Return the tick location (data coords) as a scalar."""
        self._follow_draw()
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
            # matplotlib places the labels at the new pad (its
            # ``_get_text1_transform``), failing on a pad it cannot add.
            self._pad  # noqa: B018
            if self._tick_axis is None:
                # A tick made alone holds its labels' transforms itself.
                self.label1.set_transform(self._get_text1_transform()[0])
                self.label2.set_transform(self._get_text2_transform()[0])
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
    #: The data of the tick1line, tick2line and gridline as the tick is
    #: made (x in data coords, y in axes coords), and the coordinate
    #: `update_position` moves.
    _made_data = (([0], [0]), ([0], [1]), ([0, 0], [0, 1]))
    _along = 0

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # The tick is made at 0 whatever its location, as matplotlib's is.
        for line, (x, y) in zip((self.tick1line, self.tick2line, self.gridline),
                                self._made_data):
            line.set_data(x, y)
        self.label1.set(x=0, y=0)
        self.label2.set(x=0, y=1)
        self._take_axes_transforms("x")

    def _get_text1_transform(self):
        return self.axes.get_xaxis_text1_transform(self._pad)

    def _get_text2_transform(self):
        return self.axes.get_xaxis_text2_transform(self._pad)

    def update_position(self, loc):
        """Set the location of tick in data coords with scalar *loc*."""
        if self._keeps_location(loc):
            return
        self.tick1line.set_xdata((loc,))
        self.tick2line.set_xdata((loc,))
        self.gridline.set_xdata((loc,))
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
    #: The data of the tick1line, tick2line and gridline as the tick is
    #: made (x in axes coords, y in data coords), and the coordinate
    #: `update_position` moves.
    _made_data = (([0], [0]), ([1], [0]), ([0, 1], [0, 0]))
    _along = 1

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # The tick is made at 0 whatever its location, as matplotlib's is.
        for line, (x, y) in zip((self.tick1line, self.tick2line, self.gridline),
                                self._made_data):
            line.set_data(x, y)
        self.label1.set(x=0, y=0)
        self.label2.set(x=1, y=0)
        self._take_axes_transforms("y")

    def _get_text1_transform(self):
        return self.axes.get_yaxis_text1_transform(self._pad)

    def _get_text2_transform(self):
        return self.axes.get_yaxis_text2_transform(self._pad)

    def update_position(self, loc):
        """Set the location of tick in data coords with scalar *loc*."""
        if self._keeps_location(loc):
            return
        self.tick1line.set_ydata((loc,))
        self.tick2line.set_ydata((loc,))
        self.gridline.set_ydata((loc,))
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

    def _refresh(self, text):
        """Drawing the axis leaves its label and offset text as they
        are."""

    #: The native functions filling the mirror, reading it back and giving
    #: its extent, and the one giving the text the path effects its mirror
    #: was given (None: the text is drawn through the rcParam's).
    _sync_native = _push_native = _extent_native = None
    _push_effects_native = None
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
        push_effects = type(self)._push_effects_native
        if "_path_effects" in text.__dict__ and push_effects is not None:
            # Those the text was given, in place of the rcParam's.
            push_effects(*self._args(), text._native)

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
    _push_effects_native = _axis.push_axis_label_effects
    _extent_native = _axis.axis_label_extent
    _START = {"x": (0.5, 0), "y": (0, 0.5)}


class OffsetText(_AxisText):
    """The mirror of the offset (or scale) text of an axis."""

    _sync_native = _axis.sync_offset_text
    _push_native = _axis.push_offset_text
    _extent_native = _axis.offset_text_extent
    _START = {"x": (1, 0), "y": (0, 0.5)}
