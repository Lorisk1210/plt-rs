"""
The frame of a figure or subfigure (matplotlib's ``FigureBase``
``get_edgecolor``, ``set_linewidth``, ``frameon``, ...): its background
patch, which pltrs (Rust) draws as part of the (sub)figure (see
`._background_patch`).
"""


class FrameMixin:
    """The edge colour, line width and visibility of the background of
    `.Figure` and `.SubFigure`."""

    # A (sub)figure's own alpha, transform and clip regions are kept, not
    # drawn: matplotlib's ``Figure.draw`` draws nothing with them, and its
    # background has an alpha of its own (``patch.set_alpha``).
    _kept_only = True

    def get_edgecolor(self):
        """Get the edge color of the Figure rectangle."""
        return self.patch.get_edgecolor()

    def set_edgecolor(self, color):
        """
        Set the edge color of the Figure rectangle.

        Parameters
        ----------
        color : :mpltype:`color`
        """
        self.patch.set_edgecolor(color)

    def get_linewidth(self):
        """
        Get the line width of the Figure rectangle.
        """
        return self.patch.get_linewidth()

    def set_linewidth(self, linewidth):
        """
        Set the line width of the Figure rectangle.

        Parameters
        ----------
        linewidth : number
        """
        self.patch.set_linewidth(linewidth)

    def get_frameon(self):
        """
        Return the figure's background patch visibility, i.e.
        whether the figure background will be drawn. Equivalent to
        ``Figure.patch.get_visible()``.
        """
        return self.patch.get_visible()

    def set_frameon(self, b):
        """
        Set the figure's background patch visibility, i.e.
        whether the figure background will be drawn. Equivalent to
        ``Figure.patch.set_visible()``.

        Parameters
        ----------
        b : bool
        """
        self.patch.set_visible(b)
        self.stale = True

    frameon = property(get_frameon, set_frameon)
