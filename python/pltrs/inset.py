"""
The inset module defines the InsetIndicator class, which draws the rectangle and
connectors required for `.Axes.indicate_inset` and `.Axes.indicate_inset_zoom`.

matplotlib's ``inset`` (from matplotlib 3.11.2, under the Matplotlib
License): pltrs (Rust) draws the indicator (``_native.insets``), following
the view limits of the inset Axes it indicates and choosing the connectors
that do not cross the rectangle at every draw.
"""

from . import _api, _handles, artist
from ._native import insets as _insets, patches as _patches
from .patches import Rectangle


class InsetIndicator(artist.Artist):
    """
    An artist to highlight an area of interest.

    An inset indicator is a rectangle on the plot at the position indicated by
    *bounds* that optionally has lines that connect the rectangle to an inset
    Axes (`.Axes.inset_axes`).

    .. versionadded:: 3.10
    """
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
        self._bounds = tuple(map(float, bounds))
        x, y, width, height = bounds
        self._transform_given = kwargs.pop("transform", None)
        self._props = kwargs
        self._rectangle = Rectangle((x, y), width, height, clip_on=False,
                                    **kwargs)
        super().__init__()
        self._zorder_given = zorder

    def _add_to_axes(self, axes, kind, autolim=True):
        props = {key: value for key, value in self._props.items()
                 if key != "label"}
        native = _insets.indicate_inset(
            axes._native, None if self._auto_update_bounds else self._bounds,
            None if self._inset_ax is None else self._inset_ax._native,
            {"label": self._props.get("label", "_indicate_inset"), **props})
        _handles.adopt(self, native, axes.figure)
        transform = self._transform_given
        if transform is not None:
            if hasattr(transform, "_as_mpl_transform"):
                transform = transform._as_mpl_transform(axes)
            self.set_transform(transform)
            self._rectangle.set_transform(transform)
        if self._zorder_given is not None:
            self.set_zorder(self._zorder_given)
        return self

    def _shared_setter(self, prop, val):
        """
        Helper function to set the same style property on the artist and its children.
        """
        if self._figure is not None:
            _patches.update(self._native, {prop: val})
        self._props[prop] = val
        getattr(self._rectangle, f"set_{prop}")(val)

    def set_alpha(self, alpha):
        # docstring inherited
        self._shared_setter('alpha', alpha)

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
        """
        self._shared_setter('linestyle', ls)

    def _bounds_from_inset_ax(self):
        xlim = self._inset_ax.get_xlim()
        ylim = self._inset_ax.get_ylim()
        return (xlim[0], ylim[0], xlim[1] - xlim[0], ylim[1] - ylim[0])

    @property
    def rectangle(self):
        """`.Rectangle`: the indicator frame."""
        if self._auto_update_bounds:
            self._rectangle.set_bounds(self._bounds_from_inset_ax())
        return self._rectangle

    @property
    def connectors(self):
        """
        4-tuple of `.patches.ConnectionPatch` or None
            The four connector lines connecting to (lower_left, upper_left,
            lower_right upper_right) corners of *inset_ax*. Two lines are
            set with visibility to *False*,  but the user can set the
            visibility to True if the automatic choice is not deemed correct.
        """
        if self._inset_ax is None:
            return
        raise NotImplementedError(
            "pltrs: InsetIndicator.connectors (pltrs draws the connectors "
            "with the rectangle)")

    @_api.deprecated(
        '3.10',
        message=('Since Matplotlib 3.10 indicate_inset_[zoom] returns a single '
                 'InsetIndicator artist with a rectangle property and a connectors '
                 'property.  From 3.12 it will no longer be possible to unpack the '
                 'return value into two elements.'))
    def __getitem__(self, key):
        return [self._rectangle, self.connectors][key]
