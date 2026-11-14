"""
The top level container for all the plot elements (matplotlib's
``figure``).

A `Figure` owns a pltrs (Rust) figure (``self._native``) and the registry of the
objects of its artists (``self._artists``, see `._handles`).
"""

from numbers import Integral
import functools
import logging
import math

import numpy as np

from . import _docstring
from . import colorbar as cbar  # noqa: F401  (registers the colorbar snippets)
from . import _api, _artist_clipping, _blocking_input, _handles, _values, cbook, rcParams
from . import _gridspec_kept, _transform_trees
from ._native import artist as _artist
from ._native import figure as _figure, gridspec as _gridspec
from .artist import Artist
from . import _background_patch
from . import _lazy_imports
from .axes import Axes
from .axes._base import _AxesBase
from .axes import _sharing
from .axes import _program_tick_params
from .axes import _construction
from .backend_bases import DrawEvent, FigureCanvasBase, MouseButton, NonGuiException
from .gridspec import GridSpec, SubplotParams, SubplotSpec, _figure_subplotpars
from ._program_classes import takes_from_program
from ._figure_images import FigureImagesMixin
from ._figure_layout import FigureBaseLayout, FigureLayout
from ._figure_mosaic import _MosaicMixin
from ._figure_frame import FrameMixin
from ._figure_interaction import InteractionMixin, RootFigureMixin
from .layout_engine import (ConstrainedLayoutEngine, LayoutEngine,
                            PlaceHolderLayoutEngine, TightLayoutEngine)
from .lines import Line2D
from .text import Text
from .transforms import Bbox, BboxBase
from ._text_methods import FigureText
from ._transform_trees import FigureTrees
from ._artist_staleness import _stale_figure_callback  # noqa: F401

_figure_artists = _lazy_imports.module(globals(), "._figure_artists")
_foreign_children = _lazy_imports.module(globals(), ".axes._foreign_children")
_layout_checks = _lazy_imports.module(globals(), "._layout_checks")
_figure_draws = _lazy_imports.module(globals(), "._figure_draws")
_program = _lazy_imports.module(globals(), ".projections._program")

_log = logging.getLogger(__name__)


def _native_maker(projection_class):
    """
    How the native axes of a new *projection_class* is made: by its
    constructor for a subclass of `.Axes` with a constructor of the
    program's, which matplotlib calls with the keywords of
    ``add_subplot``; by the class' own ``_make_native``; by its
    constructor for any other subclass with a constructor of its own,
    and for a class of the program's overriding what that constructor
    calls (`_constructed`); else None, for pltrs (Rust)'s axes of the
    projection.
    """
    if not callable(projection_class):
        # matplotlib calls it to make the Axes (a program's *axes_class*).
        raise TypeError(f"'{type(projection_class).__name__}' object is not callable")
    if (isinstance(projection_class, type) and issubclass(projection_class, Axes)
            and takes_from_program(projection_class, "__init__")):
        # A constructor of the program's (ultraplot's polar and 3-D Axes),
        # which matplotlib calls with the keywords whatever the class.
        return projection_class._constructed_native
    make = getattr(projection_class, "_make_native", None)
    if (make is None and projection_class.name == "rectilinear"
            and projection_class.__init__ is not Axes.__init__):
        make = projection_class._constructed_native
    if make is None and _constructed(projection_class):
        make = projection_class._constructed_native
    if make is None:
        if _program.is_program_class(projection_class):
            make = (projection_class._constructed_native
                    if projection_class.__init__ is not Axes.__init__
                    else _program.maker(projection_class))
    return make


#: What matplotlib's ``Axes.__init__`` calls on the new Axes: a class of
#: the program's overriding one is made by its constructor.
_CONSTRUCTION_CALLS = ("__init__", "clear", "cla", "grid")


@functools.cache
def _constructed(projection_class):
    """Whether a new *projection_class* is made by its constructor: a
    rectilinear or polar class of the program's overriding what
    matplotlib's ``Axes.__init__`` calls (`_CONSTRUCTION_CALLS`), which
    matplotlib calls with the arguments of ``add_subplot`` (kept per class,
    as `.takes_from_program` keeps its answers)."""
    return (_program.native_name(projection_class) in ("rectilinear", "polar")
            and any(takes_from_program(projection_class, name) for name in _CONSTRUCTION_CALLS))


class FigureBase(FigureTrees, FigureText, FigureBaseLayout, _MosaicMixin,
                 InteractionMixin, FrameMixin, Artist):
    """
    Base class for `.Figure` and `.SubFigure` containing the methods that add
    artists to the figure or subfigure, create Axes, etc.
    """

    def __init__(self, **kwargs):
        # The base made alone is an artist on no figure, given *kwargs*
        # (`.Figure` and `.SubFigure` make theirs around a native frame).
        super().__init__()
        self.set(**kwargs)

    def get_figure(self, root=None):
        """
        Return the `.Figure` or `.SubFigure` instance the (Sub)Figure belongs to.

        Parameters
        ----------
        root : bool, default=True
            If False, return the (Sub)Figure this artist is on.  If True,
            return the root Figure for a nested tree of SubFigures.

            .. deprecated:: 3.10

                From version 3.12 *root* will default to False.
        """
        if self._figure is self:
            # Top level Figure
            return self

        if self._parent is self._figure:
            # Return early to prevent the deprecation warning when *root* does
            # not matter
            return self._parent
        if root is None:
            _api.warn_deprecated(
                '3.10',
                message='From Matplotlib 3.12 SubFigure.get_figure will by '
                'default return the direct parent figure, which may be a '
                'SubFigure. To suppress this warning, pass the root parameter.  '
                'Pass `True` to maintain the old behavior and `False` to opt-in '
                'to the future behavior.')
            root = True
        return self._figure if root else self._parent

    def _gci(self):
        """
        Get the current colorable artist.

        Specifically, returns the current `.ScalarMappable` instance (`.Image`
        created by `imshow` or `figimage`, `.Collection` created by `pcolor` or
        `scatter`, etc.), or *None* if no such instance has been defined.

        The current image is an attribute of the current Axes, or the nearest
        earlier Axes in the current figure that contains an image.

        Notes
        -----
        Historically, the only colorable artists were images; hence the name
        ``gci`` (get current image).
        """
        axes = self.axes
        if not axes:
            return None
        im = self.gca()._gci()
        if im is not None:
            return im
        for ax in reversed(axes):
            im = ax._gci()
            if im is not None:
                return im
        return None

    def get_window_extent(self, renderer=None):
        return self.bbox

    def set_transform(self, t):
        """
        Set the artist transform.

        Parameters
        ----------
        t : `~matplotlib.transforms.Transform`
        """
        if hasattr(t, "is_affine"):
            Artist.set_transform(self, t)
        else:
            # A (sub)figure draws without its transform, so matplotlib
            # keeps whatever it is given.
            self._given_transform = t
            self.pchanged()

    def get_facecolor(self):
        """Get the face color of the Figure rectangle."""
        return self.patch.get_facecolor()

    def set_facecolor(self, color):
        """
        Set the face color of the Figure rectangle.

        Parameters
        ----------
        color : :mpltype:`color`
        """
        self.patch.set_facecolor(color)

    @property
    def patch(self):
        """The background of the (sub)figure: a `.Rectangle` mirroring the
        background pltrs (Rust) draws (see `._background_patch`)."""
        return _background_patch.figure_patch(self)

    def _listed_axes(self):
        """The Axes of the figure or subfigure (`Figure.axes`,
        `SubFigure.axes`)."""
        from ._native import mplot3d as _mplot3d
        listed = _mplot3d.figure_axes(self._native)
        # The 3D class (its toolkit imported) only for a figure with 3D axes.
        axes3d = None
        if any(is_3d for _, is_3d in listed):
            from .projections import get_projection_class
            axes3d = get_projection_class("3d")
        # matplotlib's stack lists the Axes as they were added, one taken
        # out and added back last (`_add_axes_back`).
        added_back = self.get_figure(root=True).__dict__.get("_axes_added_back", {})
        if added_back:
            listed = sorted(listed, key=lambda entry: added_back.get(
                _artist.id(entry[0]), _artist.id(entry[0])))
        return [_handles.wrap(native, axes3d if is_3d else Axes, self.figure)
                for native, is_3d in listed]

    def _add_axes_back(self, ax):
        """Add *ax*, an Axes of the figure taken out of it, back: the last
        of the figure's Axes, as matplotlib's stack adds it."""
        root = self.get_figure(root=True)
        added_back = root.__dict__.setdefault("_axes_added_back", {})
        last = max((added_back.get(_artist.id(other._native), _artist.id(other._native))
                    for other in root.axes), default=0)
        ax._add_to_figure(self, False)
        added_back[_artist.id(ax._native)] = last + 0.5
        self._axes_changed()
        return ax

    @property
    def _axstack(self):
        """matplotlib's register of the Axes of the figure and of its
        current one, a subfigure's its figure's (see `._axes_stack`)."""
        from ._axes_stack import _AxesStack
        return _AxesStack(self.get_figure(root=True))

    @property
    def _localaxes(self):
        """The Axes placed on this (sub)figure itself, not on a subfigure
        of it."""
        return [ax for ax in self.axes if ax.get_figure(root=False) is self]

    @property
    def texts(self):
        """The `.Text` instances added to the (sub)figure itself, in a list
        drawing those put in it (see `._figure_artists`)."""
        return _figure_artists.DrawnList(
            self, _handles.wrap_all(_figure.texts(self._native), Text, self), texts=True)

    @texts.setter
    def texts(self, texts):
        _figure_artists.replace(self.texts, texts)

    def get_children(self):
        """Get a list of artists contained in the figure."""
        return [self.patch,
                *self.artists,
                *self._localaxes,
                *self.lines,
                *self.patches,
                *self.texts,
                *self.images,
                *self.legends,
                *self.subfigs]

    def _sorted_among(self):
        # A subfigure is sorted among its parent's children; nothing sorts
        # the root figure.
        return None if self._figure is self else self._parent

    def add_gridspec(self, nrows=1, ncols=1, **kwargs):
        """
        Low-level API for creating a `.GridSpec` that has this figure as a parent.

        This is a low-level API, allowing you to create a gridspec and
        subsequently add subplots based on the gridspec. Most users do
        not need that freedom and should use the higher-level methods
        `~.Figure.subplots` or `~.Figure.subplot_mosaic`.

        Parameters
        ----------
        nrows : int, default: 1
            Number of rows in grid.

        ncols : int, default: 1
            Number of columns in grid.

        Returns
        -------
        `.GridSpec`

        Other Parameters
        ----------------
        **kwargs
            Keyword arguments are passed to `.GridSpec`.

        See Also
        --------
        matplotlib.pyplot.subplots

        Examples
        --------
        Adding a subplot that spans two rows::

            fig = plt.figure()
            gs = fig.add_gridspec(2, 2)
            ax1 = fig.add_subplot(gs[0, 0])
            ax2 = fig.add_subplot(gs[1, 0])
            # spans two rows:
            ax3 = fig.add_subplot(gs[:, 1])

        """
        _ = kwargs.pop('figure', None)  # pop in case user has added this...
        gs = GridSpec(nrows=nrows, ncols=ncols, figure=self, **kwargs)
        return gs

    def _process_projection_requirements(self, *, axes_class=None, polar=False,
                                         projection=None, **kwargs):
        """
        Handle the args/kwargs to add_axes/add_subplot/gca, returning::

            (axes_proj_class, proj_class_kwargs)

        which can be used for new Axes initialization/identification.
        """
        from .projections import get_projection_class
        if axes_class is not None:
            if polar or projection is not None:
                raise ValueError(
                    "Cannot combine 'axes_class' and 'projection' or 'polar'")
            projection_class = axes_class
        else:
            if polar:
                if projection is not None and projection != 'polar':
                    raise ValueError(
                        f"polar={polar}, yet projection={projection!r}. "
                        "Only one of these arguments should be supplied."
                    )
                projection = 'polar'
            if isinstance(projection, str) or projection is None:
                projection_class = get_projection_class(projection)
            elif hasattr(projection, '_as_mpl_axes'):
                projection_class, extra_kwargs = projection._as_mpl_axes()
                kwargs.update(**extra_kwargs)
            else:
                raise TypeError(
                    f"projection must be a string, None or implement a "
                    f"_as_mpl_axes method, not {projection!r}")
        return projection_class, kwargs

    def _new_axes(self, native, projection_class, kwargs):
        """The Axes object of the new native axes *native*, of class
        *projection_class*, sharing and styled as *kwargs* say."""
        ax = _handles.wrap(native, projection_class, self.figure)
        if _native_maker(projection_class) is None:
            # As the `__init__` of matplotlib's, which no maker ran.
            _program_tick_params.apply(ax)
        # plt.subplot reuses an axes made with the same class and keywords.
        ax._projection_init = (projection_class, dict(kwargs))
        try:
            for name in ("x", "y"):
                other = kwargs.pop(f"share{name}", None)
                if other is not None:
                    getattr(ax, f"share{name}")(other)
            # The one keyword of Axes.__init__ whose setter has another name.
            if not kwargs.pop("frameon", True):
                ax._frameon = False
            ax._internal_update(kwargs)
        except Exception:
            # matplotlib's ``Axes.__init__`` fails before the figure adds
            # the axes: none is left.
            ax.remove()
            raise
        # matplotlib's ``_add_axes_internal`` makes it current (``sca``).
        self._axes_changed()
        return ax

    @_docstring.interpd
    def add_axes(self, *args, **kwargs):
        """
        Add an `~.axes.Axes` to the figure.

        Call signatures::

            add_axes(rect, projection=None, polar=False, **kwargs)
            add_axes(ax)

        Parameters
        ----------
        rect : tuple (left, bottom, width, height)
            The dimensions (left, bottom, width, height) of the new
            `~.axes.Axes`. All quantities are in fractions of figure width and
            height.

        projection : {None, 'aitoff', 'hammer', 'lambert', 'mollweide', \
'polar', 'rectilinear', str}, optional
            The projection type of the `~.axes.Axes`. *str* is the name of
            a custom projection, see `~matplotlib.projections`. The default
            None results in a 'rectilinear' projection.

        polar : bool, default: False
            If True, equivalent to projection='polar'.

        axes_class : subclass type of `~.axes.Axes`, optional
            The `.axes.Axes` subclass that is instantiated.  This parameter
            is incompatible with *projection* and *polar*.  See
            :ref:`axisartist_users-guide-index` for examples.

        sharex, sharey : `~matplotlib.axes.Axes`, optional
            Share the x or y `~matplotlib.axis` with sharex and/or sharey.
            The axis will have the same limits, ticks, and scale as the axis
            of the shared Axes.

        label : str
            A label for the returned Axes.

        Returns
        -------
        `~.axes.Axes`, or a subclass of `~.axes.Axes`
            The returned Axes class depends on the projection used. It is
            `~.axes.Axes` if rectilinear projection is used and
            `.projections.polar.PolarAxes` if polar projection is used.

        Other Parameters
        ----------------
        **kwargs
            This method also takes the keyword arguments for
            the returned Axes class. The keyword arguments for the
            rectilinear Axes class `~.axes.Axes` can be found in
            the following table but there might also be other keyword
            arguments if another projection is used, see the actual Axes
            class.

            %(Axes:kwdoc)s

        Notes
        -----
        In rare circumstances, `.add_axes` may be called with a single
        argument, an Axes instance already created in the present figure but
        not in the figure's list of Axes.

        See Also
        --------
        .Figure.add_subplot
        .pyplot.subplot
        .pyplot.axes
        .Figure.subplots
        .pyplot.subplots

        Examples
        --------
        Some simple examples::

            rect = l, b, w, h
            fig = plt.figure()
            fig.add_axes(rect)
            fig.add_axes(rect, frameon=False, facecolor='g')
            fig.add_axes(rect, polar=True)
            ax = fig.add_axes(rect, projection='polar')
            fig.delaxes(ax)
            fig.add_axes(ax)
        """
        if not len(args) and 'rect' not in kwargs:
            raise TypeError("add_axes() missing 1 required positional argument: 'rect'")
        elif 'rect' in kwargs:
            if len(args):
                raise TypeError("add_axes() got multiple values for argument 'rect'")
            args = (kwargs.pop('rect'), )
        if len(args) != 1:
            raise _api.nargs_error("add_axes", 1, len(args))

        if isinstance(args[0], _AxesBase):
            a, = args
            if a.get_figure(root=False) is not self:
                raise ValueError(
                    "The Axes must have been created in the present figure")
            if a in self.axes:  # already here: it becomes the current axes
                self.sca(a)
                return a
            return self._add_axes_back(a)
        rect, = args
        if isinstance(rect, BboxBase):
            rect = rect.bounds
        if not np.isfinite(rect).all():
            raise ValueError(f'all entries in rect must be finite not {rect}')
        projection_class, pkw = self._process_projection_requirements(**kwargs)
        if not np.iterable(rect):
            # matplotlib's Axes places itself at a subplot for a position
            # that is no sequence.
            SubplotSpec._from_subplot_args(self, (rect,))
            return self.add_subplot(rect, **kwargs)
        Bbox.from_bounds(*rect)  # As matplotlib's Axes takes the bounds.
        rect = tuple(map(float, rect))
        make = _native_maker(projection_class)
        if make is not None:
            native, pkw = make(self, pkw, rect=rect)
        elif projection_class.name == "rectilinear":
            native = _figure.add_axes(self._native, rect)
        else:
            from .projections._program import native_name
            native = _figure.add_axes_projection(self._native, rect,
                                                 native_name(projection_class))
        return self._new_axes(native, projection_class, pkw)

    @_docstring.interpd
    def add_subplot(self, *args, **kwargs):
        """
        Add an `~.axes.Axes` to the figure as part of a subplot arrangement.

        Call signatures::

           add_subplot(nrows, ncols, index, **kwargs)
           add_subplot(pos, **kwargs)
           add_subplot(ax)
           add_subplot()

        Parameters
        ----------
        *args : int, (int, int, *index*), or `.SubplotSpec`, default: (1, 1, 1)
            The position of the subplot described by one of

            - Three integers (*nrows*, *ncols*, *index*). The subplot will
              take the *index* position on a grid with *nrows* rows and
              *ncols* columns. *index* starts at 1 in the upper left corner
              and increases to the right.  *index* can also be a two-tuple
              specifying the (*first*, *last*) indices (1-based, and including
              *last*) of the subplot, e.g., ``fig.add_subplot(3, 1, (1, 2))``
              makes a subplot that spans the upper 2/3 of the figure.
            - A 3-digit integer. The digits are interpreted as if given
              separately as three single-digit integers, i.e.
              ``fig.add_subplot(235)`` is the same as
              ``fig.add_subplot(2, 3, 5)``. Note that this can only be used
              if there are no more than 9 subplots.
            - A `.SubplotSpec`.

            In rare circumstances, `.add_subplot` may be called with a single
            argument, a subplot Axes instance already created in the
            present figure but not in the figure's list of Axes.

        projection : {None, 'aitoff', 'hammer', 'lambert', 'mollweide', \
'polar', 'rectilinear', str}, optional
            The projection type of the subplot (`~.axes.Axes`). *str* is the
            name of a custom projection, see `~matplotlib.projections`. The
            default None results in a 'rectilinear' projection.

        polar : bool, default: False
            If True, equivalent to projection='polar'.

        axes_class : subclass type of `~.axes.Axes`, optional
            The `.axes.Axes` subclass that is instantiated.  This parameter
            is incompatible with *projection* and *polar*.  See
            :ref:`axisartist_users-guide-index` for examples.

        sharex, sharey : `~matplotlib.axes.Axes`, optional
            Share the x or y `~matplotlib.axis` with sharex and/or sharey.
            The axis will have the same limits, ticks, and scale as the axis
            of the shared Axes.

        label : str
            A label for the returned Axes.

        Returns
        -------
        `~.axes.Axes`

            The Axes of the subplot. The returned Axes can actually be an
            instance of a subclass, such as `.projections.polar.PolarAxes` for
            polar projections.

        Other Parameters
        ----------------
        **kwargs
            This method also takes the keyword arguments for the returned Axes
            base class; except for the *figure* argument. The keyword arguments
            for the rectilinear base class `~.axes.Axes` can be found in
            the following table but there might also be other keyword
            arguments if another projection is used.

            %(Axes:kwdoc)s

        See Also
        --------
        .Figure.add_axes
        .pyplot.subplot
        .pyplot.axes
        .Figure.subplots
        .pyplot.subplots

        Examples
        --------
        ::

            fig = plt.figure()

            fig.add_subplot(231)
            ax1 = fig.add_subplot(2, 3, 1)  # equivalent but more general

            fig.add_subplot(232, frameon=False)  # subplot with no frame
            fig.add_subplot(233, projection='polar')  # polar subplot
            fig.add_subplot(234, sharex=ax1)  # subplot sharing x-axis with ax1
            fig.add_subplot(235, facecolor="red")  # red subplot

            ax1.remove()  # delete ax1 from the figure
            fig.add_subplot(ax1)  # add ax1 back to the figure
        """
        if 'figure' in kwargs:
            # Axes itself allows for a 'figure' kwarg, but since we want to
            # bind the created Axes to self, it is not allowed here.
            raise _api.kwarg_error("add_subplot", "figure")

        if (len(args) == 1
                and isinstance(args[0], _AxesBase)
                and args[0].get_subplotspec()):
            ax = args[0]
            if ax.get_figure(root=False) is not self:
                raise ValueError("The Axes must have been created in "
                                 "the present figure")
            if ax in self.axes:  # already here: it becomes the current axes
                self.sca(ax)
                return ax
            return self._add_axes_back(ax)
        if not args:
            args = (1, 1, 1)
        # Normalize correct ijk values to (i, j, k) here so that
        # add_subplot(211) == add_subplot(2, 1, 1).  Invalid values will
        # trigger errors later (via SubplotSpec._from_subplot_args).
        if (len(args) == 1 and isinstance(args[0], Integral)
                and 100 <= args[0] <= 999):
            args = tuple(map(int, str(args[0])))
        projection_class, pkw = self._process_projection_requirements(**kwargs)
        subplotspec = SubplotSpec._from_subplot_args(self, args)
        if _gridspec_kept.in_python(subplotspec.get_gridspec(), self):
            # matplotlib places the Axes on its spec as it makes it.
            subplotspec.get_position(self)
        spec = subplotspec._native_spec()
        make = _native_maker(projection_class)
        if make is not None:
            with _construction.given_arguments(args, subplotspec):
                native, pkw = make(self, pkw, spec=spec)
        elif projection_class.name == "rectilinear":
            native = _gridspec.add_subplot(self._native, spec)
        else:
            from .projections._program import native_name
            native = _figure.add_subplot_spec_projection(
                self._native, spec, native_name(projection_class))
        ax = self._new_axes(native, projection_class, pkw)
        # The spec given, as matplotlib's Axes keeps it (``_subplotspec``).
        ax._place_on(subplotspec)
        if _gridspec_kept.placed_by_program(subplotspec):
            # Where the program's code puts it, as matplotlib's Axes asks.
            ax._set_position(subplotspec.get_position(self))
        return ax

    def subplots(self, nrows=1, ncols=1, *, sharex=False, sharey=False,
                 squeeze=True, width_ratios=None, height_ratios=None,
                 subplot_kw=None, gridspec_kw=None):
        """
        Add a set of subplots to this figure.

        This utility wrapper makes it convenient to create common layouts of
        subplots in a single call.

        Parameters
        ----------
        nrows, ncols : int, default: 1
            Number of rows/columns of the subplot grid.

        sharex, sharey : bool or {'none', 'all', 'row', 'col'}, default: False
            Controls sharing of x-axis (*sharex*) or y-axis (*sharey*):

            - True or 'all': x- or y-axis will be shared among all subplots.
            - False or 'none': each subplot x- or y-axis will be independent.
            - 'row': each subplot row will share an x- or y-axis.
            - 'col': each subplot column will share an x- or y-axis.

            When subplots have a shared x-axis along a column, only the x tick
            labels of the bottom subplot are created. Similarly, when subplots
            have a shared y-axis along a row, only the y tick labels of the
            first column subplot are created. To later turn other subplots'
            ticklabels on, use `~matplotlib.axes.Axes.tick_params`.

            When subplots have a shared axis that has units, calling
            `.Axis.set_units` will update each axis with the new units.

            Note that it is not possible to unshare axes.

        squeeze : bool, default: True
            - If True, extra dimensions are squeezed out from the returned
              array of Axes:

              - if only one subplot is constructed (nrows=ncols=1), the
                resulting single Axes object is returned as a scalar.
              - for Nx1 or 1xM subplots, the returned object is a 1D numpy
                object array of Axes objects.
              - for NxM, subplots with N>1 and M>1 are returned as a 2D array.

            - If False, no squeezing at all is done: the returned Axes object
              is always a 2D array containing Axes instances, even if it ends
              up being 1x1.

        width_ratios : array-like of length *ncols*, optional
            Defines the relative widths of the columns. Each column gets a
            relative width of ``width_ratios[i] / sum(width_ratios)``.
            If not given, all columns will have the same width.  Equivalent
            to ``gridspec_kw={'width_ratios': [...]}``.

        height_ratios : array-like of length *nrows*, optional
            Defines the relative heights of the rows. Each row gets a
            relative height of ``height_ratios[i] / sum(height_ratios)``.
            If not given, all rows will have the same height. Equivalent
            to ``gridspec_kw={'height_ratios': [...]}``.

        subplot_kw : dict, optional
            Dict with keywords passed to the `.Figure.add_subplot` call used to
            create each subplot.

        gridspec_kw : dict, optional
            Dict with keywords passed to the
            `~matplotlib.gridspec.GridSpec` constructor used to create
            the grid the subplots are placed on.

        Returns
        -------
        `~.axes.Axes` or array of Axes
            Either a single `~matplotlib.axes.Axes` object or an array of Axes
            objects if more than one subplot was created. The dimensions of the
            resulting array can be controlled with the *squeeze* keyword, see
            above.

        See Also
        --------
        .pyplot.subplots
        .Figure.add_subplot
        .pyplot.subplot

        Examples
        --------
        ::

            # First create some toy data:
            x = np.linspace(0, 2*np.pi, 400)
            y = np.sin(x**2)

            # Create a figure
            fig = plt.figure()

            # Create a subplot
            ax = fig.subplots()
            ax.plot(x, y)
            ax.set_title('Simple plot')

            # Create two subplots and unpack the output array immediately
            ax1, ax2 = fig.subplots(1, 2, sharey=True)
            ax1.plot(x, y)
            ax1.set_title('Sharing Y axis')
            ax2.scatter(x, y)

            # Create four polar Axes and access them through the returned array
            axes = fig.subplots(2, 2, subplot_kw=dict(projection='polar'))
            axes[0, 0].plot(x, y)
            axes[1, 1].scatter(x, y)

            # Share an X-axis with each column of subplots
            fig.subplots(2, 2, sharex='col')

            # Share a Y-axis with each row of subplots
            fig.subplots(2, 2, sharey='row')

            # Share both X- and Y-axes with all subplots
            fig.subplots(2, 2, sharex='all', sharey='all')

            # Note that this is the same as
            fig.subplots(2, 2, sharex=True, sharey=True)
        """
        gridspec_kw = dict(gridspec_kw or {})
        if height_ratios is not None:
            if 'height_ratios' in gridspec_kw:
                raise ValueError("'height_ratios' must not be defined both as "
                                 "parameter and as key in 'gridspec_kw'")
            gridspec_kw['height_ratios'] = height_ratios
        if width_ratios is not None:
            if 'width_ratios' in gridspec_kw:
                raise ValueError("'width_ratios' must not be defined both as "
                                 "parameter and as key in 'gridspec_kw'")
            gridspec_kw['width_ratios'] = width_ratios

        gs = self.add_gridspec(nrows, ncols, figure=self, **gridspec_kw)
        axs = gs.subplots(sharex=sharex, sharey=sharey, squeeze=squeeze,
                          subplot_kw=subplot_kw)
        return axs

    def subfigures(self, nrows=1, ncols=1, squeeze=True,
                   wspace=None, hspace=None,
                   width_ratios=None, height_ratios=None,
                   **kwargs):
        """
        Add a set of subfigures to this figure or subfigure.

        A subfigure has the same artist methods as a figure, and is logically
        the same as a figure, but cannot print itself.
        See :doc:`/gallery/subplots_axes_and_figures/subfigures`.

        .. versionchanged:: 3.10
            subfigures are now added in row-major order.

        Parameters
        ----------
        nrows, ncols : int, default: 1
            Number of rows/columns of the subfigure grid.

        squeeze : bool, default: True
            If True, extra dimensions are squeezed out from the returned
            array of subfigures.

        wspace, hspace : float, default: None
            The amount of width/height reserved for space between subfigures,
            expressed as a fraction of the average subfigure width/height.
            If not given, the values will be inferred from rcParams if using
            constrained layout (see `~.ConstrainedLayoutEngine`), or zero if
            not using a layout engine.

        width_ratios : array-like of length *ncols*, optional
            Defines the relative widths of the columns. Each column gets a
            relative width of ``width_ratios[i] / sum(width_ratios)``.
            If not given, all columns will have the same width.

        height_ratios : array-like of length *nrows*, optional
            Defines the relative heights of the rows. Each row gets a
            relative height of ``height_ratios[i] / sum(height_ratios)``.
            If not given, all rows will have the same height.
        """
        gs = GridSpec(nrows=nrows, ncols=ncols, figure=self,
                      wspace=wspace, hspace=hspace,
                      width_ratios=width_ratios,
                      height_ratios=height_ratios,
                      left=0, right=1, bottom=0, top=1)
        for row in range(nrows):
            for col in range(ncols):
                # Each placed by the ratios as matplotlib places it.
                _gridspec_kept.subfigure_bounds(gs[row, col])
        natives = _figure.subfigures(
            self._native, (nrows, ncols), _gridspec_kept.held((wspace, hspace)),
            None if width_ratios is None
            else _gridspec_kept.native_ratios(width_ratios, ncols),
            None if height_ratios is None
            else _gridspec_kept.native_ratios(height_ratios, nrows))
        sfarr = np.empty((nrows, ncols), dtype=object)
        for i, native in enumerate(natives):
            row, col = divmod(i, ncols)
            sfarr[row, col] = SubFigure._from_native(self, native, gs[row, col],
                                                     **kwargs)
            self.subfigs.append(sfarr[row, col])
            sfarr[row, col]._listed_among = self.subfigs
        if _gridspec_kept.in_python(gs, None):
            # The layout engine lays the grid's spacing out as matplotlib's.
            _layout_checks.refresh_layout(self)
        self.stale = True
        if self.get_layout_engine() is None and (wspace is not None or
                                                 hspace is not None):
            # matplotlib places them in the grid's cells then, failing on
            # the spacing pltrs (Rust) cannot hold.
            if _gridspec_kept.in_python(gs, self):
                gs.get_grid_positions(self)

        if squeeze:
            # Discarding unneeded dimensions that equal 1.  If we only have one
            # subfigure, just return it instead of a 1-element array.
            return sfarr.item() if sfarr.size == 1 else sfarr.squeeze()
        else:
            # Returned axis array will be always 2-d, even if nrows=ncols=1.
            return sfarr

    def add_subfigure(self, subplotspec, **kwargs):
        """
        Add a `.SubFigure` to the figure as part of a subplot arrangement.

        Parameters
        ----------
        subplotspec : `.gridspec.SubplotSpec`
            Defines the region in a parent gridspec where the subfigure will
            be placed.

        Returns
        -------
        `.SubFigure`

        Other Parameters
        ----------------
        **kwargs
            Are passed to the `.SubFigure` object.

        See Also
        --------
        .Figure.subfigures
        """
        sf = SubFigure(self, subplotspec, **kwargs)
        sf._add_to_figure(self, False)
        self.subfigs.append(sf)
        sf._listed_among = self.subfigs
        if _gridspec_kept.in_python(subplotspec.get_gridspec(), None):
            # The layout engine lays the grid's spacing out as matplotlib's.
            _layout_checks.refresh_layout(self)
        self.stale = True
        return sf

    @property
    def subplotpars(self):
        """The `.SubplotParams` of the figure."""
        self._subplotpars = _figure_subplotpars(
            self._native, self.__dict__.get("_subplotpars"))
        return self._subplotpars

    def subplots_adjust(self, left=None, bottom=None, right=None, top=None,
                        wspace=None, hspace=None):
        """
        Adjust the subplot layout parameters.

        Unset parameters are left unmodified; initial values are given by
        :rc:`figure.subplot.[name]`.

        .. plot:: _embedded_plots/figure_subplots_adjust.py

        Parameters
        ----------
        left : float, optional
            The position of the left edge of the subplots,
            as a fraction of the figure width.
        right : float, optional
            The position of the right edge of the subplots,
            as a fraction of the figure width.
        bottom : float, optional
            The position of the bottom edge of the subplots,
            as a fraction of the figure height.
        top : float, optional
            The position of the top edge of the subplots,
            as a fraction of the figure height.
        wspace : float, optional
            The width of the padding between subplots,
            as a fraction of the average Axes width.
        hspace : float, optional
            The height of the padding between subplots,
            as a fraction of the average Axes height.
        """
        if (self.get_layout_engine() is not None and
                not self.get_layout_engine().adjust_compatible):
            _api.warn_external(
                "This figure was using a layout engine that is "
                "incompatible with subplots_adjust and/or tight_layout; "
                "not calling subplots_adjust.")
            return
        self.subplotpars.update(left, bottom, right, top, wspace, hspace)
        _figure.subplots_adjust(self._native, _gridspec_kept.held(
            (left, bottom, right, top, wspace, hspace)))
        # The subplots placed again, as matplotlib places them.
        _gridspec_kept.check_positions(self.axes, self)

    @_docstring.interpd
    def colorbar(self, mappable, cax=None, ax=None, use_gridspec=True,
                 **kwargs):
        """
        Add a colorbar to a plot.

        Parameters
        ----------
        mappable
            The `matplotlib.colorizer.ColorizingArtist` (i.e., `.AxesImage`,
            `.ContourSet`, etc.) described by this colorbar.  This argument is
            mandatory for the `.Figure.colorbar` method but optional for the
            `.pyplot.colorbar` function, which sets the default to the current
            image.

            Note that one can create a `.colorizer.ColorizingArtist` "on-the-fly"
            to generate colorbars not attached to a previously drawn artist, e.g.
            ::

                cr = colorizer.Colorizer(norm=norm, cmap=cmap)
                fig.colorbar(colorizer.ColorizingArtist(cr), ax=ax)

        cax : `~matplotlib.axes.Axes`, optional
            Axes into which the colorbar will be drawn.  If `None`, then a new
            Axes is created and the space for it will be stolen from the Axes(s)
            specified in *ax*.

        ax : `~matplotlib.axes.Axes` or iterable or `numpy.ndarray` of Axes, optional
            The one or more parent Axes from which space for a new colorbar Axes
            will be stolen. This parameter is only used if *cax* is not set.

            Defaults to the Axes that contains the mappable used to create the
            colorbar.

        use_gridspec : bool, optional
            If *cax* is ``None``, a new *cax* is created as an instance of
            Axes.  If *ax* is positioned with a subplotspec and *use_gridspec*
            is ``True``, then *cax* is also positioned with a subplotspec.

        Returns
        -------
        colorbar : `~matplotlib.colorbar.Colorbar`

        Other Parameters
        ----------------
        %(_make_axes_kw_doc)s
        %(_colormap_kw_doc)s

        Notes
        -----
        If *mappable* is a `~.contour.ContourSet`, its *extend* kwarg is
        included automatically.

        The *shrink* kwarg provides a simple way to scale the colorbar with
        respect to the Axes. Note that if *cax* is specified, it determines the
        size of the colorbar, and *shrink* and *aspect* are ignored.

        For more precise control, you can manually specify the positions of the
        axes objects in which the mappable and the colorbar are drawn.  In this
        case, do not use any of the Axes properties kwargs.

        It is known that some vector graphics viewers (svg and pdf) render
        white gaps between segments of the colorbar.  This is due to bugs in
        the viewers, not Matplotlib.  As a workaround, the colorbar can be
        rendered with overlapping segments::

            cbar = colorbar()
            cbar.solids.set_edgecolor("face")
            draw()

        However, this has negative consequences in other circumstances, e.g.
        with semi-transparent images (alpha < 1) and colorbar extensions;
        therefore, this workaround is not used by default (see issue #1188).

        """
        from .colorbar import Colorbar

        def warn_if_elsewhere():
            # Once the colorbar's Axes exists, as matplotlib warns.
            host = getattr(mappable, "get_figure", None)
            host = None if host is None else host(root=True)
            if host is not None and host is not self.figure:
                _api.warn_external(
                    f'Adding colorbar to a different Figure {host!r} than '
                    f'{self.figure!r} which fig.colorbar is called on.')

        if ax is None:
            ax = getattr(mappable, "axes", None)
        if cax is not None:
            warn_if_elsewhere()
            for key in ('fraction', 'pad', 'shrink', 'aspect', 'anchor',
                        'panchor'):
                kwargs.pop(key, None)
            return Colorbar(cax, mappable, **kwargs)
        if ax is None:
            raise ValueError(
                'Unable to determine Axes to steal space for Colorbar. '
                'Either provide the *cax* argument to use as the Axes for '
                'the Colorbar, provide the *ax* argument to steal space '
                'from it, or add *mappable* to an Axes.')
        if isinstance(ax, np.ndarray):
            parents = list(ax.flat)
        elif np.iterable(ax):
            parents = list(ax)
        else:
            parents = ax  # which may give the colorbar a cell of its grid
        return Colorbar._beside(mappable, parents, use_gridspec=use_gridspec,
                                made=warn_if_elsewhere, **kwargs)

    def _set_artist_props(self, a):
        # matplotlib 3.11.2's, under the Matplotlib License.
        if a != self:
            a.set_figure(self)
        a.stale_callback = _stale_figure_callback
        a.set_transform(self.transSubfigure)

    def add_artist(self, artist, clip=False):
        """
        Add an `.Artist` to the figure.

        Usually artists are added to `~.axes.Axes` objects using
        `.Axes.add_artist`; this method can be used in the rare cases where
        one needs to add artists directly to the figure instead.

        Parameters
        ----------
        artist : `~matplotlib.artist.Artist`
            The artist to add to the figure. If the added artist has no
            transform previously set, its transform will be set to
            ``figure.transSubfigure``.
        clip : bool, default: False
            Whether the added artist should be clipped by the figure patch.

        Returns
        -------
        `~matplotlib.artist.Artist`
            The added artist.
        """
        # matplotlib's gives it the figure first, which fails on what is no
        # artist, on a (sub)figure (whose figure is set at instantiation)
        # and on an artist of another (sub)figure.
        if not isinstance(artist, Artist) or (
                isinstance(artist, FigureBase)
                and artist.get_figure(root=True) is not self.get_figure(root=True)):
            artist.set_figure(self)
        figure = artist.get_figure(root=False)
        if figure is not None and figure is not self:
            raise RuntimeError("Can not put single artist in more than one figure")
        if _foreign_children.held_in_axes(self, artist):
            return _foreign_children.relist_in_figure(self, artist, clip)
        added = artist._add_to_figure(self, clip)
        if not artist.is_transform_set():
            # One only given the figure (`set_figure`) before.
            artist.set_transform(self.transSubfigure)
        _artist_clipping.take_parent_defaults(artist, self, clip)
        return added

    @property
    def artists(self):
        """The artists added to the figure itself (`add_artist`), in a list
        drawing those put in it (see `._figure_artists`)."""
        listed = {id(artist) for name in ("_lines", "_patches")
                  for artist in vars(self).get(name, ())}
        return _figure_artists.DrawnList(self, [
            _foreign_children.listed(artist) for artist in _handles.wrap_all(
                _figure.artists(self._native), Artist, self.figure)
            if id(artist) not in listed])

    @artists.setter
    def artists(self, artists):
        _figure_artists.replace(self.artists, artists)

    @property
    def patches(self):
        """The patches put in the figure's list, drawn on it as they are
        (see `._figure_artists`)."""
        return vars(self).setdefault("_patches", _figure_artists.DrawnList(self))

    @patches.setter
    def patches(self, patches):
        vars(self)["_patches"] = _figure_artists.replace(self.patches, patches)

    @property
    def lines(self):
        """The lines put in the figure's list, drawn on it as they are (see
        `._figure_artists`)."""
        return vars(self).setdefault("_lines", _figure_artists.DrawnList(self))

    @lines.setter
    def lines(self, lines):
        vars(self)["_lines"] = _figure_artists.replace(self.lines, lines)

    @_docstring.interpd
    def legend(self, *args, **kwargs):
        """
        Place a legend on the figure.

        Call signatures::

            legend()
            legend(handles, labels)
            legend(handles=handles)
            legend(labels)

        The call signatures correspond to the following different ways to use
        this method:

        **1. Automatic detection of elements to be shown in the legend**

        The elements to be added to the legend are automatically determined,
        when you do not pass in any extra arguments.

        In this case, the labels are taken from the artist. You can specify
        them either at artist creation or by calling the
        :meth:`~.Artist.set_label` method on the artist::

            ax.plot([1, 2, 3], label='Inline label')
            fig.legend()

        or::

            line, = ax.plot([1, 2, 3])
            line.set_label('Label via method')
            fig.legend()

        Specific lines can be excluded from the automatic legend element
        selection by defining a label starting with an underscore.
        This is default for all artists, so calling `.Figure.legend` without
        any arguments and without setting the labels manually will result in
        no legend being drawn.


        **2. Explicitly listing the artists and labels in the legend**

        For full control of which artists have a legend entry, it is possible
        to pass an iterable of legend artists followed by an iterable of
        legend labels respectively::

            fig.legend([line1, line2, line3], ['label1', 'label2', 'label3'])


        **3. Explicitly listing the artists in the legend**

        This is similar to 2, but the labels are taken from the artists'
        label properties. Example::

            line1, = ax1.plot([1, 2, 3], label='label1')
            line2, = ax2.plot([1, 2, 3], label='label2')
            fig.legend(handles=[line1, line2])


        **4. Labeling existing plot elements**

        .. admonition:: Discouraged

            This call signature is discouraged, because the relation between
            plot elements and labels is only implicit by their order and can
            easily be mixed up.

        To make a legend for all artists on all Axes, call this function with
        an iterable of strings, one for each legend item. For example::

            fig, (ax1, ax2) = plt.subplots(1, 2)
            ax1.plot([1, 3, 5], color='blue')
            ax2.plot([2, 4, 6], color='red')
            fig.legend(['the blues', 'the reds'])


        Parameters
        ----------
        handles : list of `.Artist`, optional
            A list of Artists (lines, patches) to be added to the legend.
            Use this together with *labels*, if you need full control on what
            is shown in the legend and the automatic mechanism described above
            is not sufficient.

            The length of handles and labels should be the same in this
            case. If they are not, they are truncated to the smaller length.

        labels : list of str, optional
            A list of labels to show next to the artists.
            Use this together with *handles*, if you need full control on what
            is shown in the legend and the automatic mechanism described above
            is not sufficient.

        Returns
        -------
        `~matplotlib.legend.Legend`

        Other Parameters
        ----------------
        %(_legend_kw_figure)s

        See Also
        --------
        .Axes.legend

        Notes
        -----
        Some artists are not supported by this function.  See
        :ref:`legend_guide` for details.
        """
        from . import legend as mlegend
        handles, labels, kwargs = mlegend._parse_legend_args(
            self.axes,
            *args,
            **kwargs
        )
        # explicitly set the bbox transform if the user hasn't.
        kwargs.setdefault("bbox_transform", self.transSubfigure)
        l = mlegend.Legend(self, handles, labels, **kwargs)
        self.legends.append(l)
        self.stale = True
        return l

    @property
    def legends(self):
        """The legends of the figure, in the order they were put in it:
        putting a legend made for the figure in the list draws it (see
        `._figure_artists`)."""
        from . import legend as mlegend
        from ._native import legend as _legend
        return _figure_artists.LegendList(self, _handles.wrap_all(
            _legend.figure_legends(self._native), mlegend.Legend, self.figure))

    @legends.setter
    def legends(self, legends):
        _figure_artists.replace(self.legends, legends)

    def clear(self, keep_observers=False):
        """
        Clear the figure.

        Parameters
        ----------
        keep_observers : bool, default: False
            Set *keep_observers* to True if, for example,
            a gui widget is tracking the Axes in the figure.
        """
        for subfig in self.subfigs:
            subfig.clear(keep_observers=keep_observers)
        for ax in tuple(self.axes):
            ax._keep_trees()
            ax.clear()
            self._axes_removed(ax)
        # matplotlib's clear empties the figure's list of texts, which its
        # super labels keep as their figure's: removing one fails on it.
        for label in (self._suptitle, self._supxlabel, self._supylabel):
            if label is not None:
                label._out_of_lists = True
        self.__dict__.pop("_super_texts_given", None)
        _figure.clear(self._native)
        self.subfigs = []
        if not keep_observers:
            self._forget_axobservers()
        self.stale = True

    def delaxes(self, ax):
        """
        Remove the `~.axes.Axes` *ax* from the figure; update the current Axes.
        """
        # Taken out of matplotlib's stack of the root figure's Axes, then out
        # of the list of this (sub)figure's own, which refuse Axes they lack.
        if ax not in self.get_figure(root=True).axes:
            raise KeyError(ax)
        if ax not in self._localaxes:
            raise ValueError("list.remove(x): x not in list")
        # Out of the figure but still of it, as matplotlib's `delaxes` leaves
        # the Axes (plotted on, added back), which `ax.remove()` then decouples.
        ax._keep_trees()
        _artist.remove(ax._native)
        self._axes_removed(ax)

    def set_figure(self, fig):
        """
        .. deprecated:: 3.10
            Currently this method will raise an exception if *fig* is anything other
            than the root `.Figure` this (Sub)Figure is on.  In future it will always
            raise an exception.
        """
        no_switch = ("The parent and root figures of a (Sub)Figure are set at "
                     "instantiation and cannot be changed.")
        if fig is self.figure:
            _api.warn_deprecated(
                "3.10",
                message=(f"{no_switch} From Matplotlib 3.12 this operation will raise "
                         "an exception."))
            return
        raise ValueError(no_switch)

    figure = property(functools.partial(get_figure, root=True), set_figure,
                      doc=("The root `Figure`.  To get the parent of a `SubFigure`, "
                           "use the `get_figure` method."))

    def clf(self, keep_observers=False):
        """
        [*Discouraged*] Alias for the `clear()` method.

        .. admonition:: Discouraged

            The use of ``clf()`` is discouraged. Use ``clear()`` instead.

        Parameters
        ----------
        keep_observers : bool, default: False
            Set *keep_observers* to True if, for example,
            a gui widget is tracking the Axes in the figure.
        """
        return self.clear(keep_observers=keep_observers)

    def _get_renderer(self):
        """The renderer of the root figure's canvas: the canvas' own, or the
        Agg renderer of the figure for a canvas without one."""
        figure = self.get_figure(root=True)
        canvas = figure.canvas
        if hasattr(canvas, 'get_renderer'):
            return canvas.get_renderer()
        from .backends.backend_agg import RendererAgg
        renderer = RendererAgg(*canvas.get_width_height(physical=True),
                               figure.dpi)
        renderer._figure = figure
        return renderer


@_docstring.interpd
class SubFigure(FigureBase):
    """
    Logical figure that can be placed inside a figure.

    See :ref:`figure-api-subfigure` for an index of methods on this class.
    Typically instantiated using `.Figure.add_subfigure` or
    `.SubFigure.add_subfigure`, or `.SubFigure.subfigures`.  A subfigure has
    the same methods as a figure except for those particularly tied to the size
    or dpi of the figure, and is confined to a prescribed region of the figure.
    For example the following puts two subfigures side-by-side::

        fig = plt.figure()
        sfigs = fig.subfigures(1, 2)
        axsL = sfigs[0].subplots(1, 2)
        axsR = sfigs[1].subplots(2, 1)

    See :doc:`/gallery/subplots_axes_and_figures/subfigures`
    """

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    #: Kept as on every figure, but a subfigure composites its images as
    #: its root figure says.
    suppressComposite = None

    @property
    def subplotpars(self):
        """The `.SubplotParams` of the subfigure: its parent's."""
        return self._parent.subplotpars

    @property
    def images(self):
        """The figure images of the subfigure: none (`~.Figure.figimage`
        places images on the root figure)."""
        return []

    def __init__(self, parent, subplotspec, *,
                 facecolor=None,
                 edgecolor=None,
                 linewidth=0.0,
                 frameon=None,
                 **kwargs):
        """
        Parameters
        ----------
        parent : `.Figure` or `.SubFigure`
            Figure or subfigure that contains the SubFigure.  SubFigures
            can be nested.

        subplotspec : `.gridspec.SubplotSpec`
            Defines the region in a parent gridspec where the subfigure will
            be placed.

        facecolor : default: ``"none"``
            The figure patch face color; transparent by default.

        edgecolor : default: :rc:`figure.edgecolor`
            The figure patch edge color.

        linewidth : float
            The linewidth of the frame (i.e. the edge linewidth of the figure
            patch).

        frameon : bool, default: :rc:`figure.frameon`
            If ``False``, suppress drawing the figure background patch.

        Other Parameters
        ----------------
        **kwargs : `.SubFigure` properties, optional

            %(SubFigure:kwdoc)s
        """
        if not isinstance(parent, FigureBase):
            parent._root_figure  # noqa: B018  (matplotlib's first read of it)
        # Placed by the ratios of its grid as matplotlib places it.
        _gridspec_kept.subfigure_bounds(subplotspec)
        linewidth = _background_patch.checked_style(
            "none" if facecolor is None else facecolor,
            rcParams['figure.edgecolor'] if edgecolor is None else edgecolor,
            linewidth)
        # Made for *parent* but not drawn there until `.Figure.add_subfigure`
        # adds it, as matplotlib's constructor leaves that to it.
        native = _figure.add_subfigure(parent._native,
                                       subplotspec._native_spec())
        _artist.remove(native)
        self._adopt_native(parent, native, subplotspec, facecolor=facecolor,
                           edgecolor=edgecolor, linewidth=linewidth,
                           frameon=frameon, **kwargs)

    @classmethod
    def _from_native(cls, parent, native, subplotspec, **kwargs):
        """The subfigure of *parent* around its native frame *native*."""
        subfigure = cls.__new__(cls)
        subfigure._adopt_native(parent, native, subplotspec, **kwargs)
        return subfigure

    def _adopt_native(self, parent, native, subplotspec, *, facecolor=None,
                      edgecolor=None, linewidth=0.0, frameon=None, **kwargs):
        """Become the subfigure of *parent* around *native*, styled by the
        keyword arguments."""
        self._parent = parent
        self._subplotspec = subplotspec
        self.subfigs = []
        _handles.adopt(self, native, parent.figure)
        self._figure._subfigures_made = True
        if facecolor is not None:
            _figure.set_facecolor(native, facecolor)
        if edgecolor is not None:
            _figure.set_edgecolor(native, edgecolor)
        if linewidth:
            _figure.set_linewidth(native, linewidth)
        if frameon is not None:
            _figure.set_frameon(native, bool(frameon))
        self._internal_update(kwargs)

    @property
    def _artists(self):
        # The root figure keeps the objects of all artists (`._handles`).
        return self._figure._artists

    def remove(self):
        """Remove the subfigure, and its artists, from its parent."""
        super().remove()
        # Still the figure of the artists removed with it, as matplotlib's.
        self._figure._artists[_artist.id(self._native)] = self
        # Out of the list of subfigures it was added to, as matplotlib's
        # removal method takes it out, which its parent's clear replaced.
        self.__dict__.get("_listed_among", self._parent.subfigs).remove(self)

    def get_constrained_layout(self):
        """
        Return whether constrained layout is being used.

        See :ref:`constrainedlayout_guide`.
        """
        return self._parent.get_constrained_layout()

    def get_constrained_layout_pads(self, relative=False):
        """
        Get padding for ``constrained_layout``.

        Returns a list of ``w_pad, h_pad`` in inches and
        ``wspace`` and ``hspace`` as fractions of the subplot.

        See :ref:`constrainedlayout_guide`.

        Parameters
        ----------
        relative : bool
            If `True`, then convert from inches to figure relative.
        """
        return self._parent.get_constrained_layout_pads(relative=relative)

    def get_layout_engine(self):
        return self._parent.get_layout_engine()

    @property
    def axes(self):
        """
        List of Axes in the SubFigure.  You can access and modify the Axes
        in the SubFigure through this list.

        Modifying this list has no effect. Instead, use `~.SubFigure.add_axes`,
        `~.SubFigure.add_subplot` or `~.SubFigure.delaxes` to add or remove an
        Axes.

        Note: The `.SubFigure.axes` property and `~.SubFigure.get_axes` method
        are equivalent.
        """
        return self._listed_axes()

    get_axes = axes.fget

    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    @property
    def canvas(self):
        return self._parent.canvas

    @property
    def dpi(self):
        return self._parent.dpi

    @dpi.setter
    def dpi(self, value):
        self._parent.dpi = value

    def get_dpi(self):
        """
        Return the resolution of the parent figure in dots-per-inch as a float.
        """
        return self._parent.dpi

    def set_dpi(self, val):
        """
        Set the resolution of parent figure in dots-per-inch.

        Parameters
        ----------
        val : float
        """
        self._parent.dpi = val
        self.stale = True


@_docstring.interpd
class Figure(FigureLayout, FigureImagesMixin, RootFigureMixin, FigureBase):
    """
    The top level container for all the plot elements.

    See `matplotlib.figure` for an index of class methods.

    Attributes
    ----------
    patch
        The `.Rectangle` instance representing the figure background patch.

    suppressComposite
        For multiple images, the figure will make composite images
        depending on the renderer option_image_nocomposite function.  If
        *suppressComposite* is a boolean, this will override the renderer.
    """

    def __str__(self):
        width, height = self.get_size_inches() * self.dpi
        return "Figure(%gx%g)" % (width, height)

    def draw(self, renderer):
        # docstring inherited
        native = _figure_draws.take(self)
        if native is not None:
            # A canvas drawing the figure through its class' own draw
            # (`._figure_draws`): pltrs (Rust) draws it here.
            native()
            return
        super().draw(renderer)

    def clear(self, keep_observers=False):
        # docstring inherited
        super().clear(keep_observers=keep_observers)

    def __repr__(self):
        width, height = self.get_size_inches() * self.dpi
        return "<{clsname} size {h:g}x{w:g} with {naxes} Axes>".format(
            clsname=self.__class__.__name__,
            h=width, w=height,
            naxes=len(self.axes),
        )

    def __init__(self, figsize=None, dpi=None, *, facecolor=None,
                 edgecolor=None, linewidth=0.0, frameon=None,
                 subplotpars=None, tight_layout=None,
                 constrained_layout=None, layout=None, **kwargs):
        """
        Parameters
        ----------
        figsize : (float, float) or (float, float, str), default: :rc:`figure.figsize`
            The figure dimensions. This can be

            - a tuple ``(width, height, unit)``, where *unit* is one of "in" (inch),
              "cm" (centimenter), "px" (pixel).
            - a tuple ``(width, height)``, which is interpreted in inches, i.e. as
              ``(width, height, "in")``.

            One of *width* or *height* may be ``None``; the respective value is
            taken from :rc:`figure.figsize`.

        dpi : float, default: :rc:`figure.dpi`
            Dots per inch.

        facecolor : default: :rc:`figure.facecolor`
            The figure patch facecolor.

        edgecolor : default: :rc:`figure.edgecolor`
            The figure patch edge color.

        linewidth : float
            The linewidth of the frame (i.e. the edge linewidth of the figure
            patch).

        frameon : bool, default: :rc:`figure.frameon`
            If ``False``, suppress drawing the figure background patch.

        subplotpars : `~matplotlib.gridspec.SubplotParams`
            Subplot parameters. If not given, the default subplot
            parameters :rc:`figure.subplot.*` are used.

        tight_layout : bool or dict, default: :rc:`figure.autolayout`
            Whether to use the tight layout mechanism. See `.set_tight_layout`.

            .. admonition:: Discouraged

                The use of this parameter is discouraged. Please use
                ``layout='tight'`` instead for the common case of
                ``tight_layout=True`` and use `.set_tight_layout` otherwise.

        constrained_layout : bool, default: :rc:`figure.constrained_layout.use`
            This is equal to ``layout='constrained'``.

            .. admonition:: Discouraged

                The use of this parameter is discouraged. Please use
                ``layout='constrained'`` instead.

        layout : {'constrained', 'compressed', 'tight', 'none', `.LayoutEngine`, \
None}, default: None
            The layout mechanism for positioning of plot elements to avoid
            overlapping Axes decorations (labels, ticks, etc). Note that
            layout managers can have significant performance penalties.

            - 'constrained': The constrained layout solver adjusts Axes sizes
              to avoid overlapping Axes decorations.  Can handle complex plot
              layouts and colorbars, and is thus recommended.

              See :ref:`constrainedlayout_guide` for examples.

            - 'compressed': uses the same algorithm as 'constrained', but
              removes extra space between fixed-aspect-ratio Axes.  Best for
              simple grids of Axes.

            - 'tight': Use the tight layout mechanism. This is a relatively
              simple algorithm that adjusts the subplot parameters so that
              decorations do not overlap.

              See :ref:`tight_layout_guide` for examples.

            - 'none': Do not use a layout engine.

            - A `.LayoutEngine` instance. Builtin layout classes are
              `.ConstrainedLayoutEngine` and `.TightLayoutEngine`, more easily
              accessible by 'constrained' and 'tight'.  Passing an instance
              allows third parties to provide their own layout engine.

            If not given, fall back to using the parameters *tight_layout* and
            *constrained_layout*, including their config defaults
            :rc:`figure.autolayout` and :rc:`figure.constrained_layout.use`.

        Other Parameters
        ----------------
        **kwargs : `.Figure` properties, optional

            %(Figure:kwdoc)s
        """
        if figsize is None:
            figsize = rcParams['figure.figsize']
        if dpi is None:
            dpi = rcParams['figure.dpi']
        width, height = _parse_figsize(figsize, dpi)
        if not all(math.isfinite(v) and v > 0 for v in (width, height)):
            raise ValueError(
                f'figure size must be positive finite not {figsize}')
        np.float64(1.0) * dpi  # matplotlib's ``Affine2D().scale(dpi)``.
        linewidth = _background_patch.checked_style(
            rcParams['figure.facecolor'] if facecolor is None else facecolor,
            rcParams['figure.edgecolor'] if edgecolor is None else edgecolor,
            linewidth)
        self._native = _figure.new(float(width), float(height), float(dpi))
        _values.remember(self, "dpi", dpi)
        self._figure = self
        self._artists = {_artist.id(self._native): self}
        _sharing.track(self)
        _transform_trees.follow_evaluations(self)
        self.subfigs = []
        FigureCanvasBase(self)
        if facecolor is not None:
            self.set_facecolor(facecolor)
        if edgecolor is not None:
            _figure.set_edgecolor(self._native, edgecolor)
        if linewidth:
            _figure.set_linewidth(self._native, linewidth)
        if frameon is not None:
            _figure.set_frameon(self._native, bool(frameon))
        self._layout_engine = None
        if layout is not None:
            if (tight_layout is not None):
                _api.warn_external(
                    "The Figure parameters 'layout' and 'tight_layout' cannot "
                    "be used together. Please use 'layout' only.")
            if (constrained_layout is not None):
                _api.warn_external(
                    "The Figure parameters 'layout' and 'constrained_layout' "
                    "cannot be used together. Please use 'layout' only.")
            self.set_layout_engine(layout=layout)
        elif tight_layout is not None:
            if constrained_layout is not None:
                _api.warn_external(
                    "The Figure parameters 'tight_layout' and "
                    "'constrained_layout' cannot be used together. Please use "
                    "'layout' parameter")
            self.set_layout_engine(layout='tight')
            if isinstance(tight_layout, dict):
                self.get_layout_engine().set(**tight_layout)
        elif constrained_layout is not None:
            if isinstance(constrained_layout, dict):
                self.set_layout_engine(layout='constrained')
                self.get_layout_engine().set(**constrained_layout)
            elif constrained_layout:
                self.set_layout_engine(layout='constrained')
        else:
            # everything is None, so use default:
            self.set_layout_engine(layout=layout)
        if subplotpars is not None:
            # matplotlib's figure keeps the parameters given, which its
            # ``clear`` resets to the rc defaults.
            subplotpars.reset()
            if isinstance(subplotpars, SubplotParams):
                self._subplotpars = _figure_subplotpars(self._native, subplotpars)
        self._internal_update(kwargs)

    @property
    def number(self):
        """The figure id, used to identify figures in `.pyplot`."""
        if hasattr(self, '_number'):
            return self._number
        else:
            raise AttributeError(
                "'Figure' object has no attribute 'number'. In the future this"
                "will change to returning 'None' instead.")

    @number.setter
    def number(self, num):
        _api.warn_deprecated(
            "3.10",
            message="Changing 'Figure.number' is deprecated since %(since)s and "
                    "will raise an error starting %(removal)s")
        self._number = num

    def get_tight_layout(self):
        """Return whether `.Figure.tight_layout` is called when drawing."""
        return isinstance(self.get_layout_engine(), TightLayoutEngine)

    @_api.deprecated("3.6", alternative="set_layout_engine",
                     pending=True)
    def set_tight_layout(self, tight):
        """
        Set whether and how `.Figure.tight_layout` is called when drawing.

        Parameters
        ----------
        tight : bool or dict with keys "pad", "w_pad", "h_pad", "rect" or None
            If a bool, sets whether to call `.Figure.tight_layout` upon drawing.
            If ``None``, use :rc:`figure.autolayout` instead.
            If a dict, pass it as kwargs to `.Figure.tight_layout`, overriding the
            default paddings.
        """
        if tight is None:
            tight = rcParams['figure.autolayout']
        _tight = 'tight' if bool(tight) else 'none'
        _tight_parameters = tight if isinstance(tight, dict) else {}
        self.set_layout_engine(_tight, **_tight_parameters)

    def get_constrained_layout(self):
        """
        Return whether constrained layout is being used.

        See :ref:`constrainedlayout_guide`.
        """
        return isinstance(self.get_layout_engine(), ConstrainedLayoutEngine)

    @_api.deprecated("3.6", alternative="set_layout_engine('constrained')",
                     pending=True)
    def set_constrained_layout(self, constrained):
        """
        Set whether ``constrained_layout`` is used upon drawing.

        If None, :rc:`figure.constrained_layout.use` value will be used.

        When providing a dict containing the keys ``w_pad``, ``h_pad``
        the default ``constrained_layout`` paddings will be
        overridden.  These pads are in inches and default to 3.0/72.0.
        ``w_pad`` is the width padding and ``h_pad`` is the height padding.

        Parameters
        ----------
        constrained : bool or dict or None
        """
        if constrained is None:
            constrained = rcParams['figure.constrained_layout.use']
        _constrained = 'constrained' if bool(constrained) else 'none'
        _parameters = constrained if isinstance(constrained, dict) else {}
        self.set_layout_engine(_constrained, **_parameters)

    @_api.deprecated("3.6", alternative="fig.get_layout_engine().get()",
                     pending=True)
    def get_constrained_layout_pads(self, relative=False):
        """
        Get padding for ``constrained_layout``.

        Returns a list of ``w_pad, h_pad`` in inches and
        ``wspace`` and ``hspace`` as fractions of the subplot.
        All values are None if ``constrained_layout`` is not used.

        See :ref:`constrainedlayout_guide`.

        Parameters
        ----------
        relative : bool
            If `True`, then convert from inches to figure relative.
        """
        if not isinstance(self.get_layout_engine(), ConstrainedLayoutEngine):
            return None, None, None, None
        info = self.get_layout_engine().get()
        w_pad = info['w_pad']
        h_pad = info['h_pad']
        wspace = info['wspace']
        hspace = info['hspace']

        if relative and (w_pad is not None or h_pad is not None):
            renderer = self._get_renderer()
            dpi = renderer.dpi
            w_pad = w_pad * dpi / renderer.width
            h_pad = h_pad * dpi / renderer.height

        return w_pad, h_pad, wspace, hspace

    def set_figwidth(self, val, forward=True):
        """
        Set the width of the figure in inches.

        Parameters
        ----------
        val : float
        forward : bool
            See `set_size_inches`.

        See Also
        --------
        matplotlib.figure.Figure.set_figheight
        matplotlib.figure.Figure.set_size_inches
        """
        self.set_size_inches(val, self.get_figheight(), forward=forward)

    def set_figheight(self, val, forward=True):
        """
        Set the height of the figure in inches.

        Parameters
        ----------
        val : float
        forward : bool
            See `set_size_inches`.

        See Also
        --------
        matplotlib.figure.Figure.set_figwidth
        matplotlib.figure.Figure.set_size_inches
        """
        self.set_size_inches(self.get_figwidth(), val, forward=forward)

    def get_figwidth(self):
        """Return the figure width in inches."""
        return self.get_size_inches()[0]

    def get_figheight(self):
        """Return the figure height in inches."""
        return self.get_size_inches()[1]

    def get_size_inches(self):
        """
        Return the current size of the figure in inches.

        Returns
        -------
        ndarray
           The size (width, height) of the figure in inches.

        See Also
        --------
        matplotlib.figure.Figure.set_size_inches
        matplotlib.figure.Figure.get_figwidth
        matplotlib.figure.Figure.get_figheight

        Notes
        -----
        The size in pixels can be obtained by multiplying with `Figure.dpi`.
        """
        return np.array(_figure.get_size_inches(self._native))

    def set_size_inches(self, w, h=None, forward=True):
        """
        Set the figure size in inches.

        Call signatures::

             fig.set_size_inches(w, h)  # OR
             fig.set_size_inches((w, h))

        Parameters
        ----------
        w : (float, float) or float
            Width and height in inches (if height not specified as a separate
            argument) or width.
        h : float
            Height in inches.
        forward : bool, default: True
            If ``True``, the canvas size is automatically updated, e.g.,
            you can resize the figure window from the shell.

        See Also
        --------
        matplotlib.figure.Figure.get_size_inches
        matplotlib.figure.Figure.set_figwidth
        matplotlib.figure.Figure.set_figheight

        Notes
        -----
        To transform from pixels to inches divide by `Figure.dpi`.
        """
        if h is None:
            w, h = w
        if w is None or h is None:
            raise ValueError(
                "Figure.set_size_inches does not accept None; provide both "
                "width and height explicitly")
        size = np.array([w, h])
        if not np.isfinite(size).all() or (size < 0).any():
            raise ValueError(f'figure size must be positive finite not {size}')
        _figure.set_size_inches(self._native, float(w), float(h))
        if forward:
            manager = self.canvas.manager
            if manager is not None:
                manager.resize(*(size * self.dpi).astype(int))

    def _get_dpi(self):
        # As given, as matplotlib keeps it.
        return _values.given(self, "dpi", _figure.get_dpi(self._native))

    def get_dpi(self):
        """Return the resolution in dots per inch as a float."""
        return self.dpi

    def _dots_per_inch(self):
        """The resolution the figure's ``dpi_scale_trans`` scales by, as
        pltrs (Rust) holds it: 1 for a resolution matplotlib keeps without
        scaling by it (see `_set_dpi`)."""
        return _figure.get_dpi(self._native)

    def set_dpi(self, val):
        """
        Set the resolution of the figure in dots-per-inch.

        Parameters
        ----------
        val : float
        """
        self._set_dpi(val)
        self.stale = True

    def _set_dpi(self, dpi, forward=True):
        """Set the resolution, passing *forward* on to `set_size_inches`:
        matplotlib sets the size again, resizing the manager's window
        unless *forward* is false (a canvas's new device pixel ratio)."""
        if dpi == self.dpi:
            return
        try:
            # matplotlib scales its ``dpi_scale_trans`` by the resolution,
            # which fails on what is no number; it keeps the resolution, its
            # transform cleared to the identity, one dot per inch.
            scale = np.eye(3)
            for row in range(2):
                for column in range(3):
                    scale[row, column] *= dpi
            number = float(dpi)
        except (TypeError, ValueError):
            _figure.set_dpi(self._native, 1.0)
            _values.keep(self, "dpi", dpi)
            raise
        _figure.set_dpi(self._native, number)
        _values.remember(self, "dpi", dpi)
        self.set_size_inches(*self.get_size_inches(), forward=forward)

    dpi = property(_get_dpi, _set_dpi, doc="The resolution in dots per inch.")

    def remove(self):
        """A root figure is in nothing it could be removed from: matplotlib's
        `.Artist.remove` refuses it, unless it was given a remove method."""
        if self.__dict__.get("_remove_method") is None:
            raise NotImplementedError('cannot remove artist')
        super().remove()

    def set_canvas(self, canvas):
        """
        Set the canvas that contains the figure

        Parameters
        ----------
        canvas : FigureCanvas
        """
        self.canvas = canvas

    def _set_base_canvas(self):
        """
        Give the figure a `.FigureCanvasBase` again, undoing any high-dpi
        scaling (what a manager does when it is destroyed).
        """
        FigureCanvasBase(self)  # Set self.canvas as a side-effect
        if self.dpi != self._original_dpi:
            self.dpi = self._original_dpi

    def _repr_html_(self):
        """The figure shown inline in a notebook by the WebAgg server, for a
        figure on a WebAgg canvas (None otherwise, IPython then asking
        the other formats)."""
        # Not isinstance, which would import webagg unconditionally.
        if 'WebAgg' in type(self.canvas).__name__:
            from .backends import backend_webagg
            return backend_webagg.ipython_inline_display(self)

    @_api.deprecated(
         "3.6", alternative="figure.get_layout_engine().set()",
         pending=True)
    def set_constrained_layout_pads(self, **kwargs):
        """
        Set padding for ``constrained_layout``.

        Tip: The parameters can be passed from a dictionary by using
        ``fig.set_constrained_layout(**pad_dict)``.

        See :ref:`constrainedlayout_guide`.

        Parameters
        ----------
        w_pad : float, default: :rc:`figure.constrained_layout.w_pad`
            Width padding in inches.  This is the pad around Axes
            and is meant to make sure there is enough room for fonts to
            look good.  Defaults to 3 pts = 0.04167 inches

        h_pad : float, default: :rc:`figure.constrained_layout.h_pad`
            Height padding in inches. Defaults to 3 pts.

        wspace : float, default: :rc:`figure.constrained_layout.wspace`
            Width padding between subplots, expressed as a fraction of the
            subplot width.  The total padding ends up being w_pad + wspace.

        hspace : float, default: :rc:`figure.constrained_layout.hspace`
            Height padding between subplots, expressed as a fraction of the
            subplot width. The total padding ends up being h_pad + hspace.

        """
        if isinstance(self.get_layout_engine(), ConstrainedLayoutEngine):
            self.get_layout_engine().set(**kwargs)

    def _axes_at(self, x, y):
        """The topmost visible Axes containing the display point (*x*, *y*),
        or None (``canvas.inaxes``)."""
        return _handles.wrap(_figure.axes_at(self._native, x, y), Axes, self)

    def show(self, warn=True):
        """
        If using a GUI backend with pyplot, display the figure window.

        If the figure was not created using `~.pyplot.figure`, it will lack
        a `~.backend_bases.FigureManagerBase`, and this method will raise an
        AttributeError.

        .. warning::

            This does not manage a GUI event loop. Consequently, the figure
            may only be shown briefly or not shown at all if you or your
            environment are not managing an event loop.

            Use cases for `.Figure.show` include running this from a GUI
            application (where there is persistently an event loop running) or
            from a shell, like IPython, that install an input hook to allow the
            interactive shell to accept input while the figure is also being
            shown and interactive.  Some, but not all, GUI toolkits will
            register an input hook on import.  See :ref:`cp_integration` for
            more details.

            If you're in a shell without input hook integration or executing a
            python script, you should use `matplotlib.pyplot.show` with
            ``block=True`` instead, which takes care of starting and running
            the event loop for you.

        Parameters
        ----------
        warn : bool, default: True
            If ``True`` and we are not running headless (i.e. on Linux with an
            unset DISPLAY), issue warning when called on a non-GUI backend.

        """
        if self.canvas.manager is None:
            raise AttributeError(
                "Figure.show works only for figures managed by pyplot, "
                "normally created by pyplot.figure()")
        try:
            self.canvas.manager.show()
        except NonGuiException as exc:
            if warn:
                _api.warn_external(str(exc))

    def ginput(self, n=1, timeout=30, show_clicks=True,
               mouse_add=MouseButton.LEFT,
               mouse_pop=MouseButton.RIGHT,
               mouse_stop=MouseButton.MIDDLE):
        """
        Blocking call to interact with a figure.

        Wait until the user clicks *n* times on the figure, and return the
        coordinates of each click in a list.

        There are three possible interactions:

        - Add a point.
        - Remove the most recently added point.
        - Stop the interaction and return the points added so far.

        The actions are assigned to mouse buttons via the arguments
        *mouse_add*, *mouse_pop* and *mouse_stop*.

        Parameters
        ----------
        n : int, default: 1
            Number of mouse clicks to accumulate. If negative, accumulate
            clicks until the input is terminated manually.
        timeout : float, default: 30 seconds
            Number of seconds to wait before timing out. If zero or negative
            will never time out.
        show_clicks : bool, default: True
            If True, show a red cross at the location of each click.
        mouse_add : `.MouseButton` or None, default: `.MouseButton.LEFT`
            Mouse button used to add points.
        mouse_pop : `.MouseButton` or None, default: `.MouseButton.RIGHT`
            Mouse button used to remove the most recently added point.
        mouse_stop : `.MouseButton` or None, default: `.MouseButton.MIDDLE`
            Mouse button used to stop input.

        Returns
        -------
        list of tuples
            A list of the clicked (x, y) coordinates.

        Notes
        -----
        The keyboard can also be used to select points in case your mouse
        does not have one or more of the buttons.  The delete and backspace
        keys act like right-clicking (i.e., remove last point), the enter key
        terminates input and any other key (not already used by the window
        manager) selects a point.
        """
        clicks = []
        marks = []

        def handler(event):
            is_button = event.name == "button_press_event"
            is_key = event.name == "key_press_event"
            # Quit (even if not in infinite mode; this is consistent with
            # MATLAB and sometimes quite useful, but will require the user to
            # test how many points were actually returned before using data).
            if (is_button and event.button == mouse_stop
                    or is_key and event.key in ["escape", "enter"]):
                self.canvas.stop_event_loop()
            # Pop last click.
            elif (is_button and event.button == mouse_pop
                  or is_key and event.key in ["backspace", "delete"]):
                if clicks:
                    clicks.pop()
                    if show_clicks:
                        marks.pop().remove()
                        self.canvas.draw()
            # Add new click.
            elif (is_button and event.button == mouse_add
                  # On macOS/gtk, some keys return None.
                  or is_key and event.key is not None):
                if event.inaxes:
                    clicks.append((event.xdata, event.ydata))
                    _log.info("input %i: %f, %f",
                              len(clicks), event.xdata, event.ydata)
                    if show_clicks:
                        line = Line2D([event.xdata], [event.ydata],
                                      marker="+", color="r")
                        event.inaxes.add_line(line)
                        marks.append(line)
                        self.canvas.draw()
            if len(clicks) == n and n > 0:
                self.canvas.stop_event_loop()

        _blocking_input.blocking_input_loop(
            self, ["button_press_event", "key_press_event"], timeout, handler)

        # Cleanup.
        for mark in marks:
            mark.remove()
        self.canvas.draw()

        return clicks

    def waitforbuttonpress(self, timeout=-1):
        """
        Blocking call to interact with the figure.

        Wait for user input and return True if a key was pressed, False if a
        mouse button was pressed and None if no input was given within
        *timeout* seconds.  Negative values deactivate *timeout*.
        """
        event = None

        def handler(ev):
            nonlocal event
            event = ev
            self.canvas.stop_event_loop()

        _blocking_input.blocking_input_loop(
            self, ["button_press_event", "key_press_event"], timeout, handler)

        return None if event is None else event.name == "key_press_event"

    def savefig(self, fname, *, transparent=None, **kwargs):
        """
        Save the current figure as an image or vector graphic to a file.

        Call signature::

          savefig(fname, *, transparent=None, dpi='figure', format=None,
                  metadata=None, bbox_inches=None, pad_inches=0.1,
                  facecolor='auto', edgecolor='auto', backend=None,
                  **kwargs
                 )

        The available output formats depend on the backend being used.

        Parameters
        ----------
        fname : str or path-like or binary file-like
            A path, or a Python file-like object, or
            possibly some backend-dependent object such as
            `matplotlib.backends.backend_pdf.PdfPages`.

            If *format* is set, it determines the output format, and the file
            is saved as *fname*.  Note that *fname* is used verbatim, and there
            is no attempt to make the extension, if any, of *fname* match
            *format*, and no extension is appended.

            If *format* is not set, then the format is inferred from the
            extension of *fname*, if there is one.  If *format* is not
            set and *fname* has no extension, then the file is saved with
            :rc:`savefig.format` and the appropriate extension is appended to
            *fname*.

        Other Parameters
        ----------------
        transparent : bool, default: :rc:`savefig.transparent`
            If *True*, the Axes patches will all be transparent; the
            Figure patch will also be transparent unless *facecolor*
            and/or *edgecolor* are specified via kwargs.

            If *False* has no effect and the color of the Axes and
            Figure patches are unchanged (unless the Figure patch
            is specified via the *facecolor* and/or *edgecolor* keyword
            arguments in which case those colors are used).

            The transparency of these patches will be restored to their
            original values upon exit of this function.

            This is useful, for example, for displaying
            a plot on top of a colored background on a web page.

        dpi : float or 'figure', default: :rc:`savefig.dpi`
            The resolution in dots per inch.  If 'figure', use the figure's
            dpi value.

        format : str
            The file format, e.g. 'png', 'pdf', 'svg', ... The behavior when
            this is unset is documented under *fname*.

        metadata : dict, optional
            Key/value pairs to store in the image metadata. The supported keys
            and defaults depend on the image format and backend:

            - 'png' with Agg backend: See the parameter ``metadata`` of
              `~.FigureCanvasAgg.print_png`.
            - 'pdf' with pdf backend: See the parameter ``metadata`` of
              `~.backend_pdf.PdfPages`.
            - 'svg' with svg backend: See the parameter ``metadata`` of
              `~.FigureCanvasSVG.print_svg`.
            - 'eps' and 'ps' with PS backend: Only 'Creator' is supported.

            Not supported for 'pgf', 'raw', and 'rgba' as those formats do not support
            embedding metadata.
            Does not currently support 'jpg', 'tiff', or 'webp', but may include
            embedding EXIF metadata in the future.

        bbox_inches : str or `.Bbox`, default: :rc:`savefig.bbox`
            Bounding box in inches: only the given portion of the figure is
            saved.  If 'tight', try to figure out the tight bbox of the figure.

        pad_inches : float or 'layout', default: :rc:`savefig.pad_inches`
            Amount of padding in inches around the figure when bbox_inches is
            'tight'. If 'layout' use the padding from the constrained or
            compressed layout engine; ignored if one of those engines is not in
            use.

        facecolor : :mpltype:`color` or 'auto', default: :rc:`savefig.facecolor`
            The facecolor of the figure.  If 'auto', use the current figure
            facecolor.

        edgecolor : :mpltype:`color` or 'auto', default: :rc:`savefig.edgecolor`
            The edgecolor of the figure.  If 'auto', use the current figure
            edgecolor.

        backend : str, optional
            Use a non-default backend to render the file, e.g. to render a
            png file with the "cairo" backend rather than the default "agg",
            or a pdf file with the "pgf" backend rather than the default
            "pdf".  Note that the default backend is normally sufficient.  See
            :ref:`the-builtin-backends` for a list of valid backends for each
            file format.  Custom backends can be referenced as "module://...".

        orientation : {'landscape', 'portrait'}
            Currently only supported by the postscript backend.

        papertype : str
            One of 'letter', 'legal', 'executive', 'ledger', 'a0' through
            'a10', 'b0' through 'b10'. Only supported for postscript
            output.

        bbox_extra_artists : list of `~matplotlib.artist.Artist`, optional
            A list of extra artists that will be considered when the
            tight bbox is calculated.

        pil_kwargs : dict, optional
            Additional keyword arguments that are passed to
            `PIL.Image.Image.save` when saving the figure.

        """
        kwargs.setdefault('dpi', rcParams['savefig.dpi'])
        if transparent is None:
            transparent = rcParams['savefig.transparent']
        if transparent:
            kwargs.setdefault('facecolor', 'none')
            kwargs.setdefault('edgecolor', 'none')
        canvas = self.canvas
        print_figure = canvas.print_figure  # Fails first on no canvas, as there.
        with cbook._setattr_cm(canvas, _transparent=transparent):
            print_figure(fname, **kwargs)

    def __reduce__(self):
        # Made around the native state of the pickle (see `._pickling`),
        # which the figure's handles reach before it is read back.
        from . import _pickling
        return (_pickling.restore_figure,
                (type(self), _pickling.state_of(self._native)),
                self.__getstate__())

    def __getstate__(self):
        state = super().__getstate__()

        # The canvas cannot currently be pickled, but this has the benefit
        # of meaning that a figure can be detached from one canvas, and
        # re-attached to another.
        state.pop("canvas")

        # The tick helpers, by axis: the tokens of their registry do not
        # pickle.
        from . import _tick_helpers
        state.pop("_tick_helpers", None)
        state["_tick_helper_uses"] = _tick_helpers.pickled(self)
        # What the figure's hooks follow: the hooks do not pickle.
        from . import _pickling
        state["_program_hooks"] = _pickling.program_hooks(self)

        # add version information to the state
        from . import __version__
        state['__mpl_version__'] = __version__

        # check whether the figure manager (if any) is registered with pyplot
        from . import _pylab_helpers
        if self.canvas.manager in _pylab_helpers.Gcf.figs.values():
            state['_restore_to_pylab'] = True
        return state

    def __setstate__(self, state):
        from . import __version__
        from . import _pickling
        version = state.pop('__mpl_version__')
        restore_to_pylab = state.pop('_restore_to_pylab', False)

        if version != __version__:
            _api.warn_external(
                f"This figure was saved with matplotlib version {version} and "
                f"loaded with {__version__} so may not function correctly."
            )
        native_state = self.__dict__.pop('_unpickling_state')
        tick_helper_uses = state.pop('_tick_helper_uses')
        program_hooks = state.pop('_program_hooks')
        self.__dict__.update(state)
        # A shallow copy (`copy.copy`) is a root figure of its own too.
        self._figure = self
        # Once the native figure is read back and the artists the
        # restoration reads have their state.
        artists = [*self.__dict__.get("_artists", {}).values(),
                   *(axis for axis, *_ in tick_helper_uses)]
        native_state.when_restored(lambda: _pickling.when_built(
            artists, lambda: self._restore_unpickled(
                restore_to_pylab, tick_helper_uses, program_hooks)))

    def _restore_unpickled(self, restore_to_pylab, tick_helper_uses,
                           program_hooks):
        """Re-initialise the state not pickled, once the native figure is
        read back (matplotlib's end of ``Figure.__setstate__``)."""
        from . import _pickling, _tick_helpers, gridspec
        _tick_helpers.unpickled(self, tick_helper_uses)
        for artist in list(self._artists.values()):
            gridspec._adopt_unpickled(artist)
            _pickling.resolve_trees(artist, restored=True)
            _sharing.rejoin(artist)
        _sharing.track(self)
        _transform_trees.follow_evaluations(self)
        FigureCanvasBase(self)  # Set self.canvas.
        # discard any changes to the dpi due to pixel ratio changes
        if '_original_dpi' in self.__dict__:
            self.dpi = self._original_dpi
        _pickling.restore_program_hooks(self, program_hooks)
        if self.get_layout_engine() is not None:
            self.get_layout_engine()._attach(self)

        if restore_to_pylab:
            # lazy import to avoid circularity
            from . import pyplot as plt, _pylab_helpers as pylab_helpers
            allnums = plt.get_fignums()
            num = max(allnums) + 1 if allnums else 1
            backend = plt._get_backend_mod()
            mgr = backend.new_figure_manager_given_figure(num, self)
            pylab_helpers.Gcf._set_new_active_manager(mgr)
            plt.draw_if_interactive()

        self.stale = True


# The methods matplotlib's figures inherit.
cbook._inherited_methods(FigureBase, Artist, ["set_transform"])
cbook._inherited_methods(SubFigure, Artist, ["remove"])
cbook._inherited_methods(Figure, Artist, ["remove"])


def figaspect(arg):
    """
    Calculate the width and height for a figure with a specified aspect ratio.

    While the height is taken from :rc:`figure.figsize`, the width is
    adjusted to match the desired aspect ratio. Additionally, it is ensured
    that the width is in the range [4., 16.] and the height is in the range
    [2., 16.]. If necessary, the default height is adjusted to ensure this.

    Parameters
    ----------
    arg : float or 2D array
        If a float, this defines the aspect ratio (i.e. the ratio height /
        width).
        In case of an array the aspect ratio is number of rows / number of
        columns, so that the array could be fitted in the figure undistorted.

    Returns
    -------
    size : (2,) array
        The width and height of the figure in inches.

    Notes
    -----
    If you want to create an Axes within the figure, that still preserves the
    aspect ratio, be sure to create it with equal width and height. See
    examples below.

    Thanks to Fernando Perez for this function.

    Examples
    --------
    Make a figure twice as tall as it is wide::

        w, h = figaspect(2.)
        fig = Figure(figsize=(w, h))
        ax = fig.add_axes((0.1, 0.1, 0.8, 0.8))
        ax.imshow(A, **kwargs)

    Make a figure with the proper aspect for an array::

        A = rand(5, 3)
        w, h = figaspect(A)
        fig = Figure(figsize=(w, h))
        ax = fig.add_axes((0.1, 0.1, 0.8, 0.8))
        ax.imshow(A, **kwargs)
    """
    isarray = hasattr(arg, 'shape') and not np.isscalar(arg)

    # min/max sizes to respect when autoscaling.
    figsize_min = np.array((4.0, 2.0))  # min length for width/height
    figsize_max = np.array((16.0, 16.0))  # max length for width/height

    # Extract the aspect ratio of the array
    if isarray:
        nr, nc = arg.shape[:2]
        arr_ratio = nr / nc
    else:
        arr_ratio = arg

    # Height of user figure defaults
    fig_height = rcParams['figure.figsize'][1]

    # New size for the figure, keeping the aspect ratio of the caller
    newsize = np.array((fig_height / arr_ratio, fig_height))

    # Sanity checks, don't drop either dimension below figsize_min
    newsize /= min(1.0, *(newsize / figsize_min))

    # Avoid humongous windows as well
    newsize /= max(1.0, *(newsize / figsize_max))

    # Finally, if we have a really funky aspect ratio, break it but respect
    # the min/max dimensions (we don't want figures 10 feet tall!)
    newsize = np.clip(newsize, figsize_min, figsize_max)
    return newsize


def _parse_figsize(figsize, dpi):
    """
    Convert a figsize expression to (width, height) in inches.

    Parameters
    ----------
    figsize : (float, float) or (float, float, str)
        This can be

        - a tuple ``(width, height, unit)``, where *unit* is one of "in" (inch),
          "cm" (centimenter), "px" (pixel).
        - a tuple ``(width, height)``, which is interpreted in inches, i.e. as
          ``(width, height, "in")``.

    dpi : float
        The dots-per-inch; used for converting 'px' to 'in'.
    """
    num_parts = len(figsize)
    if num_parts == 2:
        x, y = figsize
    elif num_parts == 3:
        x, y, unit = figsize
        if unit == 'in':
            pass
        elif unit == 'cm':
            if x is not None:
                x /= 2.54
            if y is not None:
                y /= 2.54
        elif unit == 'px':
            if x is not None:
                x /= dpi
            if y is not None:
                y /= dpi
        else:
            raise ValueError(
                f"Invalid unit {unit!r} in 'figsize'; "
                "supported units are 'in', 'cm', 'px'"
            )
    else:
        raise ValueError(
            "Invalid figsize format, expected (x, y) or (x, y, unit) but got "
            f"{figsize!r}"
        )

    if x is None and y is None:
        raise ValueError(
            "figsize=(None, None) is invalid; at least one of width or "
            "height must be provided")

    default_width, default_height = rcParams["figure.figsize"]
    if x is None:
        x = default_width
    if y is None:
        y = default_height
    return x, y


cbook._own_mixin_methods(FigureBase, Artist)
cbook._own_mixin_methods(Figure, FigureBase)


# What matplotlib's module also has from its imports, which programs
# import from it.
import inspect  # noqa: E402, F401
import itertools  # noqa: E402, F401
import threading  # noqa: E402, F401
from contextlib import ExitStack  # noqa: E402, F401
from . import backend_bases, image as mimage, legend as mlegend, projections  # noqa: E402, F401
from .artist import allow_rasterization  # noqa: E402, F401
from .backend_bases import DrawEvent  # noqa: E402, F401
from .patches import Rectangle  # noqa: E402, F401
from .transforms import Affine2D, Bbox, BboxTransformTo, TransformedBbox  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
