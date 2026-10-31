"""
The top level container for all the plot elements (matplotlib's
``figure``).

A `Figure` owns a pltrs (Rust) figure (``self._native``) and the registry of the
objects of its artists (``self._artists``, see `._handles`).
"""

from numbers import Integral
import logging
import math

import numpy as np

from . import _api, _blocking_input, _handles, _live_transforms, cbook, rcParams
from ._native import artist as _artist
from ._native import figure as _figure, gridspec as _gridspec
from ._native import transforms as _transforms
from .artist import Artist
from ._background_patch import FigurePatch
from ._figure_artists import FigureArtists
from .axes import Axes
from .axes._base import _AxesBase
from .backend_bases import FigureCanvasBase, MouseButton, NonGuiException
from .gridspec import GridSpec, SubplotParams, SubplotSpec
from ._figure_images import FigureImagesMixin
from ._figure_layout import FigureBaseLayout, FigureLayout
from ._figure_mosaic import _MosaicMixin
from ._figure_interaction import InteractionMixin
from .layout_engine import (ConstrainedLayoutEngine, LayoutEngine,
                            PlaceHolderLayoutEngine, TightLayoutEngine)
from .lines import Line2D
from .text import Text
from .transforms import Bbox, BboxBase
from ._text_methods import FigureText

_log = logging.getLogger(__name__)


def _native_maker(projection_class):
    """
    How the native axes of a new *projection_class* is made: by the class'
    own ``_make_native``; by its constructor for a subclass of `.Axes`
    with a constructor of its own, which matplotlib calls with the
    keywords of ``add_subplot``; else None, for pltrs (Rust)'s axes of the
    projection.
    """
    make = getattr(projection_class, "_make_native", None)
    if (make is None and projection_class.name == "rectilinear"
            and projection_class.__init__ is not Axes.__init__):
        make = projection_class._constructed_native
    return make


class FigureBase(FigureText, FigureBaseLayout, _MosaicMixin,
                 InteractionMixin, Artist):
    """
    Base class for `.Figure` and `.SubFigure` containing the methods that add
    artists to the figure or subfigure, create Axes, etc.
    """

    @property
    def figure(self):
        """The root `.Figure` (also of a `.SubFigure`)."""
        return self._figure

    @property
    def transSubfigure(self):
        """The transform from fractions of this (sub)figure to display
        coordinates."""
        return _live_transforms.cached(self, "transSubfigure",
                                       _transforms.trans_figure)

    @property
    def transFigure(self):
        """The transform from fractions of the root figure to display
        coordinates."""
        return self.figure.transSubfigure

    @property
    def dpi_scale_trans(self):
        """The transform from inches to display coordinates."""
        return _live_transforms.cached(self.figure, "dpi_scale_trans",
                                       _live_transforms.inches)

    @property
    def bbox(self):
        """The (sub)figure's box in display coordinates."""
        return _live_transforms.LiveBbox.spanned(self.transSubfigure)

    def get_window_extent(self, renderer=None):
        """Return the (sub)figure's box in display coordinates."""
        return self.bbox

    def get_facecolor(self):
        """Get the face color of the Figure rectangle."""
        from .colors import to_rgba
        return to_rgba(_figure.get_facecolor(self._native))

    def set_facecolor(self, color):
        """Set the face color of the Figure rectangle."""
        self.patch.set_facecolor(color)

    @property
    def patch(self):
        """The background of the (sub)figure (a `.FigurePatch`)."""
        patch = self.__dict__.get("_patch")
        if patch is None:
            patch = self.__dict__["_patch"] = FigurePatch(self)
        return patch

    def get_axes(self):
        """List of Axes in the Figure."""
        from ._native import mplot3d as _mplot3d
        from .mpl_toolkits.mplot3d import Axes3D
        return [_handles.wrap(native, Axes3D if is_3d else Axes, self.figure)
                for native, is_3d in _mplot3d.figure_axes(self._native)]

    axes = property(get_axes, doc="List of Axes in the Figure.")

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
        # plt.subplot reuses an axes made with the same class and keywords.
        ax._projection_init = (projection_class, dict(kwargs))
        for name in ("x", "y"):
            other = kwargs.pop(f"share{name}", None)
            if other is not None:
                getattr(ax, f"share{name}")(other)
        ax._internal_update(kwargs)
        return ax

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
        projection, polar, axes_class, sharex, sharey, label
            As `~.Figure.add_subplot`.

        Returns
        -------
        `~.axes.Axes`
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
            return a._add_to_figure(self, False)
        rect, = args
        if isinstance(rect, BboxBase):
            rect = rect.bounds
        if not np.isfinite(rect).all():
            raise ValueError(f'all entries in rect must be finite not {rect}')
        projection_class, pkw = self._process_projection_requirements(**kwargs)
        rect = tuple(map(float, rect))
        make = _native_maker(projection_class)
        if make is not None:
            native, pkw = make(self, pkw, rect=rect)
        elif projection_class.name == "rectilinear":
            native = _figure.add_axes(self._native, rect)
        else:
            native = _figure.add_axes_projection(self._native, rect,
                                                 projection_class.name)
        return self._new_axes(native, projection_class, pkw)

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

        projection : {None, 'aitoff', 'hammer', 'lambert', 'mollweide', \
'polar', 'rectilinear', str}, optional
            The projection type of the subplot (`~.axes.Axes`).
        polar : bool, default: False
            If True, equivalent to projection='polar'.
        axes_class : subclass type of `~.axes.Axes`, optional
            The `.axes.Axes` subclass that is instantiated.
        sharex, sharey : `~matplotlib.axes.Axes`, optional
            Share the x or y `~matplotlib.axis` with sharex and/or sharey.
        label : str
            A label for the returned Axes.

        Returns
        -------
        `~.axes.Axes`
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
            return ax._add_to_figure(self, False)
        if not args:
            args = (1, 1, 1)
        # Normalize correct ijk values to (i, j, k) here so that
        # add_subplot(211) == add_subplot(2, 1, 1).  Invalid values will
        # trigger errors later (via SubplotSpec._from_subplot_args).
        if (len(args) == 1 and isinstance(args[0], Integral)
                and 100 <= args[0] <= 999):
            args = tuple(map(int, str(args[0])))
        projection_class, pkw = self._process_projection_requirements(**kwargs)
        spec = SubplotSpec._from_subplot_args(self, args)._native_spec()
        make = _native_maker(projection_class)
        if make is not None:
            native, pkw = make(self, pkw, spec=spec)
        elif projection_class.name == "rectilinear":
            native = _gridspec.add_subplot(self._native, spec)
        else:
            native = _figure.add_subplot_spec_projection(
                self._native, spec, projection_class.name)
        return self._new_axes(native, projection_class, pkw)

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
            Controls sharing of x-axis (*sharex*) or y-axis (*sharey*).
        squeeze : bool, default: True
            Whether to squeeze the returned array of Axes.
        width_ratios, height_ratios : array-like, optional
            The relative widths of the columns, heights of the rows.
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
        width_ratios, height_ratios : array-like, optional
            The relative widths of the columns, heights of the rows.
        **kwargs
            The keyword arguments of `.SubFigure`.

        Returns
        -------
        `.SubFigure` or array of `.SubFigure`
        """
        gs = GridSpec(nrows=nrows, ncols=ncols, figure=self,
                      wspace=wspace, hspace=hspace,
                      width_ratios=width_ratios,
                      height_ratios=height_ratios,
                      left=0, right=1, bottom=0, top=1)
        natives = _figure.subfigures(
            self._native, (nrows, ncols), (wspace, hspace),
            None if width_ratios is None else [float(r) for r in width_ratios],
            None if height_ratios is None else [float(r) for r in height_ratios])
        sfarr = np.empty((nrows, ncols), dtype=object)
        for i, native in enumerate(natives):
            row, col = divmod(i, ncols)
            sfarr[row, col] = SubFigure._from_native(self, native, gs[row, col],
                                                     **kwargs)
            self.subfigs.append(sfarr[row, col])
        self.stale = True

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
        """
        sf = SubFigure(self, subplotspec, **kwargs)
        self.subfigs.append(sf)
        self.stale = True
        return sf

    @property
    def subplotpars(self):
        """The `.SubplotParams` of the figure."""
        return SubplotParams(*_gridspec.figure_params(self._native))

    def subplots_adjust(self, left=None, bottom=None, right=None, top=None,
                        wspace=None, hspace=None):
        """
        Adjust the subplot layout parameters.

        Unset parameters are left unmodified; initial values are given by
        :rc:`figure.subplot.[name]`.

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
        _figure.subplots_adjust(self._native,
                                (left, bottom, right, top, wspace, hspace))

    def set_layout_engine(self, layout=None, **kwargs):
        """
        Set the layout engine for this figure.

        Parameters
        ----------
        layout : {'constrained', 'compressed', 'tight', 'none', `.LayoutEngine`, None}

            - 'constrained' will use `~.ConstrainedLayoutEngine`
            - 'compressed' will also use `~.ConstrainedLayoutEngine`, but with
              a correction that attempts to make a good layout for fixed-aspect
              ratio Axes.
            - 'tight' uses `~.TightLayoutEngine`
            - 'none' removes layout engine.

            If a `.LayoutEngine` instance, that instance will be used.

            If `None`, the behavior is controlled by :rc:`figure.autolayout`
            (which if `True` behaves as if 'tight' was passed) and
            :rc:`figure.constrained_layout.use` (which if `True` behaves as if
            'constrained' was passed).  If both are `True`,
            :rc:`figure.autolayout` takes priority.

        **kwargs
            The keyword arguments are passed to the layout engine to set things
            like padding and margin sizes.  Only used if *layout* is a string.
        """
        if layout is None:
            if rcParams['figure.autolayout']:
                layout = 'tight'
            elif rcParams['figure.constrained_layout.use']:
                layout = 'constrained'
            else:
                self._layout_engine = None
                _figure.set_layout_engine(self._native, None)
                return
        if layout == 'tight':
            new_layout_engine = TightLayoutEngine(**kwargs)
        elif layout == 'constrained':
            new_layout_engine = ConstrainedLayoutEngine(**kwargs)
        elif layout == 'compressed':
            new_layout_engine = ConstrainedLayoutEngine(compress=True,
                                                        **kwargs)
        elif layout == 'none':
            if self._layout_engine is not None:
                new_layout_engine = PlaceHolderLayoutEngine(
                    self._layout_engine.adjust_compatible,
                    self._layout_engine.colorbar_gridspec
                )
            else:
                new_layout_engine = None
        elif isinstance(layout, LayoutEngine):
            new_layout_engine = layout
        else:
            raise ValueError(f"Invalid value for 'layout': {layout!r}")

        if new_layout_engine is None:
            _figure.set_layout_engine(self._native, None)
        else:
            new_layout_engine._attach(self)
        self._layout_engine = new_layout_engine

    def colorbar(self, mappable, cax=None, ax=None, use_gridspec=True,
                 **kwargs):
        """
        Add a colorbar to a plot: in *cax*, or in a new Axes taking space
        from *ax* (an Axes or a list of them; by default the Axes of
        *mappable*). See `.colorbar.Colorbar` for the keyword arguments.
        """
        from .colorbar import Colorbar
        host = getattr(mappable, "get_figure", None)
        host = None if host is None else host(root=True)
        if host is not None and host is not self.figure:
            _api.warn_external(
                f'Adding colorbar to a different Figure {host!r} than '
                f'{self.figure!r} which fig.colorbar is called on.')
        if ax is None:
            ax = getattr(mappable, "axes", None)
        if cax is not None:
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
        return Colorbar._beside(self, mappable, parents,
                                use_gridspec=use_gridspec, **kwargs)

    def add_artist(self, artist, clip=False):
        """
        Add an `.Artist` to the figure.

        Usually artists are added to `~.axes.Axes` objects using
        `.Axes.add_artist`; this method can be used in the rare cases where
        one needs to add artists directly to the figure instead. Without a
        transform of its own, the artist is placed in figure fractions; with
        *clip*, it is clipped to the figure patch.

        Returns
        -------
        `~matplotlib.artist.Artist`
            The added artist.
        """
        return artist._add_to_figure(self, clip)

    @property
    def artists(self):
        """The artists added to the figure itself (`add_artist`), in a list
        that adds those appended to it."""
        return FigureArtists(self, _handles.wrap_all(
            _figure.artists(self._native), Artist, self.figure))

    def legend(self, *args, **kwargs):
        """
        Place a legend on the figure.

        Call signatures::

            legend()
            legend(handles, labels)
            legend(handles=handles)
            legend(labels)

        The keyword arguments are those of `.Legend`.

        Returns
        -------
        `~matplotlib.legend.Legend`
        """
        from . import legend as mlegend
        handles, labels, kwargs = mlegend._parse_legend_args(
            self.axes,
            *args,
            **kwargs
        )
        # explicitly set the bbox transform if the user hasn't.
        kwargs.setdefault("bbox_transform", self.transSubfigure)
        return mlegend.Legend(self, handles, labels, **kwargs)

    @property
    def legends(self):
        """The legends of the figure, in the order they were made."""
        from . import legend as mlegend
        from ._native import legend as _legend
        return _handles.wrap_all(_legend.figure_legends(self._native),
                                 mlegend.Legend, self.figure)

    def get_layout_engine(self):
        """Return the `.LayoutEngine` of the figure, or None."""
        return self._layout_engine

    def clear(self, keep_observers=False):
        """
        Clear the figure.

        Parameters
        ----------
        keep_observers : bool, default: False
            Set *keep_observers* to True if, for example,
            a gui widget is tracking the Axes in the figure.
        """
        for ax in tuple(self.axes):
            ax.clear()
            self._axes_removed(ax)
        _figure.clear(self._native)
        self.subfigs = []
        if not keep_observers:
            self._forget_axobservers()

    def delaxes(self, ax):
        """
        Remove the `~.axes.Axes` *ax* from the figure; update the current Axes.
        """
        ax.remove()

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

    def clf(self, keep_observers=False):
        """[*Discouraged*] Clear the figure (an alias of `clear`)."""
        return self.clear(keep_observers=keep_observers)

    def draw_without_rendering(self):
        """
        Draw the figure with no output.  Useful to get the final size of
        artists that require a draw before their size is known (e.g. text).
        """
        self._get_renderer()  # The canvas's renderer follows the figure.
        _figure.draw_without_rendering(self._native)

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


class SubFigure(FigureBase):
    """
    Logical figure that can be placed inside a figure.

    Typically instantiated using `.Figure.add_subfigure` or
    `.SubFigure.add_subfigure`, or `.SubFigure.subfigures`.  A subfigure has
    the same methods as a figure except for those particularly tied to the size
    or dpi of the figure, and is confined to a prescribed region of the figure.
    """

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
        **kwargs
            `.SubFigure` properties.
        """
        native = _figure.add_subfigure(parent._native,
                                       subplotspec._native_spec())
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
        if facecolor is not None:
            _figure.set_facecolor(native, facecolor)
        if edgecolor is not None:
            _figure.set_edgecolor(native, edgecolor)
        if linewidth:
            _figure.set_linewidth(native, linewidth)
        if frameon is not None:
            _figure.set_frameon(native, bool(frameon))
        self._internal_update(kwargs)

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

    def get_subplotspec(self):
        """Get the SubplotSpec instance."""
        return self._subplotspec

    @property
    def _artists(self):
        # The root figure keeps the objects of all artists (`._handles`).
        return self._figure._artists

    def sca(self, a):
        """
        Set the current Axes to be *a* and return *a*: the current Axes of
        the root figure, whose stack of Axes subfigures share.
        """
        return self.figure.sca(a)

    def remove(self):
        """Remove the subfigure, and its artists, from its parent."""
        super().remove()
        self._parent.subfigs.remove(self)

    def get_constrained_layout(self):
        """
        Return whether constrained layout is being used.

        See :ref:`constrainedlayout_guide`.
        """
        return self._parent.get_constrained_layout()

    def get_layout_engine(self):
        """Return the layout engine of the root figure."""
        return self._parent.get_layout_engine()

    @property
    def canvas(self):
        """The canvas of the root figure."""
        return self._parent.canvas

    @property
    def dpi(self):
        """The resolution of the root figure."""
        return self._parent.dpi

    def get_dpi(self):
        """Return the resolution of the parent figure in dots-per-inch as a float."""
        return self._parent.dpi

    def set_dpi(self, val):
        """
        Set the resolution of parent figure in dots-per-inch.

        Parameters
        ----------
        val : float
        """
        self._parent.dpi = val


class Figure(FigureLayout, FigureImagesMixin, FigureBase):
    """
    The top level container for all the plot elements.

    See matplotlib's documentation of `matplotlib.figure.Figure`.
    """

    def __str__(self):
        width, height = self.get_size_inches() * self.dpi
        return "Figure(%gx%g)" % (width, height)

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
        if figsize is None:
            figsize = rcParams['figure.figsize']
        if dpi is None:
            dpi = rcParams['figure.dpi']
        width, height = _parse_figsize(figsize, dpi)
        if not all(math.isfinite(v) and v > 0 for v in (width, height)):
            raise ValueError(
                f'figure size must be positive finite not {figsize}')
        self._native = _figure.new(float(width), float(height), float(dpi))
        self._figure = self
        self._artists = {_artist.id(self._native): self}
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
            self.subplots_adjust(**subplotpars.to_dict())
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
            width, height = self.get_size_inches()
            w_pad = w_pad / width
            h_pad = h_pad / height

        return w_pad, h_pad, wspace, hspace

    def set_figwidth(self, val, forward=True):
        """
        Set the width of the figure in inches.

        Parameters
        ----------
        val : float
        forward : bool
            See `set_size_inches`.
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
        """
        self.set_size_inches(self.get_figwidth(), val, forward=forward)

    def get_figwidth(self):
        """Return the figure width in inches."""
        return self.get_size_inches()[0]

    def get_figheight(self):
        """Return the figure height in inches."""
        return self.get_size_inches()[1]

    def get_size_inches(self):
        """Return the current size of the figure in inches."""
        return np.array(_figure.get_size_inches(self._native))

    @property
    def bbox_inches(self):
        """The box of the figure in inches, from ``(0, 0)`` to its size."""
        return Bbox.from_bounds(0, 0, *self.get_size_inches())

    def set_size_inches(self, w, h=None, forward=True):
        """Set the figure size in inches."""
        if h is None:
            w, h = w
        _figure.set_size_inches(self._native, float(w), float(h))

    def get_dpi(self):
        """Return the resolution in dots per inch as a float."""
        return _figure.get_dpi(self._native)

    def set_dpi(self, val):
        """Set the resolution of the figure in dots-per-inch."""
        _figure.set_dpi(self._native, float(val))

    dpi = property(get_dpi, set_dpi, doc="The resolution in dots per inch.")

    def _axes_at(self, x, y):
        """The topmost visible Axes containing the display point (*x*, *y*),
        or None (``canvas.inaxes``)."""
        return _handles.wrap(_figure.axes_at(self._native, x, y), Axes, self)

    def gca(self):
        """Get the current Axes, creating one if necessary."""
        return _handles.wrap(_figure.gca(self._native), Axes, self)

    def sca(self, a):
        """Set the current Axes to be *a* and return *a*."""
        _figure.sca(self._native, a._native)
        self._axes_changed()
        return a

    @property
    def texts(self):
        """The `.Text` instances added to the figure itself."""
        return _handles.wrap_all(_figure.texts(self._native), Text, self)

    def show(self, warn=True):
        """
        If using a GUI backend with pyplot, display the figure window.

        If the figure was not created using `~.pyplot.figure`, it will lack
        a `~.backend_bases.FigureManagerBase`, and this method will raise an
        AttributeError.
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
        coordinates of each click in a list: right click (*mouse_pop*)
        removes the last point, middle click (*mouse_stop*) stops early; the
        keys backspace/delete, escape/enter and any other key do the same.
        *timeout* seconds without a click stop the input (0 or negative:
        never).

        Returns
        -------
        list of tuples
            A list of the clicked (x, y) coordinates.
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

        The canvas prints it (`.FigureCanvasBase.print_figure`), switching
        to the canvas of the format or of the *backend* asked for.
        """
        kwargs.setdefault('dpi', rcParams['savefig.dpi'])
        if transparent is None:
            transparent = rcParams['savefig.transparent']
        if transparent:
            kwargs.setdefault('facecolor', 'none')
            kwargs.setdefault('edgecolor', 'none')
        with cbook._setattr_cm(self.canvas, _transparent=transparent):
            self.canvas.print_figure(fname, **kwargs)

    def _gci(self):
        """
        Get the current colorable artist: the current image of the current
        Axes, else of the last Axes that has one, else None (helper for
        `.pyplot.gci`).
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
        version = state.pop('__mpl_version__')
        restore_to_pylab = state.pop('_restore_to_pylab', False)

        if version != __version__:
            _api.warn_external(
                f"This figure was saved with matplotlib version {version} and "
                f"loaded with {__version__} so may not function correctly."
            )
        native_state = self.__dict__.pop('_unpickling_state')
        tick_helper_uses = state.pop('_tick_helper_uses')
        self.__dict__.update(state)
        native_state.when_restored(
            lambda: self._restore_unpickled(restore_to_pylab, tick_helper_uses))

    def _restore_unpickled(self, restore_to_pylab, tick_helper_uses):
        """Re-initialise the state not pickled, once the native figure is
        read back (matplotlib's end of ``Figure.__setstate__``)."""
        from . import _tick_helpers
        _tick_helpers.unpickled(self, tick_helper_uses)
        # discard any changes to the dpi due to pixel ratio changes
        if '_original_dpi' in self.__dict__:
            self.dpi = self._original_dpi
        FigureCanvasBase(self)  # Set self.canvas.

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
