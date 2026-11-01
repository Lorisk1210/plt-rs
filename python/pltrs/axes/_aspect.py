"""
The aspect of an Axes and the limits it adjusts (matplotlib's
``axes._base`` methods ``set_aspect``, ``set_adjustable``, ``set_anchor``,
``set_box_aspect``, ``apply_aspect`` and their getters, ``dataLim`` and
``viewLim``): matplotlib's argument handling, the aspect and the limits
kept and applied by pltrs (Rust).
"""

import numpy as np

from .. import _api, _live_transforms, cbook
from .._native import axes_base as _axes_base
from ..transforms import Bbox


class _ViewLimits(_live_transforms.LiveBbox):
    """
    The view limits of an Axes (matplotlib's ``viewLim``): read from
    pltrs (Rust) whenever they are, and set as they are by `set`, without
    the autoscaling switch, the axes sharing them or the callbacks
    ``set_xlim`` involves (matplotlib assigns the box's points).
    """

    def __init__(self, axes):
        def points():
            (x0, x1), (y0, y1) = axes.get_xlim(), axes.get_ylim()
            return (x0, y0, x1, y1)
        super().__init__(points)
        self._axes = axes

    def set(self, other):
        """Set this bounding box from the "frozen" bounds of another `Bbox`."""
        # Reading the limits applies pending autoscaling first, as
        # matplotlib's ``viewLim`` property does, so that it does not
        # replace the limits set here.
        self.get_points()
        (x0, y0), (x1, y1) = other.get_points()
        self._axes.xaxis.set_view_interval(x0, x1, ignore=True)
        self._axes.yaxis.set_view_interval(y0, y1, ignore=True)


class AspectMixin:
    """The aspect methods of `~.axes._base._AxesBase`."""

    def _aspect_group(self, share):
        """The axes a setting applies to: the axes, or with *share* every
        axes sharing an axis with it."""
        if not share:
            return [self]
        group = []
        for name in self._axis_names:
            for sibling in self._shared_axes[name].get_siblings(self):
                if sibling not in group:
                    group.append(sibling)
        return group

    def get_aspect(self):
        """
        Return the aspect ratio of the Axes scaling.

        This is either "auto" or a float giving the ratio of y/x-scale.
        """
        ratio = _axes_base.get_aspect(self._native)
        return "auto" if ratio is None else ratio

    def set_aspect(self, aspect, adjustable=None, anchor=None, share=False):
        """
        Set the aspect ratio of the Axes scaling, i.e. y/x-scale.

        Parameters
        ----------
        aspect : {'auto', 'equal'} or float
            Possible values:

            - 'auto': fill the position rectangle with data.
            - 'equal': same as ``aspect=1``, i.e. same scaling for x and y.
            - *float*: The displayed size of 1 unit in y-data coordinates will
              be *aspect* times the displayed size of 1 unit in x-data
              coordinates.
        adjustable : None or {'box', 'datalim'}, optional
            If not ``None``, this defines which parameter will be adjusted to
            meet the required aspect. See `.set_adjustable` for further
            details.
        anchor : None or str or (float, float), optional
            If not ``None``, this defines where the Axes will be drawn if there
            is extra space due to aspect constraints. See `.set_anchor`.
        share : bool, default: False
            If ``True``, apply the settings to all shared Axes.
        """
        if cbook._str_equal(aspect, 'equal'):
            aspect = 1
        if not cbook._str_equal(aspect, 'auto'):
            aspect = float(aspect)  # raise ValueError if necessary
            if aspect <= 0 or not np.isfinite(aspect):
                raise ValueError("aspect must be finite and positive ")
        for ax in self._aspect_group(share):
            _axes_base.set_aspect(ax._native,
                                  None if aspect == 'auto' else aspect)
        if adjustable is None:
            adjustable = self.get_adjustable()
        self.set_adjustable(adjustable, share=share)
        if anchor is not None:
            self.set_anchor(anchor, share=share)

    def get_adjustable(self):
        """
        Return whether the Axes will adjust its physical dimension ('box') or
        its data limits ('datalim') to achieve the desired aspect ratio.
        """
        return _axes_base.get_adjustable(self._native)

    def set_adjustable(self, adjustable, share=False):
        """
        Set how the Axes adjusts to achieve the required aspect ratio.

        Parameters
        ----------
        adjustable : {'box', 'datalim'}
            If 'box', change the physical dimensions of the Axes.
            If 'datalim', change the ``x`` or ``y`` data limits.
        share : bool, default: False
            If ``True``, apply the settings to all shared Axes.
        """
        _api.check_in_list(["box", "datalim"], adjustable=adjustable)
        for ax in self._aspect_group(share):
            _axes_base.set_adjustable(ax._native, adjustable)

    def get_anchor(self):
        """
        Get the anchor location: a code (``'C'``, ``'SW'``...) or a pair of
        fractions.
        """
        code, fractions = _axes_base.get_anchor(self._native)
        return code if code is not None else fractions

    def set_anchor(self, anchor, share=False):
        """
        Define the anchor location.

        Parameters
        ----------
        anchor : (float, float) or {'C', 'SW', 'S', 'SE', 'E', 'NE', ...}
            Either an (*x*, *y*) pair of relative coordinates (0 is left or
            bottom, 1 is right or top), 'C' (center), or a cardinal direction
            ('SW', southwest, is bottom left, etc.).
        share : bool, default: False
            If ``True``, apply the settings to all shared Axes.
        """
        if not (anchor in Bbox.coefs or len(anchor) == 2):
            raise ValueError('argument must be among %s' %
                             ', '.join(Bbox.coefs))
        named = isinstance(anchor, str)
        for ax in self._aspect_group(share):
            _axes_base.set_anchor(
                ax._native, anchor if named else None,
                (0.0, 0.0) if named else tuple(map(float, anchor)))

    def get_box_aspect(self):
        """
        Return the Axes box aspect, i.e. the ratio of height to width, or
        None when it is not fixed.
        """
        return _axes_base.get_box_aspect(self._native)

    def set_box_aspect(self, aspect=None):
        """
        Set the Axes box aspect, i.e. the ratio of height to width.

        This defines the aspect of the Axes in figure space and is not to be
        confused with the data aspect (see `~.Axes.set_aspect`).

        Parameters
        ----------
        aspect : float or None
            Changes the physical dimensions of the Axes, such that the ratio
            of the Axes height to the Axes width in physical units is equal to
            *aspect*. Defining a box aspect will change the *adjustable*
            property to 'datalim' (see `~.Axes.set_adjustable`).

            *None* will disable a fixed box aspect so that height and width
            of the Axes are chosen independently.
        """
        _axes_base.set_box_aspect(self._native,
                                  None if aspect is None else float(aspect))

    def get_data_ratio(self):
        """
        Return the aspect ratio of the scaled data.
        """
        return _axes_base.get_data_ratio(self._native)

    def _get_aspect_ratio(self):
        """
        Convenience method to calculate the aspect ratio of the Axes in
        the display coordinate system.
        """
        figure_size = self.get_figure().get_size_inches()
        ll, ur = np.asarray(self.get_position()) * figure_size
        width, height = ur - ll
        return height / (width * self.get_data_ratio())

    @property
    def dataLim(self):
        """The bounding box of the data the Axes autoscales to, kept by
        pltrs (Rust) (read-only here)."""
        return _live_transforms.LiveBbox(
            lambda: _axes_base.data_lim(self._native)[0],
            lambda: _axes_base.data_lim(self._native)[1])

    @property
    def viewLim(self):
        """The view limits of the Axes, ``(x0, y0), (x1, y1)`` in data
        coordinates, which `~.Bbox.set` sets as they are."""
        # The box the transforms of a program's projection class are built on.
        built = self.__dict__.get("_viewLim")
        return _ViewLimits(self) if built is None else built

    def apply_aspect(self, position=None):
        """
        Adjust the Axes for a specified data aspect ratio.

        Depending on `.get_adjustable` this will modify either the
        Axes box (position) or the view limits. In the former case,
        `~matplotlib.axes.Axes.get_anchor` will affect the position.

        Parameters
        ----------
        position : None or .Bbox
            If not ``None``, this defines the position of the
            Axes within the figure as a Bbox.
        """
        _axes_base.apply_aspect(
            self._native, None if position is None
            else tuple(map(float, position.bounds)))
