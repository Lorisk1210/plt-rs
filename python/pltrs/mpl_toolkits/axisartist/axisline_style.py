"""
Provides classes to style the axis lines (matplotlib's
``mpl_toolkits.axisartist.axisline_style``, from matplotlib 3.11.2, under
the Matplotlib License): a style describes the arrow head pltrs (Rust)
draws at the end of an axis line (``AxisArtist.set_axisline_style``).
"""

from pltrs._patch_styles import _Style


class AxislineStyle(_Style):
    """
    A container class which defines style classes for AxisArtists.

    An instance of any axisline style class is a callable object,
    whose call signature is ::

       __call__(self, axis_artist, path, transform)

    When called, this should return an `.Artist` with the following methods::

      def set_path(self, path):
          # set the path for axisline.

      def set_line_mutation_scale(self, scale):
          # set the scale

      def draw(self, renderer):
          # draw
    """
    # An instance of a style class describes how pltrs (Rust) draws the line
    # of the axis artist it is given to.

    _style_list = {}

    class _Base:
        # The derived classes are required to be able to be initialized
        # w/o arguments, i.e., all its argument (except self) must have
        # the default values.

        def _native_text(self):
            """The style as pltrs (Rust) reads it (``"->,size=1.5"``)."""
            return self._name + "".join(
                f",{key}={value}" for key, value in self._attributes().items()
                if value is not None)

    class SimpleArrow(_Base):
        """
        A simple arrow.
        """

        _name = "->"

        def __init__(self, size=1):
            """
            Parameters
            ----------
            size : float
                Size of the arrow as a fraction of the ticklabel size.
            """

            self.size = size
            super().__init__()

        def _attributes(self):
            return {"size": self.size}

    _style_list["->"] = SimpleArrow

    class FilledArrow(SimpleArrow):
        """
        An arrow with a filled head.
        """

        _name = "-|>"

        def __init__(self, size=1, facecolor=None):
            """
            Parameters
            ----------
            size : float
                Size of the arrow as a fraction of the ticklabel size.
            facecolor : :mpltype:`color`, default: :rc:`axes.edgecolor`
                Fill color.

                .. versionadded:: 3.7
            """

            self.size = size
            self._facecolor = facecolor
            super().__init__(size=size)

        def _attributes(self):
            return {"size": self.size, "facecolor": self._facecolor}

    _style_list["-|>"] = FilledArrow


# What matplotlib's module also has from its imports, which programs
# import from it.
import math  # noqa: E402, F401
import numpy as np  # noqa: E402, F401
import pltrs as mpl  # noqa: E402, F401
from pltrs.patches import FancyArrowPatch  # noqa: E402, F401
from pltrs.path import Path  # noqa: E402, F401
from pltrs.transforms import IdentityTransform  # noqa: E402, F401
