"""
Axes inside an Axes (matplotlib's ``Axes.inset_axes``,
``secondary_xaxis``, ``secondary_yaxis`` and ``child_axes``) and the
indicators of the area an inset shows (``indicate_inset``,
``indicate_inset_zoom``), as a mixin the `~.axes.Axes` class includes:
pltrs (Rust) makes the child axes and places them anew at every layout
pass (``_native.insets``).
"""

from numbers import Real

from .. import _docstring
from .. import _handles, _live_transforms
from .. import patches  # noqa: F401  (whose Rectangle the docstrings list)
from .._native import insets as _insets
from ..transforms import Bbox, TransformedBbox


_secax_docstring = '''
Warnings
--------
This method is experimental as of 3.1, and the API may change.

Parameters
----------
location : {'top', 'bottom', 'left', 'right'} or float
    The position to put the secondary axis.  Strings can be 'top' or
    'bottom' for orientation='x' and 'right' or 'left' for
    orientation='y'. A float indicates the relative position on the
    parent Axes to put the new Axes, 0.0 being the bottom (or left)
    and 1.0 being the top (or right).

functions : 2-tuple of func, or Transform with an inverse

    If a 2-tuple of functions, the user specifies the transform
    function and its inverse.  i.e.
    ``functions=(lambda x: 2 / x, lambda x: 2 / x)`` would be an
    reciprocal transform with a factor of 2. Both functions must accept
    numpy arrays as input.

    The user can also directly supply a subclass of
    `.transforms.Transform` so long as it has an inverse.

    See :doc:`/gallery/subplots_axes_and_figures/secondary_axis`
    for examples of making these conversions.

transform : `.Transform`, optional
    If specified, *location* will be
    placed relative to this transform (in the direction of the axis)
    rather than the parent's axis. i.e. a secondary x-axis will
    use the provided y transform and the x transform of the parent.

    .. versionadded:: 3.9

Returns
-------
ax : axes._secondary_axes.SecondaryAxis
    The returned Axes is overlaid on top of the original Axes and all
    components except for the complementary axis are hidden. You may modify
    the complementary axis, e.g. by setting ticks or an axis label. However,
    it is not designed to hold data, so that you should not call any
    plotting methods on it. Its limits are derived from the parent Axes via
    the specified transformation, and setting limits on the secondary axis
    (e.g., via ``set_xlim`` or ``set_ylim``) has no effect.


Other Parameters
----------------
**kwargs : `~matplotlib.axes.Axes` properties.
    Other miscellaneous Axes parameters.
'''

_docstring.interpd.register(_secax_docstring=_secax_docstring)


class _TransformedBoundsLocator:
    """
    Axes locator for `.Axes.inset_axes` and similarly positioned Axes.

    The locator is a callable object used in `.Axes.set_aspect` to compute
    the Axes location depending on the renderer.
    """

    def __init__(self, bounds, transform):
        """
        *bounds* (a ``[l, b, w, h]`` rectangle) and *transform* together
        specify the position of the inset Axes.
        """
        self._bounds = bounds
        self._transform = transform
        self._parent = None

    def __call__(self, ax, renderer):
        # Subtracting transSubfigure will typically rely on inverted(),
        # freezing the transform; thus, this needs to be delayed until draw
        # time as transSubfigure may otherwise change after this is
        # evaluated.
        return TransformedBbox(
            Bbox.from_bounds(*self._bounds),
            self._transform - ax.get_figure(root=False).transSubfigure)

    def _place(self, ax):
        """Make pltrs (Rust) place *ax* on the bounds at every layout."""
        parent = ax if self._parent is None else self._parent
        _insets.place_inset(parent._native, ax._native,
                            tuple(map(float, self._bounds)),
                            _live_transforms.to_native(self._transform))


# matplotlib defines the class in ``axes._base``, which imports it from here.
_TransformedBoundsLocator.__module__ = __name__.rpartition(".")[0] + "._base"


class InsetsMixin:
    """The child Axes of an Axes: insets and secondary axes."""

    @property
    def child_axes(self):
        """The Axes placed inside this one: insets and secondary axes."""
        from ._axes import Axes
        return _handles.wrap_all(_insets.child_axes(self._native), Axes,
                                 self.figure)

    def add_child_axes(self, ax):
        """
        Add an `.Axes` to the Axes' children; return the child Axes.

        This is the lowlevel version.  See `.axes.Axes.inset_axes`.
        """
        if ax.__dict__.get("_child_of") is not self:
            # matplotlib adds a child twice over; pltrs (Rust) keeps one.
            _insets.add_child_axes(self._native, ax._native)
            ax._child_of = self
        self.stale = True
        return ax

    def inset_axes(self, bounds, *, transform=None, zorder=5, **kwargs):
        """
        Add a child inset Axes to this existing Axes.


        Parameters
        ----------
        bounds : [x0, y0, width, height]
            Lower-left corner of inset Axes, and its width and height.

        transform : `.Transform`
            Defaults to `!ax.transAxes`, i.e. the units of *rect* are in
            Axes-relative coordinates.

        projection : {None, 'aitoff', 'hammer', 'lambert', 'mollweide', \
'polar', 'rectilinear', str}, optional
            The projection type of the inset `~.axes.Axes`. *str* is the name
            of a custom projection, see `~matplotlib.projections`. The default
            None results in a 'rectilinear' projection.

        polar : bool, default: False
            If True, equivalent to projection='polar'.

        axes_class : subclass type of `~.axes.Axes`, optional
            The `.axes.Axes` subclass that is instantiated.  This parameter
            is incompatible with *projection* and *polar*.  See
            :ref:`axisartist_users-guide-index` for examples.

        zorder : number
            Defaults to 5 (same as `.Axes.legend`).  Adjust higher or lower
            to change whether it is above or below data plotted on the
            parent Axes.

        **kwargs
            Other keyword arguments are passed on to the inset Axes class.

        Returns
        -------
        ax
            The created `~.axes.Axes` instance.

        Examples
        --------
        This example makes two inset Axes, the first is in Axes-relative
        coordinates, and the second in data-coordinates::

            fig, ax = plt.subplots()
            ax.plot(range(10))
            axin1 = ax.inset_axes([0.8, 0.1, 0.15, 0.15])
            axin2 = ax.inset_axes(
                    [5, 7, 2.3, 2.3], transform=ax.transData)

        """
        kwargs.setdefault('label', 'inset_axes')
        fig = self.get_figure(root=False)
        projection_class, pkw = fig._process_projection_requirements(**kwargs)
        from ..projections import _program
        program = _program.is_program_class(projection_class)
        name = _program.native_name(projection_class)
        bounds = tuple(map(float, bounds))
        native = _insets.inset_axes(
            self._native, bounds,
            None if name == "rectilinear" or program else name)
        inset_ax = _handles.wrap(native, projection_class, self.figure)
        inset_ax._child_of = self
        inset_ax._init_projection()
        locator = _TransformedBoundsLocator(
            bounds, self.transAxes if transform is None else transform)
        locator._parent = self
        if transform is None:
            # pltrs (Rust) places insets in their parent's fractions itself.
            inset_ax._axes_locator = locator
        else:
            inset_ax.set_axes_locator(locator)
        inset_ax.set_zorder(zorder)
        for name in ("x", "y"):
            other = pkw.pop(f"share{name}", None)
            if other is not None:
                getattr(inset_ax, f"share{name}")(other)
        inset_ax._internal_update(pkw)
        return inset_ax

    @_docstring.interpd
    def indicate_inset(self, bounds=None, inset_ax=None, *, transform=None,
                       facecolor='none', edgecolor='0.5', alpha=0.5,
                       zorder=None, **kwargs):
        """
        Add an inset indicator to the Axes.  This is a rectangle on the plot
        at the position indicated by *bounds* that optionally has lines that
        connect the rectangle to an inset Axes (`.Axes.inset_axes`).

        Warnings
        --------
        This method is experimental as of 3.0, and the API may change.

        Parameters
        ----------
        bounds : [x0, y0, width, height], optional
            Lower-left corner of rectangle to be marked, and its width
            and height.  If not set, the bounds will be calculated from the
            data limits of *inset_ax*, which must be supplied.

        inset_ax : `.Axes`, optional
            An optional inset Axes to draw connecting lines to.  Two lines are
            drawn connecting the indicator box to the inset Axes on corners
            chosen so as to not overlap with the indicator box.

        transform : `.Transform`
            Transform for the rectangle coordinates. Defaults to
            ``ax.transData``, i.e. the units of *rect* are in the Axes' data
            coordinates.

        facecolor : :mpltype:`color`, default: 'none'
            Facecolor of the rectangle.

        edgecolor : :mpltype:`color`, default: '0.5'
            Color of the rectangle and color of the connecting lines.

        alpha : float or None, default: 0.5
            Transparency of the rectangle and connector lines.  If not
            ``None``, this overrides any alpha value included in the
            *facecolor* and *edgecolor* parameters.

        zorder : float, default: 4.99
            Drawing order of the rectangle and connector lines.  The default,
            4.99, is just below the default level of inset Axes.

        **kwargs
            Other keyword arguments are passed on to the `.Rectangle` patch:

            %(Rectangle:kwdoc)s

        Returns
        -------
        inset_indicator : `.inset.InsetIndicator`
            An artist which contains

            inset_indicator.rectangle : `.Rectangle`
                The indicator frame.

            inset_indicator.connectors : 4-tuple of `.patches.ConnectionPatch`
                The four connector lines connecting to (lower_left, upper_left,
                lower_right upper_right) corners of *inset_ax*. Two lines are
                set with visibility to *False*,  but the user can set the
                visibility to True if the automatic choice is not deemed correct.

            .. versionchanged:: 3.10
                Previously the rectangle and connectors tuple were returned.
        """
        from ..inset import InsetIndicator
        kwargs.setdefault('label', '_indicate_inset')
        if transform is not None:
            kwargs["transform"] = transform
        indicator_patch = InsetIndicator(
            bounds, inset_ax=inset_ax,
            facecolor=facecolor, edgecolor=edgecolor, alpha=alpha,
            zorder=zorder, **kwargs)
        self.add_artist(indicator_patch)
        return indicator_patch

    def indicate_inset_zoom(self, inset_ax, **kwargs):
        """
        Add an inset indicator rectangle to the Axes based on the axis
        limits for an *inset_ax* and draw connectors between *inset_ax*
        and the rectangle.

        Warnings
        --------
        This method is experimental as of 3.0, and the API may change.

        Parameters
        ----------
        inset_ax : `.Axes`
            Inset Axes to draw connecting lines to.  Two lines are
            drawn connecting the indicator box to the inset Axes on corners
            chosen so as to not overlap with the indicator box.

        **kwargs
            Other keyword arguments are passed on to `.Axes.indicate_inset`

        Returns
        -------
        inset_indicator : `.inset.InsetIndicator`
            An artist which contains

            inset_indicator.rectangle : `.Rectangle`
                The indicator frame.

            inset_indicator.connectors : 4-tuple of `.patches.ConnectionPatch`
                The four connector lines connecting to (lower_left, upper_left,
                lower_right upper_right) corners of *inset_ax*. Two lines are
                set with visibility to *False*,  but the user can set the
                visibility to True if the automatic choice is not deemed correct.

            .. versionchanged:: 3.10
                Previously the rectangle and connectors tuple were returned.
        """
        return self.indicate_inset(None, inset_ax, **kwargs)

    @_docstring.interpd
    def secondary_xaxis(self, location, functions=None, *, transform=None,
                        **kwargs):
        """
        Add a second x-axis to this `~.axes.Axes`.

        This axis is typically used to display a second x-scale for the data
        plotted on the Axes.

        %(_secax_docstring)s

        Examples
        --------
        The main axis shows frequency, and the secondary axis shows period.

        .. plot::

            fig, ax = plt.subplots()
            ax.loglog(range(1, 360, 5), range(1, 360, 5))
            ax.set_xlabel('frequency [Hz]')

            def invert(x):
                # 1/x with special treatment of x == 0
                x = np.array(x).astype(float)
                near_zero = np.isclose(x, 0)
                x[near_zero] = np.inf
                x[~near_zero] = 1 / x[~near_zero]
                return x

            # the inverse of 1/x is itself
            secax = ax.secondary_xaxis('top', functions=(invert, invert))
            secax.set_xlabel('Period [s]')
            plt.show()

        To add a secondary axis relative to your data, you can pass a transform
        to the new axis.

        .. plot::

            fig, ax = plt.subplots()
            ax.plot(range(0, 5), range(-1, 4))

            # Pass 'ax.transData' as a transform to place the axis
            # relative to your data at y=0
            secax = ax.secondary_xaxis(0, transform=ax.transData)
        """
        if not (location in ['top', 'bottom'] or isinstance(location, Real)):
            raise ValueError('secondary_xaxis location must be either '
                             'a float or "top"/"bottom"')
        from ._secondary_axes import SecondaryAxis
        return SecondaryAxis(self, 'x', location, functions, transform,
                             **kwargs)

    @_docstring.interpd
    def secondary_yaxis(self, location, functions=None, *, transform=None,
                        **kwargs):
        """
        Add a second y-axis to this `~.axes.Axes`.

        This axis is typically used to display a second y-scale for the data
        plotted on the Axes.

        %(_secax_docstring)s

        Examples
        --------
        Add a secondary Axes that converts from radians to degrees

        .. plot::

            fig, ax = plt.subplots()
            ax.plot(range(1, 360, 5), range(1, 360, 5))
            ax.set_ylabel('degrees')
            secax = ax.secondary_yaxis('right', functions=(np.deg2rad,
                                                           np.rad2deg))
            secax.set_ylabel('radians')

        To add a secondary axis relative to your data, you can pass a transform
        to the new axis.

        .. plot::

            fig, ax = plt.subplots()
            ax.plot(range(0, 5), range(-1, 4))

            # Pass 'ax.transData' as a transform to place the axis
            # relative to your data at x=3
            secax = ax.secondary_yaxis(3, transform=ax.transData)
        """
        if not (location in ['left', 'right'] or isinstance(location, Real)):
            raise ValueError('secondary_yaxis location must be either '
                             'a float or "left"/"right"')
        from ._secondary_axes import SecondaryAxis
        return SecondaryAxis(self, 'y', location, functions, transform,
                             **kwargs)
