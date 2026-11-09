"""
The aspect of an Axes and the limits it adjusts (matplotlib's
``axes._base`` methods ``set_aspect``, ``set_adjustable``, ``set_anchor``,
``set_box_aspect``, ``apply_aspect`` and their getters): matplotlib's
argument handling, the aspect kept and applied by pltrs (Rust).
"""

import numpy as np

from .. import _api, _unusable, cbook
from .._native import axes_base as _axes_base
from .. import transforms as mtransforms
from ..transforms import Bbox



def _checked_aspect(aspect):
    """*aspect* as matplotlib's `~.Axes.set_aspect` takes it: ``'auto'``,
    else a ratio (1 for ``'equal'``), raising its errors for one that is
    no finite positive number."""
    if cbook._str_equal(aspect, 'equal'):
        aspect = 1
    if not cbook._str_equal(aspect, 'auto'):
        aspect = float(aspect)  # raise ValueError if necessary
        if aspect <= 0 or not np.isfinite(aspect):
            raise ValueError("aspect must be finite and positive ")
    return aspect

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
              coordinates; e.g. for ``aspect=2`` a square in data coordinates
              will be rendered with a height of twice its width.

        adjustable : None or {'box', 'datalim'}, optional
            If not ``None``, this defines which parameter will be adjusted to
            meet the required aspect. See `.set_adjustable` for further
            details.

        anchor : None or str or (float, float), optional
            If not ``None``, this defines where the Axes will be drawn if there
            is extra space due to aspect constraints. The most common way
            to specify the anchor are abbreviations of cardinal directions:

            =====   =====================
            value   description
            =====   =====================
            'C'     centered
            'SW'    lower left corner
            'S'     middle of bottom edge
            'SE'    lower right corner
            etc.
            =====   =====================

            See `~.Axes.set_anchor` for further details.

        share : bool, default: False
            If ``True``, apply the settings to all shared Axes.

        Notes
        -----
        The aspect will require an update of the Axes position or limits (which
        one depends on *adjustable*). This update is applied lazily, the latest
        when the figure is drawn. Use `.apply_aspect` to force an update.

        See Also
        --------
        matplotlib.axes.Axes.set_adjustable
            Set how the Axes adjusts to achieve the required aspect ratio.
        matplotlib.axes.Axes.set_anchor
            Set the position in case of extra space.
        matplotlib.axes.Axes.apply_aspect
            Force the update required to meet the aspect ratio to happen
            immediately.
        """
        aspect = _checked_aspect(aspect)
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

        Newly created Axes default to 'box'.

        See Also
        --------
        matplotlib.axes.Axes.set_adjustable
            Set how the Axes adjusts to achieve the required aspect ratio.
        matplotlib.axes.Axes.set_aspect
            For a description of aspect handling.
        """
        return _axes_base.get_adjustable(self._native)

    def set_adjustable(self, adjustable, share=False):
        """
        Set how the Axes adjusts to achieve the required aspect ratio.

        Parameters
        ----------
        adjustable : {'box', 'datalim'}
            If 'box', change the physical dimensions of the Axes.
            If 'datalim', change the ``x`` or ``y`` data limits. This
            may ignore explicitly defined axis limits.

        share : bool, default: False
            If ``True``, apply the settings to all shared Axes.

        See Also
        --------
        matplotlib.axes.Axes.get_adjustable
            Return the current value of *adjustable*.
        matplotlib.axes.Axes.set_aspect
            For a description of aspect handling.

        Notes
        -----
        Shared Axes (of which twinned Axes are a special case)
        impose restrictions on how aspect ratios can be imposed.
        For twinned Axes, use 'datalim'.  For Axes that share both
        x and y, use 'box'.  Otherwise, either 'datalim' or 'box'
        may be used.  These limitations are partly a requirement
        to avoid over-specification, and partly a result of the
        particular implementation we are currently using, in
        which the adjustments for aspect ratios are done sequentially
        and independently on each Axes as it is drawn.
        """
        _api.check_in_list(["box", "datalim"], adjustable=adjustable)
        for ax in self._aspect_group(share):
            _axes_base.set_adjustable(ax._native, adjustable)

    def get_anchor(self):
        """
        Get the anchor location.

        See Also
        --------
        matplotlib.axes.Axes.set_anchor
            for a description of the anchor.
        matplotlib.axes.Axes.set_aspect
            for a description of aspect handling.
        """
        code, fractions = _axes_base.get_anchor(self._native)
        return _unusable.kept(self, "anchor", code if code is not None else fractions)

    def set_anchor(self, anchor, share=False):
        """
        Define the anchor location.

        The actual drawing area (active position) of the Axes may be smaller
        than the Bbox (original position) when a fixed aspect is required. The
        anchor defines where the drawing area will be located within the
        available space.

        Parameters
        ----------
        anchor : (float, float) or {'C', 'SW', 'S', 'SE', 'E', 'NE', ...}
            Either an (*x*, *y*) pair of relative coordinates (0 is left or
            bottom, 1 is right or top), 'C' (center), or a cardinal direction
            ('SW', southwest, is bottom left, etc.).  str inputs are shorthands
            for (*x*, *y*) coordinates, as shown in the following diagram::

               ┌─────────────────┬─────────────────┬─────────────────┐
               │ 'NW' (0.0, 1.0) │ 'N' (0.5, 1.0)  │ 'NE' (1.0, 1.0) │
               ├─────────────────┼─────────────────┼─────────────────┤
               │ 'W'  (0.0, 0.5) │ 'C' (0.5, 0.5)  │ 'E'  (1.0, 0.5) │
               ├─────────────────┼─────────────────┼─────────────────┤
               │ 'SW' (0.0, 0.0) │ 'S' (0.5, 0.0)  │ 'SE' (1.0, 0.0) │
               └─────────────────┴─────────────────┴─────────────────┘

        share : bool, default: False
            If ``True``, apply the settings to all shared Axes.

        See Also
        --------
        matplotlib.axes.Axes.set_aspect
            for a description of aspect handling.
        """
        if not (anchor in Bbox.coefs or len(anchor) == 2):
            raise ValueError('argument must be among %s' %
                             ', '.join(Bbox.coefs))
        named = isinstance(anchor, str)
        for ax in self._aspect_group(share):
            if named and anchor not in Bbox.coefs:
                # Two letters naming no anchor, which matplotlib keeps, and
                # fails on where it applies a fixed aspect.
                _unusable.keep(ax, "anchor", anchor, ax._anchor_box_corner)
                continue
            _unusable.drop(ax, "anchor")
            _axes_base.set_anchor(
                ax._native, anchor if named else None,
                (0.0, 0.0) if named else tuple(map(float, anchor)))

    def _anchor_box_corner(self, anchor):
        """What matplotlib's `apply_aspect` reads of the *anchor* when it
        shrinks the box (a fixed aspect adjusting the box, or a box
        aspect): where it puts the box (``Bbox.coefs``)."""
        if self.get_aspect() == 'auto' and self.get_box_aspect() is None:
            return None
        if self.get_adjustable() == 'box' or self.get_box_aspect() is not None:
            return Bbox.coefs[anchor]
        return None

    def get_box_aspect(self):
        """
        Return the Axes box aspect, i.e. the ratio of height to width.

        The box aspect is ``None`` (i.e. chosen depending on the available
        figure space) unless explicitly specified.

        See Also
        --------
        matplotlib.axes.Axes.set_box_aspect
            for a description of box aspect.
        matplotlib.axes.Axes.set_aspect
            for a description of aspect handling.
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

        See Also
        --------
        matplotlib.axes.Axes.set_aspect
            for a description of aspect handling.
        """
        if aspect is not None:
            aspect = float(aspect)
            # The twinned Axes adjust their data limits from now on (which
            # Axes overriding get_data_ratio refuse), as matplotlib's do;
            # the aspect is kept as given, and fails when it is applied
            # unless positive.
            for ax in self._twinned_axes.get_siblings(self):
                ax.set_adjustable("datalim")
        _axes_base.set_box_aspect(self._native, aspect)

    def reset_position(self):
        """
        Reset the active position to the original position.

        This undoes changes to the active position (as defined in
        `.set_position`) which may have been performed to satisfy fixed-aspect
        constraints.
        """
        _axes_base.reset_position(self._native)

    def get_data_ratio(self):
        """
        Return the aspect ratio of the scaled data.

        Notes
        -----
        This method is intended to be overridden by new projection types.
        """
        txmin, txmax = self.xaxis.get_transform().transform(self.get_xbound())
        tymin, tymax = self.yaxis.get_transform().transform(self.get_ybound())
        xsize = max(abs(txmax - txmin), 1e-30)
        ysize = max(abs(tymax - tymin), 1e-30)
        return ysize / xsize

    def _get_aspect_ratio(self):
        """
        Convenience method to calculate the aspect ratio of the Axes in
        the display coordinate system.
        """
        figure_size = self.get_figure().get_size_inches()
        ll, ur = np.asarray(self.get_position()) * figure_size
        width, height = ur - ll
        return height / (width * self.get_data_ratio())

    def apply_aspect(self, position=None):
        """
        Adjust the Axes for a specified data aspect ratio.

        Depending on `.get_adjustable` this will modify either the
        Axes box (position) or the view limits. In the former case,
        `~matplotlib.axes.Axes.get_anchor` will affect the position.

        Parameters
        ----------
        position : None or .Bbox

            .. note::
                This parameter exists for historic reasons and is considered
                internal. End users should not use it.

            If not ``None``, this defines the position of the
            Axes within the figure as a Bbox. See `~.Axes.get_position`
            for further details.

        Notes
        -----
        This is called automatically when each Axes is drawn.  You may need
        to call it yourself if you need to update the Axes position and/or
        view limits before the Figure is drawn.

        An alternative with a broader scope is `.Figure.draw_without_rendering`,
        which updates all stale components of a figure, not only the positioning /
        view limits of a single Axes.

        See Also
        --------
        matplotlib.axes.Axes.set_aspect
            For a description of aspect ratio handling.
        matplotlib.axes.Axes.set_adjustable
            Set how the Axes adjusts to achieve the required aspect ratio.
        matplotlib.axes.Axes.set_anchor
            Set the position in case of extra space.
        matplotlib.figure.Figure.draw_without_rendering
            Update all stale components of a figure.

        Examples
        --------
        A typical usage example would be the following. `~.Axes.imshow` sets the
        aspect to 1, but adapting the Axes position and extent to reflect this is
        deferred until rendering for performance reasons. If you want to know the
        Axes size before, you need to call `.apply_aspect` to get the correct
        values.

        >>> fig, ax = plt.subplots()
        >>> ax.imshow(np.zeros((3, 3)))
        >>> ax.bbox.width, ax.bbox.height
        (496.0, 369.59999999999997)
        >>> ax.apply_aspect()
        >>> ax.bbox.width, ax.bbox.height
        (369.59999999999997, 369.59999999999997)
        """
        if position is not None:
            if self.get_aspect() == 'auto' and self.get_box_aspect() is None:
                # matplotlib only sets the active position then, from a
                # box or its bounds.
                if not isinstance(position, mtransforms.BboxBase):
                    position = mtransforms.Bbox.from_bounds(*position)
            else:
                position = position.frozen()
        _axes_base.apply_aspect(
            self._native, None if position is None
            else tuple(map(float, position.bounds)))
        # matplotlib sets the active position anew.
        self.stale = True
