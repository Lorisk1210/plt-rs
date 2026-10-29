"""
The top level container for all the plot elements (matplotlib's
``figure``).

A `Figure` owns a pltrs (Rust) figure (``self._native``) and the registry of the
objects of its artists (``self._artists``, see `._handles`).
"""

import math

import numpy as np

from . import _api, _handles, cbook, rcParams
from ._native import artist as _artist, figure as _figure
from .artist import Artist
from .axes import Axes
from .backend_bases import FigureCanvasBase, NonGuiException
from .text import Text


class Figure(Artist):
    """
    The top level container for all the plot elements.

    See matplotlib's documentation of `matplotlib.figure.Figure`.
    """

    def __init__(self, figsize=None, dpi=None, *, facecolor=None,
                 edgecolor=None, linewidth=0.0, frameon=None, **kwargs):
        if figsize is None:
            figsize = rcParams['figure.figsize']
        if dpi is None:
            dpi = rcParams['figure.dpi']
        width, height = figsize
        if not all(math.isfinite(v) and v > 0 for v in (width, height)):
            raise ValueError(
                f'figure size must be positive finite not {figsize}')
        self._native = _figure.new(float(width), float(height), float(dpi))
        self._figure = self
        self._artists = {_artist.id(self._native): self}
        self._number = None
        FigureCanvasBase(self)
        if facecolor is not None:
            self.set_facecolor(facecolor)
        if edgecolor is not None:
            _figure.set_edgecolor(self._native, edgecolor)
        if linewidth:
            _figure.set_linewidth(self._native, linewidth)
        if frameon is not None:
            _figure.set_frameon(self._native, bool(frameon))
        self._internal_update(kwargs)

    @property
    def number(self):
        """The figure's number in pyplot, if pyplot manages it."""
        return self._number

    @number.setter
    def number(self, num):
        self._number = num

    def get_size_inches(self):
        """Return the current size of the figure in inches."""
        return np.array(_figure.get_size_inches(self._native))

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

    def get_facecolor(self):
        """Get the face color of the Figure rectangle."""
        return _figure.get_facecolor(self._native)

    def set_facecolor(self, color):
        """Set the face color of the Figure rectangle."""
        _figure.set_facecolor(self._native, color)

    def get_axes(self):
        """List of Axes in the Figure."""
        return _handles.wrap_all(_figure.axes(self._native), Axes, self)

    axes = property(get_axes, doc="List of Axes in the Figure.")

    def _axes_at(self, x, y):
        """The topmost visible Axes containing the display point (*x*, *y*),
        or None (``canvas.inaxes``)."""
        return _handles.wrap(_figure.axes_at(self._native, x, y), Axes, self)

    def add_subplot(self, *args, **kwargs):
        """Add an `~.axes.Axes` to the figure as part of a subplot arrangement."""
        if not args:
            args = (1, 1, 1)
        if len(args) == 1 and isinstance(args[0], int):
            if not 100 <= args[0] <= 999:
                raise ValueError(
                    "Integer subplot specification must be a three-digit "
                    f"number, not {args[0]}")
            args = tuple(map(int, str(args[0])))
        if len(args) != 3:
            raise TypeError(f"add_subplot() takes 1 or 3 positional arguments "
                            f"but {len(args)} were given")
        nrows, ncols, index = args
        projection = kwargs.pop('projection', None)
        if kwargs.pop('polar', False):
            if projection is not None and projection != 'polar':
                raise ValueError(
                    f"polar={True}, yet projection={projection!r}. "
                    "Only one of these arguments should be supplied."
                )
            projection = 'polar'
        if projection is None or projection == 'rectilinear':
            ax = _handles.wrap(
                _figure.add_subplot(self._native, nrows, ncols, index), Axes, self)
        else:
            from .projections import get_projection_class
            cls = get_projection_class(projection)
            ax = _handles.wrap(_figure.add_subplot_projection(
                self._native, (nrows, ncols, index), cls.name), cls, self)
        ax._internal_update(kwargs)
        return ax

    def subplots(self, nrows=1, ncols=1, *, sharex=False, sharey=False,
                 squeeze=True, width_ratios=None, height_ratios=None,
                 subplot_kw=None, gridspec_kw=None):
        """Add a set of subplots to this figure."""
        unsupported = {"sharex": sharex, "sharey": sharey,
                       "width_ratios": width_ratios,
                       "height_ratios": height_ratios,
                       "gridspec_kw": gridspec_kw}
        for name, value in unsupported.items():
            if value:
                raise NotImplementedError(f"pltrs: subplots({name}=...)")
        natives = _figure.subplots(self._native, nrows, ncols)
        axs = np.empty((nrows, ncols), dtype=object)
        for i, ax in enumerate(_handles.wrap_all(natives, Axes, self)):
            ax._internal_update(subplot_kw or {})
            axs.flat[i] = ax
        if squeeze:
            return axs.item() if axs.size == 1 else axs.squeeze()
        return axs

    def gca(self):
        """Get the current Axes, creating one if necessary."""
        return _handles.wrap(_figure.gca(self._native), Axes, self)

    def sca(self, a):
        """Set the current Axes to be *a* and return *a*."""
        _figure.sca(self._native, a._native)
        return a

    def suptitle(self, t, **kwargs):
        """Add a centered super title to the figure."""
        text = _handles.wrap(_figure.suptitle(self._native, str(t)), Text, self)
        text._internal_update(kwargs)
        return text

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

    def colorbar(self, mappable, cax=None, ax=None, use_gridspec=True,
                 **kwargs):
        """
        Add a colorbar to a plot: in *cax*, or in a new Axes taking space
        from *ax* (an Axes or a list of them; by default the Axes of
        *mappable*). See `.colorbar.Colorbar` for the keyword arguments.
        """
        from .colorbar import Colorbar
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
            parents = [ax]
        return Colorbar._beside(self, mappable, parents,
                                use_gridspec=use_gridspec, **kwargs)

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
