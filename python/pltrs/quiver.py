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

from . import (_api, _artist_clipping, _handles, _live_transforms, _values, cbook,
               collections as mcollections,
               colors as mcolors, transforms)
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
            Coordinate system and units for *X*, *Y*.
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
        _api.check_in_list(['axes', 'figure', 'data', 'inches'],
                           coordinates=coordinates)
        _api.check_in_list(['N', 'S', 'E', 'W'], labelpos=labelpos)
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
        native = _plotting.quiverkey(
            Q.axes._native, Q._native, (float(X), float(Y)), float(U), str(label),
            (float(angle), coordinates, _rgba(color),
             None if zorder is None else float(zorder)),
            (float(labelsep), labelpos, _rgba(labelcolor), font),
            mcollections._native_kwargs(
                cbook.normalize_kwargs(kwargs, mcollections.PolyCollection)))
        _handles.adopt(self, native, Q.figure)
        # matplotlib's ``quiverkey`` adds the key with ``add_artist``.
        _artist_clipping.take_parent_defaults(self, Q.axes)
        from .text import Text
        self.text = _handles.wrap(_plotting.quiverkey_text(native), Text,
                                  self.figure)

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
        The key's arrow, as the `.PolyCollection` pltrs (Rust) draws it in
        the figure's current layout (a copy: pltrs sizes the arrow anew at
        every draw).
        """
        from .path import Path
        paths = [Path(vertices, codes)
                 for vertices, codes in _plotting.quiverkey_paths(self._native)]
        return mcollections.PolyCollection([path.vertices for path in paths])

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


class Quiver(mcollections.PolyCollection):
    """
    Specialized PolyCollection for arrows.

    Use set_UVC to change the size, orientation, and color of the
    arrows; their locations can be set using set_offsets().

    pltrs makes the arrows on the Axes *ax* when the object is made (as
    `~.Axes.quiver` adds it), and sizes them from the Axes whenever the
    figure is drawn.
    """

    _PIVOT_VALS = ('tail', 'middle', 'tip')

    def __init__(self, ax, *args,
                 scale=None, headwidth=3, headlength=5, headaxislength=4.5,
                 minshaft=1, minlength=1, units='width', scale_units=None,
                 angles='uv', width=None, color='k', pivot='tail', **kwargs):
        """
        The constructor takes one required argument, an Axes
        instance, followed by the args and kwargs described
        by the pyplot interface documentation of `~.Axes.quiver`.
        """
        X, Y, U, V, C = _parse_args(*args, caller_name='quiver')
        self.X = X
        self.Y = Y
        self.XY = np.column_stack((X, Y))
        self.N = len(X)
        self.scale = scale
        self.headwidth = headwidth
        self.headlength = float(headlength)
        self.headaxislength = headaxislength
        self.minshaft = minshaft
        self.minlength = minlength
        self.units = units
        self.scale_units = scale_units
        self.angles = angles
        self.width = width

        if pivot.lower() == 'mid':
            pivot = 'middle'
        self.pivot = pivot.lower()
        _api.check_in_list(self._PIVOT_VALS, pivot=self.pivot)

        transform = kwargs.pop('transform', None)
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
            _live_transforms.given_to_native(transform))
        _handles.adopt(self, native, ax.figure)
        self._attach_native_mapping(colorizer=made)
        if not _single_color(facecolor):
            self.set_facecolor(facecolor)
        # The arrows are unstroked, as matplotlib's default ``(0,)`` says.
        _values.remember(self, "linewidth", np.atleast_1d((0,)))
        self._internal_update(kwargs)
        self.polykw = kwargs
        self._store_uvc(U, V, C)

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
        self.Umask = ma.getmask(U)
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
        """Set the components *U*, *V* of the arrows, and their colour
        values *C*."""
        U, V, C = self._checked_uvc(U, V, C)
        _plotting.quiver_set_uvc(self._native, _filled(U), _filled(V))
        if C is not None:
            self.set_array(C)
        self._store_uvc(U, V, C)


class Barbs(mcollections.PolyCollection):
    """
    Specialized PolyCollection for barbs.

    The only API method is :meth:`set_UVC`, which can be used to
    change the size, orientation, and color of the arrows.  Locations
    are changed using the :meth:`set_offsets` collection method.
    Possibly this method will be useful in animations.

    There is one internal function :meth:`_find_tails` which finds
    exactly what should be put on the barb given the vector magnitude.
    From there :meth:`_make_barbs` is used to find the vertices of the
    polygon to represent the barb based on this information.

    pltrs makes the barbs on the Axes *ax* when the object is made (as
    `~.Axes.barbs` adds it).
    """

    def __init__(self, ax, *args,
                 pivot='tip', length=7, barbcolor=None, flagcolor=None,
                 sizes=None, fill_empty=False, barb_increments=None,
                 rounding=True, flip_barb=False, **kwargs):
        """
        The constructor takes one required argument, an Axes
        instance, followed by the args and kwargs described
        by the pyplot interface documentation of `~.Axes.barbs`.
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
            _live_transforms.given_to_native(transform))
        _handles.adopt(self, native, ax.figure)
        self._attach_native_mapping(colorizer=made)
        # matplotlib's default width of the barbs.
        _values.remember(self, "linewidth", np.atleast_1d(1))
        self._internal_update(kwargs)

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
        """
        Set the components *U*, *V* of the barbs, and their colour values
        *C*; the barbs at masked vectors are left out.
        """
        x, y, u, v, c, flip = self._unmasked(U, V, C)
        _plotting.barbs_set_uvc(
            self._native, (np.asarray(x, float), np.asarray(y, float)),
            (_filled(u), _filled(v)), None,
            ((self._pivot, float(self._length)), (None, None),
             self._native_shape(flip)))
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
