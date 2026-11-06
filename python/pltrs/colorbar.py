"""
Colorbars are a visualization of the mapping from scalar values to colors.
In Matplotlib they are drawn into a dedicated `~.axes.Axes`.

.. note::
   Colorbars are typically created through `.Figure.colorbar` or its pyplot
   wrapper `.pyplot.colorbar`, which internally use `.Colorbar` together with
   `.make_axes_gridspec` (for `.GridSpec`-positioned Axes) or `.make_axes` (for
   non-`.GridSpec`-positioned Axes).

   End-users most likely won't need to directly use this module's API.

pltrs (Rust) lays colorbars out and draws them (``pltrs::colorbar``): a
`Colorbar` hands it the colormap and norm of its mappable, and hands them
over again whenever the mappable changes. The checks of the arguments,
the helpers resolving *location* and *orientation*, and the outline's and
the locator's classes (`_ColorbarSpine`, `_ColorbarAxesLocator`) are
matplotlib's (from matplotlib 3.11.2, under the Matplotlib License).
"""

import functools

import numpy as np

from . import _api, _handles, artist as martist, colorizer as mcolorizer, ticker
from . import patches as mpatches, path as mpath, spines as mspines
from . import transforms as mtransforms
from ._native import colorbar as _colorbar, patches as _patches


def _get_orientation_from_location(location):
    return _api.getitem_checked(
        {None: None, "left": "vertical", "right": "vertical",
         "top": "horizontal", "bottom": "horizontal"}, location=location)


def _get_ticklocation_from_orientation(orientation):
    return _api.getitem_checked(
        {None: "right", "vertical": "right", "horizontal": "bottom"},
        orientation=orientation)


def _remove_cbar_axes(ax, cbar):
    """
    Replacement remove method for a colorbar's axes, so that the colorbar is
    properly removed.

    Note we define this at the module level to preserve pickling. A lambda or
    local def within the Colorbar.__init__ method will not work.
    """
    cbar.remove()


class _ColorbarSpine(mspines.Spine):
    """
    The outline of a colorbar, a spine of its Axes (``ax.spines['outline']``)
    framing the colours and the extensions: pltrs (Rust) draws it as a patch
    through the Axes' ``transAxes``, as matplotlib's ``Patch.draw`` draws
    it, and moves it when the extensions change.
    """

    # A spine of no side: matplotlib's linear spines' machinery (positions,
    # the spine transform) holds Python state only.
    _position = None
    _patch_type = 'line'

    @property
    def spine_type(self):
        """``'colorbar'``."""
        return 'colorbar'

    def get_window_extent(self, renderer=None):
        # This Spine has no Axis associated with it, and doesn't need to
        # adjust its location, so we can directly get the window extent from
        # the super-super-class.
        return mpatches.Patch.get_window_extent(self, renderer=renderer)

    def get_tightbbox(self, renderer=None):
        """Like `get_window_extent`: the outline is not clipped."""
        return self.get_window_extent(renderer)

    get_path = mpatches.Patch.get_path
    get_transform = mpatches.Patch.get_transform
    get_edgecolor = mpatches.Patch.get_edgecolor
    set_linewidth = mpatches.Patch.set_linewidth
    set_linestyle = mpatches.Patch.set_linestyle
    _path = mpatches.PathPatch._path

    def set_xy(self, xy):
        """Set the vertices of the closed outline, in Axes fractions."""
        _patches.set_path_shape(self._native, mpath.Path(xy, closed=True))
        self._xy = xy
        self.stale = True

    def set_color(self, c):
        """Set the edgecolor (the facecolor stays "none")."""
        self.set_edgecolor(c)
        self.stale = True

    def get_spine_transform(self):
        """The outline has no side to place a transform on."""
        raise ValueError(f'unknown spine spine_type: {self.spine_type!r}')

    def set_position(self, position):
        """
        Set the position of the spine, as for a `.Spine`; the outline has no
        transform for it, so this raises `ValueError` once it is set, as
        matplotlib's does.
        """
        mspines._check_position(position)
        self._position = position
        self.get_spine_transform()

    def get_position(self):
        """Return the spine position (setting the default one raises, see
        `set_position`)."""
        if self._position is None:
            self.set_position(('outward', 0.0))
        return self._position

    def clear(self):
        """Clear the current spine."""
        self._position = None

    def set_bounds(self, low=None, high=None):
        """
        Set the spine bounds: refused, as matplotlib refuses them, since
        the outline has no view interval to complete them from.
        """
        raise ValueError(f'spine_type: {self.spine_type} not supported')

    def get_bounds(self):
        """Get the bounds of the spine: none."""
        return None


class _ColorbarAxesLocator:
    """
    Shrink the Axes if there are triangular or rectangular extends.

    pltrs (Rust) places a colorbar's Axes so at every layout; calling the
    locator computes the same position, as matplotlib's does.
    """
    def __init__(self, cbar):
        self._cbar = cbar
        self._orig_locator = cbar.ax.get_axes_locator()

    def __call__(self, ax, renderer):
        if self._orig_locator is not None:
            pos = self._orig_locator(ax, renderer)
        else:
            pos = ax.get_position(original=True)
        if self._cbar.extend == 'neither':
            return pos
        (lower, upper), aspect = _colorbar.locator_geometry(self._cbar._native)
        length = lower + upper + 1
        shrink = 1 / length
        offset = lower / length
        # now shrink and/or offset to take into account the
        # extend tri/rectangles.
        if self._cbar.orientation == 'vertical':
            if aspect:
                self._cbar.ax.set_box_aspect(aspect*shrink)
            pos = pos.shrunk(1, shrink).translated(0, offset * pos.height)
        else:
            if aspect:
                self._cbar.ax.set_box_aspect(1/(aspect * shrink))
            pos = pos.shrunk(shrink, 1).translated(offset * pos.width, 0)
        return pos

    def _place(self, ax):
        """Have pltrs (Rust) place *ax* as the locator does at every
        layout (`.Axes.set_axes_locator`)."""
        from .axes import _program_locators
        place = getattr(self._orig_locator, "_place", None)
        if self._orig_locator is not None and place is None:
            # The original locator is the program's: call this one.
            _program_locators.follow(ax, ax._figure)
            return
        if place is not None:
            place(ax)
        _colorbar.locate_axes(self._cbar._native)


#: The keywords of `.Figure.colorbar` that place a new colorbar Axes.
_LAYOUT_KEYS = ('location', 'orientation', 'fraction', 'shrink', 'aspect',
                'pad', 'anchor', 'panchor')


class Colorbar:
    r"""
    Draw a colorbar in an existing Axes.

    Typically, colorbars are created using `.Figure.colorbar` or
    `.pyplot.colorbar` and associated with `.ColorizingArtist`\s (such as an
    `.AxesImage` generated via `~.axes.Axes.imshow`).

    In order to draw a colorbar not associated with other elements in the
    figure, e.g. when showing a colormap by itself, one can create an empty
    `.ColorizingArtist`, or directly pass *cmap* and *norm* instead of *mappable*
    to `Colorbar`.

    Useful public methods are :meth:`set_label` and :meth:`add_lines`.

    Attributes
    ----------
    ax : `~matplotlib.axes.Axes`
        The `~.axes.Axes` instance in which the colorbar is drawn.
    """

    n_rasterize = 50  # rasterize solids if number of colors >= n_rasterize

    def __init__(
        self, ax, mappable=None, *,
        alpha=None,
        location=None,
        extend=None,
        extendfrac=None,
        extendrect=False,
        ticks=None,
        format=None,
        values=None,
        boundaries=None,
        spacing='uniform',
        drawedges=False,
        label='',
        cmap=None, norm=None,  # redundant with *mappable*
        orientation=None, ticklocation='auto',  # redundant with *location*
    ):
        self._make(ax.figure, mappable, cax=ax, parents=None, layout={},
                   alpha=alpha, location=location, extend=extend,
                   extendfrac=extendfrac, extendrect=extendrect, ticks=ticks,
                   format=format, values=values, boundaries=boundaries,
                   spacing=spacing, drawedges=drawedges, label=label,
                   cmap=cmap, norm=norm, orientation=orientation,
                   ticklocation=ticklocation)

    @classmethod
    def _beside(cls, figure, mappable, parents, *, use_gridspec=True,
                **kwargs):
        """
        A colorbar in a new Axes taking space from *parents*, an Axes or a
        list of them, as `.Figure.colorbar` makes it (matplotlib's
        `make_axes`, or for one Axes on a grid `make_axes_gridspec`, then
        `Colorbar`).
        """
        layout = {key: kwargs.pop(key) for key in _LAYOUT_KEYS
                  if key in kwargs and key not in ('location', 'orientation')}
        layout['use_gridspec'] = use_gridspec
        colorbar = cls.__new__(cls)
        colorbar._make(figure, mappable, cax=None, parents=parents,
                       layout=layout, **kwargs)
        # matplotlib's make_axes labels the Axes it makes.
        colorbar.ax.set_label('<colorbar>')
        return colorbar

    def _make(self, figure, mappable, *, cax, parents, layout, alpha=None,
              location=None, extend=None, extendfrac=None, extendrect=False,
              ticks=None, format=None, values=None, boundaries=None,
              spacing='uniform', drawedges=False, label='', cmap=None,
              norm=None, orientation=None, ticklocation='auto'):
        """Check the arguments as matplotlib does and make the native
        colorbar."""
        if mappable is None:
            mappable = mcolorizer._ScalarMappable(
                colorizer=mcolorizer.Colorizer(norm=norm, cmap=cmap))
        # A drawn mappable's opacity replaces *alpha*, as in matplotlib.
        if isinstance(mappable, martist.Artist):
            alpha = mappable.get_alpha()
        # A locator places the ticks of the long axis once it exists.
        locator = None
        if ticks is not None and not np.iterable(ticks):
            locator, ticks = ticks, None

        location_orientation = _get_orientation_from_location(location)
        _api.check_in_list(
            [None, 'vertical', 'horizontal'], orientation=orientation)
        _api.check_in_list(
            ['auto', 'left', 'right', 'top', 'bottom'],
            ticklocation=ticklocation)
        _api.check_in_list(
            ['uniform', 'proportional'], spacing=spacing)
        if location_orientation is not None and orientation is not None:
            if location_orientation != orientation:
                raise TypeError(
                    "location and orientation are mutually exclusive")
        else:
            orientation = orientation or location_orientation or "vertical"

        self.mappable = mappable
        self._figure = figure.figure  # the root figure, holding the artists
        if isinstance(format, str):
            # Check format between FormatStrFormatter and StrMethodFormatter
            try:
                self._formatter = ticker.FormatStrFormatter(format)
                _ = self._formatter(0)
            except (TypeError, ValueError):
                self._formatter = ticker.StrMethodFormatter(format)
        else:
            self._formatter = format  # Assume it is a Formatter or None
        spec = {
            'use_gridspec': layout.get('use_gridspec', False),
            'location': location,
            'orientation': orientation,
            'ticklocation': ticklocation,
            'fraction': float(layout.get('fraction', 0.15)),
            'shrink': float(layout.get('shrink', 1.0)),
            'aspect': float(layout.get('aspect', 20)),
            'pad': None if layout.get('pad') is None else float(layout['pad']),
            'anchor': layout.get('anchor'),
            'panchor': layout.get('panchor'),
            'extend': extend,
            'extendfrac': extendfrac,
            'extendrect': bool(extendrect),
            'format': format,
            'ticks': None if ticks is None else [float(t) for t in ticks],
            'drawedges': bool(drawedges),
            'label': '' if label is None else str(label),
            'spacing': spacing,
            'boundaries': (None if boundaries is None
                           else [float(b) for b in boundaries]),
            'values': None if values is None else [float(v) for v in values],
        }
        self.spacing = spacing
        self.drawedges = drawedges
        self.extendfrac = extendfrac
        self.extendrect = extendrect
        self.values = values
        self.boundaries = boundaries
        self.ticklocation = (ticklocation if ticklocation != 'auto'
                             else location if location is not None
                             else _get_ticklocation_from_orientation(orientation))
        self.set_alpha(alpha)
        spec['alpha'] = None if self.alpha is None else float(self.alpha)
        self._scale_norm()
        if isinstance(parents, list):
            parents = [ax._native for ax in parents]
        elif parents is not None:
            parents = parents._native
        self._native = _colorbar.new(
            figure._native, self._shown(),
            None if cax is None else cax._native, parents, spec)
        norm_scale = getattr(getattr(self.norm, "_scale", None), "name", None)
        if (not type(self.norm).__module__.startswith(__package__)
                or norm_scale in ("function", "functionlog")):
            # matplotlib's long axis is scaled by the program's norm, or by
            # the program's functions of a `FuncNorm`, which its tick
            # updates call.
            from . import _tick_updates
            _tick_updates.install(figure)
        if locator is not None:
            self.locator = locator
        self._keep_tickers()
        mappable.colorbar = self
        mappable.colorbar_cid = mappable.callbacks.connect(
            'changed', self.update_normal)
        self._link_axes()
        self._take_contour_values()
        self.stale = True

    def _take_contour_values(self):
        """Give the colorbar of a contour set the contour set's
        boundaries and values, as matplotlib's constructor and
        ``update_normal`` take them."""
        from .contour import ContourSet
        if isinstance(self.mappable, ContourSet):
            self.boundaries = self.mappable._levels
            self.values = self.mappable.cvalues
        # Removing the colorbar's Axes removes the colorbar.
        self.ax._remove_method = functools.partial(_remove_cbar_axes,
                                                   cbar=self)

    def _link_axes(self):
        """
        Link the Axes and the colorbar for interactive use: the toolbar
        zooms a colorbar along its long axis only, and pltrs (Rust) pans
        and zooms its norm. pltrs (Rust) stops the Axes of a colorbar of a
        `.BoundaryNorm` or a `.NoNorm` from navigating; it does not know
        when the mappable is a contour set, whose colorbar does not
        navigate either.
        """
        from .contour import ContourSet
        self.ax._colorbar = self
        # matplotlib's colorbar places its Axes with its own locator, around
        # the one the Axes had (pltrs (Rust) composes them natively).
        self.ax._axes_locator = _ColorbarAxesLocator(self)
        if isinstance(self.mappable, ContourSet):
            self.ax.set_navigate(False)

    def set_alpha(self, alpha):
        """
        Set the transparency between 0 (transparent) and 1 (opaque).

        If an array is provided, *alpha* will be set to None to use the
        transparency values associated with the colormap.
        """
        self.alpha = None if isinstance(alpha, np.ndarray) else alpha

    def _scale_norm(self):
        """
        Give the norm the limits the colorbar shows, as matplotlib's
        ``_process_values`` does to the norm the colorbar shares with its
        mappable: unset limits from the mappable's data, else (0, 1),
        widened when they coincide. pltrs (Rust) settles on the same limits
        for the copy of the norm it draws.
        """
        if self.values is not None:
            return
        if self.mappable.get_array() is not None:
            self.mappable.autoscale_None()
        if not self.norm.scaled():
            # If we still aren't scaled after autoscaling, use 0, 1 as default
            self.norm.vmin = 0
            self.norm.vmax = 1
        self.norm.vmin, self.norm.vmax = mtransforms._nonsingular(
            self.norm.vmin, self.norm.vmax, expander=0.1)

    @property
    def ax(self):
        """The `~.axes.Axes` the colorbar is drawn in."""
        from .axes import Axes
        return _handles.wrap(_colorbar.ax(self._native), Axes,
                             self._figure.get_figure(root=True))

    @property
    def cmap(self):
        """The colormap of the mappable."""
        return self.mappable.cmap

    @property
    def norm(self):
        """The norm of the mappable."""
        return self.mappable.norm

    @property
    def orientation(self):
        """``'vertical'`` or ``'horizontal'``."""
        return _colorbar.get_orientation(self._native)

    @property
    def extend(self):
        """The ends extended beyond the range: ``'neither'``, ``'both'``,
        ``'min'`` or ``'max'``."""
        return _colorbar.get_extend(self._native)

    @property
    def vmin(self):
        """The lower end of the colorbar's data range (the first boundary
        inside it, a numpy float as matplotlib reads it off its array)."""
        return np.float64(_colorbar.limits(self._native)[0])

    @property
    def vmax(self):
        """The upper end of the colorbar's data range."""
        return np.float64(_colorbar.limits(self._native)[1])

    @property
    def _boundaries(self):
        """The edges of the colour blocks, in data units (the given
        *boundaries*, or those the norm and colormap make)."""
        return _colorbar.boundaries(self._native)

    @property
    def _values(self):
        """The value at the middle of each colour block."""
        return _colorbar.values(self._native)

    def _forward_boundaries(self, x):
        """Map the boundaries evenly between 0 and 1 (the forward function
        of the long axis' scale of a colorbar with boundaries), and what
        lies well outside them to -1 and 2, keeping ticks out of the
        extensions."""
        b = self._boundaries
        y = np.interp(x, b, np.linspace(0, 1, len(b)))
        eps = (b[-1] - b[0]) * 1e-6
        y[x < b[0]-eps] = -1
        y[x > b[-1]+eps] = 2
        return y

    def _inverse_boundaries(self, x):
        """The inverse of `_forward_boundaries`."""
        b = self._boundaries
        return np.interp(x, np.linspace(0, 1, len(b)), b)

    def _scale_functions(self):
        """The functions of the long axis' scale when pltrs (Rust) scales it
        by function, as matplotlib's ``_reset_locator_formatter_scale``
        chooses them: the boundaries spaced evenly, or the norm and its
        inverse."""
        from .colors import BoundaryNorm
        if self.boundaries is not None or isinstance(self.norm, BoundaryNorm):
            return self._forward_boundaries, self._inverse_boundaries
        return self.norm, self.norm.inverse

    @property
    def outline(self):
        """The spine outlining the colours and the extensions, the
        ``'outline'`` of the Axes' spines."""
        return _handles.wrap(_colorbar.outline(self._native), _ColorbarSpine,
                             self._figure)

    @property
    def solids(self):
        """The `.QuadMesh` of the colour blocks, None when they are the
        hatched patches of a hatched contour set."""
        from .collections import QuadMesh
        return _handles.wrap(_colorbar.solids(self._native), QuadMesh,
                             self._figure)

    @property
    def solids_patches(self):
        """The hatched patches drawn as the colour blocks of a hatched
        contour set, in place of `solids`."""
        from .patches import PathPatch
        return _handles.wrap_all(_colorbar.solids_patches(self._native),
                                 PathPatch, self._figure)

    @property
    def lines(self):
        """The `.LineCollection`\\s `add_lines` drew."""
        from .collections import LineCollection
        return _handles.wrap_all(_colorbar.lines(self._native),
                                 LineCollection, self._figure)

    @property
    def _extend_patches(self):
        """The patches filling the extensions, the one at the lower end of
        the long axis first."""
        from .patches import PathPatch
        return _handles.wrap_all(_colorbar.extend_patches(self._native),
                                 PathPatch, self._figure)

    @property
    def dividers(self):
        """The `.LineCollection` of the lines between the colours: with
        *drawedges*, at the edges of the colours in the view and at the
        extended ends of the view; else none."""
        from .collections import LineCollection
        return _handles.wrap(_colorbar.dividers(self._native),
                             LineCollection, self._figure)

    @property
    def long_axis(self):
        """Axis that has decorations (ticks, etc) on it."""
        if self.orientation == 'vertical':
            return self.ax.yaxis
        return self.ax.xaxis

    @property
    def locator(self):
        """Major tick `.Locator` for the colorbar."""
        return self.long_axis.get_major_locator()

    @locator.setter
    def locator(self, loc):
        self.long_axis.set_major_locator(loc)
        self._locator = loc

    @property
    def minorlocator(self):
        """Minor tick `.Locator` for the colorbar."""
        return self.long_axis.get_minor_locator()

    @minorlocator.setter
    def minorlocator(self, loc):
        self.long_axis.set_minor_locator(loc)
        self._minorlocator = loc

    @property
    def formatter(self):
        """Major tick label `.Formatter` for the colorbar."""
        return self.long_axis.get_major_formatter()

    @formatter.setter
    def formatter(self, fmt):
        self.long_axis.set_major_formatter(fmt)
        self._formatter = fmt

    @property
    def minorformatter(self):
        """Minor tick `.Formatter` for the colorbar."""
        return self.long_axis.get_minor_formatter()

    @minorformatter.setter
    def minorformatter(self, fmt):
        self.long_axis.set_minor_formatter(fmt)

    def _keep_tickers(self):
        """
        Keep the locators and the formatter of the long axis as the
        colorbar's own, which `update_ticks` sets back: those pltrs (Rust)
        chose for the norm, or that were given (matplotlib's
        ``_get_ticker_locator_formatter``).
        """
        self._locator = self.long_axis.get_major_locator()
        self._minorlocator = self.long_axis.get_minor_locator()
        self._formatter = self.long_axis.get_major_formatter()
        self._ticker_norm = self.mappable.norm

    def update_ticks(self):
        """
        Set up the ticks and ticklabels. This should not be needed by users.

        pltrs: sets the colorbar's locators and formatter back on its long
        axis: those given through `locator`, `minorlocator` and
        `formatter`, else those chosen for its norm.
        """
        self.long_axis.set_major_locator(self._locator)
        self.long_axis.set_minor_locator(self._minorlocator)
        self.long_axis.set_major_formatter(self._formatter)

    def drag_pan(self, button, key, x, y):
        """
        Pan the colorbar: the limits of its norm follow the mouse, as the
        limits of an Axes do (``Axes.drag_pan``).
        """
        points = self.ax._get_pan_points(button, key, x, y)
        if points is not None:
            if self.orientation == 'horizontal':
                self.norm.vmin, self.norm.vmax = points[:, 0]
            elif self.orientation == 'vertical':
                self.norm.vmin, self.norm.vmax = points[:, 1]

    def update_normal(self, mappable=None):
        """
        Update solid patches, lines, etc.

        This is meant to be called when the norm of the image or contour plot
        to which this colorbar belongs changes.

        If the norm on the mappable is different than before, this resets the
        locator and formatter for the axis, so if these have been customized,
        they will need to be customized again.  However, if the norm only
        changes values of *vmin*, *vmax* or *cmap* then the old formatter
        and locator will be preserved.
        """
        if mappable is not None:
            self.mappable = mappable
        self.set_alpha(self.mappable.get_alpha())
        self._scale_norm()
        _colorbar.update_normal(self._native, self._shown())
        if self.mappable.norm is not self._ticker_norm:
            self._keep_tickers()
        self.stale = True

    def _shown(self):
        """
        What pltrs (Rust) shows: the colormap and norm of the mappable, and
        the artist pltrs (Rust) draws, if the mappable is one the colorbar
        follows: an image or a collection, whose opacity the colorbar
        takes, or a contour set, whose levels, lines and hatches it shows.
        """
        from .collections import Collection
        from .contour import ContourSet
        from .image import AxesImage
        if isinstance(self.mappable, ContourSet):
            followed = getattr(self.mappable, "_contour", None)
        elif isinstance(self.mappable, (AxesImage, Collection)):
            followed = self.mappable._native
        else:
            followed = None
        return (self.mappable.cmap, self.mappable.norm, followed)

    def set_ticks(self, ticks, *, labels=None, minor=False, **kwargs):
        """
        Set tick locations.

        Parameters
        ----------
        ticks : 1D array-like
            List of tick locations.
        labels : list of str, optional
            List of tick labels. If not set, the labels show the data value.
        minor : bool, default: False
            If ``False``, set the major ticks; if ``True``, the minor ticks.
        **kwargs
            `.Text` properties for the labels. These take effect only if you
            pass *labels*. In other cases, please use `~.Axes.tick_params`.
        """
        if np.iterable(ticks):
            self.long_axis.set_ticks(ticks, labels=labels, minor=minor,
                                     **kwargs)
        else:
            self.long_axis.set_major_locator(ticks)

    def set_ticklabels(self, ticklabels, *, minor=False, **kwargs):
        """
        [*Discouraged*] Set tick labels.

        .. admonition:: Discouraged

            The use of this method is discouraged, because of the dependency
            on tick positions. In most cases, you'll want to use
            ``set_ticks(positions, labels=labels)`` instead.

        Parameters
        ----------
        ticklabels : sequence of str or of `.Text`
            Texts for labeling each tick location in the sequence set by
            `.Colorbar.set_ticks`; the number of labels must match the number
            of locations.
        minor : bool
            If True, set minor ticks instead of major ticks.
        **kwargs
            `.Text` properties for the labels.
        """
        self.long_axis.set_ticklabels(ticklabels, minor=minor, **kwargs)

    def get_ticks(self, minor=False):
        """
        Return the ticks as a list of locations.

        Parameters
        ----------
        minor : boolean, default: False
            if True return the minor ticks.
        """
        if minor:
            return self.long_axis.get_minorticklocs()
        else:
            return self.long_axis.get_majorticklocs()

    def minorticks_on(self):
        """Turn on colorbar minor ticks."""
        _colorbar.set_minorticks(self._native, True)

    def minorticks_off(self):
        """Turn the minor ticks of the colorbar off."""
        _colorbar.set_minorticks(self._native, False)

    def set_label(self, label, *, loc=None, **kwargs):
        """
        Add a label to the long axis of the colorbar.

        Parameters
        ----------
        label : str
            The label text.
        loc : str, optional
            The location of the label.

            - For horizontal orientation one of {'left', 'center', 'right'}
            - For vertical orientation one of {'bottom', 'center', 'top'}

            Defaults to :rc:`xaxis.labellocation` or :rc:`yaxis.labellocation`
            depending on the orientation.
        **kwargs
            Keyword arguments are passed to `~.Axes.set_xlabel` /
            `~.Axes.set_ylabel`.
            Supported keywords are *labelpad* and `.Text` properties.
        """
        if self.orientation == "vertical":
            self.ax.set_ylabel(label, loc=loc, **kwargs)
        else:
            self.ax.set_xlabel(label, loc=loc, **kwargs)

    def add_lines(self, *args, **kwargs):
        """
        Draw lines on the colorbar.

        Parameters
        ----------
        levels : array-like
            The positions of the lines.
        colors : :mpltype:`color` or list of :mpltype:`color`
            Either a single color applying to all lines or one color value for
            each line.
        linewidths : float or array-like
            Either a single linewidth applying to all lines or one linewidth
            for each line.
        erase : bool, default: True
            Whether to remove any previously added lines.

        Notes
        -----
        Alternatively, this method can also be called with the signature
        ``colorbar.add_lines(contour_set, erase=True)``, in which case
        *levels*, *colors*, and *linewidths* are taken from *contour_set*.
        """
        from . import colors as mcolors, contour
        params = _api.select_matching_signature(
            [lambda self, CS, erase=True: locals(),
             lambda self, levels, colors, linewidths, erase=True: locals()],
            self, *args, **kwargs)
        if "CS" in params:
            cs = params["CS"]
            if not isinstance(cs, contour.ContourSet) or cs.filled:
                raise ValueError("If a single artist is passed to add_lines, "
                                 "it must be a ContourSet of lines")
            _colorbar.add_contour_lines(self._native, cs._contour,
                                        bool(params["erase"]))
            return
        _colorbar.add_level_lines(
            self._native,
            [float(level) for level in np.atleast_1d(params["levels"])],
            [tuple(rgba) for rgba in mcolors.to_rgba_array(params["colors"])],
            [float(width) for width in np.atleast_1d(params["linewidths"])],
            bool(params["erase"]))

    def remove(self):
        """
        Remove this colorbar from the figure.

        If the colorbar was created with ``use_gridspec=True`` the previous
        gridspec is restored.
        """
        self.mappable.callbacks.disconnect(self.mappable.colorbar_cid)
        self.mappable.colorbar = None
        self.mappable.colorbar_cid = None
        ax = self.ax
        _colorbar.remove(self._native)
        _handles.forget(ax)


ColorbarBase = Colorbar  # Backcompat API


def _made_axes(make, parents, natives, kwargs):
    """
    The colorbar Axes the native *make* (``make_axes`` or
    ``make_axes_gridspec``) makes beside *parents* (their *natives*) with
    the placement keywords of *kwargs*, and *kwargs* reduced to the
    keywords of the colorbar to make in it, as matplotlib returns them.
    """
    from .axes import Axes
    layout = {key: kwargs.pop(key) for key in ('location', 'orientation',
                                                'fraction', 'shrink', 'aspect')}
    layout.update({key: kwargs.pop(key, None) for key in ('pad', 'anchor', 'panchor')})
    layout['fraction'] = float(layout['fraction'])
    layout['shrink'] = float(layout['shrink'])
    layout['aspect'] = float(layout['aspect'])
    if layout['pad'] is not None:
        layout['pad'] = float(layout['pad'])
    # matplotlib's errors for unknown names (`_normalize_location_orientation`).
    if layout['location'] is None:
        _get_ticklocation_from_orientation(layout['orientation'])
    else:
        _get_orientation_from_location(layout['location'])
    native, kwargs['orientation'], kwargs['ticklocation'] = make(natives, layout)
    figure = parents[0].get_figure(root=True)
    cax = _handles.wrap(native, Axes, figure)
    cax.set_label('<colorbar>')
    return cax, kwargs


def make_axes(parents, location=None, orientation=None, fraction=0.15,
              shrink=1.0, aspect=20, **kwargs):
    """
    Create an `~.axes.Axes` suitable for a colorbar.

    The Axes is placed in the figure of the *parents* Axes, by resizing and
    repositioning *parents*.

    Parameters
    ----------
    parents : `~matplotlib.axes.Axes` or iterable or `numpy.ndarray` of `~.axes.Axes`
        The Axes to use as parents for placing the colorbar.
    location : None or {'left', 'right', 'top', 'bottom'}
        The location, relative to the parent Axes, where the colorbar Axes
        is created.  It also determines the *orientation* of the colorbar.
    orientation : None or {'vertical', 'horizontal'}
        The orientation of the colorbar; it is determined by *location* if
        that is given.
    fraction : float, default: 0.15
        Fraction of original Axes to use for colorbar.
    shrink : float, default: 1.0
        Fraction by which to multiply the size of the colorbar.
    aspect : float, default: 20
        Ratio of long to short dimensions.
    pad : float, default: 0.05 if vertical, 0.15 if horizontal
        Fraction of original Axes between colorbar and new image Axes.
    anchor : (float, float), optional
        The anchor point of the colorbar Axes.
    panchor : (float, float), or *False*, optional
        The anchor point of the colorbar parent Axes. If *False*, the parent
        axes' anchor will be unchanged.

    Returns
    -------
    cax : `~matplotlib.axes.Axes`
        The child Axes.
    kwargs : dict
        The reduced keyword dictionary to be passed when creating the colorbar
        instance.
    """
    # turn parents into a list if it is not already.
    if isinstance(parents, np.ndarray):
        parents = list(parents.flat)
    elif np.iterable(parents):
        parents = list(parents)
    else:
        parents = [parents]
    kwargs.update(location=location, orientation=orientation,
                  fraction=fraction, shrink=shrink, aspect=aspect)
    return _made_axes(_colorbar.make_axes, parents,
                      [ax._native for ax in parents], kwargs)


def make_axes_gridspec(parent, *, location=None, orientation=None,
                       fraction=0.15, shrink=1.0, aspect=20, **kwargs):
    """
    Create an `~.axes.Axes` suitable for a colorbar.

    The Axes is placed in the figure of the *parent* Axes, by resizing and
    repositioning *parent*.

    This function is similar to `.make_axes` and mostly compatible with it.
    Primary differences are

    - `.make_axes_gridspec` requires the *parent* to have a subplotspec.
    - `.make_axes` positions the Axes in figure coordinates;
      `.make_axes_gridspec` positions it using a subplotspec.
    - `.make_axes` updates the position of the parent.  `.make_axes_gridspec`
      replaces the parent gridspec with a new one.

    Parameters
    ----------
    parent : `~matplotlib.axes.Axes`
        The Axes to use as parent for placing the colorbar.
    location, orientation, fraction, shrink, aspect, pad, anchor, panchor
        As for `.make_axes`.

    Returns
    -------
    cax : `~matplotlib.axes.Axes`
        The child Axes.
    kwargs : dict
        The reduced keyword dictionary to be passed when creating the colorbar
        instance.
    """
    kwargs.update(location=location, orientation=orientation,
                  fraction=fraction, shrink=shrink, aspect=aspect)
    return _made_axes(_colorbar.make_axes_gridspec, [parent], parent._native,
                      kwargs)
