"""
`_AxesBase`, the base of `~.axes.Axes` (matplotlib's ``axes._base``): what
an axes is apart from its plotting methods.

Ready-made artists join an axes here (``add_artist``, ``add_line``,
``add_patch``, ``add_collection``, ``add_image``): pltrs
(Rust) adopts the artist into the axes' figure as matplotlib's method of
that name adds it (`.Artist._add_to_axes`, `.._handles.adopt`).
"""

from .. import _api, _handles, _live_transforms
from .._native import axes_base as _axes_base, gridspec as _gridspec
from .._native import transforms as _transforms
from ..artist import Artist
from ._aspect import AspectMixin
from ._blitting import BlittingMixin
from ._construction import ConstructionMixin
from ._decorations import DecorationsMixin
from ._layout import LayoutMixin
from ._limits import LimitsMixin
from ._navigation import NavigationMixin
from ._projection import ProjectionMixin
from ._ticks import TicksMixin
from ._units import UnitsMixin
from ..transforms import Bbox


class _SharedAxes:
    """
    The groups of axes sharing one axis (matplotlib's ``_shared_axes[name]``,
    a `.cbook.Grouper`), as pltrs (Rust) keeps them.
    """

    def __init__(self, name):
        self._name = name

    def _siblings(self, a):
        return _handles.wrap_all(_axes_base.shared(a._native, self._name),
                                 type(a), a.figure)

    def joined(self, a, b):
        """Return whether *a* and *b* are members of the same set."""
        return a.figure is b.figure and b in self._siblings(a)

    def get_siblings(self, a):
        """Return all of the items joined with *a*, including itself."""
        return self._siblings(a)


class _AxesBase(ConstructionMixin, TicksMixin, NavigationMixin, LimitsMixin,
                DecorationsMixin, AspectMixin, LayoutMixin, UnitsMixin,
                BlittingMixin, ProjectionMixin, Artist):
    """The base class of `~.axes.Axes`: an axes apart from its plotting
    methods."""

    _axis_names = ("x", "y")
    _shared_axes = {name: _SharedAxes(name) for name in _axis_names}

    def __str__(self):
        return "{0}({1[0]:g},{1[1]:g};{1[2]:g}x{1[3]:g})".format(
            type(self).__name__, self.get_position(original=True).bounds)

    def __repr__(self):
        fields = []
        if self.get_label():
            fields += [f"label={self.get_label()!r}"]
        if hasattr(self, "get_title"):
            titles = {}
            for k in ["left", "center", "right"]:
                title = self.get_title(loc=k)
                if title:
                    titles[k] = title
            if titles:
                fields += [f"title={titles}"]
        for name, axis in self._axis_map.items():
            if axis is not None and axis.get_label().get_text():
                fields += [f"{name}label={axis.get_label().get_text()!r}"]
        return f"<{self.__class__.__name__}: " + ", ".join(fields) + ">"

    @property
    def transData(self):
        """The transform from data coordinates to display coordinates."""
        built = self.__dict__.get("_transData")
        if built is not None:  # By a projection class of the program's.
            return built
        return _live_transforms.cached(self, "transData", _transforms.trans_data)

    @transData.setter
    def transData(self, transform):
        self.__dict__["_transData"] = transform

    @property
    def transAxes(self):
        """The transform from axes coordinates (0 to 1) to display
        coordinates."""
        built = self.__dict__.get("_transAxes")
        if built is not None:  # By a projection class of the program's.
            return built
        return _live_transforms.cached(self, "transAxes", _transforms.trans_axes)

    @transAxes.setter
    def transAxes(self, transform):
        self.__dict__["_transAxes"] = transform

    def get_xaxis_transform(self, which='grid'):
        """
        Get the transformation used for drawing x-axis labels, ticks
        and gridlines.  The x-direction is in data coordinates and the
        y-direction is in axis coordinates.
        """
        if which not in ('grid', 'tick1', 'tick2'):
            raise ValueError(f'unknown value for which: {which!r}')
        built = self.__dict__.get("_xaxis_transform")
        if built is not None:  # By a projection class of the program's.
            return built
        return _live_transforms.cached(self, "xaxis_transform",
                                       _transforms.trans_xaxis)

    def get_yaxis_transform(self, which='grid'):
        """
        Get the transformation used for drawing y-axis labels, ticks
        and gridlines.  The x-direction is in axis coordinates and the
        y-direction is in data coordinates.
        """
        if which not in ('grid', 'tick1', 'tick2'):
            raise ValueError(f'unknown value for which: {which!r}')
        built = self.__dict__.get("_yaxis_transform")
        if built is not None:  # By a projection class of the program's.
            return built
        return _live_transforms.cached(self, "yaxis_transform",
                                       _transforms.trans_yaxis)

    @property
    def bbox(self):
        """The Axes' box in display coordinates."""
        return _live_transforms.LiveBbox.spanned(self.transAxes)

    @property
    def spines(self):
        """The `.Spines` of the Axes: its `.Spine` objects by side."""
        from .._native import spines as _spines
        from ..spines import Spine, Spines
        spines = {name: _handles.wrap(native, Spine, self.figure)
                  for name, native in _spines.axes_spines(self._native)}
        colorbar = self.__dict__.get("_colorbar")
        if colorbar is not None:
            # A colorbar's outline is a spine of its Axes, as matplotlib's
            # Colorbar makes it (``_ColorbarSpine``).
            spines["outline"] = colorbar.outline
        return Spines(**spines)

    def get_subplotspec(self):
        """Return the `.SubplotSpec` associated with the subplot, or None."""
        from ..gridspec import SubplotSpec
        native = _gridspec.axes_spec(self._native)
        return None if native is None else SubplotSpec._from_native(
            native, self.get_figure(root=False))

    def set_subplotspec(self, subplotspec):
        """Set the `.SubplotSpec`. associated with the subplot."""
        _gridspec.set_axes_spec(self._native, subplotspec._native_spec())

    def get_gridspec(self):
        """Return the `.GridSpec` associated with the subplot, or None."""
        spec = self.get_subplotspec()
        return spec.get_gridspec() if spec is not None else None

    def get_position(self, original=False):
        """
        Return the position of the Axes within the figure as a `.Bbox`.

        Parameters
        ----------
        original : bool
            If ``True``, return the original position. Otherwise, return the
            active position. For an explanation of the positions see
            `.set_position`.

        Returns
        -------
        `.Bbox`
        """
        return Bbox.from_extents(*_axes_base.position(self._native,
                                                      bool(original)))

    def set_position(self, pos, which='both'):
        """
        Set the Axes position.

        Axes have two position attributes. The 'original' position is the
        position allocated for the Axes. The 'active' position is the
        position the Axes is actually drawn at. These positions are usually
        the same unless a fixed aspect is set to the Axes. See
        `.Axes.set_aspect` for details.

        Parameters
        ----------
        pos : [left, bottom, width, height] or `~matplotlib.transforms.Bbox`
            The new position of the Axes in `.Figure` coordinates.

        which : {'both', 'active', 'original'}, default: 'both'
            Determines which position variables to change.
        """
        if not isinstance(pos, Bbox):
            pos = Bbox.from_bounds(*pos)
        _axes_base.set_position(self._native, tuple(map(float, pos.bounds)),
                                which)

    def sharex(self, other):
        """
        Share the x-axis with *other*.

        This is equivalent to passing ``sharex=other`` when constructing the
        Axes, and cannot be used if the x-axis is already being shared with
        another Axes.  Note that it is not possible to unshare axes.
        """
        _api.check_isinstance(_AxesBase, other=other)
        _axes_base.share(self._native, "x", other._native)

    def sharey(self, other):
        """
        Share the y-axis with *other*.

        This is equivalent to passing ``sharey=other`` when constructing the
        Axes, and cannot be used if the y-axis is already being shared with
        another Axes.  Note that it is not possible to unshare axes.
        """
        _api.check_isinstance(_AxesBase, other=other)
        _axes_base.share(self._native, "y", other._native)

    def get_shared_x_axes(self):
        """Return an immutable view on the shared x-axes Grouper."""
        return self._shared_axes["x"]

    def get_shared_y_axes(self):
        """Return an immutable view on the shared y-axes Grouper."""
        return self._shared_axes["y"]

    def get_xticks(self, *, minor=False):
        """Return the xaxis' tick locations in data coordinates."""
        return self.xaxis.get_ticklocs(minor=minor)

    def set_xticks(self, ticks, labels=None, *, minor=False, **kwargs):
        """Set the xaxis' tick locations and optionally tick labels
        (`.Axis.set_ticks`)."""
        return self.xaxis.set_ticks(ticks, labels, minor=minor, **kwargs)

    def get_yticks(self, *, minor=False):
        """Return the yaxis' tick locations in data coordinates."""
        return self.yaxis.get_ticklocs(minor=minor)

    def set_yticks(self, ticks, labels=None, *, minor=False, **kwargs):
        """Set the yaxis' tick locations and optionally tick labels
        (`.Axis.set_ticks`)."""
        return self.yaxis.set_ticks(ticks, labels, minor=minor, **kwargs)

    def set_xticklabels(self, labels, *, minor=False, **kwargs):
        """[*Discouraged*] Set the xaxis' labels with list of string labels
        (`.Axis.set_ticklabels`)."""
        return self.xaxis.set_ticklabels(labels, minor=minor, **kwargs)

    def set_yticklabels(self, labels, *, minor=False, **kwargs):
        """[*Discouraged*] Set the yaxis' labels with list of string labels
        (`.Axis.set_ticklabels`)."""
        return self.yaxis.set_ticklabels(labels, minor=minor, **kwargs)

    def invert_xaxis(self):
        """[*Discouraged*] Invert the x-axis."""
        self.xaxis.set_inverted(not self.xaxis.get_inverted())

    def xaxis_inverted(self):
        """Return whether the xaxis is oriented in the "inverse" direction."""
        return self.xaxis.get_inverted()

    def invert_yaxis(self):
        """[*Discouraged*] Invert the y-axis."""
        self.yaxis.set_inverted(not self.yaxis.get_inverted())

    def yaxis_inverted(self):
        """Return whether the yaxis is oriented in the "inverse" direction."""
        return self.yaxis.get_inverted()

    def set_xscale(self, value, **kwargs):
        """
        Set the xaxis' scale.

        Parameters
        ----------
        value : str
            The axis scale type to apply: "linear", "log", "symlog", "logit",
            "asinh" or "function".
        **kwargs
            Keyword arguments of the scale class.
        """
        self.xaxis._set_axes_scale(value, **kwargs)

    def get_xscale(self):
        """Return the xaxis' scale (as a str)."""
        return _axes_base.get_scale(self._native, "x")

    def set_yscale(self, value, **kwargs):
        """
        Set the yaxis' scale.

        Parameters
        ----------
        value : str
            The axis scale type to apply: "linear", "log", "symlog", "logit",
            "asinh" or "function".
        **kwargs
            Keyword arguments of the scale class.
        """
        self.yaxis._set_axes_scale(value, **kwargs)

    def get_yscale(self):
        """Return the yaxis' scale (as a str)."""
        return _axes_base.get_scale(self._native, "y")

    def set_facecolor(self, color):
        """
        Set the facecolor of the Axes.

        Parameters
        ----------
        color : :mpltype:`color`
        """
        _axes_base.set_facecolor(self._native, color)

    def get_facecolor(self):
        """Get the facecolor of the Axes."""
        return _axes_base.get_facecolor(self._native)

    set_fc = set_facecolor
    get_fc = get_facecolor

    def label_outer(self, remove_inner_ticks=False):
        """
        Only show "outer" labels and tick labels.

        x-labels are only kept for subplots on the last row (or first row, if
        labels are on the top side); y-labels only for subplots on the first
        column (or last column, if labels are on the right side).

        Parameters
        ----------
        remove_inner_ticks : bool, default: False
            If True, remove the inner ticks as well (not only tick labels).
        """
        _axes_base.label_outer(self._native, bool(remove_inner_ticks))

    def set_prop_cycle(self, *args, **kwargs):
        """
        Set the property cycle of the Axes.

        The property cycle controls the style properties such as color,
        marker and linestyle of future plot commands. The style properties
        of data already added to the Axes are not modified.

        Call signatures::

          set_prop_cycle(cycler)
          set_prop_cycle(label=values, label2=values2, ...)
          set_prop_cycle(label, values)

        Form 1 sets given `~cycler.Cycler` object.

        Form 2 creates a `~cycler.Cycler` which cycles over one or more
        properties simultaneously and set it as the property cycle of the
        Axes. If multiple properties are given, their value lists must have
        the same length. This is just a shortcut for explicitly creating a
        cycler and passing it to the function, i.e. it's short for
        ``set_prop_cycle(cycler(label=values, label2=values2, ...))``.

        Form 3 creates a `~cycler.Cycler` for a single property and set it
        as the property cycle of the Axes. This form exists for compatibility
        with the original `cycler.cycler` interface. Its use is discouraged
        in favor of the kwarg form, i.e. ``set_prop_cycle(label=values)``.

        Passing ``None`` resets the cycle to :rc:`axes.prop_cycle`.
        """
        if args and kwargs:
            raise TypeError("Cannot supply both positional and keyword "
                            "arguments to this method.")
        # Can't do `args == (None,)` as that crashes cycler.
        if len(args) == 1 and args[0] is None:
            prop_cycle = None
        else:
            from ..rcsetup import _rc_value, cycler
            prop_cycle = _rc_value(cycler(*args, **kwargs))
        _axes_base.set_prop_cycle(self._native, prop_cycle)

    def _make_twin_axes(self, shared, axes_class, delta_zorder, kwargs):
        """
        Make a twin of the Axes sharing the axis *shared* (``"x"`` or
        ``"y"``), drawn *delta_zorder* above it: pltrs (Rust)'s own, or for
        an *axes_class* or keywords of `.Figure.add_subplot`, the Axes they
        make at its place, which becomes its twin.
        """
        if axes_class is None and not kwargs:
            from ._axes import Axes
            native = _axes_base.twin(self._native, shared, float(delta_zorder))
            return _handles.wrap(native, Axes, self.figure)
        if axes_class is not None:
            kwargs["axes_class"] = axes_class
        kwargs[f"share{shared}"] = self
        fig = self.get_figure(root=False)
        spec = self.get_subplotspec()
        twin = (fig.add_subplot(spec, **kwargs) if spec is not None
                else fig.add_axes(self.get_position(True), **kwargs))
        _axes_base.join_twin(self._native, twin._native, shared,
                             float(delta_zorder))
        return twin

    def twinx(self, axes_class=None, *, delta_zorder=0.0, **kwargs):
        """
        Create a twin Axes sharing the xaxis.

        Create a new Axes with an invisible x-axis and an independent
        y-axis positioned opposite to the original one (i.e. at right). The
        x-axis autoscale setting will be inherited from the original
        Axes.

        Parameters
        ----------
        axes_class : subclass type of `~.axes.Axes`, optional
            The `.axes.Axes` subclass that is instantiated.  This parameter
            is incompatible with *projection* and *polar*.

            By default, `~.axes.Axes` is used.
        delta_zorder : float, default: 0
            A zorder offset for the twin Axes, relative to the original Axes.
            The twin's zorder is set to ``self.get_zorder() + delta_zorder``.
        kwargs : dict
            The keyword arguments passed to `.Figure.add_subplot` or
            `.Figure.add_axes`.

        Returns
        -------
        Axes
            The newly created Axes instance
        """
        return self._make_twin_axes("x", axes_class, delta_zorder, kwargs)

    def twiny(self, axes_class=None, *, delta_zorder=0.0, **kwargs):
        """
        Create a twin Axes sharing the yaxis.

        Create a new Axes with an invisible y-axis and an independent
        x-axis positioned opposite to the original one (i.e. at top). The
        y-axis autoscale setting will be inherited from the original Axes.

        Parameters
        ----------
        axes_class : subclass type of `~.axes.Axes`, optional
            The `.axes.Axes` subclass that is instantiated.  This parameter
            is incompatible with *projection* and *polar*.

            By default, `~.axes.Axes` is used.
        delta_zorder : float, default: 0
            A zorder offset for the twin Axes, relative to the original Axes.
            The twin's zorder is set to ``self.get_zorder() + delta_zorder``.
        kwargs : dict
            The keyword arguments passed to `.Figure.add_subplot` or
            `.Figure.add_axes`.

        Returns
        -------
        Axes
            The newly created Axes instance
        """
        return self._make_twin_axes("y", axes_class, delta_zorder, kwargs)

    def set_zorder(self, level):
        """
        Set the zorder for the artist.  Artists with lower zorder
        values are drawn first; of twinned Axes, the bottom-most draws the
        background.
        """
        super().set_zorder(level)
        _axes_base.update_twinned_patch_visibility(self._native)

    def add_artist(self, a):
        """
        Add an `.Artist` to the Axes; return the artist.

        Use `add_artist` only for artists for which there is no dedicated
        "add" method; and if necessary, use a method such as `update_datalim`
        to manually update the `~.Axes.dataLim` if the artist is to be included
        in autoscaling.

        If no ``transform`` has been specified when creating the artist (e.g.
        ``artist.get_transform() == None``) then the transform is set to
        ``ax.transData``.
        """
        return a._add_to_axes(self, "artist")

    def add_collection(self, collection, autolim=True):
        """
        Add a `.Collection` to the Axes; return the collection.

        Parameters
        ----------
        collection : `.Collection`
            The collection to add.
        autolim : bool
            Whether to update data and view limits. If *False*, the
            collection does not take part in any limit operations.
        """
        from ..collections import Collection
        _api.check_isinstance(Collection, collection=collection)
        return collection._add_to_axes(self, "collection", autolim)

    def add_image(self, image):
        """Add an `.AxesImage` to the Axes; return the image."""
        from ..image import AxesImage
        _api.check_isinstance(AxesImage, image=image)
        return image._add_to_axes(self, "image")

    def add_line(self, line):
        """Add a `.Line2D` to the Axes; return the line."""
        from ..lines import Line2D
        _api.check_isinstance(Line2D, line=line)
        return line._add_to_axes(self, "line")

    def add_patch(self, p):
        """Add a `.Patch` to the Axes; return the patch."""
        from ..patches import Patch
        _api.check_isinstance(Patch, p=p)
        return p._add_to_axes(self, "patch")

    def add_table(self, tab):
        """Add a `.Table` to the Axes; return the table."""
        from ..table import Table
        _api.check_isinstance(Table, tab=tab)
        return tab._add_to_axes(self, "artist")
