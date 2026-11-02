"""
The transforms the ticks and tick labels of polar axes are placed with
(matplotlib's ``PolarAxes.get_xaxis_text1_transform`` and its siblings,
and ``get_yaxis_transform('tick1')``), built as matplotlib's
``PolarAxes._set_lim_and_transforms`` builds them, of matplotlib's
classes around the axes' live transforms. Each reads the axes' current
theta limits, offset, direction and radial label position whenever
points go through it, as matplotlib's invalidated transforms do. The
padding shift `_ThetaShift` is matplotlib's (from matplotlib 3.11.2,
under the Matplotlib License).
"""

import functools

import numpy as np

from .. import _api
from .. import transforms as mtransforms


def _is_full_circle_rad(thetamin, thetamax):
    """
    Determine if a wedge (in radians) spans the full circle.

    The condition is derived from :class:`~matplotlib.patches.Wedge`.
    """
    return abs(abs(thetamax - thetamin) - 2 * np.pi) < 1.74e-14


class _RLabelPosition(mtransforms.Affine2DBase):
    """The shift of the radial labels' theta to the axes' radial label
    position (matplotlib's ``PolarAxes._r_label_position``)."""

    def __init__(self, axes):
        super().__init__()
        self._axes = axes

    def get_matrix(self):
        theta = np.deg2rad(self._axes.get_rlabel_position())
        return mtransforms.Affine2D().translate(theta, 0.0).get_matrix()


class _ThetaShift(mtransforms.ScaledTranslation):
    """
    Apply a padding shift based on axes theta limits.

    This is used to create padding for radial ticks.

    Parameters
    ----------
    axes : `~matplotlib.axes.Axes`
        The owning Axes; used to determine limits.
    pad : float
        The padding to apply, in units of points.
    mode : {'min', 'max', 'rlabel'}
        Whether to shift away from the start (``'min'``) or the end (``'max'``)
        of the axes, or using the rlabel position (``'rlabel'``).
    """

    def __init__(self, axes, pad, mode):
        super().__init__(pad, pad, axes.get_figure(root=False).dpi_scale_trans)
        self.axes = axes
        self.mode = mode
        self.pad = pad

    __str__ = mtransforms._make_str_method("axes", "pad", "mode")

    def get_matrix(self):
        # The axes' limits are read every time: they change without
        # invalidating this transform.
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
        self._invalid = self._INVALID_FULL
        return super().get_matrix()


class PolarTextTransforms:
    """
    The transforms of `.PolarAxes` for the ticks and tick labels of its
    axes (a mixin of `.PolarAxes`).
    """

    @property
    def _realViewLim(self):
        """The view limits with theta turned by the axes' direction and
        offset, as the axes draws them."""
        return mtransforms.TransformedBbox(
            mtransforms.Bbox(self.viewLim.get_points()),
            mtransforms.Affine2D()
            .scale(self.get_theta_direction(), 1)
            .translate(self.get_theta_offset(), 0))

    @functools.cached_property
    def _xaxis_text_transform(self):
        # The theta labels are flipped along the radius, so that text 1 is
        # on the outside by default.
        flipr_transform = mtransforms.Affine2D() \
            .translate(0.0, -0.5) \
            .scale(1.0, -1.0) \
            .translate(0.0, 0.5)
        return flipr_transform + self.get_xaxis_transform()

    @functools.cached_property
    def _yaxis_text_wrapper(self):
        """The transform `_yaxis_text_transform` gives, which follows the
        axes' theta limits."""
        return mtransforms.TransformWrapper(self._rlabel_transform)

    @functools.cached_property
    def _rlabel_transform(self):
        # The r-axis labels are put at an angle and padded in the
        # r-direction.
        return _RLabelPosition(self) + self.transData

    @property
    def _yaxis_text_transform(self):
        """
        The transform of the radial labels: at the radial label position
        on a full circle, along the start and end edges of a wedge
        (matplotlib's ``PolarAxes.draw`` sets it so).
        """
        thetamin, thetamax = self._realViewLim.intervalx
        target = (self._rlabel_transform
                  if _is_full_circle_rad(thetamin, thetamax)
                  else super().get_yaxis_transform())
        wrapper = self._yaxis_text_wrapper
        if wrapper._child is not target:
            wrapper.set(target)
        return wrapper

    def get_xaxis_text1_transform(self, pad):
        return self._xaxis_text_transform, 'center', 'center'

    def get_xaxis_text2_transform(self, pad):
        return self._xaxis_text_transform, 'center', 'center'

    def get_yaxis_transform(self, which='grid'):
        if which in ('tick1', 'tick2'):
            return self._yaxis_text_transform
        elif which == 'grid':
            return super().get_yaxis_transform()
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
