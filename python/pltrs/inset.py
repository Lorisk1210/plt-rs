"""
The inset module defines the InsetIndicator class, which draws the rectangle and
connectors required for `.Axes.indicate_inset` and `.Axes.indicate_inset_zoom`.

matplotlib's ``inset`` (from matplotlib 3.11.2, under the Matplotlib
License): once an Axes adds it, pltrs (Rust) keeps the indicator with its
rectangle and its four connectors (``_native.insets``), places them as the
view limits of the inset Axes change, chooses which connectors are visible
when it is first drawn (or its connectors first asked for), and draws them
as matplotlib's `InsetIndicator.draw` does.
"""

from . import _api, _handles, _unusable, artist
from ._native import artist as _artist, insets as _insets
from .patches import ConnectionPatch, Rectangle


class InsetIndicator(artist.Artist):
    """
    An artist to highlight an area of interest.

    An inset indicator is a rectangle on the plot at the position indicated by
    *bounds* that optionally has lines that connect the rectangle to an inset
    Axes (`.Axes.inset_axes`).

    .. versionadded:: 3.10
    """

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    zorder = 4.99

    def __init__(self, bounds=None, inset_ax=None, zorder=None, **kwargs):
        """
        Parameters
        ----------
        bounds : [x0, y0, width, height], optional
            Lower-left corner of rectangle to be marked, and its width
            and height.  If not set, the bounds will be calculated from the
            data limits of inset_ax, which must be supplied.

        inset_ax : `~.axes.Axes`, optional
            An optional inset Axes to draw connecting lines to.  Two lines are
            drawn connecting the indicator box to the inset Axes on corners
            chosen so as to not overlap with the indicator box.

        zorder : float, default: 4.99
            Drawing order of the rectangle and connector lines.  The default,
            4.99, is just below the default level of inset Axes.

        **kwargs
            Other keyword arguments are passed on to the `.Rectangle` patch.
        """
        if bounds is None and inset_ax is None:
            raise ValueError("At least one of bounds or inset_ax must be supplied")

        self._inset_ax = inset_ax
        self._auto_update_bounds = bounds is None
        if bounds is None:
            bounds = self._bounds_from_inset_ax()
        x, y, width, height = bounds
        # A transform needing the Axes (Cartopy's) resolves once one adds
        # the indicator.
        self._transform_given = kwargs.pop("transform", None)
        self._rectangle = Rectangle((x, y), width, height, clip_on=False,
                                    **kwargs)
        super().__init__()
        self.set_zorder(zorder)
        # Initial style properties for the artist should match the
        # rectangle: its alpha, and the line style the connectors are made
        # with (pltrs (Rust) gives them the rest).
        artist.Artist.set_alpha(self, self._rectangle.get_alpha())
        self._linestyle = self._rectangle.get_linestyle()

    def _add_to_axes(self, axes, kind, autolim=True):
        transform = self._transform_given
        if transform is not None:
            if hasattr(transform, "_as_mpl_transform"):
                transform = transform._as_mpl_transform(axes)
            self._rectangle.set_transform(transform)
        native = _insets.adopt_indicator(
            axes._native, self._native, self._rectangle._native,
            self._auto_update_bounds,
            None if self._inset_ax is None else self._inset_ax._native)
        _handles.adopt(self, native, axes.figure)
        _handles.adopt(self._rectangle, _insets.indicator_rectangle(native),
                       axes.figure)
        return self

    def _shared_setter(self, prop, val):
        """
        Helper function to set the same style property on the artist and its children.
        """
        if prop == "alpha":
            artist.Artist.set_alpha(self, val)
        elif prop == "linestyle":
            self._linestyle = val
        connectors = (() if self._figure is None else self._connector_objects(
            _insets.indicator_parts(self._native)))
        artist.setp([self.rectangle, *connectors], prop, val)

    def _connector_objects(self, natives):
        """The `.ConnectionPatch` objects of the connectors *natives*,
        made as matplotlib's `_update_connectors` makes them."""
        connectors = tuple(_handles.wrap(native, ConnectionPatch, self._figure)
                           for native in natives)
        for connector in connectors:
            if "_given_ends" not in connector.__dict__:
                # pltrs (Rust) places them, and gave them the rest of the
                # indicator's style.
                connector._given_ends = [
                    [self._inset_ax.transAxes, None],
                    [self.rectangle.get_data_transform(), None]]
                connector._linestyle = self._linestyle
        return connectors

    def set_alpha(self, alpha):
        # docstring inherited
        # matplotlib keeps the alpha before its rectangle refuses it: it
        # reads back, and the connectors matplotlib makes with it when
        # they are first asked for (or drawn) fail (see `._unusable`).
        self._given_alpha = alpha
        try:
            artist._check_alpha(alpha)
        except (TypeError, ValueError):
            if not self._connectors_made():
                _unusable.keep(self, "alpha", alpha, artist._check_alpha,
                               measured=False)
            raise
        _unusable.drop(self, "alpha")
        self._shared_setter('alpha', alpha)

    def get_alpha(self):
        # docstring inherited
        return self.__dict__.get("_given_alpha", super().get_alpha())

    def _connectors_made(self):
        """Whether matplotlib's indicator would have made its connectors:
        once they were asked for, or the indicator was drawn."""
        return (self.__dict__.get("_connectors_asked", False)
                or (self._figure is not None and _artist.draw_count(self._native) > 0))

    def set_edgecolor(self, color):
        """
        Set the edge color of the rectangle and the connectors.

        Parameters
        ----------
        color : :mpltype:`color` or None
        """
        self._shared_setter('edgecolor', color)

    def set_color(self, c):
        """
        Set the edgecolor of the rectangle and the connectors, and the
        facecolor for the rectangle.

        Parameters
        ----------
        c : :mpltype:`color`
        """
        self._shared_setter('edgecolor', c)
        self._shared_setter('facecolor', c)

    def set_linewidth(self, w):
        """
        Set the linewidth in points of the rectangle and the connectors.

        Parameters
        ----------
        w : float or None
        """
        self._shared_setter('linewidth', w)

    def set_linestyle(self, ls):
        """
        Set the linestyle of the rectangle and the connectors.

        Parameters
        ----------
        ls : {'-', '--', '-.', ':', '', ...} or (offset, on-off-seq)
            Possible values:

            - A string:

              =======================================================  ================
              linestyle                                                description
              =======================================================  ================
              ``'-'`` or ``'solid'``                                   solid line
              ``'--'`` or ``'dashed'``                                 dashed line
              ``'-.'`` or ``'dashdot'``                                dash-dotted line
              ``':'`` or ``'dotted'``                                  dotted line
              ``''`` or ``'none'`` (discouraged: ``'None'``, ``' '``)  draw nothing
              =======================================================  ================

            - A tuple describing the start position and lengths of dashes and spaces:

                  (offset, onoffseq)

              where

              - *offset* is a float specifying the offset (in points); i.e. how much
                is the dash pattern shifted.
              - *onoffseq* is a sequence of on and off ink in points. There can be
                arbitrary many pairs of on and off values.

              Example: The tuple ``(0, (10, 5, 1, 5))`` means that the pattern starts
              at the beginning of the line. It draws a 10 point long dash,
              then a 5 point long space, then a 1 point long dash, followed by a 5 point
              long space, and then the pattern repeats.

            For examples see :doc:`/gallery/lines_bars_and_markers/linestyles`.
        """
        self._shared_setter('linestyle', ls)

    def _bounds_from_inset_ax(self):
        xlim = self._inset_ax.get_xlim()
        ylim = self._inset_ax.get_ylim()
        return (xlim[0], ylim[0], xlim[1] - xlim[0], ylim[1] - ylim[0])

    @property
    def rectangle(self):
        """`.Rectangle`: the indicator frame."""
        if self._figure is None:
            return self._rectangle
        return _handles.wrap(_insets.indicator_rectangle(self._native),
                             Rectangle, self._figure)

    @property
    def connectors(self):
        """
        4-tuple of `.patches.ConnectionPatch` or None
            The four connector lines connecting to (lower_left, upper_left,
            lower_right upper_right) corners of *inset_ax*. Two lines are
            set with visibility to *False*,  but the user can set the
            visibility to True if the automatic choice is not deemed correct.
        """
        natives = _insets.indicator_connectors(self._native)
        if natives is None:
            return None
        _unusable.check(self)
        self._connectors_asked = True
        return self._connector_objects(natives)

    @_api.deprecated(
        '3.10',
        message=('Since Matplotlib 3.10 indicate_inset_[zoom] returns a single '
                 'InsetIndicator artist with a rectangle property and a connectors '
                 'property.  From 3.12 it will no longer be possible to unpack the '
                 'return value into two elements.'))
    def __getitem__(self, key):
        return [self.rectangle, self.connectors][key]
