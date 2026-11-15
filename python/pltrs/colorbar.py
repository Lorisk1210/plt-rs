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
import inspect
import logging

import numpy as np

from . import _docstring
from . import _api, _handles, _lazy_imports, _named, artist as martist
from . import _values as _kept_values
from . import colorizer as mcolorizer
from . import ticker
from . import patches as mpatches, path as mpath, spines as mspines
from . import transforms as mtransforms
from ._native import colorbar as _colorbar, patches as _patches

_colorbar_checks = _lazy_imports.module(globals(), "._colorbar_checks")

_log = logging.getLogger(__name__)


def _get_orientation_from_location(location):
    return _api.getitem_checked(
        {None: None, "left": "vertical", "right": "vertical",
         "top": "horizontal", "bottom": "horizontal"}, location=location)


def _get_ticklocation_from_orientation(orientation):
    return _api.getitem_checked(
        {None: "right", "vertical": "right", "horizontal": "bottom"},
        orientation=orientation)


def _check_bracketable(values):
    """
    Bracket *values* by half the step between them as matplotlib's
    ``Colorbar._process_values`` does, failing with numpy's errors for
    values it cannot bracket: fewer than two, or not 1-D (pltrs (Rust)
    brackets the others when it lays the colorbar out).
    """
    array = np.array(values)
    b = np.zeros(len(values) + 1)
    b[1:-1] = 0.5 * (array[:-1] + array[1:])
    b[0] = 2.0 * b[1] - b[2]
    b[-1] = 2.0 * b[-2] - b[-3]


class _UnfinishedColorbarLocator:
    """
    The locator a colorbar whose making failed leaves on its Axes:
    matplotlib's constructor installs its `_ColorbarAxesLocator` before it
    computes with the rest of its arguments. Extended, it places the Axes
    as the locator it replaced does, then fails reading the boundaries the
    colorbar never computed.
    """

    def __init__(self, ax):
        self._orig_locator = ax.get_axes_locator()

    def __call__(self, ax, renderer):
        if self._orig_locator is not None:
            self._orig_locator(ax, renderer)
        raise AttributeError("'Colorbar' object has no attribute '_boundaries'")


def _leave_unfinished_locator(cax, extend, mappable, cmap, norm):
    """Place *cax* as the locator of a colorbar that failed to be made
    leaves it: extended (*extend*, else as the colormap or norm of
    *mappable*, or *cmap* and *norm* without one, extend), the colorbar
    fails placing it; else it keeps its place."""
    if extend is None:
        if mappable is not None:
            cmap, norm = mappable.cmap, mappable.norm
        extend = getattr(cmap, 'colorbar_extend', False)
        if extend is False:
            extend = getattr(norm, 'extend', 'neither')
    if extend != 'neither':
        cax.set_axes_locator(_UnfinishedColorbarLocator(cax))


def _redraw_while_limiting(norm, cax, orientation):
    """
    The limits matplotlib's colorbar gives *cax*, the Axes it is made in,
    while its constructor gives the unscaled *norm* the limits (0, 1): it is
    connected to its mappable already, so each limit that changes redraws
    it (``update_normal``, which limits the norm again, nested), and each
    redraw sets the short axis to (0, 1) and the long one to the norm's
    limits, through the scales *cax* has then (a log-scaled one warns of
    a limit of zero).
    """
    vertical = orientation == 'vertical'
    set_long, set_short = ((cax.set_ylim, cax.set_xlim) if vertical
                           else (cax.set_xlim, cax.set_ylim))
    inverted = (cax.yaxis if vertical else cax.xaxis).get_inverted()
    limits = {"vmin": norm.vmin, "vmax": norm.vmax}

    def assign(name, value):
        if limits[name] != value:
            limits[name] = value
            redraw()

    def redraw():
        if limits["vmin"] is None or limits["vmax"] is None:
            assign("vmin", 0)
            assign("vmax", 1)
        lower, upper = limits["vmin"], limits["vmax"]
        if inverted:
            lower, upper = upper, lower
        set_short(0, 1)
        set_long(lower, upper)

    assign("vmin", 0)
    assign("vmax", 1)


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

    def __init__(self, axes):
        # matplotlib's: an outline of no path yet, in the Axes' fractions,
        # which a colorbar gives the path of its colours and extensions; a
        # loose patch styled as matplotlib's spines are (pltrs (Rust) draws
        # a colorbar's own).
        from . import rcParams
        self._ax = axes
        mpatches.Patch.__init__(self)
        self._spine_axes = axes
        self.set_figure(axes.get_figure(root=False))
        self.set_facecolor('none')
        self.set_edgecolor(rcParams['axes.edgecolor'])
        self.set_linewidth(rcParams['axes.linewidth'])
        self.set_capstyle('projecting')
        self.set_zorder(2.5)
        mpatches.Patch.set_transform(self, axes.transAxes)

    @_kept_values.assignable
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

    get_path = mpatches.Patch._native_path
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


_docstring.interpd.register(
    _make_axes_kw_doc="""
location : None or {'left', 'right', 'top', 'bottom'}
    The location, relative to the parent Axes, where the colorbar Axes
    is created.  It also determines the *orientation* of the colorbar
    (colorbars on the left and right are vertical, colorbars at the top
    and bottom are horizontal).  If None, the location will come from the
    *orientation* if it is set (vertical colorbars on the right, horizontal
    ones at the bottom), or default to 'right' if *orientation* is unset.

orientation : None or {'vertical', 'horizontal'}
    The orientation of the colorbar.  It is preferable to set the *location*
    of the colorbar, as that also determines the *orientation*; passing
    incompatible values for *location* and *orientation* raises an exception.

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
    Defaults to (0.0, 0.5) if vertical; (0.5, 1.0) if horizontal.

panchor : (float, float), or *False*, optional
    The anchor point of the colorbar parent Axes. If *False*, the parent
    axes' anchor will be unchanged.
    Defaults to (1.0, 0.5) if vertical; (0.5, 0.0) if horizontal.""",
    _colormap_kw_doc="""
extend : {'neither', 'both', 'min', 'max'}
    Make pointed end(s) for out-of-range values (unless 'neither').  These are
    set for a given colormap using the colormap set_under and set_over methods.

extendfrac : {*None*, 'auto', length, lengths}
    If set to *None*, both the minimum and maximum triangular colorbar
    extensions will have a length of 5% of the interior colorbar length (this
    is the default setting).

    If set to 'auto', makes the triangular colorbar extensions the same lengths
    as the interior boxes (when *spacing* is set to 'uniform') or the same
    lengths as the respective adjacent interior boxes (when *spacing* is set to
    'proportional').

    If a scalar, indicates the length of both the minimum and maximum
    triangular colorbar extensions as a fraction of the interior colorbar
    length.  A two-element sequence of fractions may also be given, indicating
    the lengths of the minimum and maximum colorbar extensions respectively as
    a fraction of the interior colorbar length.

extendrect : bool
    If *False* the minimum and maximum colorbar extensions will be triangular
    (the default).  If *True* the extensions will be rectangular.

ticks : None or list of ticks or Locator
    If None, ticks are determined automatically from the input.

format : None or str or Formatter
    If None, `~.ticker.ScalarFormatter` is used.
    Format strings, e.g., ``"%4.2e"`` or ``"{x:.2e}"``, are supported.
    An alternative `~.ticker.Formatter` may be given instead.

drawedges : bool
    Whether to draw lines at color boundaries.

label : str
    The label on the colorbar's long axis.

boundaries, values : None or a sequence
    If unset, the colormap will be displayed on a 0-1 scale.
    If sequences, *values* must have a length 1 less than *boundaries*.  For
    each region delimited by adjacent entries in *boundaries*, the color mapped
    to the corresponding value in *values* will be used.  The size of each
    region is determined by the *spacing* parameter.
    Normally only useful for indexed colors (i.e. ``norm=NoNorm()``) or other
    unusual circumstances.

spacing : {'uniform', 'proportional'}
    For discrete colorbars (`.BoundaryNorm` or contours), 'uniform' gives each
    color the same space; 'proportional' makes the space proportional to the
    data interval.""")


@_docstring.interpd
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
    lines : list
        A list of `.LineCollection` (empty if no lines were drawn).
    dividers : `.LineCollection`
        A LineCollection (empty if *drawedges* is ``False``).
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
        """
        Parameters
        ----------
        ax : `~matplotlib.axes.Axes`
            The `~.axes.Axes` instance in which the colorbar is drawn.

        mappable : `.ColorizingArtist`
            The mappable whose colormap and norm will be used.

            To show the colors versus index instead of on a 0-1 scale, set the
            mappable's norm to ``colors.NoNorm()``.

        alpha : float
            The colorbar transparency between 0 (transparent) and 1 (opaque).

        location : None or {'left', 'right', 'top', 'bottom'}
            Set the colorbar's *orientation* and *ticklocation*. Colorbars on
            the left and right are vertical, colorbars at the top and bottom
            are horizontal. The *ticklocation* is the same as *location*, so if
            *location* is 'top', the ticks are on the top. *orientation* and/or
            *ticklocation* can be provided as well and overrides the value set by
            *location*, but there will be an error for incompatible combinations.

            .. versionadded:: 3.7

        %(_colormap_kw_doc)s

        Other Parameters
        ----------------
        cmap : `~matplotlib.colors.Colormap`, default: :rc:`image.cmap`
            The colormap to use.  This parameter is ignored, unless *mappable* is
            None.

        norm : `~matplotlib.colors.Normalize`
            The normalization to use.  This parameter is ignored, unless *mappable*
            is None.

        orientation : None or {'vertical', 'horizontal'}
            If None, use the value determined by *location*. If both
            *orientation* and *location* are None then defaults to 'vertical'.

        ticklocation : {'auto', 'left', 'right', 'top', 'bottom'}
            The location of the colorbar ticks. The *ticklocation* must match
            *orientation*. For example, a horizontal colorbar can only have ticks
            at the top or the bottom. If 'auto', the ticks will be the same as
            *location*, so a colorbar to the left will have ticks to the left. If
            *location* is None, the ticks will be at the bottom for a horizontal
            colorbar and at the right for a vertical.
        """
        try:
            self._make(None, mappable, cax=ax, parents=None, layout={},
                       alpha=alpha, location=location, extend=extend,
                       extendfrac=extendfrac, extendrect=extendrect, ticks=ticks,
                       format=format, values=values, boundaries=boundaries,
                       spacing=spacing, drawedges=drawedges, label=label,
                       cmap=cmap, norm=norm, orientation=orientation,
                       ticklocation=ticklocation)
        except Exception:
            if self.__dict__.pop("_placing", False) and isinstance(ax, martist.Artist):
                _leave_unfinished_locator(ax, extend, mappable, cmap, norm)
            raise

    @classmethod
    def _beside(cls, mappable, parents, *, use_gridspec=True, made=None,
                **kwargs):
        """
        A colorbar in a new Axes taking space from *parents*, an Axes or a
        list of them, in the (sub)figure of the first, as `.Figure.colorbar`
        makes it (matplotlib's `make_axes`, or for one Axes on a grid
        `make_axes_gridspec`, then `Colorbar`); *made*, if given, is called
        once the Axes exists.
        """
        layout = {key: kwargs.pop(key) for key in _LAYOUT_KEYS
                  if key in kwargs and key not in ('location', 'orientation')}
        first = parents[0] if isinstance(parents, list) else parents
        figure = first.get_figure(root=False)
        if set(kwargs) - set(inspect.signature(cls).parameters):
            # matplotlib makes the colorbar's Axes first, then refuses the
            # keywords making the colorbar in it.
            from .axes import Axes
            engine = figure.get_layout_engine()
            on_grid = (use_gridspec and (engine is None or engine.colorbar_gridspec)
                       and isinstance(parents, Axes) and parents.get_subplotspec())
            make = make_axes_gridspec if on_grid else make_axes
            current = figure.gca()
            cax, kwargs = make(parents, **layout, **kwargs)
            # matplotlib's `Figure.colorbar` makes the Axes current again
            # and turns the new Axes' grid off before the colorbar.
            figure.sca(current)
            cax.grid(visible=False, which='both', axis='both')
            if made is not None:
                made()
            return cls(cax, mappable, **kwargs)
        layout['use_gridspec'] = use_gridspec
        colorbar = cls.__new__(cls)
        colorbar._make(figure, mappable, cax=None, parents=parents,
                       layout=layout, made=made, **kwargs)
        # matplotlib's make_axes labels the Axes it makes.
        colorbar.ax.set_label('<colorbar>')
        return colorbar

    # Keywords matplotlib's constructor does not take are refused in its
    # name.
    @_named.named("Colorbar.__init__")
    def _make(self, figure, mappable, *, cax, parents, layout, made=None,
              alpha=None,
              location=None, extend=None, extendfrac=None, extendrect=False,
              ticks=None, format=None, values=None, boundaries=None,
              spacing='uniform', drawedges=False, label='', cmap=None,
              norm=None, orientation=None, ticklocation='auto'):
        """Check the arguments as matplotlib does and make the native
        colorbar in *figure* (that of *cax* for None), calling *made* (if
        given) once it exists."""
        if mappable is None:
            mappable = mcolorizer._ScalarMappable(
                colorizer=mcolorizer.Colorizer(norm=norm, cmap=cmap))
        mappable.cmap, mappable.norm  # noqa: B018  (matplotlib's first reads)
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
        # matplotlib's constructor places *cax* with its locator here,
        # before it computes with the rest of its arguments.
        self._placing = True
        if figure is None:
            if not isinstance(cax, martist.Artist):
                # matplotlib's colorbar places *cax* with a locator around
                # the one it had.
                cax._axes_locator  # noqa: B018
            figure = cax.figure
        _colorbar_checks.check_colorbar(extend, ticks, locator, format, extendfrac,
                                        orientation, ticklocation, location)

        if values is not None and boundaries is None:
            _check_bracketable(values)
        elif values is None and boundaries is not None:
            _colorbar_checks.check_boundaries(boundaries, extend, mappable.norm)
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
        self._scale_norm(drawn_in=None if cax is None else (cax, orientation))
        if isinstance(parents, list):
            parents = [ax._native for ax in parents]
        elif parents is not None:
            parents = parents._native
        # pltrs (Rust) scales the long axis as it makes the colorbar, and an
        # Axes whose transform trees are in use then asks for that scale
        # (`_default_axis_scale`): the Axes given knows its colorbar first.
        if cax is not None:
            unlinked = cax.__dict__.get("_colorbar", cax)
            cax._colorbar = self
        try:
            self._native = _colorbar.new(
                figure._native, self._shown(),
                None if cax is None else cax._native, parents, spec)
        except BaseException:
            if cax is not None:
                if unlinked is cax:
                    del cax._colorbar
                else:
                    cax._colorbar = unlinked
            raise
        if made is not None:
            made()
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
        elif ticks is not None:
            # matplotlib's locator of the ticks given, which keeps them as
            # given (integers stay).
            self.locator = ticker.FixedLocator(ticks, nbins=len(ticks))
        if isinstance(format, ticker.Formatter):
            # matplotlib sets a formatter given on the long axis, which
            # formats its ticks through it (``format_ticks``: a
            # `.ScalarFormatter` labels nothing called tick by tick).
            self.formatter = format
        self._keep_tickers()
        mappable.colorbar = self
        mappable.colorbar_cid = mappable.callbacks.connect(
            'changed', self.update_normal)
        self._link_axes()
        self._take_contour_values()
        self._adopt_solids()
        self.stale = True
        del self._placing

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
        # matplotlib's colorbar lays its extends out again when the limits
        # of its Axes change (an inverted axis flips them).
        self._extend_cid1 = self.ax.callbacks.connect(
            "xlim_changed", self._do_extends)
        self._extend_cid2 = self.ax.callbacks.connect(
            "ylim_changed", self._do_extends)

    def _do_extends(self, ax=None):
        """
        Add the extend tri/rectangles on the outside of the Axes.

        ax is unused, but required due to the callbacks on xlim/ylim changed
        """
        # pltrs (Rust) lays the extends out as it draws the colorbar.
        self.stale = True

    def set_alpha(self, alpha):
        """
        Set the transparency between 0 (transparent) and 1 (opaque).

        If an array is provided, *alpha* will be set to None to use the
        transparency values associated with the colormap.
        """
        self.alpha = None if isinstance(alpha, np.ndarray) else alpha

    def _scale_norm(self, drawn_in=None):
        """
        Give the norm the limits the colorbar shows, as matplotlib's
        ``_process_values`` does to the norm the colorbar shares with its
        mappable: unset limits from the mappable's data, else (0, 1),
        widened when they coincide. pltrs (Rust) settles on the same limits
        for the copy of the norm it draws; a norm it maps through in
        Python, it autoscales and limits itself, as often as matplotlib's
        colorbar does. *drawn_in*, ``(cax, orientation)``, is the Axes a
        colorbar being made is given, which matplotlib's colorbar redraws
        into as it limits the norm (see `_redraw_while_limiting`).
        """
        if self.values is not None or _colorbar.maps_through_python(self.norm):
            return
        if self.mappable.get_array() is not None:
            self.mappable.autoscale_None()
        if not self.norm.scaled():
            if drawn_in is not None and getattr(self.norm, "_scale", None) is None:
                # (A norm of a scale, a logarithm's say, refuses to invert
                # the limits it is given first, which ends the redraw.)
                _redraw_while_limiting(self.norm, *drawn_in)
            # If we still aren't scaled after autoscaling, use 0, 1 as default
            self.norm.vmin = 0
            self.norm.vmax = 1
        self.norm.vmin, self.norm.vmax = mtransforms._nonsingular(
            self.norm.vmin, self.norm.vmax, expander=0.1)

    @_kept_values.assignable
    def ax(self):
        """The `~.axes.Axes` the colorbar is drawn in."""
        removed = self.__dict__.get("_removed_ax")
        if removed is not None:
            # Still the Axes it was, once taken out of the figure.
            return removed
        from .axes import Axes
        return _handles.wrap(_colorbar.ax(self._native), Axes,
                             self._figure.get_figure(root=True))

    @_kept_values.assignable
    def cmap(self):
        """The colormap of the mappable."""
        return self.mappable.cmap

    @_kept_values.assignable
    def norm(self):
        """The norm of the mappable."""
        return self.mappable.norm

    @_kept_values.assignable
    def orientation(self):
        """``'vertical'`` or ``'horizontal'``."""
        return _colorbar.get_orientation(self._native)

    @property
    def extend(self):
        """The ends extended beyond the range: ``'neither'``, ``'both'``,
        ``'min'`` or ``'max'``."""
        return _kept_values.given(self, "extend", _colorbar.get_extend(self._native))

    @extend.setter
    def extend(self, extend):
        # matplotlib's Axes locator reads it at every draw, making room for
        # the extensions its next update draws; another value extends
        # neither end there (as 'neither', but for the box aspect its
        # locator gives the Axes anew), and reads back as given.
        named = isinstance(extend, str) and extend in ('neither', 'both', 'min', 'max')
        _colorbar.set_extend(self._native, extend if named else 'neither')
        (_kept_values.remember if named else _kept_values.keep)(self, "extend", extend)
        self.stale = True

    @_kept_values.assignable
    def vmin(self):
        """The lower end of the colorbar's data range (the first boundary
        inside it, a numpy float as matplotlib reads it off its array)."""
        return np.float64(_colorbar.limits(self._native)[0])

    @_kept_values.assignable
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

    @_kept_values.assignable
    def outline(self):
        """The spine outlining the colours and the extensions, the
        ``'outline'`` of the Axes' spines."""
        return _handles.wrap(_colorbar.outline(self._native), _ColorbarSpine,
                             self._figure)

    @_kept_values.assignable
    def solids(self):
        """The `.QuadMesh` of the colour blocks, None when they are the
        hatched patches of a hatched contour set."""
        from .collections import QuadMesh
        return _handles.wrap(_colorbar.solids(self._native), QuadMesh,
                             self._figure)

    def _adopt_solids(self):
        """Have the colour blocks (`solids`) map through the colorbar's very
        colormap and norm, as matplotlib's ``pcolormesh(..., cmap=self.cmap,
        norm=self.norm)`` makes them whenever it draws them anew (on
        `__init__` and `update_normal`), and the patches of the extensions
        read back as matplotlib's."""
        solids = self.solids
        if solids is not None:
            solids._attach_native_mapping(cmap=self.cmap, norm=self.norm)
            # Made of the mesh's coordinates (``pcolormesh(X, Y, C)``).
            solids._coordinates_masked = True
        # The extensions' z-order is matplotlib's ``np.nextafter`` of the
        # Axes background's, a numpy float.
        for patch in self._extend_patches:
            _kept_values.remember(patch, "zorder", np.float64(patch.get_zorder()))

    @_kept_values.assignable
    def solids_patches(self):
        """The hatched patches drawn as the colour blocks of a hatched
        contour set, in place of `solids`."""
        from .patches import PathPatch
        return _handles.wrap_all(_colorbar.solids_patches(self._native),
                                 PathPatch, self._figure)

    @_kept_values.assignable
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

    @_kept_values.assignable
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

    def _short_axis(self):
        """Return the short axis"""
        if self.orientation == 'vertical':
            return self.ax.xaxis
        return self.ax.yaxis

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
        """
        # pltrs: sets the colorbar's locators and formatter back on its long
        # axis: those given through `locator`, `minorlocator` and `formatter`,
        # else those chosen for its norm.
        self.long_axis.set_major_locator(self._locator)
        self.long_axis.set_minor_locator(self._minorlocator)
        self.long_axis.set_major_formatter(self._formatter)

    # matplotlib 3.11.2's, under the Matplotlib License: the view of a
    # colorbar's interactive navigation is its norm's range.
    def _get_view(self):
        return self.norm.vmin, self.norm.vmax

    def _set_view(self, view):
        self.norm.vmin, self.norm.vmax = view

    def drag_pan(self, button, key, x, y):
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
        if mappable:
            # The mappable keyword argument exists because
            # ScalarMappable.changed() emits self.callbacks.process('changed', self)
            # in contrast, ColorizingArtist (and Colorizer) emit
            # self.callbacks.process('changed') without passing a mappable.
            self.mappable = mappable
        # What a program assigned that matplotlib's update sets anew: the
        # colormap and norm of the mappable, the range and the colours
        # (``_draw_all``).
        _kept_values.take(self, "cmap", "norm", "vmin", "vmax", "solids", "solids_patches")
        _log.debug('colorbar update normal %r %r', self.mappable.norm, self.norm)
        self.set_alpha(self.mappable.get_alpha())
        self._scale_norm()
        _colorbar.update_normal(self._native, self._shown())
        if self.mappable.norm is not self._ticker_norm:
            self._keep_tickers()
        self._adopt_solids()
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

            If you are using this method, you should always fix the tick
            positions before, e.g. by using `.Colorbar.set_ticks` or by
            explicitly setting a `~.ticker.FixedLocator` on the long axis
            of the colorbar. Otherwise, ticks are free to move and the
            labels may end up in unexpected positions.

        Parameters
        ----------
        ticklabels : sequence of str or of `.Text`
            Texts for labeling each tick location in the sequence set by
            `.Colorbar.set_ticks`; the number of labels must match the number
            of locations.

        update_ticks : bool, default: True
            This keyword argument is ignored and will be removed.
            Deprecated

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
        """
        Turn on colorbar minor ticks.
        """
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

        The lines are appended to the list :attr:`!lines`.

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
        erase = bool(params["erase"])
        # A list the program assigned to `lines`: its lines are removed when
        # erased, else the new ones are appended to it, as matplotlib's are.
        assigned = _kept_values.take(self, "lines").get("lines")
        if erase and assigned:
            for lines in assigned:
                lines.remove()
        if "CS" in params:
            cs = params["CS"]
            if not isinstance(cs, contour.ContourSet) or cs.filled:
                raise ValueError("If a single artist is passed to add_lines, "
                                 "it must be a ContourSet of lines")
            _colorbar.add_contour_lines(self._native, cs._contour, erase)
        else:
            _colorbar.add_level_lines(
                self._native,
                [float(level) for level in np.atleast_1d(params["levels"])],
                [tuple(rgba) for rgba in mcolors.to_rgba_array(params["colors"])],
                [float(width) for width in np.atleast_1d(params["linewidths"])],
                erase)
        if assigned is not None and not erase:
            assigned.append(self.lines[-1])
            self.lines = assigned

    def remove(self):
        """
        Remove this colorbar from the figure.

        If the colorbar was created with ``use_gridspec=True`` the previous
        gridspec is restored.
        """
        # matplotlib's takes the Axes out first, with the removal method it
        # had (its figure's `delaxes`), which fails on an Axes taken out.
        self.ax._check_removable()
        self.mappable.callbacks.disconnect(self.mappable.colorbar_cid)
        self.mappable.colorbar = None
        self.mappable.colorbar_cid = None
        # Remove the extension callbacks
        self.ax.callbacks.disconnect(self._extend_cid1)
        self.ax.callbacks.disconnect(self._extend_cid2)
        ax = self.ax
        _colorbar.remove(self._native)
        _handles.forget(ax)
        self._removed_ax = ax
        self._give_back_place(ax)

    def _give_back_place(self, ax):
        """Fail as matplotlib's removal fails at its end, giving the Axes of
        the mappable back the place the colorbar Axes *ax* took from it,
        when the mappable has no Axes (removed): pltrs (Rust) gave the
        place back."""
        try:
            parent = self.mappable.axes
        except AttributeError:
            return
        if parent is None:
            try:
                ax.get_subplotspec().get_gridspec()._subplot_spec
            except AttributeError:  # use_gridspec was False
                getattr(parent, "get_position")
            getattr(parent, "set_subplotspec")


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
    aspect = layout['aspect']
    # No aspect (None) is matplotlib's box aspect of none, set at the end.
    layout['aspect'] = 1.0 if aspect is None else float(aspect)
    if layout['pad'] is not None:
        layout['pad'] = float(layout['pad'])
    native, kwargs['orientation'], kwargs['ticklocation'] = make(natives, layout)
    figure = parents[0].get_figure(root=True)
    cax = _handles.wrap(native, Axes, figure)
    cax.set_label('<colorbar>')
    if aspect is None:
        cax.set_box_aspect(None)
    return cax, kwargs


@_docstring.interpd
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
    %(_make_axes_kw_doc)s

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
    _colorbar_checks.check_make_axes(parents, location, orientation, fraction,
                                     shrink, aspect, kwargs)
    kwargs.update(location=location, orientation=orientation,
                  fraction=fraction, shrink=shrink, aspect=aspect)
    return _made_axes(_colorbar.make_axes, parents,
                      [ax._native for ax in parents], kwargs)


@_docstring.interpd
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
    %(_make_axes_kw_doc)s

    Returns
    -------
    cax : `~matplotlib.axes.Axes`
        The child Axes.
    kwargs : dict
        The reduced keyword dictionary to be passed when creating the colorbar
        instance.
    """
    _colorbar_checks.check_make_axes_gridspec(parent, location, orientation,
                                              fraction, shrink, aspect, kwargs)
    kwargs.update(location=location, orientation=orientation,
                  fraction=fraction, shrink=shrink, aspect=aspect)
    return _made_axes(_colorbar.make_axes_gridspec, [parent], parent._native,
                      kwargs)


# What matplotlib's module also has from its imports, which programs
# import from it.
from . import cbook, collections, colors, contour  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
