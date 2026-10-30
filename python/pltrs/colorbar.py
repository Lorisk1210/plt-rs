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

import numpy as np

from . import _api, _handles, colorizer as mcolorizer, ticker
from ._native import colorbar as _colorbar


def _get_orientation_from_location(location):
    return _api.getitem_checked(
        {None: None, "left": "vertical", "right": "vertical",
         "top": "horizontal", "bottom": "horizontal"}, location=location)


def _get_ticklocation_from_orientation(orientation):
    return _api.getitem_checked(
        {None: "right", "vertical": "right", "horizontal": "bottom"},
        orientation=orientation)


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
        A colorbar in a new Axes taking space from *parents*, as
        `.Figure.colorbar` makes it (matplotlib's `make_axes` or
        `make_axes_gridspec`, then `Colorbar`).
        """
        layout = {key: kwargs.pop(key) for key in _LAYOUT_KEYS
                  if key in kwargs and key not in ('location', 'orientation')}
        layout['use_gridspec'] = use_gridspec
        colorbar = cls.__new__(cls)
        colorbar._make(figure, mappable, cax=None, parents=list(parents),
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
        if ticks is not None and not np.iterable(ticks):
            raise NotImplementedError("pltrs: Colorbar(ticks=<Locator>)")

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
        self._figure = figure
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
        self._native = _colorbar.new(
            figure._native, self._shown(),
            None if cax is None else cax._native,
            None if parents is None else [ax._native for ax in parents],
            spec)
        self.spacing = spacing
        self.drawedges = drawedges
        self.extendfrac = extendfrac
        self.extendrect = extendrect
        self.values = values
        self.boundaries = boundaries
        mappable.colorbar = self
        mappable.colorbar_cid = mappable.callbacks.connect(
            'changed', self.update_normal)

    @property
    def ax(self):
        """The `~.axes.Axes` the colorbar is drawn in."""
        from .axes import Axes
        return _handles.wrap(_colorbar.ax(self._native), Axes, self._figure)

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

    def _long_axis(self):
        """Return the long axis."""
        if self.orientation == 'vertical':
            return self.ax.yaxis
        return self.ax.xaxis

    @property
    def locator(self):
        """Major tick `.Locator` for the colorbar."""
        return self._long_axis().get_major_locator()

    @locator.setter
    def locator(self, loc):
        self._long_axis().set_major_locator(loc)

    @property
    def minorlocator(self):
        """Minor tick `.Locator` for the colorbar."""
        return self._long_axis().get_minor_locator()

    @minorlocator.setter
    def minorlocator(self, loc):
        self._long_axis().set_minor_locator(loc)

    @property
    def formatter(self):
        """Major tick label `.Formatter` for the colorbar."""
        return self._long_axis().get_major_formatter()

    @formatter.setter
    def formatter(self, fmt):
        self._long_axis().set_major_formatter(fmt)

    @property
    def minorformatter(self):
        """Minor tick `.Formatter` for the colorbar."""
        return self._long_axis().get_minor_formatter()

    @minorformatter.setter
    def minorformatter(self, fmt):
        self._long_axis().set_minor_formatter(fmt)

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
        _colorbar.update_normal(self._native, self._shown())

    def _shown(self):
        """
        What pltrs (Rust) shows: the colormap and norm of the mappable, and
        the image pltrs (Rust) draws, whose opacity the colorbar takes, if
        the mappable is one.
        """
        from .image import AxesImage
        image = self.mappable if isinstance(self.mappable, AxesImage) else None
        return (self.mappable.cmap, self.mappable.norm,
                None if image is None else image._native)

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
        """
        if kwargs:
            raise NotImplementedError("pltrs: Colorbar.set_label(**kwargs)")
        _colorbar.set_label(self._native, '' if label is None else str(label),
                            loc)

    def remove(self):
        """
        Remove this colorbar from the figure.

        If the colorbar was created with ``use_gridspec=True`` the previous
        gridspec is restored.
        """
        self.mappable.callbacks.disconnect(self.mappable.colorbar_cid)
        self.mappable.colorbar = None
        self.mappable.colorbar_cid = None
        _colorbar.remove(self._native)


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
