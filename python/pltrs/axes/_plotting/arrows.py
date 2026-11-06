"""
`~.Axes.arrow`: a `.FancyArrow` added to the Axes, which autoscales its
view.
"""

from ... import _docstring
from ... import patches as mpatches


class ArrowsMixin:
    """`~.Axes.arrow`."""

    @_docstring.interpd
    def arrow(self, x, y, dx, dy, **kwargs):
        """
        [*Discouraged*] Add an arrow to the Axes.

        This draws an arrow from ``(x, y)`` to ``(x+dx, y+dy)``.

        .. admonition:: Discouraged

            The use of this method is discouraged because it is not guaranteed
            that the arrow renders reasonably. For example, the resulting arrow
            is affected by the Axes aspect ratio and limits, which may distort
            the arrow.

            Consider using `~.Axes.annotate` without a text instead, e.g. ::

                ax.annotate("", xytext=(0, 0), xy=(0.5, 0.5),
                            arrowprops=dict(arrowstyle="->"))

        Parameters
        ----------
        %(FancyArrow)s

        Returns
        -------
        `.FancyArrow`
            The created `.FancyArrow` object.
        """
        a = mpatches.FancyArrow(x, y, dx, dy, **kwargs)
        self.add_patch(a)
        self._request_autoscale_view()
        return a
