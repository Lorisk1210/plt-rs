"""
`~.Axes.arrow`: a `.FancyArrow` added to the Axes, which autoscales its
view.
"""

from ... import patches as mpatches


class ArrowsMixin:
    """`~.Axes.arrow`."""

    def arrow(self, x, y, dx, dy, **kwargs):
        """
        Add an arrow to the Axes.

        This draws an arrow from ``(x, y)`` to ``(x+dx, y+dy)``.

        Parameters
        ----------
        x, y : float
            The x and y coordinates of the arrow base.
        dx, dy : float
            The length of the arrow along x and y direction.
        **kwargs
            The keyword arguments of `.FancyArrow` (*width*,
            *length_includes_head*, *head_width*, *head_length*, *shape*,
            *overhang*, *head_starts_at_zero*) and `.Patch` properties.

        Returns
        -------
        `.FancyArrow`
            The created `.FancyArrow` object.
        """
        a = mpatches.FancyArrow(x, y, dx, dy, **kwargs)
        self.add_patch(a)
        self._request_autoscale_view()
        return a
