"""
What an Axes draws around its data (matplotlib's ``set_axis_off``,
``axis()``, ``set_frame_on``), its background (``Axes.patch``) and the
blended transforms of its axes (``get_xaxis_transform``): pltrs (Rust)
keeps them (``_native.axes_base``).
"""

from .. import _api
from .._background_patch import BackgroundPatch
from .._native import axes_base as _axes_base


class AxesPatch(BackgroundPatch):
    """
    The background of an Axes (matplotlib's ``Axes.patch``, a unit
    `.Rectangle` in axes coordinates): pltrs (Rust) draws it as part of the
    axes, so this object reads and sets the axes' background properties.
    """

    def __init__(self, axes):
        super().__init__()
        self.axes = axes

    def get_figure(self, root=False):
        """Return the figure of the Axes."""
        return self.axes.get_figure(root=root)

    def get_transform(self):
        """Return the transform of the unit square: ``axes.transAxes``."""
        return self.axes.transAxes

    def get_visible(self):
        """Return whether the background is drawn."""
        return _axes_base.get_patch_visible(self.axes._native)

    def set_visible(self, b):
        """Set whether the background is drawn."""
        _axes_base.set_patch_visible(self.axes._native, bool(b))

    def get_facecolor(self):
        """Return the face color of the background."""
        return self.axes.get_facecolor()

    def _set_facecolor(self, color):
        self.axes.set_facecolor(color)

    def get_edgecolor(self):
        """Return the edge color of the background."""
        return _axes_base.get_patch_edgecolor(self.axes._native)

    def _set_edgecolor(self, color):
        _axes_base.set_patch_edgecolor(self.axes._native, color)

    def get_linewidth(self):
        """Return the line width of the background's edge in points."""
        return _axes_base.get_patch_linewidth(self.axes._native)

    def set_linewidth(self, w):
        """Set the line width of the background's edge in points."""
        _axes_base.set_patch_linewidth(self.axes._native, float(w))

    def get_zorder(self):
        """Return the z-order of the background."""
        return _axes_base.get_patch_zorder(self.axes._native)

    def set_zorder(self, level):
        """Set the z-order of the background."""
        _axes_base.set_patch_zorder(self.axes._native, float(level))

    zorder = property(get_zorder, set_zorder)

    def _get_gid(self):
        return _axes_base.get_patch_gid(self.axes._native)

    def _set_gid(self, gid):
        _axes_base.set_patch_gid(self.axes._native, gid)

    def contains_point(self, point, radius=None):
        """Return whether the display point *point* is in the background."""
        return self.axes.contains_point(point)

    def contains(self, mouseevent):
        """Return whether the mouse event occurred in the background."""
        return self.axes.contains(mouseevent)

    def _clip_spec(self):
        """The native clip path of artists clipped to the background: the
        outline of the axes' patch, followed as the axes changes."""
        return ("artist", self.axes._native)


class DecorationsMixin:
    """The decoration methods of `~.axes.Axes`."""

    @property
    def patch(self):
        """The background of the Axes (an `AxesPatch`)."""
        patch = self.__dict__.get("_patch")
        if patch is None:
            patch = self.__dict__["_patch"] = AxesPatch(self)
        return patch

    @property
    def axison(self):
        """Whether the patch, spines and axes are drawn."""
        return _axes_base.get_axis_on(self._native)

    def set_axis_off(self):
        """
        Hide all visual components of the x- and y-axis.

        This sets a flag to suppress drawing of all axis decorations, i.e.
        axis labels, axis spines, and the axis tick component (tick markers,
        tick labels, and grid lines). Individual visibility settings of
        these components are ignored as long as `set_axis_off()` is in
        effect.
        """
        _axes_base.set_axis_on(self._native, False)

    def set_axis_on(self):
        """
        Do not hide all visual components of the x- and y-axis.

        This reverts the effect of a prior `.set_axis_off()` call. Whether the
        individual axis decorations are drawn is controlled by their
        respective visibility settings.

        This is on by default.
        """
        _axes_base.set_axis_on(self._native, True)

    def set_rasterization_zorder(self, z):
        """
        Set the zorder threshold for rasterization for vector graphics output.

        All artists with a zorder below the given value will be rasterized if
        they support rasterization.

        This setting is ignored for pixel-based output.

        Parameters
        ----------
        z : float or None
            The zorder below which artists are rasterized.
            If ``None`` rasterization based on zorder is deactivated.
        """
        _axes_base.set_rasterization_zorder(
            self._native, None if z is None else float(z))

    def get_rasterization_zorder(self):
        """Return the zorder value below which artists will be rasterized."""
        return _axes_base.get_rasterization_zorder(self._native)

    def get_frame_on(self):
        """Get whether the Axes rectangle patch is drawn."""
        return _axes_base.get_frame_on(self._native)

    def set_frame_on(self, b):
        """
        Set whether the Axes rectangle patch is drawn.

        Parameters
        ----------
        b : bool
        """
        _axes_base.set_frame_on(self._native, bool(b))

    def axis(self, arg=None, /, *, emit=True, **kwargs):
        """
        Convenience method to get or set some axis properties.

        Call signatures::

          xmin, xmax, ymin, ymax = axis()
          xmin, xmax, ymin, ymax = axis([xmin, xmax, ymin, ymax])
          xmin, xmax, ymin, ymax = axis(option)
          xmin, xmax, ymin, ymax = axis(**kwargs)

        Parameters
        ----------
        xmin, xmax, ymin, ymax : float, optional
            The axis limits to be set.  This can also be achieved using ::

                ax.set(xlim=(xmin, xmax), ylim=(ymin, ymax))

        option : bool or str
            If a bool, turns axis lines and labels on or off. If a string,
            possible values are 'off', 'on', 'equal', 'scaled', 'tight',
            'auto', 'image' and 'square'.

        emit : bool, default: True
            Whether observers are notified of the axis limit change.

        Returns
        -------
        xmin, xmax, ymin, ymax : float
            The axis limits.
        """
        if isinstance(arg, (str, bool)):
            if arg is True:
                arg = 'on'
            if arg is False:
                arg = 'off'
            _axes_base.axis_mode(self._native, arg.lower())
        else:
            if arg is not None:
                if len(arg) != 2*len(self._axis_names):
                    raise TypeError(
                        "The first argument to axis() must be an iterable of the form "
                        "[{}]".format(", ".join(
                            f"{name}min, {name}max" for name in self._axis_names)))
                limits = {
                    name: arg[2*i:2*(i+1)]
                    for i, name in enumerate(self._axis_names)
                }
            else:
                limits = {}
                for name in self._axis_names:
                    ax_min = kwargs.pop(f'{name}min', None)
                    ax_max = kwargs.pop(f'{name}max', None)
                    limits[name] = (ax_min, ax_max)
            for name, (ax_min, ax_max) in limits.items():
                if ax_min is not None or ax_max is not None:
                    getattr(self, f'set_{name}lim')(ax_min, ax_max)
        if kwargs:
            raise _api.kwarg_error("axis", kwargs)
        lims = ()
        for name in self._axis_names:
            lims += tuple(getattr(self, f'get_{name}lim')())
        return lims

    def _set_position(self, pos, which='both'):
        """
        Set the position as `set_position` does, for the navigation
        toolbar restoring a view (matplotlib's private version, which keeps
        the Axes in its layout).
        """
        self.set_position(pos, which)

    def get_xaxis(self):
        """[*Discouraged*] Return the XAxis instance (``ax.xaxis``)."""
        return self.xaxis

    def get_yaxis(self):
        """[*Discouraged*] Return the YAxis instance (``ax.yaxis``)."""
        return self.yaxis
