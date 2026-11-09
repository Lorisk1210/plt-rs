"""
`_AxesBase`, the base of `~.axes.Axes` (matplotlib's ``axes._base``): what
an axes is apart from its plotting methods.

Ready-made artists join an axes here (``add_artist``, ``add_line``,
``add_patch``, ``add_collection``, ``add_image``): pltrs
(Rust) adopts the artist into the axes' figure as matplotlib's method of
that name adds it (`.Artist._add_to_axes`, `.._handles.adopt`).
"""

import textwrap

from .. import _api, _artist_clipping, _handles, _values, cbook
from .._native import axes_base as _axes_base, gridspec as _gridspec
from .._transform_trees import AxesTrees
from ..artist import Artist, allow_rasterization
from ..transforms import IdentityTransform
from . import _foreign_children
from ._artist_list import ArtistList
from ._aspect import AspectMixin
from ._blitting import BlittingMixin
from ._children import ChildrenMixin
from ._clear import ClearMixin
from ._construction import ConstructionMixin
from ._decorations import DecorationsMixin
from ._insets import ChildAxesMixin
from ._insets import _TransformedBoundsLocator  # noqa: F401  (matplotlib defines it here)
from ._layout import LayoutMixin
from ._limits import LimitsMixin
from ._navigation import NavigationMixin
from ._projection import ProjectionMixin
from ._sharing import SharedGroups
from ._prop_cycles import PropCycleMixin
from ._ticks import TicksMixin, _axis_method_wrapper
from ._units import UnitsMixin
from ..transforms import Bbox


class _SharedAxes(SharedGroups):
    """
    The groups of axes sharing one axis (matplotlib's ``_shared_axes[name]``,
    a `.cbook.Grouper`), as pltrs (Rust) keeps them.
    """

    def __init__(self, name):
        self._name = name

    def _siblings(self, a):
        return _handles.wrap_all(_axes_base.shared(a._native, self._name),
                                 type(a), a._figure)


@_api.define_aliases({"facecolor": ["fc"]})
class _AxesBase(ClearMixin, ChildrenMixin, ChildAxesMixin,
                AxesTrees, ConstructionMixin, TicksMixin, NavigationMixin,
                LimitsMixin, DecorationsMixin, AspectMixin, LayoutMixin,
                UnitsMixin, BlittingMixin, ProjectionMixin, PropCycleMixin,
                Artist):
    """The base class of `~.axes.Axes`: an axes apart from its plotting
    methods."""

    def set_figure(self, fig):
        # docstring inherited: matplotlib's override has none.
        super().set_figure(fig)
        # matplotlib's makes the Axes' boxes and transforms anew: the view
        # the unit box, no data limits, and an identity ``transScale`` until
        # a scale is set again.
        _axes_base.reset_for_figure(self._native)
        self.transScale.set(IdentityTransform())

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    _axis_names = ("x", "y")
    _shared_axes = {name: _SharedAxes(name) for name in _axis_names}

    #: pltrs (Rust) holds an Axes in no other Axes: another Axes'
    #: `add_artist` lists it through a stand-in (`._foreign_children`).
    _held_in_axes = False

    ArtistList = ArtistList

    def __str__(self):
        return "{0}({1[0]:g},{1[1]:g};{1[2]:g}x{1[3]:g})".format(
            type(self).__name__, self._active_bounds())

    def _active_bounds(self):
        """The bounds of the active position as it is now, the aspect not
        applied (matplotlib's ``_position``)."""
        return self._position.bounds

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

    def get_xaxis_transform(self, which='grid'):
        """
        Get the transformation used for drawing x-axis labels, ticks
        and gridlines.  The x-direction is in data coordinates and the
        y-direction is in axis coordinates.

        .. note::

            This transformation is primarily used by the
            `~matplotlib.axis.Axis` class, and is meant to be
            overridden by new kinds of projections that may need to
            place axis elements in different locations.

        Parameters
        ----------
        which : {'grid', 'tick1', 'tick2'}
        """
        if which == 'grid':
            return self._xaxis_transform
        elif which == 'tick1':
            # for cartesian projection, this is bottom spine
            return self.spines.bottom.get_spine_transform()
        elif which == 'tick2':
            # for cartesian projection, this is top spine
            return self.spines.top.get_spine_transform()
        else:
            raise ValueError(f'unknown value for which: {which!r}')

    def get_yaxis_transform(self, which='grid'):
        """
        Get the transformation used for drawing y-axis labels, ticks
        and gridlines.  The x-direction is in axis coordinates and the
        y-direction is in data coordinates.

        .. note::

            This transformation is primarily used by the
            `~matplotlib.axis.Axis` class, and is meant to be
            overridden by new kinds of projections that may need to
            place axis elements in different locations.

        Parameters
        ----------
        which : {'grid', 'tick1', 'tick2'}
        """
        if which == 'grid':
            return self._yaxis_transform
        elif which == 'tick1':
            # for cartesian projection, this is left spine
            return self.spines.left.get_spine_transform()
        elif which == 'tick2':
            # for cartesian projection, this is right spine
            return self.spines.right.get_spine_transform()
        else:
            raise ValueError(f'unknown value for which: {which!r}')

    @property
    def spines(self):
        """The `.Spines` of the Axes: its `.Spine` objects by side."""
        from .._native import spines as _spines
        from ..spines import Spine, Spines
        programmed = self.__dict__.get("_program_projection")
        if programmed is not None:
            # The spines a program's projection class made.
            spines = dict(programmed.named_spines)
        else:
            spines = {name: _handles.wrap(native, Spine, self._figure)
                      for name, native in _spines.axes_spines(self._native)}
        colorbar = self.__dict__.get("_colorbar")
        if colorbar is not None:
            # A colorbar's outline is a spine of its Axes, as matplotlib's
            # Colorbar makes it (``_ColorbarSpine``).
            spines["outline"] = colorbar.outline
        # One container, as matplotlib's: what the program adds or deletes
        # stays, and the spines made since join it.
        kept = self.__dict__.get("_spines_kept")
        if kept is None:
            kept = self._spines_kept = Spines(**spines)
            self._spines_seen = set(spines)
        for name in spines.keys() - self._spines_seen:
            kept._join(name, spines[name])
        self._spines_seen |= spines.keys()
        return kept

    def get_subplotspec(self):
        """Return the `.SubplotSpec` associated with the subplot, or None."""
        from ..gridspec import SubplotSpec
        if "_unplaced_subplotspec" in self.__dict__:
            return self._unplaced_subplotspec
        native = _gridspec.axes_spec(self._native)
        if native is None:
            return None
        spec = SubplotSpec._from_native(native, self.get_figure(root=False))
        # The same object while the Axes stays in that block, as matplotlib
        # keeps its ``_subplotspec``.
        kept = self.__dict__.get("_subplotspec_kept")
        if kept != spec:
            kept = self._subplotspec_kept = spec
        return kept

    def set_subplotspec(self, subplotspec):
        """Set the `.SubplotSpec`. associated with the subplot."""
        # matplotlib keeps the spec, then places the Axes at the position
        # the spec gives, which fails for what is no spec (the Axes staying
        # where it was).
        try:
            subplotspec.get_position(self.get_figure(root=False))
        except Exception:
            self._unplaced_subplotspec = subplotspec
            raise
        self.__dict__.pop("_unplaced_subplotspec", None)
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
        if not original and self.get_axes_locator():
            # Only drawing places an axes with a locator, and applies its
            # aspect.
            return Bbox.from_extents(*_axes_base.located_position(self._native))
        position = Bbox.from_extents(*_axes_base.position(self._native,
                                                          bool(original)))
        if not original:
            # matplotlib's apply_aspect sets the active position anew.
            self.stale = True
        return position

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

            .. note::
                This parameter is considered internal. End users should not use it.

                For native Matplotlib `.Axes`, the active position is
                determined by a combination of the original position and the
                aspect ratio.  Any active position set by the user will therefore be
                overwritten by internal handling.  This option is retained as it may be
                relevant for some third party `.Axes` subclasses.

        See Also
        --------
        matplotlib.transforms.Bbox.from_bounds
        matplotlib.transforms.Bbox.from_extents
        """
        if not isinstance(pos, Bbox):
            pos = Bbox.from_bounds(*pos)
        if which not in ('both', 'active', 'original'):
            # matplotlib changes neither position then, and takes the Axes
            # out of its layout.
            self.set_in_layout(False)
            return
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
        self.xaxis._take_shared_tickers()

    def sharey(self, other):
        """
        Share the y-axis with *other*.

        This is equivalent to passing ``sharey=other`` when constructing the
        Axes, and cannot be used if the y-axis is already being shared with
        another Axes.  Note that it is not possible to unshare axes.
        """
        _api.check_isinstance(_AxesBase, other=other)
        _axes_base.share(self._native, "y", other._native)
        self.yaxis._take_shared_tickers()

    @property
    def _sharex(self):
        """The Axes whose x-axis this one shares (``sharex=``), or None."""
        return self._shared_from("x")

    @property
    def _sharey(self):
        """The Axes whose y-axis this one shares (``sharey=``), or None."""
        return self._shared_from("y")

    def _shared_from(self, name):
        native = _axes_base.shared_from(self._native, name)
        return None if native is None else _handles.wrap(native, type(self), self._figure)

    def _sci(self, im):
        """
        Set the current image.

        This image will be the target of colormap functions like
        ``pyplot.viridis``, and other functions such as `~.pyplot.clim`.  The
        current image is an attribute of the current Axes.
        """
        from .. import collections as mcoll, image as mimage
        _api.check_isinstance((mcoll.Collection, mimage.AxesImage), im=im)
        self._current_image = im

    def _gci(self):
        """Helper for `~matplotlib.pyplot.gci`; do not use elsewhere."""
        return getattr(self, '_current_image', None)

    @classmethod
    def get_shared_x_axes(cls):
        """Return an immutable view on the shared x-axes Grouper."""
        return cbook.GrouperView(cls._shared_axes["x"])

    @classmethod
    def get_shared_y_axes(cls):
        """Return an immutable view on the shared y-axes Grouper."""
        return cbook.GrouperView(cls._shared_axes["y"])

    def invert_xaxis(self):
        """
        [*Discouraged*] Invert the x-axis.

        .. admonition:: Discouraged

            The use of this method is discouraged.
            Use `.Axes.set_xinverted` instead.

        See Also
        --------
        get_xinverted
        get_xlim, set_xlim
        get_xbound, set_xbound
        """
        self.xaxis.set_inverted(not self.xaxis.get_inverted())

    xaxis_inverted = _axis_method_wrapper("xaxis", "get_inverted")
    if xaxis_inverted.__doc__:
        xaxis_inverted.__doc__ = ("[*Discouraged*] " + xaxis_inverted.__doc__ +
                                  textwrap.dedent("""

        .. admonition:: Discouraged

            The use of this method is discouraged.
            Use `.Axes.get_xinverted` instead.
        """))

    get_xscale = _axis_method_wrapper("xaxis", "get_scale")
    set_xscale = _axis_method_wrapper("xaxis", "_set_axes_scale")
    get_xticks = _axis_method_wrapper("xaxis", "get_ticklocs")
    set_xticks = _axis_method_wrapper("xaxis", "set_ticks",
                                      doc_sub={'set_ticks': 'set_xticks'})
    set_xticklabels = _axis_method_wrapper(
        "xaxis", "set_ticklabels",
        doc_sub={"Axis.set_ticks": "Axes.set_xticks"})

    def invert_yaxis(self):
        """
        [*Discouraged*]  Invert the y-axis.

        .. admonition:: Discouraged

            The use of this method is discouraged.
            Use `.Axes.set_yinverted` instead.

        See Also
        --------
        get_yinverted
        get_ylim, set_ylim
        get_ybound, set_ybound
        """
        self.yaxis.set_inverted(not self.yaxis.get_inverted())

    yaxis_inverted = _axis_method_wrapper("yaxis", "get_inverted")
    if yaxis_inverted.__doc__:
        yaxis_inverted.__doc__ = ("[*Discouraged*] " + yaxis_inverted.__doc__ +
                                  textwrap.dedent("""

        .. admonition:: Discouraged

            The use of this method is discouraged.
            Use `.Axes.get_yinverted` instead.
        """))

    get_yscale = _axis_method_wrapper("yaxis", "get_scale")
    set_yscale = _axis_method_wrapper("yaxis", "_set_axes_scale")
    get_yticks = _axis_method_wrapper("yaxis", "get_ticklocs")
    set_yticks = _axis_method_wrapper("yaxis", "set_ticks",
                                      doc_sub={'set_ticks': 'set_yticks'})
    set_yticklabels = _axis_method_wrapper(
        "yaxis", "set_ticklabels",
        doc_sub={"Axis.set_ticks": "Axes.set_yticks"})

    def set_facecolor(self, color):
        """
        Set the facecolor of the Axes.

        Parameters
        ----------
        color : :mpltype:`color`
        """
        self._facecolor = color
        self.stale = True
        return self.patch.set_facecolor(color)

    def get_facecolor(self):
        """Get the facecolor of the Axes."""
        return self.patch.get_facecolor()

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

            .. versionadded:: 3.8
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

        Parameters
        ----------
        cycler : `~cycler.Cycler` or ``None``
            Set the given Cycler. *None* resets to the cycle defined by the
            current style.

            .. ACCEPTS: `~cycler.Cycler`

        label : str
            The property key. Must be a valid `.Artist` property.
            For example, 'color' or 'linestyle'. Aliases are allowed,
            such as 'c' for 'color' and 'lw' for 'linewidth'.

        values : iterable
            Finite-length iterable of the property values. These values
            are validated and will raise a ValueError if invalid.

        See Also
        --------
        matplotlib.rcsetup.cycler
            Convenience function for creating validated cyclers for properties.
        cycler.cycler
            The original function for creating unvalidated cyclers.

        Examples
        --------
        Setting the property cycle for a single property:

        >>> ax.set_prop_cycle(color=['red', 'green', 'blue'])

        Setting the property cycle for simultaneously cycling over multiple
        properties (e.g. red circle, green plus, blue cross):

        >>> ax.set_prop_cycle(color=['red', 'green', 'blue'],
        ...                   marker=['o', '+', 'x'])

        """
        if args and kwargs:
            raise TypeError("Cannot supply both positional and keyword "
                            "arguments to this method.")
        # Can't do `args == (None,)` as that crashes cycler.
        if len(args) == 1 and args[0] is None:
            prop_cycle = given = None
        else:
            from ..rcsetup import _rc_value, cycler
            given = cycler(*args, **kwargs)
            prop_cycle = _rc_value(given)
        _axes_base.set_prop_cycle(self._native, prop_cycle)
        _values.remember(self, "prop_cycle", given)

    def _make_twin_axes(self, shared, axes_class, delta_zorder, kwargs):
        """
        Make a twin of the Axes sharing the axis *shared* (``"x"`` or
        ``"y"``), drawn *delta_zorder* above it: pltrs (Rust)'s own, or for
        an *axes_class* or keywords of `.Figure.add_subplot`, the Axes they
        make at its place, which becomes its twin.
        """
        if self.get_figure(root=False) is None:
            # matplotlib's makes the twin with its figure's `add_subplot`
            # (`add_axes` without a subplot spec), which an Axes removed
            # from its figure has not.
            name = "add_subplot" if self.get_subplotspec() else "add_axes"
            raise AttributeError(f"'NoneType' object has no attribute '{name}'",
                                 name=name, obj=None)
        try:
            self.get_zorder() + delta_zorder
        except TypeError:
            # matplotlib adds the twin sharing the axis and has both Axes
            # adjust their data limits, then fails to set the twin's
            # z-order: the twin stays as it was made, not joined as a twin
            # nor set up by `twinx`.
            kwargs = {**kwargs, f"share{shared}": self}
            if axes_class:
                kwargs["axes_class"] = axes_class
            fig = self.get_figure(root=False)
            spec = self.get_subplotspec()
            twin = (fig.add_subplot(spec, **kwargs) if spec is not None
                    else fig.add_axes(self.get_position(True), **kwargs))
            self.set_adjustable('datalim')
            twin.set_adjustable('datalim')
            raise
        if not axes_class and not kwargs:
            from ._axes import Axes
            native = _axes_base.twin(self._native, shared, float(delta_zorder))
            return _handles.wrap(native, Axes, self._figure)
        if axes_class:
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
        Axes.  To ensure that the tick marks of both y-axes align, see
        `~matplotlib.ticker.LinearLocator`.

        Parameters
        ----------
        axes_class : subclass type of `~.axes.Axes`, optional
            The `.axes.Axes` subclass that is instantiated.  This parameter
            is incompatible with *projection* and *polar*.  See
            :ref:`axisartist_users-guide-index` for examples.

            By default, `~.axes.Axes` is used.

            .. versionadded:: 3.11

        delta_zorder : float, default: 0
            A zorder offset for the twin Axes, relative to the original Axes.
            The twin's zorder is set to ``self.get_zorder() + delta_zorder``.
            By default (*delta_zorder* is 0), the twin has the same zorder as
            the original Axes.

        kwargs : dict
            The keyword arguments passed to `.Figure.add_subplot` or `.Figure.add_axes`.

            .. versionadded:: 3.11

        Returns
        -------
        Axes
            The newly created Axes instance

        Notes
        -----
        For those who are 'picking' artists while using twinx, pick
        events are only called for the artists in the top-most Axes.
        """
        return self._make_twin_axes("x", axes_class, delta_zorder, kwargs)

    def twiny(self, axes_class=None, *, delta_zorder=0.0, **kwargs):
        """
        Create a twin Axes sharing the yaxis.

        Create a new Axes with an invisible y-axis and an independent
        x-axis positioned opposite to the original one (i.e. at top). The
        y-axis autoscale setting will be inherited from the original Axes.
        To ensure that the tick marks of both x-axes align, see
        `~matplotlib.ticker.LinearLocator`.

        Parameters
        ----------
        axes_class : subclass type of `~.axes.Axes`, optional
            The `.axes.Axes` subclass that is instantiated.  This parameter
            is incompatible with *projection* and *polar*.  See
            :ref:`axisartist_users-guide-index` for examples.

            By default, `~.axes.Axes` is used.

            .. versionadded:: 3.11

        delta_zorder : float, default: 0
            A zorder offset for the twin Axes, relative to the original Axes.
            The twin's zorder is set to ``self.get_zorder() + delta_zorder``.
            By default (*delta_zorder* is 0), the twin has the same zorder as
            the original Axes.

        kwargs : dict
            The keyword arguments passed to `.Figure.add_subplot` or `.Figure.add_axes`.

            .. versionadded:: 3.11

        Returns
        -------
        Axes
            The newly created Axes instance

        Notes
        -----
        For those who are 'picking' artists while using twiny, pick
        events are only called for the artists in the top-most Axes.
        """
        return self._make_twin_axes("y", axes_class, delta_zorder, kwargs)

    def set_zorder(self, level):
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
        if not isinstance(a, Artist):
            return _foreign_children.add(self, a)
        if _foreign_children.held_elsewhere(self, a):
            return _foreign_children.relist(self, a)
        added = a._add_to_axes(self, "artist")
        _artist_clipping.take_parent_defaults(a, self)
        return added

    def add_collection(self, collection, autolim=True):
        """
        Add a `.Collection` to the Axes; return the collection.

        Parameters
        ----------
        collection : `.Collection`
            The collection to add.
        autolim : bool
            Whether to update data and view limits.

            If *False*, the collection does not take part in any limit
            operations.

            .. versionchanged:: 3.11

               Since 3.11 ``autolim=True`` matches the standard behavior
               of other ``add_[artist]`` methods: Axes data and view limits
               are both updated in the method, and the collection will
               be considered in future data limit updates through
               `.relim`.

               Prior to matplotlib 3.11 this was only a one-time update
               of the data limits. Updating view limits required an
               explicit call to `~.Axes.autoscale_view`, and collections
               did not take part in `.relim`.

            As an implementation detail, the value "_datalim_only" is
            supported to smooth the internal transition from pre-3.11
            behavior. This is not a public interface and will be removed
            again in the future.
        """
        from ..collections import Collection
        _api.check_isinstance(Collection, collection=collection)
        figure = collection.get_figure(root=True)
        if figure is not None and figure is not self._figure:
            _foreign_children.add_drawn(self, collection)
        return collection._add_to_axes(self, "collection", autolim)

    def add_image(self, image):
        """
        Add an `.AxesImage` to the Axes; return the image.
        """
        from ..image import AxesImage
        _api.check_isinstance(AxesImage, image=image)
        if _foreign_children.add_held(self, image):
            return image
        return image._add_to_axes(self, "image")

    def add_line(self, line):
        """
        Add a `.Line2D` to the Axes; return the line.
        """
        from ..lines import Line2D
        _api.check_isinstance(Line2D, line=line)
        if _foreign_children.add_held(self, line):
            return line
        return line._add_to_axes(self, "line")

    def add_patch(self, p):
        """
        Add a `.Patch` to the Axes; return the patch.
        """
        from ..patches import Patch
        _api.check_isinstance(Patch, p=p)
        if _foreign_children.add_held(self, p):
            return p
        return p._add_to_axes(self, "patch")

    def add_table(self, tab):
        """
        Add a `.Table` to the Axes; return the table.
        """
        from ..table import Table
        _api.check_isinstance(Table, tab=tab)
        if _foreign_children.add_held(self, tab):
            return tab
        added = tab._add_to_axes(self, "artist")
        _artist_clipping.take_parent_defaults(tab, self)
        return added


# matplotlib's ``_AxesBase.ArtistList``.
_AxesBase.ArtistList = ArtistList
ArtistList.__qualname__ = "_AxesBase.ArtistList"
cbook._set_module(__name__, ArtistList)

# matplotlib defines the methods of the mixins on _AxesBase itself.
cbook._own_mixin_methods(_AxesBase, Artist)
