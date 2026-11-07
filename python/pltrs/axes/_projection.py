"""
The methods a projection class overrides to give its Axes axes and spines
of their own (matplotlib's ``_gen_axes_spines`` and ``_init_axis``; its
transforms are `.._transform_trees`'), as a mixin of `._base._AxesBase`,
after matplotlib 3.11.2 (Matplotlib License): what pltrs (Rust) asks of
the Axes of a projection class the program defines
(`..projections._program`) while it lays the Axes out and draws it.
"""


class ProjectionMixin:
    """The construction methods projection classes override."""

    @property
    def axes(self):
        """The Axes itself (matplotlib's ``self.axes = self``), or for a
        child Axes (an inset, a secondary axis) the Axes it is a child of,
        as matplotlib's ``add_child_axes`` sets it; the one it was set to
        since (None once it was removed)."""
        if "_axes" in self.__dict__:
            return self._axes
        return self.__dict__.get("_child_of", self)

    @axes.setter
    def axes(self, new_axes):
        current = self.axes
        if new_axes is not None and current is not None and new_axes is not current:
            raise ValueError("Can not reset the Axes. You are probably trying to reuse "
                             "an artist in more than one Axes which is not supported")
        self._axes = new_axes

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
