"""
The methods a projection class overrides to give its Axes transforms,
axes and spines of their own (matplotlib's ``_set_lim_and_transforms``,
``_update_transScale``, ``_gen_axes_spines`` and ``_init_axis``), as a
mixin of `._base._AxesBase`, after matplotlib 3.11.2 (Matplotlib License).

pltrs (Rust) gives the Axes of its own projections their transforms
itself: ``transData`` and its friends are live transforms
(`.._live_transforms`). These methods build matplotlib's Python
transforms for the Axes of a projection class the program defines
(`..projections._program`), whose subclass composes them, and whose
projection pltrs (Rust) asks while it lays the Axes out and draws it.
"""

from .. import transforms as mtransforms


class ProjectionMixin:
    """The construction methods projection classes override."""

    @property
    def axes(self):
        """The Axes itself (matplotlib's ``self.axes = self``)."""
        return self

    @axes.setter
    def axes(self, new_axes):
        if new_axes is not None and new_axes is not self:
            raise ValueError("Can not reset the Axes. You are probably trying to reuse "
                             "an artist in more than one Axes which is not supported")

    def _set_lim_and_transforms(self):
        """
        Set the *_xaxis_transform*, *_yaxis_transform*, *transScale*,
        *transData*, *transLimits* and *transAxes* transformations.

        .. note::

            This method is primarily used by rectilinear projections of the
            `~matplotlib.axes.Axes` class, and is meant to be overridden by
            new kinds of projection Axes that need different transformations
            and limits. (See `~matplotlib.projections.polar.PolarAxes` for an
            example.)
        """
        # transAxes stays the live transform of the Axes' box.
        self.transScale = mtransforms.TransformWrapper(
            mtransforms.IdentityTransform())
        self.transLimits = mtransforms.BboxTransformFrom(
            mtransforms.TransformedBbox(self._viewLim, self.transScale))
        # The parentheses are important for efficiency here -- they group
        # the last two (which are usually affines) separately from the first
        # (which, with log-scaling can be non-affine).
        self.transData = self.transScale + (self.transLimits + self.transAxes)
        self._xaxis_transform = mtransforms.blended_transform_factory(
            self.transData, self.transAxes)
        self._yaxis_transform = mtransforms.blended_transform_factory(
            self.transAxes, self.transData)
        # What a subclass leaves of these is drawn as rectilinear axes are.
        self._rectilinear_axis_transforms = (self._xaxis_transform,
                                             self._yaxis_transform)

    def _update_transScale(self):
        """Have *transScale* follow the scales of the axes (for transforms
        built by `_set_lim_and_transforms`; pltrs (Rust) scales its own)."""
        trans_scale = self.__dict__.get("transScale")
        if trans_scale is not None:
            trans_scale.set(mtransforms.blended_transform_factory(
                self.xaxis.get_transform(), self.yaxis.get_transform()))

    def _gen_axes_spines(self, locations=None, offset=0.0, units='inches'):
        """
        Returns
        -------
        dict
            Mapping of spine names to `.Line2D` or `.Patch` instances that are
            used to draw Axes spines.

            In the standard Axes, spines are single line segments, but in other
            projections they may not be.

        Notes
        -----
        Intended to be overridden by new projection types.
        """
        from ..spines import Spine
        return {side: Spine.linear_spine(self, side)
                for side in ['left', 'right', 'bottom', 'top']}

    def _init_axis(self):
        """Make the `.XAxis` and `.YAxis` of the Axes and register them with
        its spines (overridden by projections with axes of their own)."""
        from ..axis import XAxis, YAxis
        self.xaxis = XAxis(self, clear=False)
        self.spines.bottom.register_axis(self.xaxis)
        self.spines.top.register_axis(self.xaxis)
        self.yaxis = YAxis(self, clear=False)
        self.spines.left.register_axis(self.yaxis)
        self.spines.right.register_axis(self.yaxis)

    def _init_projection(self):
        """Hand the Axes of a projection class the program defines over to
        pltrs (Rust), once the Axes is in its figure's registry with its
        Python state set up (a no-op for the Axes of pltrs (Rust)'s own
        projections)."""
        from ..projections import _program
        if _program.is_program_class(type(self)):
            _program.set_up(self)
