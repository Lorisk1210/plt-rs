"""
Support for plotting vector fields (matplotlib's ``quiver``).

Presently this contains Quiver and Barb. Quiver plots an arrow in the
direction of the vector, with the size of the arrow related to the
magnitude of the vector.

Barbs are like quiver in that they point along a vector, but
the magnitude of the vector is given schematically by the presence of barbs
or flags on the barb.

This will also become a home for things such as standard
deviation ellipses, which can and will be derived very easily from
the Quiver code.

The argument parsing and checks are matplotlib's (from matplotlib 3.11.2,
under the Matplotlib License); pltrs (Rust) makes the arrows, barbs and
keys, sizing the arrows from the Axes when the figure is drawn, as
matplotlib's ``Quiver.draw`` does.
"""

import numpy as np
from numpy import ma

from . import _docstring
from . import (_api, _handles, _live_transforms, _unusable, _values, cbook,
               collections as mcollections, colors as mcolors, transforms)
from . import artist as martist
from ._native import artist as _artist, plotting as _plotting


def _parse_args(*args, caller_name='function'):
    """
    Helper function to parse positional parameters for colored vector plots.

    This is currently used for Quiver and Barbs.

    Parameters
    ----------
    *args : list
        list of 2-5 arguments. Depending on their number they are parsed to::

            U, V
            U, V, C
            X, Y, U, V
            X, Y, U, V, C

    caller_name : str
        Name of the calling method (used in error messages).
    """
    X = Y = C = None

    nargs = len(args)
    if nargs == 2:
        # The use of atleast_1d allows for handling scalar arguments while also
        # keeping masked arrays
        U, V = np.atleast_1d(*args)
    elif nargs == 3:
        U, V, C = np.atleast_1d(*args)
    elif nargs == 4:
        X, Y, U, V = np.atleast_1d(*args)
    elif nargs == 5:
        X, Y, U, V, C = np.atleast_1d(*args)
    else:
        raise _api.nargs_error(caller_name, takes="from 2 to 5", given=nargs)

    nr, nc = (1, U.shape[0]) if U.ndim == 1 else U.shape

    if X is not None:
        X = X.ravel()
        Y = Y.ravel()
        if len(X) == nc and len(Y) == nr:
            X, Y = (a.ravel() for a in np.meshgrid(X, Y))
        elif len(X) != len(Y):
            raise ValueError('X and Y must be the same size, but '
                             f'X.size is {X.size} and Y.size is {Y.size}.')
    else:
        indexgrid = np.meshgrid(np.arange(nc), np.arange(nr))
        X, Y = (np.ravel(a) for a in indexgrid)
    # Size validation for U, V, C is left to the set_UVC method.
    return X, Y, U, V, C


def _check_consistent_shapes(*arrays):
    all_shapes = {a.shape for a in arrays}
    if len(all_shapes) != 1:
        raise ValueError('The shapes of the passed in arrays do not match')


def _filled(values):
    """*values* (a masked array) as floats, NaN where masked."""
    return ma.filled(ma.asarray(values, dtype=float), np.nan)


#: The coordinates a `QuiverKey` is placed in, in the order of matplotlib's
#: transforms of them (its ``_set_transform``).
_QUIVERKEY_COORDINATES = dict.fromkeys(["data", "axes", "figure", "inches"])


def _rgba(color):
    """*color* as an RGBA tuple, or None."""
    return None if color is None else tuple(map(float, mcolors.to_rgba(color)))


def _single_color(color):
    """Whether *color* is one colour (rather than one per item)."""
    try:
        mcolors.to_rgba(color)
    except ValueError:
        return False
    return True


def _mapped(C, cmap, norm, clim):
    """
    The colour mapping of the values *C*: ``(colorizer, native)``, the
    Python `.Colorizer` and the native values and mapping (both None
    without values).
    """
    if C is None:
        return None, None
    from .axes._plotting.scatter import _mapping
    vmin, vmax = clim if clim is not None else (None, None)
    made, mapping = _mapping(cmap, norm, vmin, vmax)
    return made, (_filled(C).ravel(), mapping)


class QuiverKey(martist.Artist):
    """Labelled arrow for use as a quiver plot scale key."""

    def set_figure(self, fig):
        # docstring inherited: matplotlib's override has none.
        super().set_figure(fig)
    halign = {'N': 'center', 'S': 'center', 'E': 'left', 'W': 'right'}
    valign = {'N': 'bottom', 'S': 'top', 'E': 'center', 'W': 'center'}
    pivot = {'N': 'middle', 'S': 'middle', 'E': 'tip', 'W': 'tail'}

    def __init__(self, Q, X, Y, U, label,
                 *, angle=0, coordinates='axes', color=None, labelsep=0.1,
                 labelpos='N', labelcolor=None, fontproperties=None,
                 zorder=None, **kwargs):
        """
        Add a key to a quiver plot.

        The positioning of the key depends on *X*, *Y*, *coordinates*, and
        *labelpos*.  If *labelpos* is 'N' or 'S', *X*, *Y* give the position of
        the middle of the key arrow.  If *labelpos* is 'E', *X*, *Y* positions
        the head, and if *labelpos* is 'W', *X*, *Y* positions the tail; in
        either of these two cases, *X*, *Y* is somewhere in the middle of the
        arrow+label key object.

        Parameters
        ----------
        Q : `~matplotlib.quiver.Quiver`
            A `.Quiver` object as returned by a call to `~.Axes.quiver()`.
        X, Y : float
            The location of the key.
        U : float
            The length of the key.
        label : str
            The key label (e.g., length and units of the key).
        angle : float, default: 0
            The angle of the key arrow, in degrees anti-clockwise from the
            horizontal axis.
        coordinates : {'axes', 'figure', 'data', 'inches'}, default: 'axes'
            Coordinate system and units for *X*, *Y*: 'axes' and 'figure' are
            normalized coordinate systems with (0, 0) in the lower left and
            (1, 1) in the upper right; 'data' are the axes data coordinates
            (used for the locations of the vectors in the quiver plot itself);
            'inches' is position in the figure in inches, with (0, 0) at the
            lower left corner.
        color : :mpltype:`color`
            Overrides face and edge colors from *Q*.
        labelpos : {'N', 'S', 'E', 'W'}
            Position the label above, below, to the right, to the left of the
            arrow, respectively.
        labelsep : float, default: 0.1
            Distance in inches between the arrow and the label.
        labelcolor : :mpltype:`color`, default: :rc:`text.color`
            Label color.
        fontproperties : dict, optional
            A dictionary with keyword arguments accepted by the
            `~matplotlib.font_manager.FontProperties` initializer:
            *family*, *style*, *variant*, *size*, *weight*.
        zorder : float
            The zorder of the key. The default is 0.1 above *Q*.
        **kwargs
            Any additional keyword arguments are used to override vector
            properties taken from *Q*.
        """
        # matplotlib's constructor aligns the label by its position.
        self.halign[labelpos]
        self.Q = Q
        self.X = X
        self.Y = Y
        self.U = U
        self.angle = angle
        self.coord = coordinates
        self.color = color
        self.label = label
        self._labelsep_inches = labelsep
        self.labelpos = labelpos
        self._kw = kwargs  # Remove when kw deprecation elapses.
        font = None
        if fontproperties is not None:
            from .font_manager import FontManager, FontProperties
            font = FontManager._props(FontProperties._from_any(fontproperties))
        # matplotlib keeps coordinates and a colour as they are, failing on
        # what is none as it draws the key: pltrs (Rust) places such a key
        # in axes fractions and colours its arrow as its quiver's.
        usable_coords = (isinstance(coordinates, str)
                         and coordinates in _QUIVERKEY_COORDINATES)
        usable_color = color is None or mcolors.is_color_like(color)
        # The Axes the quiver is on, or was made for before it is added.
        made_for = Q.axes or Q._made_for
        native = _plotting.quiverkey(
            made_for._native, Q._native, (float(X), float(Y)), float(U), str(label),
            (float(angle), coordinates if usable_coords else "axes",
             _rgba(color) if usable_color else None,
             None if zorder is None else float(zorder)),
            (float(labelsep), labelpos, _rgba(labelcolor), font),
            mcollections._native_kwargs(
                cbook.normalize_kwargs(kwargs, mcollections.PolyCollection)))
        _handles.adopt(self, native, made_for.figure)
        if not usable_coords:
            _unusable.keep(self, "coordinates", coordinates,
                           lambda coords: _api.getitem_checked(
                               _QUIVERKEY_COORDINATES, coordinates=coords),
                           measured=False)
        if not usable_color:
            _unusable.keep(self, "color", color, mcolors.to_rgba_array,
                           measured=False)
        self._vector = mcollections.PolyCollection([], **{**Q.polykw, **kwargs})
        from .text import Text
        self.text = _handles.wrap(_plotting.quiverkey_text(native), Text,
                                  made_for.figure)
        # matplotlib's constructor makes the key in no figure, which its
        # Axes' ``quiverkey`` (or the program) then adds with
        # ``add_artist``: pltrs (Rust) made it apart until then.
        self._decouple(removed=False)

    kw = _api.deprecated("3.11")(  # Also remove self._kw when deprecation elapses.
        property(lambda self: self._kw))
    fontproperties = _api.deprecated(
        "3.11", alternative="quiverkey.text.get_fontproperties()")(
            property(lambda self: self.text.get_fontproperties()))
    labelcolor = _api.deprecated(
        "3.11", alternative="quiverkey.text.get_color()")(
            property(lambda self: self.text.get_color()))
    verts = _api.deprecated(
        "3.11", alternative="[p.vertices for p in quiverkey.vector.get_paths()]")(
            property(lambda self: [p.vertices for p in self.vector.get_paths()]))

    @property
    def labelsep(self):
        return self._labelsep_inches * self.Q.axes.get_figure(root=True).dpi

    @property
    def vector(self):
        """
        The key's arrow, a `.PolyCollection` (matplotlib's, which holds no
        arrow until the key is drawn): the arrow pltrs (Rust) drew last.
        """
        draws = 0 if self._figure is None else _artist.draw_count(self._native)
        if draws and self.__dict__.get("_vector_draws") != draws:
            self._vector_draws = draws
            # The outline without its closing vertex, which the collection
            # adds back.
            self._vector.set(
                verts=[vertices[:-1] for vertices, _
                       in _plotting.quiverkey_paths(self._native)],
                offsets=[(self.X, self.Y)],
                offset_transform=self.get_transform(),
                transform=self.Q.get_transform())
            if self.color is not None:
                self._vector.set_color(self.color)
        return self._vector

    @vector.setter
    def vector(self, collection):
        self._vector = collection

    def get_transform(self):
        """Return the transform of the key's coordinates once the key was
        drawn (matplotlib's ``_set_transform``, which its draw runs)."""
        if self._figure is None or not _artist.draw_count(self._native):
            return super().get_transform()
        fig = self.Q.axes.get_figure(root=False)
        return _api.getitem_checked({
            "data": self.Q.axes.transData,
            "axes": self.Q.axes.transAxes,
            "figure": fig.transFigure,
            "inches": fig.dpi_scale_trans,
        }, coordinates=self.coord)

    @martist.allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    def contains(self, mouseevent):
        # docstring inherited
        return super().contains(mouseevent)


class Quiver(mcollections.PolyCollection):
    """
    Specialized PolyCollection for arrows.

    Use set_UVC to change the size, orientation, and color of the
    arrows; their locations can be set using set_offsets().

    Much of the work in this class is done in the draw()
    method so that as much information as possible is available
    about the plot.  In subsequent draw() calls, recalculation
    is limited to things that might have changed, so there
    should be no performance penalty from putting the calculations
    in the draw() method.
    """
    # pltrs makes the arrows for the Axes *ax* when the object is made,
    # which `~.Axes.quiver` adds to them, and sizes them from the Axes
    # whenever the figure is drawn.

    _PIVOT_VALS = ('tail', 'middle', 'tip')

    def __init__(self, ax, *args,
                 scale=None, headwidth=3, headlength=5, headaxislength=4.5,
                 minshaft=1, minlength=1, units='width', scale_units=None,
                 angles='uv', width=None, color='k', pivot='tail', **kwargs):
        """
        The constructor takes one required argument, an Axes
        instance, followed by the args and kwargs described
        by the following pyplot interface documentation:
        %s
        """
        # The Axes the arrows are made for, which a key made for them
        # before they are added is made for too.
        self._made_for = ax
        X, Y, U, V, C = _parse_args(*args, caller_name='quiver')
        self.X = X
        self.Y = Y
        self.XY = np.column_stack((X, Y))
        self.N = len(X)
        self.headwidth = headwidth
        self.headlength = float(headlength)
        self.headaxislength = headaxislength
        self.minshaft = minshaft
        self.minlength = minlength
        self.units = units
        self.scale_units = scale_units
        self.angles = angles

        if pivot.lower() == 'mid':
            pivot = 'middle'
        self.pivot = pivot.lower()
        _api.check_in_list(self._PIVOT_VALS, pivot=self.pivot)

        transform = kwargs.pop('transform', None)
        # The keywords of the collection, as matplotlib keeps them for the
        # key's arrow.
        polykw = {**kwargs}
        polykw.setdefault('facecolors', color)
        polykw.setdefault('linewidths', (0,))
        cmap, norm, clim = (kwargs.pop(key, None) for key in ('cmap', 'norm', 'clim'))
        kwargs = cbook.normalize_kwargs(kwargs, mcollections.PolyCollection)
        facecolor = kwargs.pop('facecolor', color)
        U, V, C = self._checked_uvc(U, V, C)
        made, mapped = _mapped(C, cmap, norm, clim)
        native = _plotting.quiver(
            ax._native, (self.XY[:, 0].astype(float), self.XY[:, 1].astype(float)),
            (_filled(U), _filled(V)), mapped,
            (None if scale is None else float(scale), scale_units, units,
             angles if isinstance(angles, str) else np.ravel(angles).astype(float),
             None if width is None else float(width)),
            (float(headwidth), float(headlength), float(headaxislength),
             float(minshaft), float(minlength)),
            (self.pivot, _rgba(facecolor) if _single_color(facecolor)
             else (0., 0., 0., 1.)),
            _live_transforms.given_to_native(transform, ax))
        _handles.adopt(self, native, ax.figure)
        for name, value in (("scale", scale), ("width", width)):
            if value is not None:
                _values.remember(self, name, value)
        # matplotlib's attribute: the transform of the arrows' positions.
        self.transform = ax.transData if transform is None else transform
        self._attach_native_mapping(colorizer=made)
        if not _single_color(facecolor):
            self.set_facecolor(facecolor)
        # The arrows are unstroked, as matplotlib's default ``(0,)`` says.
        _values.remember(self, "linewidth", np.atleast_1d((0,)))
        self._internal_update(kwargs)
        self.polykw = polykw
        self._store_uvc(U, V, C)
        # matplotlib's constructor puts the arrows on no figure: they join
        # that of the Axes they are added to.
        self._decouple(removed=False)

    def _sizes(index, name, doc):
        """The attribute *name*, the *index* of what pltrs (Rust) gives
        for the arrows' sizes: as the program gave it, or as the first
        draw worked it out (matplotlib's ``_init`` and ``_make_verts``
        set it once), None before."""
        def get(self):
            native = _plotting.quiver_sizes(self._native)[index]
            return _values.given(
                self, name, None if native is None else np.float64(native))

        def set(self, value):
            _plotting.quiver_set_size(
                self._native, name, None if value is None else float(value))
            _values.remember(self, name, value)
            self.stale = True
        return property(get, set, doc=doc)

    scale = _sizes(0, "scale", "Arrow-width units per unit of arrow length.")
    width = _sizes(1, "width", "The shaft width, in arrow-width units.")
    del _sizes

    @property
    def span(self):
        """The width of the Axes in arrow-width units at the last draw
        (set by matplotlib's ``_init``)."""
        span = _plotting.quiver_sizes(self._native)[2]
        if span is None:
            raise AttributeError(
                f"{type(self).__name__!r} object has no attribute 'span'")
        return np.float64(span)

    def get_paths(self):
        """The arrows of the last draw, in arrow-width units (each draw
        makes them anew, matplotlib's ``set_verts(verts, closed=False)``);
        none before the first."""
        from .path import Path
        return [Path(vertices) for vertices in _plotting.quiver_paths(self._native)]

    @martist.allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    def _checked_uvc(self, U, V, C):
        """*U*, *V* and *C* as masked arrays sharing their mask, checked
        against the number of arrows."""
        # We need to ensure we have a copy, not a reference
        # to an array that might change before draw().
        U = ma.masked_invalid(U, copy=True).ravel()
        V = ma.masked_invalid(V, copy=True).ravel()
        if C is not None:
            C = ma.masked_invalid(C, copy=True).ravel()
        for name, var in zip(('U', 'V', 'C'), (U, V, C)):
            if not (var is None or var.size == self.N or var.size == 1):
                raise ValueError(f'Argument {name} has a size {var.size}'
                                 f' which does not match {self.N},'
                                 ' the number of arrow positions')

        mask = ma.mask_or(U.mask, V.mask, copy=False, shrink=True)
        if C is not None:
            mask = ma.mask_or(mask, C.mask, copy=False, shrink=True)
            if mask is ma.nomask:
                C = C.filled()
            else:
                C = ma.array(C, mask=mask, copy=False)
        return (ma.array(U, mask=mask, copy=False), ma.array(V, mask=mask, copy=False),
                C)

    def _store_uvc(self, U, V, C):
        """Keep the components as matplotlib's attributes."""
        self.U = U.filled(1)
        self.V = V.filled(1)
        # ``nomask`` when nothing is masked, as matplotlib's ``mask_or``
        # shrinks it.
        self.Umask = ma.make_mask(ma.getmask(U), shrink=True)
        self._C = C

    def get_transform(self):
        """
        Return the transform of the arrows: from arrow width units to
        pixels once the quiver was drawn (matplotlib's ``_set_transform``,
        which each draw runs, setting a new `.Affine2D`), the one given
        before.
        """
        draws = 0 if self._figure is None else _artist.draw_count(self._native)
        if not draws:
            return super().get_transform()
        made = self.__dict__.get("_drawn_transform")
        if made is None or made[0] != draws:
            made = self._drawn_transform = (
                draws, transforms.Affine2D().scale(self._dots_per_unit(self.units)))
        return made[1]

    def _keep_drawn_state(self):
        # The transform of its arrows, made with its Axes.
        self.get_transform()

    def _dots_per_unit(self, units):
        """Return a scale factor for converting from units to pixels."""
        bb = self.axes.bbox
        vl = self.axes.viewLim
        return _api.getitem_checked({
            'x': bb.width / vl.width,
            'y': bb.height / vl.height,
            'xy': np.hypot(*bb.size) / np.hypot(*vl.size),
            'width': bb.width,
            'height': bb.height,
            'dots': 1.,
            'inches': self.axes.get_figure(root=True).dpi,
        }, units=units)

    def get_datalim(self, transData):
        # docstring inherited
        trans = self.get_transform()
        offset_trf = self.get_offset_transform()
        full_transform = (trans - transData) + (offset_trf - transData)
        XY = full_transform.transform(self.XY)
        bbox = transforms.Bbox.null()
        bbox.update_from_data_xy(XY, ignore=True)
        return bbox

    def set_UVC(self, U, V, C=None):
        U, V, C = self._checked_uvc(U, V, C)
        _plotting.quiver_set_uvc(self._native, _filled(U), _filled(V))
        if C is not None:
            self.set_array(C)
        self._store_uvc(U, V, C)


_quiver_doc = r"""
Plot a 2D field of arrows.

Call signature::

  quiver([X, Y], U, V, [C], /, **kwargs)

*X*, *Y* define the arrow locations, *U*, *V* define the arrow directions, and
*C* optionally sets the color. The arguments *X*, *Y*, *U*, *V*, *C* are
positional-only.

**Arrow length**

The default settings auto-scales the length of the arrows to a reasonable size.
To change this behavior see the *scale* and *scale_units* parameters.

**Arrow shape**

The arrow shape is determined by *width*, *headwidth*, *headlength* and
*headaxislength*. See the notes below.

**Arrow styling**

Each arrow is internally represented by a filled polygon with a default edge
linewidth of 0. As a result, an arrow is rather a filled area, not a line with
a head, and `.PolyCollection` properties like *linewidth*, *edgecolor*,
*facecolor*, etc. act accordingly.


Parameters
----------
X, Y : 1D or 2D array-like, optional
    The x and y coordinates of the arrow locations.

    If not given, they will be generated as a uniform integer meshgrid based
    on the dimensions of *U* and *V*.

    If *X* and *Y* are 1D but *U*, *V* are 2D, *X*, *Y* are expanded to 2D
    using ``X, Y = np.meshgrid(X, Y)``. In this case ``len(X)`` and ``len(Y)``
    must match the column and row dimensions of *U* and *V*.

U, V : 1D or 2D array-like
    The x and y direction components of the arrow vectors. The interpretation
    of these components (in data or in screen space) depends on *angles*.

    *U* and *V* must have the same number of elements, matching the number of
    arrow locations in *X*, *Y*. *U* and *V* may be masked. Locations masked
    in any of *U*, *V*, and *C* will not be drawn.

C : 1D or 2D array-like, optional
    Numeric data that defines the arrow colors by colormapping via *norm* and
    *cmap*.

    This does not support explicit colors. If you want to set colors directly,
    use *color* instead.  The size of *C* must match the number of arrow
    locations.

angles : {'uv', 'xy'} or array-like, default: 'uv'
    Method for determining the angle of the arrows.

    - 'uv':  Arrow directions are based on
      :ref:`display coordinates <coordinate-systems>`; i.e. a 45° angle will
      always show up as diagonal on the screen, irrespective of figure or Axes
      aspect ratio or Axes data ranges. This is useful when the arrows represent
      a quantity whose direction is not tied to the x and y data coordinates.

      If *U* == *V* the orientation of the arrow on the plot is 45 degrees
      counter-clockwise from the horizontal axis (positive to the right).

    - 'xy': Arrow direction in data coordinates, i.e. the arrows point from
      (x, y) to (x+u, y+v). This is ideal for vector fields or gradient plots
      where the arrows should directly represent movements or gradients in the
      x and y directions.

    - Arbitrary angles may be specified explicitly as an array of values
      in degrees, counter-clockwise from the horizontal axis.

      In this case *U*, *V* is only used to determine the length of the
      arrows.

      For example, ``angles=[30, 60, 90]`` will orient the arrows at 30, 60, and 90
      degrees respectively, regardless of the *U* and *V* components.

    Note: inverting a data axis will correspondingly invert the
    arrows only with ``angles='xy'``.

pivot : {'tail', 'middle', 'tip'}, default: 'tail'
    The part of the arrow that is anchored to the *X*, *Y* grid. The arrow
    rotates about this point.

    .. admonition:: Discouraged

        'mid' is a synonym for 'middle', which is retained for backwards
        compatibility.  New code should use 'middle'.

scale : float, optional
    Scales the length of the arrow inversely.

    Number of data values represented by one unit of arrow length on the plot.
    For example, if the data represents velocity in meters per second (m/s), the
    scale parameter determines how many meters per second correspond to one unit of
    arrow length relative to the width of the plot.
    Smaller scale parameter makes the arrow longer.

    By default, an autoscaling algorithm is used to scale the arrow length to a
    reasonable size, which is based on the average vector length and the number of
    vectors.

    The arrow length unit is given by the *scale_units* parameter.

scale_units : {'width', 'height', 'dots', 'inches', 'x', 'y', 'xy'}, default: 'width'
    The physical image unit, which is used for rendering the scaled arrow data U, V.

    The rendered arrow length is given by:

    - length in x direction = :math:`\frac{u}{\mathrm{scale}}\,\mathrm{scale\_unit}`
    - length in y direction = :math:`\frac{v}{\mathrm{scale}}\,\mathrm{scale\_unit}`
    For example, ``(u, v) = (0.5, 0)`` with ``scale=10, scale_units="width"`` results
    in a horizontal arrow with a length of *0.5 / 10 * "width"*, i.e. 0.05 times the
    Axes width.

    Supported values are:

    - 'width' or 'height': The arrow length is scaled relative to the width or height
       of the Axes.
       For example, ``scale_units='width', scale=1.0``, will result in an arrow length
       of width of the Axes.

    - 'dots': The arrow length of the arrows is in measured in display dots (pixels).

    - 'inches': Arrow lengths are scaled based on the DPI (dots per inch) of the figure.
       This ensures that the arrows have a consistent physical size on the figure,
       in inches, regardless of data values or plot scaling.
       For example, ``(u, v) = (1, 0)`` with ``scale_units='inches', scale=2`` results
       in a 0.5 inch-long arrow.

    - 'x' or 'y': The arrow length is scaled relative to the x or y axis units.
       For example, ``(u, v) = (0, 1)`` with ``scale_units='x', scale=1`` results
       in a vertical arrow with the length of 1 x-axis unit.

    - 'xy': Arrow length will be same as 'x' or 'y' units.
       This is useful for creating vectors in the x-y plane where u and v have
       the same units as x and y. To plot vectors in the x-y plane with u and v having
       the same units as x and y, use ``angles='xy', scale_units='xy', scale=1``.

    Note: Setting *scale_units* without setting scale does not have any effect because
    the scale units only differ by a constant factor and that is rescaled through
    autoscaling.

units : {'width', 'height', 'dots', 'inches', 'x', 'y', 'xy'}, default: 'width'
    Affects the arrow size (except for the length). In particular, the shaft
    *width* is measured in multiples of this unit.

    Supported values are:

    - 'width', 'height': The width or height of the Axes.
    - 'dots', 'inches': Pixels or inches based on the figure dpi.
    - 'x', 'y', 'xy': *X*, *Y* or :math:`\\sqrt{X^2 + Y^2}` in data units.

    The following table summarizes how these values affect the visible arrow
    size under zooming and figure size changes:

    =================  =================   ==================
    units              zoom                figure size change
    =================  =================   ==================
    'x', 'y', 'xy'     arrow size scales   —
    'width', 'height'  —                   arrow size scales
    'dots', 'inches'   —                   —
    =================  =================   ==================

width : float, optional
    Shaft width in arrow units. All head parameters are relative to *width*.

    The default depends on choice of *units* above, and number of vectors;
    a typical starting value is about 0.005 times the width of the plot.

headwidth : float, default: 3
    Head width as multiple of shaft *width*. See the notes below.

headlength : float, default: 5
    Head length as multiple of shaft *width*. See the notes below.

headaxislength : float, default: 4.5
    Head length at shaft intersection as multiple of shaft *width*.
    See the notes below.

minshaft : float, default: 1
    Length below which arrow scales, in units of head length. Do not
    set this to less than 1, or small arrows will look terrible!

minlength : float, default: 1
    Minimum length as a multiple of shaft width; if an arrow length
    is less than this, plot a dot (hexagon) of this diameter instead.

color : :mpltype:`color` or list :mpltype:`color`, optional
    Explicit color(s) for the arrows. If *C* has been set, *color* has no
    effect.

    This is a synonym for the `.PolyCollection` *facecolor* parameter.

Other Parameters
----------------
data : indexable object, optional
    DATA_PARAMETER_PLACEHOLDER

**kwargs : `~matplotlib.collections.PolyCollection` properties, optional
    All other keyword arguments are passed on to `.PolyCollection`:

    %(PolyCollection:kwdoc)s

Returns
-------
`~matplotlib.quiver.Quiver`

See Also
--------
.Axes.quiverkey : Add a key to a quiver plot.

Notes
-----

**Arrow shape**

The arrow is drawn as a polygon using the nodes as shown below. The values
*headwidth*, *headlength*, and *headaxislength* are in units of *width*.

.. image:: /_static/quiver_sizes.svg
   :width: 500px

The defaults give a slightly swept-back arrow. Here are some guidelines how to
get other head shapes:

- To make the head a triangle, make *headaxislength* the same as *headlength*.
- To make the arrow more pointed, reduce *headwidth* or increase *headlength*
  and *headaxislength*.
- To make the head smaller relative to the shaft, scale down all the head
  parameters proportionally.
- To remove the head completely, set all *head* parameters to 0.
- To get a diamond-shaped head, make *headaxislength* larger than *headlength*.
- Warning: For *headaxislength* < (*headlength* / *headwidth*), the "headaxis"
  nodes (i.e. the ones connecting the head with the shaft) will protrude out
  of the head in forward direction so that the arrow head looks broken.
""" % _docstring.interpd.params

_docstring.interpd.register(quiver_doc=_quiver_doc)

# The constructor's docstring, as matplotlib's substitutes it (the class is
# defined above, which its plotting functions need).
_docstring.Substitution(_quiver_doc)(Quiver.__init__)

_barbs_doc = r"""
Plot a 2D field of wind barbs.

Call signature::

  barbs([X, Y], U, V, [C], /, **kwargs)

Where *X*, *Y* define the barb locations, *U*, *V* define the barb
directions, and *C* optionally sets the color.

The arguments *X*, *Y*, *U*, *V*, *C* are positional-only and may be
1D or 2D. *U*, *V*, *C* may be masked arrays, but masked *X*, *Y*
are not supported at present.

Barbs are traditionally used in meteorology as a way to plot the speed
and direction of wind observations, but can technically be used to
plot any two dimensional vector quantity.  As opposed to arrows, which
give vector magnitude by the length of the arrow, the barbs give more
quantitative information about the vector magnitude by putting slanted
lines or a triangle for various increments in magnitude, as show
schematically below::

  :                   /\    \
  :                  /  \    \
  :                 /    \    \    \
  :                /      \    \    \
  :               ------------------------------

The largest increment is given by a triangle (or "flag"). After those
come full lines (barbs). The smallest increment is a half line.  There
is only, of course, ever at most 1 half line.  If the magnitude is
small and only needs a single half-line and no full lines or
triangles, the half-line is offset from the end of the barb so that it
can be easily distinguished from barbs with a single full line.  The
magnitude for the barb shown above would nominally be 65, using the
standard increments of 50, 10, and 5.

See also https://en.wikipedia.org/wiki/Wind_barb.

Parameters
----------
X, Y : 1D or 2D array-like, optional
    The x and y coordinates of the barb locations. See *pivot* for how the
    barbs are drawn to the x, y positions.

    If not given, they will be generated as a uniform integer meshgrid based
    on the dimensions of *U* and *V*.

    If *X* and *Y* are 1D but *U*, *V* are 2D, *X*, *Y* are expanded to 2D
    using ``X, Y = np.meshgrid(X, Y)``. In this case ``len(X)`` and ``len(Y)``
    must match the column and row dimensions of *U* and *V*.

U, V : 1D or 2D array-like
    The x and y components of the barb shaft.

C : 1D or 2D array-like, optional
    Numeric data that defines the barb colors by colormapping via *norm* and
    *cmap*.

    This does not support explicit colors. If you want to set colors directly,
    use *barbcolor* instead.

length : float, default: 7
    Length of the barb in points; the other parts of the barb
    are scaled against this.

pivot : {'tip', 'middle'} or float, default: 'tip'
    The part of the arrow that is anchored to the *X*, *Y* grid. The barb
    rotates about this point. This can also be a number, which shifts the
    start of the barb that many points away from grid point.

barbcolor : :mpltype:`color` or color sequence
    The color of all parts of the barb except for the flags.  This parameter
    is analogous to the *edgecolor* parameter for polygons, which can be used
    instead. However this parameter will override facecolor.

flagcolor : :mpltype:`color` or color sequence
    The color of any flags on the barb.  This parameter is analogous to the
    *facecolor* parameter for polygons, which can be used instead. However,
    this parameter will override facecolor.  If this is not set (and *C* has
    not either) then *flagcolor* will be set to match *barbcolor* so that the
    barb has a uniform color. If *C* has been set, *flagcolor* has no effect.

sizes : dict, optional
    A dictionary of coefficients specifying the ratio of a given
    feature to the length of the barb. Only those values one wishes to
    override need to be included.  These features include:

    - 'spacing' - space between features (flags, full/half barbs)
    - 'height' - height (distance from shaft to top) of a flag or full barb
    - 'width' - width of a flag, twice the width of a full barb
    - 'emptybarb' - radius of the circle used for low magnitudes

fill_empty : bool, default: False
    Whether the empty barbs (circles) that are drawn should be filled with
    the flag color.  If they are not filled, the center is transparent.

rounding : bool, default: True
    Whether the vector magnitude should be rounded when allocating barb
    components.  If True, the magnitude is rounded to the nearest multiple
    of the half-barb increment.  If False, the magnitude is simply truncated
    to the next lowest multiple.

barb_increments : dict, optional
    A dictionary of increments specifying values to associate with
    different parts of the barb. Only those values one wishes to
    override need to be included.

    - 'half' - half barbs (Default is 5)
    - 'full' - full barbs (Default is 10)
    - 'flag' - flags (default is 50)

flip_barb : bool or array-like of bool, default: False
    Whether the lines and flags should point opposite to normal.
    Normal behavior is for the barbs and lines to point right (comes from wind
    barbs having these features point towards low pressure in the Northern
    Hemisphere).

    A single value is applied to all barbs. Individual barbs can be flipped by
    passing a bool array of the same size as *U* and *V*.

Returns
-------
barbs : `~matplotlib.quiver.Barbs`

Other Parameters
----------------
data : indexable object, optional
    DATA_PARAMETER_PLACEHOLDER

**kwargs
    The barbs can further be customized using `.PolyCollection` keyword
    arguments:

    %(PolyCollection:kwdoc)s
""" % _docstring.interpd.params

_docstring.interpd.register(barbs_doc=_barbs_doc)


class Barbs(mcollections.PolyCollection):
    """
    Specialized PolyCollection for barbs.

    The only API method is :meth:`set_UVC`, which can be used to
    change the size, orientation, and color of the arrows.  Locations
    are changed using the :meth:`set_offsets` collection method.
    Possibly this method will be useful in animations.

    There is one internal function :meth:`!_find_tails` which finds
    exactly what should be put on the barb given the vector magnitude.
    From there :meth:`!_make_barbs` is used to find the vertices of the
    polygon to represent the barb based on this information.
    """
    # pltrs makes the barbs for the Axes *ax* when the object is made,
    # which `~.Axes.barbs` adds to them.

    @_docstring.interpd
    def __init__(self, ax, *args,
                 pivot='tip', length=7, barbcolor=None, flagcolor=None,
                 sizes=None, fill_empty=False, barb_increments=None,
                 rounding=True, flip_barb=False, **kwargs):
        """
        The constructor takes one required argument, an Axes
        instance, followed by the args and kwargs described
        by the following pyplot interface documentation:
        %(barbs_doc)s
        """
        self.sizes = sizes or dict()
        self.fill_empty = fill_empty
        self.barb_increments = barb_increments or dict()
        self.rounding = rounding
        self.flip = np.atleast_1d(flip_barb)
        transform = kwargs.pop('transform', None)
        self._pivot = pivot
        self._length = length
        cmap, norm, clim = (kwargs.pop(key, None) for key in ('cmap', 'norm', 'clim'))

        # Parse out the data arrays from the various configurations supported
        x, y, u, v, c = _parse_args(*args, caller_name='barbs')
        self.x = x
        self.y = y
        x, y, u, v, c, flip = self._unmasked(u, v, c)
        made, mapped = _mapped(c, cmap, norm, clim)
        if not all(c is None or _single_color(c) for c in (barbcolor, flagcolor)):
            # Colours per barb are the collection's, as matplotlib makes
            # *barbcolor* the edges and *flagcolor* the faces.
            if None in (barbcolor, flagcolor):
                kwargs['edgecolors'] = 'face'
                kwargs['facecolors'] = flagcolor if flagcolor is not None else barbcolor
            else:
                kwargs['edgecolors'] = barbcolor
                kwargs['facecolors'] = flagcolor
            barbcolor = flagcolor = None
        native = _plotting.barbs(
            ax._native, (np.asarray(x, float), np.asarray(y, float)),
            (_filled(u), _filled(v)), mapped, (pivot, float(length)),
            (_rgba(barbcolor), _rgba(flagcolor)), self._native_shape(flip),
            _live_transforms.given_to_native(transform, ax))
        _handles.adopt(self, native, ax.figure)
        self._offsets_dtype = np.result_type(x, y)
        self._attach_native_mapping(colorizer=made)
        # matplotlib's default width of the barbs.
        _values.remember(self, "linewidth", np.atleast_1d(1))
        self._internal_update(kwargs)
        # On no figure until added, as the arrows of a `Quiver`.
        self._decouple(removed=False)

    def _unmasked(self, U, V, C):
        """
        Keep the components *U*, *V* (copies, masked where invalid, as
        matplotlib's ``u`` and ``v``) and return the positions, components,
        colour values *C* and flip flags of the vectors left once the
        masked ones are dropped.
        """
        # We need to ensure we have a copy, not a reference to an array that
        # might change before draw().
        self.u = ma.masked_invalid(U, copy=True).ravel()
        self.v = ma.masked_invalid(V, copy=True).ravel()

        # Flip needs to have the same number of entries as everything else.
        # Use broadcast_to to avoid a bloated array of identical values.
        # (can't rely on actual broadcasting)
        if len(self.flip) == 1:
            flip = np.broadcast_to(self.flip, self.u.shape)
        else:
            flip = self.flip

        if C is not None:
            c = ma.masked_invalid(C, copy=True).ravel()
            x, y, u, v, c, flip = cbook.delete_masked_points(
                self.x.ravel(), self.y.ravel(), self.u, self.v, c,
                flip.ravel())
            _check_consistent_shapes(x, y, u, v, c, flip)
        else:
            c = None
            x, y, u, v, flip = cbook.delete_masked_points(
                self.x.ravel(), self.y.ravel(), self.u, self.v, flip.ravel())
            _check_consistent_shapes(x, y, u, v, flip)
        return x, y, u, v, c, flip

    def _native_shape(self, flip):
        """The sizes, increments and flags of the barbs, with the flip
        flags *flip*, as pltrs (Rust) takes them."""
        sizes = {'spacing': 0.125, 'height': 0.4, 'width': 0.25,
                 'emptybarb': 0.15, **self.sizes}
        increments = {'half': 5, 'full': 10, 'flag': 50, **self.barb_increments}
        return ((float(sizes['spacing']), float(sizes['height']),
                 float(sizes['width']), float(sizes['emptybarb'])),
                bool(self.fill_empty),
                (float(increments['half']), float(increments['full']),
                 float(increments['flag'])),
                bool(self.rounding), [bool(f) for f in np.ravel(flip)] or [False])

    def set_UVC(self, U, V, C=None):
        x, y, u, v, c, flip = self._unmasked(U, V, C)
        _plotting.barbs_set_uvc(
            self._native, (np.asarray(x, float), np.asarray(y, float)),
            (_filled(u), _filled(v)), None,
            ((self._pivot, float(self._length)), (None, None),
             self._native_shape(flip)))
        # matplotlib keeps the offsets of the barbs drawn as they are.
        self._offsets_dtype = np.result_type(x, y)
        # Set the color array
        if C is not None:
            self.set_array(c)
        self.stale = True

    def set_offsets(self, xy):
        """
        Set the offsets for the barb polygons.  This saves the offsets passed
        in and masks them as appropriate for the existing U/V data.

        Parameters
        ----------
        xy : sequence of pairs of floats
        """
        self.x = xy[:, 0]
        self.y = xy[:, 1]
        x, y, u, v = cbook.delete_masked_points(
            self.x.ravel(), self.y.ravel(), self.u, self.v)
        _check_consistent_shapes(x, y, u, v)
        xy = np.column_stack((x, y))
        super().set_offsets(xy)
        self.stale = True


# What matplotlib's module also has from its imports, which programs
# import from it.
import math  # noqa: E402, F401
from . import text as mtext  # noqa: E402, F401
from .patches import CirclePolygon  # noqa: E402, F401
