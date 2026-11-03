"""
The transform trees of polar Axes (matplotlib's
``PolarAxes._set_lim_and_transforms``) and the transforms their ticks and
tick labels are placed with (``get_xaxis_text1_transform`` and its
siblings), of matplotlib's classes (from matplotlib 3.11.2, under the
Matplotlib License), over the leaves pltrs (Rust) holds: the view limits
and scales of every Axes (`.._transform_trees`), and the theta offset and
direction, the radial origin and the radial label position, which
`PolarTrees._apply_projection_leaves` gives the tree.
"""

import numpy as np

from .. import _api
from .. import transforms as mtransforms
from .._native import transforms as _transforms
from .._transform_trees import TreeAttribute, set_matrix


def _is_full_circle_rad(thetamin, thetamax):
    """
    Determine if a wedge (in radians) spans the full circle.

    The condition is derived from :class:`~matplotlib.patches.Wedge`.
    """
    return abs(abs(thetamax - thetamin) - 2 * np.pi) < 1.74e-14


class _ThetaShift(mtransforms.ScaledTranslation):
    """
    Apply a padding shift based on axes theta limits.

    This is used to create padding for radial ticks.

    Parameters
    ----------
    axes : `~matplotlib.axes.Axes`
        The owning Axes; used to determine limits.
    pad : float
        The padding to apply, in points.
    mode : {'min', 'max', 'rlabel'}
        Whether to shift away from the start (``'min'``) or the end (``'max'``)
        of the axes, or using the rlabel position (``'rlabel'``).
    """
    def __init__(self, axes, pad, mode):
        super().__init__(pad, pad, axes.get_figure(root=False).dpi_scale_trans)
        self.set_children(axes._realViewLim)
        self.axes = axes
        self.mode = mode
        self.pad = pad

    __str__ = mtransforms._make_str_method("axes", "pad", "mode")

    def get_matrix(self):
        if self._invalid:
            if self.mode == 'rlabel':
                angle = (
                    np.deg2rad(self.axes.get_rlabel_position()
                               * self.axes.get_theta_direction())
                    + self.axes.get_theta_offset()
                    - np.pi / 2
                )
            elif self.mode == 'min':
                angle = self.axes._realViewLim.xmin - np.pi / 2
            elif self.mode == 'max':
                angle = self.axes._realViewLim.xmax + np.pi / 2
            self._t = (self.pad * np.cos(angle) / 72, self.pad * np.sin(angle) / 72)
        return super().get_matrix()


class PolarTrees:
    """The transform trees of `.PolarAxes` (a mixin of it)."""

    transShift = TreeAttribute()
    transProjection = TreeAttribute()
    transProjectionAffine = TreeAttribute()
    transWedge = TreeAttribute()
    axesLim = TreeAttribute()
    _originViewLim = TreeAttribute()
    _realViewLim = TreeAttribute()
    _direction = TreeAttribute()
    _theta_offset = TreeAttribute()
    _r_label_position = TreeAttribute()
    _xaxis_text_transform = TreeAttribute()
    _yaxis_text_transform = TreeAttribute()

    def _set_lim_and_transforms(self):
        # A view limit where the minimum radius can be locked if the user
        # specifies an alternate origin.
        self._originViewLim = mtransforms.LockableBbox(self._viewLim)

        # Handle angular offset and direction (pltrs (Rust)'s, given by
        # `_apply_projection_leaves`).
        self._direction = mtransforms.Affine2D().scale(1.0, 1.0)
        self._theta_offset = mtransforms.Affine2D().translate(0.0, 0.0)
        self.transShift = self._direction + self._theta_offset
        # A view limit shifted to the correct location after accounting for
        # orientation and offset.
        self._realViewLim = mtransforms.TransformedBbox(self._viewLim,
                                                        self.transShift)

        # Transforms the x and y axis separately by a scale factor
        # It is assumed that this part will have non-linear components
        self.transScale = mtransforms.TransformWrapper(
            mtransforms.IdentityTransform())

        # Scale view limit into a bbox around the selected wedge. This may be
        # smaller than the usual unit axes rectangle if not plotting the full
        # circle.
        from .polar import _WedgeBbox
        self.axesLim = _WedgeBbox((0.5, 0.5),
                                  self._realViewLim, self._originViewLim)

        # Scale the wedge to fill the axes.
        self.transWedge = mtransforms.BboxTransformFrom(self.axesLim)

        # Scale the axes to fill the figure.
        self.transAxes = mtransforms.BboxTransformTo(self.bbox)

        # A (possibly non-linear) projection on the (already scaled)
        # data.  This one is aware of rmin
        self.transProjection = self.PolarTransform(
            self,
            scale_transform=self.transScale
        )
        # Add dependency on rorigin.
        self.transProjection.set_children(self._originViewLim)

        # An affine transformation on the data, generally to limit the
        # range of the axes
        self.transProjectionAffine = self.PolarAffine(self.transScale,
                                                      self._originViewLim)

        # The complete data transformation stack -- from data all the
        # way to display coordinates
        self.transData = (
            self.transScale +
            self.transShift +
            self.transProjection +
            (
                self.transProjectionAffine +
                self.transWedge +
                self.transAxes
            )
        )

        # This is the transform for theta-axis ticks.  It is
        # equivalent to transData, except it always puts r == 0.0 and r == 1.0
        # at the edge of the axis circles.
        self._xaxis_transform = (
            mtransforms.blended_transform_factory(
                mtransforms.IdentityTransform(),
                mtransforms.BboxTransformTo(self._viewLim)) +
            self.transData)
        # The theta labels are flipped along the radius, so that text 1 is on
        # the outside by default. This should work the same as before.
        flipr_transform = mtransforms.Affine2D() \
            .translate(0.0, -0.5) \
            .scale(1.0, -1.0) \
            .translate(0.0, 0.5)
        self._xaxis_text_transform = flipr_transform + self._xaxis_transform

        # This is the transform for r-axis ticks.  It scales the theta
        # axis so the gridlines from 0.0 to 1.0, now go from thetamin to
        # thetamax.
        self._yaxis_transform = (
            mtransforms.blended_transform_factory(
                mtransforms.BboxTransformTo(self._viewLim),
                mtransforms.IdentityTransform()) +
            self.transData)
        # The r-axis labels are put at an angle and padded in the r-direction
        self._r_label_position = mtransforms.Affine2D() \
            .translate(np.deg2rad(22.5), 0.0)
        self._yaxis_text_transform = mtransforms.TransformWrapper(
            self._r_label_position + self.transData)
        # pltrs (Rust) draws through its own: the polar axis transforms.
        self._xaxis_transform._live = _transforms.trans_xaxis(self._native)
        self._yaxis_transform._live = _transforms.trans_yaxis(self._native)

    def _apply_projection_leaves(self, leaves):
        """Give the tree pltrs (Rust)'s theta offset and direction, radial
        origin and radial label position, and the radial labels the
        transform matplotlib's ``draw`` gives them for the theta limits."""
        _, theta_offset, direction, rorigin, rlabel_position = leaves
        set_matrix(self._direction,
                   mtransforms.Affine2D().scale(direction, 1.0).get_matrix())
        set_matrix(self._theta_offset,
                   mtransforms.Affine2D().translate(theta_offset, 0.0).get_matrix())
        set_matrix(self._r_label_position,
                   mtransforms.Affine2D().translate(rlabel_position, 0.0).get_matrix())
        if self._originViewLim.locked_y0 != rorigin:
            self._originViewLim.locked_y0 = rorigin
        thetamin, thetamax = self._realViewLim.intervalx
        if _is_full_circle_rad(thetamin, thetamax):
            yaxis_text_transform = self._r_label_position + self.transData
        else:
            yaxis_text_transform = self._yaxis_transform
        if self._yaxis_text_transform != yaxis_text_transform:
            self._yaxis_text_transform.set(yaxis_text_transform)

    def get_xaxis_transform(self, which='grid'):
        _api.check_in_list(['tick1', 'tick2', 'grid'], which=which)
        return self._xaxis_transform

    def get_xaxis_text1_transform(self, pad):
        return self._xaxis_text_transform, 'center', 'center'

    def get_xaxis_text2_transform(self, pad):
        return self._xaxis_text_transform, 'center', 'center'

    def get_yaxis_transform(self, which='grid'):
        if which in ('tick1', 'tick2'):
            return self._yaxis_text_transform
        elif which == 'grid':
            return self._yaxis_transform
        else:
            _api.check_in_list(['tick1', 'tick2', 'grid'], which=which)

    def get_yaxis_text1_transform(self, pad):
        thetamin, thetamax = self._realViewLim.intervalx
        if _is_full_circle_rad(thetamin, thetamax):
            return self._yaxis_text_transform, 'bottom', 'left'
        elif self.get_theta_direction() > 0:
            halign = 'left'
            pad_shift = _ThetaShift(self, pad, 'min')
        else:
            halign = 'right'
            pad_shift = _ThetaShift(self, pad, 'max')
        return self._yaxis_text_transform + pad_shift, 'center', halign

    def get_yaxis_text2_transform(self, pad):
        if self.get_theta_direction() > 0:
            halign = 'right'
            pad_shift = _ThetaShift(self, pad, 'max')
        else:
            halign = 'left'
            pad_shift = _ThetaShift(self, pad, 'min')
        return self._yaxis_text_transform + pad_shift, 'center', halign
