"""
What an Axes draws around its data (matplotlib's ``set_axis_off``,
``axis()``, ``set_frame_on``), its background (``Axes.patch``), the
labels of its axes (``set_xlabel``) and the blended transforms of its axes
(``get_xaxis_transform``): pltrs (Rust) keeps them (``_native.axes_base``,
``_native.axes``).
"""

from .. import _api, _background_patch, _values
from .._native import axes as _axes, axes_base as _axes_base


class DecorationsMixin:
    """The decoration methods of `~.axes.Axes`."""

    @property
    def patch(self):
        """The background of the Axes: the patch `_gen_axes_patch` made,
        mirroring the background pltrs (Rust) draws (see
        `._background_patch`)."""
        return _background_patch.axes_patch(self)

    @patch.setter
    def patch(self, patch):
        _background_patch.set_axes_patch(self, patch)

    def _fit_patch(self, patch):
        """Give *patch*, the background, what the Axes' draw gives it:
        nothing here (`.PolarAxes` fits its `.Wedge` to the view)."""

    def _gen_axes_patch(self):
        """
        Returns
        -------
        Patch
            The patch used to draw the background of the Axes.  It is also used
            as the clipping path for any data elements on the Axes.

            In the standard Axes, this is a rectangle, but in other projections
            it may not be.

        Notes
        -----
        Intended to be overridden by new projection types.
        """
        from ..patches import Rectangle
        return Rectangle((0.0, 0.0), 1.0, 1.0)

    @property
    def axison(self):
        """Whether the patch, spines and axes are drawn."""
        return _axes_base.get_axis_on(self._native)

    def set_axis_off(self):
        """
        Hide all visual components of the x- and y-axis.

        This sets a flag to suppress drawing of all axis decorations, i.e.
        axis labels, axis spines, and the axis tick component (tick markers,
        tick labels, and grid lines). Individual visibility settings of these
        components are ignored as long as `set_axis_off()` is in effect.
        """
        _axes_base.set_axis_on(self._native, False)

    def set_axis_on(self):
        """
        Do not hide all visual components of the x- and y-axis.

        This reverts the effect of a prior `.set_axis_off()` call. Whether the
        individual axis decorations are drawn is controlled by their respective
        visibility settings.

        This is on by default.
        """
        _axes_base.set_axis_on(self._native, True)

    def set_rasterization_zorder(self, z):
        """
        Set the zorder threshold for rasterization for vector graphics output.

        All artists with a zorder below the given value will be rasterized if
        they support rasterization.

        This setting is ignored for pixel-based output.

        See also :doc:`/gallery/misc/rasterization_demo`.

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

    @property
    def _frameon(self):
        """Whether the Axes rectangle patch is drawn (matplotlib's
        attribute, which pltrs (Rust) holds)."""
        # matplotlib keeps the object it was given (``1``, ``'False'``).
        return _values.given_flag(self, "frameon", _axes_base.get_frame_on(self._native))

    @_frameon.setter
    def _frameon(self, b):
        _axes_base.set_frame_on(self._native, bool(b))
        _values.remember(self, "frameon", b)

    def get_frame_on(self):
        """Get whether the Axes rectangle patch is drawn."""
        return self._frameon

    def set_frame_on(self, b):
        """
        Set whether the Axes rectangle patch is drawn.

        Parameters
        ----------
        b : bool
        """
        self._frameon = b

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
            possible values are:

            ================ ===========================================================
            Value            Description
            ================ ===========================================================
            'off' or `False` Hide all axis decorations, i.e. axis labels, spines,
                             tick marks, tick labels, and grid lines.
                             This is the same as `~.Axes.set_axis_off()`.
            'on' or `True`   Do not hide all axis decorations, i.e. axis labels, spines,
                             tick marks, tick labels, and grid lines.
                             This is the same as `~.Axes.set_axis_on()`.
            'equal'          Set equal scaling (i.e., make circles circular) by
                             changing the axis limits. This is the same as
                             ``ax.set_aspect('equal', adjustable='datalim')``.
                             Explicit data limits may not be respected in this case.
            'scaled'         Set equal scaling (i.e., make circles circular) by
                             changing dimensions of the plot box. This is the same as
                             ``ax.set_aspect('equal', adjustable='box', anchor='C')``.
                             Additionally, further autoscaling will be disabled.
            'tight'          Set limits just large enough to show all data, then
                             disable further autoscaling.
            'auto'           Automatic scaling (fill plot box with data).
            'image'          'scaled' with axis limits equal to data limits.
            'square'         Square plot; similar to 'scaled', but initially forcing
                             ``xmax-xmin == ymax-ymin``.
            ================ ===========================================================

        emit : bool, default: True
            Whether observers are notified of the axis limit change.
            This option is passed on to `~.Axes.set_xlim` and
            `~.Axes.set_ylim`.

        Returns
        -------
        xmin, xmax, ymin, ymax : float
            The axis limits.

        See Also
        --------
        matplotlib.axes.Axes.set_xlim
        matplotlib.axes.Axes.set_ylim

        Notes
        -----
        For 3D Axes, this method additionally takes *zmin*, *zmax* as
        parameters and likewise returns them.
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
        """
        [*Discouraged*] Return the XAxis instance.

        .. admonition:: Discouraged

            The use of this function is discouraged. You should instead
            directly access the attribute `~.Axes.xaxis`.
        """
        return self.xaxis

    def get_yaxis(self):
        """
        [*Discouraged*] Return the YAxis instance.

        .. admonition:: Discouraged

            The use of this function is discouraged. You should instead
            directly access the attribute `~.Axes.yaxis`.
        """
        return self.yaxis

    def set_xlabel(self, xlabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """
        Set the label for the x-axis.

        Parameters
        ----------
        xlabel : str
            The label text.

        labelpad : float, default: :rc:`axes.labelpad`
            Spacing in points from the Axes bounding box including ticks
            and tick labels.  If None, the previous value is left as is.

        loc : {'left', 'center', 'right'}, default: :rc:`xaxis.labellocation`
            The label position. This is a high-level alternative for passing
            parameters *x* and *horizontalalignment*.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.text.Text` properties
            `.Text` properties control the appearance of the label.

        See Also
        --------
        text : Documents the properties supported by `.Text`.
        """
        return self.xaxis._set_axes_label(xlabel, fontdict, labelpad, loc,
                                          kwargs)

    def get_xlabel(self):
        """
        Get the xlabel text string.
        """
        return _axes.get_xlabel(self._native)

    def set_ylabel(self, ylabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """
        Set the label for the y-axis.

        Parameters
        ----------
        ylabel : str
            The label text.

        labelpad : float, default: :rc:`axes.labelpad`
            Spacing in points from the Axes bounding box including ticks
            and tick labels.  If None, the previous value is left as is.

        loc : {'bottom', 'center', 'top'}, default: :rc:`yaxis.labellocation`
            The label position. This is a high-level alternative for passing
            parameters *y* and *horizontalalignment*.

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.text.Text` properties
            `.Text` properties control the appearance of the label.

        See Also
        --------
        text : Documents the properties supported by `.Text`.
        """
        return self.yaxis._set_axes_label(ylabel, fontdict, labelpad, loc,
                                          kwargs)

    def get_ylabel(self):
        """
        Get the ylabel text string.
        """
        return _axes.get_ylabel(self._native)
