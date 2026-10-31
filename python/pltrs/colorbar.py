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
over again whenever the mappable changes. The checks of the arguments and
the helpers resolving *location* and *orientation* are matplotlib's (from
matplotlib 3.11.2, under the Matplotlib License).
"""

import functools

import numpy as np

from . import _api, _handles, artist as martist, colorizer as mcolorizer, ticker
from . import transforms as mtransforms
from ._native import colorbar as _colorbar


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
        if alpha is not None:
            raise NotImplementedError("pltrs: Colorbar(alpha=...)")
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
        self.set_alpha(mappable.get_alpha()
                       if isinstance(mappable, martist.Artist) else None)
        self._scale_norm()
        if isinstance(parents, list):
            parents = [ax._native for ax in parents]
        elif parents is not None:
            parents = parents._native
        self._native = _colorbar.new(
            figure._native, self._shown(),
            None if cax is None else cax._native, parents, spec)
        if locator is not None:
            self.locator = locator
        mappable.colorbar = self
        mappable.colorbar_cid = mappable.callbacks.connect(
            'changed', self.update_normal)
        self._link_axes()
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
        """The lower end of the colorbar's data range."""
        return _colorbar.limits(self._native)[0]

    @property
    def vmax(self):
        """The upper end of the colorbar's data range."""
        return _colorbar.limits(self._native)[1]

    @property
    def _boundaries(self):
        """The edges of the colour blocks, in data units (the given
        *boundaries*, or those the norm and colormap make)."""
        return _colorbar.boundaries(self._native)

    @property
    def _values(self):
        """The value at the middle of each colour block."""
        return _colorbar.values(self._native)

    @property
    def outline(self):
        """The patch outlining the colours and the extensions."""
        from .patches import PathPatch
        return _handles.wrap(_colorbar.outline(self._native), PathPatch,
                             self._figure)

    @property
    def solids(self):
        """The `.QuadMesh` of the colour blocks, None when they are the
        hatched patches of a hatched contour set."""
        from .collections import QuadMesh
        return _handles.wrap(_colorbar.solids(self._native), QuadMesh,
                             self._figure)

    @property
    def _extend_patches(self):
        """The patches filling the extensions, the one at the lower end of
        the long axis first."""
        from .patches import PathPatch
        return _handles.wrap_all(_colorbar.extend_patches(self._native),
                                 PathPatch, self._figure)

    @property
    def dividers(self):
        """The lines between the colours drawn with *drawedges*."""
        return _Dividers(self)

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

    @property
    def minorlocator(self):
        """Minor tick `.Locator` for the colorbar."""
        return self.long_axis.get_minor_locator()

    @minorlocator.setter
    def minorlocator(self, loc):
        self.long_axis.set_minor_locator(loc)

    @property
    def formatter(self):
        """Major tick label `.Formatter` for the colorbar."""
        return self.long_axis.get_major_formatter()

    @formatter.setter
    def formatter(self, fmt):
        self.long_axis.set_major_formatter(fmt)

    @property
    def minorformatter(self):
        """Minor tick `.Formatter` for the colorbar."""
        return self.long_axis.get_minor_formatter()

    @minorformatter.setter
    def minorformatter(self, fmt):
        self.long_axis.set_minor_formatter(fmt)

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
        self.set_alpha(self.mappable.get_alpha()
                       if isinstance(self.mappable, martist.Artist) else None)
        self._scale_norm()
        _colorbar.update_normal(self._native, self._shown())

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
        """
        if labels is not None or minor or kwargs:
            raise NotImplementedError(
                "pltrs: Colorbar.set_ticks(labels=..., minor=..., **kwargs)")
        _colorbar.set_ticks(self._native, [float(t) for t in ticks])

    def get_ticks(self, minor=False):
        """
        Return the ticks as a list of locations.

        Parameters
        ----------
        minor : boolean, default: False
            if True return the minor ticks.
        """
        return _colorbar.get_ticks(self._native, bool(minor))

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


class _Dividers:
    """
    The lines between the colours of a colorbar drawn with *drawedges*
    (matplotlib's ``Colorbar.dividers``): pltrs (Rust) draws them with the
    colorbar, for its current view.
    """

    def __init__(self, colorbar):
        self._colorbar = colorbar

    def get_segments(self):
        """The segments, as ``(2, 2)`` arrays of their ends."""
        return _colorbar.divider_segments(self._colorbar._native)


ColorbarBase = Colorbar  # Backcompat API
